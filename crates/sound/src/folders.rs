//! A music folder on the file system, scanned off the calling thread.
//!
//! **A folder may live on a slow medium** (an NFS share, an SD card), and a read
//! there can block in the kernel for as long as the mount takes to answer or give
//! up. So nothing here runs on the thread that asks: [`FolderScan::start`] hands
//! the walk to a worker and returns, the worker writes its answer into shared
//! state, and a caller polls [`FolderScan::status`]. A scan that has not answered
//! within [`SCAN_DEADLINE`] reads as [`Status::TimedOut`], whatever the worker is
//! still doing; the worker is detached (never joined), so a hung mount costs a
//! parked thread, not a frame or the process's exit. If it answers late the
//! status moves to [`Status::Ready`].
//!
//! All file system access goes through [`Reader`], so a test can stand a reader
//! that blocks until released in for a hung mount.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use web_time::Instant;

/// How long a scan may stay unanswered before it is reported as timed out.
pub const SCAN_DEADLINE: Duration = Duration::from_secs(20);

/// How deep below the configured folder the walk looks.
const MAX_DEPTH: usize = 6;

/// The most songs one folder contributes, so a whole music library does not hold
/// the walk (and the menu's reading of it) for minutes.
const MAX_FILES: usize = 100_000;

/// The extensions this reads, lower case: only the formats `oag_music` decodes outside a
/// disc (`docs/architecture/music-threads.md`, "The music pool").
const EXTENSIONS: [&str; 1] = ["mp3"];

/// The file system as the scan sees it.
pub trait Reader: std::fmt::Debug + Send + Sync + 'static {
    /// The song files below `root`, each as a path, in any order.
    ///
    /// # Errors
    ///
    /// A folder that is missing, unreadable or unreachable.
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>>;

    /// One file's bytes.
    ///
    /// # Errors
    ///
    /// A file that is gone or unreadable.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
}

/// The real file system, with the walk's answer cached per directory.
///
/// The cache keeps, for each directory, its modification time and the songs and
/// subfolders it held. A later walk stats a directory and reuses the entry when
/// the time is unchanged, so a library on an NFS share costs one `stat` per
/// folder instead of a listing of each. A directory whose time moved is
/// listed again; a nested change moves only its own directory's time, so the
/// walk stays right below it.
#[derive(Debug, Clone, Default)]
pub struct Disk {
    cache: Option<PathBuf>,
}

impl Disk {
    /// A reader that keeps its per-directory cache under `directory`, one file per
    /// folder walked.
    #[must_use]
    pub fn cached_in(directory: PathBuf) -> Self {
        Self {
            cache: Some(directory),
        }
    }

    fn cache_file(&self, root: &Path) -> Option<PathBuf> {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in root.to_string_lossy().bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
        self.cache
            .as_ref()
            .map(|directory| directory.join(format!("music-folder-{hash:016x}.tsv")))
    }
}

/// One directory as last walked.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Listed {
    modified: u128,
    files: Vec<String>,
    folders: Vec<String>,
}

type DirCache = std::collections::BTreeMap<PathBuf, Listed>;

fn read_cache(file: &Path) -> DirCache {
    let mut out = DirCache::new();
    let Ok(text) = std::fs::read_to_string(file) else {
        return out;
    };
    let mut current: Option<PathBuf> = None;
    for line in text.lines() {
        let mut parts = line.splitn(3, '\t');
        match (parts.next(), parts.next(), parts.next()) {
            (Some("D"), Some(modified), Some(path)) => {
                let Ok(modified) = modified.parse() else {
                    current = None;
                    continue;
                };
                let path = PathBuf::from(path);
                out.insert(
                    path.clone(),
                    Listed {
                        modified,
                        files: Vec::new(),
                        folders: Vec::new(),
                    },
                );
                current = Some(path);
            }
            (Some("F"), Some(name), _) => {
                if let Some(listed) = current.as_ref().and_then(|path| out.get_mut(path)) {
                    listed.files.push(name.to_string());
                }
            }
            (Some("S"), Some(name), _) => {
                if let Some(listed) = current.as_ref().and_then(|path| out.get_mut(path)) {
                    listed.folders.push(name.to_string());
                }
            }
            _ => {}
        }
    }
    out
}

fn write_cache(file: &Path, cache: &DirCache) {
    let mut text = String::new();
    for (path, listed) in cache {
        let usable = |name: &String| !name.contains(['\t', '\n']);
        if path.to_string_lossy().contains(['\t', '\n']) {
            continue;
        }
        text.push_str(&format!("D\t{}\t{}\n", listed.modified, path.display()));
        for name in listed.files.iter().filter(|name| usable(name)) {
            text.push_str(&format!("F\t{name}\n"));
        }
        for name in listed.folders.iter().filter(|name| usable(name)) {
            text.push_str(&format!("S\t{name}\n"));
        }
    }
    let temporary = file.with_extension("tmp");
    let written = std::fs::create_dir_all(file.parent().unwrap_or(Path::new(".")))
        .and_then(|()| std::fs::write(&temporary, text))
        .and_then(|()| std::fs::rename(&temporary, file));
    if let Err(error) = written {
        log::warn!(
            "audio: could not keep the music folder cache {}: {error}",
            file.display()
        );
    }
}

impl Reader for Disk {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        let file = self.cache_file(root);
        let old = file.as_deref().map(read_cache).unwrap_or_default();
        let mut new = DirCache::new();
        let mut found = Vec::new();
        // The root's own failure is the answer; a subfolder that will not read is
        // skipped and logged, so one bad entry does not lose the rest.
        walk(root, 0, &mut found, &old, &mut new)?;
        if let Some(file) = file {
            write_cache(&file, &new);
        }
        Ok(found)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }
}

