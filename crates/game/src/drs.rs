//! Dynamic resolution: the closed loop that decides how many pixels the next
//! frame is drawn with.
//!
//! [`Controller`] measures nothing and draws nothing. It is fed one GPU
//! timing per frame and emits a rectangle, exactly as [`crate::perf::Meter`]
//! is fed a duration and emits a [`crate::perf::Stats`] - and for the same
//! reason: a policy that reads its own clock cannot be tested against a
//! sequence somebody chose. Everything below is a pure function of what was
//! recorded.
//!
//! # Why this is not a determinism problem
//!
//! The argument `perf.rs` already makes, verbatim: **nothing here is read by
//! the simulation.** The composition root feeds this module and hands the
//! result to a render target; no path leads from a [`Controller`] back into a
//! tick. The timestep stays fixed at 60 Hz per ADR-0007 whatever the
//! resolution does, so a scale that moves several times a second changes how
//! many pixels a frame has and never how far the world advanced.
//!
//! # The budget subtracts what it can measure
//!
//! The signal this controls on is a [`Cost`]: the `race` pass and the
//! motion-blur chain, which both draw through the render extent and both fall
//! when it does, against the FSR 3.1 chain, which does not. The budget is the
//! target frame period **minus the measured fixed cost** minus a [`Residual`]
//! for the presentation work nothing times.
//!
//! The *signal* is still not the wall clock, and that part of ADR-0040 stands:
//! under [`crate::perf::Vsync::On`] the loop sleeps to the refresh and under
//! any [`crate::perf::FrameLimit`] it sleeps to the limit, so
//! `interval - scene` is a slack term that absorbs whatever the scene did not
//! use - the ratio would be 1.0 at every render scale and the controller would
//! be inert in precisely the configuration a player turns it on for.
//!
//! What [ADR-0044] adds is a *second* input, to the budget rather than to the
//! ratio, and it arrives in the shape everything else here does: the caller
//! hands over how long its frame took, this module never asks. A wall-clock
//! frame time is trusted only where it can be proved uncontaminated - see
//! [`residual`], which is that proof and nothing else - and the reserve it
//! learns can only ever be smaller than the constant that preceded it.
//!
//! [ADR-0044]: ../../../docs/architecture/adr/0044-the-residual-is-a-learned-upper-bound.md
//!
//! What ADR-0040 got wrong, and [ADR-0042] replaces, is the *other* half: a
//! single constant share standing in for everything the budget could not see.
//! Measured, that share has to be about 0.3 for one render profile and 0.9 for
//! another on the same machine and circuit, so no constant could serve both.
//!
//! [ADR-0042]: ../../../docs/architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md
//!
//! # The invariant that survives review
//!
//! **This never writes `[graphics] render_scale`.** It *reads* it as the
//! ceiling and emits a separate runtime rectangle. Persisting a controller
//! output into a player's settings file would make their chosen quality drift
//! downward with every session on a loaded machine - and this module cannot
//! do it by construction, because it does not import [`crate::settings`] and
//! is handed [`Limits`] by value.

use serde::{Deserialize, Serialize};

pub mod policy;
pub mod residual;

pub use policy::{
    COOLDOWN, Controller, DEADBAND, FALL_STEPS, GRID, RISE_PATIENCE, RISE_STEPS, STEP,
};
pub use residual::Residual;

