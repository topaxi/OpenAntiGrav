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

/// The start-line boost multiplier the engine scales its output by, `1.0` while
/// racing.
///
/// `T = T * craft+0x294 * 2.0`, unconditionally, at the end of
/// `Ship_UpdateEngine`. **`craft+0x294` is now recovered**, and it is the
/// start-line boost multiplier rather than a hidden global gain - see
/// `docs/ghidra/functions/psp-pulse/engine.md`, "The two engine multipliers are
/// recovered". Three writers agree it is `1.0` outside the start-line window:
/// the craft constructor `Ship_InitCraft` (`0x08849354`), the per-race reset
/// `Race_ResetCraftBoosts_q` (`0x088271b4`), and `Ship_UpdateStartBoost`
/// (`0x0883fdec`), which drives it from a `windowStart`/`overallDuration`/
/// `stallMul`/`normalMul`/`boostMul` group loaded from XML for the first second
/// or two of a race and then stores `1.0` back. So `1.0` here is a **recovered
/// value**, at confidence 88, not the identity chosen for want of anything
/// better. Confidence 88.
///
/// The start boost itself is not implemented: it needs the race-start grade at
/// `player+0x36c` and the XML group above, neither of which this crate has.
///
/// # The measured 17x gap is real, and it is not this constant
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
/// **`0.084` is not what `craft+0x294` holds.** It holds `1.0` while racing, read out
/// of three writers. Nor is it the other engine multiplier: `craft+0x2a0` is `1.2`,
/// the speed-up pickup, written together with the `0x0004` flag that gates it, so it
/// contributes nothing on a Time Trial lap and could not scale *down* if it did. Both
/// arms of the "it's a multiplier nobody has read" hypothesis are therefore closed,
/// and the factor lives somewhere else.
///
/// # The gap is on the *resistance* side, not the thrust side
///
/// The shipped `Engine` block for Assegai/Venom has now been read off a real disc,
/// in both the PSP and the PS2 asset sets, which agree. The numbers themselves are
/// not recorded here - see `docs/formats/handling-stats.md`, which deliberately keeps
/// shipped design values out of this repository - but three things follow from them
/// and those *are* recorded, because they close three hypotheses:
///
/// - **`accelcap` is positive**, so the force-law refutation of a negative one
///   (below) is confirmed empirically as well as structurally.
/// - **`amount` is a plain integer with no exponent notation**, so the
///   exponent-blind-parser hypothesis has nothing to misread here.
/// - **`raw * 0.001` reproduces exactly what [`crate::params::Engine::amount`]
///   already holds**, so the load-time scale is not the error either.
///
/// Substituting the real values, the **cap arm binds** at the recorded speed and the
/// original's own engine puts about **58** into the force accumulator there. That is
/// the same 58 this crate computes. So the thrust path is not wrong at all:
///
/// ```text
/// original at 24 units/s:  T = 58, and the recording is steady or decelerating
///                          => the original's total resistance there is ~58
/// this crate at 24:        drag 0.005 * 24^2 + rolling 2.0 = 4.88
/// ```
///
/// **So the missing factor of about 12 is a resistance that this crate does not
/// have**, and every constant in this module - `ENGINE_OUTPUT_SCALE`,
/// `ENGINE_OUTPUT_DOUBLE`, `amount`, `accelcap` - is now positively confirmed rather
/// than merely unrefuted. The 0.084 fitted earlier was fitting the wrong end of the
/// balance.
///
/// A precise target for whoever picks this up: a grounded quadratic coefficient of
/// `0.1` rather than `0.005` puts the equilibrium at **23.6 units/s** (`T = 57.60`
/// against `R = 57.70`), the exact floor of the recorded 23.6-25.1 band. That is
/// almost certainly a coincidence of magnitude rather than the mechanism - see the
/// dead end below - but it pins what has to be found: something contributing about
/// `0.095 * v^2` of opposing force while grounded.
///
/// Three hypotheses for *where* were checked and are dead:
///
/// - **Not the load-time scaling.** `HandlingXml_ParseEngine` (`0x0883945c`)
///   multiplies `amount` in place by exactly `0.001` and stores `accelcap` verbatim;
///   `oag_gameplay::handling::ENGINE_AMOUNT_SCALE` is `0.001` on the same field and
///   nothing else. Confidence 90, and now confirmed against the shipped value too.
/// - **Not a negative `accelcap`.** The cap feeds a plain `min` with no clamp at zero
///   anywhere before the accumulate, so a negative one would make `cap` negative at
///   low speed, `min` would select it over any non-negative `throttle * amount`, and
///   a ship at full throttle on the start line would be pushed *backwards* - unable
///   to reach the speed at which the cap turns positive. The shipped value is
///   positive, so this is now moot as well as impossible.
/// - **Not an asymmetry between the two force accumulators**, the most attractive
///   structural explanation: thrust goes through the body-*local* accumulator while
///   drag and rolling resistance go through the *world* one, but `Body_Integrate`
///   (`0x0015d088`, PS2) rotates the local one by the basis and then applies the same
///   `invMass * h` to both, which is exactly what [`crate::forces`]'s `drain` does.
///   Confidence 85.
///
/// Two further dead ends, both on the resistance side and both worth not re-testing:
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
///   `Body::mass == Physical::mass`, and a body mass an order of magnitude above the
///   parameter one would put that at 1.6.
///
/// # The track-section force was the lead, and it is the wrong sign
///
/// `FUN_08848f9c` was the only world-force writer both unimplemented here and
/// track-dependent, so it was the obvious candidate. It has now been read, along
/// with the two per-speed-class tables it indexes, and it is
/// **`Ship_ApplySpeedupPad`**: the speed-pad boost. The tables at `0x08b36bc0` and
/// `0x08b36bd0` are filled by `Xml_ReadGlobalSettings` (`0x0883a970`) from
/// `<GlobalClass name="..."><SpeedupPads amount="..." time="..."/></GlobalClass>`,
/// one entry per speed class. It pushes the craft *along* the section direction,
/// so it can only add speed. **It cannot be the missing resistance.**
///
/// # Two damping terms nobody had recorded - real, but far too small
///
/// Re-reading `Body_Integrate` turned up a pair of terms the physics docs do not
/// mention at all. Inside the sub-step loop, after both force accumulators:
///
/// ```text
/// velocity        -= velocity        * h * body+0x384
/// angularVelocity -= angularVelocity * h * body+0x380
/// ```
///
/// `Body_Init` (`0x0015cb98`, PS2) zeroes both, and the ship-entity constructor
/// (`FUN_00150d20`) then sets **both to `0.01`**, along with `body+0x388 = 0.4` and
/// `body+0x394 = 0.1`. So a real craft carries a linear velocity damping of `0.01`
/// per second that this crate does not implement. Confidence 80.
///
/// It should be implemented for fidelity, but it is **not** the gap: at `0.01` and
/// the `invMass` of `1.0` that `Body_Init` defaults to, it opposes motion with about
/// `0.24` of equivalent force at 24 units/s, against the ~53 that is missing.
///
/// # Resolved: the gap is thrust this crate applies and the original does not
///
/// `Ship_UpdateEngine` has an **early return that [`engine`] below does not
/// implement**. With flag `0x200` clear, `craft+0x290 > 0` makes it zero the throttle
/// state and return having written no thrust and no lift at all (`0x0884c634`, read
/// from disassembly with the branch-likely delay slots resolved; confidence 88).
/// `craft+0x2e0 > 0` does the same when flag `0x10` is clear.
///
/// `craft+0x290` is a **collision stun timer**. `Ship_ApplyLateralGrip` decrements it
/// and returns early while it runs, so a stunned craft also gets no lateral grip;
/// `Ship_ApplyCollisionImpulse` (`0x0883f274`) arms it with `craft+0x290 += 0.5` when
/// it applies a contact impulse. A hit costs half a second of engine *and* grip, and
/// repeated contact keeps re-arming it. Confidence 85.
///
/// That closes the balance. `Ship_UpdateCraft`'s callee set has now been enumerated
/// directly and contains no unaccounted force term, so nothing can supply the ~53 of
/// resistance the earlier reading demanded. A craft inside the stun window instead
/// has `T = 0`, leaving only `0.005 * 24^2 + 2.0 = 4.88` to decelerate it - gently and
/// monotonically, at `4.88 / m`. The capture falls from 24.271 to 23.571 across its
/// 200 ticks, about `-0.21` units/s^2, i.e. a craft mass near 23.
///
/// **So `ENGINE_OUTPUT_SCALE` is right, every constant here is right, and the drag
/// coefficients are right.** What is missing is the gate, not a magnitude. It is not
/// implemented here because it needs the contact impulse and the `craft+0x1c0` flag
/// word, and because [`engine`] has no state to hold a timer in - it belongs with the
/// collision response, alongside the stun's other half in the grip term.
///
/// # A caveat on the framing, which the resolution above supersedes
///
/// Everything above treats the recording as a *speed equilibrium*. It is a
/// **3.33-second window** (200 ticks), and over that long a slow transient toward a
/// far higher equilibrium is not distinguishable from a steady state by the speed
/// range alone. What makes it an equilibrium rather than a transient is the recorded
/// *sign*: the capture is described as very slightly decelerating, and a ship
/// climbing toward a higher equilibrium would be accelerating. The entire "resistance
/// is 12x short" conclusion rests on that one sign, so it is worth re-confirming
/// straight off the capture before anyone changes a coefficient on the strength of
/// it. If the sign is actually positive, the diagnosis flips again - to the *mass*,
/// which cancels out of an equilibrium but sets the whole timescale of a transient.
pub const ENGINE_OUTPUT_SCALE: f32 = 1.0;

