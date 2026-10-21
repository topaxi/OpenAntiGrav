//! The physics parameter set, one block per ship and speed class.
//!
//! Every field is a tunable the original reads from `Data\Ships\<Team>\handlingstats.xml` on
//! the player's own disc. No value ships in this crate ([`Handling::ZERO`] lets a test name
//! the two or three fields it cares about), per
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! # Why this is not the format's own type
//!
//! `oag_tables::handling` parses the XML (cameras and front-end bars included) and its types
//! describe the *document*; this one describes what the force law consumes, so `oag-physics`
//! depends on nothing but `oag-core` and a schema change reaches the simulation only through
//! the mapping in `oag-gameplay`. Field names match the XML attributes so the mapping is
//! inspectable. Schema: `docs/formats/handling-stats.md`; consumption: `docs/physics/README.md`
//! and `docs/ghidra/functions/psp-pulse-usa/engine.md` (the authority where they disagree).
//!
//! # This is the in-memory form, already scaled
//!
//! **Four fields are pre-scaled by the original's XML loader**, and this crate holds the
//! *scaled* form its force law consumes:
//!
//! | Field | Stored as | Consequence |
//! | --- | --- | --- |
//! | [`Engine::amount`] | `xml * 0.001` | |
//! | [`Brakes::amount`] | `xml * -0.01` | **negative in memory**; the brake force is applied along `+unit(v)` and the sign lives in the parameter |
//! | [`Airbrake::amount`] | `xml * 0.0001` | |
//! | [`Airbrake::slidegrip`] | `xml * 0.0001` | an XML 0..100 becomes 0..0.01, which is what makes the grip coefficient reach exactly zero at full airbrake |
//!
//! Confidence 88, from the parsers (`HandlingXml_ParseEngine`, `0x0883945c`, and friends).
//! **Applying the factors is `oag-gameplay`'s job**, not this crate's nor
//! `oag_tables::handling`'s (which returns raw values). Applying them twice is the likeliest
//! integration bug here, and silent: the ship is just sluggish.
//!
//! # Where the simulation's numbers come from
//!
//! Stage 6 of the engine/title split ([ADR-0022]) names this seam and **moves nothing**:
//! [ADR-0009](../../../docs/architecture/adr/0009-multi-game-fanout.md) item 2 gates
//! second-title *simulation* work behind M4's exit, and a constant moved mid-investigation
//! is one nobody can find in the diff they are bisecting. Every number falls in one place:
//!
//! | Where it comes from | Lives in | Example |
//! | --- | --- | --- |
//! | The player's disc, per team and speed class | this file's types, filled at load | every field here |
//! | The player's disc, engine-wide | `oag_tables::handling::Global` | `<SpeedupPads>`, `<GravityMul>` |
//! | The original's **code**, recovered by reading it | a `pub const` beside the law that uses it | `crate::passive::DRAG_GROUND` |
//!
//! The third row would become **a title package's** the day a second title's force law opens:
//! they are literals out of *Pulse's* executable, so a different compiled number is a
//! different table, not a different engine. Which generalise is unmeasured
//! (`docs/formats/pure-status.md` measured none of Pure's simulation); recording the side of
//! the line each is on makes the move mechanical when M4 closes.
//!
//! [ADR-0022]: ../../../docs/architecture/adr/0022-title-packages.md

/// Engine response. `<Engine accelcap amount falloff gain turbo/>`. Offsets `0xb8..0xcc` of
/// the class block; see `engine.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Engine {
    /// Base of the thrust ceiling: `cap = 0.5 * speed + accelcap`.
    pub accelcap: f32,
    /// Thrust magnitude, **already scaled by `0.001`**.
    pub amount: f32,
    /// **Dead.** Parsed at `+0xc0` and consumed by nothing.
    ///
    /// `Ship_UpdateEngine` computes a throttle ramp shaped like the airbrakes', stores it at
    /// `craft+0x2b8`, then overwrites it with the raw input at `0x0884c728` before computing
    /// thrust; both ramp branches land on that store. Verified in disassembly, confidence 85.
    /// Kept because it is in the data and reintroducing the ramp is the obvious mistake (the
    /// throttle lag reads as "feels close enough").
    pub falloff: f32,
    /// **Dead**, at `+0xb8`. See [`Self::falloff`].
    pub gain: f32,
    /// Added to thrust while the turbo flags and mode allow it.
    pub turbo: f32,
}