/// The most of a frame that may be reserved for the work nothing times, and
/// what is reserved until a frame has proved otherwise.
///
/// **Two roles since [ADR-0044], and neither of them is "the reserve".**
/// [`Residual`] starts here and learns down from it, so this is the prior a
/// session opens on and the ceiling on what any amount of learning may reserve:
/// a floor under the *budget*, not a floor under the reserve. A floor under
/// the reserve is the one thing it cannot be: the machine ADR-0044 was written
/// for needs to reserve **less** than this, and a constant that could only be
/// raised would have no way to say so.
///
/// [ADR-0044]: ../../../docs/architecture/adr/0044-the-residual-is-a-learned-upper-bound.md
///
/// **A choice, and a much narrower one than the constant it replaces.**
/// `SCENE_SHARE` was 0.45 and asserted that the one timed pass was 45 % of a
/// frame; measured on an HD/Fury circuit at 2560x1440 it was **20 %**, and the
/// error was not a bad number but an unfixable one - a share that has to be
/// 0.3 for `msaa = 4x` with `motion_blur = high` and 0.9 with both off cannot
/// be a constant at all. See
/// [ADR-0042](../../../docs/architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md).
///
/// What is left for a constant to cover is the HUD, the UI composite, the
/// blit, the performance overlay, the MSAA resolve and the driver's and
/// frame loop's own CPU-side overhead - all of it at *presentation*
/// resolution, over the whole allocation, or off the render-extent path
/// entirely, so none of it should move when the extent does or vary with the
/// render-profile rows the way [`Cost::scalable`] and [`Cost::fixed`] do.
///
/// **Raised from 0.15 to 0.20 once `hd_bloom` joined [`Cost::scalable`]**,
/// per [ADR-0043](../../../docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md).
/// The original 0.15 was measured solo (one ship, no opponents) and folded
/// `hd_bloom` into the residual, which is what made 1.45 ms of a 9.3 ms frame
/// hold steady across four render profiles: on that circuit `hd_bloom` costs
/// about 0.36 ms regardless of `msaa`/`motion_blur`, so it never showed up as
/// motion in the number. With `hd_bloom` measured and moved out, a fielded
/// grid of seven opponents (`--mode single_race`, the mode every real race
/// runs) left roughly 2.4 ms of a ~8.7 ms frame still unaccounted for -
/// AI, physics and draw-call overhead for seven more craft, none of it timed
/// by anything this module reads. 0.20 covers that on the one circuit
/// measured; it has not been checked against a heavier `hd_bloom` ladder, a
/// second title, or a second adapter, and under-reserving is still the safer
/// direction to be wrong in than over-reserving - see
/// [ADR-0042](../../../docs/architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md).
pub const RESIDUAL_SHARE: f32 = 0.20;

/// One frame's GPU cost, split by whether the controller can change it.
///
/// **The split is the whole of ADR-0042.** A budget that exists to be divided
/// by a moving cost must not have a fixed cost folded into it, and until this
/// type there was nowhere to say which was which: the controller was handed
/// one number, the `race` pass, and a constant that pretended to stand for
/// everything else.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cost {
    /// What falls when the render extent does: the `race` pass, the
    /// motion-blur chain and the HD/Fury bloom chain - all three drawn
    /// through the extent, all three timed. See
    /// [ADR-0043](../../../docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md)
    /// for the third.
    pub scalable: f32,
    /// What does not: the FSR 3.1 chain, measured, and **all of it**.
    ///
    /// Conservative rather than exact - the chain's render-resolution passes
    /// do shrink with the extent, and only its accumulate and RCAS passes are
    /// truly presentation-bound. Counting the whole reading as fixed makes the
    /// budget smaller than the truth and so biases the controller toward
    /// falling, which is the safe direction for the bug ADR-0042 exists to
    /// fix. Splitting it needs a second pair inside `Fsr3::render`.
    ///
    /// Zero on every frame no temporal reconstruction ran, which is honest
    /// rather than a default: nothing fixed was paid.
    pub fixed: f32,
}

impl Cost {
    /// The whole reading, for a log line.
    #[must_use]
    pub fn total(self) -> f32 {
        self.scalable + self.fixed
    }
}

/// How many frames a second dynamic resolution is aiming for, or `off`.
///
/// A type of its own rather than [`crate::perf::FrameLimit`], which already
/// means "how many frames a second the loop *may produce*" - the same numbers
/// against a different question, and its `unlimited` is meaningless as a
/// budget. `Off` is a value here rather than a separate enable toggle: one
/// row, one answer, and no pair of keys that can disagree.
///
/// Zero is off rather than a rate of zero, which is why the field is private
/// and [`Target::hz`] returns an `Option` - the same shape, and the same
/// argument, as `FrameLimit`'s sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Target(u32);

impl Target {
    /// No controller: the frame is drawn at the `render_scale` ceiling.
    pub const OFF: Self = Self(0);

