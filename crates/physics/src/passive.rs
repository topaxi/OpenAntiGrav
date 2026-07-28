//! The passive terms: drag, resistance, damping and gravity.
//!
//! Each is small, always on, and easy to overlook, and together they are what makes
//! the ship handle at all: there is **no speed-dependent steering**, so the drag,
//! the weathervane torque and the angular damping in this module carry that load.
//!
//! # There *is* a dedicated airbrake drag term, and it is not implemented
//!
//! An earlier revision of this doc stated "there is no dedicated airbrake drag
//! term". That is false. `Ship_UpdateAirbrakes` reads `Airbrake.drag` (class
//! `+0xe8`, reached as `+0x54` through the `craft+0x70` pointer) at `0x0884cce8`
//! in the PSP `BOOT.BIN` and forms, gated on `forwardSpeed > 0`:
//!
//! ```text
//! forward * |airbrake_l - airbrake_r| * Airbrake.drag * |steer| * 1e-5 * forwardSpeed
//! ```
//!
//! at `0x0884ccc8`-`0x0884cdb4`. Confidence 88, read from the instruction stream
//! with the Allegrex module. **The scale is `1e-5`, not the `0.01` an earlier
//! capstone reading recorded**: there are two literals, `0x3c23d70a` (`0.01`) at
//! `0x0884ccf4` and `0x3a83126f` (`0.001`) at `0x0884cd78`, both applied to the
//! same vector. The direction is `craft+0x180`, the ship's own forward axis.
//! It belongs in [`crate::airbrake`] rather than here, and it is unimplemented:
//! it is identically zero in both captured traces (both hold the airbrakes equal
//! and the steering at zero), so nothing available today can verify it. A capture
//! with *asymmetric* airbrake input is what would test it.
//!
//! Recorded here because this module is where someone would look for it. It used
//! to be flagged as a candidate for the missing speed-proportional resistance
//! `docs/physics/force-balance-ground-truth.md` tracked as the M4 blocker;
//! **that blocker is resolved and there is no missing resistance**. The force law
//! in this crate reproduces a real standing-start capture from `fs = 0.56` to
//! `fs = 47.67` at rms `0.127` units of force, and the `~52`-unit deficit in the
//! two older captures is a per-frame *velocity* reduction applied by the
//! collision path, outside every force accumulator. So this term is still worth
//! implementing for fidelity, and it is still zero in every capture available,
//! but nothing hangs on it.
//!
//! From `docs/ghidra/functions/psp-pulse/engine.md`, "The passive terms" and the
//! gravity paragraph. Confidence 74 to 80, decompilation only, nothing
//! runtime-verified - **except the four drag coefficients**, which have since
//! been read straight out of the PSP `BOOT.BIN` instruction stream in
//! `Ship_ApplyQuadraticDrag` (`0x08848e28`) as the immediates `0xbf666666`
//! (`-0.9`, the `craft+0x2a4 == 0` branch), `0xbdcccccd` ([`DRAG_REVERSING`]),
//! `0xbba3d70a` ([`DRAG_GROUND`]) and `0xbb03126f` ([`DRAG_AIR`]), against the
//! `0xbe4ccccd` (`-0.2`) threshold. Those four are confidence **95**.
//!
//! [`ROLLING_RESISTANCE`] is now confirmed **including its magnitude**, read out
//! of Ghidra with the Allegrex module rather than by hand:
//! `Ship_ApplyRollingResistance` (`0x08848f4c`) is nine instructions -
//! `vmul.t`/`vfad.t`/`vrsq.s` to build `1/|v|`, a `vpfxs [-X,-Y,-Z,0]` on the
//! `vscl.t` to give `-v/|v|`, and then `vadd.t C610,C610,C610`, a **self-add
//! that doubles it**. So the magnitude is exactly `2`, with no parameter and no
//! prefix-encoded literal anywhere. Confidence 92.
//!
//! # `craft+0x2ec` is `|dot(velocity, forward)|`, and that makes one branch dead
//!
//! `Ship_UpdateCraft` computes the cached speed at `0x0884992c`-`0x08849938` as
//! `vdot.t` **followed by `vabs.s`**, so the field every passive term reads is
//! non-negative. `search_instructions` over every `0x2ec(` displacement finds
//! exactly one writer on a craft base (that store), so nothing else can make it
//! negative. Two consequences, both implemented here:
//!
//! - `Ship_ApplyQuadraticDrag`'s `craft+0x2ec < -0.2` test can never be true, so
//!   [`DRAG_REVERSING`] is **unreachable in the original**. It is kept as a
//!   recorded constant rather than deleted, because the same `-0.1` immediate is
//!   present in the PS2 build too and the branch exists.
//! - The drag term is therefore `k * |forwardSpeed| * velocity` with `k < 0`, so
//!   it is dissipative *in both directions of travel*. The worry recorded below
//!   on [`quadratic_drag`] - that the literal form adds energy while reversing -
//!   was an artefact of feeding it a signed speed, and is resolved.
//!
//! Confidence 90 on the `vabs.s`, 88 on the sole-writer negative (the scan cannot
//! see a store through a rebased pointer, the same caveat `engine.md` records).

