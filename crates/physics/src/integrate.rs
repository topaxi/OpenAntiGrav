//! The integrator: three explicit Euler sub-steps of `dt/3`.
//!
//! ```text
//! dt = clamp(measured_dt, 0, 0.06666)
//! evaluate all forces once at full dt
//! integrate 3 explicit Euler sub-steps of dt/3
//! ```
//!
//! From `docs/physics/README.md`, which resolved the question
//! `docs/architecture/adr/0007-fixed-timestep-vs-original.md` left open: the craft path does
//! **not** sub-step at 1/60. The count is fixed at three and the size is `dt/3`, so the
//! original's ship handling is frame-rate dependent; at this project's fixed 60 Hz it
//! degenerates to three sub-steps of 1/180, which is what the original does at its target
//! frame rate.
//!
//! # The loop lives here and nowhere else
//!
//! [`crate::forces::evaluate`] has no loop; this module has the loop and cannot evaluate a
//! force, as [`integrate`] is handed a [`Body`], not a ship, parameter set or raycaster.
//! Re-evaluating forces per sub-step is the obvious "improvement" and wrong; the split makes it
//! unavailable rather than merely discouraged.
//!
//! # That split decides the stability margin
//!
//! The acceleration is computed once from the frame's starting state and held, so a stiff
//! term's stability is governed by the **frame** `H = dt`, not the sub-step `H/3`: sub-stepping
//! refines where the state lands, it does not refresh the force. For `a = -k*theta -
//! c*theta_dot`:
//!
//! ```text
//! det = (1 - k*H^2/3)(1 - c*H) + k*H^2 - c*k*H^3/3      stable while det <= 1
//!     <=>  c >= (2/3) * k * H   <=>   H <= 3c / (2k)
//! ```
//!
//! a **stricter** bound than the `h <= c/k` of a force per sub-step. For the alignment torque
//! that is an 11 % overshoot against a 2.22x one (worked numbers and the confirming
//! measurement: `docs/physics/README.md`, "Alignment gain: the measurements behind the
//! numbers", and [`crate::hover::ALIGNMENT_GAIN`]). Anyone re-deriving a stability margin for
//! a term in this engine starts here.

use oag_core::math::{Quat, Vec3};

use crate::collide::Raycaster;
use crate::forces::{self, Environment, Evaluated};
use crate::hover;
use crate::params::Handling;
use crate::ship::{Body, MAX_DT, SUBSTEPS, ShipControls, ShipState};
use crate::wall;

/// Clamps a measured frame delta to what the original will integrate:
/// `clamp(measured_dt, 0, MAX_DT)`. A `NaN` stays `NaN` on purpose: a broken clock should
/// show as a broken simulation, not a ship that quietly stops.
#[must_use]
pub fn clamp_dt(measured_dt: f32) -> f32 {
    measured_dt.clamp(0.0, MAX_DT)
}