    /// What a fresh install gets.
    ///
    /// **Off**, which is the footing every enhancement in this project starts
    /// on: the out-of-the-box run reproduces what the settings already say
    /// rather than opting a player into a picture that changes size under
    /// load without them asking.
    pub const DEFAULT: Self = Self::OFF;

    /// The fastest target that can be asked for.
    ///
    /// Not a preference: past here the budget is shorter than the readings
    /// this controls on are precise about, so what it delivers stops being
    /// what it was asked for.
    pub const MAX: u32 = 480;

    /// The rates the menus offer: off, then the refresh rates displays are
    /// actually built at.
    pub const OFFERED: [Self; 6] = [
        Self::OFF,
        Self(30),
        Self(60),
        Self(90),
        Self(120),
        Self(144),
    ];

    /// The rate aimed for, or `None` when this is off.
    #[must_use]
    pub fn hz(self) -> Option<u32> {
        (self.0 > 0).then_some(self.0)
    }

    /// Whether a controller runs at all.
    #[must_use]
    pub fn is_on(self) -> bool {
        self.0 > 0
    }

    /// This target, held under a presentation rate that will not deliver it.
    ///
    /// **Aiming above the frame limiter is asking for frames the loop is not
    /// allowed to produce.** At `target_fps = 144` behind a 60 limit the
    /// budget would be 2.4x too tight, so the controller would drop the
    /// resolution - permanently, and to hold a rate the limiter forbids
    /// whatever it does. Every pixel it gave up would buy nothing.
    ///
    /// `None` means nothing bounds the rate and the target stands. That covers
    /// [`crate::perf::FrameLimit::UNLIMITED`], and it also covers
    /// [`crate::perf::Vsync::On`], where the *display* is the bound and this
    /// build cannot ask a surface what its refresh is. Guessing the
    /// simulation's 60 there - which is what `Session::presentation_hz` falls
    /// back to for the overlay's graph scale, and says so - would silently cap
    /// a 144 Hz panel's target at 60. A wrong clamp is worse than none,
    /// because the menu says the target it was given.
    #[must_use]
    pub fn at_most(self, hz: Option<u32>) -> Self {
        match (self.hz(), hz) {
            (Some(target), Some(bound)) if target > bound => Self(bound),
            _ => self,
        }
    }

    /// The target frame period in seconds, or `None` when off.
    #[must_use]
    pub fn period(self) -> Option<f32> {
        self.hz().map(|hz| 1.0 / hz as f32)
    }

    /// How long the *scalable* work may take, given what the fixed work
    /// already cost this frame and what is being held back for the work
    /// nothing times - or `None` when this target is off.
    ///
    /// `period - fixed - residual`. The measured fixed cost is
    /// **subtracted** rather than absorbed into a share, which is the whole
    /// change ADR-0042 makes: a controller that shrinks the extent buys back
    /// nothing from the FSR 3.1 chain, so pretending a constant fraction of
    /// the frame covers it made the budget wrong by a factor that varied with
    /// the render profile.
    ///
    /// **The residual is now a [`Residual`] rather than a constant**, which is
    /// the whole of [ADR-0044] - it opens at `RESIDUAL_SHARE * period` and
    /// falls from there on frames that can prove the machine spends less. It
    /// can never rise past the constant, so this budget is never smaller than
    /// the one the same reading produced before that ADR landed;
    /// `a_learned_residual_never_reserves_more_than_the_constant` pins that.
    ///
    /// [ADR-0044]: ../../../docs/architecture/adr/0044-the-residual-is-a-learned-upper-bound.md
    ///
    /// **May be zero or negative, and the caller must not treat that as an
    /// error.** It is the honest answer to "how much room is left for the
    /// scene at this target on this machine": none. See
    /// [`Controller::record`], which stops stepping rather than grinding to
    /// the floor for frames that will never arrive.
    #[must_use]
    pub fn scalable_budget(self, fixed_seconds: f32, residual: Residual) -> Option<f32> {
        let period = self.period()?;
        let fixed = if fixed_seconds.is_finite() && fixed_seconds > 0.0 {
            fixed_seconds
        } else {
            0.0
        };
        Some(period - fixed - residual.seconds(period))
    }

    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> String {
        match self.hz() {
            Some(hz) => hz.to_string(),
            None => "off".to_string(),
        }
    }
}

