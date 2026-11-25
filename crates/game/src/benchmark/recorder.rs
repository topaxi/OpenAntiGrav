//! What a benchmark run keeps while it flies: one entry per drawn window frame, and the state at
//! the tick the race ended.
//!
//! Fed by the frame loop with the same `elapsed` the performance meter gets, under the same
//! rule: a frame that carried a load is not a frame time (`Session::stalled`), so it is never
//! recorded. The recorder reads no clock of its own and nothing here reaches the simulation.

use web_time::Instant;

/// One drawn frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// The wall time from the previous frame's start to this one's, in microseconds.
    pub frame_us: f64,
    /// The lap the player was on.
    pub lap: u32,
    /// Simulation ticks the frame stepped.
    pub ticks: u32,
    /// The world's tick once the frame had stepped.
    pub tick: u64,
}

/// Where the race ended, taken on the tick it ended and not a frame later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ending {
    /// The world's tick.
    pub tick: u64,
    /// `RaceSim::state_hash` at that tick.
    pub state_hash: u64,
    /// The race ended on the player's own crossing (not the tick cap).
    pub finished: bool,
}

/// A run in progress.
#[derive(Debug)]
pub struct Recorder {
    frames: Vec<Frame>,
    ending: Option<Ending>,
    started: Instant,
    load_start: String,
}

impl Recorder {
    /// A run that starts now; `load_start` is the machine's load average at this moment.
    #[must_use]
    pub fn begin(load_start: String) -> Self {
        Self {
            frames: Vec::new(),
            ending: None,
            started: Instant::now(),
            load_start,
        }
    }

    /// Records a frame. Ignored once the race has ended: the stats screen's own frames are not
    /// the race's.
    pub fn frame(&mut self, frame_seconds: f32, lap: u32, ticks: u32, tick: u64) {
        if self.ending.is_some() {
            return;
        }
        self.frames.push(Frame {
            frame_us: f64::from(frame_seconds) * 1e6,
            lap,
            ticks,
            tick,
        });
    }

    /// The race ended at `tick` with `state_hash`. Only the first call counts.
    pub fn end(&mut self, ending: Ending) {
        self.ending.get_or_insert(ending);
    }

    /// Whether the race has ended.
    #[must_use]
    pub fn ended(&self) -> bool {
        self.ending.is_some()
    }

    /// The frames so far.
    #[must_use]
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Where the race ended, once it has.
    #[must_use]
    pub fn ending(&self) -> Option<Ending> {
        self.ending
    }

    /// Seconds since the run began.
    #[must_use]
    pub fn wall_seconds(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    /// The load average when the run began.
    #[must_use]
    pub fn load_start(&self) -> &str {
        &self.load_start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_after_the_end_are_not_the_races() {
        let mut run = Recorder::begin("0 0 0".into());
        run.frame(0.016, 1, 1, 300);
        run.end(Ending {
            tick: 10,
            state_hash: 7,
            finished: true,
        });
        run.frame(0.5, 3, 1, 301);
        run.end(Ending {
            tick: 99,
            state_hash: 8,
            finished: false,
        });
        assert_eq!(run.frames().len(), 1);
        assert_eq!(run.ending().map(|e| e.tick), Some(10));
    }
}
