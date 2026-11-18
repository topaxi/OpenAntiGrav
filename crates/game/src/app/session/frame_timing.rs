//! Where a frame's input sample sits against its swapchain acquire.
//!
//! Input is sampled once per tick inside the frame's tick loop
//! (`Controls::player_snapshots`, which polls the pads there), and under a
//! blocking present mode (`Vsync::On`, or `smooth` falling back to Fifo) the
//! acquire is where the loop waits for a free image. Input read before that
//! wait is that much older by the time the image is drawn. This counts, per
//! second of window frames, how long the acquire waited and how far the
//! frame's first input sample sat before (positive) or after (negative) the
//! acquire returned. The window's own numbers, logged at `debug`; not an
//! input-to-photon measurement, which needs a probe at the glass.

use std::time::Duration;

use web_time::Instant;

/// How often a summary is due.
const WINDOW: Duration = Duration::from_secs(1);

/// One window of frames, summed.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Summary {
    /// Frames that acquired an image.
    pub(crate) frames: u32,
    /// Mean seconds the acquire waited.
    pub(crate) mean_wait: f32,
    /// Frames that sampled input at all (a frame that ran no tick did not).
    pub(crate) sampled: u32,
    /// Of those, how many sampled before the acquire returned.
    pub(crate) sampled_before: u32,
    /// Mean seconds from the first input sample to the acquire returning:
    /// positive when input was read before the wait, negative after it.
    pub(crate) mean_lead: f32,
}

/// The current frame's readings and the window they fold into.
#[derive(Debug)]
pub(crate) struct FrameTiming {
    acquired: Option<(Instant, f32)>,
    sampled: Option<Instant>,
    started: Option<Instant>,
    frames: u32,
    wait_sum: f32,
    sampled_frames: u32,
    sampled_before: u32,
    lead_sum: f32,
}

impl Default for FrameTiming {
    fn default() -> Self {
        Self {
            acquired: None,
            sampled: None,
            started: None,
            frames: 0,
            wait_sum: 0.0,
            sampled_frames: 0,
            sampled_before: 0,
            lead_sum: 0.0,
        }
    }
}

impl FrameTiming {
    /// The acquire started at `start` and returned an image at `end`.
    pub(crate) fn acquired(&mut self, start: Instant, end: Instant) {
        self.acquired = Some((end, end.duration_since(start).as_secs_f32()));
    }

    /// Input was sampled at `at`; only the frame's first sample counts.
    pub(crate) fn sampled(&mut self, at: Instant) {
        self.sampled.get_or_insert(at);
    }

    /// Folds the frame in, and hands back a summary once a window has passed.
    /// A frame that acquired nothing (a lost surface, a skipped draw) is left
    /// out, so the means are over frames that drew.
    pub(crate) fn end_frame(&mut self, now: Instant) -> Option<Summary> {
        let started = *self.started.get_or_insert(now);
        let sampled = self.sampled.take();
        if let Some((acquired_at, wait)) = self.acquired.take() {
            self.frames += 1;
            self.wait_sum += wait;
            if let Some(sampled_at) = sampled {
                self.sampled_frames += 1;
                let lead = if sampled_at <= acquired_at {
                    self.sampled_before += 1;
                    acquired_at.duration_since(sampled_at).as_secs_f32()
                } else {
                    -sampled_at.duration_since(acquired_at).as_secs_f32()
                };
                self.lead_sum += lead;
            }
        }
        if now.duration_since(started) < WINDOW || self.frames == 0 {
            return None;
        }
        let summary = Summary {
            frames: self.frames,
            mean_wait: self.wait_sum / self.frames as f32,
            sampled: self.sampled_frames,
            sampled_before: self.sampled_before,
            mean_lead: if self.sampled_frames == 0 {
                0.0
            } else {
                self.lead_sum / self.sampled_frames as f32
            },
        };
        *self = Self {
            started: Some(now),
            ..Self::default()
        };
        Some(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn input_read_before_the_wait_leads_it_and_after_trails_it() {
        let t0 = Instant::now();
        let mut timing = FrameTiming::default();
        assert_eq!(timing.end_frame(t0), None, "the window has only begun");
        // Sampled at +1 ms, then an acquire that waited from +2 to +10 ms.
        timing.sampled(t0 + ms(1));
        timing.acquired(t0 + ms(2), t0 + ms(10));
        assert_eq!(timing.end_frame(t0 + ms(12)), None);
        // Acquired first (+20 to +24 ms), sampled after at +26 ms, twice.
        timing.acquired(t0 + ms(20), t0 + ms(24));
        timing.sampled(t0 + ms(26));
        timing.sampled(t0 + ms(30));
        let summary = timing
            .end_frame(t0 + ms(1_001))
            .expect("a second has passed");
        assert_eq!(summary.frames, 2);
        assert_eq!(summary.sampled, 2);
        assert_eq!(summary.sampled_before, 1);
        assert!((summary.mean_wait - 0.006).abs() < 1e-6);
        // (+9 ms) and (-2 ms), averaged.
        assert!((summary.mean_lead - 0.0035).abs() < 1e-6);
    }

    #[test]
    fn a_frame_that_acquired_nothing_is_not_counted() {
        let t0 = Instant::now();
        let mut timing = FrameTiming::default();
        timing.end_frame(t0);
        timing.sampled(t0 + ms(1));
        timing.end_frame(t0 + ms(5));
        timing.acquired(t0 + ms(6), t0 + ms(7));
        let summary = timing.end_frame(t0 + ms(1_100)).expect("due");
        assert_eq!(summary.frames, 1);
        assert_eq!(summary.sampled, 0, "the skipped frame's sample was dropped");
    }
}
