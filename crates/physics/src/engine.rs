//! Engine, brakes, steering and pitch: the control force law.
//!
//! Transcribed from `docs/ghidra/functions/psp-pulse-usa/engine.md`, which read them out
//! of `BOOT.BIN`. The page caps itself at 84 because nothing here is runtime-verified.
//!
//! Not what a reimplementation would guess:
//!
//! - **The throttle is not ramped.** `<Engine gain/>` and `<Engine falloff/>` are dead
//!   ([`crate::params::Engine::falloff`]).
//! - **There is no brake axis.** The brake engages when both airbrake inputs are positive
//!   and is applied only while grounded.
//! - **`Brakes.amount` is negative in memory** and the force is applied along
//!   `+unit(velocity)`. Negating here as well would accelerate under braking.
//! - **Every term here reads the *previous* frame's groundedness.** Hover is step 8 of 15
//!   and clears the contact flag on entry ([`crate::ship::ShipState::grounded_prev`]).
//!
//! **No control input writes an angular Z component** (engine, brakes, steering, pitch,
//! airbrakes): roll is never commanded. Angular Z is written only by the angular damping
//! and the surface-alignment torque. Confidence 80 on the narrowed negative; the page's
//! original "no Z component is ever written" was too strong.

use oag_core::math::Vec3;

use crate::params::Handling;
use crate::ship::{ShipControls, ShipState};

/// The start-line boost multiplier the engine scales its output by, `1.0` while racing.
///
/// `T = T * craft+0x294 * 2.0`, unconditionally, at the end of `Ship_UpdateEngine`.
/// `craft+0x294` is the start-line boost multiplier (`engine.md`, "The two engine
/// multipliers are recovered"). Three writers agree it is `1.0` outside the start window:
/// `Ship_InitCraft` (`0x08849354`), `Race_ResetCraftBoosts_q` (`0x088271b4`) and
/// `Ship_UpdateStartBoost` (`0x0883fdec`), which drives it from a
/// `windowStart`/`overallDuration`/`stallMul`/`normalMul`/`boostMul` XML group and then
/// stores `1.0` back. A **recovered value**, confidence 88. It is
/// [`crate::launch::LaunchState::multiplier`]: `1.0` outside the launch window and
/// `<StartBoost>`'s figures inside it ([`crate::launch`]).
///
/// # The 17x speed gap was a missing gate, not a constant
///
/// The reference capture (`docs/reverse-engineering/ppsspp-debugger.md`; Time Trial,
/// Venom, Talon's Junction White, Assegai) holds **23.6 to 25.1 units/s at throttle 100**
/// and decelerates very slightly, while this crate passed 55 by tick 30 and had no
/// equilibrium below about 128. Every constant was checked and cleared, with the
/// evidence in `engine.md`: the load-time `amount * 0.001`
/// (`HandlingXml_ParseEngine`, `0x0883945c`, confidence 90), a negative `accelcap`, the
/// two force accumulators (`Body_Integrate`, `0x0015d088` PS2, confidence 85), the drag
/// coefficients (`-0.005`/`-0.002`/`-0.1`, `Ship_ApplyQuadraticDrag` `0x0015c3a0`,
/// confidence 88), the mass (hover height 4.002 against 3.978 predicted for equal
/// masses), and `Ship_ApplySpeedupPad` (`0x08848f9c`), which only adds speed. The
/// `Body_Integrate` linear/angular velocity damping (`body+0x384`/`+0x380`, both `0.01`,
/// confidence 80) is real but opposes only about `0.24` of the missing ~53.
///
/// **The resolution:** `Ship_UpdateEngine` has an early return that [`engine`]
/// implements. With flag `0x200` clear, `craft+0x290 > 0` zeroes the throttle state and
/// returns having written no thrust and no lift (`0x0884c634`, confidence 88);
/// `craft+0x2e0 > 0` does the same when flag `0x10` is clear. `craft+0x290` is the
/// collision stun timer (`Ship_ApplyCollisionImpulse`, `0x0883f274`, arms it with
/// `+= 0.5`; confidence 85). In the stun window `T = 0` and only `0.005 * 24^2 + 2.0 =
/// 4.88` decelerates the craft, matching the capture's `-0.21` units/s^2 at a mass near
/// 23. See [`ShipState::stun_timer`], [`crate::wall::resolve`].
///
/// **Not established:** which arm fired in the reference capture. A Time Trial has no
/// weapon in the air (making the slowdown arm unlikely, an inference from the recovered
/// law) and holding accelerate should not scrape a wall (making the stun unlikely too).
/// Neither timer is in the capture; recording `craft+0x290` and `craft+0x2e0` in
/// `scripts/psp-trace.py` and re-capturing would settle it.
///
/// The "equilibrium" reading rests on the recording's slight deceleration over a
/// 3.33-second window. If that sign were positive the diagnosis would flip to the mass,
/// so re-confirm it off the capture before changing a coefficient.
pub const ENGINE_OUTPUT_SCALE: f32 = crate::launch::LaunchState::NEUTRAL;

