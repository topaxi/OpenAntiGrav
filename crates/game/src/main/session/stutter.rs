//! The `frame: N ms` stutter warn, rate-limited: the first slow frame is logged
//! at once, and while frames keep running slow, one summary per window follows.
//!
//! The first line stays immediate because it is the one that pairs with an
//! audio underrun in the same terminal. A machine that is slow every frame
//! (software Vulkan, 5-20 fps) would otherwise print a line per frame.

use std::time::Duration;

use web_time::Instant;

/// Past any ordinary jitter at 60 Hz (two and a half frames) and well under the
/// load stalls `Session::stalled` already excludes.
const THRESHOLD: Duration = Duration::from_millis(40);

/// How long slow frames are counted before they are summarised in one line.
const WINDOW: Duration = Duration::from_secs(5);

/// Counts slow frames between summaries.
#[derive(Debug, Default)]
pub(crate) struct Stutter {
    /// When the current window opened; `None` while no window is open, so the
    /// next slow frame is logged on its own line at once.
    since: Option<Instant>,
    /// Slow frames in the current window, not counting the one that opened it.
    count: u32,
    /// The slowest of those.
    worst: Duration,
}

impl Stutter {
    /// Takes every measured frame, slow or not, and returns the line to warn
    /// with, if any. Fast frames are needed too: they are what closes a window
    /// whose slow frames have stopped.
    pub(crate) fn observe(&mut self, now: Instant, elapsed: Duration) -> Option<String> {
        let slow = elapsed > THRESHOLD;
        let Some(start) = self.since else {
            if !slow {
                return None;
            }
            self.since = Some(now);
            return Some(format!("frame: {:.1} ms", millis(elapsed)));
        };
        if slow {
            self.count += 1;
            self.worst = self.worst.max(elapsed);
        }
        let open_for = now.duration_since(start);
        if open_for < WINDOW {
            return None;
        }
        if self.count == 0 {
            self.since = None;
            return None;
        }
        let line = format!(
            "frame: {} more over {} ms in the last {:.1} s, worst {:.1} ms",
            self.count,
            THRESHOLD.as_millis(),
            open_for.as_secs_f32(),
            millis(self.worst),
        );
        self.since = Some(now);
        self.count = 0;
        self.worst = Duration::ZERO;
        Some(line)
    }
}

fn millis(duration: Duration) -> f32 {
    duration.as_secs_f32() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAST: Duration = Duration::from_millis(16);

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn fast_frames_log_nothing() {
        let mut stutter = Stutter::default();
        let t0 = Instant::now();
        for i in 0..1000 {
            assert_eq!(stutter.observe(t0 + ms(16 * i), FAST), None);
        }
    }

    #[test]
    fn the_first_slow_frame_is_logged_at_once() {
        let mut stutter = Stutter::default();
        let t0 = Instant::now();
        assert_eq!(stutter.observe(t0, FAST), None);
        assert_eq!(stutter.observe(t0 + ms(16), ms(41)), Some("frame: 41.0 ms".into()));
    }

    #[test]
    fn exactly_the_threshold_is_not_slow() {
        let mut stutter = Stutter::default();
        assert_eq!(stutter.observe(Instant::now(), THRESHOLD), None);
    }

    #[test]
    fn a_slow_machine_gets_one_summary_per_window() {
        let mut stutter = Stutter::default();
        let t0 = Instant::now();
        let mut lines = Vec::new();
        // 90 s at 10 fps, every frame 100 ms, the slowest 250 ms.
        for i in 0..900 {
            let elapsed = if i == 7 { ms(250) } else { ms(100) };
            lines.extend(stutter.observe(t0 + ms(100 * i), elapsed));
        }
        assert_eq!(lines.len(), 1 + 17, "{lines:#?}");
        assert_eq!(lines[0], "frame: 100.0 ms");
        assert_eq!(lines[1], "frame: 50 more over 40 ms in the last 5.0 s, worst 250.0 ms");
        assert_eq!(lines[2], "frame: 50 more over 40 ms in the last 5.0 s, worst 100.0 ms");
    }

    #[test]
    fn a_quiet_window_closes_and_the_next_stutter_is_immediate_again() {
        let mut stutter = Stutter::default();
        let t0 = Instant::now();
        assert!(stutter.observe(t0, ms(90)).is_some());
        assert_eq!(stutter.observe(t0 + ms(1000), ms(60)), None);
        // The window ends on a fast frame and still reports the slow one in it.
        assert_eq!(
            stutter.observe(t0 + ms(5000), FAST),
            Some("frame: 1 more over 40 ms in the last 5.0 s, worst 60.0 ms".into())
        );
        // A window with no slow frame closes without a line.
        assert_eq!(stutter.observe(t0 + ms(10_000), FAST), None);
        assert_eq!(stutter.observe(t0 + ms(10_016), ms(70)), Some("frame: 70.0 ms".into()));
    }
}
