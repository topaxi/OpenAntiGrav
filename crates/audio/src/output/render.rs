//! The thread that renders ahead of the device.
//!
//! # Why this exists
//!
//! Until 2026-08-31 the `cpal` callback rendered the mixer itself, so the only
//! audio queued anywhere was whatever the device chose to ask for in one
//! callback - and on a modern desktop the client does not choose that number.
//! cpal's PipeWire backend turns a buffer-size request into a `node.latency`
//! property, which asks PipeWire for a *ceiling* on latency rather than a
//! floor; measured on this machine, requests of 40 ms, 85 ms and 200 ms all
//! came back as the graph's own 256-frame quantum, 5.3 ms. Meanwhile an HD race
//! on the same machine produced frames of 40 to 80 ms with a callback 25 ms
//! late behind them. A queue that small cannot cover a stall that large, and no
//! amount of asking makes it bigger.
//!
//! So this project holds the audio itself. A dedicated thread renders the mixer
//! into a lock-free ring ahead of time and the callback does nothing but copy
//! out of it. Two things follow, and both are the point:
//!
//! 1. **The tolerance is ours.** How much audio is queued is
//!    [`super::MIN_BUFFER`] and the caller's own target, not the sound
//!    server's quantum, and it is the same number on every host.
//! 2. **The audio thread never takes the mixer lock.** It was doing so with a
//!    bounded spin ([ADR-0031](../../../../docs/architecture/adr/0031-wait-briefly-for-the-mixer-lock.md)),
//!    which was correct and is now unnecessary: the frame loop and this thread
//!    contend for the mixer, and this thread can afford to block where the
//!    callback could not.
//!
//! What it costs is latency, and honestly this time: a cue is heard when the
//! ring drains to it, so the queue depth *is* the delay. That number is now a
//! decision rather than an accident.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::mixer::{CHANNELS, Mixer};

/// Output frames rendered per pass under the mixer lock.
///
/// 512, about 10 ms at 48 kHz. Small enough that the frame loop never waits
/// long for the lock - a pass over 32 voices is tens of microseconds - and
/// large enough that the loop below is not woken hundreds of times a second to
/// do nothing.
const CHUNK_FRAMES: usize = 512;

/// How long the thread sleeps when the ring is full.
///
/// Half a chunk, so the ring is topped up well before the device has drained
/// what a chunk holds. **Polling rather than parking**: waking this thread from
/// the callback would put a futex wake on the audio thread, and 200 wake-ups a
/// second of a thread that usually finds nothing to do is the cheaper of the
/// two.
const POLL: Duration = Duration::from_millis(5);

/// The render-ahead thread, stopped and joined when [`Output`] drops.
///
/// [`Output`]: super::Output
#[derive(Debug)]
pub(crate) struct Ahead {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Ahead {
    /// Starts rendering `mixer` into `ring` and holds it at `target` samples.
    ///
    /// **`target`, not "full".** The ring is one chunk larger than the target so
    /// that a write always fits, and occupancy is latency: filling to the brim
    /// would put the delay *above* the number the caller asked for rather than
    /// at it. Stopping a chunk short instead keeps it inside
    /// `(target - CHUNK_FRAMES, target]`, so the depth a caller names is a
    /// ceiling on how late a cue is heard and not a floor.
    pub(crate) fn spawn(
        mixer: Arc<Mutex<Mixer>>,
        mut ring: rtrb::Producer<f32>,
        target: usize,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::Builder::new()
            .name("oag-audio-render".to_string())
            .spawn(move || {
                let mut scratch = vec![0.0f32; CHUNK_FRAMES * CHANNELS];
                let capacity = ring.buffer().capacity();
                while !flag.load(Ordering::Relaxed) {
                    // Fill while another whole chunk still fits under the
                    // target, then sleep. A partial push would leave the ring's
                    // occupancy sawing against the chunk size for no gain.
                    while capacity - ring.slots() + scratch.len() <= target
                        && ring.slots() >= scratch.len()
                    {
                        match mixer.lock() {
                            Ok(mut mixer) => mixer.render(&mut scratch),
                            // A poisoned mixer is a panic somewhere else that
                            // this thread cannot fix; it stops rather than
                            // spinning on a lock that will never be taken.
                            Err(_) => return,
                        }
                        let (_, rest) = ring.push_partial_slice(&scratch);
                        debug_assert!(rest.is_empty(), "room was checked above");
                    }
                    std::thread::sleep(POLL);
                }
            })
            .expect("spawning the audio render thread");
        Self {
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for Ahead {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            // Joined rather than detached: the thread holds an `Arc` to the
            // mixer, and a run that exits while it is mid-render is a render
            // into a buffer whose owner has gone.
            let _ = handle.join();
        }
    }
}

/// Samples to hold ahead of the device, for a target depth in seconds.
///
/// This is the number [`Ahead::spawn`] holds the ring at, and therefore the
/// ceiling on how late a cue is heard.
pub(crate) fn target_samples(sample_rate: u32, target: Duration) -> usize {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a sample count from a caller-supplied duration"
    )]
    let frames = (sample_rate as f32 * target.as_secs_f32()) as usize;
    // At least a chunk, or the loop above can never write anything.
    (frames * CHANNELS).max(CHUNK_FRAMES * CHANNELS)
}

/// Samples the ring is built with, for a given target.
///
/// One chunk over the target, so a write always fits and the target itself is
/// reachable rather than a chunk short of the brim.
pub(crate) fn ring_capacity(sample_rate: u32, target: Duration) -> usize {
    target_samples(sample_rate, target) + CHUNK_FRAMES * CHANNELS
}
