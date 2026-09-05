//! Airbrakes, lateral grip and the sideshift impulse.
//!
//! The force law is from `docs/physics/README.md`; which accumulator each term
//! writes, and which frame it is expressed in, is from
//! `docs/ghidra/functions/psp-pulse-usa/engine.md`. The two things this module exists to
//! keep honest:
//!
//! - **`amount` is a lateral force gain, not a drag.** An earlier revision of this
//!   bullet continued "and there is no dedicated airbrake drag term at all", and
//!   that half was **false** - the code a hundred lines below has always applied
//!   one, and `crates/physics/src/passive.rs` and
//!   `docs/physics/force-balance-ground-truth.md` both went on to describe the
//!   term as *unimplemented*. It is implemented, here, and
//!   `Ship_UpdateAirbrakes` (`0x0884c9a4`) has now been read end to end to
//!   confirm the form factor for factor; see [`evaluate`].
//!
//!   What the sentence should have said is that the `drag` parameter **does not
//!   slow the ship down**: the term points along `+forward` with every factor
//!   non-negative, so for a ship moving forwards it is a (very small)
//!   *acceleration*. Speed loss under braking is still indirect: lateral grip
//!   converting sideways motion into body-frame force, the separate brake that
//!   both airbrakes together engage (see [`crate::engine::brakes`]), and the
//!   always-on drag in [`crate::passive`].
//! - **Airbrakes produce no direct roll torque**, and nothing in this module writes an
//!   angular Z at all. Confidence 85, established by checking every write to the
//!   original's angular accumulators, and it is a negative rather than an omission:
//!   visible roll must come from the surface-alignment torque and the bank-coupling term
//!   in [`crate::hover`], or from graphics-only state that never touches the simulation.
//!   Adding a roll torque here because the ship looks wrong would be exactly the wrong
//!   move, so a test pins the absence.
//!
//!   The wider claim that *nothing* writes an angular Z is too strong, and
//!   `docs/ghidra/functions/psp-pulse-usa/engine.md` narrowed it: **no control input** writes
//!   one, so roll is never commanded, but the angular damping and the surface-alignment
//!   torque both do, passively. Confidence 80 on the narrowed version, lower because
//!   `Ship_UpdateMagLock` was not scanned.
//!
//! The airbrake states themselves are ramped in [`crate::controls`], on the
//! original's `0..=100` scale.

use oag_core::math::Vec3;

use crate::params::Handling;
use crate::ship::{ShipControls, ShipState, Sideshift};

/// What the airbrake path contributed this frame.
///
/// A world-space force and a body-local angular term, matching the accumulators
/// `Ship_UpdateAirbrakes` writes: `craft+0x330` and `craft+0x340`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AirbrakeForces {
    /// World-space force: the forward slide term and the lateral term.
    pub world_force: Vec3,
    /// Body-local angular. **Its X and Z components are always zero**; only yaw is
    /// written.
    pub local_angular: Vec3,
}

