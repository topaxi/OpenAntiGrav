//! The CPU governor: the closed loop that turns the frame thread's own extras down when the
//! CPU, not the GPU, is what keeps a frame over its budget.
//!
//! [`crate::drs`] sheds pixels when the GPU is the slow side and does nothing while the GPU is
//! idle. A CPU-bound device (a handheld, an old phone) is slow on the other side: the frame
//! thread spends its budget recording and submitting passes, and fewer pixels would not move
//! that. [`Governor`] reads the frame thread's CPU time beside the GPU time and, when the
//! CPU is over the target period while the GPU is not, steps down an ordered list of
//! CPU-heavy knobs; it restores them one at a time once the CPU has headroom to pay for them.
//!
//! Like [`crate::drs::Controller`] it measures nothing and draws nothing: it is fed one
//! `(cpu, gpu)` pair a frame and answers with a level, so every sequence can be chosen by a
//! test. The level counts how many knobs of the caller's ordered list are turned down; which
//! knob each step is, is the caller's. This module imports no settings, so it cannot write
//! one: the level is a runtime override and nothing a player saved moves.
//!
//! # How it avoids fighting resolution control
//!
//! It acts only when the smoothed GPU time is **under** [`GPU_BOUND`] of the budget. A frame
//! the GPU keeps over budget is the resolution controller's, and the governor holds (neither
//! sheds nor restores) until the GPU is back under. With no GPU reading it holds too.
//!
//! # How it avoids oscillating
//!
//! - Decisions are taken on smoothed times, and a shed needs [`SHED_SECONDS`] of them over.
//! - A shed starts a [`COOLDOWN_SECONDS`] in which nothing is decided, and the saving is read
//!   as the smoothed CPU time before the shed minus after it, per knob.
//! - A restore needs the CPU, plus the saving that knob measured, to land at or under
//!   [`RESTORE_FIT`] of the budget for [`RESTORE_SECONDS`] running, and the GPU to be clear.
//! - A shed within the relapse window of a restore doubles the next restore's patience, up to
//!   [`MAX_BACKOFF`] doublings, and each window that holds pays one doubling back.
//!
//! All the thresholds are **chosen, not measured**; `docs/rendering/cpu-governor.md` has the
//! Deck numbers they were checked against.

/// The smoothed CPU time, as a share of the budget, above which the frame is CPU-over.
pub const OVER: f32 = 1.0;

/// The smoothed GPU time, as a share of the budget, from which the GPU counts as the bound side.
pub const GPU_BOUND: f32 = 0.9;

/// How long the CPU must stay over before a knob is shed.
pub const SHED_SECONDS: f32 = 1.0;

/// How long nothing is decided after a change, so the smoothed times settle on the new level.
pub const COOLDOWN_SECONDS: f32 = 1.5;

/// A restore must predict the CPU time at or under this share of the budget.
pub const RESTORE_FIT: f32 = 0.8;

/// How long a restore must be predicted to fit, before any backoff.
pub const RESTORE_SECONDS: f32 = 6.0;

/// The most doublings of the restore patience a run of relapses earns.
pub const MAX_BACKOFF: u32 = 3;

/// The time constant of the smoothed times, in seconds.
const SMOOTHING_SECONDS: f32 = 0.4;

/// What a knob is believed to save, as a share of the budget, until a shed has measured it.
const ASSUMED_SAVING: f32 = 0.1;

/// The most knobs a governor can hold down at once.
pub const MAX_LEVEL: usize = 8;

/// One frame's reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// The frame thread's CPU time this frame, in seconds, without the present wait.
    pub cpu: f32,
    /// The GPU time of a frame, in seconds, or `None` before any has been read.
    pub gpu: Option<f32>,
    /// The target frame period in seconds, or `None` when nothing paces the frame (no refresh
    /// target, no limit): there is no budget to be over, and the governor is inert.
    pub period: Option<f32>,
}

/// A change of level, for the one log line each earns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Change {
    /// The level before.
    pub from: usize,
    /// The level after.
    pub to: usize,
    /// The smoothed CPU time at the change, in seconds.
    pub cpu: f32,
    /// The smoothed GPU time at the change, in seconds, if one was read.
    pub gpu: Option<f32>,
    /// The budget, in seconds.
    pub period: f32,
}

/// The governor's state.
#[derive(Debug, Clone)]
pub struct Governor {
    level: usize,
    cpu: Option<f32>,
    gpu: Option<f32>,
    over_frames: u32,
    fit_frames: u32,
    cooldown: u32,
    /// The smoothed CPU time before the shed that set the cooldown, to learn its saving from.
    before: Option<f32>,
    /// What each level's knob saved, indexed by the level it was shed from.
    saving: [Option<f32>; MAX_LEVEL],
    backoff: u32,
    /// Frames since the last restore, while that restore has not yet proved itself.
    since_restore: Option<u32>,
}

impl Default for Governor {
    fn default() -> Self {
        Self::new()
    }
}

