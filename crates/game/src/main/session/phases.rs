//! Where the frame thread's CPU goes, by phase, for `--frame-log`.
//!
//! `Session::cpu_cost` says how long a frame took on this thread; this says which stretch of
//! the frame it was. Each [`Phase`] is the wall-clock time from the previous mark to its own,
//! summed over a second of frames and printed beside the frame-log line. Off unless
//! `--frame-log` is on, so a normal run reads no clock for it.

use web_time::Instant;

/// One stretch of the frame, in the order the frame runs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// The fixed-timestep ticks and the stage update: input, simulation, audio cues.
    Ticks,
    /// End-race flow and the scene's per-frame preparation, before `draw`.
    Prep,
    /// `draw` up to the stage dispatch: the view, the encoder, target sizing, DRS limits.
    Setup,
    /// The stage's own drawing into the encoder: for a race, the scene record.
    Scene,
    /// The race's HUD and bloom composite, and the menu and upscale passes after the stage.
    Compose,
    /// The performance overlay and cursor, and the timestamp resolves.
    Overlay,
    /// `encoder.finish()` and `Queue::submit`.
    Submit,
    /// Mapping and reading last frame's timestamps, and feeding the resolution controller.
    Readback,
    /// `Queue::present`.
    Present,
}

const NAMES: [&str; 9] = [
    "ticks", "prep", "setup", "scene", "compose", "overlay", "submit", "readback", "present",
];

/// The accumulated marks of the current second.
#[derive(Debug)]
pub(crate) struct PhaseClock {
    enabled: bool,
    last: Option<Instant>,
    sums: [f32; 9],
    frames: u32,
}

impl PhaseClock {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            last: None,
            sums: [0.0; 9],
            frames: 0,
        }
    }

    /// Starts a frame's measurement at the current instant.
    pub(crate) fn begin(&mut self) {
        if self.enabled {
            self.last = Some(Instant::now());
        }
    }

    /// Charges the time since the previous mark to `phase`.
    pub(crate) fn mark(&mut self, phase: Phase) {
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        if let Some(last) = self.last.replace(now) {
            self.sums[phase as usize] += now.duration_since(last).as_secs_f32();
        }
    }

    /// Counts a finished frame.
    pub(crate) fn end_frame(&mut self) {
        if self.enabled {
            self.frames += 1;
        }
    }

    /// The mean milliseconds per frame of each phase since the last call, as `name=ms` fields,
    /// and the counters reset. `None` when disabled or no frame was counted.
    pub(crate) fn take_line(&mut self) -> Option<String> {
        if !self.enabled || self.frames == 0 {
            return None;
        }
        let per = 1000.0 / self.frames as f32;
        let total: f32 = self.sums.iter().sum::<f32>() * per;
        let mut line = String::from("frame-phases");
        for (name, sum) in NAMES.iter().zip(self.sums) {
            line.push_str(&format!(" {name}={:.2}", sum * per));
        }
        line.push_str(&format!(" total={total:.2}"));
        self.sums = [0.0; 9];
        self.frames = 0;
        Some(line)
    }
}