/// The flag-gated engine multiplier: the speed-up pickup, a flat +20 % on thrust.
///
/// `if (flags & 0x0004) T *= craft+0x2a0`, in `Ship_UpdateEngine`. The only writer using a
/// craft base is `0x0883b3b8`, which writes the value and the gating flag in one branch
/// (`craft+0x2a0 = 1.2` with the flag set, `0` with it cleared), so the `0` case is
/// unreachable through the read. Confidence 85; weaker is calling the `0x800`
/// pickup-flag bit "speed-up".
///
/// # Still not applied
///
/// It was briefly wired up as the Turbo pickup's effect, **wrongly, by two orders of
/// magnitude**: a `1.2` multiplier on thrust already clamped to `0.5 * speed +
/// accelcap` is a few units of force, while `Engine.turbo`, added *uncapped* on the next
/// branch, is authored an order of magnitude above `accelcap` on every shipped class.
/// Also, the arming branch is in the **HUD update** (`0x0883b3b8`), sets HUD icon id `6`
/// and drives a bar from the pickup's `+0x148` timer, so `0x800` is a timed effect the HUD
/// fills. Which pickup arms it is unidentified (id `6` lands on `Shield` or `Autopilot`
/// depending on where the class-name pool starts counting); it stays unapplied. See
/// [`ShipState::turbo_timer`] and `docs/gameplay/pickups.md`.
pub const ENGINE_PICKUP_SPEEDUP: f32 = 1.2;

/// The fixed doubling on the engine's output, from the same expression.
pub const ENGINE_OUTPUT_DOUBLE: f32 = 2.0;

/// The fraction of thrust available with no ground contact:
/// `T = T * grounded + T * 0.2 * (1 - grounded)`, blended through half-grounded, not
/// switched.
pub const ENGINE_AIR_THRUST: f32 = 0.2;

/// Speed below which the brake force fades out linearly:
/// `if (speed < 10.0) dir *= speed * 0.1`. Reaches zero at a standstill, which also avoids
/// `velocity / speed` at rest.
pub const BRAKE_FADE_SPEED: f32 = 10.0;

/// What the engine contributed, in the body-local force accumulator.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EngineForce {
    /// Thrust along body forward, the accumulator's `.z`.
    pub thrust: f32,
    /// Boost lift along body up, the accumulator's `.y`.
    /// Boost lift along body up, the accumulator's `.y`.
    ///
    /// **Always zero here.** It is non-zero only under the turbo flags (bits of the
    /// undecoded `craft+0x1c0`) and the boost scale is a global that was not read.
    ///
    /// The original's final accumulate reads a VFPU register for this lane that the
    /// function never loads; the reading is that the caller's `vzero.q` left it at zero,
    /// so the result is `accumulator.y = lift`. Confidence 65.
    pub lift: f32,
}

