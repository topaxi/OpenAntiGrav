//! Engine, brakes, steering and pitch: the control force law.
//!
//! All four are transcribed from `docs/ghidra/functions/psp-pulse/engine.md`, which
//! read them out of `BOOT.BIN`. Nothing here is runtime-verified; the page caps
//! itself at 84 for exactly that reason.
//!
//! Four things in this module are not what a reimplementation would guess:
//!
//! - **The throttle is not ramped.** `<Engine gain/>` and `<Engine falloff/>` are
//!   dead; see [`crate::params::Engine::falloff`].
//! - **There is no brake axis.** The brake engages when both airbrake inputs are
//!   positive at once, and it is applied only while grounded, so braking does
//!   nothing in the air.
//! - **`Brakes.amount` is negative in memory** and the force is applied along
//!   `+unit(velocity)`. Negating here as well would accelerate under braking.
//! - **Every term in this module reads the *previous* frame's groundedness.** Hover
//!   is step 8 of 15 and clears the contact flag on entry, so the engine, the
//!   brakes and pitch all run against last frame's contacts. See
//!   [`crate::ship::ShipState::grounded_prev`].
//!
//! **No control input writes an angular Z component.** Not the engine, not the
//! brakes, not the steering, not the pitch axis, not the airbrakes: roll is never
//! commanded. Angular Z is written in exactly two places and both are passive, the
//! angular damping and the surface-alignment torque. Confidence 80 on the narrowed
//! negative, and it is narrower than `docs/physics/README.md`'s original claim that
//! "no Z component is ever written", which was too strong.

use oag_core::math::Vec3;

use crate::params::Handling;
use crate::ship::{ShipControls, ShipState};

/// The unknown per-craft scalar the engine multiplies its output by.
///
/// `T = T * craft+0x294 * 2.0`, unconditionally, at the end of
/// `Ship_UpdateEngine`. **Nothing was found that writes `craft+0x294`**, so the
/// identity is chosen here: it keeps the `* 2.0` visible without inventing a
/// second factor. A guess awaiting M3, and one worth checking, since anything
/// other than 1.0 scales all thrust.
///
/// # M3 has now checked it, and `1.0` is measurably wrong
///
/// A real capture of the reference scenario
/// (`docs/reverse-engineering/ppsspp-debugger.md`; Time Trial, Venom, Talon's
/// Junction White, Assegai) holds **23.6 to 25.1 units/s at throttle 100** across 200
/// ticks and is very slightly *decelerating*, so the original's net longitudinal force
/// is about zero there. This crate, replaying the same capture through
/// `oag-trace run`, passes 55 by tick 30, 81 by tick 100 and 170 by tick 190: it has
/// **no speed equilibrium at all** below about 128 units/s.
///
/// The arithmetic that gap implies, with every other term left exactly as transcribed:
///
/// ```text
/// cap  = 0.5 * 23.35 + accelcap                 // the cap does bind at racing speed
/// T    = min(throttle * amount, cap) * X * 2    // X is this constant
/// need = 0.005 * 24^2 + 2.0                     // grounded drag + rolling resistance
///      = 4.88, against 58 at X = 1
/// ```
///
/// so `X ~= 0.084`. Set to that, and to nothing else, the replay holds 22 to 25
/// units/s against the recording's 23.6 to 25.1 for **160 of the 200 ticks** - the
/// whole speed divergence closes on this one scalar.
///
/// **It is deliberately left at `1.0`.** `0.084` is a number fitted to one capture and
/// nothing has been read that writes `craft+0x294`; committing it would put an
/// invented constant where a recovered one belongs, and six months from now the two
/// would be indistinguishable. What the measurement buys is a *target*: whoever reads
/// `craft+0x294`'s writer can check the value they find against 0.084 before believing
/// it.
///
/// Two things narrow where else the factor could live, and both are dead ends:
///
/// - **Not the drag coefficients.** `-0.005` grounded, `-0.002` airborne and `-0.1`
///   reversing are confirmed at instruction level in the PS2 build as well
///   (`Ship_ApplyQuadraticDrag`, `0x0015c3a0`), at confidence 88. Forcing the grounded
///   coefficient to `-0.1` *does* reproduce the recorded speed just as well, which is
///   what makes it worth stating that this route is closed by evidence rather than by
///   preference.
/// - **Not the mass.** The two masses cancel out of a force balance entirely, and the
///   ratio between them is independently pinned near 1 by the hover height: the same
///   capture rests 4.002 above the surface against the 3.978 the spring predicts when
///   `Body::mass == Physical::mass`, and a body mass 17x the parameter one would put
///   that at 1.6.
///
/// The remaining unread candidate is `craft+0x2a0`, the flag-gated multiplier this
/// crate also does not implement.
pub const ENGINE_OUTPUT_SCALE: f32 = 1.0;

