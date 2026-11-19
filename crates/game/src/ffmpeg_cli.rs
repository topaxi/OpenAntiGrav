//! `--fetch-ffmpeg`: the download the front end offers, run from the command line.
//!
//! Naming the flag is the consent, so nothing is asked. It opens no window and no audio and
//! exits 0 once an `ffmpeg` is in place (the player's own, or the pinned build it just
//! installed) and 1 when the download failed (`docs/architecture/ffmpeg.md`).

use anyhow::{Result, bail};
use oag_ffmpeg::{Pin, Progress, Tool};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// Fetches the pinned build under `root` unless a usable `ffmpeg` is already found.
pub(crate) fn run(root: &std::path::Path, use_pinned: bool) -> Result<()> {
    if let Some(found) = oag_ffmpeg::resolve(Tool::Ffmpeg)
        && (!use_pinned || found.starts_with(root))
    {
        println!(
            "ffmpeg is already available at {}; nothing to download",
            found.display()
        );
        return Ok(());
    }
    let Some(pin) = Pin::current() else {
        bail!(
            "no ffmpeg build is pinned for this OS and architecture; install ffmpeg with your \
             package manager (docs/overview/installing.md)"
        );
    };
    println!(
        "downloading ffmpeg {} ({} MB, {}) from {}",
        pin.version,
        pin.megabytes(),
        pin.license,
        pin.url
    );
    let progress = Arc::new(Progress::default());
    progress.total.store(pin.size, Ordering::Relaxed);
    let shared = Arc::clone(&progress);
    let root = root.to_path_buf();
    let mut task = oag_thread::Task::spawn("ffmpeg-download", move || {
        oag_ffmpeg::install(pin, &root, &shared)
    });
    let mut last = Instant::now();
    while !task.is_finished() {
        std::thread::sleep(Duration::from_millis(200));
        if last.elapsed() >= Duration::from_secs(2) {
            last = Instant::now();
            println!(
                "  {} of {} MB",
                progress.done.load(Ordering::Relaxed) / 1_000_000,
                progress.total.load(Ordering::Relaxed) / 1_000_000
            );
        }
    }
    match task.join() {
        Some(Ok(Ok(dir))) => {
            println!("installed in {}", dir.display());
            Ok(())
        }
        Some(Ok(Err(why))) => bail!("the ffmpeg download failed: {why:#}"),
        _ => bail!("the ffmpeg download stopped unexpectedly"),
    }
}
