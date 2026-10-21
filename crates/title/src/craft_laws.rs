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
    /// airborne (`oag_physics::engine::pitch`). Measured on Pulse and HD.
    pub air_pitch_bias: Sourced<bool>,
    /// `Ship_UpdatePitch` drops the pitch input unless the hull is within the alignment cone
    /// (`Ship_TestPitchAlignment`, `oag_physics::engine::pitch_aligned`). A title with no
    /// `CraftLaws` (Pulse) runs it.
    pub pitch_gate: Sourced<bool>,
    /// In an Eliminator race a craft pointing more than about 114 degrees off the track's
    /// direction (`dot(forward, tangent) < -0.4`) loses its engine and holds both airbrakes
    /// full: Pulse PSP's `craft+0x1c0 & 0x2000`, set by `FUN_088418e0` in game modes 8 and 18
    /// (`oag_raceplay`'s `wrong_way_kill`). Wipeout Pure's executable reads no such bit. A title
    /// with no `CraftLaws` (Pulse) runs it.
    pub eliminator_wrong_way_kill: Sourced<bool>,
    /// The sideshift gestures are withheld in the grid state (Pulse's `craft+0x1c0 & 2` test in
    /// `Ship_UpdateSideshiftInput_q`). A title with no `CraftLaws` (Pulse) runs it.
    pub sideshift_grid_gate: Sourced<bool>,
    /// A running Turbo (Pulse's `craft+0x1c0 & 0x200`) escapes the engine's stun return,
    /// zeroes a speed pad's force and fades it back in over a second, and lifts with d-pad Up
    /// held. A title with no `CraftLaws` (Pulse) runs them.
    pub turbo_running_laws: Sourced<bool>,
    /// The per-class `<GravityMul>` scale multiplies `flight_gravity` (the airborne term), as
    /// Wipeout HD's `Craft_Update` does, rather than Pulse's `normal_gravity`
    /// (`oag_physics::passive::ClassGravityTerm`).
    pub class_gravity_on_flight: Sourced<bool>,
    /// The inertia diagonal rotates with the craft (`R I R^T`) rather than staying fixed in
    /// world axes as Pulse's `Body_Integrate` keeps it (`oag_physics::InertiaFrame`).
    pub inertia_rotates: Sourced<bool>,
    /// `Ship_UpdateEngine` doubles its output, `T * craft+0x294 * 2.0`, as Pulse's does.
    /// Wipeout Pure's engine has no doubling (`oag_physics::laws::Laws::engine_output_double`).
    pub engine_output_double: Sourced<bool>,
    /// The grounded quadratic drag coefficient, `Ship_ApplyQuadraticDrag`'s: Pulse `-0.005`,
    /// Wipeout Pure `-0.0025` (`oag_physics::laws::Laws::drag_ground`).
    pub drag_ground: Sourced<f32>,
    /// The two-point hover's bank-to-yaw gain, `torque.y += gain * right.y`: Pulse `30`,
    /// Wipeout Pure `50` (`oag_physics::laws::Laws::bank_to_yaw`).
    pub bank_to_yaw: Sourced<f32>,
    /// The bank-to-yaw gain in a Zone race: Pulse's and Pure's four-corner hovers load `50`,
    /// Wipeout HD's Zone keeps its racing hover's `30`
    /// (`oag_physics::laws::Laws::bank_to_yaw_zone`).
    pub bank_to_yaw_zone: Sourced<f32>,
    /// `Body_ResolveContact` applies no impulse to a wall contact whose point already moves out
    /// (`vn > 0`): Wipeout HD's (`0x000f7770`), measured live. Pulse PSP's is two-sided
    /// (`oag_physics::laws::Laws::wall_one_sided`).
    pub wall_one_sided: Sourced<bool>,
    /// Each resolved wall contact queues a yaw torque turning the nose away from the wall,
    /// integrated on the next step: Wipeout HD's `Body_ResolveContact` tail, measured live
    /// (`oag_physics::wall::steer_away_torque`). Pulse has none.
    pub wall_steer_away: Sourced<bool>,
    /// `Body_ResolveContact` crosses the world lever arm with the craft's **body-local** rate,
    /// `v + (R^T omega) x r`, where Pulse's is textbook: Wipeout HD's, measured live
    /// (`oag_physics::laws::Laws::wall_point_rate_body_local`).
    pub wall_point_rate_body_local: Sourced<bool>,
    /// The track header's flag turns on the collision cage, Pulse's `AiTrack_CageEnabled`
    /// (`*(track + 4) + 0x18 & 1`, `oag_gameplay::cage`). Wipeout HD runs no cage off it
    /// (`docs/ghidra/functions/ps3-hdfury-eu/collision-cage.md`). A title with no `CraftLaws`
    /// (Pulse) runs it.
    pub cage_from_track_header: Sourced<bool>,
    /// The pitch response a title compiles into its engine, used where a `<Class>` authors no
    /// `<pitch>`: Wipeout Pure's. `None`: the file's own block, or the stand-in.
    pub code_pitch: Option<Sourced<CodePitch>>,
    /// The sideshift force a title compiles into its engine, used where a `<Class>` authors no
    /// `<Airbrake sideshift>`: Wipeout Pure's `200`. `None`: the file's, or none at all.
    pub code_sideshift: Option<Sourced<f32>>,
    /// A launch boost a title compiles into its engine rather than authoring as
    /// `<Handling><Global><StartBoost>`: Wipeout Pure's. `None`: the file's, or none.
    pub code_start_boost: Option<Sourced<CodeStartBoost>>,
    /// The drawn hull's steering lean reads the team's `<Tilt shipAmount shipSpeed>` and smooths
    /// at this rate, Wipeout HD's `10.0` (`oag_physics::SteerLean`). `None`: Pulse's compiled-in
    /// gains (`oag_physics::SteerLean::PULSE`), which Pure's executable carries too.
    pub tilt_steer_lean: Option<Sourced<f32>>,
    /// The cockpit camera's lean reads the team's `<Tilt internalAmount internalSpeed>` with
    /// these two code literals, Wipeout HD's (`oag_physics::CameraLean`). `None`: Pulse's
    /// compiled-in gains (`oag_physics::CameraLean::PULSE`).
    pub tilt_camera_lean: Option<Sourced<TiltCameraLean>>,
}