/// The lateral grip coefficient `k`.
///
/// ```text
/// k = max(L, R) * (0.01 - slidegrip) - 1.0
/// ```
///
/// **This is the literal form from `docs/physics/README.md`, and it was right all
/// along.** It reads as absurd until one fact is added:
/// [`Airbrake::slidegrip`](crate::params::Airbrake::slidegrip) is **pre-scaled by
/// `1e-4` at load**, so an XML value on 0..100 is 0..0.01 in memory, and the airbrake
/// states run 0..100. Then `(0.01 - slidegrip)` runs from `0.01` at `slidegrip = 0`
/// down to exactly `0` at `slidegrip = 100`, and with the `max(L, R)` factor reaching
/// 100 the coefficient is exactly `0` at full airbrake with `slidegrip = 0` (grip
/// vanishes, the ship drifts freely) and stays at `-1` for `slidegrip = 100` (grip
/// unchanged). Which is precisely the behaviour that page's prose described.
///
/// This crate previously implemented the prose reading,
/// `max(L, R) * (1.0 - 0.01 * slidegrip) - 1.0`, on the grounds that the pseudocode
/// looked garbled. It was not garbled; the load-time factor was simply not known yet.
/// **A genuine ambiguity resolved by finding the missing scale factor, not a reading
/// error**, and worth recording as such: a formula that cannot be right on the units
/// in front of you is evidence about the units, not only about the formula.
/// Confidence 90, arithmetic rather than interpretation.
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
/// `speed` is the craft's cached speed, `|dot(velocity, forward)|`, rather than
/// `|velocity|`. That used to be a pick; it is now read: every one of the three
/// terms loads `craft+0x2ec`, which
/// `docs/ghidra/functions/psp-pulse-usa/engine.md` establishes as the `vabs.s`-ed
/// dot product.
///
/// Lateral grip is **not** here: it is a separate function in the original, running
/// after hover and reading a different groundedness. See [`lateral_grip`].
///
/// # `Ship_UpdateAirbrakes` read end to end
///
/// `0x0884c9a4` in the PSP `BOOT.BIN`, force block `0x0884ccb4`-`0x0884cf94`,
/// with the Allegrex module. Every factor below is an instruction, not an
/// inference:
///
/// ```text
/// 0884ccb4  lwc1  f12,0x2ec(s0)   ; the cached speed
/// 0884ccb8  c.le.s f12,f14        ; f14 == 0.0, set once at 0884c9cc
/// 0884ccc0  bc1t  0x0884cf98      ; speed <= 0 skips the whole force block
/// 0884ccc8  lwc1  f12,0x2c4(s0)   ; ramped airbrake L
/// 0884cccc  lwc1  f13,0x2c8(s0)   ; ramped airbrake R
/// 0884ccd0  sub.s f12,f12,f13
/// 0884ccd4  lw    a0,0x78(s0)     ; the *input snapshot*, not the ramped state
/// 0884ccd8  lwc1  f15,0x0(a0)     ; steerX
/// 0884ccdc  abs.s f15,f15
/// 0884cce0  abs.s f12,f12
/// 0884cce8  lwc1  f16,0x54(a0)    ; Airbrake.drag, class +0xe8
/// 0884ccec  mul.s f12,f12,f16
/// 0884ccf0  mul.s f12,f12,f15
/// 0884ccf4  lui   a0,0x3c23       ; 0x3c23d70a == 0.01
/// 0884cd00  mul.s f12,f12,f13     ; slide
/// 0884cd18  lv.q  C500,0x0(a0)    ; a0 == craft+0x180 == forward
/// 0884cd1c  vscl.q C300,C500,S400 ; forward * speed
/// 0884cd54  vscl.q C300,C500,S400 ; * slide
/// 0884cd78  lui   a1,0x3a83       ; 0x3a83126f == 0.001
/// 0884cd98  vscl.q C300,C500,S400 ; * 0.001
/// 0884cf88  vadd.t C300,C300,C600 ; craft+0x330, the world force accumulator
/// ```
///
/// Three things that follow, all of which correct something previously written
/// down:
///
/// - **The scale is `1e-5`.** Two literals, `0.01` and `0.001`, applied to the
///   same vector. A capstone pass recorded only the first. This function's
///   association order is the binary's, left to right at both levels, which is
///   what the pinning test asserts against.
/// - **The sign is settled: it accelerates.** This used to carry a comment
///   reading "sign unresolved - a guess awaiting M3". `sp+0x10` is written
///   fresh at `0x0884cdb8` rather than accumulated into, every factor is
///   non-negative, and the `vadd.t` at `0x0884cf88` *adds* the result to
///   `craft+0x330`. So a ship moving forwards is pushed forwards. Confidence
///   **90**: read at instruction level including the accumulate, on one binary.
/// - **The gate is on the absolute cached speed, not on a signed forward
///   speed.** `docs/physics/force-balance-ground-truth.md` recorded it as
///   "gated on `forwardSpeed > 0`"; the instruction loads `craft+0x2ec`, which
///   is `|dot(v, forward)|`. The consequence is real rather than pedantic: a
///   **reversing** ship is not excluded, and gets the same `+forward` push,
///   which for it is a deceleration. Pinned by a test.
///
/// Confidence **88** on the formula, the ceiling for a single binary
/// corroborated by the PS2 shape; the magnitude is only weakly corroborated by
/// the `talons-junction-airbrake-asymmetric` capture (consistent within a
/// factor of 2, conf ~55, wall-contaminated throughout). **The implementation
/// follows the read values and is not tuned to that capture.**
///
/// # Why `(R - L)` is negated here
///
/// The transcription above is in the original's frame, and **two conversions
/// separate that frame from this crate's**, both settled in
/// `docs/ghidra/functions/psp-pulse-usa/engine.md` ("The basis is positively
/// oriented, and row 0 points left"):
///
/// - **The lateral axis.** `craft+0x170` is `body+0x00`, row 0, and row 0 is the
///   ship's **left**: the original's own recordings rotate `forward` toward
///   `+row0` on a left turn, and `cross(row0, row1) = +row2` where this crate's
///   `cross(right, up) = -forward`. So the original's `right * ...` is
///   `-Body::right() * ...` here.
/// - **The yaw sign.** The original's angular accumulators carry the negated
///   physical rotation (`e' = e x w`, the `w_game = -w_physics` convention), so
///   a positive `angularLocal.y` there turns the nose **right**, while a
///   positive `local_angular.y` here turns it **left**.
///   [`crate::engine::steering`] negates the literal law for exactly this reason
///   and documents it at length.
///
/// Both terms below consume `(R - L)` exactly once, and both conversions are a
/// single sign flip, so negating the factor once - rather than negating two
/// expressions, or worse, one - is the whole conversion. The forward `drag` term
/// takes `|L - R|` and is untouched by it.
///
/// **This was a real bug, found after a user reported that the airbrakes felt
/// reversed.** The steering sign was fixed
/// in `5ad69f3` ("physics: refine yaw calculations"), in `engine::steering` and
/// nowhere else; the airbrake path shares the convention and was missed, so from
/// that commit until this one **braking one side turned the nose away from it**.
/// The reason it survived is worth recording: the only committed scenario that
/// exercises the term,
/// `verification/scenarios/airbrake-asymmetric.inputs`, holds the brake and the
/// steering on the *same* side, and the steering drive is much the larger of the
/// two, so the run still curved the right way and only the *rate* was wrong. It
/// takes an airbrake held with `steer == 0` to see it, which
/// [`crate::forces`]'s `braking_the_left_airbrake_alone_turns_the_ship_toward_its_own_left`
/// now does on every run.
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
    // `(R - L)` in the original, negated once here, for this crate's frame. See
    // "Why `(R - L)` is negated here" in this function's header: the original's
    // row 0 is the ship's **left** and its angular `.y` is the negated physical
    // one, and both terms below consume this factor exactly once, so a single
    // negation carries both conversions rather than two compensating signs.
    let imbalance = left - right_brake;

    let forward = body.forward();
    let right = body.right();
    let speed = forward_speed.abs();

    // The original's `c.le.s`/`bc1t` at `0x0884ccb8` skips all three terms below
    // when the cached speed is not positive. Reproduced on the **absolute**
    // speed, which is what `craft+0x2ec` holds, rather than on the argument:
    // gating the signed argument would exclude a reversing ship, which the
    // original does not do.
    //
    // Numerically this is a no-op - every term carries `speed` as a factor, so
    // they are all exactly zero here anyway - and it is written out so that a
    // reader comparing against the disassembly does not go looking for a branch
    // that is missing.
    if speed <= 0.0 {
        return AirbrakeForces::default();
    }

    let mut world = Vec3::ZERO;

    // slide = |L - R| * drag * |steerX| * 0.01
    //
    // `steerX` is the **raw input**, `craft+0x78 + 0x0`, while `L` and `R` are
    // the *ramped* states at `craft+0x2c4`/`+0x2c8`. The asymmetry is the
    // original's and is exactly the kind of thing a tidying pass unifies by
    // accident, so it is called out here and pinned by a test.
    let slide = (left - right_brake).abs() * handling.airbrake.drag * input.steer_x.abs() * 0.01;

    // Along `+forward`, and it accelerates. See this function's header for the
    // instructions that settle the sign.
    world += forward * speed * slide * 0.001;

    // `amount` is a lateral force gain. Braking harder on one side pushes the ship
    // sideways; it does not slow it down directly. The original writes it along
    // `craft+0x170`, which is row 0 and so the ship's **left**; `imbalance`
    // carries that sign flip, so braking left pushes the body to the right - the
    // craft rotates into the corner while the body runs wide, which is what
    // `slidegrip` cutting lateral grip at the same time is for.
    world += right * speed * handling.airbrake.amount * imbalance;

    // Yaw only. The X and Z components are never written - see the module docs.
    // Positive is nose-left in this crate's frame, so braking the left side turns
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
///
/// Two things distinguish this from the rest of the airbrake path, both from
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`:
///
/// - It writes the **body-local** force accumulator directly rather than the world
///   one, which is the only reason we know that accumulator's `.x` is the right axis.
/// - It runs **after** hover, so `grounded` here is **this** frame's contact
///   fraction, unlike the engine, the brakes, drag, gravity and pitch. The caller also
///   gates it on the leap timer having expired.
///
/// While [`ShipState::roll_payout_timer`] is running, both `grip_ground` and
/// `grip_air` are scaled by [`ROLL_GRIP_MULTIPLIER`] - one of the three
/// consumers of the original's `craft+0x1c0 & 0x400`, alongside the hover
/// spring's rebound override in `crate::hover` and the turbo add in
/// `crate::engine`. Confidence 88; see
/// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
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
/// `Ship_ApplyLateralGrip` (`0x08848b78`) multiplies both `stats+0x10`
/// (`grip_ground`) and `stats+0x14` (`grip_air`) by this literal on the
/// `craft+0x1c0 & 0x400` branch - a complete instruction-level scan of
/// `.text`'s only three consumers of that bit, confidence 88. See
/// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
pub const ROLL_GRIP_MULTIPLIER: f32 = 1.5;

/// How long one sideshift pushes for, in seconds.
///
/// The literal `0.2` (`0x3e4ccccd`) that `Ship_UpdateSideshiftInput_q`
/// (`0x08846a54`) writes into whichever of its two per-side timers fired. The
/// timers count down by `dt` and the craft flag - and therefore the force - lasts
/// exactly as long as the timer does. Confidence **80**; see
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`.
pub const SIDESHIFT_DURATION: f32 = 0.2;

