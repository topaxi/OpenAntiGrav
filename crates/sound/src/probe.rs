//! Counts the music work done on each thread: every disc read, listing and
//! decode this crate does goes through [`work`].
//!
//! The point is the **frame thread**. A whole-track decode or a PSARC listing
//! there is a stalled frame (248 to 264 ms on the Steam Deck for HD's listing,
//! `docs/rendering/hd-gpu-cost.md`), and the 40 ms frame warning does not see
//! one on a frame the session marks as a load stall. So the host marks its
//! frame thread once ([`mark_frame_thread`]) and any work there is logged with
//! its duration, and a test asserts [`on_this_thread`] does not move across a
//! call (`crates/game/tests/music_resume_ground_truth.rs`).
//!
//! Nested calls count once each, the outermost alone is timed and logged.

use std::cell::Cell;
use std::time::Duration;

use web_time::Instant;

thread_local! {
    static CALLS: Cell<u32> = const { Cell::new(0) };
    static TIME: Cell<Duration> = const { Cell::new(Duration::ZERO) };
    static DEPTH: Cell<u32> = const { Cell::new(0) };
    static FRAME: Cell<bool> = const { Cell::new(false) };
}

/// The music work this thread has done so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Count {
    /// How many reads, listings and decodes started here, nested ones included.
    pub calls: u32,
    /// Wall time spent in the outermost of them.
    pub time: Duration,
}

/// What this thread has done so far. Compare two readings around a call.
#[must_use]
pub fn on_this_thread() -> Count {
    Count {
        calls: CALLS.with(Cell::get),
        time: TIME.with(Cell::get),
    }
}

/// Marks the calling thread as the one that draws frames, so music work on it
/// is logged at warn with its duration.
pub fn mark_frame_thread() {
    FRAME.with(|frame| frame.set(true));
}

/// Held for the length of one piece of music work, `what` naming it.
pub(crate) struct Work {
    what: &'static str,
    start: Instant,
}

/// Starts counting one piece of work; it ends when the guard drops.
pub(crate) fn work(what: &'static str) -> Work {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    DEPTH.with(|depth| depth.set(depth.get() + 1));
    Work {
        what,
        start: Instant::now(),
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let depth = DEPTH.with(|depth| {
            let left = depth.get().saturating_sub(1);
            depth.set(left);
            left
        });
        if depth > 0 {
            return;
        }
        let took = self.start.elapsed();
        TIME.with(|time| time.set(time.get() + took));
        if FRAME.with(Cell::get) {
            log::warn!(
                "audio: {} on the frame thread took {:.1} ms",
                self.what,
                took.as_secs_f64() * 1000.0
            );
        }
    }
}
