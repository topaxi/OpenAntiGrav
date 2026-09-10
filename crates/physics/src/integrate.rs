//! The integrator: three explicit Euler sub-steps of `dt/3`.
//!
//! ```text
//! dt = clamp(measured_dt, 0, 0.06666)
//! evaluate all forces once at full dt
//! integrate 3 explicit Euler sub-steps of dt/3
//! ```
//!
//! From `docs/physics/README.md`, which resolved the question
//! `docs/architecture/adr/0007-fixed-timestep-vs-original.md` left open: the craft
//! path does **not** sub-step at 1/60. The sub-step *count* is fixed at three and
//! the sub-step *size* is `dt/3`, so it varies with the frame, and the original's
//! ship handling is therefore frame-rate dependent. At this project's fixed 60 Hz
//! the structure degenerates to three sub-steps of 1/180, which is exactly what
//! the original does when it holds its target frame rate.
//!
//! # The loop lives here and nowhere else
//!
//! Force evaluation is [`crate::forces::evaluate`] and it has no loop. This module
//! has the loop and cannot evaluate a force, because [`integrate`] is handed a
//! [`Body`] and not a ship, a parameter set or a raycaster. Re-evaluating forces
//! per sub-step is the obvious "improvement" and it is wrong; the split is what
//! makes it unavailable rather than merely discouraged.
//!
//! # That split decides the stability margin, and it is easy to get wrong
//!
//! Because the acceleration is computed once from the frame's starting state and
//! held, a stiff term's stability is governed by the **frame** `H = dt`, not by the
//! sub-step `H/3`. Sub-stepping refines where the state lands inside the frame; it
//! does not refresh the force, so it does not buy the stability that three genuine
//! Euler steps would. Propagating one frame of this scheme for `a = -k*theta -
//! c*theta_dot` gives
//!
//! ```text
//! det = (1 - k*H^2/3)(1 - c*H) + k*H^2 - c*k*H^3/3      stable while det <= 1
//!     <=>  c >= (2/3) * k * H   <=>   H <= 3c / (2k)
//! ```
//!
//! which is a **different and stricter** bound than the `h <= c/k` one gets by
//! assuming a force per sub-step. For the surface-alignment torque it is the
//! difference between an 11 % overshoot and a 2.22x one; the worked numbers and the
//! measurement that confirms them are on [`crate::hover::ALIGNMENT_GAIN`]. Anyone
//! re-deriving a stability margin for a term in this engine has to start here.

use oag_core::math::{Quat, Vec3};

use crate::collide::Raycaster;
use crate::forces::{self, Environment, Evaluated};
use crate::hover;
use crate::params::Handling;
use crate::ship::{Body, MAX_DT, SUBSTEPS, ShipControls, ShipState};
use crate::wall;

/// Clamps a measured frame delta to what the original will integrate.
///
/// `clamp(measured_dt, 0, MAX_DT)`. A `NaN` delta stays `NaN` deliberately rather
/// than being mapped to zero: a broken clock should show up as a broken
/// simulation, not as a ship that quietly stops.
#[must_use]
pub fn clamp_dt(measured_dt: f32) -> f32 {
    measured_dt.clamp(0.0, MAX_DT)
}

