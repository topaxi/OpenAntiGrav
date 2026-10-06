//! How hard this craft can stop, measured by stopping it.
//!
//! The backward pass in [`super::SpeedPlan`] needs a deceleration for every
//! speed, and the physics has no closed form worth trusting for one: the
//! airbrakes ramp in at `Airbrake.gain`, drag and the symmetric brake both scale
//! with speed, and a slope adds or takes away gravity. So the craft is
//! accelerated along the course's own line from the grid and then braked hard,
//! and the deceleration it actually showed is binned by speed.
//!
//! It is a first guess the search corrects, not the arbiter: a braking zone that
//! starts too late still fails at the corner, and the corner's ceilings fall
//! until the zone is long enough.

use oag_physics::Raycaster;

use super::{Course, Craft, Run, SpeedPlan, forward_speed};
use crate::Tuning;

const BIN: f32 = 10.0;

/// Bins in the table: up to 640 units per second, past every class's top
/// speed with a pad boost on it.
const BINS: usize = 64;

/// The fraction of the measured deceleration the plan banks on. **Chosen, not
/// measured**: a margin for slopes, a craft already turning, and the brake
/// ramp the calibration run does not see again.
const SAFETY: f32 = 0.8;

/// Ticks the calibration accelerates for at most. **Chosen.**
const RUN_UP: u32 = 1_200;

/// Ticks it brakes for at most. **Chosen.**
const BRAKING: u32 = 400;

/// Deceleration by speed, units per second squared.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decel {
    bins: [f32; BINS],
}

impl Decel {
    /// What a build with nothing measured falls back on. **Chosen, not
    /// measured**, and deliberately timid: a plan built on it brakes early.
    pub const FALLBACK: Self = Self { bins: [20.0; BINS] };

    /// The deceleration the plan banks on at `speed`.
    #[must_use]
    pub fn at(&self, speed: f32) -> f32 {
        let bin = (speed.max(0.0) / BIN) as usize;
        self.bins[bin.min(BINS - 1)]
    }

    /// Builds a table from per-bin sums and counts, filling every empty bin
    /// from its nearest measured neighbour **below** (or above, under the
    /// lowest), so a speed above anything measured uses the highest measured
    /// figure rather than an extrapolation.
    pub(super) fn from_samples(sum: &[f32; BINS], count: &[u32; BINS]) -> Self {
        let mut bins = [f32::NAN; BINS];
        for b in 0..BINS {
            if count[b] > 0 {
                bins[b] = (sum[b] / count[b] as f32).max(1.0) * SAFETY;
            }
        }
        let Some(lowest) = (0..BINS).find(|&b| !bins[b].is_nan()) else {
            return Self::FALLBACK;
        };
        let floor = bins[lowest];
        let mut last = floor;
        for value in &mut bins {
            if value.is_nan() {
                *value = last;
            } else {
                last = *value;
            }
        }
        Self { bins }
    }
}

/// Accelerates from the grid, brakes hard, and bins what it saw. Returns the
/// table and the physics steps it took.
pub(super) fn calibrate<R: Raycaster + ?Sized>(
    course: &Course<'_, R>,
    craft: &Craft,
    tuning: &Tuning,
    yaw: f32,
) -> (Decel, u64) {
    let mut steps = 0u64;
    let mut run: Run = SpeedPlan::start_run(craft);
    let mut fastest = run;
    let mut top = 0.0f32;
    for _ in 0..RUN_UP {
        let tick = course.tick(&mut run, craft, tuning, yaw, |_, _| (1.0, 0.0));
        steps += 1;
        if tick.failure.is_some() {
            break;
        }
        let speed = forward_speed(&run.state);
        if speed > top {
            top = speed;
            fastest = run;
        }
    }

    let mut sum = [0.0f32; BINS];
    let mut count = [0u32; BINS];
    let mut run = fastest;
    for _ in 0..BRAKING {
        let before = forward_speed(&run.state);
        if before < BIN {
            break;
        }
        let grounded = run.state.time_airborne <= 0.0;
        let tick = course.tick(&mut run, craft, tuning, yaw, |_, _| (0.0, 1.0));
        steps += 1;
        // A craft braking from top speed is often braking into the corner that
        // ended its run-up, and runs wide of it. Its deceleration is still
        // the brake's while nothing touches it, so only the touching ticks
        // are dropped - and a wreck ends the run.
        if tick.failure == Some(super::Failure::Wrecked) {
            break;
        }
        let after = forward_speed(&run.state);
        if tick.contact {
            continue;
        }
        if grounded && run.state.time_airborne <= 0.0 && after < before {
            let bin = ((before / BIN) as usize).min(BINS - 1);
            sum[bin] += (before - after) / course.dt;
            count[bin] += 1;
        }
    }
    (Decel::from_samples(&sum, &count), steps)
}
