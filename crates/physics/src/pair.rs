//! Craft against craft: the two-body contact response.
//!
//! `Body_ResolveContactPair` (`0x0884ef30`) reimplemented, at confidence **80**
//! for the response and **nothing at all** for the shape - see
//! [`overlap`]. The recovery is on
//! `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
//!
//! # A craft bounces off another craft differently from how it bounces off a wall
//!
//! The one-body path in [`crate::wall`] takes its restitution from the body -
//! `0.4` on a ship - and folds friction into the same impulse. The pair resolver
//! does neither: restitution is a **code literal** giving `e = 0.1`, and there is
//! no tangential term at all. Both are what the original does, in the same
//! build, and the split is deliberate rather than an oversight here.
//!
//! It also has a separating test the one-body path does not: a pair already
//! moving apart faster than [`SEPARATING_LIMIT`] is left alone.

use oag_core::math::Vec3;

use crate::params::Dimensions;
use crate::ship::{Body, ShipState};

/// `1 + e` from the resolver's hardcoded `-1.1` numerator, so `e = 0.1`.
///
/// **Not the same as a craft against the track**, which uses the per-body `0.4`
/// the ship constructor writes. Confidence 80: a literal in the decompiled
/// arithmetic, and the PS2's own pair resolver carries the same one.
pub const PAIR_RESTITUTION_PLUS_ONE: f32 = 1.1;

/// Relative normal speed above which a pair is treated as already separating.
///
/// The resolver's `((vA . n) - (vB . n)) - 0.5 <= 0` gate, a code literal.
pub const SEPARATING_LIMIT: f32 = 0.5;

/// Fraction of the overlap each body is pushed along the normal.
///
/// A quarter each way, the same for both bodies whatever their masses - which is
/// what the original does, and is why it is a constant here rather than a mass
/// split.
pub const POSITIONAL_SPLIT: f32 = 0.25;

/// A resolved contact between two craft.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PairContact {
    /// The normal, pointing from B toward A.
    pub normal: Vec3,
    /// How far the two hulls interpenetrate.
    pub depth: f32,
    /// The magnitude of the impulse that was applied along the normal.
    pub impulse: f32,
}

/// Whether two craft are touching, and where.
///
/// # This shape is ours, and it is the weakest part of the file
///
/// The original tests hull against hull through a **box against box**
/// narrowphase (`0x08815ccc`), which is unread - so what shape it uses, and how
/// it picks a contact point and normal from an overlap, is not known. This uses
/// a **sphere** whose radius is half the hull's diagonal from
/// [`Dimensions`], centred on the body, and takes the contact point as the
/// midpoint between the two centres.
///
/// A sphere is round and a craft is not, so this is generous at the corners and
/// mean along the flanks - the exact places a race brushes. Recovering
/// `0x08815ccc` is what replaces it, and until then a doc note is the honest
/// thing rather than a claim.
#[must_use]
pub fn overlap(
    a: &Body,
    a_size: &Dimensions,
    b: &Body,
    b_size: &Dimensions,
) -> Option<(Vec3, Vec3, f32)> {
    let between = a.position - b.position;
    let distance = between.length();
    let reach = hull_radius(a_size) + hull_radius(b_size);
    if distance >= reach || distance <= f32::EPSILON {
        return None;
    }
    let normal = between / distance;
    let point = b.position + normal * (distance * 0.5);
    Some((normal, point, reach - distance))
}

/// Half the hull's diagonal: the sphere [`overlap`] tests with.
#[must_use]
pub fn hull_radius(size: &Dimensions) -> f32 {
    let half = Vec3::new(size.width, size.height, size.length) * 0.5;
    half.length()
}