/// The fixed doubling on the engine's output, from the same expression.
pub const ENGINE_OUTPUT_DOUBLE: f32 = 2.0;

/// The fraction of thrust available with no ground contact.
///
/// `T = T * grounded + T * 0.2 * (1 - grounded)`, so 20 % airborne and blended
/// through the half-grounded case rather than switched.
pub const ENGINE_AIR_THRUST: f32 = 0.2;

/// Speed below which the brake force fades out linearly, and the reciprocal used
/// to fade it.
///
/// `if (speed < 10.0) dir *= speed * 0.1`, which reaches zero at a standstill and
/// is also what keeps `velocity / speed` from being asked for at rest.
pub const BRAKE_FADE_SPEED: f32 = 10.0;

/// What the engine contributed, in the body-local force accumulator.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EngineForce {
    /// Thrust along body forward, the accumulator's `.z`.
    pub thrust: f32,
    /// Boost lift along body up, the accumulator's `.y`.
    ///
    /// **Always zero here.** It is non-zero only under the turbo flags, which are
    /// bits of the undecoded `craft+0x1c0` flag word, and the boost scale is a
    /// global that was not read. Carried as a field so the shape is visible.
    ///
    /// The original's final accumulate reads a VFPU register for this lane that the
    /// function never loads; the reading is that the caller's `vzero.q` left it at
    /// zero, making the observable result `accumulator.y = lift`. Confidence 65 on
    /// that explanation, and it is why this is an addition to a zeroed accumulator
    /// rather than an attempt to reproduce a register carry.
    pub lift: f32,
}

impl EngineForce {
    /// As a body-local force vector. Body forward is `-Z`, so thrust is negated
    /// into the `.z` lane.
    #[must_use]
    pub fn as_local_force(self) -> Vec3 {
        // The original's row 2 is forward and its accumulator `.z` is "forward",
        // so a positive thrust pushes along the craft's forward axis. This crate's
        // body forward is `-Z` (see `Body::forward`), which is where the negation
        // comes from; it is a convention difference, not a sign finding. Handedness
        // itself is no longer open - `docs/ghidra/functions/psp-pulse/engine.md`
        // measured the original's basis as positively oriented under the ordinary
        // cross product, the same arithmetic this crate uses - but that measurement
        // says nothing about this particular row-to-axis mapping.
        Vec3::new(0.0, self.lift, -self.thrust)
    }
}