impl std::str::FromStr for Target {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        if text.eq_ignore_ascii_case("off") {
            return Ok(Self::OFF);
        }
        let hz: u32 = text.parse().map_err(|_| {
            format!("{text:?} is not a dynamic-resolution target; write a rate or \"off\"")
        })?;
        // Zero is rejected rather than accepted as a second spelling of off,
        // the reason `FrameLimit` rejects it: two spellings for one value is
        // how a settings file and a menu row start disagreeing about what is
        // selected.
        if hz == 0 || hz > Self::MAX {
            return Err(format!(
                "a dynamic-resolution target is 1 to {} frames a second, or \"off\"",
                Self::MAX
            ));
        }
        Ok(Self(hz))
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}

impl TryFrom<String> for Target {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Target> for String {
    fn from(target: Target) -> Self {
        target.to_string()
    }
}

/// What bounds a controller this frame, read fresh off the settings.
///
/// By value, and holding pixels rather than settings: it is what makes the
/// module docs' invariant structural rather than a promise. A [`Controller`]
/// cannot reach `render_scale` to write it because it is never handed
/// anything that owns one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The render target's allocation - `render_scale` in pixels, and the
    /// most the controller may ask for.
    ceiling: (u32, u32),
    /// The fewest pixels it may fall to, already `<= ceiling` on both axes.
    floor: (u32, u32),
    /// The rate aimed for, or [`Target::OFF`].
    target: Target,
}

impl Limits {
    /// Bounds for one frame, with the floor brought under the ceiling.
    ///
    /// **The clamp is the point.** The floor row and the render-scale row are
    /// independent, and the menu's warning about a floor at or above the
    /// ceiling is advisory - it tells a player, it does not stop them. A
    /// `floor > ceiling` reaching an `Ord::clamp` further down would panic in
    /// the frame loop, so it is resolved here, once, where the two numbers
    /// first meet.
    #[must_use]
    pub fn new(ceiling: (u32, u32), floor: (u32, u32), target: Target) -> Self {
        let ceiling = (ceiling.0.max(1), ceiling.1.max(1));
        Self {
            ceiling,
            floor: (floor.0.clamp(1, ceiling.0), floor.1.clamp(1, ceiling.1)),
            target,
        }
    }

    /// The allocation: what the frame is drawn at with no controller running.
    #[must_use]
    pub fn ceiling(self) -> (u32, u32) {
        self.ceiling
    }

    /// The fewest pixels the controller may ask for.
    #[must_use]
    pub fn floor(self) -> (u32, u32) {
        self.floor
    }

    /// The rate aimed for.
    #[must_use]
    pub fn target(self) -> Target {
        self.target
    }

    /// The floor as a fraction of the ceiling, on the tighter axis.
    ///
    /// One number for both axes because the scale is one number: the extent
    /// keeps the allocation's aspect, so a floor that is 0.5 on one axis and
    /// 0.6 on the other is a floor of 0.6.
    #[must_use]
    fn floor_fraction(self) -> f32 {
        let axis = |floor: u32, ceiling: u32| floor as f32 / ceiling.max(1) as f32;
        axis(self.floor.0, self.ceiling.0).max(axis(self.floor.1, self.ceiling.1))
    }

    /// The pixels a scale of `fraction` means, never outside the bounds.
    #[must_use]
    fn pixels(self, fraction: f32) -> (u32, u32) {
        let axis = |ceiling: u32, floor: u32| {
            let scaled = (f64::from(ceiling) * f64::from(fraction)).round();
            let scaled = if scaled.is_finite() {
                scaled as u32
            } else {
                ceiling
            };
            scaled.clamp(floor.min(ceiling), ceiling)
        };
        (
            axis(self.ceiling.0, self.floor.0),
            axis(self.ceiling.1, self.floor.1),
        )
    }
}

#[cfg(test)]
mod tests;
