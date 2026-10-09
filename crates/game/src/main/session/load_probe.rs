//! `--measure-race-load`: launching races straight from the menus and timing
//! every main-thread frame of the loading screen's hand-off to each.
//!
//! The recording itself is [`oag_present::perf::transition::Recorder`]; this is
//! only where the window's frame loop feeds it and what drives the run - the
//! boot sequence skipped, `LAUNCH RACE` pressed on the first menu frame, the
//! race left to run for [`RACE_FRAMES`] and then escaped from, as many times
//! as asked.

use web_time::{Duration, Instant};

use log::debug;

use oag_present::perf::transition::{FRAME_BUDGET, Frame, Recorder};

use crate::stage::Stage;

use super::Session;

/// How many race frames each transition is followed into: three seconds at
/// 60 Hz. **Chosen, not measured** - long enough to cover the first frames a
/// new scene draws, which is where a driver that defers a pipeline compile to
/// first use would show.
const RACE_FRAMES: u32 = 180;

/// Where one `--measure-race-load` run has got to.
pub(crate) struct LoadProbe {
    runs_left: u32,
    run: u32,
    measuring: bool,
    race_frames: u32,
    recorder: Recorder,
    frame_start: Instant,
}

impl LoadProbe {
    /// A probe that will measure `runs` transitions; `None` for zero.
    pub(crate) fn new(runs: u32) -> Option<Self> {
        (runs > 0).then(|| Self {
            runs_left: runs,
            run: 0,
            measuring: false,
            race_frames: 0,
            recorder: Recorder::default(),
            frame_start: Instant::now(),
        })
    }

    /// Records a named span of main-thread work, when a transition is being
    /// measured.
    pub(crate) fn span(&mut self, label: &'static str, took: Duration) {
        if self.measuring {
            self.recorder.span(label, took);
        }
    }
}

fn stage_name(stage: &Stage) -> &'static str {
    match stage {
        Stage::Launcher(_) => "launcher",
        Stage::Loading(_) => "loading",
        Stage::Frontend(_) => "frontend",
        Stage::Menu(_) => "menu",
        Stage::Race(_) => "race",
    }
}

impl Session {
    /// Records a span on the probe, when there is one. A no-op otherwise.
    pub(crate) fn probe_span(&mut self, label: &'static str, took: Duration) {
        if let Some(probe) = self.load_probe.as_mut() {
            probe.span(label, took);
        }
    }

    /// The top of a frame: start its clock, and launch the next race if the
    /// menus are up and a run is still owed.
    pub(crate) fn probe_begin_frame(&mut self) {
        let Some(probe) = self.load_probe.as_mut() else {
            return;
        };
        probe.frame_start = Instant::now();
        if probe.measuring || !matches!(self.stage, Stage::Menu(_)) {
            return;
        }
        probe.measuring = true;
        probe.run += 1;
        probe.race_frames = 0;
        probe.recorder = Recorder::default();
        debug!("--measure-race-load: run {} launching", probe.run);
        let start = Instant::now();
        if let Err(e) = self.launch_race() {
            log::error!("--measure-race-load: cannot launch a race: {e:#}");
            self.quit = true;
            return;
        }
        self.probe_span("launch_race (loading screen built)", start.elapsed());
    }

    /// The bottom of a frame: record it, and end the run once the race has
    /// been on screen for [`RACE_FRAMES`].
    pub(crate) fn probe_end_frame(&mut self, interval: Duration) {
        let stage = stage_name(&self.stage);
        let Some(probe) = self.load_probe.as_mut() else {
            return;
        };
        if !probe.measuring {
            return;
        }
        probe.recorder.frame(Frame {
            stage,
            interval,
            work: probe.frame_start.elapsed(),
        });
        if stage != "race" {
            return;
        }
        probe.race_frames += 1;
        if probe.race_frames < RACE_FRAMES {
            return;
        }
        // Printed as well as logged, so a run with the log filtered down to
        // warnings still says what it found.
        for line in probe.recorder.report(FRAME_BUDGET) {
            debug!("{line}");
            println!("{line}");
        }
        if let Some(worst) = probe.recorder.worst() {
            println!(
                "race load run {}: worst main-thread frame {:.1} ms ({})",
                probe.run,
                worst.work.as_secs_f64() * 1000.0,
                worst.stage
            );
        }
        probe.measuring = false;
        probe.runs_left = probe.runs_left.saturating_sub(1);
        if probe.runs_left == 0 {
            self.quit = true;
        } else {
            self.escape();
        }
    }
}