/// Integrates a body over `dt` in [`SUBSTEPS`] explicit Euler sub-steps.
///
/// The accumulated force and torque are converted to accelerations **once**, from
/// the state at the start of the frame, and every sub-step then uses those same
/// accelerations. The accumulators are not cleared; see [`step`].
///
/// # Explicit, not semi-implicit
///
/// Each sub-step reads start-of-sub-step values for *both* updates: position
/// advances by the old velocity, and only then does velocity advance by the
/// acceleration. Writing `v += a*h` first and then `p += v*h` is semi-implicit
/// (symplectic) Euler, which is a different integrator with different stability -
/// it is what one writes by reflex, and it is not what the page specifies.
/// Reproducing "explicit Euler" literally is the choice here.
///
/// # The angular-velocity sign convention, which is a whole-crate contract
///
/// This uses the **textbook** derivative, `q' = (omega as a pure quaternion) * q / 2`,
/// which is `e' = omega x e` on the basis rows. **The original does not.**
/// `Body_Integrate`'s skew matrix, read with its base constants confirmed zero,
/// builds `d(row0) = h * (-z * row1 + y * row2)`, which is `e' = e x omega` - the
/// opposite sign. So the engine's angular velocity is the negation of this one,
/// `w_game = -w_physics`, consistently and everywhere.
///
/// That single substitution explains two things that read as errors and are not:
/// `Body_AddForceAtPoint` computing torque as `F x r`, and the surface alignment being
/// `-400 * cross(up, avgNormal)`. Under `w_game = -w_physics` both are exactly what
/// the textbook `r x F` and `+400 * cross(up, n)` give. Nothing in the original is
/// inverted and nothing there needs compensating; the two candidates that were being
/// hunted for - a pre-negated force at the call site, and the matrix at `body+0xc0` -
/// are both ruled out, the latter because it is the basis's rotation inverse and a
/// determinant of `+1` cannot flip a sign.
///
/// **This is not the handedness question**, which is separately settled and fine:
/// `cross(row0, row1) = row2` on the PSP measurement, and the PS2's orthonormaliser
/// rebuilds `row0 = row1 x row2`, which is the same statement. The two were being
/// conflated and are independent.
///
/// # The rule, and what breaks if it is half-applied
///
/// Exactly **one** flip, applied consistently. This crate takes the textbook
/// integration above, so every torque expression inherited from the disassembly is
/// negated once on the way in. Today that is three places:
///
/// - [`crate::ship::Body::add_force_at_point`], `r x F` against a read `F x r`.
/// - [`crate::hover::ALIGNMENT_GAIN`], `+400` against a read `-400`.
/// - [`crate::passive::WEATHERVANE_GROUND`], `+0.1` against a read `-0.1`.
///
/// Each of those three carries its own local argument about which direction aligns, and
/// each argument is sound - but they are **one convention**, not three coincidences, and
/// "correcting" any single one back to its literal reading gives **unconditional
/// divergence** rather than the mild instability a mistuned magnitude produces.
/// `crate::ship::tests::a_force_at_a_point_makes_torque_the_textbook_way_round` pins the
/// first of them, which was previously asserted only as being non-zero.
///
/// Angular damping is deliberately excluded: `tau = -c * w` is a negative multiple of
/// `w` under either convention, so [`crate::passive::angular_damping`] is already right
/// and must not be flipped with the rest.
///
/// # Angular integration
///
/// Torque is world space and this crate treats the inertia tensor as a
/// body-space diagonal (the original does not - see below), so the
/// torque is rotated into the body, divided component-wise, and rotated back -
/// once, using the frame's starting orientation. A zero inertia component
/// contributes zero angular acceleration rather than an infinity, which is what
/// makes a zeroed parameter set safe to integrate.
///
/// **The original's diagonal is fixed in world axes, not body axes, and this
/// crate declines to follow it - chosen, not measured.** `Body_Integrate`'s
/// `R I R^T` sandwich is only the trip in and out of the frame its two fields
/// are stored in, so what the engine multiplies by is a world-axis constant;
/// that reading explains `100.00 %` of the recorded momentum column on three
/// captures. It is not adopted here because the gap it was proposed to close
/// belongs to a different identity entirely - one with no tensor in it - and
/// because adopting it is a four-site convention change ([`crate::wall`]'s
/// contact denominator and angular response, here, and
/// [`crate::forces::YAW_INVERSE_INERTIA`]) for no measured payoff, which moves
/// the committed reference hash as early as tick 600 of `Corridor`. **It is a
/// real divergence and it is carried knowingly**: at mild bank off a magstrip
/// the original does drive attitude through this path, with a world-axis
/// tensor. The measurement, the decision, what would settle it and the
/// condition to revisit it are `docs/physics/cornering-ground-truth.md`, "The
/// tensor is a world-axis diagonal". Note also that what is here is not the
/// textbook treatment either: it carries no gyroscopic `omega x (I omega)`
/// term and evaluates the rotation once per frame.
///
/// The orientation advances by the linear quaternion derivative
/// `q + 0.5 * (omega as a pure quaternion) * q * h` and is **renormalised every
/// sub-step**, not once per frame: explicit Euler on a quaternion grows its norm,
/// and letting it drift for three sub-steps before correcting would put the error
/// into the orientation used by the next sub-step's `up` axis. Deriving the
/// rotation from `sin`/`cos` of the angle would be more accurate and is
/// deliberately avoided, because transcendentals resolve to the platform's libm
/// and are a portability risk; see `docs/architecture/determinism.md`.
pub fn integrate(body: &mut Body, dt: f32) {
    // A `NaN` or infinite delta is a no-op rather than a body full of `NaN`.
    // `clamp_dt` deliberately preserves a `NaN` so a broken clock is visible at
    // the boundary; letting it past this point would poison the world snapshot,
    // and every subsequent tick would report the same failure with no way back
    // to its cause.
    if dt <= 0.0 || !dt.is_finite() {
        return;
    }

    let acceleration = if body.mass > 0.0 {
        body.force / body.mass
    } else {
        Vec3::ZERO
    };

    let local_torque = body.orientation.inverse() * body.torque;
    let local_angular_acceleration = Vec3::new(
        divide_or_zero(local_torque.x, body.inertia.x),
        divide_or_zero(local_torque.y, body.inertia.y),
        divide_or_zero(local_torque.z, body.inertia.z),
    );
    let angular_acceleration = body.orientation * local_angular_acceleration;

    let h = dt / (SUBSTEPS as f32);

    for _ in 0..SUBSTEPS {
        let velocity = body.linear_velocity;
        let angular_velocity = body.angular_velocity;

        body.position += velocity * h;
        body.linear_velocity = velocity + acceleration * h;

        let spin = Quat::from_xyzw(
            angular_velocity.x,
            angular_velocity.y,
            angular_velocity.z,
            0.0,
        );
        let derivative = spin * body.orientation * 0.5;
        body.orientation = normalise_or_identity(body.orientation + derivative * h);
        body.angular_velocity = angular_velocity + angular_acceleration * h;
    }
}

