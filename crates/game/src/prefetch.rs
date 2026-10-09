//! Converting every transcodable asset up front, on a worker thread.
//!
//! Both caches are filled **lazily, per asset, on first use** - a `.PMF` when
//! the screen that plays it opens, an `.at3` when the track that needs it
//! starts - which is why a cold boot takes about half a second rather than ten
//! minutes. That is the right default and it stays the default. This module is
//! the other option, taken only when `--prefetch` asks for it: pay the whole
//! bill once, in the background, and never pay any of it again.
//!
//! # What it costs, measured on this workspace
//!
//! | | |
//! | --- | --- |
//! | One front-end `.at3` on demand | 77 ms |
//! | All 93 ATRAC3+ streams | 5.5 s, producing 0.70 GB of PCM |
//! | `Intro.PMF` alone, 1200 frames | 57 s, at 48 ms a frame |
//! | All ~13,000 PSP movie frames | about 10 minutes |
//!
//! **Video is ~99% of it**, and the per-frame figure is the one to trust rather
//! than a whole-clip average taken from the wrong clip: the 260-frame dev/pub
//! reel is a near-static logo card and encodes at 6 ms a frame, eight times
//! faster than real footage. Timing the prefetch off that reel says the job is
//! a minute long; it is not.
//!
//! # Why a worker thread
//!
//! Ten minutes of `ffmpeg` on the thread that draws is ten minutes of frozen
//! window, and [ADR-0010](../../../docs/architecture/adr/0010-movie-decode-thread.md)
//! already settled that argument for the decode side - its own "what else would
//! benefit from this" list names `movie::run_ffmpeg` explicitly. The same
//! reasoning applies here and the same shape answers it: one worker owning its
//! own archive handles, talking to whoever spawned it through a snapshot of a
//! [`Progress`] and nothing else.
//!
//! Nothing here is reachable from the simulation, and nothing the simulation
//! reads depends on when a conversion finished - the game goes on playing
//! lazily while this runs, hitting the same caches. See
//! `docs/architecture/determinism.md`.
//!
//! # Resumability is free, and is kept that way
//!
//! Both caches are keyed by content: [`oag_music::at3`] on an FNV-1a of the RIFF
//! bytes, [`crate::movie`] on the entry's name hash and size. So an interrupted
//! run loses only the asset that was in flight, and the next one picks up
//! whatever finished. The planning pass below checks for each cache file
//! *before* counting the asset, so the total a caller sees is what is left to
//! do rather than what exists.
//!
//! **With one exception, named because the total would otherwise be quietly
//! wrong**: the PS2's four loose movies are always counted. A cache name is
//! built from the file's path and its length, and getting a length out of a
//! disc image without reading the file means walking the ISO directory - which
//! [`oag_assets::read_loose_file`] does not expose, and which is a second copy of
//! its path matching to write for four files. So they are visited every run,
//! and [`movie::open`] returns each one from the cache in milliseconds. The 115
//! PSP assets, which is where the ten minutes actually is, are checked exactly.
//!
//! # What is enumerated
//!
//! | Source | What is converted |
//! | --- | --- |
//! | PSP | The 93 RIFF-wrapped ATRAC3+ streams and the 22 `.PMF` movies, across `Data.wad`, `FE.wad`, `FEData.wad` and `BEData.wad` |
//! | PS2 | The four loose movies under `DATA/MOVIES/`: `INTRO512.PSS`, `INTRO640.PSS`, `BG512.IPF`, `BG640.IPF` |
//!
//! **The PS2's audio is deliberately absent**, and that is not an omission:
//! `PS2MUSIC.WAD` is raw interleaved `s16le` at 48 kHz (see
//! `docs/formats/ps2-audio.md`) and `PRERACE.WAD` is unparsed but is not a
//! codec this workspace shells out for. Neither goes near `ffmpeg`, so there is
//! nothing to convert and nothing to cache. The worker says so rather than
//! silently listing four items on a disc that has 585 MiB of sound on it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use log::{info, warn};
use oag_formats::wad::Compression;
use oag_pulse as pulse;

