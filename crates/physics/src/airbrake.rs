//! Airbrakes, lateral grip and the sideshift impulse.
//!
//! The force law is from `docs/physics/README.md`; which accumulator each term
//! writes, and which frame it is expressed in, is from
//! `docs/ghidra/functions/psp-pulse/engine.md`. The two things this module exists to
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
//!   `docs/ghidra/functions/psp-pulse/engine.md` narrowed it: **no control input** writes
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
/// `docs/ghidra/functions/psp-pulse/engine.md` establishes as the `vabs.s`-ed
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
/// `docs/ghidra/functions/psp-pulse/engine.md` ("The basis is positively
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
/// `docs/ghidra/functions/psp-pulse/engine.md`:
///
/// - It writes the **body-local** force accumulator directly rather than the world
///   one, which is the only reason we know that accumulator's `.x` is the right axis.
/// - It runs **after** hover, so `grounded` here is **this** frame's contact
///   fraction, unlike the engine, the brakes, drag, gravity and pitch. The caller also
///   gates it on the leap timer having expired.
#[must_use]
pub fn lateral_grip(state: &ShipState, handling: &Handling, grounded: f32) -> Vec3 {
    let right = state.body.right();
    let lateral_velocity = state.body.linear_velocity.dot(right);
    let k = lateral_grip_coefficient(handling, state.airbrake_left, state.airbrake_right);

    let mut lateral = handling.antigrav.grip_ground * lateral_velocity * k * grounded;
    lateral += handling.antigrav.grip_air * lateral_velocity * k * (1.0 - grounded);

    Vec3::new(lateral, 0.0, 0.0)
}

/// How long one sideshift pushes for, in seconds.
///
/// The literal `0.2` (`0x3e4ccccd`) that `Ship_UpdateSideshiftInput_q`
/// (`0x08846a54`) writes into whichever of its two per-side timers fired. The
/// timers count down by `dt` and the craft flag - and therefore the force - lasts
/// exactly as long as the timer does. Confidence **80**; see
/// `docs/ghidra/functions/psp-pulse/engine.md`.
pub const SIDESHIFT_DURATION: f32 = 0.2;

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
/// (`docs/ghidra/functions/psp-pulse/engine.md`, measured - the same convention
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
/// # What is still not implemented
///
/// The *trigger*. The original arms a sideshift from a stick flick - the axis has
/// to return inside `+/-10` to re-arm - or from a tap pattern in a three-entry
/// history, and it is `oag-input`'s business rather than this crate's. Here the
/// [`Sideshift`] input stays the "fire now" edge, and firing refreshes the timer.
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