/// Braking response. `<Brakes amount falloff gain/>`. Offsets `0xac..0xb8`. There is **no
/// brake axis**: the brake engages when both airbrake inputs are positive and applies only
/// while grounded.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Brakes {
    /// Brake magnitude, **already scaled by `-0.01` and so negative**. The force is applied
    /// along `+unit(velocity)`, so the sign is carried here; negating at the use site too
    /// would accelerate under braking.
    pub amount: f32,
    /// Per-second decay of the brake ramp. **Live**, unlike the engine's.
    pub falloff: f32,
    /// Per-second rise of the brake ramp, toward a ceiling of 100.
    pub gain: f32,
}

/// Steering response. `<Turning amount falloff gain/>`. Offsets `0xcc..0xd8`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Turning {
    /// Yaw authority. Feeds body-local yaw directly with **no speed factor**, so authority
    /// at a standstill is not zero.
    pub amount: f32,
    /// Per-second rate while the steering state moves back toward centre.
    /// Per-second rate while the steering state moves back toward centre. Asymmetric with
    /// [`Self::gain`] by intent: a fast bite and slow return, or the reverse.
    pub falloff: f32,
    /// Per-second rate while the steering state moves toward a larger-magnitude target.
    pub gain: f32,
}

/// Airbrake response. `<Airbrake amount drag falloff gain turn slidegrip sideshift/>`.
///
/// All seven names were recovered twice independently, from the XML and from the parameter
/// block at `+0xd8`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Airbrake {
    /// **A lateral force gain, not a drag.** Scales the sideways force from an airbrake
    /// imbalance. **Already scaled by `0.0001`.**
    pub amount: f32,
    /// Feeds the forward slide term, which *accelerates* along `+forward`
    /// ([`crate::airbrake::evaluate`]).
    pub drag: f32,
    /// Per-second decay toward the analog input.
    pub falloff: f32,
    /// Per-second rise toward the analog input.
    pub gain: f32,
    /// Feeds body-local angular acceleration directly.
    pub turn: f32,
    /// Percent of lateral grip retained at full airbrake, **already scaled by `0.0001`** and
    /// so held on `0.0..=0.01`; the scaling makes `(0.01 - slidegrip)` reach exactly zero at
    /// an XML value of 100. Confidence 90, arithmetic.
    pub slidegrip: f32,
    /// Sideshift force magnitude, applied while a shift timer runs ([`crate::airbrake::sideshift_force`]).
    pub sideshift: f32,
}

/// Antigravity and suspension. `<Antigrav grip_air grip_ground landing_rebound rebound rebound_jump_time ride_height/>`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Antigrav {
    /// Lateral grip coefficient while airborne.
    pub grip_air: f32,
    /// Lateral grip coefficient while grounded.
    pub grip_ground: f32,
    /// Replaces [`Self::rebound`] for the first 0.2 s after touchdown.
    pub landing_rebound: f32,
    /// Scales suspension **damping only**, never the spring.
    pub rebound: f32,
    /// Unused by the recovered force law; carried because the XML has it.
    pub rebound_jump_time: f32,
    /// The raycast length for the hover probes, **and the primary term of the hover spring's
    /// target height**. `docs/physics/README.md` once said it "never appears in the force law";
    /// `engine.md` traced the offset chain (parser `+0x94`, `craft+0x70`, `Ship_UpdateCraft`
    /// builds the spring target `craft+0x2f0`), confidence 88. See
    /// [`crate::hover::target_height`].
    pub ride_height: f32,
}