use crate::movie::{self, Extent};

mod manifest;
use manifest::Owned;

/// Where to read from and where the two caches are.
#[derive(Debug, Clone)]
pub struct Options {
    /// A disc image, or a directory extracted with `oag-unpack`. The same
    /// string `boot::Options::source` carries: the worker opens its own
    /// handles on it rather than borrowing anyone else's.
    pub source: String,
    /// Where converted movie frames land - [`oag_source::cache::default_cache_dir`].
    pub movies: PathBuf,
    /// Where decoded PCM lands - [`oag_source::cache::default_audio_cache_dir`].
    pub audio: PathBuf,
    /// Convert every movie again even when the cache already holds it, and
    /// overwrite what is there. See [`movie::Decode::refresh`].
    ///
    /// **Only the pictures.** The planning pass still skips the sounds it finds
    /// cached, so a refreshed run is the movie conversion over again and not the
    /// ATRAC3+ one - which is what the flag says on the tin, and is also the
    /// half that is worth ten minutes rather than five seconds.
    pub refresh_video: bool,
}

/// How far the conversion has got.
///
/// A snapshot, handed out by value: a caller polls it from whatever loop it
/// already has, and holds no lock while it draws. A GPU loading screen is the
/// eventual reader; the text lines the worker prints are the one that exists
/// today.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    /// Still walking the archives and checking the caches, so [`Self::total`]
    /// is not yet meaningful. The walk reads all 93 ATRAC3+ streams to hash
    /// them, so it is a second or two rather than instant.
    pub planning: bool,
    /// Assets that still needed converting when the walk finished. Already
    /// cached ones are **not** counted here; they are in [`Self::cached`] -
    /// except the PS2's four loose movies, which are always counted for the
    /// reason the module documentation gives.
    pub total: usize,
    /// Assets converted so far, successfully or not.
    pub done: usize,
    /// Assets the planning pass found already in the cache and skipped.
    pub cached: usize,
    /// Assets whose conversion failed. Never fatal - see [`run`].
    pub failed: usize,
    /// What is converting right now, by name where the disc has one and by
    /// name hash where it does not.
    pub current: Option<String>,
    /// Nothing left to do, for any reason including having been asked to stop.
    pub finished: bool,
    /// The stage a race load has reached, `0` before the first or when
    /// nothing reports one. Only a race load sets it: see
    /// `race::LoadWorker::progress`.
    pub load_stage: u8,
}

impl From<oag_raceplay::LoadProgress> for Progress {
    /// A race load's snapshot, uncounted: `total` stays zero, so
    /// [`crate::loading::Screen::draw_list`] draws no figures, and the stage
    /// rides in [`Progress::load_stage`].
    fn from(load: oag_raceplay::LoadProgress) -> Self {
        Self {
            current: load.current,
            finished: load.finished,
            load_stage: load.stage,
            ..Self::default()
        }
    }
}

impl Progress {
    /// Assets accounted for out of the ones that needed doing, as a fraction
    /// from 0.0 to 1.0. `1.0` before planning finishes, because nothing is
    /// outstanding until something has been counted.
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            return 1.0;
        }
        self.done as f32 / self.total as f32
    }
}