impl Governor {
    /// A governor at full quality with nothing learned.
    #[must_use]
    pub fn new() -> Self {
        Self {
            level: 0,
            cpu: None,
            gpu: None,
            over_frames: 0,
            fit_frames: 0,
            cooldown: 0,
            before: None,
            saving: [None; MAX_LEVEL],
            backoff: 0,
            since_restore: None,
        }
    }

    /// How many knobs of the ordered list are turned down; 0 is full quality.
    #[must_use]
    pub fn level(&self) -> usize {
        self.level
    }

    /// The smoothed CPU time in seconds, for the frame log.
    #[must_use]
    pub fn cpu(&self) -> Option<f32> {
        self.cpu
    }

    /// Back to full quality with nothing remembered: a stage change, or the governor switched off.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Feeds one frame. `steps` is how many knobs the caller can turn down right now.
    /// Returns the change of level this frame caused, if any.
    pub fn record(&mut self, sample: Sample, steps: usize) -> Option<Change> {
        let steps = steps.min(MAX_LEVEL);
        let Some(period) = sample.period.filter(|p| p.is_finite() && *p > 0.0) else {
            return self.fall_to(0, sample.period.unwrap_or(0.0));
        };
        let alpha = (period / SMOOTHING_SECONDS).clamp(0.01, 1.0);
        let cpu = smooth(&mut self.cpu, sample.cpu, alpha);
        let gpu = sample.gpu.map(|g| smooth(&mut self.gpu, g, alpha));
        if self.level > steps {
            return self.fall_to(steps, period);
        }
        let frames = |seconds: f32| (seconds / period).ceil().max(1.0) as u32;
        if let Some(frames_since) = self.since_restore.as_mut() {
            *frames_since += 1;
            if *frames_since >= frames(RESTORE_SECONDS) << MAX_BACKOFF {
                self.since_restore = None;
                self.backoff = self.backoff.saturating_sub(1);
            }
        }
        if self.cooldown > 0 {
            self.cooldown -= 1;
            if self.cooldown == 0
                && let Some(before) = self.before.take()
                && self.level > 0
            {
                self.saving[self.level - 1] = Some((before - cpu).max(0.0));
            }
            return None;
        }
        // GPU-bound, or no GPU reading: the resolution controller's turn.
        let gpu_clear = gpu.is_some_and(|g| g < GPU_BOUND * period);
        if !gpu_clear {
            self.over_frames = 0;
            self.fit_frames = 0;
            return None;
        }
        if cpu > OVER * period {
            self.fit_frames = 0;
            self.over_frames += 1;
            if self.over_frames >= frames(SHED_SECONDS) && self.level < steps {
                return self.shed(period, cpu, gpu, frames(COOLDOWN_SECONDS));
            }
            return None;
        }
        self.over_frames = 0;
        if self.level == 0 {
            return None;
        }
        let saving = self.saving[self.level - 1].unwrap_or(ASSUMED_SAVING * period);
        if cpu + saving <= RESTORE_FIT * period {
            self.fit_frames += 1;
            if self.fit_frames >= frames(RESTORE_SECONDS) << self.backoff {
                return self.restore(period, cpu, gpu, frames(COOLDOWN_SECONDS));
            }
        } else {
            self.fit_frames = 0;
        }
        None
    }

    fn shed(&mut self, period: f32, cpu: f32, gpu: Option<f32>, cooldown: u32) -> Option<Change> {
        if self.since_restore.take().is_some() {
            self.backoff = (self.backoff + 1).min(MAX_BACKOFF);
        }
        let change = self.change(self.level + 1, cpu, gpu, period);
        self.level += 1;
        self.begin_cooldown(cooldown, Some(cpu));
        Some(change)
    }

    fn restore(
        &mut self,
        period: f32,
        cpu: f32,
        gpu: Option<f32>,
        cooldown: u32,
    ) -> Option<Change> {
        let change = self.change(self.level - 1, cpu, gpu, period);
        self.level -= 1;
        self.since_restore = Some(0);
        self.begin_cooldown(cooldown, None);
        Some(change)
    }

    fn begin_cooldown(&mut self, cooldown: u32, before: Option<f32>) {
        self.cooldown = cooldown;
        self.before = before;
        self.over_frames = 0;
        self.fit_frames = 0;
    }

    /// Drops to `target` (0 for an inert governor, the step count when knobs went away).
    fn fall_to(&mut self, target: usize, period: f32) -> Option<Change> {
        if self.level <= target {
            if target == 0 {
                self.reset();
            }
            return None;
        }
        let change = self.change(target, self.cpu.unwrap_or(0.0), self.gpu, period);
        let keep = self.saving;
        self.reset();
        self.saving = keep;
        self.level = target;
        Some(change)
    }

    fn change(&self, to: usize, cpu: f32, gpu: Option<f32>, period: f32) -> Change {
        Change {
            from: self.level,
            to,
            cpu,
            gpu,
            period,
        }
    }
}

fn smooth(slot: &mut Option<f32>, value: f32, alpha: f32) -> f32 {
    let next = match *slot {
        Some(previous) => previous + alpha * (value - previous),
        None => value,
    };
    *slot = Some(next);
    next
}

#[cfg(test)]
mod tests;
