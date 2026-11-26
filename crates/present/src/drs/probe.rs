//! The probe rise: after a long quiet hold below the ceiling, one grid step up on trial, kept
//! only if it ran no late frame and its cost came in under what a GPU at a steady clock would
//! have paid for the extra pixels. See
//! [ADR-0096](../../../../docs/architecture/adr/0096-dynamic-resolution-probes-a-rise-a-falling-clock-hides.md).
//!
//! # Why the normal rise cannot see this room
//!
//! A GPU whose governor lowers its clock when the load falls answers a lower scale with a
//! slower clock, so its timestamps (milliseconds, not cycles) stay where they were: on a Steam
//! Deck at 3840x2160 the readings sat inside [`super::DEADBAND`] from 0.75 to 0.85 scale, and a
//! rise needs [`super::RISE_PATIENCE`] readings under it. Pinned at its top clock, the same race
//! settled at 0.96 (`docs/rendering/dynamic-resolution.md`, "Why the 4K settle is so deep").
//! No GPU duration says whether there is room; only trying a step says it.
//!
//! # Why a steady clock does not keep the step
//!
//! Cost goes as the pixel count, so after a step from `here` to `next` a GPU at a steady clock
//! reads at most its pre-step cost times `(next / here)^2`, and about that much when the cost
//! is all per pixel. A governor that raises its clock to carry the extra pixels reads less. [`KEEP_SHARE`] is the line between the two: a trial
//! whose mean cost, sampled once [`RAMP_SECONDS`] have passed, is above `KEEP_SHARE` of that
//! prediction learned only that the pixels cost what they cost, and goes back. So the settled
//! scale of a GPU that holds its clock is the one the normal rise and its overshoot guard
//! choose; a probe there is a reverted pair of size changes, and each one doubles the next
//! hold ([`MAX_BACKOFF`]).
//!
//! Nothing here reads a clock or knows a platform: the same rule runs everywhere.

use super::{COOLDOWN, DEADBAND};

/// How long the scale must sit quiet below the ceiling before a probe, before any backoff.
///
/// Quiet is every reading at or under [`DEADBAND`]'s top with a wall-clock frame time that was
/// not late; a step, a late frame or a frame with no wall clock starts the count again.
/// **Chosen**: unpinned, the Deck held one 4K Zone scale for 10 to 30 s at a time, so this fires
/// there, and it is long against the 1 to 5 s between a scene's cost spikes.
pub const HOLD_SECONDS: f32 = 4.0;

/// The most doublings of the hold that reverted probes earn: four seconds becomes at most 128.
/// A kept probe clears them. **Chosen**, to keep a GPU at a steady clock under one reverted pair
/// of size changes a minute once it has learned.
pub const MAX_BACKOFF: u32 = 5;

/// How long after the step the trial's readings are not judged: the governor's ramp, during
/// which the readings are still taken at the old clock and would undo the step.
///
/// Measured on the Steam Deck at 3840x2160 (`docs/rendering/dynamic-resolution.md`, "The probe
/// rise"): where the clock rose after a step up, it reached 90 % of its rise in 0.01 to 1.6 s,
/// and 0.5 s judged most trials half way up.
pub const RAMP_SECONDS: f32 = 1.5;

/// How long the trial samples its cost once the ramp has passed.
pub const SAMPLE_SECONDS: f32 = 0.5;

/// The share of the steady-clock prediction a trial's cost must come in under to be kept.
///
/// A governor that carries all of the step's extra pixels reads `1 / (next / here)^2` of the
/// prediction: 0.886 for 0.80 to 0.85, 0.905 for 0.95 to 1.00. A steady clock reads 1.0 on a
/// cost that is all per pixel, which the Deck's race pass is (measured over the full-size cost
/// times the scale squared, 0.99 to 1.00 with the clock pinned), and less where part of the
/// cost does not scale: 0.94 for 0.95 to 1.00 on the modelled machine in `tests/machines.rs`,
/// whose scene is 60 % flat. **Chosen** between the two, nearer the governor's side, so a
/// steady clock keeps its settle even with a large flat share.
pub const KEEP_SHARE: f32 = 0.92;

/// The frame periods a frame may take on the wall clock before it counts as late.
///
/// Under a limiter the loop's deadlines sit on a grid, so a frame that ran a quarter of a
/// period long has already pushed the next one off it. **Chosen** above the worst per-second
/// p99 the Deck showed at a held 60 (19.6 ms of 16.67).
pub const LATE_PERIODS: f32 = 1.25;

/// Whether a frame that took `frame_seconds` on the wall clock was late at a target `period`.
#[must_use]
pub fn late(frame_seconds: f32, period: f32) -> bool {
    frame_seconds > period * LATE_PERIODS
}

/// `seconds` in frames of a target `period`, at least one.
fn frames(seconds: f32, period: f32) -> u32 {
    let frames = (seconds / period).ceil();
    if frames.is_finite() && frames >= 1.0 {
        frames as u32
    } else {
        1
    }
}

/// How a trial ended, for the log line that says so.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// Kept: the sampled cost over the steady-clock prediction.
    Kept(f32),
    /// Reverted on a late frame.
    Late,
    /// Reverted on cost: the sampled cost over the steady-clock prediction, and over the
    /// budget.
    Cost {
        /// Sampled cost over the steady-clock prediction.
        predicted: f32,
        /// Sampled cost over the scalable budget.
        budget: f32,
    },
}