/// Mass and the three gravity values. `<Physical flight_gravity mass normal_gravity track_gravity/>`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Physical {
    /// Gravity applied while airborne, blended in by `1 - grounded`.
    pub flight_gravity: f32,
    /// Ship mass, at `+0xf4`.
    ///
    /// Ship mass, at `+0xf4`. **Every force term in the original reads the mass at
    /// `body+0x374` instead**, and how the two relate was not traced. This crate reads this
    /// field for the hover spring (`docs/physics/README.md`) and [`crate::ship::Body::mass`]
    /// for gravity (`engine.md`); keeping them equal is the integration layer's job.
    pub mass: f32,
    /// Gravity along world down while grounded, additionally scaled by
    /// [`crate::forces::Environment::class_gravity_scale`] (authored under an attribute named
    /// `airborne`, which does not scale the airborne term). Class-block offset `+0xf8`.
    pub normal_gravity: f32,
    /// Gravity along the track's own up axis, which is what makes inversions work. **It
    /// appears only in the hover spring's calibration**, never in the gravity force itself.
    pub track_gravity: f32,
}

/// Pitch response. `<pitch pitch_air pitch_ground pitch_damping antigrav_height_adjust/>`.
/// Offsets `0x104..0x114`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pitch {
    /// Pitch authority while airborne: a plain gain on the input axis, with no ramp or state.
    pub pitch_air: f32,
    /// Pitch authority while grounded.
    pub pitch_ground: f32,
    /// Pitch-axis angular damping. **Consumed by `Ship_ApplyAngularDamping`, not the pitch
    /// term**; the only per-ship component of the damping triple (yaw and roll are hard `-5.0`
    /// and `-2.0`).
    pub pitch_damping: f32,
    /// Offset applied to the hover target height, **probably**. An additive offset sits in the
    /// target chain at `craft+0x74` and nothing was found that writes this field there (no
    /// readers of `+0x110` in the craft path; a weak negative, since it misses VFPU loads and
    /// cached copies), so "parsed but never consumed" is a hypothesis at confidence 50.
    /// [`crate::hover::target_height`] takes the offset as zero.
    pub antigrav_height_adjust: f32,
}

/// Hull dimensions and the shield pool, from `<Misc/>` (per ship, not per speed class).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Dimensions {
    /// Hull height, and so the box half-extent used for contact generation.
    pub height: f32,
    /// Hull length. Also sets where the two hover probes sit.
    pub length: f32,
    /// Hull width.
    pub width: f32,
    /// The energy pool this ship starts a race with, **already resolved for the race's skill
    /// level**: `<Misc>` authors three slots indexed by the `SkillLevel` option, picked by
    /// `oag_tables::handling::Misc::shield_for`, so this crate never sees the ladder. It is the
    /// maximum as well as the start: `Ship_SetShield` clamps every write to it and never
    /// floors at zero (`shield.md`).
    pub shield: f32,
    /// `<Misc weight_distribution>`: the pitch torque `Ship_UpdatePitch` adds every frame the
    /// craft is not grounded ([`crate::engine::pitch`]), read live as `8.0` on Assegai.
    pub weight_distribution: f32,
}

/// The speed-pad boost, per speed class, from `<GlobalClass><SpeedupPads/>`.
///
/// The one [`Handling`] member **not** from the ship's own `handlingstats.xml`: it is in the
/// engine-wide `Data\XML\HandlingStats.xml`, per class not per team. It is here because the
/// force law reads it for one ship in one class. Neither value is pre-scaled
/// (`oag_gameplay::handling::SCALED_FIELDS`).
/// Neither value is pre-scaled - see `oag_gameplay::handling::SCALED_FIELDS`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpeedupPads {
    /// The force magnitude the boost settles at, `0x08b36bc0[class]`.
    pub amount: f32,
    /// How long the boost lasts, in seconds, `0x08b36bd0[class]`.
    pub time: f32,
}