impl EngineForce {
    /// As a body-local force vector. Body forward is `-Z`, so thrust is negated into `.z`.
    #[must_use]
    pub fn as_local_force(self) -> Vec3 {
        // The original's row 2 and accumulator `.z` are "forward"; this crate's body
        // forward is `-Z` (`Body::forward`), so the negation is a convention difference,
        // not a sign finding. Handedness is not open (`engine.md` measured the original's
        // basis as positively oriented) but that says nothing about this row-to-axis map.
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
/// if (craft+0x31c < 1.0) { T *= craft+0x31c; craft+0x31c = 1.0 }  // one-shot scale
/// ```
///
/// `speed` is `|dot(velocity, forward)|`, not `|velocity|`. `grounded` is the
/// previous frame's 0/0.5/1 fraction.
/// `speed` is `|dot(velocity, forward)|`, not `|velocity|`. `grounded` is the previous
/// frame's 0/0.5/1 fraction.
///
/// The early return **is** implemented, on both timers. Its `0x0200` escape and the
/// slowdown arm's `0x0010` escape are not (nothing decodes those flags), so a stunned
/// ship here always loses its engine where the original might not. The slowdown arm is
/// live: [`crate::slowdown::add`] arms [`ShipState::slowdown_timer`] from a weapon impact.
///
/// **The one-shot scale at `craft+0x31c` is implemented, as `thrust_scale`.** A value
/// test, not a flag test: below `1.0` it multiplies the doubled thrust once on both the
/// throttle and four-corner branches, and the original writes `1.0` back. Its one writer
/// is the LeachBeam's drain (`slowShipFactor`, `Ship_ApplyPendingWeaponDamage`), so the
/// caller holds the armed value
/// ([`crate::forces::Environment::thrust_scale`]) and does the write-back.
///
/// **Not implemented**, all flag-gated on the undecoded `craft+0x1c0`: the uncapped mode
/// (`cap = 1e10`), the [`ENGINE_PICKUP_SPEEDUP`] multiplier, turbo's boost lift and the
/// kill switch at bit `0x2000`. Implementing them would mean inventing their triggers. (The
/// four-corner mode's `(flags & 1) && !(flags & 2)` gate is groundedness and
/// [`ShipState::on_grid`].)
/// implementing them would mean inventing their triggers.
#[must_use]
pub fn engine(
    state: &ShipState,
    handling: &Handling,
    grounded: f32,
    forward_speed: f32,
    auto_speed: Option<f32>,
    thrust_scale: f32,
) -> EngineForce {
    // The prologue's early return (`0x0884c634`): a stunned or weapon-slowed craft gets no
    // thrust and no lift. The original's `craft+0x2b8 = 0` here is **not reproduced**:
    // `controls::update` rewrites `ShipState::thrust` every tick, so it would be
    // overwritten before anything read it, and only the HUD throttle display sees it.
    // Keeping `engine` a function of `&ShipState` is worth more than a cosmetic store.
    if state.stun_timer > 0.0 || state.slowdown_timer > 0.0 {
        return EngineForce::default();
    }

    // The four-corner branch (`0x0884c834`) replaces the whole throttle path: the target
    // is the output directly and the `0.5 * speed + accelcap` clamp does not apply.
    //
    // **The gate is approximated.** The original tests `(flags & 1) && !(flags & 2)` on
    // the undecoded `craft+0x1c0` and writes `0.0` when it fails. Bit 0 is ground contact
    // (the bit that gates `brakes`), so groundedness stands in; bit 1 is the grid state.
    if let Some(target) = auto_speed {
        // Bit 1 of `craft+0x1c0` is the grid state (`Craft_EnterGridState`, `0x088486d4`),
        // so `on_grid` is that bit. Read live 2026-10-02 on a Zone engine: `flags` `0x3`
        // through state 0 with the craft still (`0.02` units/s), `0x1` and `95.2` thrust
        // from the first state-1 frame.
        let thrust = if grounded > 0.0 && !state.on_grid {
            target
        } else {
            0.0
        };
        return EngineForce {
            // **The launch multiplier applies here too**: the tail is shared
            // (`craft+0x294`, `0x0884c918`) and reads no mode. Live on a Zone engine
            // 2026-10-02: GO read `34.0 * 1.4 * 2` = `95.2` coasting (grade 0) and `1.2`
            // held (grade 1), then `1.0` after 60 frames.
            thrust: one_shot_scale(
                thrust * state.launch.multiplier * ENGINE_OUTPUT_SCALE * ENGINE_OUTPUT_DOUBLE,
                thrust_scale,
            ),
            lift: 0.0,
        };
    }

    let throttle = state.thrust;

    let mut thrust = throttle * handling.engine.amount;
    // Blended, not switched: a half-grounded ship gets 60 %.
    thrust = thrust * grounded + thrust * ENGINE_AIR_THRUST * (1.0 - grounded);

    let mut cap = 0.5 * forward_speed.abs() + handling.engine.accelcap;
    if throttle > crate::controls::CONTROL_MAX {
        cap *= throttle * 0.01;
    }
    thrust = thrust.min(cap);

    // The turbo add, `T += Engine.turbo`, sits between the cap and the doubling as in the
    // original, so it is **uncapped**. Either source arms it, as the original's
    // `(flags & 0x200) || (flags & 0x400)` reads: a fired Turbo pickup
    // ([`ShipState::turbo_timer`]) or a barrel roll's landing payout
    // ([`ShipState::roll_payout_timer`]); see `input-bindings.md`.
    if state.turbo_timer > 0.0 || state.roll_payout_timer > 0.0 {
        thrust += handling.engine.turbo;
    }

    thrust = one_shot_scale(
        thrust * state.launch.multiplier * ENGINE_OUTPUT_DOUBLE,
        thrust_scale,
    );

    EngineForce { thrust, lift: 0.0 }
}

/// `if (craft+0x31c < 1.0) T *= craft+0x31c`, after the doubling, on both branches; the
/// last thing `Ship_UpdateEngine` does to `T` before the kill switch. Lift is not scaled.
fn one_shot_scale(thrust: f32, scale: f32) -> f32 {
    if scale < 1.0 { thrust * scale } else { thrust }
}

/// Counts a fired Turbo pickup down.
///
/// Called once a tick from [`crate::step`], **after** the force law has read the timer, so
/// the tick a pickup fires on is boosted. That ordering is this project's, not a reading
/// (the original keeps a bit and a timer in two undecoded words); see
/// [`ENGINE_PICKUP_SPEEDUP`].
///
/// **It boosts one tick more than the arithmetic suggests**: a `0.75` s pickup runs 46
/// ticks at 60 Hz, not 45, because 45 sequential `f32` subtractions of `1/60` leave a
/// residue above zero. Pinned by `a_fired_turbo_multiplies_thrust_for_its_authored_duration`.
///
/// Floored at zero so `turbo_timer > 0.0` is the whole gate and an expired pickup does
/// not drift the determinism hash.
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
/// The caller must apply the grounded gate: `Ship_UpdateBrakes` runs only when the
/// *previous* frame's contact flag is set, and only outside the four-corner mode.
///
/// A VFPU target prefix on the accumulate reads, under the standard selector, as zeroing
/// the world `y` lane (a horizontal brake force). **Confidence 55**, the prefix encoding
/// unconfirmed, so not implemented; low stakes since braking runs only while grounded,
/// where velocity is nearly horizontal. Awaiting M3.
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
/// `Turning.amount` feeds body-local yaw **directly, with no speed factor**, so steering
/// at a standstill is not zero.
///
/// This is the literal law, but the transcription predicts a yaw rate 22x the original's;
/// [`crate::forces::YAW_INVERSE_INERTIA`] stands in for the missing term, applied once to
/// the whole yaw axis in [`crate::forces::evaluate`] (the airbrake's yaw and bank-to-yaw
/// share the discrepancy). Read it before changing anything here.
///
/// # Why the literal `steer * Turning.amount` is negated here
///
/// `engine.md` ("The basis is positively oriented, and row 0 points left") measured a
/// running race: holding right yaws at a mean `-1.42 rad/s` about up, left `+1.51 rad/s`.
/// The law is **`yaw_rate = -k * steer`**, confidence 90, a plain sign and not a
/// handedness flip (the page rules handedness out). `steer` is positive to the right
/// ([`ShipControls::steer_x`]) and this crate needs a negative `local_angular.y` to turn
/// the nose right, as in the weathervane test in `crate::passive`. The base `yaw` is
/// negated before the reverse-controls blend.
///
/// The blend is applied unconditionally: it is the identity at zero
/// (`yaw * (1 - 2 * 0) == yaw`).
///
/// **Not implemented:** the steering bias at `craft+0x2e4` behind flag `0x20`, and the two
/// `craft+0x2a4` modes that force `steer = 0`; both need undecoded state.
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
/// `pitch_air` and `pitch_ground` are plain gains on the input, with no ramp and no state.
/// The `grounded` test is the **boolean** contact flag, previous frame's.
///
/// **Not implemented:** the per-team in-air pitch bias at `stats_base + 0x90` (outside
/// every class block, XML element unknown, so no [`Handling`] field), and the gate at
/// `FUN_088492bc` (never decoded, but observed not to fire in the ordinary case: a
/// grounded craft on the start line pitches the moment the axis is held, in
/// `data/traces/talons-junction-pitch-both-ways.csv`).
/// # The scale and the sign, both measured
///
/// Read by probing the control block the original's `Ship_UpdatePitch` reads,
/// `*(craft+0x78) + 0x10`. Confidence **90** (one binary and emulator version, but the
/// field is read directly and the d-pad and analog stick agree):
///
/// - **The axis is on the `0..=100` control scale, not `-1..=1`:** d-pad up writes
///   exactly `-100`, the stick's full deflection `98.2`-`98.8` (the PSP's byte
///   quantisation). It is the same [`crate::controls::CONTROL_RANGE`] as steering, which
///   reaches it through its ramp; pitch has no ramp, so it is scaled here. Without it
///   this term was **100x** weak.
/// - **The axis is negative-nose-up:** up writes `-100` and the forward row goes down;
///   down writes `+100` and the nose rises (120 ticks each). That is a *control binding*,
///   so it lives in `oag_gameplay::ship_controls`: [`ShipControls::steer_y`] stays
///   positive-nose-up.
#[must_use]
pub fn pitch(controls: &ShipControls, handling: &Handling, grounded: bool) -> f32 {
    let gain = if grounded {
        handling.pitch.pitch_ground
    } else {
        handling.pitch.pitch_air
    };

    controls.steer_y * crate::controls::CONTROL_RANGE * gain
}

/// Below this many seconds left, the boost stops ramping and holds flat: the `0.1` of
/// `Ship_ApplySpeedupPad`. Its reciprocal is [`PAD_RAMP_RATE`], so the two branches meet
/// exactly (`amount * 10 * 0.1 == amount`).
pub const PAD_RAMP_FLOOR: f32 = 0.1;

/// How fast the boost decays once the pad is behind the ship, per second: the `10.0` of
/// `Ship_ApplySpeedupPad`, `1.0 / PAD_RAMP_FLOOR`. Applied to the *remaining* time, so the
/// force falls linearly from `amount * 10 * time` to `amount` at [`PAD_RAMP_FLOOR`] and
/// holds.
pub const PAD_RAMP_RATE: f32 = 10.0;

/// How far the pitch axis must be deflected before the speed-pad tilt counts as held.
///
/// **This project's own number**, the one judgement call in [`speedup_pad`]: the original
/// tests a bit, and [`ShipControls`] is normalised by contract. Half deflection: a D-pad
/// or key gives exactly `-1.0`, and a stick must be pushed decisively. **Chosen, not
/// measured.** Compared at `<=` against a **negative** `steer_y` (see [`speedup_pad`]).
pub const SPEEDPAD_JUMP_THRESHOLD: f32 = 0.5;

/// The speed-pad boost, into the **world** force accumulator.
///
/// `Ship_ApplySpeedupPad` (`0x08848f9c`), step 15 of `Ship_UpdateCraft`. Confidence **85**
/// on the force law, **90** on the tunables being stored unscaled (`Xml_ReadGlobalSettings`).
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
/// The decrement comes **before** the force is read off the timer, the original's order:
/// it lowers the peak by one `dt` of ramp (about 6 % at 60 Hz), and
/// `docs/physics/cornering-ground-truth.md` measured five pad crossings closely enough to
/// prefer it.
/// # `<Special speedpad_jump>`
///
/// `if (controls->0x24 & 1) dir += craft+0x160 * g_speedpad_jump`:
///
/// - **`craft+0x160` is the hull's up axis**: live in PPSSPP its dot product against
///   the body's up row (`body+0x010`) is `+1.000000` and `0.000000` against the others.
/// - **`controls->0x24 & 1` is D-pad Up**: a one-hot sweep of all twelve buttons against
///   `*(craft+0x78)+0x24` sets bit 0 only for Up.
/// - **The magnitude is `<Special speedpad_jump>`**, read live at `0x08b36bec`, `0.1` on
///   both shipped discs, taken from the player's disc through [`Handling::speedpad_jump`].
///
/// **It is not a jump.** The sum is not renormalised, so `0.1` against a unit direction
/// tilts the boost by `atan(0.1)` = **5.71 degrees** and adds 0.5 % force, for at most the
/// class's `time` (`0.24`-`0.27 s`), which the hover spring mostly absorbs.
/// ## The judgement call: an axis standing in for a digital button
///
/// The original gates on a bit; [`ShipControls`] is normalised by contract, so there is no
/// bit and a second control input for one branch would be worse than a threshold.
/// [`SPEEDPAD_JUMP_THRESHOLD`] is that threshold. **The sign is the surprising half**:
/// `oag_gameplay::ship_controls` maps D-pad Up to `steer_y = -1` (up pitches the nose
/// *down* here; the inversion is at the input boundary), so the gate is
/// `steer_y <= -threshold`. Backwards would tilt the boost upward whenever the player
/// pitches down, behaviour that looks deliberate (see `crate::controls`' module docs).
///
/// # Deliberately not here: the `craft+0x2cc` scale
///
/// `if (craft+0x2cc < 1.0) f *= craft+0x2cc`, a one-second fade-in. `craft+0x2cc` is
/// written only in `Ship_UpdateEngine`'s prologue (`flags & 0x200 ? 0.0 : craft+0x2cc +
/// dt`), so it is **seconds since flag `0x200` was last set**, one of the eleven
/// undecoded bits of `craft+0x1c0`; it is not the contact ratio an earlier reading assumed
/// (an unbounded accumulator cannot be a groundedness). Omitting it gives full strength
/// from the first tick, which the trace measures: `0x200` is evidently clear during
/// ordinary racing.
pub fn speedup_pad(
    state: &mut ShipState,
    controls: &ShipControls,
    handling: &Handling,
    up: Vec3,
    pad_hit: Option<Vec3>,
    dt: f32,
) -> Vec3 {
    // Assignment, not a maximum: standing on a pad holds the timer full and the countdown
    // starts when the ship leaves.
    if let Some(direction) = pad_hit {
        state.pad_timer = handling.speedup_pads.time;
        state.pad_direction = direction;
    }

    if state.pad_timer <= 0.0 {
        return Vec3::ZERO;
    }

    // The original lets this go slightly negative and gates on `> 0.0` next frame;
    // clamping is the same behaviour with a tidier state hash.
    state.pad_timer = (state.pad_timer - dt).max(0.0);

    let amount = handling.speedup_pads.amount;
    let force = if state.pad_timer > PAD_RAMP_FLOOR {
        amount * PAD_RAMP_RATE * state.pad_timer
    } else {
        amount
    };

    // Read fresh each tick from *this* tick's input and hull, not baked into
    // `pad_direction` when the pad arms: the original recomputes `dir` inside the
    // `timer > 0` block, so a boost outliving the pad tilts the moment the player pitches
    // up and stops when they let go.
    let direction = if controls.steer_y <= -SPEEDPAD_JUMP_THRESHOLD {
        state.pad_direction + up * handling.speedpad_jump
    } else {
        state.pad_direction
    };

    direction * force
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod auto_speed_tests {
    use super::{ENGINE_OUTPUT_DOUBLE, ENGINE_OUTPUT_SCALE, engine};
    use crate::params::Handling;
    use crate::ship::ShipState;

    /// A ship with the throttle released: Zone accelerates with nothing held down.
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

        // Throttle at zero and yet there is thrust, as in Zone.
        let idle = engine(&state, &handling, 1.0, 0.0, Some(50.0), 1.0).thrust;
        state.thrust = 100.0;
        let full = engine(&state, &handling, 1.0, 0.0, Some(50.0), 1.0).thrust;
        assert_eq!(idle, full, "the throttle changed the auto-speed output");
        assert_eq!(idle, 50.0 * ENGINE_OUTPUT_SCALE * ENGINE_OUTPUT_DOUBLE);
    }

    #[test]
    fn the_auto_speed_branch_is_not_capped_by_accelcap() {
        // The ordinary branch clamps against `0.5 * speed + accelcap` (zero for
        // `Handling::ZERO`); the four-corner branch has no clamp.
        let state = grounded_ship();
        let force = engine(&state, &Handling::ZERO, 1.0, 0.0, Some(10_000.0), 1.0).thrust;
        assert!(force > 0.0, "the cap bound a branch that has no cap");
    }

    #[test]
    fn the_auto_speed_branch_carries_the_launch_multiplier() {
        // `craft+0x294` is in the shared tail (live on a Zone engine, see `engine`).
        let mut state = grounded_ship();
        state.launch.multiplier = 1.4;
        let boosted = engine(&state, &Handling::ZERO, 1.0, 0.0, Some(34.0), 1.0).thrust;
        state.launch.multiplier = 1.0;
        let resting = engine(&state, &Handling::ZERO, 1.0, 0.0, Some(34.0), 1.0).thrust;
        assert_eq!(boosted, resting * 1.4, "the launch multiplier was dropped");
    }

    #[test]
    fn a_zone_craft_on_the_grid_gets_nothing_until_it_is_released() {
        // `(flags & 1) && !(flags & 2)`: bit 1 is the grid state.
        let mut state = grounded_ship();
        state.on_grid = true;
        let held = engine(&state, &Handling::ZERO, 1.0, 0.0, Some(50.0), 1.0).thrust;
        assert_eq!(held, 0.0, "auto-speed ran under the grid state");
        state.on_grid = false;
        let released = engine(&state, &Handling::ZERO, 1.0, 0.0, Some(50.0), 1.0).thrust;
        assert!(released > 0.0);
    }

    #[test]
    fn an_airborne_zone_craft_gets_nothing() {
        // The gate: the original writes `0.0` when the contact bit is clear.
        let state = grounded_ship();
        let force = engine(&state, &Handling::ZERO, 0.0, 0.0, Some(50.0), 1.0).thrust;
        assert_eq!(force, 0.0);
    }

    #[test]
    fn without_a_target_the_ordinary_path_is_untouched() {
        let mut state = grounded_ship();
        state.thrust = 100.0;
        let handling = Handling::ZERO;
        assert_eq!(
            engine(&state, &handling, 1.0, 40.0, None, 1.0),
            engine(&state, &handling, 1.0, 40.0, None, 1.0)
        );
        // `Handling::ZERO` has no engine amount: the ordinary path is zero, auto-speed is
        // not.
        assert_eq!(engine(&state, &handling, 1.0, 40.0, None, 1.0).thrust, 0.0);
        assert!(engine(&state, &handling, 1.0, 40.0, Some(50.0), 1.0).thrust > 0.0);
    }
}

#[cfg(test)]
mod speedup_pad_tests;