use oag_core::math::Vec3;

use crate::params::Handling;

/// Quadratic drag coefficient while reversing, `forwardSpeed < -0.2`.
///
/// Twenty to fifty times the forward coefficients, which is what makes the ship
/// reluctant to fly backwards.
pub const DRAG_REVERSING: f32 = -0.1;

/// Quadratic drag coefficient while grounded.
pub const DRAG_GROUND: f32 = -0.005;

/// Quadratic drag coefficient while airborne.
///
/// **Smaller than [`DRAG_GROUND`]**, not larger. Worth stating because the
/// intuitive ordering is the other way round and a reimplementation that "fixed"
/// it would change how far a ship carries off a jump.
pub const DRAG_AIR: f32 = -0.002;

/// Forward speed below which the reversing drag coefficient applies.
pub const DRAG_REVERSE_THRESHOLD: f32 = -0.2;

/// The always-on rolling resistance magnitude, applied along `-unit(velocity)`.
///
/// Constant in magnitude, independent of speed and of every handling parameter.
/// **Gated on `forwardSpeed > 0`**, so it does not act on a ship that is reversing
/// or moving purely sideways - which is also what keeps `unit(velocity)` from being
/// asked for at rest.
pub const ROLLING_RESISTANCE: f32 = 2.0;

/// Weathervane coefficient while grounded.
///
/// # Why this is `+0.1` where the page writes `-0.1`
///
/// `docs/ghidra/functions/psp-pulse/engine.md` writes
/// `angularWorld += cross(forward, velocity) * (grounded ? -0.1 : -0.3)` and describes
/// the term, two lines later, as "what turns the nose toward the direction of travel".
/// In **this crate's** right-handed frame those two statements are incompatible, by
/// the same identity that settled the surface-alignment torque:
///
/// ```text
/// omega = f x v
/// d(f)/dt = omega x f = (f x v) x f = v * (f . f) - f * (v . f) = v - f * (v . f)
/// ```
///
/// which points from `f` toward `v`. So a torque along `+cross(forward, velocity)`
/// turns the nose toward travel and a negative coefficient turns it away, whatever the
/// magnitude. An anti-weathervaning term would make every ship spin out of every
/// corner, so the behaviour the page states is implemented and the sign it writes is
/// not.
///
/// **This is the second independent term with that pattern**, and the pair is more
/// informative than either alone: both this and `-400 * cross(up, avgNormal)` carry a
/// negative coefficient on a cross product that has to be positive for the behaviour
/// the same page describes. Two separate transcription errors of the same shape is a
/// poor explanation; a **systematic handedness difference** between the original's
/// frame and this crate's is a good one, and the page lists handedness as not
/// determined. That is now the leading explanation for both, and it predicts that any
/// further cross-product term recovered from this path will need the same flip - a
/// prediction worth checking rather than a conclusion.
///
/// Confidence 84 on the direction, the rubric's ceiling for decompilation-only
/// evidence. **The magnitude is unverified** and stays M3 work.
pub const WEATHERVANE_GROUND: f32 = 0.1;

/// Weathervane coefficient while airborne, three times the grounded one.
///
/// See [`WEATHERVANE_GROUND`] for the sign.
pub const WEATHERVANE_AIR: f32 = 0.3;

/// Yaw angular damping, hard-coded for every craft in the game.
pub const YAW_DAMPING: f32 = -5.0;

/// Roll angular damping, hard-coded for every craft in the game.
///
/// `-5.0` in the undecoded `craft+0x2a4` mode 0 and `-2.0` otherwise; mode 0 also
/// disables `rebound` and multiplies drag by about 180, so it reads as a front-end
/// or display mode rather than a racing one. The racing value is used here and the
/// mode is not implemented, since interpreting that enum is a guess at confidence
/// 40.
pub const ROLL_DAMPING: f32 = -2.0;

