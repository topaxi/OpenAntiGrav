//! Airbrakes, lateral grip and the sideshift impulse.
//!
//! The force law is from `docs/physics/README.md`; which accumulator each term writes and
//! in which frame is from `docs/ghidra/functions/psp-pulse-usa/engine.md`. Two things to
//! keep honest:
//!
//! - **`amount` is a lateral force gain, not a drag.** The `drag` parameter **does not
//!   slow the ship**: its term points along `+forward` with every factor non-negative, so
//!   moving forwards it is a very small *acceleration* (implemented in [`evaluate`],
//!   `Ship_UpdateAirbrakes` `0x0884c9a4` read end to end). Speed loss under braking is
//!   indirect: lateral grip, the separate brake both airbrakes engage
//!   ([`crate::engine::brakes`]) and the always-on drag in [`crate::passive`].
//! - **Airbrakes produce no direct roll torque**; nothing here writes an angular Z.
//!   Confidence 85, from checking every write to the original's angular accumulators.
//!   Visible roll comes from the surface-alignment torque and bank coupling in
//!   [`crate::hover`], or graphics-only state. Adding a roll torque because the ship looks
//!   wrong would be the wrong move; a test pins the absence. The wider "nothing writes an
//!   angular Z" is too strong (`engine.md`): no *control input* does, but angular damping
//!   and the alignment torque do passively. Confidence 80 for the narrowed claim, lower
//!   because `Ship_UpdateMagLock` was not scanned.
//!
//! The airbrake states are ramped in [`crate::controls`], on the original's `0..=100` scale.

use oag_core::math::Vec3;

use crate::controls::CONTROL_RANGE;
use crate::params::Handling;
use crate::ship::{ShipControls, ShipState, Sideshift};

/// What the airbrake path contributed this frame: a world-space force and a body-local
/// angular term, matching `Ship_UpdateAirbrakes`' `craft+0x330` and `craft+0x340`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AirbrakeForces {
    /// World-space force: the forward slide term and the lateral term.
    pub world_force: Vec3,
    /// Body-local angular. **Its X and Z components are always zero**; only yaw is written.
    pub local_angular: Vec3,
}

/// The lateral grip coefficient `k`.
///
/// ```text
/// k = max(L, R) * (0.01 - slidegrip) - 1.0
/// ```
///
/// The literal form from `docs/physics/README.md`, right all along: it reads as absurd
/// until one learns [`Airbrake::slidegrip`](crate::params::Airbrake::slidegrip) is
/// **pre-scaled by `1e-4` at load** (XML 0..100 is 0..0.01 in memory) while the airbrake
/// states run 0..100. Then `(0.01 - slidegrip)` runs from `0.01` to `0`, so `k` is `0` at
/// full airbrake with `slidegrip = 0` (grip vanishes, the ship drifts) and stays `-1` for
/// `slidegrip = 100`, the behaviour the page's prose described. (This crate once
/// implemented the prose reading; a formula that cannot be right on the units in front of
/// you is evidence about the units.) Confidence 90, arithmetic.
#[must_use]
pub fn lateral_grip_coefficient(handling: &Handling, left: f32, right: f32) -> f32 {
    let applied = left.max(right);

    applied * (0.01 - handling.airbrake.slidegrip) - 1.0
}

