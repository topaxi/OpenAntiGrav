//! The policy itself: what a measured scene cost does to the render scale.
//!
//! Split out of [`super`] so the type that carries the setting and the loop
//! that acts on it can be read separately - and so the five constants below
//! sit together, because two of them are only correct with respect to each
//! other. See [`DEADBAND`].

use super::{Cost, Limits, Residual};

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
    /// Consecutive frames with nothing left to decide, counted toward
    /// [`RISE_PATIENCE`] before [`Self::unreachable`] says so.
    ///
    /// **A run rather than a flag, for the reason a rise needs one.** At a
    /// target the machine can only just hold, the controller sits between two
    /// adjacent grid points and the "nowhere left to go" condition is true on
    /// some frames and false on others - measured, 68 times in 41 seconds. A
    /// flag would report every one of those as a fresh spell.
    unreachable: u32,
    /// What this machine spends on work no timer here reaches, learned from
    /// the frames that can prove it - see [`super::residual`].
    ///
    /// **Session state, not a setting**, and the type is what says so: it
    /// carries no `Serialize` and this module cannot reach `crate::settings`
    /// to write one anyway.
    residual: Residual,
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
            unreachable: 0,
            residual: Residual::new(),
        }
    }

    /// Records one frame's [`Cost`] and how long that frame took on the
    /// caller's wall clock, and returns the extent to draw next, or `None`
    /// when nothing should move.
    ///
    /// **`frame_seconds` is an input to the budget and never to the ratio.**
    /// It is the same `elapsed` the composition root feeds
    /// [`crate::perf::Meter`], handed over rather than read here, and all it
    /// does is let [`super::Residual`] tighten what is held back for the work
    /// no timer reaches - see that module for why a paced loop's wall clock is
    /// an upper bound rather than a measurement, and [ADR-0044] for the machine
    /// that made a constant untenable. Passing no reading at all is how a
    /// caller says it has none - a frame that carried a load, a meter it just
    /// cleared - and leaves the estimate exactly where it was.
    ///
    /// [ADR-0044]: ../../../../docs/architecture/adr/0044-the-residual-is-a-learned-upper-bound.md
    ///
    /// A returned `None` covers five quiet cases - the target is off, the reading is
    /// inside [`DEADBAND`], a [`COOLDOWN`] is still running, the correction
    /// quantised back onto the scale already in force, or the target is out of
    /// reach at any resolution. The fourth is deliberately not a step: burning
    /// a cooldown on a no-op would halve how often the controller could
    /// respond to a real overrun.
    ///
    /// A reading that is not a finite positive number is dropped, the way
    /// [`crate::perf::Meter::record`] drops one: a clock that went backwards
    /// is not a fast frame.
    ///
    /// **The unreachable branch is not an error path.** When the measured
    /// fixed cost alone fills the target frame period, no render scale exists
    /// that would deliver the rate - so the controller holds where it is
    /// rather than walking to the floor to buy frames that will never arrive.
    /// That is the same failure [`super::Target::at_most`] was written to
    /// prevent, reached from the other direction, and it is reported through
    /// [`Self::unreachable`] so the caller can say so once.
    pub fn record(
        &mut self,
        cost: Cost,
        frame_seconds: Option<f32>,
        limits: Limits,
    ) -> Option<(u32, u32)> {
        let period = limits.target().period()?;
        let scalable = cost.scalable;
        if !scalable.is_finite() || scalable <= 0.0 {
            return None;
        }
        // **Before the budget is taken, so this frame's evidence reaches this
        // frame's decision**, and before every branch below, so a cooldown or
        // a comfortable stretch does not throw the reading away: the estimate
        // is learned from what the frame cost, not from what the controller
        // decided to do about it.
        if let Some(frame_seconds) = frame_seconds {
            self.residual.observe(frame_seconds, cost.total(), period);
        }
        let budget = limits.target().scalable_budget(cost.fixed, self.residual)?;
        // **Before the cooldown**, so a target that has become unreachable is
        // reported on the frame it happens rather than up to `COOLDOWN` frames
        // later - and so the cooldown cannot be spent on a frame there was
        // never a decision to make.
        if budget <= 0.0 {
            self.unreachable = self.unreachable.saturating_add(1);
            return None;
        }
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return None;
        }
        let ratio = scalable / budget;
        if DEADBAND.contains(&ratio) {
            // Comfortable, and comfortable is not headroom: a frame in the
            // band breaks the run a rise has to earn.
            self.under = 0;
            // It does break the *other* run, though: a frame that fits is the
            // plainest possible evidence the target is not out of reach.
            self.unreachable = 0;
            return None;
        }
        if ratio < *DEADBAND.start() {
            self.under = self.under.saturating_add(1);
            self.unreachable = 0;
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
        let floor = floor_steps(limits);
        let next = next.clamp(floor, GRID);
        // **A rise has to land with room to spare, not merely inside the
        // band.** Cost goes as the pixel count, so stepping from `here` to
        // `next` multiplies it by `(next / here)^2` - and near the floor that
        // is a much bigger jump than near the ceiling: one step of twenty is
        // 10 % of the cost at the top of the range and 21 % at half scale.
        // [`DEADBAND`] is 15 % wide, so below about 70 % scale a rise that the
        // band permits lands *outside* it on the other side and is undone next
        // frame. Measured once the budget could see the whole scalable cost:
        // 176 changes in 41 seconds, against the 143 a minute that
        // [`RISE_PATIENCE`] was added to fix.
        //
        // Predicting against `DEADBAND.start()` rather than `.end()` is the
        // margin, and it is deliberately conservative: the quadratic model is
        // optimistic, because a scene pass has per-frame cost in it that does
        // not scale with pixels at all - measured at half scale the real
        // multiplier was 1.30 where the model said 1.21. A refused rise costs
        // sharpness the machine could not hold anyway; an accepted one that
        // overshoots is the dropped frame this whole loop exists to avoid.
        if next > self.steps {
            let growth = (f64::from(next) / here).powi(2);
            if f64::from(ratio) * growth > f64::from(*DEADBAND.start()) {
                return None;
            }
        }

        if next == self.steps {
            // **Parked at the floor and still over budget is unreachable too.**
            // The quantised correction has nowhere left to go, so the caller
            // should hear the same thing it hears when the budget goes
            // non-positive rather than watching a silent controller sit at the
            // floor missing its target.
            if next == floor && ratio > 1.0 {
                self.unreachable = self.unreachable.saturating_add(1);
            } else {
                self.unreachable = 0;
            }
            return None;
        }
        self.steps = next;
        self.cooldown = COOLDOWN;
        self.under = 0;
        // A step taken is a decision made, so whatever run of "nowhere left to
        // go" was building is over.
        self.unreachable = 0;
        Some(limits.pixels(self.scale()))
    }

    /// Whether the target cannot be met at any resolution this controller may
    /// choose.
    ///
    /// True once the measured fixed cost alone has filled the frame period, or
    /// the scale has been parked at the floor and still over budget, for
    /// [`RISE_PATIENCE`] consecutive frames. For a log line and a menu note,
    /// never for a decision - the controller has already made the only
    /// decision available, which is to stop.
    ///
    /// **Earned by a run, exactly as a rise is**, and for the same reason: a
    /// marginal target makes the underlying condition flicker, and a caller
    /// that logged the flicker would print a paragraph a second about a
    /// situation that has not changed.
    #[must_use]
    pub fn unreachable(&self) -> bool {
        self.unreachable >= RISE_PATIENCE
    }

    /// What this session has learned it spends outside every timer, for a log
    /// line and for the budget arithmetic a reader has to be able to redo.
    ///
    /// **The budget is no longer a constant within a session**, so a `ratio`
    /// printed on one frame and a `ratio` printed a minute later are not
    /// comparable without this - which is why the trace line that prints the
    /// cost prints the reserve beside it.
    #[must_use]
    pub fn residual(&self) -> Residual {
        self.residual
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
    ///
    /// **[`Self::residual`] survives too, for the same reason the scale
    /// does.** It is a property of the machine rather than of the last few
    /// frames, and a load is the one moment its evidence is worthless -
    /// throwing away what a whole race taught it, to relearn from the frames
    /// straight after a stall, would be the same mistake as snapping back to
    /// the ceiling. The stalled frame itself never reaches it: the caller
    /// passes no wall-clock reading for the frame it dropped.
    pub fn reset(&mut self) {
        self.cooldown = COOLDOWN;
        self.under = 0;
        self.unreachable = 0;
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
