//! Which hover probes a craft hangs on, and how their springs are summed.
//!
//! Pulse's racing law ([`Rig::TWO_POINT`]) is two probes on the centre line, `0.3` of the
//! load each, pushed along the craft's own up axis, with the rear probe **derived** from the
//! front one above [`super::FAST_PROBE_SPEED`]. Wipeout HD's craft hangs on four, `0.15` each,
//! casts all four every frame and pushes each along its hit normal (the race writes a title's
//! own rig, `oag_title::hover_rig`, into [`crate::ShipState::hover_rig`]).
//! `Craft_IntegrateHull` (`0x000ef450`) marches the four segments unconditionally and
//! `Craft_HoverFourPoint` (`0x000ede88`) springs them
//! (`docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md`, confidence 88).
//!
//! The two carry the same vertical stiffness (`4 * 0.15 == 2 * 0.3`) and the same pitch
//! stiffness on paper (`4 * 0.15 * 4.5^2 == 2 * 0.3 * 4.5^2`). What separates them in pitch is
//! the derived rear hit, whose flat `6.0` slope gain stands in for the `9`-unit spacing and so
//! leaves Pulse's craft two thirds of the geometric pitch stiffness.

use oag_core::math::Vec3;

/// The most probes any rig carries.
pub const MAX_PROBES: usize = 4;

/// How the contacting probes' normals become the one the alignment torque and downforce use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalMean {
    /// Pulse: the mean of the contacting normals, normalised.
    Normalised,
    /// HD: the sum of the contacting normals times `0.25`, not normalised, or the one normal
    /// as found when a single probe is in contact (`0x000ee02c`-`0x000ee060`).
    QuarterSum,
}

/// One title's hover probe set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rig {
    /// Body-space probe offsets; only the first [`Self::count`] are used. Body forward is
    /// `-Z`, front first.
    pub offsets: [Vec3; MAX_PROBES],
    /// How many of [`Self::offsets`] are probes.
    pub count: usize,
    /// Each spring's share of `mass * (normal_gravity + track_gravity)` per unit of
    /// compression, before `HOVER_K`: `0.3` on Pulse's two-point law, `0.15` on HD.
    pub spring_share: f32,
    /// Above [`super::FAST_PROBE_SPEED`], cast the front probe only and derive the rear hit
    /// from it (Pulse, two probes only).
    pub derive_rear: bool,
    /// Push each spring along its hit normal (HD) rather than the craft's up axis (Pulse).
    pub along_normal: bool,
    /// See [`NormalMean`].
    pub normal_mean: NormalMean,
}

impl Rig {
    /// Pulse's `Ship_HoverTwoPoint`: every title's law unless it measured its own.
    pub const TWO_POINT: Self = Self {
        offsets: [
            Vec3::new(
                0.0,
                -super::PROBE_DROP_RAW * super::TARGET_GLOBAL_SCALE,
                -super::PROBE_HALF_SPACING_RAW * super::TARGET_GLOBAL_SCALE,
            ),
            Vec3::new(
                0.0,
                -super::PROBE_DROP_RAW * super::TARGET_GLOBAL_SCALE,
                super::PROBE_HALF_SPACING_RAW * super::TARGET_GLOBAL_SCALE,
            ),
            Vec3::ZERO,
            Vec3::ZERO,
        ],
        count: 2,
        spring_share: 0.3,
        derive_rear: true,
        along_normal: false,
        normal_mean: NormalMean::Normalised,
    };

    /// The probes in use.
    #[must_use]
    pub fn offsets(&self) -> &[Vec3] {
        &self.offsets[..self.count.min(MAX_PROBES)]
    }

    /// This frame's groundedness from its contact count: `0.5` a probe on Pulse's two,
    /// `0.25` a probe on HD's four (`craft+0x304 += 0.25`, `0x000ee2a4`).
    #[must_use]
    pub fn grounded(&self, contacts: u32) -> f32 {
        if self.count == 2 {
            return crate::ship::ShipState::quantise_grounded(contacts);
        }
        let count = self.count.max(1) as u32;
        (contacts.min(count) as f32) / (count as f32)
    }
}

impl Default for Rig {
    fn default() -> Self {
        Self::TWO_POINT
    }
}

#[cfg(test)]
mod tests;