/// The airbrake terms: the forward slide, the lateral force and the yaw.
///
/// ```text
/// slide  = |L - R| * drag * |steerX| * 0.01
/// world += forward * speed * slide * 0.001
/// world += right   * speed * amount * (R - L)
/// angularLocal.y += speed * turn * (R - L) * 0.001
/// ```
///
/// `speed` is the cached `|dot(velocity, forward)|` (`craft+0x2ec`, the `vabs.s`-ed dot
/// product per `engine.md`), which all three terms load. Lateral grip is **not** here: it
/// is a separate original function running after hover and reading a different
/// groundedness ([`lateral_grip`]).
/// # `Ship_UpdateAirbrakes` read end to end
///
/// `0x0884c9a4` in the PSP `BOOT.BIN`, force block `0x0884ccb4`-`0x0884cf94` (listing in
/// `engine.md` and `docs/physics/force-balance-ground-truth.md`). It loads the cached
/// speed (`0x0884ccb4`, skipping the block when `<= 0`), the ramped `L`/`R`
/// (`craft+0x2c4`/`+0x2c8`), the **input snapshot's** `steerX` (`*(craft+0x78)`), and
/// `Airbrake.drag` (class `+0xe8`), multiplies by `0.01` (`0x3c23d70a`), scales `forward`
/// (`craft+0x180`) by speed, slide and `0.001` (`0x3a83126f`), and `vadd.t`s into
/// `craft+0x330` at `0x0884cf88`.
///
/// - **The scale is `1e-5`**: two literals, `0.01` and `0.001`, on the same vector (an
///   earlier capstone pass recorded only the first). The association order is the
///   binary's, left to right at both levels, which the pinning test asserts.
/// - **The sign is settled: it accelerates.** `sp+0x10` is written fresh at `0x0884cdb8`,
///   every factor is non-negative, and the `vadd.t` adds. Confidence **90**.
/// - **The gate is on the absolute cached speed**, not a signed forward speed
///   (`force-balance-ground-truth.md` had "gated on `forwardSpeed > 0`"). A **reversing**
///   ship is not excluded and gets the same `+forward` push, a deceleration. Pinned by a
///   test.
///
/// Confidence **88** on the formula (single binary corroborated by the PS2 shape); the
/// magnitude is weakly corroborated by `talons-junction-airbrake-asymmetric` (within a
/// factor of 2, conf ~55, wall-contaminated). **The implementation follows the read values
/// and is not tuned to that capture.**
/// # Why `(R - L)` is negated here
///
/// Two conversions separate the original's frame from this crate's, both settled in
/// `engine.md` ("The basis is positively oriented, and row 0 points left"):
///
/// - **The lateral axis.** `craft+0x170` is `body+0x00`, row 0, the ship's **left**
///   (`cross(row0, row1) = +row2` where this crate has `cross(right, up) = -forward`), so
///   the original's `right * ...` is `-Body::right() * ...` here.
/// - **The yaw sign.** The original's angular accumulators carry the negated physical
///   rotation (`w_game = -w_physics`): a positive `angularLocal.y` there turns the nose
///   **right**, here **left** ([`crate::engine::steering`] negates for the same reason).
///
/// Both terms consume `(R - L)` once and each conversion is one flip, so negating the
/// factor once is the whole conversion. The forward `drag` term takes `|L - R|` and is
/// untouched.
///
/// **A real bug once**: the steering sign was fixed in `5ad69f3` and the airbrake path,
/// sharing the convention, was missed, so braking one side turned the nose away from it.
/// It survived because the only committed scenario for the term
/// (`verification/scenarios/airbrake-asymmetric.inputs`) holds brake and steering on the
/// *same* side and steering dominates, so only the *rate* was wrong. An airbrake held with
/// `steer == 0` shows it, as
/// `braking_the_left_airbrake_alone_turns_the_ship_toward_its_own_left` in
/// [`crate::forces`] now does every run.
#[must_use]
pub fn evaluate(
    state: &ShipState,
    input: &ShipControls,
    handling: &Handling,
    forward_speed: f32,
) -> AirbrakeForces {
    let body = &state.body;
    let left = state.airbrake_left;
    let right_brake = state.airbrake_right;
    // `(R - L)` in the original, negated once for this crate's frame (see "Why `(R - L)` is
    // negated here"): one negation carries both frame conversions.
    let imbalance = left - right_brake;

    let forward = body.forward();
    let right = body.right();
    let speed = forward_speed.abs();

    // The original's `c.le.s`/`bc1t` at `0x0884ccb8` skips all three terms when the cached
    // speed is not positive. Gated on the **absolute** speed (what `craft+0x2ec` holds), not
    // the argument: a signed gate would exclude a reversing ship, which the original does
    // not. Numerically a no-op (every term has `speed` as a factor); written out so a reader
    // comparing with the disassembly does not look for a missing branch.
    if speed <= 0.0 {
        return AirbrakeForces::default();
    }

    let mut world = Vec3::ZERO;

    // slide = |L - R| * drag * |steerX| * 0.01
    //
    // `steerX` is the **raw input** (`craft+0x78 + 0x0`) while `L` and `R` are the *ramped*
    // states (`craft+0x2c4`/`+0x2c8`): the original's asymmetry, which a tidying pass would
    // unify by accident, so a test pins it.
    //
    // **On the original's `+/-100` scale**, not `ShipControls`' `-1..=1`:
    // `Ship_UpdateSteering` loads the same `+0x0` (`0x088487a0`) and compares it with the
    // ramped `craft+0x2c0` (`0x088487b8`), which runs to `+/-100`; `engine.md` measured the
    // sibling `+0x10` axis live at `+/-100`. Until 2026-09-30 this was 100x weak: a craft
    // holding one airbrake into a turn lost about `0.19 * speed` units/s^2 of forward push
    // (`talons-junction-clean-lap.csv`, `docs/physics/cornering-ground-truth.md`, whose fit
    // of this term at `1.03`-`1.07` assumed this scale).
    let steer_x = input.steer_x.abs() * CONTROL_RANGE;
    let slide = (left - right_brake).abs() * handling.airbrake.drag * steer_x * 0.01;

    // Along `+forward`, and it accelerates (see the header).
    world += forward * speed * slide * 0.001;

    // `amount` is a lateral force gain: braking harder on one side pushes the ship sideways,
    // not slower. The original writes it along `craft+0x170` (the ship's **left**) and
    // `imbalance` carries the flip, so braking left pushes the body right: the craft
    // rotates into the corner while the body runs wide, which is what `slidegrip` cutting
    // grip at the same time is for.
    world += right * speed * handling.airbrake.amount * imbalance;

    // Yaw only (module docs). Positive is nose-left here, so braking the left side turns
    // the nose toward the braked side.
    let local_angular = Vec3::new(0.0, speed * handling.airbrake.turn * imbalance * 0.001, 0.0);

    AirbrakeForces {
        world_force: world,
        local_angular,
    }
}

