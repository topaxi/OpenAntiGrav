//! The reflection rung: REFLECTION RESOLUTION `auto`, the glass and sea target stepped from
//! the original's size to `half` and `quarter` by its own timed cost.
//!
//! The target is a separately encoded pass with its own GPU time (the overlay's `GLASS` row),
//! and on a tile-based phone it is the largest single thing in an HD frame: the S21 pays 10 to
//! 17 ms for Vineta K's at render scale 25, and a fixed `quarter` takes its median frame from
//! 41.9 to 24.9 ms (`docs/tools/android.md`, "What `quarter` buys on Vineta K"). A desktop GPU
//! pays about 0.12 ms for the same target (`docs/rendering/water.md`, "The reflection target's
//! resolution"). So the rung is decided by measured time alone, never by platform.
//!
//! # The signals
//!
//! Fed once a frame ([`Reading`]): the glass pass's seconds on a frame drawn at the step in
//! force, the frame's wall-clock seconds, the frame's timed GPU seconds, and the budget's
//! period. All three are smoothed ([`SMOOTHING_SECONDS`]).
//!
//! - **Missing the budget is read off the wall clock**, not off [`super::Cost`]: the glass is not
//!   in the scalable cost the extent loop divides, and on the S21 the scene pass reads 9.5 ms in
//!   a 42 ms frame, so only the wall clock sees the miss. A paced loop (vsync, a frame limit)
//!   reads at or a little above its period, so a miss is a smoothed frame over [`OVER`] of it.
//! - **The share is the glass over that frame**, with an absolute [`FLOOR_SECONDS`] under it: a
//!   desktop at a 144 Hz or 240 Hz budget can miss with a 0.12 ms target in a 6 ms frame, and a
//!   target that cheap is not what to give up.
//! - **A restore is predicted off the GPU time**, which a paced wall clock cannot show headroom
//!   in: the timed GPU frame less the glass now plus the glass one step up (learned, as the blur
//!   rung learns its saving) must land at or under [`RESTORE_FIT`] of the budget, with the wall
//!   clock not over, for the blur rung's [`super::RESTORE_SECONDS`] running and the same
//!   relapse backoff ([`super::patience`]).
//!
//! # Before the render scale
//!
//! While this rung still has a step to shed and wants to, or is waiting out the cooldown of one
//! it just took, [`super::Controller::record`] holds the extent where it is rather than falling
//! ([`Rung::holds_extent`]). With the share under the threshold it never holds, so the extent
//! loop runs exactly as it did before the rung existed. A restore waits for the extent to be at
//! its ceiling, as the blur rung's does: reflections come back last.
//!
//! # The player's choice
//!
//! Only `auto` moves; a fixed choice is drawn as it is. Like the extent, the step is runtime
//! state: this module imports no settings and writes none. All thresholds are **chosen, not
//! measured** against more than the S21 and the desktop numbers above; `docs/rendering/water.md`
//! ("REFLECTION RESOLUTION `auto`") has the runs they were checked against.

use oag_display::display::ReflectionResolution;

use super::patience::Patience;

/// A smoothed wall-clock frame over this share of the budget's period misses it. Above 1.0 so a
/// paced loop that holds its rate (a frame a hair over the period) is not a miss.
pub const OVER: f32 = 1.1;

/// The glass pass's share of the frame above which giving a step up is worth the coarser view.
/// Under the S21's `half` share on Vineta K (4.9 ms of a 30.3 ms frame, 16 %), so a frame that
/// still misses at half goes on to quarter rather than hovering at the edge of the threshold.
pub const SHARE: f32 = 0.10;

/// The glass pass's cost under which it is never stepped down, whatever its share.
pub const FLOOR_SECONDS: f32 = 0.001;

/// How long a step down must be wanted before it is taken.
pub const SHED_SECONDS: f32 = 0.5;

/// How long nothing is decided after a step, while the smoothed times settle at the new size.
pub const COOLDOWN_SECONDS: f32 = 1.0;

/// A restore must predict the timed GPU frame at or under this share of the budget.
pub const RESTORE_FIT: f32 = 0.8;

/// The time constant of the smoothed times.
pub const SMOOTHING_SECONDS: f32 = 0.25;

/// The smallest believed cost of a step over the one above it: a step has a quarter of the
/// pixels, so it cannot cost less than a quarter, and clamping keeps a reading taken while the
/// scene changed from multiplying a prediction by more than four.
const MIN_STEP_RATIO: f32 = 0.25;

/// One frame's reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    /// The glass pass's GPU seconds on a frame drawn at the step in force, or `None` when no
    /// such reading came back this frame.
    pub glass: Option<f32>,
    /// The frame's timed GPU seconds (every timed pass, the glass included), or `None`.
    pub gpu: Option<f32>,
    /// The frame's wall-clock seconds.
    pub frame: f32,
    /// The budget's period in seconds, or `None` when nothing paces the frame and there is no
    /// budget to miss: the rung is then inert.
    pub period: Option<f32>,
}

/// A change of step, for the one log line each earns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Change {
    /// The step before.
    pub from: ReflectionResolution,
    /// The step after.
    pub to: ReflectionResolution,
    /// The smoothed glass pass at the change, seconds.
    pub glass: f32,
    /// The smoothed wall-clock frame at the change, seconds.
    pub frame: f32,
    /// The budget, seconds.
    pub period: f32,
}