/// A running conversion, and the handle onto its progress.
///
/// Dropping this asks the worker to stop but does **not** wait for it: the
/// worker is usually blocked inside a 57-second `ffmpeg`, and a `Drop` that
/// joined would turn every early return in `main` - a bad `--menu` file, a
/// screenshot that will not write - into a ten-minute silence with no
/// explanation. [`Self::join`] is the deliberate wait, called where finishing
/// is actually what is wanted.
#[derive(Debug)]
pub struct Prefetch {
    progress: Arc<Mutex<Progress>>,
    /// Read between assets, never during one. `ffmpeg` is a child process and
    /// interrupting it partway would leave the truncated output the cache
    /// already knows how to reject, so stopping is a decision taken at the
    /// boundary.
    stop: Arc<AtomicBool>,
    /// `None` once [`Self::join`] has taken it.
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Prefetch {
    /// Starts converting, and returns immediately.
    #[must_use]
    pub fn spawn(options: Options) -> Self {
        let progress = Arc::new(Mutex::new(Progress {
            planning: true,
            ..Progress::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = std::thread::Builder::new()
            // Named for the same reason `movie-decode` is: it should be obvious
            // in `top` which thread is spending ten minutes in `ffmpeg`.
            .name("prefetch".to_string())
            .spawn({
                let progress = Arc::clone(&progress);
                let stop = Arc::clone(&stop);
                move || run(&options, &progress, &stop)
            })
            // A machine that cannot start a thread cannot run the game either,
            // matching `movie::Feed::spawn`.
            .expect("spawning the prefetch thread");

        Self {
            progress,
            stop,
            worker: Some(worker),
        }
    }

    /// How far it has got, right now.
    #[must_use]
    pub fn progress(&self) -> Progress {
        self.lock().clone()
    }

    /// Whether there is nothing left to do.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.lock().finished
    }

    /// Waits for the conversion to finish, however long that takes.
    ///
    /// Called at the exits where the run is over and the work should not be
    /// thrown away half done. Returns at once if it has already finished, and
    /// on a second call.
    pub fn join(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }

    /// The progress, with a poisoned lock treated as a lost worker.
    ///
    /// Same rule as [`movie::Feed`]'s: a panicked worker means no more
    /// conversions, not a reason to bring the caller down with it. The last
    /// snapshot it wrote is still a true statement about what got done.
    fn lock(&self) -> std::sync::MutexGuard<'_, Progress> {
        self.progress
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Drop for Prefetch {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// One asset to convert, and everything needed to convert it.
///
/// A `Sound` carries its bytes because the planning pass had to read them to
/// hash them - the cache key *is* the content, so there is no cheaper question
/// to ask. A movie carries only where to find itself, because the planning pass
/// answered "already cached?" from the WAD directory and a 2048-byte header and
/// never touched the 40 MiB behind it.
#[derive(Debug)]
enum Task {
    /// A RIFF-wrapped ATRAC3+ stream, read.
    Sound { label: String, riff: Vec<u8> },
    /// A `.PMF` in one of the PSP archives, to be read when its turn comes.
    Movie {
        label: String,
        archive: usize,
        index: usize,
        key: String,
    },
    /// A PS2 movie sitting loose on the disc's own filesystem.
    Loose { label: String, path: &'static str },
}

impl Task {
    fn label(&self) -> &str {
        match self {
            Self::Sound { label, .. } | Self::Movie { label, .. } | Self::Loose { label, .. } => {
                label
            }
        }
    }
}

/// Plans, then converts, reporting as it goes.
///
/// **Never fatal, at any step.** A source that will not open, an `ffmpeg` that
/// is not installed, one movie that fails to convert: each is a line on stdout
/// naming what is missing, and the game goes on running lazily beside it. That
/// is the same degradation a missing `ffmpeg` already gets from
/// [`crate::movie`] and [`oag_music::at3`] - a black picture and silence rather
/// than a failed boot - and prefetching is strictly an optimisation, so failing
/// a run over it would be worse than not having it.
fn run(options: &Options, progress: &Mutex<Progress>, stop: &AtomicBool) {
    let finish = |progress: &Mutex<Progress>| {
        let mut state = lock(progress);
        state.planning = false;
        state.current = None;
        state.finished = true;
    };

    if let Some(missing) = missing_tool() {
        warn!(
            "prefetch: {missing} is not on PATH, so nothing can be converted ahead of time; \
             the game still runs, converting what it can when it needs it"
        );
        finish(progress);
        return;
    }

    info!("prefetch: reading the archives to see what needs converting");
    let started = web_time::Instant::now();
    let mut planned = match plan(options) {
        Ok(planned) => planned,
        Err(e) => {
            warn!("prefetch: nothing converted ({e:#})");
            finish(progress);
            return;
        }
    };
    for note in &planned.notes {
        info!("prefetch: {note}");
    }

    let total = planned.tasks.len();
    let cached = planned.cached;
    {
        let mut state = lock(progress);
        state.planning = false;
        state.total = total;
        state.cached = cached;
    }
    info!(
        "prefetch: {total} asset(s) to convert, {cached} already cached, after {:.1} s of planning",
        started.elapsed().as_secs_f32()
    );

    let mut failed = 0usize;
    for at in 0..total {
        if stop.load(Ordering::Relaxed) {
            info!("prefetch: stopping after {at} of {total}; what finished stays cached");
            break;
        }
        let label = planned.tasks[at].label().to_string();
        lock(progress).current = Some(label.clone());
        info!("prefetch: [{}/{total}] {label}", at + 1);

        if let Err(e) = convert(&planned.tasks[at], &mut planned.archives, options) {
            failed += 1;
            warn!("prefetch: {label} failed: {e:#}");
        }

        let mut state = lock(progress);
        state.done = at + 1;
        state.failed = failed;
    }

    let done = lock(progress).done;
    info!(
        "prefetch: {done} converted, {cached} already cached, {failed} failed, in {:.1} s",
        started.elapsed().as_secs_f32()
    );
    match manifest::write(&options.movies, &options.source, &planned.owned) {
        Ok(Some(path)) => info!("prefetch: wrote the cache manifest {}", path.display()),
        Ok(None) => {}
        Err(e) => warn!("prefetch: could not write the cache manifest ({e})"),
    }
    finish(progress);
}

/// Converts one asset through the path that already knows how.
///
/// Nothing here transcodes: [`oag_music::at3::ensure_cached`] and
/// [`movie::open`] are the same calls the lazy path makes, which is what
/// guarantees the file this writes is the file the game later reads. Both
/// re-check the cache themselves, so the planning pass being wrong about an
/// asset costs a wasted read rather than a wrong answer.
fn convert(task: &Task, archives: &mut [oag_assets::Archive], options: &Options) -> Result<()> {
    match task {
        Task::Sound { riff, .. } => {
            oag_music::at3::ensure_cached(riff, &options.audio)?;
            Ok(())
        }
        Task::Movie {
            archive,
            index,
            key,
            ..
        } => {
            let blob = archives
                .get_mut(*archive)
                .context("the archive this movie was planned from is gone")?
                .read(*index)?;
            report_picture(&movie::open(
                &blob,
                key,
                &options.movies,
                Extent::Whole,
                movie::Decode {
                    refresh: options.refresh_video,
                    // Always, and this is not a preference. The GStreamer path
                    // writes no cache file - it decodes into memory and hands
                    // back frames - so a prefetch that took it would report
                    // every movie converted, leave the cache empty, and plan the
                    // same movies again next run. See `Decode::prefer_cache`.
                    prefer_cache: true,
                    ..movie::Decode::default()
                },
                // The worker prints its progress rather than drawing it, and it
                // already names each asset as it starts one. A per-frame report
                // would be a second progress display on the same stdout.
                None,
            )?)
        }
        Task::Loose { path, .. } => {
            // Read whole, unlike the WAD movies: a loose file's cache key is
            // its path and its length, and reaching a PS2 movie at all goes
            // through `read_loose_file`, which reads it whole anyway.
            let Some((found, blob)) = oag_assets::read_loose_file(&options.source, &[path])? else {
                anyhow::bail!("{path} is no longer on the source");
            };
            let key = format!("{}-{}", found.replace(['/', '\\'], "_"), blob.len());
            report_picture(&movie::open(
                &blob,
                &key,
                &options.movies,
                Extent::Whole,
                movie::Decode {
                    refresh: options.refresh_video,
                    // Always, and this is not a preference. The GStreamer path
                    // writes no cache file - it decodes into memory and hands
                    // back frames - so a prefetch that took it would report
                    // every movie converted, leave the cache empty, and plan the
                    // same movies again next run. See `Decode::prefer_cache`.
                    prefer_cache: true,
                    ..movie::Decode::default()
                },
                // The worker prints its progress rather than drawing it, and it
                // already names each asset as it starts one. A per-frame report
                // would be a second progress display on the same stdout.
                None,
            )?)
        }
    }
}

/// Turns a movie that came back without a picture into an error.
///
/// [`movie::open`] reports a failed transcode as an `Ok` movie carrying a
/// [`Movie::no_picture_reason`](movie::Movie::no_picture_reason), because for
/// the front end that is a degradation and not a failure. For a prefetch it is
/// exactly a failure: nothing was cached, and a run that printed "22 movies
/// converted" having converted none would be worse than useless.
fn report_picture(movie: &movie::Movie) -> Result<()> {
    match &movie.no_picture_reason {
        Some(reason) => anyhow::bail!("{reason}"),
        None => Ok(()),
    }
}

/// What the walk found: everything the conversion pass needs and nothing else.
#[derive(Debug)]
struct Plan {
    /// Opened once here so the conversion pass does not reopen them, and so a
    /// `Task::Movie` can name an archive by index rather than by specifier.
    archives: Vec<oag_assets::Archive>,
    /// What is left to do, in the order it will be done.
    tasks: Vec<Task>,
    /// How many assets were already in a cache and are therefore not in
    /// [`Self::tasks`].
    cached: usize,
    /// Lines worth printing about what was deliberately not listed, or about an
    /// archive that would not open.
    notes: Vec<String>,
    /// Every cache file this source owns, for [`manifest`].
    owned: Owned,
}

/// Walks the source and works out what is left to do.
fn plan(options: &Options) -> Result<Plan> {
    let layout = oag_assets::Layout::resolve(&options.source, oag_pulse::TITLE)
        .with_context(|| format!("resolving the archives in {}", options.source))?;

    let mut archives = Vec::new();
    let mut notes = Vec::new();
    for spec in archive_specs(&layout) {
        match oag_assets::Archive::open(&spec) {
            Ok(archive) => archives.push(archive),
            // A source is allowed not to carry every archive: `FEData.wad` and
            // `BEData.wad` are derived by name below rather than resolved, and
            // an extracted directory may hold only some of them.
            Err(e) => notes.push(format!("skipping {spec}: {e}")),
        }
    }

    let names = KnownNames::build();
    let cached_movies = existing(&options.movies);
    let mut tasks = Vec::new();
    let mut cached = 0usize;
    let mut owned = Owned::default();

    for (at, archive) in archives.iter_mut().enumerate() {
        let short = short_label(archive.label()).to_string();
        let entries: Vec<(usize, u32, u32)> = archive
            .directory()
            .entries
            .iter()
            .enumerate()
            // Every blob in every shipped PSP archive is stored as-is, and the
            // PS2's are LZSS throughout and hold neither container. Skipping
            // the compressed ones is what stops the peek below from falling
            // back to a full decompressed read of all 7,200 of them.
            .filter(|(_, entry)| entry.compression == Compression::None)
            .map(|(index, entry)| (index, entry.name_hash, entry.size_uncompressed))
            .collect();

        for (index, hash, size) in entries {
            let Ok(head) = archive.peek(index, 12) else {
                continue;
            };

            if head.starts_with(oag_video::pmf::MAGIC) {
                let key = format!("{hash:08x}-{size}");
                owned.movie_keys.push(key.clone());
                // The question is not asked at all under `--refresh-video`: the
                // point of that flag is to convert the ones that *are* cached,
                // so counting them as `cached` and skipping them would leave it
                // with nothing to do.
                if !options.refresh_video && movie_is_cached(archive, index, &key, &cached_movies) {
                    cached += 1;
                    continue;
                }
                tasks.push(Task::Movie {
                    label: format!("{short} {}", names.of(hash)),
                    archive: at,
                    index,
                    key,
                });
            } else if head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WAVE") {
                // Read rather than peeked, because the cache key is an FNV-1a
                // of the whole file: there is no cheaper question to ask than
                // the one that needs all the bytes. 46 MiB across the 93 of
                // them, and the ones that turn out to need converting keep
                // their blob so the conversion pass does not read them twice.
                let Ok(blob) = archive.read(index) else {
                    continue;
                };
                owned
                    .sound_files
                    .extend(oag_music::at3::cache_file(&blob, &options.audio));
                if oag_music::at3::is_cached(&blob, &options.audio) {
                    cached += 1;
                    continue;
                }
                tasks.push(Task::Sound {
                    label: format!("{short} {}", names.of(hash)),
                    riff: blob,
                });
            }
        }
    }

    if layout.platform == oag_assets::Platform::Ps2 {
        // Listed unconditionally, unlike everything above: see the module
        // documentation for why a loose file's cache name cannot be built
        // without reading it. `movie::open` returns a cached one at once, so a
        // resumed run pays four reads rather than four transcodes.
        for path in PS2_MOVIES {
            tasks.push(Task::Loose {
                label: (*path).to_string(),
                path,
            });
        }
        notes.push(format!(
            "{} and {} hold raw PCM rather than a codec, so there is nothing to convert in \
             either",
            pulse::archives::ps2::MUSIC,
            pulse::archives::ps2::PRERACE
        ));
    }

    Ok(Plan {
        archives,
        tasks,
        cached,
        notes,
        owned,
    })
}

/// The PS2 release's movies, which are loose on the disc rather than in a WAD.
///
/// All four cuts, not the two a boot plays: `oag_game::boot`'s `LOOSE_MOVIES`
/// prefers the 60 Hz `640` pair, where the original picks between the pairs on
/// `g_refresh_mode` - the player's own answer to a first-boot question, not a
/// region (`docs/ghidra/functions/ps2-pulse-eu/refresh-mode.md`). A prefetch
/// that converted only one pair would leave the other to a lazy transcode in
/// the middle of a boot, so both pairs are converted; that is the point of
/// prefetching, and it is what makes the preference cheap to revisit.
///
/// Matched by trailing path components through
/// [`oag_assets::read_loose_file`], so the disc's serial-named directory - `54748/`
/// on this pressing - does not have to be spelled out. See
/// `docs/ps2/pulse-disc-layout.md`.
const PS2_MOVIES: &[&str] = &[
    "DATA/MOVIES/INTRO512.PSS",
    "DATA/MOVIES/INTRO640.PSS",
    "DATA/MOVIES/BG512.IPF",
    "DATA/MOVIES/BG640.IPF",
];

/// Whether a `.PMF` entry's converted frames are already in the cache, answered
/// without reading the movie.
///
/// The cache file's name carries the entry key, the picture size and the frame
/// count - see [`movie::cache_name`] - and the first two come off the WAD
/// directory and the PSMF header's first 2048 bytes. The frame count does not:
/// [`movie::open`] counts H.264 access units in the demuxed stream, where the
/// header only states a duration, and the two can legitimately differ by one
/// frame. So three names are tried around the header's figure.
///
/// **A tolerance of one is the same tolerance `open_psmf` already warns
/// outside of**: it prints a warning when the access-unit count and the
/// duration's implied count disagree by more than one frame, so a movie where
/// this check could go wrong is one the loader has already complained about.
///
/// Being wrong here is cheap either way. A false miss costs one read of a movie
/// [`movie::open`] then finds cached and returns immediately. A false hit costs
/// the prefetch nothing and leaves that one movie to the lazy path, which
/// re-checks the exact name.
fn movie_is_cached(
    archive: &mut oag_assets::Archive,
    index: usize,
    key: &str,
    existing: &[String],
) -> bool {
    let Ok(head) = archive.peek(index, oag_video::pmf::HEADER_LEN as u64) else {
        return false;
    };
    let Ok(header) = oag_video::pmf::Header::parse(&head) else {
        return false;
    };
    let Some(video) = header.video else {
        return false;
    };
    let (width, height) = (u32::from(video.width), u32::from(video.height));
    let expected = header.expected_frame_count();

    (expected.saturating_sub(1)..=expected.saturating_add(1))
        .filter_map(|n| usize::try_from(n).ok())
        .map(|n| movie::cache_name(key, width, height, Some(n)))
        .any(|name| existing.contains(&name))
}

/// The file names already in a cache directory.
///
/// Listed once rather than `stat`ed per candidate: the movie check asks three
/// questions about each of 22 movies, and a directory that does not exist yet
/// is an empty list rather than an error - the ordinary first-run case.
fn existing(cache_dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Every archive worth walking, in the order they are opened.
///
/// [`oag_assets::Layout`] resolves the two archives the game itself reads - the bulk
/// one and its companion - because those are the two anything else needs.
/// `FEData.wad` and `BEData.wad` are neither, and between them they hold five
/// of the disc's 22 movies and one of its 93 sounds, so a prefetch that skipped
/// them would leave a lazy transcode behind for no reason.
///
/// They are derived from the bulk archive's own specifier rather than resolved,
/// because a specifier is either `<image>:<path>` or a plain path and replacing
/// the trailing file name is correct for both. Only on a PSP source: the PS2's
/// bulk archive is `WADS2.WAD` and the derivation would produce a name no disc
/// has, which would open as nothing and be reported as a skip for no reason.
fn archive_specs(layout: &oag_assets::Layout) -> Vec<String> {
    let mut specs = layout.patch.clone();
    specs.push(layout.data.clone());
    specs.extend(layout.fe.clone());
    specs.extend(layout.extra.iter().cloned());

    if layout.platform == oag_assets::Platform::Psp
        && let Some(stem) = layout.data.strip_suffix("Data.wad")
    {
        specs.push(format!("{stem}FEData.wad"));
        specs.push(format!("{stem}BEData.wad"));
    }
    specs
}

/// An archive specifier's last path component, for a progress line.
///
/// `data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/Data.wad` is accurate and
/// unreadable at one line per asset; `Data.wad` is what tells them apart.
fn short_label(spec: &str) -> &str {
    spec.rsplit(['/', '\\', ':']).next().unwrap_or(spec)
}

/// The entry names this project has recovered, by hash.
///
/// A WAD directory stores only hashes, so most of what the worker converts can
/// only be named `hash:71d3c1ec`. These few can do better, and they are the
/// ones a reader is most likely to be waiting on: the intro is 1200 frames of
/// the ten minutes on its own.
///
/// Every name here is evidenced elsewhere rather than guessed - the two movies
/// in [`pulse::names`], the eight music tracks from the `%d` template at
/// `0x08a88e94` that `oag_sound` documents.
#[derive(Debug)]
struct KnownNames(Vec<(u32, String)>);

impl KnownNames {
    fn build() -> Self {
        let mut names = vec![
            pulse::names::INTRO_MOVIE.to_string(),
            pulse::names::BACKDROP_MOVIE.to_string(),
        ];
        names.extend((1..=8).map(|n| format!(r"Data\Music\FEMusic\frontend{n}.at3")));
        Self(
            names
                .into_iter()
                .map(|name| (oag_formats::wad::hash_name(&name), name))
                .collect(),
        )
    }

    fn of(&self, hash: u32) -> String {
        self.0
            .iter()
            .find(|(known, _)| *known == hash)
            .map_or_else(|| format!("hash:{hash:08x}"), |(_, name)| name.clone())
    }
}

/// Which of the two tools is missing, if either.
///
/// Probed once rather than discovered 115 times: without `ffmpeg` every single
/// asset fails with the same message, and 115 copies of it bury the one line
/// that matters. `ffprobe` ships beside `ffmpeg` but is a separate binary and a
/// separate failure - a PS2 source needs it to read a `.PSS`'s own width,
/// height and frame rate, which that container carries no header for.
fn missing_tool() -> Option<&'static str> {
    for tool in ["ffmpeg", "ffprobe"] {
        match std::process::Command::new(tool).arg("-version").output() {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Some(tool),
            _ => {}
        }
    }
    None
}

fn lock(progress: &Mutex<Progress>) -> std::sync::MutexGuard<'_, Progress> {
    progress
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two archives the game reads plus the two it does not, and only on
    /// the platform whose bulk archive is spelled `Data.wad`. Both forms of
    /// specifier, because a disc spec carries a colon the plain path does not.
    #[test]
    fn a_psp_source_walks_all_four_of_its_archives() {
        let layout = oag_assets::Layout {
            platform: oag_assets::Platform::Psp,
            data: "image.chd:PSP_GAME/USRDIR/Data.wad".to_string(),
            fe: Some("image.chd:PSP_GAME/USRDIR/FE.wad".to_string()),
            patch: Vec::new(),
            extra: Vec::new(),
            serial: None,
        };
        assert_eq!(
            archive_specs(&layout),
            [
                "image.chd:PSP_GAME/USRDIR/Data.wad",
                "image.chd:PSP_GAME/USRDIR/FE.wad",
                "image.chd:PSP_GAME/USRDIR/FEData.wad",
                "image.chd:PSP_GAME/USRDIR/BEData.wad",
            ]
        );

        let extracted = oag_assets::Layout {
            data: "extracted/PSP_GAME/USRDIR/Data.wad".to_string(),
            fe: None,
            ..layout
        };
        assert_eq!(
            archive_specs(&extracted),
            [
                "extracted/PSP_GAME/USRDIR/Data.wad",
                "extracted/PSP_GAME/USRDIR/FEData.wad",
                "extracted/PSP_GAME/USRDIR/BEData.wad",
            ]
        );
    }

    /// The PS2's bulk archive is `WADS2.WAD`, and deriving siblings off it
    /// would name files no pressing has.
    #[test]
    fn a_ps2_source_derives_no_siblings() {
        let layout = oag_assets::Layout {
            platform: oag_assets::Platform::Ps2,
            data: "image.chd:54748/WADS2.WAD".to_string(),
            fe: Some("image.chd:54748/WADSP.WAD".to_string()),
            patch: Vec::new(),
            extra: Vec::new(),
            serial: None,
        };
        assert_eq!(
            archive_specs(&layout),
            ["image.chd:54748/WADS2.WAD", "image.chd:54748/WADSP.WAD"]
        );
    }

    #[test]
    fn a_label_is_the_archive_rather_than_the_whole_specifier() {
        assert_eq!(
            short_label("data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/Data.wad"),
            "Data.wad"
        );
        assert_eq!(short_label("Data.wad"), "Data.wad");
    }

    /// The two movies and the eight music tracks resolve to their real names,
    /// and everything else to the hash that addresses it - which is the only
    /// honest thing to print for an entry nobody has named.
    #[test]
    fn known_entries_are_named_and_the_rest_are_hashes() {
        let names = KnownNames::build();
        assert_eq!(names.of(0x71d3_c1ec), pulse::names::INTRO_MOVIE);
        assert_eq!(
            names.of(oag_formats::wad::hash_name(
                r"Data\Music\FEMusic\frontend1.at3"
            )),
            r"Data\Music\FEMusic\frontend1.at3"
        );
        assert_eq!(names.of(0x3d2c_85f8), "hash:3d2c85f8");
    }

    /// Nothing outstanding reads as complete rather than as a division by
    /// zero, which is what a loading screen polling this before the walk
    /// finishes would otherwise get.
    #[test]
    fn progress_of_nothing_is_complete() {
        assert_eq!(Progress::default().fraction(), 1.0);
        let half = Progress {
            total: 4,
            done: 2,
            ..Progress::default()
        };
        assert_eq!(half.fraction(), 0.5);
    }

    /// A cache directory that does not exist yet is the ordinary first run.
    #[test]
    fn an_absent_cache_directory_lists_nothing() {
        assert!(existing(Path::new("no/such/cache")).is_empty());
    }
}
