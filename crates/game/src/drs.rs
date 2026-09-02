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
//! # The budget is a share of a frame, not a measured frame
//!
//! The signal this controls on is [`crate::main::session::Session::scene_cost`] -
//! the `race` pass alone, timed on the GPU. What it is measured *against* is
//! [`SCENE_SHARE`] of the target frame period, and the alternative - some
//! headroom derived from the wall-clock frame interval - is degenerate rather
//! than merely worse. Under [`crate::perf::Vsync::On`] the loop sleeps to the
//! refresh and under any [`crate::perf::FrameLimit`] it sleeps to the limit,
//! so `interval - scene` is a slack term that absorbs whatever the scene did
//! not use: the ratio would be 1.0 at every render scale and the controller
//! would be inert in precisely the configuration a player turns it on for.
//! It would also put a clock inside the one module whose whole selling point
//! is that it has none. See ADR-0040.
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

pub use policy::{COOLDOWN, Controller, DEADBAND, FALL_STEPS, GRID, RISE_STEPS, STEP};

/// What fraction of a frame the scene pass alone is allowed to take.
///
/// **A choice, not a measurement**, and it exists because the budget is the
/// `race` pass and nothing else - deliberately and incompletely, see the
/// budget table on `docs/rendering/dynamic-resolution.md`. A target rate
/// cannot become a scene budget without naming how much of a frame that one
/// pass may be, and the rest has to cover the bloom, the motion blur, the
/// resolve, the UI composite and the driver's own overhead, none of which is
/// timed.
///
/// For scale: 1.556 ms of a 16.67 ms frame at 100 % on the development
/// machine is 9 %, but that is one adapter on one track, and a controller
/// that aimed at 9 % would chase a resolution the rest of the frame cannot
/// afford. **This rises when Phase 5's passes join the budget**; it is a
/// constant to revisit, not a formula to redesign.
pub const SCENE_SHARE: f32 = 0.45;

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

    /// How long the scene pass may take, in seconds, or `None` when off.
    ///
    /// [`SCENE_SHARE`] of the target frame period. See the module docs for why
    /// this is a share of a period rather than anything measured.
    #[must_use]
    pub fn scene_budget(self) -> Option<f32> {
        self.hz().map(|hz| SCENE_SHARE / hz as f32)
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