/// Integrates a body over `dt` in [`SUBSTEPS`] explicit Euler sub-steps.
///
/// Force and torque become accelerations **once**, from the state at the start of the frame,
/// and every sub-step reuses them. The accumulators are not cleared ([`step`]).
///
/// # Explicit, not semi-implicit
///
/// Each sub-step reads start-of-sub-step values for *both* updates: position advances by the
/// old velocity, then velocity by the acceleration. `v += a*h` then `p += v*h` is
/// semi-implicit (symplectic) Euler, a different integrator with different stability, which
/// one writes by reflex and the page does not specify.
///
/// # The angular-velocity sign convention, a whole-crate contract
///
/// This uses the **textbook** derivative, `q' = (omega as a pure quaternion) * q / 2`
/// (`e' = omega x e` on the basis rows). **The original does not**: `Body_Integrate`'s skew
/// matrix, with its base constants confirmed zero, builds `d(row0) = h * (-z * row1 + y *
/// row2)`, i.e. `e' = e x omega`. The engine's angular velocity is the negation of this one,
/// `w_game = -w_physics`, consistently everywhere.
///
/// That explains two apparent errors: `Body_AddForceAtPoint` computing torque as `F x r`, and
/// the alignment being `-400 * cross(up, avgNormal)`; under `w_game = -w_physics` both equal
/// the textbook `r x F` and `+400 * cross(up, n)`. Nothing in the original is inverted. The
/// two candidates hunted (a pre-negated force at the call site, the matrix at `body+0xc0`) are
/// ruled out, the latter because it is the basis's rotation inverse and a determinant of `+1`
/// cannot flip a sign. **This is not the handedness question**, separately settled:
/// `cross(row0, row1) = row2` on the PSP measurement, and the PS2's orthonormaliser rebuilds
/// `row0 = row1 x row2`, the same statement.
///
/// # The rule, and what breaks if it is half-applied
///
/// Exactly **one** flip, applied consistently: every torque expression inherited from the
/// disassembly is negated once on the way in. Today three places:
///
/// - [`crate::ship::Body::add_force_at_point`], `r x F` against a read `F x r`.
/// - [`crate::hover::ALIGNMENT_GAIN`], `+400` against a read `-400`.
/// - [`crate::passive::WEATHERVANE_GROUND`], `+0.1` against a read `-0.1`.
///
/// Each carries its own sound local argument, but they are **one convention**, not three
/// coincidences, and "correcting" any one back to the literal gives **unconditional
/// divergence**, not the mild instability of a mistuned magnitude.
/// `crate::ship::tests::a_force_at_a_point_makes_torque_the_textbook_way_round` pins the
/// first. Angular damping is excluded: `tau = -c * w` is a negative multiple of `w` under
/// either convention, so [`crate::passive::angular_damping`] is right and must not be flipped.
///
/// # Angular integration
///
/// Torque is world space and this crate treats the inertia tensor as a body-space diagonal, so
/// the torque is rotated into the body, divided component-wise and rotated back once, using the
/// frame's starting orientation. A zero inertia component contributes zero acceleration, not
/// infinity, so a zeroed parameter set is safe.
///
/// **The original's diagonal is fixed in world axes and this crate declines to follow it:
/// chosen, not measured.** `Body_Integrate`'s `R I R^T` is only the trip in and out of the frame
/// its fields are stored in, so the engine multiplies by a world-axis constant, which explains
/// `100.00 %` of the recorded momentum column on three captures. Not adopted: the gap it was
/// proposed to close belongs to a different identity with no tensor in it, and it is a
/// four-site convention change ([`crate::wall`]'s contact denominator and angular response,
/// here, [`crate::forces::YAW_INVERSE_INERTIA`]) for no measured payoff, moving the committed
/// reference hash as early as tick 600 of `Corridor`. **A real divergence, carried
/// knowingly**: at mild bank off a magstrip the original drives attitude through this path with
/// a world-axis tensor. Measurement, decision and the condition to revisit:
/// `docs/physics/cornering-ground-truth.md`, "The tensor is a world-axis diagonal". What is here
/// is not the textbook treatment either: no gyroscopic `omega x (I omega)` term, and the
/// rotation is evaluated once per frame.
///
/// The orientation advances by `q + 0.5 * (omega as a pure quaternion) * q * h` and is
/// **renormalised every sub-step**: explicit Euler grows a quaternion's norm, and drifting for
/// three sub-steps would put the error into the next sub-step's `up` axis. `sin`/`cos` of the
/// angle would be more accurate but transcendentals resolve to the platform's libm, a
/// portability risk (`docs/architecture/determinism.md`).
pub fn integrate(body: &mut Body, dt: f32) {
    // A `NaN` or infinite delta is a no-op, not a body full of `NaN`. `clamp_dt` preserves a
    // `NaN` so a broken clock is visible at the boundary; past this point it would poison the
    // snapshot with no way back to its cause.
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

/// Normalises a quaternion, falling back to the identity for a degenerate one that a hard
/// Euler sub-step spin would otherwise turn into `NaN` for the whole snapshot.
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
/// Returns what the force evaluation produced, for tests and debug views. The accumulators
/// are cleared at the top of the frame and left populated on exit.
///
/// `state.body.mass` is what the integrator divides by and `handling.physical.mass` what the
/// hover spring multiplies by: one quantity stored twice, kept equal by the caller (the
/// `oag-gameplay` integration layer sets both). Nothing copies one over the other, because a
/// mismatch is a bug worth seeing.
///
/// # The wall constraint runs last
///
/// After the body moves, [`crate::wall::resolve`] pushes the hull out of any non-hoverable
/// surface and removes the inward velocity. A projection, not a force term: a contact spring
/// stiff enough to stop a ship in one frame is the stiffness a force held across three
/// sub-steps cannot carry. The sweep starts from the position **after** force evaluation,
/// because the hover path's penetration escape is a deliberate teleport and sweeping across it
/// would invent a contact.
///
/// # The energy pool is charged after the contacts are resolved
///
/// [`crate::damage`] turns the frame's contact impulses into a subtraction from
/// [`ShipState::shield`]. It runs here because its input, `WallResponse::impulse_sum`, is
/// derived state a caller is told not to read, and a pool written outside this crate is a
/// field the determinism gate cannot see. The original does the same at the same point
/// (`FUN_088418e0`'s contact loop over the ring the step just filled). [`Evaluated::shield`]
/// carries what it did; ignoring it loses a sound cue only.
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
    wall::pre_integration_clip(state, handling, env, raycaster, dt);
    let mut evaluated = forces::evaluate(state, controls, handling, env, raycaster, dt);

    let before_integration = state.body.position;
    integrate(&mut state.body, dt);

    // **Ours, the only mechanism this crate adds rather than reproduces.** The recovered
    // contact test casts downward only, so a hull that crosses a surface between ticks stays
    // on the wrong side and no probe can find its way back ([`hover::sweep`]).
    //
    // Unconditional. Gating on `state.grounded == 0` was exactly wrong: `evaluate` sets it
    // *before* the integrator moves the craft, so on the tick the hull crosses the surface it
    // still reads the contact from the way in.
    if let Some((correction, velocity)) = hover::sweep(state, env, raycaster, before_integration) {
        state.body.position += correction;
        state.body.linear_velocity = velocity;
    }

    evaluated.wall = wall::resolve(state, handling, env, raycaster);
    // `Ship_ApplyCollisionImpulse`'s port: consumes `state.pending_impulse` every tick, as the
    // original does for `entity->0x4c + 0x110` (`FUN_0883f540`, unconditionally per ship).
    // Currently a guaranteed no-op, as no producer is ported; see
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
    // After `evaluate`, so the tick a pickup is fired on gets the boost (`advance_turbo`).
    crate::engine::advance_turbo(state, dt);
    // After `apply_contact`, so the tick a Shield is fired on is protected, not skipped: same
    // argument, as the damage path reads this timer.
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
