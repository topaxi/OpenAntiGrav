//! Engine, brakes, steering and pitch: the control force law.
//!
//! All four are transcribed from `docs/ghidra/functions/psp-pulse-usa/engine.md`, which
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
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The two engine multipliers are
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
/// coefficients are right.** What was missing is the gate, not a magnitude. It is now
/// implemented: [`ShipState::stun_timer`] holds the timer, [`crate::wall::resolve`]
/// arms it, and [`engine`] and the lateral grip both read it.
///
/// # Which timer is armed in the reference capture is *not* established
///
/// The early return has two arms, and this analysis shows only that one of them must
/// have been taken. It does **not** show which:
///
/// - **The collision stun** (`craft+0x290`), if the run touched a wall. Sustained
///   contact keeps re-arming it, which would hold thrust at zero for the whole
///   window. Against this: a Time Trial run holding accelerate should not be scraping
///   a wall, and the stun also kills lateral grip, which would show as a visible
///   slide.
/// - **The leap timer** (`craft+0x2e0`), whose arming condition
///   [`ShipState::leap_timer`] records as unknown, with "a leap, a respawn and a race
///   start are all plausible". A capture taken near a race start would have it
///   running, which fits a clean straight-line run better than wall contact does.
///
/// **Neither timer is in the capture**, so no amount of re-reading the existing CSV
/// settles it. The decisive measurement is one line in `scripts/psp-trace.py`: record
/// `craft+0x290` and `craft+0x2e0` alongside the columns it already takes, and
/// re-capture. That distinguishes the two arms, and it would also confirm the gate
/// fired at all rather than leaving this a very good inference.
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
/// # Applied since 2026-08-11, gated on [`ShipState::turbo_timer`]
///
/// The gate is the one part of this that is not recovered. The original's is a bit
/// of `craft+0x1c0`, armed from the pickup word by the same function that forces the
/// HUD icon and reads the pickup's own `+0x148` timer - so the shape is "a held
/// pickup, for as long as it lasts", and a `f32` of seconds is that shape without
/// the two undecoded words. What is recovered is the **multiply and its constant**,
/// and where in `Ship_UpdateEngine` they land, which is what this is.
///
/// Two things support reading the `0x800` pickup as the **Turbo** weapon rather than
/// something else, neither conclusive on its own: the same branch forces HUD icon
/// id `6`, and `Arcade_HUD.xml` names its icons after the weapons in the class-name
/// pool's order; and the disc's own manual text says a time trial "will be given a
/// free turbo pickup once per lap", so a Turbo pickup exists and does something to
/// the engine. Confidence **80** on the identification, against 85 on the arithmetic.
/// See `docs/gameplay/pickups.md`.
///
/// **Not to be confused with `Engine.turbo`**, which is a flat *additive* thrust
/// under flag bits `0x200`/`0x400` and a mode enum. `0x400`'s only known writer is
/// the barrel roll (`docs/ghidra/functions/psp-pulse-usa/input-bindings.md`), so
/// that term belongs to a different mechanic and stays unimplemented here.
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
        // itself is no longer open - `docs/ghidra/functions/psp-pulse-usa/engine.md`
        // measured the original's basis as positively oriented under the ordinary
        // cross product, the same arithmetic this crate uses - but that measurement
        // says nothing about this particular row-to-axis mapping.
        Vec3::new(0.0, self.lift, -self.thrust)
    }
}