/// The engine.
///
/// ```text
/// T = throttle * Engine.amount                 // amount already * 1e-3 at load
/// T = T * grounded + T * 0.2 * (1 - grounded)
/// cap = 0.5 * speed + Engine.accelcap
/// if (throttle > 100.0)  cap = throttle * 0.01 * cap
/// T = min(T, cap)
/// T = T * craft+0x294 * 2.0
/// ```
///
/// `speed` is `|dot(velocity, forward)|`, not `|velocity|`. `grounded` is the
/// previous frame's 0/0.5/1 fraction.
///
/// **Not implemented, all of it flag-gated on the undecoded `craft+0x1c0`:** the
/// uncapped mode (`cap = 1e10`), the `craft+0x2a0` multiplier, turbo and its boost
/// lift, the one-shot scale at `craft+0x31c`, the kill switch at bit `0x2000`, and
/// the four-corner mode's auto-speed law. Each needs a flag nobody has decoded, so
/// implementing them would mean inventing their triggers.
#[must_use]
pub fn engine(
    state: &ShipState,
    handling: &Handling,
    grounded: f32,
    forward_speed: f32,
) -> EngineForce {
    let throttle = state.thrust;

    let mut thrust = throttle * handling.engine.amount;
    // Blended rather than switched, so a half-grounded ship gets 60 %.
    thrust = thrust * grounded + thrust * ENGINE_AIR_THRUST * (1.0 - grounded);

    let mut cap = 0.5 * forward_speed.abs() + handling.engine.accelcap;
    if throttle > crate::controls::CONTROL_MAX {
        cap *= throttle * 0.01;
    }
    thrust = thrust.min(cap);

    thrust = thrust * ENGINE_OUTPUT_SCALE * ENGINE_OUTPUT_DOUBLE;

    EngineForce { thrust, lift: 0.0 }
}

/// The brake, as a world-space force.
///
/// ```text
/// speed = |velocity|
/// dir   = speed != 0 ? velocity / speed : velocity
/// if (speed < 10.0)  dir *= speed * 0.1
/// worldForce += dir * Brakes.amount * brake        // amount is negative
/// ```
///
/// The caller must apply the grounded gate: `Ship_UpdateBrakes` is called only when
/// the contact flag is set, which is the *previous* frame's flag, and only outside
/// the four-corner mode.
///
/// A VFPU target prefix on the accumulate reads, under the standard per-component
/// selector, as zeroing the world `y` lane, which would make the brake force
/// horizontal. **That is confidence 55** - the prefix encoding was not confirmed -
/// so it is not implemented, and the page itself notes it is low-stakes because
/// braking only runs while grounded, where the velocity is nearly horizontal. A
/// pick awaiting M3.
#[must_use]
pub fn brakes(state: &ShipState, handling: &Handling) -> Vec3 {
    if state.brake <= 0.0 {
        return Vec3::ZERO;
    }

    let velocity = state.body.linear_velocity;
    let speed = velocity.length();
    let mut direction = if speed != 0.0 {
        velocity / speed
    } else {
        velocity
    };
    if speed < BRAKE_FADE_SPEED {
        direction *= speed * 0.1;
    }

    direction * handling.brakes.amount * state.brake
}

/// Steering, as a body-local yaw contribution.
///
/// ```text
/// yaw = steer * Turning.amount
/// if (reverse) yaw = blend > 1.0 ? -yaw : yaw * (1.0 - 2.0 * blend)
/// ```
///
/// `Turning.amount` feeds body-local yaw **directly, with no speed factor**, so
/// steering authority at a standstill is not zero and any speed dependence comes
/// from the damping and grip terms instead.
///
/// # Why the literal `steer * Turning.amount` is negated here
///
/// `docs/ghidra/functions/psp-pulse/engine.md` ("The basis is positively
/// oriented, and row 0 points left") measured this off a running race: holding
/// right yaws at a mean `-1.42 rad/s` about the up axis, holding left `+1.51
/// rad/s`, mirror-symmetric in both sign and magnitude. The measured law is
/// **`yaw_rate = -k * steer`**, confidence 90 - a plain literal-transcription
/// sign, not a handedness flip. The same page rules handedness out as the
/// explanation here: the original's basis is positively oriented under the
/// ordinary component-wise cross product, the same arithmetic this crate uses,
/// so there is no component-level handedness difference to blame. `steer` is
/// positive to the right by the time it reaches here (see
/// [`ShipControls::steer_x`]), and this crate's own convention already requires
/// a negative `local_angular.y` to turn the nose right - see the weathervane
/// direction test in `crate::passive` for the same identity applied to a
/// different term. Negating the base `yaw` here, before the reverse-controls
/// blend, satisfies both.
///
/// The reverse-controls blend is applied unconditionally here rather than behind
/// the original's flag, because it is the identity at zero and continuous through
/// it: `yaw * (1 - 2 * 0) == yaw`. So a ship with no reverse-controls pickup
/// behaves exactly as if the branch were skipped.
///
/// **Not implemented:** the steering bias at `craft+0x2e4` behind flag `0x20`, and
/// the two `craft+0x2a4` modes that force `steer = 0`. Both need undecoded state.
#[must_use]
pub fn steering(state: &ShipState, handling: &Handling) -> f32 {
    let yaw = -(state.steer * handling.turning.amount);

    if state.reverse_controls > 1.0 {
        -yaw
    } else {
        yaw * (1.0 - 2.0 * state.reverse_controls)
    }
}