fn modified_nanos(dir: &Path) -> io::Result<u128> {
    let time = std::fs::metadata(dir)?.modified()?;
    Ok(time
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos()))
}

fn list_dir(dir: &Path) -> io::Result<Listed> {
    let modified = modified_nanos(dir)?;
    let mut listed = Listed {
        modified,
        files: Vec::new(),
        folders: Vec::new(),
    };
    for entry in std::fs::read_dir(dir)?.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if kind.is_dir() {
            listed.folders.push(name);
        } else if is_song(Path::new(&name)) {
            listed.files.push(name);
        }
    }
    listed.files.sort();
    listed.folders.sort();
    Ok(listed)
}

fn walk(
    dir: &Path,
    depth: usize,
    found: &mut Vec<PathBuf>,
    old: &DirCache,
    new: &mut DirCache,
) -> io::Result<()> {
    let listed = match modified_nanos(dir) {
        Ok(modified) => match old.get(dir) {
            Some(kept) if kept.modified == modified => Ok(kept.clone()),
            _ => list_dir(dir),
        },
        Err(error) => Err(error),
    };
    let listed = match listed {
        Ok(listed) => listed,
        Err(error) if depth == 0 => return Err(error),
        Err(error) => {
            log::warn!("audio: skipping music folder {}: {error}", dir.display());
            return Ok(());
        }
    };
    found.extend(listed.files.iter().map(|name| dir.join(name)));
    new.insert(dir.to_path_buf(), listed.clone());
    if depth < MAX_DEPTH {
        for name in &listed.folders {
            if found.len() >= MAX_FILES {
                break;
            }
            walk(&dir.join(name), depth + 1, found, old, new)?;
        }
    }
    Ok(())
}

/// Whether `path` names a file this can decode.
#[must_use]
pub fn is_song(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
}

/// Where a folder's scan stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// The worker has not answered yet.
    Scanning,
    /// The folder was walked: how many songs it holds.
    Ready(usize),
    /// The worker has not answered within [`SCAN_DEADLINE`]: a hung mount.
    TimedOut,
    /// The folder will not read, and why.
    Failed(String),
}

#[derive(Debug)]
enum State {
    Scanning,
    Ready(Vec<PathBuf>),
    Failed(String),
}

/// One folder's scan, shared by every clone.
#[derive(Debug, Clone)]
pub struct FolderScan {
    started: Instant,
    deadline: Duration,
    state: Arc<Mutex<State>>,
}

impl FolderScan {
    /// Starts walking `root` with `reader` on a worker and returns at once.
    #[must_use]
    pub fn start(reader: Arc<dyn Reader>, root: PathBuf) -> Self {
        Self::start_within(reader, root, SCAN_DEADLINE)
    }

    /// [`Self::start`] with its own `deadline`.
    #[must_use]
    pub fn start_within(reader: Arc<dyn Reader>, root: PathBuf, deadline: Duration) -> Self {
        let state = Arc::new(Mutex::new(State::Scanning));
        let scan = Self {
            started: Instant::now(),
            deadline,
            state: Arc::clone(&state),
        };
        let name = "music-folder";
        let task = oag_thread::Task::spawn(name, move || {
            let _work = crate::probe::work("a music folder's scan");
            let began = Instant::now();
            let answer = match reader.list(&root) {
                Ok(mut files) => {
                    files.sort();
                    log::info!(
                        "audio: music folder {} lists {} songs in {} ms",
                        root.display(),
                        files.len(),
                        began.elapsed().as_millis()
                    );
                    State::Ready(files)
                }
                Err(error) => {
                    log::warn!(
                        "audio: music folder {} will not read: {error}",
                        root.display()
                    );
                    State::Failed(error.to_string())
                }
            };
            if let Ok(mut held) = oag_thread::lock(&state) {
                *held = answer;
            }
        });
        // Detached on purpose: see the module comment. A thread that would not
        // start reads as finished with the state still `Scanning`.
        if task.is_finished()
            && let Ok(mut held) = oag_thread::lock(&scan.state)
            && matches!(*held, State::Scanning)
        {
            *held = State::Failed("the scan thread would not start".to_string());
        }
        scan
    }

    /// Where the scan stands. Never waits on the worker.
    #[must_use]
    pub fn status(&self) -> Status {
        let Ok(held) = oag_thread::lock(&self.state) else {
            return Status::Failed("the scan panicked".to_string());
        };
        match &*held {
            State::Scanning if self.started.elapsed() >= self.deadline => Status::TimedOut,
            State::Scanning => Status::Scanning,
            State::Ready(files) => Status::Ready(files.len()),
            State::Failed(why) => Status::Failed(why.clone()),
        }
    }

    /// The songs found, in path order; empty until [`Status::Ready`].
    #[must_use]
    pub fn files(&self) -> Vec<PathBuf> {
        match oag_thread::lock(&self.state).as_deref() {
            Ok(State::Ready(files)) => files.clone(),
            _ => Vec::new(),
        }
    }
}

/// Reads and decodes one song of a folder. Runs on a music worker.
///
/// # Errors
///
/// A file that is gone or will not decode; the caller logs the reason and the
/// song stays out of the run, so a mount that vanished mid-race costs the next
/// song, never the one already playing.
pub fn load(reader: &dyn Reader, path: &Path) -> anyhow::Result<oag_audio::Sound> {
    use anyhow::Context;
    let _work = crate::probe::work("a music folder song's read and decode");
    let bytes = reader
        .read(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let pcm =
        oag_music::mp3::decode(&bytes).with_context(|| format!("decoding {}", path.display()))?;
    oag_audio::Sound::new(pcm.samples, pcm.channels, pcm.sample_rate)
        .with_context(|| path.display().to_string())
}