/// The flag-gated engine multiplier: the speed-up pickup, a flat +20 % on thrust.
///
/// `if (flags & 0x0004) T *= craft+0x2a0`, in `Ship_UpdateEngine`. The only writer
/// that uses a craft base is `0x0883b3b8`, and it writes the value and the gating
/// flag in the same branch:
///
/// ```text
/// if ((pickup->0x1b8 & 0x800) == 0) { craft+0x2a0 = 0;   flags &= ~0x0004 }
/// else                              { craft+0x2a0 = 1.2; flags |=  0x0004 }
/// ```
///
/// so the `0` case is unreachable through the read: the flag is set only on the
/// branch that stores `0x3f99999a` (`1.2f`). Confidence 85; the constant is read
/// straight out of the instruction stream, and what is weaker is calling the `0x800`
/// pickup-flag bit "speed-up" specifically.
///
/// **Not applied.** Doing so needs the pickup system and the `craft+0x1c0` flag word,
/// neither of which this crate has, and on the Time Trial capture the trace is
/// compared against there is no pickup, so the multiplier is inert there anyway. It is
/// recorded because it was one of the two candidates for the missing 17x on thrust,
/// and it rules itself out: `1.2` cannot scale anything down.
pub const ENGINE_PICKUP_SPEEDUP: f32 = 1.2;

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
/// uncapped mode (`cap = 1e10`), the [`ENGINE_PICKUP_SPEEDUP`] multiplier, turbo and its boost
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