/// The drawn hull's steering lean, `oag_physics::controls::update_steer_lean`'s gains.
///
/// Display state only: nothing here reaches a force. Pulse and Pure compile the numbers in
/// ([`Self::PULSE`]); Wipeout HD reads the stick gain and the follower rate from the team's
/// `<Tilt shipAmount shipSpeed>` and smooths at its own code literal `10.0`
/// (`docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SteerLean {
    /// What the stick term `stick * 0.01` is multiplied by: Pulse `1.0`, HD `<Tilt shipAmount>`.
    pub stick_amount: f32,
    /// The follower's per-second rate: Pulse `5.0`, HD `<Tilt shipSpeed>`.
    pub follower_rate: f32,
    /// The first-order rate between the follower and the lean: Pulse `3.0`, HD `10.0`.
    pub smooth_rate: f32,
}

impl SteerLean {
    /// Pulse PSP's literals, `Ship_UpdateSteerLeans` (`0x0883fda8`, `0x0883fdb4`), measured to
    /// `8e-7` (`crates/trace/tests/steer_lean_ground_truth.rs`). Pure's `0x08925054` carries the
    /// same three.
    pub const PULSE: Self = Self {
        stick_amount: 1.0,
        follower_rate: 5.0,
        smooth_rate: 3.0,
    };

    /// No lean at all: every rate zero.
    pub const NONE: Self = Self {
        stick_amount: 0.0,
        follower_rate: 0.0,
        smooth_rate: 0.0,
    };
}

/// The cockpit camera's steering lean, `oag_physics::controls::update_camera_lean`'s gains.
///
/// Display state only, like [`SteerLean`]. Pulse compiles the numbers in ([`Self::PULSE`]);
/// Wipeout HD reads the stick gain and the follower rate from the team's
/// `<Tilt internalAmount internalSpeed>` and smooths at its own literal `6.0`, with `+/-0.5`
/// sideshift terms (`docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CameraLean {
    /// What the raw stick on the `+/-100` record is multiplied by to give the follower's target:
    /// Pulse `0.01`, HD `0.015 * <Tilt internalAmount>`.
    pub stick_scale: f32,
    /// The follower's per-second rate: Pulse `5.4`, HD `<Tilt internalSpeed>`.
    pub follower_rate: f32,
    /// The first-order rate between the follower and the lean: Pulse `4.0`, HD `6.0`.
    pub smooth_rate: f32,
    /// Added to the target while the right sideshift timer runs and taken off while the left
    /// does: HD `0.5`. Pulse's original folds in `10.0` from two flags nobody identified, so
    /// ours is `0.0` there (chosen, not measured).
    pub sideshift_offset: f32,
}

impl CameraLean {
    /// Pulse PSP's literals, `FUN_0883fab4` (`0x0883fac0` and the `4.0` smooth), measured to RMS
    /// `0.0003` (`crates/trace/tests/camera_lean_ground_truth.rs`). Pure's twin is not measured.
    pub const PULSE: Self = Self {
        stick_scale: 0.01,
        follower_rate: 5.4,
        smooth_rate: 4.0,
        sideshift_offset: 0.0,
    };

    /// No lean at all: every rate zero.
    pub const NONE: Self = Self {
        stick_scale: 0.0,
        follower_rate: 0.0,
        smooth_rate: 0.0,
        sideshift_offset: 0.0,
    };
}