/// What a trial decided this reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Verdict {
    /// Still ramping or sampling.
    Running,
    /// The step stays.
    Kept,
    /// Back to this grid point.
    Reverted(u32),
}

/// A step on trial.
#[derive(Debug, Clone, Copy)]
struct Trial {
    /// The grid point it stepped from, and goes back to.
    origin: u32,
    /// The scalable cost a steady clock would read at the new scale.
    expected: f32,
    /// Readings since the step.
    elapsed: u32,
    /// Scalable cost summed over the readings after the ramp.
    sum: f64,
    /// How many readings `sum` holds.
    count: u32,
}

/// The hold toward the next probe, and the trial in progress.
#[derive(Debug, Clone, Default)]
pub(super) struct Probe {
    /// Consecutive quiet readings at the scale in force.
    quiet: u32,
    /// Scalable cost summed over the last [`SAMPLE_SECONDS`] of the hold.
    sum: f64,
    /// How many readings `sum` holds.
    count: u32,
    /// Doublings of the hold earned by reverted probes.
    backoff: u32,
    /// The step on trial, if any.
    trial: Option<Trial>,
    /// How the last trial ended.
    outcome: Option<Outcome>,
    /// Never due: the controller without a probe, for the tests that compare against it.
    #[cfg(test)]
    pub(super) off: bool,
}

impl Probe {
    /// The hold in force at a target `period`, after backoff.
    fn hold(&self, period: f32) -> u32 {
        frames(HOLD_SECONDS, period) << self.backoff.min(MAX_BACKOFF)
    }

    /// Whether a step is on trial.
    pub(super) fn on_trial(&self) -> bool {
        self.trial.is_some()
    }

    /// How the last trial ended, or `None` before the first or after a stall ended one.
    pub(super) fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// Counts one acted-on reading toward the hold, or breaks it.
    pub(super) fn observe(&mut self, scalable: f32, quiet: bool, period: f32) {
        if !quiet {
            self.interrupt();
            return;
        }
        self.quiet = self.quiet.saturating_add(1);
        let sample = frames(SAMPLE_SECONDS, period);
        if self.quiet.saturating_add(sample) > self.hold(period) {
            self.sum += f64::from(scalable);
            self.count += 1;
        }
    }

    /// The scale moved or something else acted: the hold starts again.
    pub(super) fn interrupt(&mut self) {
        self.quiet = 0;
        self.sum = 0.0;
        self.count = 0;
    }

    /// Whether the hold has run long enough for a probe.
    pub(super) fn due(&self, period: f32) -> bool {
        #[cfg(test)]
        if self.off {
            return false;
        }
        self.trial.is_none() && self.count > 0 && self.quiet >= self.hold(period)
    }

    /// Puts a step from grid point `origin` to `next` on trial.
    pub(super) fn start(&mut self, origin: u32, next: u32) {
        let mean = (self.sum / f64::from(self.count.max(1))) as f32;
        let growth = (next as f32 / origin.max(1) as f32).powi(2);
        self.trial = Some(Trial {
            origin,
            expected: mean * growth,
            elapsed: 0,
            sum: 0.0,
            count: 0,
        });
        self.interrupt();
    }

    /// Feeds one reading to the trial, or `None` when there is none.
    ///
    /// A late frame reverts at once, except inside the step's own [`COOLDOWN`]: the step
    /// itself costs a long frame or two (measured on the Deck with its clock pinned, 29 to
    /// 36 ms between frames straight after a step), which says nothing about whether the new
    /// scale fits. Otherwise readings inside the ramp are not judged, and
    /// once the sample is complete the step is kept only if its mean cost is under both
    /// [`KEEP_SHARE`] of the steady-clock prediction and [`DEADBAND`]'s top of `budget`.
    pub(super) fn judge(
        &mut self,
        scalable: f32,
        late: bool,
        period: f32,
        budget: f32,
    ) -> Option<Verdict> {
        let trial = self.trial.as_mut()?;
        trial.elapsed += 1;
        if late && trial.elapsed > COOLDOWN {
            return Some(self.revert(Outcome::Late));
        }
        let ramp = frames(RAMP_SECONDS, period);
        if trial.elapsed <= ramp {
            return Some(Verdict::Running);
        }
        trial.sum += f64::from(scalable);
        trial.count += 1;
        if trial.count < frames(SAMPLE_SECONDS, period) {
            return Some(Verdict::Running);
        }
        let measured = (trial.sum / f64::from(trial.count)) as f32;
        let predicted = measured / trial.expected;
        if measured <= trial.expected * KEEP_SHARE && measured <= budget * DEADBAND.end() {
            self.trial = None;
            self.backoff = 0;
            self.outcome = Some(Outcome::Kept(predicted));
            Some(Verdict::Kept)
        } else {
            Some(self.revert(Outcome::Cost {
                predicted,
                budget: measured / budget,
            }))
        }
    }

    /// Ends the trial as a step that did not hold.
    fn revert(&mut self, outcome: Outcome) -> Verdict {
        self.outcome = Some(outcome);
        let origin = self.trial.take().map_or(0, |trial| trial.origin);
        self.backoff = (self.backoff + 1).min(MAX_BACKOFF);
        Verdict::Reverted(origin)
    }

    /// A stall: the readings in flight describe another picture, so the hold starts again and
    /// a trial ends where it stands without a verdict. The backoff is the machine's and stays.
    pub(super) fn stall(&mut self) {
        if self.trial.take().is_some() {
            self.outcome = None;
        }
        self.interrupt();
    }
}