/// Component-wise division that treats a zero denominator as a zero result.
fn divide_or_zero(numerator: f32, denominator: f32) -> f32 {
    if denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    }
}

/// Normalises a quaternion, falling back to the identity for a degenerate one.
///
/// A body spun hard enough for one Euler sub-step to cancel the quaternion out
/// would otherwise produce `NaN` and destroy the whole world snapshot.
fn normalise_or_identity(q: Quat) -> Quat {
    let length_squared = q.length_squared();
    if length_squared > 0.0 && length_squared.is_finite() {
        q / length_squared.sqrt()
    } else {
        Quat::IDENTITY
    }
}

/// One frame: clamp, evaluate once, integrate three sub-steps.
///
/// Returns what the force evaluation produced, for tests and debug views. The
/// accumulators are cleared at the top of the frame and left populated on exit, so
/// after `step` returns they still hold what this frame applied.
///
/// `state.body.mass` is what the integrator divides by and
/// `handling.physical.mass` is what the hover spring multiplies by. They are the
/// same quantity stored twice, and keeping them equal is the caller's job - the
/// integration layer in `oag-gameplay` sets both from the parameter set. This
/// function does not silently copy one over the other, because a mismatch is a
/// bug worth seeing rather than papering over.
///
/// # The wall constraint runs last
///
/// After the body has moved, [`crate::wall::resolve`] pushes the hull back out of
/// any non-hoverable surface it ended up inside and removes the inward velocity.
/// It is a projection rather than a force term for the stability reason above: a
/// contact spring stiff enough to stop a ship within one frame is precisely the
/// stiffness a force evaluated once and held across three sub-steps cannot carry.
///
/// The sweep it does starts from the position **after** force evaluation, not
/// before, because the hover path's penetration escape is a deliberate teleport
/// and sweeping across it would invent a contact out of it.
///
/// # The energy pool is charged after the contacts are resolved
///
/// [`crate::damage`] turns the frame's contact impulses into a subtraction from
/// [`ShipState::shield`]. It runs here rather than in a caller because the input
/// is `WallResponse::impulse_sum`, which is derived state a caller is told not to
/// read - and because a pool written outside this crate is a simulation field the
/// determinism gate cannot see. The original does the same thing at the same
/// point, in `FUN_088418e0`'s contact loop, which walks the ring the physics step
/// just filled.
///
/// [`Evaluated::shield`] carries what it did; a caller that ignores it loses a
/// sound cue and nothing else.
pub fn step<R: Raycaster + ?Sized>(
    state: &mut ShipState,
    controls: &ShipControls,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    measured_dt: f32,
) -> Evaluated {
    let dt = clamp_dt(measured_dt);

    state.body.clear_accumulators();
    let mut evaluated = forces::evaluate(state, controls, handling, env, raycaster, dt);

    let before_integration = state.body.position;
    integrate(&mut state.body, dt);

    // **Ours, and the only place this crate adds a mechanism rather than
    // reproducing one.** The recovered contact test casts downward only, so a
    // hull that crosses a surface between two ticks is on the wrong side of it
    // for ever after and no probe can find its way back. See [`hover::sweep`].
    //
    // Unconditional, and the first attempt at this gated it on `state.grounded`
    // being zero, which is exactly wrong: `grounded` is set by `evaluate`
    // *before* the integrator moves the craft, so on the one tick that matters -
    // the tick the hull crosses the surface - it still reads the contact the
    // craft had on the way in. The gate skipped the only case it was for.
    if let Some((correction, velocity)) = hover::sweep(state, env, raycaster, before_integration) {
        state.body.position += correction;
        state.body.linear_velocity = velocity;
    }

    evaluated.wall = wall::resolve(state, handling, env, raycaster, before_integration);
    // `Ship_ApplyCollisionImpulse`'s port: consumes `state.pending_impulse` every
    // tick, same as the original consumes `entity->0x4c + 0x110`. Currently a
    // guaranteed no-op - nothing in this crate writes `pending_impulse` yet, both
    // producers (`Weapon_PostBlastImpulse_q` and its unnamed sibling) are still
    // unported - but it has to run every tick regardless, the way the original's
    // `FUN_0883f540` calls it unconditionally per ship. See
    // `crate::wall::apply_pending_impulse`.
    wall::apply_pending_impulse(state);
    evaluated.shield = crate::damage::apply_contact(
        state,
        &handling.dimensions,
        &evaluated.wall,
        env.damage_rules,
    );
    crate::damage::regenerate(state, &handling.dimensions, env.damage_rules, dt);
    crate::damage::advance_state(state, dt);
    // After `evaluate`, so the tick a pickup is fired on gets the boost. See
    // `crate::engine::advance_turbo`.
    crate::engine::advance_turbo(state, dt);
    // And after `apply_contact` above, so the tick a Shield is fired on is
    // protected rather than skipped - the same argument, one line later because
    // what reads this timer is the damage path rather than the force law.
    crate::damage::advance_shield_pickup(state, dt);

    evaluated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_delta_is_clamped_to_what_the_original_integrates() {
        assert_eq!(clamp_dt(-1.0), 0.0);
        assert_eq!(clamp_dt(0.0), 0.0);
        assert_eq!(clamp_dt(1.0 / 60.0), 1.0 / 60.0);
        assert_eq!(clamp_dt(10.0), MAX_DT);
    }

    #[test]
    fn a_sixtieth_of_a_second_is_never_clamped() {
        const { assert!(1.0f32 / 60.0 < MAX_DT) };
        assert_eq!(clamp_dt(1.0 / 60.0), 1.0 / 60.0);
    }

    /// Explicit Euler advances position by the *old* velocity, so after one frame
    /// from rest under a constant acceleration the position has not moved yet
    /// while the velocity has. Semi-implicit Euler would have moved it.
    #[test]
    fn the_first_frame_from_rest_changes_velocity_before_position() {
        let mut body = Body {
            force: Vec3::new(0.0, 3.0, 0.0),
            ..Body::default()
        };
        integrate(&mut body, 3.0);

        // Three sub-steps of h = 1.0 at an acceleration of 3. Position advances by
        // the velocity the sub-step *started* with, so by 0, then 3, then 6: nine
        // units. Semi-implicit Euler would advance by 3, 6, then 9 and finish at
        // eighteen, which is what this test exists to exclude.
        assert_eq!(body.linear_velocity, Vec3::new(0.0, 9.0, 0.0));
        assert_eq!(body.position, Vec3::new(0.0, 9.0, 0.0));
    }

    #[test]
    fn a_zero_delta_is_a_no_op() {
        let before = Body {
            linear_velocity: Vec3::new(1.0, 2.0, 3.0),
            angular_velocity: Vec3::new(0.1, 0.2, 0.3),
            force: Vec3::new(4.0, 5.0, 6.0),
            torque: Vec3::new(0.4, 0.5, 0.6),
            ..Body::default()
        };
        let mut after = before;
        integrate(&mut after, 0.0);
        assert_eq!(after, before);
    }

    #[test]
    fn a_negative_delta_is_a_no_op() {
        let before = Body {
            force: Vec3::new(0.0, 1.0, 0.0),
            ..Body::default()
        };
        let mut after = before;
        integrate(&mut after, -0.5);
        assert_eq!(after, before);
    }

    #[test]
    fn a_massless_body_does_not_accelerate_to_infinity() {
        let mut body = Body {
            mass: 0.0,
            force: Vec3::new(0.0, 1.0, 0.0),
            ..Body::default()
        };
        integrate(&mut body, 1.0 / 60.0);
        assert_eq!(body.linear_velocity, Vec3::ZERO);
        assert!(body.position.is_finite());
    }

    #[test]
    fn a_body_with_no_inertia_does_not_spin_up_to_infinity() {
        let mut body = Body {
            inertia: Vec3::ZERO,
            torque: Vec3::new(0.0, 1.0, 0.0),
            ..Body::default()
        };
        integrate(&mut body, 1.0 / 60.0);
        assert_eq!(body.angular_velocity, Vec3::ZERO);
        assert_eq!(body.orientation, Quat::IDENTITY);
    }

    #[test]
    fn the_orientation_stays_a_unit_quaternion_over_many_frames() {
        let mut body = Body {
            angular_velocity: Vec3::new(3.0, -7.0, 11.0),
            ..Body::default()
        };
        for _ in 0..6000 {
            integrate(&mut body, 1.0 / 60.0);
        }
        assert!(
            (body.orientation.length() - 1.0).abs() < 1e-4,
            "length drifted to {}",
            body.orientation.length()
        );
    }

    #[test]
    fn a_torque_free_body_keeps_its_angular_velocity() {
        let mut body = Body {
            angular_velocity: Vec3::new(0.0, 1.0, 0.0),
            ..Body::default()
        };
        integrate(&mut body, 1.0 / 60.0);
        assert_eq!(body.angular_velocity, Vec3::new(0.0, 1.0, 0.0));
    }

    /// The sub-step count is fixed at three, so an integration of `dt` is not the
    /// same as three integrations of `dt/3` - the latter is nine sub-steps. Pinned
    /// because the difference is the whole reason the count matters.
    #[test]
    fn three_sub_steps_of_a_third_are_not_the_same_as_nine() {
        let start = Body {
            force: Vec3::new(0.0, 1.0, 0.0),
            ..Body::default()
        };

        let mut one_frame = start;
        integrate(&mut one_frame, 0.03);

        let mut three_frames = start;
        for _ in 0..3 {
            integrate(&mut three_frames, 0.01);
        }

        assert_ne!(one_frame.position, three_frames.position);
    }
}