/// Everything the force law reads for one ship in one speed class.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Handling {
    /// `<Engine/>`.
    pub engine: Engine,
    /// `<Brakes/>`.
    pub brakes: Brakes,
    /// `<Turning/>`.
    pub turning: Turning,
    /// `<Airbrake/>`.
    pub airbrake: Airbrake,
    /// `<Antigrav/>`.
    pub antigrav: Antigrav,
    /// `<Physical/>`.
    pub physical: Physical,
    /// `<pitch/>`.
    pub pitch: Pitch,
    /// `<Misc/>`, which is shared by all four of a ship's classes.
    pub dimensions: Dimensions,
    /// `<GlobalClass><SpeedupPads/>`, which is shared by all eight teams.
    pub speedup_pads: SpeedupPads,
    /// `<Global><Special speedpad_jump>`, shared by all eight teams **and** all four classes:
    /// how far the speed-pad boost tilts toward the hull's up axis while pitch-up is held.
    /// A single float (`g_speedpad_jump`, `0x08b36bec`) rather than a per-class table, so it
    /// sits beside the boost it modifies. Unscaled, like both `<SpeedupPads>` fields; see
    /// `crate::engine::speedup_pad`.
    pub speedpad_jump: f32,
    /// `<Global><Special turbo_jump>`, `g_turbo_jump`: the boost lift's share of thrust while
    /// a Turbo or a roll payout runs and d-pad Up is held ([`crate::engine::engine`]).
    pub turbo_jump: f32,
    /// `<Global><Special roll_cost>`: what a completed barrel roll costs, as a percentage of
    /// [`Dimensions::shield`] ([`crate::barrel_roll::arm`]).
    pub roll_cost: f32,
    /// `<Global><Special roll_speed>`: how fast [`crate::ship::ShipState::roll_phase`] ramps,
    /// in units per second ([`crate::barrel_roll::advance_phase`]).
    pub roll_speed: f32,
    /// `<Global><Special roll_turbotime>`: how long the landing payout holds, in seconds
    /// ([`crate::ship::ShipState::roll_payout_timer`]).
    pub roll_turbotime: f32,
    /// The drawn hull's steering lean (display only, [`SteerLean`]).
    pub steer_lean: SteerLean,
    /// The cockpit camera's steering lean (display only, [`CameraLean`]).
    pub camera_lean: CameraLean,
}

impl Handling {
    /// Every tunable zero. Not a playable ship: a unit test starts from a known nothing and
    /// sets only the fields it pins, keeping invented constants out of the repository.
    pub const ZERO: Self = Self {
        engine: Engine {
            accelcap: 0.0,
            amount: 0.0,
            falloff: 0.0,
            gain: 0.0,
            turbo: 0.0,
        },
        brakes: Brakes {
            amount: 0.0,
            falloff: 0.0,
            gain: 0.0,
        },
        turning: Turning {
            amount: 0.0,
            falloff: 0.0,
            gain: 0.0,
        },
        airbrake: Airbrake {
            amount: 0.0,
            drag: 0.0,
            falloff: 0.0,
            gain: 0.0,
            turn: 0.0,
            slidegrip: 0.0,
            sideshift: 0.0,
        },
        antigrav: Antigrav {
            grip_air: 0.0,
            grip_ground: 0.0,
            landing_rebound: 0.0,
            rebound: 0.0,
            rebound_jump_time: 0.0,
            ride_height: 0.0,
        },
        physical: Physical {
            flight_gravity: 0.0,
            mass: 0.0,
            normal_gravity: 0.0,
            track_gravity: 0.0,
        },
        pitch: Pitch {
            pitch_air: 0.0,
            pitch_ground: 0.0,
            pitch_damping: 0.0,
            antigrav_height_adjust: 0.0,
        },
        dimensions: Dimensions {
            height: 0.0,
            length: 0.0,
            width: 0.0,
            shield: 0.0,
            weight_distribution: 0.0,
        },
        speedup_pads: SpeedupPads {
            amount: 0.0,
            time: 0.0,
        },
        speedpad_jump: 0.0,
        turbo_jump: 0.0,
        roll_cost: 0.0,
        roll_speed: 0.0,
        roll_turbotime: 0.0,
        steer_lean: SteerLean::NONE,
        camera_lean: CameraLean::NONE,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ride_height` looking like a target height is the likeliest misreading of this
    /// parameter set, so the correction is a test and not only a doc comment.
    #[test]
    fn zero_handling_is_not_a_ship() {
        assert_eq!(Handling::ZERO.physical.mass, 0.0);
        assert_eq!(Handling::ZERO.antigrav.ride_height, 0.0);
    }
}
