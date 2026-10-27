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
    /// This frame's own marks, for the spike line.
    frame: [f32; 9],
    /// `OAG_PHASE_SPIKE_MS`: a frame whose phases total more than this prints its own line.
    spike_ms: Option<f32>,
}

impl PhaseClock {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            last: None,
            sums: [0.0; 9],
            frames: 0,
            frame: [0.0; 9],
            spike_ms: std::env::var("OAG_PHASE_SPIKE_MS")
                .ok()
                .and_then(|text| text.parse().ok()),
        }
    }

    /// Starts a frame's measurement at the current instant.
    pub(crate) fn begin(&mut self) {
        if self.enabled {
            self.last = Some(Instant::now());
            self.frame = [0.0; 9];
        }
    }

    /// Charges the time since the previous mark to `phase`.
    pub(crate) fn mark(&mut self, phase: Phase) {
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        if let Some(last) = self.last.replace(now) {
            let seconds = now.duration_since(last).as_secs_f32();
            self.sums[phase as usize] += seconds;
            self.frame[phase as usize] += seconds;
        }
    }

    /// Counts a finished frame. `tick` labels the spike line, when this frame earns one.
    pub(crate) fn end_frame(&mut self, tick: u64) {
        if !self.enabled {
            return;
        }
        self.frames += 1;
        if let Some(line) = self.spike_line(tick) {
            println!("{line}");
        }
    }

    /// This frame's own phases as one line, when their total passes `OAG_PHASE_SPIKE_MS`.
    fn spike_line(&self, tick: u64) -> Option<String> {
        let total: f32 = self.frame.iter().sum::<f32>() * 1000.0;
        if !self.spike_ms.is_some_and(|limit| total > limit) {
            return None;
        }
        let mut line = format!("frame-spike tick={tick}");
        for (name, seconds) in NAMES.iter().zip(self.frame) {
            line.push_str(&format!(" {name}={:.2}", seconds * 1000.0));
        }
        Some(format!("{line} total={total:.2}"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_over_the_spike_limit_prints_its_own_phases_and_one_under_prints_nothing() {
        let mut clock = PhaseClock::new(true);
        clock.spike_ms = Some(10.0);
        clock.frame[Phase::Prep as usize] = 0.0125;
        clock.frame[Phase::Scene as usize] = 0.0005;
        assert_eq!(
            clock.spike_line(6957).as_deref(),
            Some(
                "frame-spike tick=6957 ticks=0.00 prep=12.50 setup=0.00 scene=0.50 \
                 compose=0.00 overlay=0.00 submit=0.00 readback=0.00 present=0.00 total=13.00"
            )
        );
        clock.frame = [0.001; 9];
        assert_eq!(clock.spike_line(1), None);
        clock.spike_ms = None;
        clock.frame[Phase::Prep as usize] = 1.0;
        assert_eq!(clock.spike_line(1), None);
    }
}