/// Vertical damping coefficient, along the ship's own up axis.
///
/// **Airborne only** - see [`vertical_damping`], which is scaled by
/// `1 - grounded` and is therefore identically zero on a fully grounded craft.
pub const VERTICAL_DAMPING: f32 = -0.25;

/// Quadratic drag, as a world-space force.
///
/// ```text
/// k = forwardSpeed < -0.2 ? -0.1 : grounded ? -0.005 : -0.002
/// worldForce += velocity * forwardSpeed * k
/// ```
///
/// `grounded` is the **previous** frame's contact flag, and `forward_speed` is
/// signed: `dot(velocity, forward)`.
///
/// **`forward_speed` is the cached `craft+0x2ec`, which is `|dot(velocity,
/// forward)|`** - see this module's header. That settles what used to be recorded
/// here as an open sign question: with a non-negative speed and `k < 0` the power
/// delivered, `|v|^2 * forwardSpeed * k`, is negative in *both* directions of
/// travel, so the term is dissipative while reversing too and adds energy nowhere.
///
/// It also means the `forward_speed < -0.2` arm can never be taken by the
/// original, so [`DRAG_REVERSING`] is unreachable there. The branch is kept
/// because the immediate is genuinely in both binaries; a caller that passes a
/// signed speed will exercise it and diverge.
///
/// The exact shape, from `Ship_ApplyQuadraticDrag` (`0x08848e28`): the coefficient
/// is selected first (a four-way branch, `craft+0x2a4 == 0` taking `-0.9`), then
/// `vscl.t C600,C200,S310` scales the **whole velocity vector** by the scalar
/// speed and `vscl.t C600,C600,S610` by the coefficient. So the lateral components
/// of the velocity are dragged as well; projected onto `forward` it is `k * fs^2`,
/// which is why the along-track balance sees a quadratic.
#[must_use]
pub fn quadratic_drag(velocity: Vec3, forward_speed: f32, grounded: bool) -> Vec3 {
    let k = if forward_speed < DRAG_REVERSE_THRESHOLD {
        DRAG_REVERSING
    } else if grounded {
        DRAG_GROUND
    } else {
        DRAG_AIR
    };

    velocity * forward_speed * k
}

/// Rolling resistance, as a world-space force.
///
/// Zero unless `forward_speed > 0`, which is both what the original does and what
/// keeps this finite at rest.
#[must_use]
pub fn rolling_resistance(velocity: Vec3, forward_speed: f32) -> Vec3 {
    if forward_speed <= 0.0 {
        return Vec3::ZERO;
    }

    let speed = velocity.length();
    if speed <= 0.0 {
        return Vec3::ZERO;
    }

    -(velocity / speed) * ROLLING_RESISTANCE
}

/// The weathervane torque, in the world angular accumulator.
///
/// ```text
/// angularWorld += cross(forward, velocity) * (grounded ? -0.1 : -0.3)
/// ```
///
/// **This, and not any control term, is what turns the nose toward the direction of
/// travel**, and it is three times stronger in the air. `grounded` here is **this**
/// frame's contact flag: the weathervane runs after hover in the original's order.
///
/// The coefficients' **sign is flipped** relative to the page; see
/// [`WEATHERVANE_GROUND`] for the derivation and for why a handedness difference is
/// the leading explanation.
///
/// Note that "travel" is three-dimensional: a ship falling straight down gets a
/// nose-down pitch out of this, because `cross(forward, velocity)` has a pitch
/// component whenever the velocity leaves the ship's own horizontal plane. That is a
/// property of the term as recovered, not an artefact.
#[must_use]
pub fn weathervane(forward: Vec3, velocity: Vec3, grounded: bool) -> Vec3 {
    let k = if grounded {
        WEATHERVANE_GROUND
    } else {
        WEATHERVANE_AIR
    };

    forward.cross(velocity) * k
}

/// Angular damping, in the body-local angular accumulator.
///
/// ```text
/// angularLocal += (-pitch_damping, -5.0, -2.0) * bodyAngularVelocityLocal
/// ```
///
/// Only the pitch axis is tunable per ship; yaw and roll damping are hard-coded for
/// every craft in the game. `pitch_damping` is negated here and is expected
/// positive in the data, which is the same convention `Brakes.amount` does *not*
/// use - there the sign is in the parameter. Worth keeping straight.
#[must_use]
pub fn angular_damping(handling: &Handling, local_angular_velocity: Vec3) -> Vec3 {
    let coefficients = Vec3::new(-handling.pitch.pitch_damping, YAW_DAMPING, ROLL_DAMPING);

    coefficients * local_angular_velocity
}

