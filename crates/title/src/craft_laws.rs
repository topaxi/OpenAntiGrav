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
    /// `Ship_UpdatePitch` adds the team's `<Misc weight_distribution>` as a pitch torque while
    /// airborne (`oag_physics::engine::pitch`). Measured on Pulse and HD and held off for now;
    /// a title with no `CraftLaws` flies without it too.
    pub air_pitch_bias: Sourced<bool>,
    /// The per-class `<GravityMul>` scale multiplies `flight_gravity` (the airborne term), as
    /// Wipeout HD's `Craft_Update` does, rather than Pulse's `normal_gravity`
    /// (`oag_physics::passive::ClassGravityTerm`).
    pub class_gravity_on_flight: Sourced<bool>,
    /// `Ship_UpdateEngine` doubles its output, `T * craft+0x294 * 2.0`, as Pulse's does.
    /// Wipeout Pure's engine has no doubling (`oag_physics::laws::Laws::engine_output_double`).
    pub engine_output_double: Sourced<bool>,
    /// The grounded quadratic drag coefficient, `Ship_ApplyQuadraticDrag`'s: Pulse `-0.005`,
    /// Wipeout Pure `-0.0025` (`oag_physics::laws::Laws::drag_ground`).
    pub drag_ground: Sourced<f32>,
    /// The two-point hover's bank-to-yaw gain, `torque.y += gain * right.y`: Pulse `30`,
    /// Wipeout Pure `50` (`oag_physics::laws::Laws::bank_to_yaw`).
    pub bank_to_yaw: Sourced<f32>,
}