/// Resolves one craft-to-craft contact, moving both bodies.
///
/// Returns `None` when the pair is not touching, or when the gate refuses it.
///
/// The order of the two arguments does not change the outcome: the impulse is
/// equal and opposite and the positional split is symmetric, so `resolve(a, b)`
/// and `resolve(b, a)` leave the same world. That is a property worth having,
/// because a pair loop has to visit each pair in *some* order and the simulation
/// must not depend on which.
pub fn resolve(
    a: &mut ShipState,
    a_size: &Dimensions,
    b: &mut ShipState,
    b_size: &Dimensions,
) -> Option<PairContact> {
    let (normal, point, depth) = overlap(&a.body, a_size, &b.body, b_size)?;

    let r_a = point - a.body.position;
    let r_b = point - b.body.position;
    let velocity_a = a.body.velocity_at(point);
    let velocity_b = b.body.velocity_at(point);

    // The gate. `vn` is positive while the two are separating, so a pair already
    // coming apart faster than the limit is left alone.
    let vn = (velocity_a - velocity_b).dot(normal);
    if vn - SEPARATING_LIMIT > 0.0 {
        return None;
    }

    // `n . ((I^-1 (r x n)) x r)` for each body, which is the pair's angular
    // compliance, plus both inverse masses. The one-body resolver has one mass
    // here; having two is what makes this the pair path.
    let angular = angular_term(&a.body, r_a, normal) + angular_term(&b.body, r_b, normal);
    let denominator = angular + inverse_mass(&a.body) + inverse_mass(&b.body);
    if denominator <= f32::EPSILON {
        return None;
    }

    let impulse = -(PAIR_RESTITUTION_PLUS_ONE * vn) / denominator;
    apply_at_point(&mut a.body, normal * impulse, point);
    apply_at_point(&mut b.body, -normal * impulse, point);

    // The positional split: a quarter of the overlap each way, mass-independent.
    a.body.position += normal * (POSITIONAL_SPLIT * depth);
    b.body.position -= normal * (POSITIONAL_SPLIT * depth);

    Some(PairContact {
        normal,
        depth,
        impulse,
    })
}

/// `v += J/m` and `omega += I^-1 (r x J)`, the second in the body's own frame.
///
/// [`Body::apply_impulse`] is the linear half only, so the angular half lives
/// here. `Body_ApplyImpulseAtPoint` (`0x0884d64c`) is the original's, and this
/// is the same two lines; what is *not* reproduced is the wall path's partial
/// angular share (see [`crate::wall`]), because the pair resolver applies its
/// impulse through the full function with no such scaling.
fn apply_at_point(body: &mut Body, impulse: Vec3, point: Vec3) {
    body.apply_impulse(impulse);

    let r = point - body.position;
    let local_r = body.orientation.inverse() * r;
    let local_j = body.orientation.inverse() * impulse;
    let torque = local_r.cross(local_j);
    let local_delta = Vec3::new(
        safe_divide(torque.x, body.inertia.x),
        safe_divide(torque.y, body.inertia.y),
        safe_divide(torque.z, body.inertia.z),
    );
    body.angular_velocity += body.orientation * local_delta;
}

fn inverse_mass(body: &Body) -> f32 {
    if body.mass <= f32::EPSILON {
        0.0
    } else {
        1.0 / body.mass
    }
}

/// `n . ((I^-1 (r x n)) x r)`, in the body's own frame.
fn angular_term(body: &Body, r: Vec3, normal: Vec3) -> f32 {
    let local_r = body.orientation.inverse() * r;
    let local_n = body.orientation.inverse() * normal;
    let torque = local_r.cross(local_n);
    let inertia = body.inertia;
    // Component-wise inverse of the diagonal tensor, guarding a zero axis rather
    // than dividing by it: `Handling::ZERO` really does have one.
    let angular = Vec3::new(
        safe_divide(torque.x, inertia.x),
        safe_divide(torque.y, inertia.y),
        safe_divide(torque.z, inertia.z),
    );
    angular.cross(local_r).dot(local_n)
}

