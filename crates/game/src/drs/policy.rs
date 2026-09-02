//! The policy itself: what a measured scene cost does to the render scale.
//!
//! Split out of [`super`] so the type that carries the setting and the loop
//! that acts on it can be read separately - and so the five constants below
//! sit together, because two of them are only correct with respect to each
//! other. See [`DEADBAND`].

use super::Limits;

/// The band around the budget in which nothing happens, as a fraction of it.
///
/// A frame sitting comfortably should not twitch, and a controller that
/// corrected every reading would step on noise. What decides the *width* is
/// not comfort though - it is [`STEP`]. One grid step of 5 % changes the cost
/// by about 10 %, so a band narrower than that lets a single correction land
/// back outside the band on the other side, and the scale ping-pongs between
/// two adjacent grid points forever. **The band has to be wider than one
/// step's worth of cost**, which is what
/// `the_deadband_is_wider_than_one_grid_step` asserts.
pub const DEADBAND: std::ops::RangeInclusive<f32> = 0.80..=0.95;

/// The most grid steps one correction may fall by.
///
/// Asymmetric with [`RISE_STEPS`] on purpose: a dropped frame has already been
/// seen, so falling fast costs sharpness a player was not going to get anyway.
/// Five steps is a quarter of the ceiling.
pub const FALL_STEPS: u32 = 5;

/// The most grid steps one correction may rise by.
///
/// One, because a rise that overshoots is a dropped frame - the exact thing
/// the controller exists to avoid - and climbing back is free while nothing is
/// going wrong.
///
/// **Counted in steps rather than as a percentage, and that is not cosmetic.**
/// A 4 % ceiling on the rise against a 5 % grid is a controller that can never
/// rise at all: from any grid point, `scale * 1.04` floors straight back onto
/// the point it started from, forever. Expressing both clamps in the grid's
/// own units makes that unrepresentable rather than a pair of numbers somebody
/// has to keep consistent by hand.
pub const RISE_STEPS: u32 = 1;

/// How many grid points the scale has between nothing and the ceiling.
///
/// The scale is held as a count of these rather than as a fraction, and that
/// is not tidiness: `0.5 / 0.05` is not exactly `10.0` in `f32`, so a policy
/// that derived the current grid point by dividing would sometimes floor a
/// legitimate one-step rise straight back onto the point it started from - a
/// controller that silently stops climbing at some scales and not others.
/// Integers on the way in and out, floats only for the correction itself.
pub const GRID: u32 = 20;

/// The grid the scale lands on, as a fraction of the ceiling.
///
/// Quantised so the extent takes a small set of values rather than drifting
/// continuously: a hysteresis band is only meaningful against a grid, and a
/// scale that settles on one of twenty values is one somebody can read off
/// the `dev` overlay and recognise.
pub const STEP: f32 = 1.0 / GRID as f32;

/// How many readings after a step are ignored.
///
/// A GPU timestamp resolves a frame or more late - measured on the
/// development machine at exactly one frame, over 5,306 consecutive readings.
/// A controller acting on every reading would therefore correct twice for one
/// overrun and then twice back. Four covers the frames in flight and the
/// timer's own four-slot ring.
pub const COOLDOWN: u32 = 4;

/// How many consecutive under-budget readings a rise needs.
///
/// **The cooldown is not enough on its own, and the run that proved it is on
/// the record.** With a cooldown alone, a 4K race aiming at 144 flipped
/// between 100 % and 95 % 143 times in a minute - not because the policy was
/// wrong about either reading, but because the scene pass genuinely costs
/// 0.75 to 1.07 of its budget at one fixed scale as the camera moves. Every
/// individual decision was right and the picture still changed size twice a
/// second, which is the one artefact a player would notice.
///
/// So a fall answers a single frame - it has already been seen - and a rise
/// has to be *earned* by a run of frames that all had room. Anything in the
/// deadband or over budget breaks the run, which is what makes this hysteresis
/// rather than a longer cooldown.
pub const RISE_PATIENCE: u32 = 8;

/// The closed loop, as a pure function of fed measurements.
///
/// `record` in, a rectangle out. It never reads a clock, a GPU or a settings
/// file; see [`super`] for why that is what makes it testable and why it is
/// not a determinism problem.
#[derive(Debug, Clone)]
pub struct Controller {
    /// The scale in force, counted in [`GRID`] points up from nothing.
    /// `GRID` itself is the ceiling.
    steps: u32,
    /// Readings still to be ignored after the last step.
    cooldown: u32,
    /// Consecutive readings with room to spare, counted toward
    /// [`RISE_PATIENCE`]. Reset by anything that is not one.
    under: u32,
}

impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}

impl Controller {
    /// A controller at the ceiling, having measured nothing.
    #[must_use]
    pub fn new() -> Self {
        Self {
            steps: GRID,
            cooldown: 0,
            under: 0,
        }
    }

    /// Records one scene-pass timing and returns the extent to draw next, or
    /// `None` when nothing should move.
    ///
    /// `None` covers four different quiet cases - the target is off, the
    /// reading is inside [`DEADBAND`], a [`COOLDOWN`] is still running, or the
    /// correction quantised back onto the scale already in force. The last is
    /// deliberately not a step: burning a cooldown on a no-op would halve how
    /// often the controller could respond to a real overrun.
    ///
    /// A reading that is not a finite positive number is dropped, the way
    /// [`crate::perf::Meter::record`] drops one: a clock that went backwards
    /// is not a fast frame.
    pub fn record(&mut self, scene_seconds: f32, limits: Limits) -> Option<(u32, u32)> {
        let budget = limits.target().scene_budget()?;
        if !scene_seconds.is_finite() || scene_seconds <= 0.0 {
            return None;
        }
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return None;
        }
        let ratio = scene_seconds / budget;
        if DEADBAND.contains(&ratio) {
            // Comfortable, and comfortable is not headroom: a frame in the
            // band breaks the run a rise has to earn.
            self.under = 0;
            return None;
        }
        if ratio < *DEADBAND.start() {
            self.under += 1;
            if self.under < RISE_PATIENCE {
                return None;
            }
        } else {
            self.under = 0;
        }

        // Cost is per-pixel and the scale is per-axis, so the correction that
        // multiplies the pixel count by `budget / measured` multiplies each
        // axis by its square root. Counted in grid points, which is what keeps
        // the two clamps and the quantisation commensurable - see
        // [`RISE_STEPS`] for the bug that is not expressible this way.
        let here = f64::from(self.steps);
        let wanted = here * f64::from((1.0f32 / ratio).sqrt());
        let delta = (wanted - here).clamp(-f64::from(FALL_STEPS), f64::from(RISE_STEPS));
        // **Down onto the grid, in both directions.** Rounding to nearest
        // would let a rise land above what the measurement supports, and a
        // rise that overshoots is the dropped frame this exists to avoid.
        let next = (here + delta).floor().clamp(0.0, f64::from(GRID)) as u32;
        let next = next.clamp(floor_steps(limits), GRID);

        if next == self.steps {
            return None;
        }
        self.steps = next;
        self.cooldown = COOLDOWN;
        self.under = 0;
        Some(limits.pixels(self.scale()))
    }

    /// Throws the recent past away without moving the scale.
    ///
    /// Called from the same guard `perf::Meter::clear` is: a track load lands
    /// in one frame's timing and is not a frame time in any useful sense.
    /// **The scale deliberately stays where it was.** Snapping back to the
    /// ceiling the instant a race loads would put the heaviest frame of the
    /// run at full resolution, which is the drop this whole loop exists to
    /// avoid; what has to be thrown away is the readings in flight, and the
    /// cooldown is what does it.
    pub fn reset(&mut self) {
        self.cooldown = COOLDOWN;
        self.under = 0;
    }

    /// The extent to draw at, re-derived from the bounds in force now.
    ///
    /// The caller applies this **every frame**, not only when [`record`]
    /// returns something: `upscale::Framebuffer::resize` resets the extent to
    /// the allocation whenever it reallocates, so a scale applied once would
    /// silently return to full size on the next window resize.
    ///
    /// [`record`]: Self::record
    #[must_use]
    pub fn extent(&self, limits: Limits) -> (u32, u32) {
        if !limits.target().is_on() {
            return limits.ceiling();
        }
        limits.pixels(steps_scale(self.steps.clamp(floor_steps(limits), GRID)))
    }

    /// The scale in force, as a fraction of the ceiling. For tests and the
    /// log, never for a settings file.
    #[must_use]
    pub fn scale(&self) -> f32 {
        steps_scale(self.steps)
    }
}

/// A grid point as a fraction of the ceiling.
fn steps_scale(steps: u32) -> f32 {
    steps as f32 / GRID as f32
}

/// The lowest grid point the floor allows.
///
/// Rounded **up**, so a floor between two points is honoured rather than
/// undercut by a fifth of a step - and clamped to at least one, because a
/// scale of zero is a frame with no pixels in it.
fn floor_steps(limits: Limits) -> u32 {
    let wanted = (f64::from(limits.floor_fraction()) * f64::from(GRID)).ceil();
    (wanted as u32).clamp(1, GRID)
}