/// The engine.
///
/// ```text
/// if (craft+0x290 > 0 || (!(flags & 0x10) && craft+0x2e0 > 0))
///     && !(flags & 0x200)  { craft+0x2b8 = 0; return }   // no thrust, no lift
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
/// The early return **is** implemented, on both timers. The `0x0200` escape from it
/// is not, because nothing decodes that flag; the effect is that a stunned ship here
/// always loses its engine where the original might not. The `0x0010` escape on the
/// leap arm is not implemented either, for the same reason - and since nothing in
/// this crate arms [`ShipState::leap_timer`], that arm is inert today regardless.
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
    auto_speed: Option<f32>,
) -> EngineForce {
    // The prologue's early return, at `0x0884c634`. A stunned or leaping craft gets
    // no thrust and no lift at all - the original writes nothing to either
    // accumulator and returns. See `ShipState::stun_timer`.
    //
    // **The original's `craft+0x2b8 = 0` on this path is not reproduced.**
    // `ShipState::thrust` is rewritten from the input by `controls::update` every
    // tick, so zeroing it here would be overwritten before anything read it; the
    // original's store is observable only to the other readers of `craft+0x2b8`
    // within the stun, which are the HUD's throttle display. Keeping `engine` a
    // function of `&ShipState` is worth more than a cosmetic store.
    if state.stun_timer > 0.0 || state.leap_timer > 0.0 {
        return EngineForce::default();
    }

    // The four-corner branch, at `0x0884c834`. It replaces the whole throttle
    // path: the target is used as the output directly, and the `0.5 * speed +
    // accelcap` clamp below does not apply to it - the ordinary branch computes
    // that `min`, this one simply overwrites the slot.
    //
    // **The gate is approximated.** The original tests `(flags & 1) && !(flags &
    // 2)` on the undecoded `craft+0x1c0` and writes `0.0` when it fails. Bit 0 is
    // known to mean ground contact - it is the same bit that gates `brakes` - so
    // groundedness stands in for it here. Bit 1 is not decoded, and treating it
    // as always clear is the assumption; the effect is that a Zone craft here
    // always gets its auto-speed on the ground where the original might not.
    if let Some(target) = auto_speed {
        let thrust = if grounded > 0.0 { target } else { 0.0 };
        return EngineForce {
            thrust: thrust * ENGINE_OUTPUT_SCALE * ENGINE_OUTPUT_DOUBLE,
            lift: 0.0,
        };
    }

    let throttle = state.thrust;

    let mut thrust = throttle * handling.engine.amount;
    // Blended rather than switched, so a half-grounded ship gets 60 %.
    thrust = thrust * grounded + thrust * ENGINE_AIR_THRUST * (1.0 - grounded);

    let mut cap = 0.5 * forward_speed.abs() + handling.engine.accelcap;
    if throttle > crate::controls::CONTROL_MAX {
        cap *= throttle * 0.01;
    }
    thrust = thrust.min(cap);

    // `if (flags & 0x0004) T *= craft+0x2a0`, between the cap and the doubling,
    // which is where the original puts it - see `ENGINE_PICKUP_SPEEDUP`. It is
    // **after** the `min`, so a fired Turbo pushes the craft past the cap rather
    // than being clipped by it, which is what makes it worth firing at speed.
    if state.turbo_timer > 0.0 {
        thrust *= ENGINE_PICKUP_SPEEDUP;
    }

    thrust = thrust * ENGINE_OUTPUT_SCALE * ENGINE_OUTPUT_DOUBLE;

    EngineForce { thrust, lift: 0.0 }
}