/// Vertical damping, as a world-space force.
///
/// ```text
/// worldForce += up * dot(up, velocity) * -0.25 * (1 - grounded)
/// ```
///
/// Along the ship's own up axis, not world up.
///
/// # The scale is `1 - grounded`, not `1 - magLockBlend`
///
/// This term is inlined in `Ship_UpdateCraft` (step 14) rather than living in a
/// function of its own, and it was previously transcribed with the magstrip
/// blend. It is not that field. Read at instruction level in the PSP `BOOT.BIN`:
///
/// ```text
/// 08849c60  lwc1   f12,0x2b0(s0)     ; craft+0x2b0 == the 0/0.5/1 grounded fraction
/// 08849c68  mtc1   a0,f13            ; 1.0f  (0x3f800000)
/// 08849c6c  sub.s  f12,f13,f12       ; 1 - grounded
/// 08849c74  mtc1   a0,f14            ; -0.25f (0xbe800000)
/// 08849c78  mul.s  f12,f12,f14
/// 08849c94  lv.q   C110,0x10(a1)     ; a1 == craft+0x80, so C110 == craft+0x90 == up
/// 08849ca0  vdot.t S601,C110,C200    ; dot(up, velocity)
/// 08849ca4  vscl.t C610,C110,S601
/// 08849ca8  vscl.t C610,C610,S600    ; * -0.25 * (1 - grounded)
/// 08849cac  vadd.t C300,C300,C610    ; craft+0x330, the world force accumulator
/// ```
///
/// `craft+0x280` is the magstrip blend (it is what `Ship_UpdateMagLock` ramps and
/// what the hover spring scales by); `craft+0x2b0` is the grounded fraction the
/// hover loop rebuilds each frame. The two are distinct fields and this term reads
/// the second one. Confidence **92**.
///
/// So **a grounded craft gets no vertical damping at all**, and the term only ever
/// acts in the air or on a half-contact. A reimplementation that applied it while
/// grounded adds an extra suspension damper the original does not have.
///
/// `grounded` here is **this** frame's fraction: the term runs at step 14, after
/// hover has rewritten `craft+0x2b0` at step 8.
#[must_use]
pub fn vertical_damping(up: Vec3, velocity: Vec3, grounded: f32) -> Vec3 {
    up * up.dot(velocity) * VERTICAL_DAMPING * (1.0 - grounded)
}

