//! A title's craft-body laws where they differ from Pulse's: the mass the inertia tensor is built
//! with and whether the steering ramp stops at its target.
//!
//! Wipeout HD's `Ship_Construct` builds the craft's box inertia with mass `1.0` where Pulse passes
//! `0.9`, and its `Craft_UpdateSteering` clamps each ramp step at the target where Pulse's
//! overshoots and cycles (`docs/ghidra/functions/ps3-hdfury-eu/craft-inertia.md`). Data, not a
//! comparison on a title's name (ADR-0058). A title with `None` flies Pulse's laws.

use crate::pre_race::Sourced;

/// One title's craft-body laws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CraftLaws {
    /// The mass the `(12, 8, 12)` box inertia is built with (Pulse `0.9`).
    pub inertia_mass: Sourced<f32>,
    /// The steering ramp stops at its target rather than overshooting by a step.
    pub steer_ramp_clamped: Sourced<bool>,
}