/// Lateral grip, as a **body-local** force along the right axis.
///
/// ```text
/// k = max(L, R) * (0.01 - slidegrip) - 1.0
/// local.x += grip_ground * dot(vel, right) * k * grounded
/// local.x += grip_air    * dot(vel, right) * k * (1 - grounded)
/// ```
/// Two things distinguish this from the rest of the airbrake path (`engine.md`):
///
/// - It writes the **body-local** force accumulator directly, which is how we know that
///   accumulator's `.x` is the right axis.
/// - It runs **after** hover, so `grounded` is **this** frame's contact fraction, unlike
///   the engine, brakes, drag, gravity and pitch. The caller also gates it on the weapon
///   slowdown timer having expired.
///
/// While [`ShipState::roll_payout_timer`] runs, `grip_ground` and `grip_air` are scaled by
/// [`ROLL_GRIP_MULTIPLIER`], one of three consumers of `craft+0x1c0 & 0x400` (with the
/// hover rebound override and the turbo add). Confidence 88; `input-bindings.md`.
#[must_use]
pub fn lateral_grip(state: &ShipState, handling: &Handling, grounded: f32) -> Vec3 {
    let right = state.body.right();
    let lateral_velocity = state.body.linear_velocity.dot(right);
    let k = lateral_grip_coefficient(handling, state.airbrake_left, state.airbrake_right);
    let roll_boost = if state.roll_payout_timer > 0.0 {
        ROLL_GRIP_MULTIPLIER
    } else {
        1.0
    };

    let mut lateral = handling.antigrav.grip_ground * lateral_velocity * k * grounded;
    lateral += handling.antigrav.grip_air * lateral_velocity * k * (1.0 - grounded);

    Vec3::new(lateral * roll_boost, 0.0, 0.0)
}

/// The lateral-grip multiplier while the barrel roll's landing payout runs.
///
/// `Ship_ApplyLateralGrip` (`0x08848b78`) multiplies `stats+0x10` (`grip_ground`) and
/// `stats+0x14` (`grip_air`) by this literal on the `craft+0x1c0 & 0x400` branch; a full
/// instruction scan of `.text` finds only three consumers of that bit. Confidence 88; see
/// `input-bindings.md`.
pub const ROLL_GRIP_MULTIPLIER: f32 = 1.5;