/// An exponential mean over wall-clock time.
#[derive(Debug, Clone, Copy, Default)]
struct Smoothed(Option<f32>);

impl Smoothed {
    fn add(&mut self, value: f32, dt: f32) {
        if !(value.is_finite() && value >= 0.0) {
            return;
        }
        self.0 = Some(match self.0 {
            None => value,
            Some(mean) => {
                let alpha = 1.0 - (-dt.max(0.0) / SMOOTHING_SECONDS).exp();
                mean + (value - mean) * alpha
            }
        });
    }
}

/// The rung's state.
#[derive(Debug, Clone, Default)]
pub struct Rung {
    /// Index into [`ReflectionResolution::STEPS`]: 0 is the original's size.
    level: usize,
    glass: Smoothed,
    frame: Smoothed,
    gpu: Smoothed,
    /// Seconds a step down has been wanted, running.
    wanted: f32,
    /// Seconds still to wait after a step.
    cooldown: f32,
    /// Whether the last frame wanted a step down.
    wants: bool,
    /// The smoothed glass on the frame a step down was taken, until the step below has read
    /// its own and learned the ratio.
    before: Option<f32>,
    /// Each step's cost over the one above it, by the lower step's level, once learned.
    ratio: [Option<f32>; ReflectionResolution::STEPS.len()],
    patience: Patience,
}

impl Rung {
    /// A rung at the original's size, having measured nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The step to draw at under `auto`.
    #[must_use]
    pub fn step(&self) -> ReflectionResolution {
        ReflectionResolution::STEPS[self.level]
    }

    /// Whether the extent loop should hold rather than fall: this rung still has a step to
    /// shed and wants it, or is settling after one it shed.
    #[must_use]
    pub fn holds_extent(&self) -> bool {
        let floor = ReflectionResolution::STEPS.len() - 1;
        (self.wants && self.level < floor) || (self.cooldown > 0.0 && self.before.is_some())
    }

    /// Records one frame and returns the change of step it made, if any. `at_ceiling` is
    /// whether the extent is at its ceiling (always, with no resolution target): a restore
    /// waits for it.
    pub fn record(&mut self, reading: Reading, at_ceiling: bool) -> Option<Change> {
        let Some(period) = reading.period.filter(|p| p.is_finite() && *p > 0.0) else {
            self.wants = false;
            self.wanted = 0.0;
            return None;
        };
        let dt = if reading.frame.is_finite() && reading.frame > 0.0 {
            reading.frame
        } else {
            return None;
        };
        self.patience.begin_frame(period);
        self.frame.add(reading.frame, dt);
        if let Some(glass) = reading.glass {
            self.glass.add(glass, dt);
        }
        if let Some(gpu) = reading.gpu {
            self.gpu.add(gpu, dt);
        }
        if self.cooldown > 0.0 {
            self.cooldown -= dt;
            return None;
        }
        let (Some(glass), Some(frame)) = (self.glass.0, self.frame.0) else {
            self.wants = false;
            return None;
        };
        if let Some(before) = self.before.take()
            && before > 0.0
        {
            self.ratio[self.level] = Some((glass / before).clamp(MIN_STEP_RATIO, 1.0));
        }
        let over = frame > OVER * period;
        self.wants = over && glass >= FLOOR_SECONDS && glass >= SHARE * frame;
        let floor = ReflectionResolution::STEPS.len() - 1;
        if self.wants && self.level < floor {
            self.wanted += dt;
            if self.wanted < SHED_SECONDS {
                return None;
            }
            self.patience.shed();
            self.before = Some(glass);
            return Some(self.move_to(self.level + 1, glass, frame, period));
        }
        self.wanted = 0.0;
        if self.level == 0 {
            return None;
        }
        let up = glass / self.ratio[self.level].unwrap_or(MIN_STEP_RATIO);
        let fits = at_ceiling
            && !over
            && self
                .gpu
                .0
                .is_some_and(|gpu| gpu - glass + up <= RESTORE_FIT * period);
        if !self.patience.ready(fits) {
            return None;
        }
        Some(self.move_to(self.level - 1, glass, frame, period))
    }

    fn move_to(&mut self, level: usize, glass: f32, frame: f32, period: f32) -> Change {
        let from = self.step();
        self.level = level;
        self.wanted = 0.0;
        self.cooldown = COOLDOWN_SECONDS;
        // The smoothed glass belongs to the step it was drawn at.
        self.glass = Smoothed::default();
        self.gpu = Smoothed::default();
        Change {
            from,
            to: self.step(),
            glass,
            frame,
            period,
        }
    }

    /// A stall (a load): the runs and the cooldown start again and the smoothed times are
    /// dropped; the step and what was learned stay, as the extent's scale does.
    pub fn stall(&mut self) {
        self.glass = Smoothed::default();
        self.frame = Smoothed::default();
        self.gpu = Smoothed::default();
        self.wanted = 0.0;
        self.wants = false;
        self.cooldown = COOLDOWN_SECONDS;
        self.patience.reset_run();
    }

    /// A new circuit: back to the original's size with nothing learned, since a different
    /// target (a sea's, a glass tunnel's) costs differently.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests;