/// How long after a sideshift before another can be triggered, in seconds.
///
/// The literal `1.0` that `Ship_UpdateSideshiftInput_q` writes into
/// `entity+0x8ac` on every tick either side's timer is running. That timer
/// counts down by `dt` and gates the whole trigger block, so it is a second
/// measured from the *end* of a shift, not from the trigger - `~1.2 s` total
/// with [`SIDESHIFT_DURATION`]. Confidence **95**, confirmed 2026-08-27 by a
/// real capture of `sideshift-double-tap.inputs`; see
/// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
pub const SIDESHIFT_LOCKOUT: f32 = 1.0;

/// How long a first airbrake press stays a candidate for a double tap, in
/// seconds.
///
/// The literal `0.25` (`0x3e800000`) the veteran branch writes into
/// `entity+0x89c`/`entity+0x8a0`. Confidence **85**.
pub const SIDESHIFT_TAP_WINDOW: f32 = 0.25;

/// How far the steering axis must move to arm and then fire a novice flick.
///
/// The original tests its `+/-100` axis against `+/-10`; this crate's
/// [`ShipControls::steer_x`] is the same axis on `-1..=1`, so the threshold is
/// `0.1`. Inside it the flick arms, outside it the flick fires. Confidence
/// **85**.
pub const SIDESHIFT_FLICK_THRESHOLD: f32 = 0.1;