/// How long one sideshift pushes for, in seconds: the literal `0.2` (`0x3e4ccccd`)
/// `Ship_UpdateSideshiftInput_q` (`0x08846a54`) writes into whichever per-side timer fired.
/// The craft flag, and so the force, lasts as long as the timer. Confidence **80**
/// (`engine.md`).
pub const SIDESHIFT_DURATION: f32 = 0.2;

/// How long after a sideshift before another can be triggered, in seconds.
///
/// The literal `1.0` `Ship_UpdateSideshiftInput_q` writes into `entity+0x8ac` every tick
/// either timer runs. It gates the whole trigger block, so it is a second from the *end*
/// of a shift (`~1.2 s` total with [`SIDESHIFT_DURATION`]). Confidence **95**, confirmed
/// 2026-08-27 by a capture of `sideshift-double-tap.inputs` (`input-bindings.md`).
pub const SIDESHIFT_LOCKOUT: f32 = 1.0;

/// How long a first airbrake press stays a candidate for a double tap, in seconds: the
/// literal `0.25` (`0x3e800000`) the veteran branch writes into `entity+0x89c`/`+0x8a0`.
/// Confidence **85**.
pub const SIDESHIFT_TAP_WINDOW: f32 = 0.25;

/// How far the steering axis must move to arm and then fire a novice flick.
///
/// The original tests its `+/-100` axis against `+/-10`; [`ShipControls::steer_x`] is the
/// same axis on `-1..=1`, so `0.1`. Inside it the flick arms, outside it fires.
/// Confidence **85**.
pub const SIDESHIFT_FLICK_THRESHOLD: f32 = 0.1;

/// The sideshift, as a **world-space force** while its timer runs.
///
/// Not a one-shot velocity change (the shape this function once had, answering
/// `docs/physics/README.md`'s open "impulse or velocity" question). `Ship_UpdateAirbrakes`'
/// tail (`0x0884c9a4`) is
///
/// ```text
/// if (craft+0x1c0 & 1) {                          // last frame's contact flag
///     if (craft+0x1c0 & 0x800)                    // the LEFT flick's timer is running
///         Body_AddForceWorld(body, craft+0x170 *  handling.sideshift);
///     if (craft+0x1c0 & 0x1000)                   // the RIGHT flick's timer
///         Body_AddForceWorld(body, craft+0x170 * -handling.sideshift);
/// }
/// ```
///
/// an ordinary force added every frame for [`SIDESHIFT_DURATION`], divided by mass by the
/// integrator, and **switched off in the air**. Confidence **85**.
///
/// **Direction.** `craft+0x170` is row 0, the ship's *left* (`engine.md`), so `0x800`
/// pushes left and `0x1000` right. In `Ship_UpdateSideshiftInput_q` the steering axis
/// crossing `+10` arms `0x1000` and `-10` arms `0x800`, on the axis the
/// `steer-left`/`steer-right` captures measure as positive-right. **A craft shifts toward
/// the side it was flicked**, as [`Sideshift::Right`] to `+`[`crate::ship::Body::right`]
/// already meant: a confirmation of what had been a guess.
///
/// Both gestures are in [`advance_sideshift`], as per-craft state like the original; the
/// input layer supplies buttons, not decisions (`input-bindings.md`).
#[must_use]
pub fn sideshift_force(state: &ShipState, handling: &Handling, grounded: f32) -> Vec3 {
    if grounded <= 0.0 {
        return Vec3::ZERO;
    }

    let right = state.body.right();
    let mut force = Vec3::ZERO;
    // Left first, matching the field's order. Both can run at once and the original lets
    // the forces cancel rather than picking a winner.
    if state.sideshift_timers[0] > 0.0 {
        force -= right * handling.airbrake.sideshift;
    }
    if state.sideshift_timers[1] > 0.0 {
        force += right * handling.airbrake.sideshift;
    }
    force
}

