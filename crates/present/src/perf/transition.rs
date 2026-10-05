//! The load-to-race transition, timed frame by frame on the main thread.
//!
//! [`super::Meter`] deliberately drops every frame that carries a load - see
//! the `stalled` flag on the composition root's session - because a load is
//! not a frame time for the overlay's purposes. That is exactly the frame a
//! player sees freeze, so this is the other instrument: it keeps every frame
//! between "launch the race" and a few seconds into it, stalled or not, and
//! says which of them went over a frame's budget and what stage was on screen
//! when they did.
//!
//! Fed durations rather than reading a clock itself, for the same reason
//! [`super::Meter::record`] is: nothing here reaches the simulation, and a
//! pure function of a fed sequence is what its tests can drive. The window's
//! `--measure-race-load` is the one production caller.

use std::time::Duration;

/// One frame at 60 Hz: 16.7 ms.
///
/// **Chosen, not measured.** The rate `docs/architecture/determinism.md`
/// fixes the timestep at, and the rate a Steam Deck's own panel runs the
/// loading screen at; a frame longer than this is a frame the loading screen's
/// wave did not advance on.
pub const FRAME_BUDGET: Duration = Duration::from_micros(16_667);

/// One frame of the transition, as the main thread saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    /// Which stage was on screen: `"menu"`, `"loading"` or `"race"`.
    pub stage: &'static str,
    /// From the start of the previous frame to the start of this one - what a
    /// player sees as the gap between two pictures.
    pub interval: Duration,
    /// This frame's own main-thread work: the stage update, the ticks and the
    /// draw, from the top of the frame to after its present.
    pub work: Duration,
}

/// Every frame of one transition, and the named spans inside it.
#[derive(Debug, Clone, Default)]
pub struct Recorder {
    frames: Vec<Frame>,
    spans: Vec<(&'static str, Duration)>,
}

impl Recorder {
    /// Records one frame.
    pub fn frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    /// Records a named span of main-thread work, such as the loading screen's
    /// own build or the hand-off to the race.
    pub fn span(&mut self, label: &'static str, took: Duration) {
        self.spans.push((label, took));
    }

    /// Every frame recorded, in order.
    #[must_use]
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// The frame with the longest [`Frame::work`].
    ///
    /// Work rather than interval: the interval also carries the compositor's
    /// wait for vblank and whatever else the machine was doing, which is not
    /// this thread's to answer for. Both are reported by [`Self::report`].
    #[must_use]
    pub fn worst(&self) -> Option<Frame> {
        self.frames.iter().copied().max_by_key(|frame| frame.work)
    }

    /// How many frames' work went over `budget`.
    #[must_use]
    pub fn over(&self, budget: Duration) -> usize {
        self.frames
            .iter()
            .filter(|frame| frame.work > budget)
            .count()
    }

    /// The table `--measure-race-load` prints: one row per stage, then every
    /// frame over `budget`, then every named span.
    #[must_use]
    pub fn report(&self, budget: Duration) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "race load transition: {} frames, {} over {:.1} ms of work",
            self.frames.len(),
            self.over(budget),
            ms(budget)
        ));
        let mut stages: Vec<&'static str> = Vec::new();
        for frame in &self.frames {
            if !stages.contains(&frame.stage) {
                stages.push(frame.stage);
            }
        }
        for stage in stages {
            let of: Vec<&Frame> = self.frames.iter().filter(|f| f.stage == stage).collect();
            let worst_work = of.iter().map(|f| f.work).max().unwrap_or_default();
            let worst_interval = of.iter().map(|f| f.interval).max().unwrap_or_default();
            let over = of.iter().filter(|f| f.work > budget).count();
            lines.push(format!(
                "  {stage:<8} {:>5} frames, worst work {:>8.1} ms, worst interval {:>8.1} ms, {over} over budget",
                of.len(),
                ms(worst_work),
                ms(worst_interval),
            ));
        }
        for (index, frame) in self.frames.iter().enumerate() {
            if frame.work > budget {
                lines.push(format!(
                    "  over budget: frame {index} ({}) work {:.1} ms, interval {:.1} ms",
                    frame.stage,
                    ms(frame.work),
                    ms(frame.interval)
                ));
            }
        }
        for (label, took) in &self.spans {
            lines.push(format!("  span {label}: {:.1} ms", ms(*took)));
        }
        lines
    }
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(stage: &'static str, work_ms: u64) -> Frame {
        Frame {
            stage,
            interval: Duration::from_millis(work_ms + 1),
            work: Duration::from_millis(work_ms),
        }
    }

    #[test]
    fn the_worst_frame_is_the_longest_work_not_the_longest_interval() {
        let mut recorder = Recorder::default();
        recorder.frame(frame("loading", 3));
        recorder.frame(Frame {
            stage: "loading",
            interval: Duration::from_millis(900),
            work: Duration::from_millis(2),
        });
        recorder.frame(frame("race", 40));
        assert_eq!(recorder.worst().map(|f| f.stage), Some("race"));
        assert_eq!(recorder.over(FRAME_BUDGET), 1);
    }

    #[test]
    fn the_report_names_every_stage_every_overrun_and_every_span() {
        let mut recorder = Recorder::default();
        recorder.frame(frame("menu", 2));
        recorder.frame(frame("loading", 250));
        recorder.frame(frame("race", 4));
        recorder.span("hand-off", Duration::from_millis(5));
        let report = recorder.report(FRAME_BUDGET).join("\n");
        assert!(report.contains("3 frames, 1 over 16.7 ms"), "{report}");
        for needle in [
            "menu",
            "loading",
            "race",
            "frame 1 (loading)",
            "span hand-off",
        ] {
            assert!(report.contains(needle), "{needle} missing from {report}");
        }
    }

    #[test]
    fn an_empty_transition_reports_no_worst_frame() {
        assert_eq!(Recorder::default().worst(), None);
    }
}