/// The literals Wipeout HD's cockpit camera lean adds to `<Tilt>`'s `internalAmount` and
/// `internalSpeed` (`oag_physics::CameraLean`); a title that has them also bounds the follower's
/// step the way HD does (`oag_physics::CameraClamp::Hd`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TiltCameraLean {
    /// The first-order rate between the follower and the lean: HD `6.0`.
    pub smooth_rate: f32,
    /// What a running sideshift timer adds to or takes off the target: HD `0.5`.
    pub sideshift_offset: f32,
}

/// A launch boost compiled into the engine: the seven figures Pulse's `<StartBoost>` authors,
/// with the same meaning (`oag_physics::launch::StartBoost`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CodeStartBoost {
    /// Seconds after GO at which the perfect window opens.
    pub window_start: f32,
    /// Seconds after GO at which the perfect window closes.
    pub window_end: f32,
    /// Seconds after GO at which the stall window closes.
    pub stall_end: f32,
    /// How long the multiplier is held, in seconds.
    pub overall_duration: f32,
    /// The multiplier for a stall (thrust before the window, or just after it).
    pub stall_mul: f32,
    /// The multiplier for a normal launch.
    pub normal_mul: f32,
    /// The multiplier for a perfect launch.
    pub boost_mul: f32,
    /// Only the player's engine reads the multiplier (Pure: `0x0892f60c` tests the craft's
    /// entity against the race's player), so the field launches unboosted.
    pub player_only: bool,
}

/// A pitch response compiled into the engine rather than authored as `<pitch>`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CodePitch {
    /// Torque per unit of pitch input while airborne (Pulse's `pitch_air`).
    pub air: f32,
    /// Torque per unit of pitch input while grounded (Pulse's `pitch_ground`).
    pub ground: f32,
    /// The racing pitch damping (Pulse's `pitch_damping`).
    pub damping: f32,
    /// The pitch damping in the grid state (`oag_physics::laws::Laws::pitch_damping_grid`).
    pub grid_damping: f32,
}