/// Runs both sideshift gestures and the timers they arm.
///
/// Ported from `Ship_UpdateSideshiftInput_q` (`0x08846a54`) in its order: count every timer
/// down, run whichever gesture the pilot is making, refresh the lockout from what is
/// running. Counting down first makes a shift last [`SIDESHIFT_DURATION`], not one tick
/// more.
///
/// # The two gestures
///
/// The original has **two control schemes** (`input-bindings.md`):
///
/// - **Novice** holds one button (`OPT_CTRL_SS`, `L` by default) and *flicks* the stick.
///   The flick arms when the axis is inside [`SIDESHIFT_FLICK_THRESHOLD`] and fires when
///   it crosses back out. **The craft shifts toward the side it was flicked.**
/// - **Veteran** has no sideshift button (`L` and `R` are the airbrakes) and *double-taps*
///   an airbrake within [`SIDESHIFT_TAP_WINDOW`].
///
/// Both machines run unconditionally, unlike the original's scheme flag, which is safe
/// because the input layer fills only the live scheme's fields
/// (`oag_gameplay::controls::ship_controls`); the scheme is an options setting, not
/// physics.
///
/// # Not ported
///
/// The original gates the block on `craft+0x1c0 & 2`, a flag bit nothing has identified
/// (`engine.md` names eleven bits and only bit 0 is established), so our sideshift may be
/// available where the original withholds it.
pub fn advance_sideshift(state: &mut ShipState, input: &ShipControls, dt: f32) {
    // The original's countdown is gated, not clamped (`if (t > 0.0f) t -= dt;`), so it lands
    // slightly negative on expiry and freezes. Deliberately not matched: every reader gates
    // on `> 0.0`/`<= 0.0`, so the residue is unobservable, and these three feed
    // `ShipState::hash_state`'s committed golden hashes (`input-bindings.md`, countdown
    // section).
    for timer in &mut state.sideshift_timers {
        *timer = (*timer - dt).max(0.0);
    }
    for window in &mut state.shift_tap_windows {
        *window = (*window - dt).max(0.0);
    }
    state.shift_lockout = (state.shift_lockout - dt).max(0.0);

    // `ShipControls::sideshift` is the direct request and bypasses the lockout; the
    // gestures do not. Both sides are tracked separately because the original arms two
    // independent timers and lets the forces cancel.
    let mut fire_left = input.sideshift == Sideshift::Left;
    let mut fire_right = input.sideshift == Sideshift::Right;

    if state.shift_lockout <= 0.0 {
        if input.shift_modifier {
            if state.shift_armed {
                if input.steer_x > SIDESHIFT_FLICK_THRESHOLD {
                    fire_right = true;
                    state.shift_armed = false;
                } else if input.steer_x < -SIDESHIFT_FLICK_THRESHOLD {
                    fire_left = true;
                    state.shift_armed = false;
                }
            } else if input.steer_x.abs() < SIDESHIFT_FLICK_THRESHOLD {
                state.shift_armed = true;
            }
        }

        // A tap is ignored while that side is already shifting: the original's own
        // `timer <= 0` guard, not a second lockout.
        if input.shift_tap_left && state.sideshift_timers[0] <= 0.0 {
            if state.shift_tap_windows[0] > 0.0 {
                fire_left = true;
            } else {
                state.shift_tap_windows[0] = SIDESHIFT_TAP_WINDOW;
            }
        }
        if input.shift_tap_right && state.sideshift_timers[1] <= 0.0 {
            if state.shift_tap_windows[1] > 0.0 {
                fire_right = true;
            } else {
                state.shift_tap_windows[1] = SIDESHIFT_TAP_WINDOW;
            }
        }
    }

    if fire_left {
        state.sideshift_timers[0] = SIDESHIFT_DURATION;
    }
    if fire_right {
        state.sideshift_timers[1] = SIDESHIFT_DURATION;
    }

    if state.sideshift_timers[0] > 0.0 || state.sideshift_timers[1] > 0.0 {
        state.shift_lockout = SIDESHIFT_LOCKOUT;
    }
}

#[cfg(test)]
mod tests;