/// Gravity, as a world-space force.
///
/// ```text
/// worldForce.y += -( normal_gravity * classScale * mass * grounded
///                  + flight_gravity             * mass * (1 - grounded) )
/// ```
///
/// Four things about this are not what `docs/physics/README.md` guessed before the
/// term itself was read:
///
/// - **It acts on world `.y` only.** There is no track-relative component:
///   `track_gravity` appears in the hover spring's calibration and nowhere else.
/// - `normal_gravity` applies on the ground and `flight_gravity` in the air, blended
///   by the 0/0.5/1 fraction rather than switched, and **only `normal_gravity` gets
///   the per-class scale**.
/// - `grounded` is the **previous** frame's fraction.
/// - `mass` is the rigid body's, read from `body+0x374`, not
///   [`crate::params::Physical::mass`]. How the two relate was not traced.
///
/// The negation lives in the code rather than in the data, and that is checkable
/// without trusting the VFPU prefix encoding it was read from: the hover spring
/// multiplies by `normal_gravity + track_gravity` and must push the ship up, so both
/// are positive in the data, so a downward gravity force needs the sign applied here.
#[must_use]
pub fn gravity(handling: &Handling, mass: f32, class_gravity_scale: f32, grounded: f32) -> Vec3 {
    let ground = handling.physical.normal_gravity * class_gravity_scale * mass * grounded;
    let air = handling.physical.flight_gravity * mass * (1.0 - grounded);

    Vec3::new(0.0, -(ground + air), 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{Physical, Pitch};

    #[test]
    fn drag_opposes_travel_while_moving_forward() {
        let velocity = Vec3::new(0.0, 0.0, -40.0);
        let force = quadratic_drag(velocity, 40.0, true);
        assert!(force.z > 0.0, "force was {force:?}");
    }

    #[test]
    fn drag_grows_with_the_square_of_speed() {
        let slow = quadratic_drag(Vec3::new(0.0, 0.0, -10.0), 10.0, true).length();
        let fast = quadratic_drag(Vec3::new(0.0, 0.0, -20.0), 20.0, true).length();
        assert!(
            (fast - slow * 4.0).abs() < 1e-4,
            "{fast} was not four times {slow}"
        );
    }

    /// The airborne coefficient is the **smaller** one. Pinned because the
    /// intuitive ordering is the opposite and this is exactly the kind of thing a
    /// reimplementation tidies up by accident.
    #[test]
    fn airborne_drag_is_weaker_than_grounded_drag() {
        let velocity = Vec3::new(0.0, 0.0, -40.0);
        let ground = quadratic_drag(velocity, 40.0, true).length();
        let air = quadratic_drag(velocity, 40.0, false).length();
        assert!(air < ground, "air {air} was not below ground {ground}");
    }

    #[test]
    fn reversing_selects_a_much_larger_drag_coefficient() {
        let forward = quadratic_drag(Vec3::new(0.0, 0.0, -40.0), 40.0, true).length();
        let reversing = quadratic_drag(Vec3::new(0.0, 0.0, 40.0), -40.0, true).length();
        assert!(reversing > forward * 10.0);
    }

    #[test]
    fn drag_is_zero_at_rest() {
        assert_eq!(quadratic_drag(Vec3::ZERO, 0.0, true), Vec3::ZERO);
    }

    #[test]
    fn rolling_resistance_is_a_constant_magnitude_opposing_travel() {
        let slow = rolling_resistance(Vec3::new(0.0, 0.0, -1.0), 1.0);
        let fast = rolling_resistance(Vec3::new(0.0, 0.0, -100.0), 100.0);

        assert_eq!(slow, Vec3::new(0.0, 0.0, ROLLING_RESISTANCE));
        assert_eq!(fast, slow);
    }

    /// The gate is on the forward speed, not on the speed, so a ship falling
    /// straight down or sliding sideways gets none of this.
    #[test]
    fn rolling_resistance_needs_forward_motion_and_is_never_nan() {
        assert_eq!(rolling_resistance(Vec3::ZERO, 0.0), Vec3::ZERO);
        assert_eq!(
            rolling_resistance(Vec3::new(0.0, -50.0, 0.0), 0.0),
            Vec3::ZERO
        );
        assert_eq!(
            rolling_resistance(Vec3::new(0.0, 0.0, 40.0), -40.0),
            Vec3::ZERO
        );
    }

    /// The nose is turned toward travel by this term alone, so its direction is
    /// worth pinning: a ship sliding to its right must be yawed toward its right.
    #[test]
    fn the_weathervane_turns_the_nose_toward_the_direction_of_travel() {
        let forward = Vec3::NEG_Z;
        // Mostly forward, drifting to the right.
        let velocity = Vec3::new(10.0, 0.0, -40.0);
        let torque = weathervane(forward, velocity, true);

        // A right-handed rotation about -Y turns -Z toward +X, so the yaw component
        // must be negative for the nose to swing right, and this crate's forward is
        // -Z. With the sign as the page writes it, this comes out positive and the
        // nose swings away from travel; see `WEATHERVANE_GROUND`.
        assert!(torque.y < 0.0, "torque was {torque:?}");
    }

    /// The same invariant stated the way the derivation does, so it holds for any
    /// attitude and any velocity rather than only the case above.
    #[test]
    fn the_weathervane_torque_points_from_the_forward_axis_toward_the_velocity() {
        let forward = Vec3::new(0.0, 0.0, -1.0);

        for velocity in [
            Vec3::new(10.0, 0.0, -40.0),
            Vec3::new(-10.0, 0.0, -40.0),
            Vec3::new(0.0, -30.0, -40.0),
            Vec3::new(5.0, 5.0, -40.0),
        ] {
            for grounded in [true, false] {
                let torque = weathervane(forward, velocity, grounded);
                let aligning = forward.cross(velocity);
                assert!(
                    torque.dot(aligning) > 0.0,
                    "torque {torque:?} was not aligning for velocity {velocity:?}"
                );
            }
        }
    }

    #[test]
    fn the_weathervane_is_three_times_stronger_in_the_air() {
        let forward = Vec3::NEG_Z;
        let velocity = Vec3::new(10.0, 0.0, -40.0);
        let ground = weathervane(forward, velocity, true);
        let air = weathervane(forward, velocity, false);
        assert_eq!(air, ground * 3.0);
    }

    #[test]
    fn the_weathervane_is_zero_when_already_pointing_along_travel() {
        let torque = weathervane(Vec3::NEG_Z, Vec3::new(0.0, 0.0, -40.0), true);
        assert_eq!(torque, Vec3::ZERO);
    }

    #[test]
    fn angular_damping_opposes_rotation_on_every_axis() {
        let handling = Handling {
            pitch: Pitch {
                pitch_damping: 3.0,
                ..Pitch::default()
            },
            ..Handling::ZERO
        };
        let damping = angular_damping(&handling, Vec3::new(1.0, 1.0, 1.0));

        assert!(damping.x < 0.0);
        assert!(damping.y < 0.0);
        assert!(damping.z < 0.0);
    }

    /// Only the pitch axis is per ship. Yaw and roll are the same for every craft
    /// in the game, so a zeroed parameter set still damps them.
    #[test]
    fn only_the_pitch_axis_of_the_angular_damping_is_tunable() {
        let damping = angular_damping(&Handling::ZERO, Vec3::ONE);
        assert_eq!(damping.x, 0.0);
        assert_eq!(damping.y, YAW_DAMPING);
        assert_eq!(damping.z, ROLL_DAMPING);
    }

    #[test]
    fn vertical_damping_opposes_motion_along_the_ships_own_up_axis() {
        let up = Vec3::Y;
        let falling = vertical_damping(up, Vec3::new(0.0, -10.0, 0.0), 0.0);
        assert!(falling.y > 0.0);

        // Sideways motion is not damped by this term at all.
        assert_eq!(
            vertical_damping(up, Vec3::new(10.0, 0.0, 0.0), 0.0),
            Vec3::ZERO
        );
    }

    /// `Ship_UpdateCraft`'s inline step 14 scales by `1 - craft+0x2b0`, so a craft
    /// on both hover points gets nothing from this term at all.
    #[test]
    fn a_grounded_craft_gets_no_vertical_damping() {
        let falling = Vec3::new(0.0, -10.0, 0.0);
        assert_eq!(vertical_damping(Vec3::Y, falling, 1.0), Vec3::ZERO);

        // A half contact halves it, because the field is the 0/0.5/1 fraction
        // rather than a flag.
        let half = vertical_damping(Vec3::Y, falling, 0.5);
        let airborne = vertical_damping(Vec3::Y, falling, 0.0);
        assert_eq!(half, airborne * 0.5);
    }

    #[test]
    fn gravity_acts_on_world_down_only() {
        let handling = Handling {
            physical: Physical {
                normal_gravity: 10.0,
                ..Physical::default()
            },
            ..Handling::ZERO
        };
        let force = gravity(&handling, 2.0, 1.0, 1.0);
        assert_eq!(force, Vec3::new(0.0, -20.0, 0.0));
    }

    /// `track_gravity` is in the hover spring's calibration and **not** in the
    /// gravity term, which is a correction to what was previously inferred from the
    /// field names alone.
    #[test]
    fn track_gravity_is_not_part_of_the_gravity_force() {
        let mut handling = Handling::ZERO;
        handling.physical.track_gravity = 100.0;
        assert_eq!(gravity(&handling, 1.0, 1.0, 1.0), Vec3::ZERO);
        assert_eq!(gravity(&handling, 1.0, 1.0, 0.0), Vec3::ZERO);
    }

    #[test]
    fn the_two_gravities_blend_by_groundedness_and_only_one_is_class_scaled() {
        let handling = Handling {
            physical: Physical {
                normal_gravity: 10.0,
                flight_gravity: 4.0,
                ..Physical::default()
            },
            ..Handling::ZERO
        };

        assert_eq!(gravity(&handling, 1.0, 1.0, 1.0).y, -10.0);
        assert_eq!(gravity(&handling, 1.0, 1.0, 0.0).y, -4.0);
        assert_eq!(gravity(&handling, 1.0, 1.0, 0.5).y, -7.0);

        // The per-class scale reaches `normal_gravity` and not `flight_gravity`.
        assert_eq!(gravity(&handling, 1.0, 2.0, 1.0).y, -20.0);
        assert_eq!(gravity(&handling, 1.0, 2.0, 0.0).y, -4.0);
    }

    #[test]
    fn a_zero_parameter_set_has_no_gravity() {
        assert_eq!(gravity(&Handling::ZERO, 1.0, 1.0, 1.0), Vec3::ZERO);
        assert_eq!(gravity(&Handling::ZERO, 1.0, 1.0, 0.0), Vec3::ZERO);
    }
}
