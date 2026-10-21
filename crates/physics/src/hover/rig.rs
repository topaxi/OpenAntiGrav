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
    /// Pulse's Zone hover: the sum times `1.0 / n`, not normalised (`Ship_HoverFourCorner`,
    /// `div.s` at `0x0884b6c0`, `vscl.q` at `0x0884b6d8`).
    Mean,
}

/// Which spring law a probe in contact pushes with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpringLaw {
    /// `Ship_HoverTwoPoint` and HD's `Craft_HoverFourPoint`:
    /// `mass * share * (t - h) * K * (0.75 g' + 0.25) * gravity`, damped by
    /// `1 + rebound * clamp(-0.1 vn, -1, 2)`, faded by `1 - magLockBlend`.
    Racing,
    /// Pulse's Zone hover, `Ship_HoverFourCorner` (`0x0884b4a8`-`0x0884b538`):
    /// `(((t - h) * K) * gravity) * share * mass * (1 + 0.8 * clamp(-0.1 vn, -1, 1))`. No load
    /// factor, no rebound, no magnet-lock fade ([`super::FOUR_CORNER_DAMPER`]).
    FourCorner,
}

/// Whether a probe inside a `MagFloor` moves the body out.
///
/// The escape's gate is the hit record's mesh-shape tag on both engines (Pulse
/// `craft+0x208 == 1`, HD `hit+0x28 == 1`), so a magstrip passes it as a floor or a wall does.
/// HD adds `craft+0x2c0 != 1.0`: a craft at full magnet lock does not escape
/// (`Craft_HoverFourPoint`, `0x000ee634`-`0x000ee63c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagFloorEscape {
    /// Whatever the magnet lock: Pulse's `Ship_HoverTwoPoint` tests nothing but the height and
    /// the tag before it moves the body (`0x0884a808`-`0x0884a86c`). Live under player input on
    /// Talon's Junction, all 578 magstrip probe hits carry the tag `1`; the hold keeps them
    /// `2.19` or more above the strip, so the escape is reachable there only by a craft the
    /// hold has lost (`airborne-ground-truth.md`).
    Always,
    /// Unless [`crate::ShipState::mag_lock_blend`] is `1.0` (HD).
    BelowFullLock,
}

/// One title's hover probe set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rig {
    /// Body-space probe offsets; only the first [`Self::count`] are used. Body forward is
    /// `-Z`, front first.
    pub offsets: [Vec3; MAX_PROBES],
    /// How many of [`Self::offsets`] are probes. A `u8` so the rig, which every craft in the
    /// `World` carries, stays 72 bytes.
    pub count: u8,
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
    /// Apply every penetrating probe's escape, summed, rather than the largest alone. Every
    /// rig the originals were read for sums: HD's `Craft_HoverFourPoint` and Pulse's and Pure's
    /// `Ship_HoverTwoPoint`, whose probe loops take each height off probe and hit points fixed
    /// before the loop, so one probe's `Body_Translate` never shortens the next probe's escape
    /// (Pulse `0x0884a798`-`0x0884ab90`, seen live on a landing: `1.698` recorded, `1.691`
    /// summed, `1.221` the larger alone; `airborne-ground-truth.md`). `false` exists for the
    /// comparison only.
    pub sum_escapes: bool,
    /// A probe under `1.0` unit moves the body out (`Ship_HoverTwoPoint`, HD's
    /// `Craft_HoverFourPoint`). Pulse's Zone hover, `Ship_HoverFourCorner` (`0x0884ae90`), has
    /// no escape in its probe loop: the height store at `0x0884b2e0` goes straight to the
    /// surface-class test (`docs/ghidra/functions/psp-pulse-usa/zone-hover.md`).
    pub probe_escape: bool,
    /// Whether a probe inside a `MagFloor` escapes ([`MagFloorEscape`]). A `Floor` and a `Wall`
    /// always do: the gate is the hit record's mesh-shape tag, not a floor class. Pulse's
    /// `Collision_RaycastMesh` writes `1` and `Collision_RaycastBox` `3` (`collision.md`), and
    /// HD's `Collision_MarchSegment` writes `1` on its mesh path and nowhere else
    /// (`0x00036bb4`-`0x00036bb8`, read through to `hit+0x28` at `0x00036d44`). Measured live on
    /// Pulse: a derived rear hit on a wall at height `0.798` lifts the recorded body `0.2026`,
    /// against `0.2017` predicted (`airborne-ground-truth.md`).
    pub mag_floor_escape: MagFloorEscape,
    /// See [`SpringLaw`].
    pub spring: SpringLaw,
    /// The alignment torque has the right axis projected out, so it never pitches
    /// (`Ship_HoverTwoPoint`, `0x0884ac94`-`0x0884acfc`; HD). Pulse's Zone hover sends
    /// `-400 * (up x n)` to the body whole (`0x0884b704`-`0x0884b758`).
    pub align_projects_pitch_out: bool,
    /// The downforce is scaled by the grounded fraction and by `1 - magLockBlend`
    /// (`Ship_HoverTwoPoint`, `0x0884ad78`-`0x0884ae18`; HD). Pulse's Zone hover applies
    /// `-track_gravity * mass` along the normal whatever the contact count
    /// (`0x0884b7b0`-`0x0884b818`).
    pub downforce_scaled: bool,
    /// The Zone hover's trailing ray: a 10-unit ray down from the craft's centre that pulls the
    /// body this share of the way to `3.0` above whatever it hits, every frame ([`super::snap`]).
    /// Pulse's `Ship_HoverFourCorner` closes a quarter ([`super::SNAP_GAIN`]); Wipeout HD's
    /// `Craft_HoverFourPoint` with its second argument set closes half (`0x008a918c`). `0.0`:
    /// no ray (a plain `f32` rather than an `Option`, which would grow the hashed `World`).
    pub snap: f32,
    /// The craft update skips the brakes: `Ship_UpdateCraft` calls `Ship_UpdateBrakes` only outside
    /// game mode 6 (`0x08849ab8`-`0x08849ac4`), the mode whose hover this rig is. Not a hover
    /// term, but it is selected by the same mode, so it rides on the mode's rig.
    pub brakes_off: bool,
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
        sum_escapes: true,
        probe_escape: true,
        mag_floor_escape: MagFloorEscape::Always,
        spring: SpringLaw::Racing,
        align_projects_pitch_out: true,
        downforce_scaled: true,
        snap: 0.0,
        brakes_off: false,
    };

    /// The probes in use.
    #[must_use]
    pub fn offsets(&self) -> &[Vec3] {
        &self.offsets[..usize::from(self.count).min(MAX_PROBES)]
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
