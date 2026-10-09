//! A title's hover probe set: where the craft's suspension springs hang and how they push.
//!
//! Pulse's racing hover (`Ship_HoverTwoPoint`) is two probes on the centre line, `0.3` of the
//! load each, pushed along the craft's up axis, with the rear hit derived from the front one
//! above 50 units/s. Wipeout HD's craft hangs on four probes, `0.15` each, casts every one every
//! frame and pushes each along its own hit normal (`Craft_IntegrateHull`, `0x000ef450`, and
//! `Craft_HoverFourPoint`, `0x000ede88`; `docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md`).
//! The probe set is data, so it is here and not a comparison on a title's name (ADR-0058). A
//! title with `None` flies Pulse's two-point law.

use crate::pre_race::Sourced;

/// One title's hover probe set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoverRig {
    /// The probes' body-space offsets `(right, up, back)`, after the craft's `0.75` scale, in the
    /// engine's frame (forward is `-z`).
    pub probes: Sourced<&'static [[f32; 3]]>,
    /// Each spring's share of `mass * (normal_gravity + track_gravity)` per unit of compression.
    pub spring_share: Sourced<f32>,
    /// Every probe is cast every frame, whatever the speed (no derived rear hit).
    pub cast_every_probe: Sourced<bool>,
    /// Each spring pushes along its hit normal rather than the craft's up axis.
    pub along_hit_normal: Sourced<bool>,
    /// The contact normals are summed and scaled by `0.25` (the one normal alone when a single
    /// probe touches) rather than averaged and normalised.
    pub quarter_sum_normal: Sourced<bool>,
}