/// Arms the timer a fired sideshift runs on, and counts both down by `dt`.
///
/// The countdown is the original's, in `Ship_UpdateSideshiftInput_q`; the arming
/// is this crate's stand-in for a trigger that lives in the input layer. Firing
/// while a timer runs refreshes it, as a second flick does there.
pub fn advance_sideshift(state: &mut ShipState, input: &ShipControls, dt: f32) {
    for timer in &mut state.sideshift_timers {
        *timer = (*timer - dt).max(0.0);
    }
    match input.sideshift {
        Sideshift::None => {}
        Sideshift::Left => state.sideshift_timers[0] = SIDESHIFT_DURATION,
        Sideshift::Right => state.sideshift_timers[1] = SIDESHIFT_DURATION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{Airbrake, Antigrav};
    use crate::ship::Body;

    /// Arbitrary round numbers, in the scaled in-memory form: `amount` and
    /// `slidegrip` are what the loader would have stored, not what the XML holds.
    /// **Not recovered values.**
    fn test_handling() -> Handling {
        Handling {
            airbrake: Airbrake {
                // An XML `amount` of 20 would be stored as 0.002.
                amount: 0.002,
                drag: 5.0,
                falloff: 400.0,
                gain: 800.0,
                turn: 3.0,
                // An XML `slidegrip` of 50 is stored as 0.005.
                slidegrip: 0.005,
                sideshift: 6.0,
            },
            antigrav: Antigrav {
                grip_air: 1.0,
                grip_ground: 2.0,
                ..Antigrav::default()
            },
            ..Handling::ZERO
        }
    }

    /// Forty units per second forwards and three to the right. Forward is `-Z`.
    fn moving_ship() -> ShipState {
        ShipState {
            body: Body {
                linear_velocity: Vec3::new(3.0, 0.0, -40.0),
                ..Body::default()
            },
            ..ShipState::default()
        }
    }

    /// Braking one side must turn the nose **toward** that side, and in this
    /// crate's frame that is a **positive** `local_angular.y` for the left
    /// brake. It is the same convention [`crate::engine::steering`] satisfies by
    /// negating its own literal law, and the same identity `crate::forces`'s
    /// `holding_right_turns_the_ship_toward_its_own_right_axis` checks end to
    /// end for steering.
    ///
    /// A test that only asserted `left == -right` would pass with the whole term
    /// inverted, which is exactly the bug this pins: from `5ad69f3` until Task
    /// #34 the airbrake yaw ran the other way, because the steering fix was
    /// applied in `engine::steering` alone and this module shares the
    /// convention.
    #[test]
    fn braking_the_left_side_yaws_the_nose_left_and_pushes_the_body_right() {
        let handling = test_handling();
        let mut state = moving_ship();
        state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
        let input = ShipControls::default();

        state.airbrake_left = 100.0;
        state.airbrake_right = 0.0;
        let left_brake = evaluate(&state, &input, &handling, 40.0);

        state.airbrake_left = 0.0;
        state.airbrake_right = 100.0;
        let right_brake = evaluate(&state, &input, &handling, 40.0);

        assert!(
            left_brake.local_angular.y > 0.0,
            "the left brake yawed {}, and nose-left is positive here",
            left_brake.local_angular.y
        );
        assert!(
            right_brake.local_angular.y < 0.0,
            "the right brake yawed {}, and nose-right is negative here",
            right_brake.local_angular.y
        );

        // The original writes the lateral term along `craft+0x170`, which is the
        // ship's left, so braking left pushes the body to the right: the craft
        // rotates into the corner while its mass runs wide. `+X` is right here,
        // and the ship is aimed along `-Z`, so `.x` isolates it.
        assert!(
            left_brake.world_force.x > 0.0,
            "the left brake pushed {} laterally, expected +X (right)",
            left_brake.world_force.x
        );
        assert!(right_brake.world_force.x < 0.0);
    }

    /// The confidence-85 negative, pinned as a test because it is the exact thing a
    /// reimplementation is tempted to "fix".
    #[test]
    fn the_airbrake_path_never_writes_a_roll_torque() {
        let handling = test_handling();
        let mut state = moving_ship();

        for (left, right) in [(100.0, 0.0), (0.0, 100.0), (100.0, 100.0), (35.0, 75.0)] {
            state.airbrake_left = left;
            state.airbrake_right = right;
            for steer in [-1.0, 0.0, 0.6] {
                let input = ShipControls {
                    steer_x: steer,
                    ..ShipControls::default()
                };
                let forces = evaluate(&state, &input, &handling, 40.0);
                assert_eq!(
                    forces.local_angular.z, 0.0,
                    "left {left} right {right} steer {steer}"
                );
                assert_eq!(forces.local_angular.x, 0.0);
            }
        }
    }

    /// Symmetric input, symmetric result: with the two sides equal there is no
    /// imbalance, so no lateral force and no yaw, whatever else is going on.
    #[test]
    fn equal_airbrakes_produce_no_lateral_force_and_no_yaw() {
        let handling = test_handling();
        let mut state = moving_ship();
        state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
        state.airbrake_left = 60.0;
        state.airbrake_right = 60.0;

        let input = ShipControls {
            steer_x: 0.5,
            ..ShipControls::default()
        };
        let forces = evaluate(&state, &input, &handling, 40.0);

        assert_eq!(forces.local_angular, Vec3::ZERO);
        assert_eq!(forces.world_force.x, 0.0);
    }

    /// Mirroring the two sides mirrors the result exactly, which pins that no term
    /// treats one side preferentially.
    #[test]
    fn mirroring_the_airbrakes_negates_the_lateral_force_and_the_yaw() {
        let handling = test_handling();
        let input = ShipControls {
            steer_x: 0.5,
            ..ShipControls::default()
        };

        let mut state = moving_ship();
        state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);

        state.airbrake_left = 100.0;
        state.airbrake_right = 0.0;
        let left_heavy = evaluate(&state, &input, &handling, 40.0);

        state.airbrake_left = 0.0;
        state.airbrake_right = 100.0;
        let right_heavy = evaluate(&state, &input, &handling, 40.0);

        assert_eq!(left_heavy.local_angular, -right_heavy.local_angular);
        assert_eq!(left_heavy.world_force.x, -right_heavy.world_force.x);
        // The forward slide term uses |L - R| and so is unchanged by mirroring.
        assert_eq!(left_heavy.world_force.z, right_heavy.world_force.z);
    }

    #[test]
    fn the_slide_term_needs_both_an_imbalance_and_steering() {
        let handling = test_handling();
        let mut state = moving_ship();
        state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
        state.airbrake_left = 100.0;
        state.airbrake_right = 0.0;

        let straight = evaluate(&state, &ShipControls::default(), &handling, 40.0);
        assert_eq!(straight.world_force.z, 0.0);

        let turning = evaluate(
            &state,
            &ShipControls {
                steer_x: 1.0,
                ..ShipControls::default()
            },
            &handling,
            40.0,
        );
        assert_ne!(turning.world_force.z, 0.0);
    }

    /// The `drag` term's exact magnitude, in the binary's own association order.
    ///
    /// Asserted against a recomputed product rather than a decimal literal on
    /// purpose: both `0.01f32` and `0.001f32` are inexact, so the chain does not
    /// land on the round number the algebra suggests, and a literal would either
    /// fail or force a tolerance that stops pinning anything.
    #[test]
    fn the_airbrake_drag_term_has_the_magnitude_the_instruction_stream_forms() {
        let handling = test_handling();
        let mut state = moving_ship();
        state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
        state.airbrake_left = 100.0;
        state.airbrake_right = 0.0;
        let input = ShipControls {
            steer_x: 1.0,
            ..ShipControls::default()
        };

        let forces = evaluate(&state, &input, &handling, 40.0);

        // `slide` first, then `(forward * speed) * slide * 0.001` - the two
        // `vscl.q`s and the two literals, in order.
        let slide = 100.0f32 * handling.airbrake.drag * 1.0 * 0.01;
        let expected = -(40.0f32 * slide * 0.001);

        // Forward is `-Z`, and the lateral term is along `+X`, so `.z` isolates
        // the drag term.
        assert_eq!(forces.world_force.z, expected);
        assert_eq!(forces.world_force.y, 0.0);
        assert!(expected < 0.0, "the term must point along +forward");
    }

    /// Both factors are needed, and "zero" means exactly zero rather than small.
    #[test]
    fn the_drag_term_is_exactly_zero_without_both_an_imbalance_and_steering() {
        let handling = test_handling();
        let mut state = moving_ship();
        state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);

        // Equal airbrakes, full steering.
        state.airbrake_left = 70.0;
        state.airbrake_right = 70.0;
        let equal = evaluate(
            &state,
            &ShipControls {
                steer_x: 1.0,
                ..ShipControls::default()
            },
            &handling,
            40.0,
        );
        assert_eq!(equal.world_force.z, 0.0);

        // Full imbalance, no steering.
        state.airbrake_left = 100.0;
        state.airbrake_right = 0.0;
        let straight = evaluate(&state, &ShipControls::default(), &handling, 40.0);
        assert_eq!(straight.world_force.z, 0.0);
    }

    /// The gate is on `craft+0x2ec`, the **absolute** cached speed, so a
    /// reversing ship gets the term too - pointed along `+forward`, which for it
    /// is a deceleration. A gate on a signed forward speed would zero this.
    #[test]
    fn the_drag_term_points_along_forward_in_both_directions_of_travel() {
        let handling = test_handling();
        let mut state = moving_ship();
        state.airbrake_left = 100.0;
        state.airbrake_right = 0.0;
        let input = ShipControls {
            steer_x: 1.0,
            ..ShipControls::default()
        };

        state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
        let forwards = evaluate(&state, &input, &handling, 40.0);

        state.body.linear_velocity = Vec3::new(0.0, 0.0, 40.0);
        let reversing = evaluate(&state, &input, &handling, -40.0);

        assert_eq!(reversing.world_force.z, forwards.world_force.z);
        assert!(forwards.world_force.z < 0.0);
    }

    /// The slide term reads the **raw** `steerX` from the input snapshot, while
    /// the airbrake sides are the ramped states. Pinned because unifying the two
    /// is a natural-looking tidy-up that would diverge.
    #[test]
    fn the_drag_term_reads_the_raw_steering_input_not_a_ramped_state() {
        let handling = test_handling();
        let mut state = moving_ship();
        state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
        state.airbrake_left = 100.0;
        state.airbrake_right = 0.0;

        let half = evaluate(
            &state,
            &ShipControls {
                steer_x: 0.5,
                ..ShipControls::default()
            },
            &handling,
            40.0,
        );
        let full = evaluate(
            &state,
            &ShipControls {
                steer_x: 1.0,
                ..ShipControls::default()
            },
            &handling,
            40.0,
        );

        // Linear in the raw input, with no ramp state anywhere in between.
        assert_eq!(half.world_force.z, full.world_force.z * 0.5);
        // And it is `|steerX|`, so the opposite lock gives the same thing.
        let mirrored = evaluate(
            &state,
            &ShipControls {
                steer_x: -1.0,
                ..ShipControls::default()
            },
            &handling,
            40.0,
        );
        assert_eq!(mirrored.world_force.z, full.world_force.z);
    }

    #[test]
    fn a_stationary_ship_gets_nothing_from_the_airbrakes() {
        let handling = test_handling();
        let state = ShipState {
            airbrake_left: 100.0,
            ..ShipState::default()
        };
        let forces = evaluate(
            &state,
            &ShipControls {
                steer_x: 1.0,
                ..ShipControls::default()
            },
            &handling,
            0.0,
        );
        assert_eq!(forces.world_force, Vec3::ZERO);
        assert_eq!(forces.local_angular, Vec3::ZERO);
    }

    #[test]
    fn lateral_grip_opposes_sideways_velocity() {
        let handling = test_handling();
        let state = moving_ship();
        // Sliding right, so the grip term must push left, along local -X.
        assert!(lateral_grip(&state, &handling, 1.0).x < 0.0);
    }

    /// The two endpoints of the resolved `slidegrip` reading, on the scaled value.
    #[test]
    fn grip_is_unchanged_by_the_airbrakes_at_a_slidegrip_of_one_hundred() {
        let handling = Handling {
            airbrake: Airbrake {
                // XML 100, scaled by 1e-4.
                slidegrip: 0.01,
                ..test_handling().airbrake
            },
            ..test_handling()
        };
        assert_eq!(lateral_grip_coefficient(&handling, 0.0, 0.0), -1.0);
        assert_eq!(lateral_grip_coefficient(&handling, 100.0, 100.0), -1.0);
    }

    #[test]
    fn grip_vanishes_at_full_airbrake_with_a_slidegrip_of_zero() {
        let handling = Handling {
            airbrake: Airbrake {
                slidegrip: 0.0,
                ..test_handling().airbrake
            },
            ..test_handling()
        };
        assert_eq!(lateral_grip_coefficient(&handling, 0.0, 0.0), -1.0);
        assert_eq!(lateral_grip_coefficient(&handling, 100.0, 0.0), 0.0);
    }

    /// Half airbrake lands between the two endpoints, which is what makes this a
    /// slide control rather than a switch.
    #[test]
    fn grip_falls_off_progressively_between_those_endpoints() {
        let handling = Handling {
            airbrake: Airbrake {
                slidegrip: 0.0,
                ..test_handling().airbrake
            },
            ..test_handling()
        };
        let none = lateral_grip_coefficient(&handling, 0.0, 0.0);
        let half = lateral_grip_coefficient(&handling, 50.0, 0.0);
        let full = lateral_grip_coefficient(&handling, 100.0, 0.0);

        assert_eq!(none, -1.0);
        assert_eq!(full, 0.0);
        assert!(half > none && half < full);
    }

    #[test]
    fn groundedness_selects_between_the_two_grip_coefficients() {
        let handling = test_handling();
        let state = moving_ship();

        let ground = lateral_grip(&state, &handling, 1.0).x;
        let air = lateral_grip(&state, &handling, 0.0).x;
        let half = lateral_grip(&state, &handling, 0.5).x;

        // `grip_ground` is twice `grip_air` in the fixture, so grounded grip is
        // stronger and half-grounded sits between the two.
        assert!(ground < air);
        assert!(half < air && half > ground);
    }

    /// A sideshift pushes toward the side it was fired at, for as long as its
    /// timer runs, and only while the craft is on the ground.
    ///
    /// The direction is the recovered one rather than the guessed one - see
    /// [`sideshift_force`] - and it is asserted both ways round so an
    /// unconditional push cannot pass.
    #[test]
    fn a_sideshift_pushes_toward_the_side_it_was_fired_at_while_grounded() {
        let handling = test_handling();
        let mut state = ShipState::default();

        // Nothing fired: no force, and the timers stay down.
        advance_sideshift(&mut state, &ShipControls::default(), 1.0 / 60.0);
        assert_eq!(sideshift_force(&state, &handling, 1.0), Vec3::ZERO);

        let fire = |side| ShipControls {
            sideshift: side,
            ..ShipControls::default()
        };

        let mut right = ShipState::default();
        advance_sideshift(&mut right, &fire(Sideshift::Right), 1.0 / 60.0);
        assert_eq!(
            sideshift_force(&right, &handling, 1.0),
            Vec3::new(6.0, 0.0, 0.0)
        );
        // Switched off in the air, which the old velocity-change shape was not.
        assert_eq!(sideshift_force(&right, &handling, 0.0), Vec3::ZERO);

        let mut left = ShipState::default();
        advance_sideshift(&mut left, &fire(Sideshift::Left), 1.0 / 60.0);
        assert_eq!(
            sideshift_force(&left, &handling, 1.0),
            Vec3::new(-6.0, 0.0, 0.0)
        );

        // Both at once cancel, as the original's two flags do.
        let mut both = ShipState::default();
        advance_sideshift(&mut both, &fire(Sideshift::Left), 1.0 / 60.0);
        advance_sideshift(&mut both, &fire(Sideshift::Right), 1.0 / 60.0);
        assert_eq!(sideshift_force(&both, &handling, 1.0), Vec3::ZERO);
    }

    /// It lasts [`SIDESHIFT_DURATION`] and then stops - it is not one frame, and
    /// it is not forever.
    #[test]
    fn a_sideshift_pushes_for_its_whole_timer_and_then_stops() {
        let handling = test_handling();
        let mut state = ShipState::default();
        let dt = 1.0 / 60.0;

        advance_sideshift(
            &mut state,
            &ShipControls {
                sideshift: Sideshift::Right,
                ..ShipControls::default()
            },
            dt,
        );

        let mut pushing = 0;
        for _ in 0..60 {
            if sideshift_force(&state, &handling, 1.0) != Vec3::ZERO {
                pushing += 1;
            }
            advance_sideshift(&mut state, &ShipControls::default(), dt);
        }

        // 0.2 s at 60 Hz, plus the frame it fired on: the original arms the timer
        // after the countdown, in the same order this does, so a fired sideshift
        // always gets its whole duration and never a partial first frame.
        assert_eq!(pushing, (SIDESHIFT_DURATION / dt).round() as u32 + 1);
    }

    #[test]
    fn a_zero_handling_ship_gets_nothing_from_the_airbrake_path() {
        let state = moving_ship();
        let input = ShipControls {
            steer_x: 1.0,
            steer_y: 0.0,
            thrust: 1.0,
            airbrake_left: 1.0,
            airbrake_right: 0.0,
            sideshift: Sideshift::Right,
        };
        let forces = evaluate(&state, &input, &Handling::ZERO, 40.0);
        assert_eq!(forces.world_force, Vec3::ZERO);
        assert_eq!(forces.local_angular, Vec3::ZERO);
        assert_eq!(lateral_grip(&state, &Handling::ZERO, 1.0), Vec3::ZERO);
        let mut fired = state;
        advance_sideshift(&mut fired, &input, 1.0 / 60.0);
        assert_eq!(sideshift_force(&fired, &Handling::ZERO, 1.0), Vec3::ZERO);
    }
}
