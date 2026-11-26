//! HD's width throttle, the controller behind TARGET FPS `original`: the scene's width drops
//! one notch a frame while the GPU's frame runs long and comes back a notch at a time, and
//! the height never moves. The law is the title's
//! ([`oag_title::width_throttle::Law`]); see `docs/rendering/dynamic-resolution.md`,
//! "A title's own width throttle".
//!
//! It shares nothing with [`super::policy`]'s budget loop: no budget, residual, deadband,
//! cooldown or patience, because the original has none.

use oag_title::width_throttle::Law;

/// The frames' worth of the paced period a frame may span before it missed a vblank.
const MISSED_PERIODS: f32 = 1.5;

/// The law with the rates this run aims and paces at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spec {
    law: Law,
    /// The rate the law's two durations are scaled to, never below the law's own.
    threshold_hz: u32,
    /// The rate the loop is paced at, whose vblank a frame can miss.
    paced_hz: u32,
}

impl Spec {
    /// `law` on a loop held to `held_hz`, or to the law's own rate while nothing says.
    #[must_use]
    pub fn new(law: Law, held_hz: Option<u32>) -> Self {
        let paced_hz = held_hz.unwrap_or(law.frame_hz).max(1);
        Self {
            law,
            threshold_hz: paced_hz.max(law.frame_hz),
            paced_hz,
        }
    }

    /// The rate the law's durations are scaled to.
    #[must_use]
    pub fn threshold_hz(self) -> u32 {
        self.threshold_hz
    }

    /// The rate the loop is paced at.
    #[must_use]
    pub fn paced_hz(self) -> u32 {
        self.paced_hz
    }

    /// The width drawn at `level` of a target `ceiling` pixels wide.
    #[must_use]
    pub fn width(self, ceiling: u32, level: u32) -> u32 {
        let notches = u64::from(self.law.width_notches.max(1));
        let kept = notches.saturating_sub(u64::from(level.min(self.law.max_level)));
        let width = (u64::from(ceiling) * kept + notches / 2) / notches;
        u32::try_from(width).unwrap_or(ceiling).max(1)
    }

    /// The narrowest width of a target `ceiling` pixels wide.
    #[must_use]
    pub fn narrowest(self, ceiling: u32) -> u32 {
        self.width(ceiling, self.law.max_level)
    }

    /// Whether a frame that took `frame_seconds` on the wall clock missed a vblank.
    #[must_use]
    pub fn missed(self, frame_seconds: f32) -> bool {
        frame_seconds * self.paced_hz as f32 > MISSED_PERIODS
    }

    /// Which way a frame moves the level: `1` narrower, `-1` wider, `0` neither.
    ///
    /// Compared in whole microseconds against the durations scaled by `hz / frame_hz`, cross
    /// multiplied, so the law's own rate compares its own integers exactly.
    fn step_of(self, gpu_seconds: f32, late: bool) -> i8 {
        let micros = (f64::from(gpu_seconds) * 1e6) as u64;
        let scaled = micros.saturating_mul(u64::from(self.threshold_hz));
        let at = |duration: u32| u64::from(duration) * u64::from(self.law.frame_hz);
        if scaled > at(self.law.narrow_above_us) || late {
            1
        } else if scaled <= at(self.law.widen_up_to_us) {
            -1
        } else {
            0
        }
    }
}

/// The level in force and the GPU time waiting to step it.
#[derive(Debug, Clone, Default)]
pub struct Throttle {
    level: u32,
    /// The newest GPU frame time offered since the last step.
    gpu: Option<f32>,
}

impl Throttle {
    /// Notches taken off the width.
    #[must_use]
    pub fn level(&self) -> u32 {
        self.level
    }

    /// A frame's GPU time. Only the newest before a [`Self::step`] counts, so a backlog of
    /// readings drained in one frame moves the level once.
    pub fn offer(&mut self, gpu_seconds: f32) {
        if gpu_seconds.is_finite() && gpu_seconds > 0.0 {
            self.gpu = Some(gpu_seconds);
        }
    }

    /// Ends a frame: at most one notch, from the GPU time offered during it and whether the
    /// frame took long enough on the wall clock to have missed a vblank. Returns whether the
    /// level moved.
    ///
    /// With no GPU time offered nothing moves, so a device that times nothing never
    /// narrows on missed frames it could never widen from again.
    pub fn step(&mut self, frame_seconds: Option<f32>, spec: Spec) -> bool {
        let Some(gpu) = self.gpu.take() else {
            return false;
        };
        let late = frame_seconds.is_some_and(|frame| spec.missed(frame));
        let before = self.level;
        self.level = match spec.step_of(gpu, late) {
            1 => self.level.saturating_add(1),
            -1 => self.level.saturating_sub(1),
            _ => self.level,
        }
        .min(spec.law.max_level);
        self.level != before
    }
}

#[cfg(test)]
mod tests;