fn safe_divide(numerator: f32, denominator: f32) -> f32 {
    if denominator.abs() <= f32::EPSILON {
        0.0
    } else {
        numerator / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::Quat;

    fn hull() -> Dimensions {
        Dimensions {
            width: 4.0,
            height: 2.0,
            length: 8.0,
            ..Dimensions::default()
        }
    }

    fn craft(position: Vec3, velocity: Vec3) -> ShipState {
        ShipState {
            body: Body {
                position,
                orientation: Quat::IDENTITY,
                linear_velocity: velocity,
                mass: 1.0,
                ..Body::default()
            },
            ..ShipState::default()
        }
    }

    #[test]
    fn craft_far_apart_do_not_touch() {
        let a = craft(Vec3::new(100.0, 0.0, 0.0), Vec3::ZERO);
        let b = craft(Vec3::ZERO, Vec3::ZERO);
        assert!(overlap(&a.body, &hull(), &b.body, &hull()).is_none());
    }

    #[test]
    fn a_head_on_pair_is_pushed_apart() {
        let reach = hull_radius(&hull()) * 2.0;
        let mut a = craft(Vec3::new(reach * 0.5, 0.0, 0.0), Vec3::new(-10.0, 0.0, 0.0));
        let mut b = craft(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let contact = resolve(&mut a, &hull(), &mut b, &hull()).expect("they overlap");

        assert!(contact.impulse > 0.0);
        // A was moving left and is pushed right; B the other way.
        assert!(
            a.body.linear_velocity.x > -10.0,
            "{:?}",
            a.body.linear_velocity
        );
        assert!(
            b.body.linear_velocity.x < 10.0,
            "{:?}",
            b.body.linear_velocity
        );
        // And they are further apart than they were.
        assert!(a.body.position.x > reach * 0.5);
        assert!(b.body.position.x < 0.0);
    }

    /// The gate: a pair already coming apart is left alone, which is the one
    /// thing the two-body path does that the one-body path does not.
    #[test]
    fn a_pair_already_separating_is_left_alone() {
        let reach = hull_radius(&hull()) * 2.0;
        let mut a = craft(Vec3::new(reach * 0.5, 0.0, 0.0), Vec3::new(50.0, 0.0, 0.0));
        let mut b = craft(Vec3::ZERO, Vec3::new(-50.0, 0.0, 0.0));
        assert!(resolve(&mut a, &hull(), &mut b, &hull()).is_none());
    }

    /// Momentum is conserved by the impulse half: equal and opposite, so the sum
    /// of `m*v` does not move. The positional split is a separate correction and
    /// is deliberately not momentum-conserving - it is a position fix, and the
    /// original's is symmetric rather than mass-weighted.
    #[test]
    fn the_impulse_conserves_momentum() {
        let reach = hull_radius(&hull()) * 2.0;
        let mut a = craft(Vec3::new(reach * 0.5, 0.0, 0.0), Vec3::new(-10.0, 0.0, 0.0));
        let mut b = craft(Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0));
        a.body.mass = 3.0;
        b.body.mass = 1.0;
        let before = a.body.linear_velocity * a.body.mass + b.body.linear_velocity * b.body.mass;
        resolve(&mut a, &hull(), &mut b, &hull()).expect("they overlap");
        let after = a.body.linear_velocity * a.body.mass + b.body.linear_velocity * b.body.mass;
        assert!(
            (after - before).length() < 1e-3,
            "before {before:?} after {after:?}"
        );
    }

    /// Which craft a pair loop happens to visit first must not change the world
    /// it leaves. See `docs/architecture/determinism.md`.
    #[test]
    fn the_argument_order_does_not_change_the_outcome() {
        let reach = hull_radius(&hull()) * 2.0;
        let start_a = craft(Vec3::new(reach * 0.5, 0.0, 0.0), Vec3::new(-10.0, 0.0, 3.0));
        let start_b = craft(Vec3::ZERO, Vec3::new(7.0, 0.0, -1.0));

        let (mut a1, mut b1) = (start_a, start_b);
        resolve(&mut a1, &hull(), &mut b1, &hull()).expect("they overlap");

        let (mut a2, mut b2) = (start_a, start_b);
        resolve(&mut b2, &hull(), &mut a2, &hull()).expect("they overlap");

        assert_eq!(a1.body.linear_velocity, a2.body.linear_velocity);
        assert_eq!(b1.body.linear_velocity, b2.body.linear_velocity);
        assert_eq!(a1.body.position, a2.body.position);
        assert_eq!(b1.body.position, b2.body.position);
    }

    /// A glancing hit spins the craft, or a race is bumper cars on rails.
    #[test]
    fn an_off_centre_hit_produces_spin() {
        let reach = hull_radius(&hull()) * 2.0;
        let mut a = craft(
            Vec3::new(reach * 0.6, 0.0, reach * 0.4),
            Vec3::new(-20.0, 0.0, 0.0),
        );
        let mut b = craft(Vec3::ZERO, Vec3::ZERO);
        resolve(&mut a, &hull(), &mut b, &hull()).expect("they overlap");
        assert!(
            b.body.angular_velocity.length() > 0.0,
            "an off-centre impulse must produce angular velocity"
        );
    }
}
