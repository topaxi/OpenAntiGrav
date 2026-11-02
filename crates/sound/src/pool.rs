//! The music pool: every soundtrack source this machine can open, and which of
//! them a race draws songs from.
//!
//! # Sources
//!
//! - The **booted disc**, listed at once and enabled by default.
//! - **Another title's disc image** found on the search path
//!   ([`Library::containers`]), one row per pressing, disabled by default. The
//!   booted title's own other pressing is not a row: choosing between the two
//!   encodes of one soundtrack is the `MUSIC SOURCE` row's job.
//! - Each **folder** listed in `settings.toml` (`[audio] music_folders`),
//!   disabled until switched on.
//!
//! The disc and folder scans each run on their own worker, apart from the
//! booted disc's survey, so a hung mount never delays the booted disc's music
//! ([`MusicDiscs::wait_surveyed`] gates only the survey).
//!
//! # What a race draws from
//!
//! The race playlist is one numbered list: the booted soundtrack in its own
//! order first (when the booted disc is enabled), then every other enabled
//! source's songs. With only the booted disc enabled the list is that
//! soundtrack, so the choice is the one the game always made. A title found on
//! two pressings contributes its songs once, from the first enabled pressing in
//! id order, so no song plays twice from two encodes.
//!
//! The list grows as scans land; an index past the songs known so far fetches
//! nothing and the playlist wraps at the length known when it is asked.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use oag_disc::Platform;
use serde::{Deserialize, Serialize};

use crate::folders::{self, FolderScan, Reader};
use crate::library::Library;
use crate::listings::Listings;
use crate::{Track, identify, probe};

/// A folder named in the settings file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MusicFolder {
    /// Where it is on this machine.
    pub path: PathBuf,
    /// Whether its songs are in the pool. Switched on by editing the file until
    /// the menu has a text input and a way to add folders.
    #[serde(default)]
    pub enabled: bool,
}

/// What a source's row says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceStatus {
    /// Songs found.
    Ready(usize),
    /// Still being read.
    Scanning,
    /// Cannot be used, and why.
    Unavailable(String),
}

/// One row of the MUSIC page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRow {
    /// The stable key the settings file keeps the switch under.
    pub id: String,
    /// What the row shows: the title's name (with its platform when two pressings of one
    /// title are listed), or the folder's last path component.
    pub label: String,
    /// The line under the row: the image's or folder's full path.
    pub detail: Option<String>,
    /// Whether the source is in the pool.
    pub enabled: bool,
    /// What the scan found.
    pub status: SourceStatus,
}

/// One song of the pool beyond the booted soundtrack.
#[derive(Debug, Clone)]
pub(crate) enum Extra {
    /// A track of another title's disc.
    Disc {
        source: String,
        platform: Platform,
        track: Track,
        label: String,
    },
    /// A file in a folder.
    File(PathBuf),
}

#[derive(Debug)]
struct Booted {
    id: String,
    title: &'static str,
    platform: Platform,
    source: String,
}

#[derive(Debug)]
struct Disc {
    id: String,
    title: &'static str,
    source: String,
    platform: Platform,
    tracks: Vec<Track>,
}

#[derive(Debug)]
struct Folder {
    id: String,
    path: PathBuf,
    scan: FolderScan,
}

#[derive(Debug, Default)]
struct Inner {
    booted: Option<Booted>,
    discs: Vec<Disc>,
    discs_scanned: bool,
    discs_started: bool,
    folders: Vec<Folder>,
    /// The player's explicit switches, by source id. A source with none is on
    /// only if it is the booted disc.
    switches: BTreeMap<String, bool>,
    /// The folders named in settings, with their switches (kept so a scan can
    /// be started once the reader is known).
    named: Vec<MusicFolder>,
    /// How folder files are read; the real file system until a test says otherwise.
    reader: Option<Arc<dyn Reader>>,
}