/// The sideshift, as a **world-space force** while its timer runs.
///
/// # It was a one-shot velocity change, and that was the wrong shape
///
/// `docs/physics/README.md` recorded a sideshift as "a one-shot lateral impulse
/// applied straight to the body", with a flagged open question - impulse or
/// velocity, which decides whether mass divides it - and this function used to
/// answer it by applying a velocity change once, on the frame the input arrived.
///
/// It is neither. `Ship_UpdateAirbrakes`' tail (`0x0884c9a4`, the block after the
/// airbrake force block) is
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
/// so it is an ordinary force, added every frame for the
/// [`SIDESHIFT_DURATION`] its timer runs, divided by mass by the integrator like
/// any other, and **switched off in the air**. Confidence **85**.
///
/// # The direction, which was an unevidenced coin flip
///
/// `craft+0x170` is the basis's **row 0**, which is the ship's *left*
/// (`docs/ghidra/functions/psp-pulse-usa/engine.md`, measured - the same convention
/// that inverted the airbrakes). So the `0x800` branch pushes left and the
/// `0x1000` branch pushes right, and which flick sets which is read in
/// `Ship_UpdateSideshiftInput_q`: the steering axis crossing `+10` arms
/// `0x1000` and crossing `-10` arms `0x800`, on the same `+/-100` axis whose sign
/// the `steer-left`/`steer-right` captures measure as **positive-right**.
///
/// **A craft therefore shifts toward the side it was flicked**, which is what
/// [`Sideshift::Right`] to `+`[`crate::ship::Body::right`] already meant. The
/// coin flip lands on the side the crate had guessed - a confirmation rather than
/// a fix, and now it is evidence instead of a guess.
///
/// # The trigger
///
/// Both of the original's gestures are in [`advance_sideshift`] now. They are
/// per-craft state in the original and so they are per-craft state here; the
/// input layer supplies buttons, not decisions. See
/// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
#[must_use]
pub fn sideshift_force(state: &ShipState, handling: &Handling, grounded: f32) -> Vec3 {
    if grounded <= 0.0 {
        return Vec3::ZERO;
    }

    let right = state.body.right();
    let mut force = Vec3::ZERO;
    // Left first, matching the field's own order. Both can run at once, and the
    // original lets the two forces cancel rather than picking a winner.
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
/// Ported from `Ship_UpdateSideshiftInput_q` (`0x08846a54`) in the order that
/// function has: count every timer down, run whichever gesture the pilot is
/// making, then refresh the lockout from whatever ended up running. Doing the
/// countdown first is what makes a shift last
/// [`SIDESHIFT_DURATION`] rather than one tick more.
///
/// # The two gestures
///
/// The original ships **two control schemes** and the sideshift is a different
/// gesture in each - see
/// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`, which reads the options
/// module that decides between them.
///
/// - **Novice** holds one dedicated button (`OPT_CTRL_SS`, bound to `L` by
///   default) and *flicks* the stick. The flick has to arm first, by the axis
///   being inside [`SIDESHIFT_FLICK_THRESHOLD`], and fires when it crosses back
///   out. **The craft shifts toward the side it was flicked.**
/// - **Veteran** has no sideshift button - it spends `L` and `R` on the two
///   airbrakes - and *double-taps* an airbrake instead, within
///   [`SIDESHIFT_TAP_WINDOW`].
///
/// Both machines run here unconditionally, which is not the original's shape:
/// there, the scheme flag picks one branch. It is safe because the input layer
/// only ever fills the fields of the live scheme
/// (`oag_gameplay::controls::ship_controls` takes the scheme and zeroes the
/// other), so the dormant machine sees nothing to act on. Keeping the scheme
/// out of this crate is deliberate: it is an options setting, not physics.
///
/// # What is deliberately not ported
///
/// The original also gates the whole block on `craft+0x1c0 & 2`, a flag bit
/// nothing has identified (`engine.md` names eleven bits of that word and only
/// bit 0 is established). It is left out rather than guessed at; the effect is
/// that our sideshift is available in a state the original may withhold it in.
pub fn advance_sideshift(state: &mut ShipState, input: &ShipControls, dt: f32) {
    // The original's own countdown is gated, not clamped - `if (t > 0.0f) t
    // -= dt;` - so it lands slightly negative on the tick that crosses zero
    // and then freezes there, where `.max(0.0)` here converges to exactly
    // `0.0`. Deliberately not matched: every reader of these fields, original
    // and port alike, gates on `> 0.0`/`<= 0.0`, so the residue changes
    // nothing observable, and these three feed `ShipState::hash_state`'s
    // committed golden hashes. See
    // `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`'s countdown
    // section for the decompiled evidence.
    for timer in &mut state.sideshift_timers {
        *timer = (*timer - dt).max(0.0);
    }
    for window in &mut state.shift_tap_windows {
        *window = (*window - dt).max(0.0);
    }
    state.shift_lockout = (state.shift_lockout - dt).max(0.0);

    // `ShipControls::sideshift` is the direct request and bypasses the lockout;
    // the two gestures do not. Both sides are tracked separately because the
    // original arms two independent timers and lets the forces cancel rather
    // than picking a winner.
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

        // A tap is ignored outright while that side is already shifting, which
        // is the original's own `timer <= 0` guard and not a second lockout.
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