/// Counts a fired Turbo pickup down.
///
/// Called once a tick from [`crate::step`], **after** the force law has read the
/// timer, so a pickup fired with `time` seconds on it boosts for exactly that
/// many seconds' worth of ticks rather than one fewer. That ordering is this
/// project's, not a reading: the original's equivalent is a bit and a timer in
/// two undecoded words, and nothing has been read that says when the bit clears.
/// See [`ENGINE_PICKUP_SPEEDUP`].
///
/// Floored at zero rather than allowed to go negative, so
/// `turbo_timer > 0.0` is the whole of the gate and a long-expired pickup does
/// not drift the determinism hash by an ever-growing negative.
pub fn advance_turbo(state: &mut ShipState, dt: f32) {
    state.turbo_timer = (state.turbo_timer - dt).max(0.0);
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
/// This function is the literal law, unmodified. The transcription nonetheless
/// predicts a yaw rate 22x higher than the original's, and what stands in for the
/// missing term is [`crate::forces::YAW_INVERSE_INERTIA`], applied once to the
/// whole yaw axis in [`crate::forces::evaluate`] rather than here - the airbrake's
/// yaw and bank-to-yaw share the discrepancy, and scaling only this term throws
/// their ratios out. Read that constant before changing anything here.
///
/// # Why the literal `steer * Turning.amount` is negated here
///
/// `docs/ghidra/functions/psp-pulse-usa/engine.md` ("The basis is positively
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

/// The pitch axis, as a body-local pitch torque.
///
/// ```text
/// p = controls.pitch * (grounded ? pitch_ground : pitch_air)
/// ```
///
/// `pitch_air` and `pitch_ground` are plain gains on the input axis, not rates:
/// there is no ramp and no state. The `grounded` test here is the **boolean**
/// contact flag rather than the 0/0.5/1 fraction, and it is the previous frame's.
///
/// **Not implemented:** the per-team in-air pitch bias at `stats_base + 0x90`,
/// which sits outside every class block and whose XML element is not known, so
/// there is no field in [`Handling`] to read it from. The gate at `FUN_088492bc`
/// is also not implemented and was never decoded, but it has now been *observed*
/// not to fire in the ordinary case: a grounded craft standing still on the start
/// line pitches the moment the axis is held, in
/// `data/traces/talons-junction-pitch-both-ways.csv`.
///
/// # The scale and the sign, both measured
///
/// Two things this used to get wrong, each recorded at the time as a guess
/// awaiting M3 and each now read out of the running game by probing the control
/// block the original's own `Ship_UpdatePitch` reads, `*(craft+0x78) + 0x10`:
///
/// - **The axis is on the `0..=100` control scale, not `-1..=1`.** `up` on the
///   d-pad writes exactly `-100` there and the analog stick's full deflection
///   writes `98.2`-`98.8`, which is the PSP's own byte quantisation of the same
///   thing. That is the same [`crate::controls::CONTROL_RANGE`] the steering axis
///   is on - steering reaches it through its ramp, and pitch, having no ramp,
///   has to be scaled here. Without it this term was **100x** weak.
/// - **The axis is negative-nose-up.** `up` on the d-pad writes `-100` and the
///   recorded forward row goes **down**; `down` writes `+100` and the nose
///   rises, each held for 120 ticks. That inversion is a *control binding*, not
///   a property of this term, so it lives in `oag_gameplay::ship_controls`
///   where the stick is mapped: [`ShipControls::steer_y`] stays
///   positive-nose-up and this expression stays the shape the original has.
///
/// The old note here said the handedness measurement "was about steering, not
/// pitch, and does not by itself pin this term's polarity", which was right - so
/// the polarity was measured on its own rather than inferred from it. Confidence
/// **90**: one binary and one emulator version, but the field is read directly
/// and the d-pad and the analog stick agree on it.
#[must_use]
pub fn pitch(controls: &ShipControls, handling: &Handling, grounded: bool) -> f32 {
    let gain = if grounded {
        handling.pitch.pitch_ground
    } else {
        handling.pitch.pitch_air
    };

    controls.steer_y * crate::controls::CONTROL_RANGE * gain
}

/// Below this many seconds left, the boost stops ramping and holds flat.
///
/// `Ship_ApplySpeedupPad`'s `0.1`. Its reciprocal is [`PAD_RAMP_RATE`], which is
/// what makes the two branches meet exactly rather than step: at the crossover
/// `amount * 10 * 0.1 == amount`. Worth stating because a reader meeting two
/// magic numbers in a ternary has no way to see that they are one number.
pub const PAD_RAMP_FLOOR: f32 = 0.1;

/// How fast the boost decays once the pad is behind the ship, per second.
///
/// `Ship_ApplySpeedupPad`'s `10.0`, and `1.0 / PAD_RAMP_FLOOR`. Applied to the
/// *remaining* time, so the force falls linearly from `amount * 10 * time` at the
/// moment of the last contact to `amount` at [`PAD_RAMP_FLOOR`] and then holds.
pub const PAD_RAMP_RATE: f32 = 10.0;

/// How far the pitch axis must be deflected before the speed-pad tilt counts as
/// held.
///
/// **This project's own number, and the single judgement call in
/// [`speedup_pad`].** The original tests a bit; there is no bit here, because
/// [`ShipControls`] is normalised by contract. Half deflection is the obvious
/// place to put the line: a D-pad or a keyboard produces exactly `-1.0` and is
/// never ambiguous, and an analog stick has to be pushed decisively rather than
/// brushed. It is **not** a recovered value and must not be quoted as one.
///
/// Compared at `<=` against a **negative** `steer_y`: see [`speedup_pad`]'s docs
/// for why up is negative on that axis.
pub const SPEEDPAD_JUMP_THRESHOLD: f32 = 0.5;

/// The speed-pad boost, into the **world** force accumulator.
///
/// `Ship_ApplySpeedupPad` (`0x08848f9c`), step 15 of `Ship_UpdateCraft` and the
/// last one this crate was missing. Confidence **85** on the force law, which was
/// read whole; **90** on the tunables being stored unscaled, from
/// `Xml_ReadGlobalSettings`.
///
/// ```text
/// if (inside a pad) { timer = time[class]; amount = amount[class]; dir = pad row 2 }
/// if (timer > 0) {
///     timer -= dt
///     f = (timer > 0.1) ? amount * 10 * timer : amount
///     if (craft+0x2cc < 1.0) f *= craft+0x2cc
///     worldForce += dir * f
/// }
/// ```
///
/// The decrement comes **before** the force is read off the timer. That is the
/// original's ordering and it is not cosmetic: it lowers the peak by one `dt` of
/// ramp, about 6 % at 60 Hz, and `docs/physics/cornering-ground-truth.md` measured
/// five pad crossings on one lap closely enough to prefer it.
///
/// # `<Special speedpad_jump>`, and the one judgement call in it
///
/// `if (controls->0x24 & 1) dir += craft+0x160 * g_speedpad_jump` is now here,
/// and neither half of it is a guess any more:
///
/// - **`craft+0x160` is the hull's up axis.** Live in PPSSPP its dot product
///   against the body's own up row (`body+0x010`) reads `+1.000000`, and
///   `0.000000` against both of the others; it is unit length. So the term tilts
///   the boost toward the sky in the *craft's* frame, which is what `up` is here.
/// - **`controls->0x24 & 1` is D-pad Up.** From a one-hot sweep of all twelve
///   buttons against `*(craft+0x78)+0x24`: only Up sets bit 0.
/// - **The magnitude is `<Special speedpad_jump>`**, read live at `0x08b36bec`
///   and `0.1` on both shipped discs, and it comes off the player's own disc
///   through [`Handling::speedpad_jump`] rather than being written down here.
///
/// **It is not a jump, and the name is misleading.** The sum is *not*
/// renormalised, so at `0.1` against a unit direction the boost tilts by
/// `atan(0.1)` - **5.71 degrees** - and gains `sqrt(1.01)`, **0.5 %** more force,
/// for at most the class's `time` (`0.24`-`0.27 s`, also read live). A hover
/// spring that holds the craft on a cushion absorbs most of a 5.7-degree tilt,
/// which is why nothing visibly leaps in the original either.
///
/// ## The judgement call: a normalised axis standing in for a digital button
///
/// The original gates on a **bit**. [`ShipControls`] is normalised by contract -
/// `oag-gameplay` owns that type and the simulation may not reach past it to the
/// input system - so there is no bit to test, and inventing a second control
/// input for one branch would be worse than the threshold.
///
/// [`SPEEDPAD_JUMP_THRESHOLD`] is that threshold, and **the sign is the
/// surprising half**: `oag_gameplay::ship_controls` maps D-pad Up to
/// `steer_y = -1`, because up on the stick pitches the nose *down* in this game
/// and the inversion lives at the input boundary. So the gate is
/// `steer_y <= -threshold`, not `>=`. Getting that backwards would tilt the boost
/// upward whenever the player pitches down - behaviour that looks deliberate,
/// which is the exact failure `crate::controls`' module docs warn about.
///
/// # One branch of the original is still deliberately not here
///
/// **The `craft+0x2cc` scale.** `if (craft+0x2cc < 1.0) f *= craft+0x2cc`, a
/// one-second fade-in. `craft+0x2cc` is written in exactly one place -
/// `Ship_UpdateEngine`'s prologue, as `flags & 0x200 ? 0.0 : craft+0x2cc + dt` -
/// so it is **seconds since flag `0x200` was last set**, and bit `0x200` is one
/// of the eleven undecoded bits of `craft+0x1c0`. It is emphatically *not* the
/// contact ratio, which an earlier reading of this function assumed: an unbounded
/// accumulator cannot be a `0..1` groundedness. Leaving it out means the boost is
/// at full strength from its first tick, which is what the trace measures anyway:
/// `0x200` is evidently not set during ordinary racing, or the captured peaks
/// would have been scaled down.
pub fn speedup_pad(
    state: &mut ShipState,
    controls: &ShipControls,
    handling: &Handling,
    up: Vec3,
    pad_hit: Option<Vec3>,
    dt: f32,
) -> Vec3 {
    // Assignment, not a maximum: standing on a pad holds the timer at full and
    // the countdown effectively starts when the ship leaves.
    if let Some(direction) = pad_hit {
        state.pad_timer = handling.speedup_pads.time;
        state.pad_direction = direction;
    }

    if state.pad_timer <= 0.0 {
        return Vec3::ZERO;
    }

    // The original lets this go slightly negative and gates on `> 0.0` next
    // frame; clamping is the same behaviour with a tidier state hash, since the
    // only other writer assigns.
    state.pad_timer = (state.pad_timer - dt).max(0.0);

    let amount = handling.speedup_pads.amount;
    let force = if state.pad_timer > PAD_RAMP_FLOOR {
        amount * PAD_RAMP_RATE * state.pad_timer
    } else {
        amount
    };

    // Read fresh every tick from *this* tick's input and *this* tick's hull, and
    // deliberately not baked into `pad_direction` when the pad arms it: the
    // original recomputes `dir` inside the `timer > 0` block, so a boost that
    // outlives the pad still tilts the moment the player pitches up, and stops
    // tilting the moment they let go.
    let direction = if controls.steer_y <= -SPEEDPAD_JUMP_THRESHOLD {
        state.pad_direction + up * handling.speedpad_jump
    } else {
        state.pad_direction
    };

    direction * force
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

        let force = engine(&state, &handling, 1.0, 40.0, None).as_local_force();
        // Body forward is -Z.
        assert!(force.z < 0.0, "force was {force:?}");
        assert_eq!(force.x, 0.0);
        assert_eq!(force.y, 0.0);
    }

    /// The whole point of the gate: a stunned ship at full throttle produces
    /// nothing. This is what stopped this crate accelerating to four times the
    /// original's speed, so it pins the behaviour rather than just the field.
    #[test]
    fn a_stunned_ship_produces_no_thrust_at_full_throttle() {
        let handling = test_handling();
        let mut state = ship_moving_forward(24.0);
        state.thrust = 100.0;

        let running = engine(&state, &handling, 1.0, 24.0, None);
        assert!(
            running.thrust > 0.0,
            "the un-stunned baseline must be non-zero"
        );

        state.stun_timer = 0.5;
        assert_eq!(
            engine(&state, &handling, 1.0, 24.0, None),
            EngineForce::default()
        );
    }

    /// The gate is `> 0`, so a timer that has counted exactly to zero is running
    /// again. An `>=` here would leave every ship permanently dead.
    #[test]
    fn a_stun_timer_at_exactly_zero_does_not_gate() {
        let handling = test_handling();
        let mut state = ship_moving_forward(24.0);
        state.thrust = 100.0;
        state.stun_timer = 0.0;

        assert!(engine(&state, &handling, 1.0, 24.0, None).thrust > 0.0);
    }

    /// The same early return has a second arm, on the leap timer.
    #[test]
    fn a_leaping_ship_produces_no_thrust_either() {
        let handling = test_handling();
        let mut state = ship_moving_forward(24.0);
        state.thrust = 100.0;
        state.leap_timer = 1.0;

        assert_eq!(
            engine(&state, &handling, 1.0, 24.0, None),
            EngineForce::default()
        );
    }

    #[test]
    fn no_throttle_is_no_thrust() {
        let handling = test_handling();
        let state = ship_moving_forward(40.0);
        assert_eq!(engine(&state, &handling, 1.0, 40.0, None).thrust, 0.0);
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

        let grounded = engine(&state, &handling, 1.0, 0.0, None).thrust;
        let airborne = engine(&state, &handling, 0.0, 0.0, None).thrust;
        let half = engine(&state, &handling, 0.5, 0.0, None).thrust;

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

        let stationary = engine(&state, &handling, 1.0, 0.0, None).thrust;
        let fast = engine(&state, &handling, 1.0, 200.0, None).thrust;
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
            engine(&state, &handling, 1.0, 60.0, None).thrust,
            engine(&state, &handling, 1.0, -60.0, None).thrust
        );
    }

    #[test]
    fn a_zero_parameter_engine_produces_nothing() {
        let mut state = ship_moving_forward(40.0);
        state.thrust = 100.0;
        assert_eq!(engine(&state, &Handling::ZERO, 1.0, 40.0, None).thrust, 0.0);
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
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md` this pins against. A test that
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

    /// The gains, on the control scale the axis was measured to be on.
    ///
    /// The expected value carries [`crate::controls::CONTROL_RANGE`] and a
    /// The expected value carries [`crate::controls::CONTROL_RANGE`], which is a
    /// measurement rather than bookkeeping: the original's pitch axis reads
    /// `+/-100`, not `-1..=1`. See [`pitch`]. This test previously asserted the
    /// bare gain and passed while the term was 100x weak.
    #[test]
    fn the_pitch_axis_uses_a_different_gain_on_the_ground_than_in_the_air() {
        let handling = test_handling();
        let nose_up = ShipControls {
            steer_y: 1.0,
            ..ShipControls::default()
        };
        let scale = crate::controls::CONTROL_RANGE;

        assert_eq!(
            pitch(&nose_up, &handling, true),
            scale * handling.pitch.pitch_ground
        );
        assert_eq!(
            pitch(&nose_up, &handling, false),
            scale * handling.pitch.pitch_air
        );
    }

    /// A nose-up request must produce a positive body-local pitch torque.
    ///
    /// `oag_physics`' body frame is right-handed on `(x right, y up, z back)` and
    /// forward is `-Z`, so `omega = +k * x` swings the nose toward `+up`. The
    /// original's own chain agrees through a different convention: `down` on the
    /// d-pad writes `+100` to the pitch axis, that lands in the game-side
    /// accumulator, and the game's basis advance (`e' = e x omega`) tips its
    /// forward row up. Both routes say the sign of this term is the sign of "nose
    /// up", which is what the assertion below is.
    #[test]
    fn a_nose_up_request_pitches_the_nose_up() {
        let handling = test_handling();
        let nose_up = ShipControls {
            steer_y: 1.0,
            ..ShipControls::default()
        };

        assert!(
            pitch(&nose_up, &handling, false) > 0.0,
            "steer_y is documented positive nose up, so this must be a positive \
             body-local pitch torque; got {}",
            pitch(&nose_up, &handling, false)
        );
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

#[cfg(test)]
mod auto_speed_tests {
    use super::{ENGINE_OUTPUT_DOUBLE, ENGINE_OUTPUT_SCALE, engine};
    use crate::params::Handling;
    use crate::ship::ShipState;

    /// A ship with the throttle released, which is the interesting case: Zone
    /// mode accelerates with nothing held down.
    fn grounded_ship() -> ShipState {
        ShipState {
            thrust: 0.0,
            ..ShipState::default()
        }
    }

    #[test]
    fn the_auto_speed_branch_ignores_the_throttle_entirely() {
        let mut state = grounded_ship();
        let handling = Handling::ZERO;

        // Throttle at zero, and yet there is thrust: that is the point of the
        // mode. A Zone craft accelerates with nothing held down.
        let idle = engine(&state, &handling, 1.0, 0.0, Some(50.0)).thrust;
        state.thrust = 100.0;
        let full = engine(&state, &handling, 1.0, 0.0, Some(50.0)).thrust;
        assert_eq!(idle, full, "the throttle changed the auto-speed output");
        assert_eq!(idle, 50.0 * ENGINE_OUTPUT_SCALE * ENGINE_OUTPUT_DOUBLE);
    }

    #[test]
    fn the_auto_speed_branch_is_not_capped_by_accelcap() {
        // The ordinary branch clamps against `0.5 * speed + accelcap`, and
        // `Handling::ZERO` makes that clamp zero. The four-corner branch has no
        // clamp at all, so a large target survives it.
        let state = grounded_ship();
        let force = engine(&state, &Handling::ZERO, 1.0, 0.0, Some(10_000.0)).thrust;
        assert!(force > 0.0, "the cap bound a branch that has no cap");
    }

    #[test]
    fn an_airborne_zone_craft_gets_nothing() {
        // The gate: the original writes `0.0` when the contact bit is clear.
        let state = grounded_ship();
        let force = engine(&state, &Handling::ZERO, 0.0, 0.0, Some(50.0)).thrust;
        assert_eq!(force, 0.0);
    }

    #[test]
    fn without_a_target_the_ordinary_path_is_untouched() {
        let mut state = grounded_ship();
        state.thrust = 100.0;
        let handling = Handling::ZERO;
        assert_eq!(
            engine(&state, &handling, 1.0, 40.0, None),
            engine(&state, &handling, 1.0, 40.0, None)
        );
        // `Handling::ZERO` has no engine amount, so the ordinary path is zero and
        // the auto-speed path is not. That difference is the whole branch.
        assert_eq!(engine(&state, &handling, 1.0, 40.0, None).thrust, 0.0);
        assert!(engine(&state, &handling, 1.0, 40.0, Some(50.0)).thrust > 0.0);
    }
}

#[cfg(test)]
mod speedup_pad_tests {
    use super::{PAD_RAMP_FLOOR, PAD_RAMP_RATE, SPEEDPAD_JUMP_THRESHOLD, speedup_pad};
    use crate::params::{Handling, SpeedupPads};
    use crate::ship::{ShipControls, ShipState};
    use oag_core::math::Vec3;

    /// Upright, with the pitch axis centred: the tilt branch not taken.
    const UP: Vec3 = Vec3::Y;

    fn centred() -> ShipControls {
        ShipControls::default()
    }

    /// The pitch axis held the way the input layer reports d-pad Up, which is
    /// **negative**. See `speedup_pad`'s own documentation for why.
    fn pitched_up() -> ShipControls {
        ShipControls {
            steer_y: -1.0,
            ..ShipControls::default()
        }
    }

    /// Round invented numbers. `time` is deliberately well above
    /// [`PAD_RAMP_FLOOR`] so both branches of the law are reachable.
    fn padded_handling() -> Handling {
        Handling {
            speedup_pads: SpeedupPads {
                amount: 2.0,
                time: 0.5,
            },
            ..Handling::ZERO
        }
    }

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn a_ship_that_has_touched_no_pad_gets_no_boost() {
        let mut state = ShipState::default();
        assert_eq!(
            speedup_pad(&mut state, &centred(), &padded_handling(), UP, None, DT),
            Vec3::ZERO
        );
        assert_eq!(state.pad_timer, 0.0);
    }

    /// The shape of the law: `amount * 10 * remaining` while there is more than
    /// [`PAD_RAMP_FLOOR`] left, flat `amount` after that, and **the decrement
    /// happens first** - so the very first tick is already one `dt` down the ramp
    /// rather than at `amount * 10 * time`.
    #[test]
    fn the_boost_ramps_down_and_then_holds_flat() {
        let handling = padded_handling();
        let mut state = ShipState::default();

        let first = speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::NEG_Z), DT);
        let peak = handling.speedup_pads.amount * PAD_RAMP_RATE * (handling.speedup_pads.time - DT);
        assert!((first.length() - peak).abs() < 1e-3, "{first} vs {peak}");
        assert!(
            first.length()
                < handling.speedup_pads.amount * PAD_RAMP_RATE * handling.speedup_pads.time,
            "reading the force before the decrement would give the larger value"
        );

        // Falls monotonically while the ramp branch is live.
        let mut previous = first.length();
        while state.pad_timer > PAD_RAMP_FLOOR {
            let now = speedup_pad(&mut state, &centred(), &handling, UP, None, DT).length();
            assert!(now < previous, "{now} is not below {previous}");
            previous = now;
        }

        // And then holds flat at `amount` until the timer runs out.
        while state.pad_timer > 0.0 {
            let now = speedup_pad(&mut state, &centred(), &handling, UP, None, DT).length();
            assert!((now - handling.speedup_pads.amount).abs() < 1e-6, "{now}");
        }
        assert_eq!(
            speedup_pad(&mut state, &centred(), &handling, UP, None, DT),
            Vec3::ZERO,
            "an expired boost applies nothing"
        );
    }

    /// The two branches meet rather than step, which is the whole reason the
    /// constants are a reciprocal pair.
    #[test]
    fn the_ramp_and_the_flat_branch_meet_at_the_crossover() {
        assert_eq!(PAD_RAMP_RATE * PAD_RAMP_FLOOR, 1.0);
    }

    /// Standing on a pad re-arms the timer every tick, so the force does not
    /// decay while the ship is still inside the volume. This is what makes a slow
    /// crossing boost for longer than a fast one, and it is why
    /// `Environment::pad_hit` is a per-tick containment answer rather than an
    /// entry edge.
    #[test]
    fn staying_inside_a_pad_holds_the_boost_at_its_peak() {
        let handling = padded_handling();
        let mut state = ShipState::default();

        let first =
            speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::NEG_Z), DT).length();
        for _ in 0..120 {
            let now =
                speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::NEG_Z), DT).length();
            assert!((now - first).abs() < 1e-6, "{now} drifted from {first}");
        }
    }

    /// Leaving the pad does not re-aim the remaining boost. A ship that turns
    /// after crossing keeps being pushed the way the pad faced, which is the
    /// original holding `craft+0x1b0` rather than recomputing it.
    #[test]
    fn the_push_direction_is_held_after_the_pad_is_behind_the_ship() {
        let handling = padded_handling();
        let mut state = ShipState::default();

        speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::X), DT);
        let later = speedup_pad(&mut state, &centred(), &handling, UP, None, DT);
        assert_eq!(later.normalize(), Vec3::X);
        assert_eq!(state.pad_direction, Vec3::X);
    }

    /// `<Special speedpad_jump>`: the numbers, not just the shape. At the live
    /// `0.1` against a unit pad direction the boost tilts by `atan(0.1)` and
    /// gains `sqrt(1.01)`, which is the arithmetic that makes this "not a jump".
    #[test]
    fn the_tilt_is_a_small_angle_and_a_smaller_force_increase() {
        let handling = Handling {
            speedpad_jump: 0.1,
            ..padded_handling()
        };
        let mut plain = ShipState::default();
        let mut tilted = ShipState::default();

        let without = speedup_pad(&mut plain, &centred(), &handling, UP, Some(Vec3::X), DT);
        let with = speedup_pad(&mut tilted, &pitched_up(), &handling, UP, Some(Vec3::X), DT);

        let angle = without
            .normalize()
            .dot(with.normalize())
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        assert!((angle - 5.71).abs() < 0.01, "{angle} degrees");

        let gain = with.length() / without.length();
        assert!((gain - 1.01f32.sqrt()).abs() < 1e-5, "{gain}");
        // Toward the hull's up, not away from it and not sideways.
        assert!(with.dot(UP) > 0.0 && without.dot(UP) == 0.0, "{with}");
    }

    /// The gate is on the **negative** side of the pitch axis, because that is
    /// where `oag_gameplay::ship_controls` puts d-pad Up. A reimplementation that
    /// took the positive side would tilt on nose-down, which is the failure this
    /// pins.
    #[test]
    fn only_a_pitch_up_input_tilts_the_boost() {
        let handling = Handling {
            speedpad_jump: 0.5,
            ..padded_handling()
        };
        let plain = {
            let mut state = ShipState::default();
            speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::X), DT)
        };

        for (name, steer_y, tilts) in [
            ("centred", 0.0, false),
            ("nose down, full", 1.0, false),
            (
                "nose down, past the threshold",
                SPEEDPAD_JUMP_THRESHOLD,
                false,
            ),
            ("nose up, just under the threshold", -0.49, false),
            ("nose up, on the threshold", -SPEEDPAD_JUMP_THRESHOLD, true),
            ("nose up, full", -1.0, true),
        ] {
            let controls = ShipControls {
                steer_y,
                ..ShipControls::default()
            };
            let mut state = ShipState::default();
            let force = speedup_pad(&mut state, &controls, &handling, UP, Some(Vec3::X), DT);
            assert_eq!(force.dot(UP) > 0.0, tilts, "{name}: {force}");
            if !tilts {
                assert_eq!(force, plain, "{name} moved the boost");
            }
        }
    }

    /// The tilt is read from *this* tick's input, not baked into
    /// `pad_direction` when the pad armed the boost. So it can start and stop
    /// while the boost runs on behind the pad, which is what the original's
    /// recomputation of `dir` inside the timer block does.
    #[test]
    fn the_tilt_follows_the_input_rather_than_the_pad() {
        let handling = Handling {
            speedpad_jump: 0.5,
            ..padded_handling()
        };
        let mut state = ShipState::default();

        // Crossed with the axis centred, so nothing is stored tilted.
        speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::X), DT);
        assert_eq!(state.pad_direction, Vec3::X, "the stored axis is the pad's");

        // Pitch up after the pad: the remaining boost tilts anyway.
        let after = speedup_pad(&mut state, &pitched_up(), &handling, UP, None, DT);
        assert!(after.dot(UP) > 0.0, "{after}");
        assert_eq!(state.pad_direction, Vec3::X, "and the state stays clean");

        // Let go: it stops tilting on the very next tick.
        let released = speedup_pad(&mut state, &centred(), &handling, UP, None, DT);
        assert_eq!(released.dot(UP), 0.0, "{released}");
    }

    /// A disc whose `<Special speedpad_jump>` is zero, or a load that could not
    /// read the file, boosts exactly as it did before this branch existed.
    #[test]
    fn a_zero_jump_changes_nothing() {
        let handling = padded_handling();
        assert_eq!(handling.speedpad_jump, 0.0, "the fixture's default");
        let mut plain = ShipState::default();
        let mut pitched = ShipState::default();
        assert_eq!(
            speedup_pad(&mut plain, &centred(), &handling, UP, Some(Vec3::X), DT),
            speedup_pad(
                &mut pitched,
                &pitched_up(),
                &handling,
                UP,
                Some(Vec3::X),
                DT
            )
        );
    }
}