/// The pitch axis, as a body-local pitch contribution.
///
/// ```text
/// p = controls.pitch * (grounded ? pitch_ground : pitch_air)
/// ```
///
/// `pitch_air` and `pitch_ground` are plain gains on the input axis, not rates:
/// there is no ramp and no state. The `grounded` test here is the **boolean**
/// contact flag rather than the 0/0.5/1 fraction, and it is the previous frame's.
///
/// **Not implemented:** the gate at `FUN_088492bc`, which was not decoded, so pitch
/// input applies unconditionally; and the per-team in-air pitch bias at
/// `stats_base + 0x90`, which sits outside every class block and whose XML element
/// is not known, so there is no field in [`Handling`] to read it from.
///
/// The sign convention is **not settled**: [`ShipControls::steer_y`] is documented
/// as positive nose up, and the original's axis polarity for this specific term was
/// not read. Handedness itself is settled -
/// `docs/ghidra/functions/psp-pulse/engine.md` measured the original's basis as
/// positively oriented under the ordinary cross product - but that measurement was
/// about steering, not pitch, and does not by itself pin this term's polarity.
/// Taken as-is, a guess awaiting M3.
#[must_use]
pub fn pitch(controls: &ShipControls, handling: &Handling, grounded: bool) -> f32 {
    let gain = if grounded {
        handling.pitch.pitch_ground
    } else {
        handling.pitch.pitch_air
    };

    controls.steer_y * gain
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{Brakes, Engine, Pitch, Turning};
    use crate::ship::Body;

    /// Arbitrary round numbers, already in the scaled in-memory form.
    /// **Not recovered values.**
    fn test_handling() -> Handling {
        Handling {
            engine: Engine {
                accelcap: 20.0,
                amount: 0.4,
                falloff: 1.0,
                gain: 1.0,
                turbo: 5.0,
            },
            brakes: Brakes {
                amount: -0.5,
                falloff: 200.0,
                gain: 400.0,
            },
            turning: Turning {
                amount: 0.01,
                falloff: 200.0,
                gain: 400.0,
            },
            pitch: Pitch {
                pitch_air: 0.02,
                pitch_ground: 0.005,
                pitch_damping: 3.0,
                antigrav_height_adjust: 0.0,
            },
            ..Handling::ZERO
        }
    }

    fn ship_moving_forward(speed: f32) -> ShipState {
        ShipState {
            body: Body {
                linear_velocity: Vec3::new(0.0, 0.0, -speed),
                ..Body::default()
            },
            ..ShipState::default()
        }
    }

    #[test]
    fn full_throttle_pushes_the_ship_along_its_forward_axis() {
        let handling = test_handling();
        let mut state = ship_moving_forward(40.0);
        state.thrust = 100.0;

        let force = engine(&state, &handling, 1.0, 40.0).as_local_force();
        // Body forward is -Z.
        assert!(force.z < 0.0, "force was {force:?}");
        assert_eq!(force.x, 0.0);
        assert_eq!(force.y, 0.0);
    }

    #[test]
    fn no_throttle_is_no_thrust() {
        let handling = test_handling();
        let state = ship_moving_forward(40.0);
        assert_eq!(engine(&state, &handling, 1.0, 40.0).thrust, 0.0);
    }

    /// An airborne ship keeps a fifth of its thrust, and a half-grounded one lands
    /// between the two rather than snapping.
    #[test]
    fn thrust_is_reduced_but_not_removed_with_no_contact() {
        let handling = Handling {
            engine: Engine {
                // A cap high enough not to bind, so this test sees only the blend.
                accelcap: 1000.0,
                ..test_handling().engine
            },
            ..test_handling()
        };
        let mut state = ship_moving_forward(0.0);
        state.thrust = 100.0;

        let grounded = engine(&state, &handling, 1.0, 0.0).thrust;
        let airborne = engine(&state, &handling, 0.0, 0.0).thrust;
        let half = engine(&state, &handling, 0.5, 0.0).thrust;

        assert_eq!(airborne, grounded * ENGINE_AIR_THRUST);
        assert!(half > airborne && half < grounded);
    }

    /// The cap rises with speed, which is what makes `accelcap` a floor on the
    /// ceiling rather than a top speed.
    #[test]
    fn the_thrust_cap_grows_with_forward_speed() {
        let handling = test_handling();
        let mut state = ship_moving_forward(0.0);
        state.thrust = 100.0;

        let stationary = engine(&state, &handling, 1.0, 0.0).thrust;
        let fast = engine(&state, &handling, 1.0, 200.0).thrust;
        assert!(fast > stationary, "{fast} was not above {stationary}");
    }

    /// The cap reads the *magnitude* of the forward speed, so reversing does not
    /// give a ship a smaller ceiling than standing still.
    #[test]
    fn the_thrust_cap_uses_the_magnitude_of_the_forward_speed() {
        let handling = test_handling();
        let mut state = ship_moving_forward(0.0);
        state.thrust = 100.0;

        assert_eq!(
            engine(&state, &handling, 1.0, 60.0).thrust,
            engine(&state, &handling, 1.0, -60.0).thrust
        );
    }

    #[test]
    fn a_zero_parameter_engine_produces_nothing() {
        let mut state = ship_moving_forward(40.0);
        state.thrust = 100.0;
        assert_eq!(engine(&state, &Handling::ZERO, 1.0, 40.0).thrust, 0.0);
    }

    #[test]
    fn the_brake_opposes_travel_rather_than_accelerating_it() {
        let handling = test_handling();
        let mut state = ship_moving_forward(40.0);
        state.brake = 100.0;

        let force = brakes(&state, &handling);
        // Travelling along -Z, so the brake must push along +Z.
        assert!(force.z > 0.0, "force was {force:?}");
    }

    /// `Brakes.amount` is negative in memory. A reimplementation that negated here
    /// as well would accelerate under braking, which this pins.
    #[test]
    fn a_positive_brake_amount_would_accelerate_and_is_therefore_not_what_the_data_holds() {
        let mut handling = test_handling();
        handling.brakes.amount = 0.5;
        let mut state = ship_moving_forward(40.0);
        state.brake = 100.0;

        // Documenting the trap: with the sign flipped the force is along travel.
        assert!(brakes(&state, &handling).z < 0.0);
    }

    #[test]
    fn no_brake_state_is_no_brake_force() {
        let handling = test_handling();
        let state = ship_moving_forward(40.0);
        assert_eq!(state.brake, 0.0);
        assert_eq!(brakes(&state, &handling), Vec3::ZERO);
    }

    #[test]
    fn the_brake_fades_out_at_low_speed_and_is_exactly_zero_at_rest() {
        let handling = test_handling();

        let mut at_rest = ship_moving_forward(0.0);
        at_rest.brake = 100.0;
        assert_eq!(brakes(&at_rest, &handling), Vec3::ZERO);

        let mut crawling = ship_moving_forward(1.0);
        crawling.brake = 100.0;
        let mut fast = ship_moving_forward(100.0);
        fast.brake = 100.0;

        assert!(brakes(&crawling, &handling).length() < brakes(&fast, &handling).length());
    }

    /// Holding right (`steer > 0`) must turn the ship right, which in this crate's
    /// convention needs a **negative** `local_angular.y` contribution - see the
    /// weathervane direction test in `crate::passive` for the same identity
    /// applied to a different term, and the measured law in
    /// `docs/ghidra/functions/psp-pulse/engine.md` this pins against. A test that
    /// only checked `right == -left` would pass whether or not the whole thing
    /// were inverted, which is exactly the bug this pins.
    #[test]
    fn steering_right_yaws_negative_and_is_symmetric_with_left() {
        let handling = test_handling();
        let mut state = ShipState {
            steer: 100.0,
            ..ShipState::default()
        };

        let right = steering(&state, &handling);
        state.steer = -100.0;
        let left = steering(&state, &handling);

        assert!(
            right < 0.0,
            "steering right yawed {right}, expected negative"
        );
        assert_eq!(right, -left);
    }

    /// Steering authority does not vanish at a standstill: `Turning.amount` feeds
    /// yaw with no speed factor at all.
    #[test]
    fn steering_authority_is_independent_of_speed() {
        let handling = test_handling();
        let stationary = ShipState {
            steer: 50.0,
            ..ShipState::default()
        };
        let mut fast = ship_moving_forward(200.0);
        fast.steer = 50.0;

        assert_eq!(steering(&stationary, &handling), steering(&fast, &handling));
    }

    /// The reverse-controls pickup is a blend: normal at 0, dead at 0.5, inverted
    /// at 1. A boolean would snap where the original ramps.
    #[test]
    fn reversed_controls_blend_through_dead_rather_than_snapping() {
        let handling = test_handling();
        let mut state = ShipState {
            steer: 100.0,
            ..ShipState::default()
        };

        let normal = steering(&state, &handling);

        state.reverse_controls = 0.0;
        assert_eq!(steering(&state, &handling), normal);

        state.reverse_controls = 0.25;
        assert_eq!(steering(&state, &handling), normal * 0.5);

        state.reverse_controls = 0.5;
        assert_eq!(steering(&state, &handling), 0.0);

        state.reverse_controls = 1.0;
        assert_eq!(steering(&state, &handling), -normal);

        state.reverse_controls = 2.0;
        assert_eq!(steering(&state, &handling), -normal);
    }

    #[test]
    fn the_pitch_axis_uses_a_different_gain_on_the_ground_than_in_the_air() {
        let handling = test_handling();
        let controls = ShipControls {
            steer_y: 1.0,
            ..ShipControls::default()
        };

        assert_eq!(
            pitch(&controls, &handling, true),
            handling.pitch.pitch_ground
        );
        assert_eq!(pitch(&controls, &handling, false), handling.pitch.pitch_air);
    }

    #[test]
    fn pitch_is_a_plain_gain_with_no_state_and_is_symmetric() {
        let handling = test_handling();
        let up = ShipControls {
            steer_y: 1.0,
            ..ShipControls::default()
        };
        let down = ShipControls {
            steer_y: -1.0,
            ..ShipControls::default()
        };

        assert_eq!(
            pitch(&up, &handling, false),
            -pitch(&down, &handling, false)
        );
        assert_eq!(pitch(&ShipControls::default(), &handling, false), 0.0);
    }

    /// `pitch_damping` is consumed by the angular damping term, not by the pitch
    /// input, so changing it must not change the pitch response.
    #[test]
    fn pitch_damping_does_not_affect_the_pitch_input() {
        let controls = ShipControls {
            steer_y: 1.0,
            ..ShipControls::default()
        };
        let mut handling = test_handling();
        let before = pitch(&controls, &handling, true);
        handling.pitch.pitch_damping *= 10.0;
        assert_eq!(pitch(&controls, &handling, true), before);
    }
}