/// The pool, shared by every clone of one [`crate::MusicDiscs`].
#[derive(Debug, Clone, Default)]
pub struct Pool {
    inner: Arc<Mutex<Inner>>,
    /// Counts every change a row could show, so a menu re-supplies on a change.
    revision: Arc<AtomicU32>,
}

fn disc_id(title: &str, platform: Platform) -> String {
    format!("disc:{}:{}", slug(title), slug(&platform.to_string()))
}

/// Lower case with every run of other characters as one `-`, so an id is one word.
pub(crate) fn slug(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn folder_id(path: &std::path::Path) -> String {
    format!("folder:{}", path.display())
}

impl Pool {
    fn with<R>(&self, f: impl FnOnce(&mut Inner) -> R) -> Option<R> {
        oag_thread::lock(&self.inner)
            .ok()
            .map(|mut inner| f(&mut inner))
    }

    fn bump(&self) {
        self.revision.fetch_add(1, Ordering::Relaxed);
    }

    /// A number that changes whenever a row, a switch or a scan result did.
    #[must_use]
    pub fn revision(&self) -> u32 {
        // A folder scan answers on its own thread, so fold the answers in.
        let answers = self
            .with(|inner| {
                inner
                    .folders
                    .iter()
                    .map(|folder| match folder.scan.status() {
                        folders::Status::Scanning => 0,
                        folders::Status::Ready(n) => 1 + u32::try_from(n).unwrap_or(0),
                        folders::Status::TimedOut => 2,
                        folders::Status::Failed(_) => 3,
                    })
                    .fold(0u32, u32::wrapping_add)
            })
            .unwrap_or(0);
        self.revision.load(Ordering::Relaxed).wrapping_add(answers)
    }

    /// Records the booted disc, which is a row from the start.
    pub(crate) fn set_booted(&self, title: &'static str, platform: Platform, source: &str) {
        let id = disc_id(title, platform);
        self.with(|inner| {
            inner.booted = Some(Booted {
                id,
                title,
                platform,
                source: source.to_string(),
            });
        });
        self.bump();
    }

    /// Applies the player's switches and the folders named in settings.
    ///
    /// Cheap and never reads the file system: a folder new to the list is only
    /// recorded; [`Self::scan_folders`] starts its scan.
    pub fn configure(&self, switches: &BTreeMap<String, bool>, folders: &[MusicFolder]) {
        self.with(|inner| {
            inner.switches = switches.clone();
            inner.named = folders.to_vec();
        });
        self.bump();
    }

    /// Switches one source on or off.
    pub fn set_enabled(&self, id: &str, on: bool) {
        self.with(|inner| {
            inner.switches.insert(id.to_string(), on);
        });
        self.bump();
    }

    /// The switches the player has made, as the settings file keeps them. A source
    /// not here is on only if it is the booted disc or a folder whose `enabled` is set.
    #[must_use]
    pub fn switches(&self) -> BTreeMap<String, bool> {
        self.with(|inner| inner.switches.clone())
            .unwrap_or_default()
    }

    fn enabled_in(inner: &Inner, id: &str) -> bool {
        inner.switches.get(id).copied().unwrap_or_else(|| {
            inner.booted.as_ref().is_some_and(|b| b.id == id)
                || inner
                    .named
                    .iter()
                    .any(|folder| folder.enabled && folder_id(&folder.path) == id)
        })
    }

    /// Whether the booted disc's soundtrack is in the pool.
    #[must_use]
    pub fn booted_enabled(&self) -> bool {
        self.with(|inner| {
            inner
                .booted
                .as_ref()
                .is_none_or(|booted| Self::enabled_in(inner, &booted.id))
        })
        .unwrap_or(true)
    }

    /// Every source, for the MUSIC page.
    #[must_use]
    pub fn rows(&self) -> Vec<SourceRow> {
        self.with(|inner| {
            let mut rows = Vec::new();
            // (id, title, platform, source, songs) of every disc row, booted first.
            let mut discs: Vec<(&str, &str, Platform, &str, usize)> = Vec::new();
            if let Some(booted) = &inner.booted {
                discs.push((&booted.id, booted.title, booted.platform, &booted.source, 0));
            }
            for disc in &inner.discs {
                discs.push((
                    &disc.id,
                    disc.title,
                    disc.platform,
                    &disc.source,
                    disc.tracks.len(),
                ));
            }
            for &(id, title, platform, source, songs) in &discs {
                let same_title = discs.iter().filter(|other| other.1 == title).count();
                let mut label = title.to_string();
                if same_title > 1 {
                    label = format!("{label} ({platform})");
                }
                rows.push(SourceRow {
                    id: id.to_string(),
                    label,
                    detail: Some(source.to_string()),
                    enabled: Self::enabled_in(inner, id),
                    status: SourceStatus::Ready(songs),
                });
            }
            for folder in &inner.folders {
                let status = match folder.scan.status() {
                    folders::Status::Scanning => SourceStatus::Scanning,
                    folders::Status::Ready(n) => SourceStatus::Ready(n),
                    folders::Status::TimedOut => {
                        SourceStatus::Unavailable("SLOW OR NO ANSWER".to_string())
                    }
                    folders::Status::Failed(why) => SourceStatus::Unavailable(why),
                };
                rows.push(SourceRow {
                    id: folder.id.clone(),
                    label: folder.path.file_name().map_or_else(
                        || folder.path.display().to_string(),
                        |name| name.to_string_lossy().into_owned(),
                    ),
                    detail: Some(folder.path.display().to_string()),
                    enabled: Self::enabled_in(inner, &folder.id),
                    status,
                });
            }
            rows
        })
        .unwrap_or_default()
    }

    /// Whether the search for other discs is still running.
    #[must_use]
    pub fn scanning_discs(&self) -> bool {
        self.with(|inner| inner.discs_started && !inner.discs_scanned)
            .unwrap_or(false)
    }

    /// The songs beyond the booted soundtrack, in playlist order. Never reads.
    pub(crate) fn extras(&self) -> Vec<Extra> {
        self.with(|inner| {
            let mut out = Vec::new();
            let mut taken: Vec<&'static str> = Vec::new();
            let mut discs: Vec<&Disc> = inner.discs.iter().collect();
            discs.sort_by(|a, b| a.id.cmp(&b.id));
            for disc in discs {
                if !Self::enabled_in(inner, &disc.id) || taken.contains(&disc.title) {
                    continue;
                }
                taken.push(disc.title);
                out.extend(disc.tracks.iter().map(|track| Extra::Disc {
                    source: disc.source.clone(),
                    platform: disc.platform,
                    track: *track,
                    label: disc.title.to_string(),
                }));
            }
            for folder in &inner.folders {
                if Self::enabled_in(inner, &folder.id) {
                    out.extend(folder.scan.files().into_iter().map(Extra::File));
                }
            }
            out
        })
        .unwrap_or_default()
    }

    /// The reader folder files go through.
    pub(crate) fn reader(&self) -> Arc<dyn Reader> {
        self.with(|inner| inner.reader.clone())
            .flatten()
            .unwrap_or_else(|| Arc::new(folders::Disk::default()))
    }

    /// Starts scanning every configured folder that has no scan yet, on workers,
    /// reading through `reader`.
    pub fn scan_folders(&self, reader: &Arc<dyn Reader>) {
        self.with(|inner| inner.reader = Some(Arc::clone(reader)));
        if cfg!(target_arch = "wasm32") {
            return;
        }
        let started = self.with(|inner| {
            let mut started = false;
            for folder in inner.named.clone() {
                let id = folder_id(&folder.path);
                if inner.folders.iter().any(|known| known.id == id) {
                    continue;
                }
                let scan = FolderScan::start(Arc::clone(reader), folder.path.clone());
                inner.folders.push(Folder {
                    id,
                    path: folder.path,
                    scan,
                });
                started = true;
            }
            started
        });
        if started == Some(true) {
            self.bump();
        }
    }

    /// Waits up to `limit` for every source that is switched on to finish its scan. For a
    /// music worker's thread, never the frame thread: a race that starts before a folder
    /// has been listed waits here rather than starting on nothing.
    pub(crate) fn wait_scans(&self, limit: std::time::Duration) {
        let until = web_time::Instant::now() + limit;
        while web_time::Instant::now() < until {
            let pending = self
                .with(|inner| {
                    let discs = !inner.discs_scanned
                        && inner.discs_started
                        && inner
                            .switches
                            .iter()
                            .any(|(id, on)| *on && id.starts_with("disc:"));
                    let folders = inner.folders.iter().any(|folder| {
                        Self::enabled_in(inner, &folder.id)
                            && matches!(folder.scan.status(), folders::Status::Scanning)
                    });
                    discs || folders
                })
                .unwrap_or(false);
            if !pending {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }

    /// Looks for the other title discs on a worker, apart from the survey.
    pub(crate) fn discs_unstarted(&self) -> bool {
        self.with(|inner| !std::mem::replace(&mut inner.discs_started, true))
            .unwrap_or(false)
    }

    pub(crate) fn scan_discs(
        &self,
        library: &'static dyn Library,
        listings: Listings,
        wait: impl FnOnce() + Send + 'static,
    ) {
        if cfg!(target_arch = "wasm32") {
            self.with(|inner| inner.discs_scanned = true);
            return;
        }
        let pool = self.clone();
        let task = oag_thread::Task::spawn("music-discs", move || {
            wait();
            let _work = probe::work("the other discs' soundtrack scan");
            // A scan that panics still answers, with what it found so far (nothing), so
            // nothing waits on it for ever.
            let read = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pool.read_discs(library, &listings);
            }));
            if read.is_err() {
                log::warn!("audio: the other discs' soundtrack scan panicked");
                pool.with(|inner| inner.discs_scanned = true);
            }
        });
        if task.is_finished() {
            self.with(|inner| inner.discs_scanned = true);
        }
    }

    fn read_discs(&self, library: &'static dyn Library, listings: &Listings) {
        let booted_title = self
            .with(|inner| inner.booted.as_ref().map(|booted| booted.title))
            .flatten();
        let mut found = Vec::new();
        for path in library.sources() {
            let source = path.to_string_lossy().into_owned();
            let Some((title, layout)) = identify(library, &source) else {
                log::debug!("audio: music pool: {source} is not a title this build opens");
                continue;
            };
            if Some(title.name) == booted_title {
                continue;
            }
            let platform = layout.platform;
            let soundtrack = match listings.read(library, &source, platform) {
                Ok(Some(soundtrack)) => soundtrack,
                Ok(None) => {
                    log::debug!("audio: music pool: {source} carries no soundtrack");
                    continue;
                }
                Err(error) => {
                    log::debug!("audio: music pool: {source} will not list: {error:#}");
                    continue;
                }
            };
            if soundtrack.tracks.is_empty() {
                continue;
            }
            let id = disc_id(title.name, platform);
            if found.iter().any(|known: &Disc| known.id == id) {
                continue;
            }
            found.push(Disc {
                id,
                title: title.name,
                source,
                platform,
                tracks: soundtrack.tracks,
            });
        }
        log::info!(
            "audio: music pool found {} other soundtracks on the search path",
            found.len()
        );
        self.with(|inner| {
            inner.discs = found;
            inner.discs_scanned = true;
        });
        self.bump();
    }
}

/// The race playlist's length for a booted soundtrack of `booted` songs.
pub(crate) fn playlist_len(pool: &Pool, booted: usize) -> usize {
    let base = if pool.booted_enabled() { booted } else { 0 };
    base + pool.extras().len()
}
