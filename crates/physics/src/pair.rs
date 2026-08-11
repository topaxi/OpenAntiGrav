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
/// # The detection is ours. The original's is a stub.
///
/// `Collision_DispatchPair` (`0x08816eac`) sends a box proxy against a box proxy
/// to `0x08815ccc`, and `0x08815ccc` is **two instructions - `jr ra; nop`**. It
/// reports nothing. Craft are box proxies, so **craft-to-craft contact never
/// comes out of Pulse's narrowphase**; whatever reaches
/// `Body_ResolveContactPair` for a pair of craft is somewhere else and has not
/// been found. See
/// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
///
/// So this is not an approximation of a recovered test - there is nothing to
/// approximate. It is an **oriented box against an oriented box**, by the
/// separating-axis theorem over the usual fifteen axes, using the hull's own
/// `<Misc width height length>`. A box is what the shape kind says a craft is,
/// which is the whole of the argument for it.
///
/// It replaced a sphere of half the hull's diagonal, which was **far too big**:
/// on a 4 x 2 x 8 hull that sphere reaches 4.58 units where the flank is 2 away,
/// so craft shoved each other while visibly apart.
///
/// Returns the normal (pointing from `b` toward `a`), the contact point, and the
/// penetration depth.
#[must_use]
pub fn overlap(
    a: &Body,
    a_size: &Dimensions,
    b: &Body,
    b_size: &Dimensions,
) -> Option<(Vec3, Vec3, f32)> {
    let between = a.position - b.position;
    // A sphere reject first, so the fifteen-axis test only runs on candidates.
    // Half the diagonal bounds the box, so this never drops a real overlap.
    if between.length_squared() >= (hull_radius(a_size) + hull_radius(b_size)).powi(2) {
        return None;
    }

    let a_axes = axes(a);
    let b_axes = axes(b);
    let a_half = half_extents(a_size);
    let b_half = half_extents(b_size);

    let mut best_depth = f32::INFINITY;
    let mut best_axis = Vec3::ZERO;

    // The fifteen: three faces each, then the nine edge-edge cross products.
    // Built in a fixed order and compared with a strict `<`, so the axis chosen
    // cannot depend on anything but the geometry.
    let mut candidates = [Vec3::ZERO; 15];
    candidates[..3].copy_from_slice(&a_axes);
    candidates[3..6].copy_from_slice(&b_axes);
    let mut at = 6;
    for a_axis in a_axes {
        for b_axis in b_axes {
            candidates[at] = a_axis.cross(b_axis);
            at += 1;
        }
    }

    for axis in candidates {
        // A near-zero cross product means the two edges are parallel and the
        // axis carries no information; the face axes already cover that case.
        let length = axis.length();
        if length <= 1.0e-4 {
            continue;
        }
        let axis = axis / length;
        let reach = project(&a_axes, a_half, axis) + project(&b_axes, b_half, axis);
        let distance = between.dot(axis).abs();
        let depth = reach - distance;
        if depth <= 0.0 {
            return None;
        }
        if depth < best_depth {
            best_depth = depth;
            // Oriented from `b` toward `a`, which is what the caller expects.
            best_axis = if between.dot(axis) < 0.0 { -axis } else { axis };
        }
    }

    if !best_depth.is_finite() {
        return None;
    }
    // On `a`'s surface along the normal, then half the overlap back inside, so
    // the point sits in the middle of the overlapping slab rather than on either
    // hull.
    let support = project(&a_axes, a_half, best_axis);
    let point = a.position - best_axis * (support - best_depth * 0.5);
    Some((best_axis, point, best_depth))
}

/// The hull's own axes in world space: right, up, forward.
fn axes(body: &Body) -> [Vec3; 3] {
    [body.right(), body.up(), body.forward()]
}

/// How much of the authored hull box a craft actually collides with.
///
/// **Ours, and a feel knob rather than a measurement.** There is no original to
/// match here - `0x08815ccc` reports nothing - so nothing sets this but play.
///
/// The reason it is below `1.0`: `<Misc width height length>` is a **bounding**
/// box, sized to the hull's widest point. Measured on a real Pulse craft it is
/// `5.5 x 3.5 x 13`, whose half-length of `6.5` lands within a whisker of the
/// `6.45` bounding radius `oag-view` reports for the shipped Feisar mesh - so the
/// box bounds the model rather than tracing it. A Wipeout hull tapers hard toward
/// the nose, so a full-size box has the craft colliding along its whole length at
/// the width of its widest point, and two craft passing bump where the models
/// visibly miss. That was reported from play twice.
///
/// Applied to every axis rather than only to width, because the taper is in
/// plan *and* in profile.
pub const HULL_SCALE: f32 = 0.75;

/// `<Misc width height length>` as half-extents on those axes, scaled by
/// [`HULL_SCALE`].
fn half_extents(size: &Dimensions) -> Vec3 {
    Vec3::new(size.width, size.height, size.length) * (0.5 * HULL_SCALE)
}

/// How far a box reaches along `axis` from its own centre.
fn project(axes: &[Vec3; 3], half: Vec3, axis: Vec3) -> f32 {
    (half.x * axes[0].dot(axis)).abs()
        + (half.y * axes[1].dot(axis)).abs()
        + (half.z * axes[2].dot(axis)).abs()
}

/// Half the hull's diagonal: the sphere the broadphase rejects with.
#[must_use]
pub fn hull_radius(size: &Dimensions) -> f32 {
    half_extents(size).length()
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

    /// The regression the play reports produced, twice: craft shoved each other
    /// while visibly apart.
    ///
    /// Two hulls of width `w` touch flank to flank at `w * HULL_SCALE` and not
    /// before - written against the constant rather than a literal, so retuning
    /// the feel moves one number and not a test's meaning.
    #[test]
    fn side_by_side_craft_touch_at_the_scaled_flank_and_not_before() {
        let b = craft(Vec3::ZERO, Vec3::ZERO);
        let touch_at = hull().width * HULL_SCALE;

        let just_clear = craft(Vec3::new(touch_at * 1.05, 0.0, 0.0), Vec3::ZERO);
        assert!(
            overlap(&just_clear.body, &hull(), &b.body, &hull()).is_none(),
            "craft just outside {touch_at} are not touching"
        );
        let just_touching = craft(Vec3::new(touch_at * 0.95, 0.0, 0.0), Vec3::ZERO);
        assert!(
            overlap(&just_touching.body, &hull(), &b.body, &hull()).is_some(),
            "craft just inside {touch_at} are touching"
        );
    }

    /// The scale is a reduction, or it is not doing its job. Asserted through the
    /// geometry rather than on the constant itself, which the compiler would fold
    /// away.
    #[test]
    fn the_hull_scale_shrinks_the_box() {
        let b = craft(Vec3::ZERO, Vec3::ZERO);
        // Just inside the authored flank, outside the scaled one.
        let between = craft(Vec3::new(hull().width * 0.95, 0.0, 0.0), Vec3::ZERO);
        assert!(
            overlap(&between.body, &hull(), &b.body, &hull()).is_none(),
            "the authored box would collide here and the scaled one must not"
        );
    }

    /// A hull is longer than it is wide, and the test has to know it. Nose to
    /// tail they touch at 8; side by side at 4. A sphere cannot tell the two
    /// apart, which is what made it feel wrong.
    #[test]
    fn the_hull_is_longer_than_it_is_wide() {
        let b = craft(Vec3::ZERO, Vec3::ZERO);
        // Halfway between the scaled width and the scaled length: touching
        // nose to tail, clear abreast. A sphere cannot tell the two apart, which
        // is what made it feel wrong.
        let gap = (hull().width + hull().length) * 0.5 * HULL_SCALE;
        let nose_to_tail = craft(Vec3::new(0.0, 0.0, gap), Vec3::ZERO);
        assert!(overlap(&nose_to_tail.body, &hull(), &b.body, &hull()).is_some());
        let abreast = craft(Vec3::new(gap, 0.0, 0.0), Vec3::ZERO);
        assert!(overlap(&abreast.body, &hull(), &b.body, &hull()).is_none());
    }

    /// Turn one craft ninety degrees and its footprint changes with it, which is
    /// the whole point of testing oriented boxes rather than axis-aligned ones.
    #[test]
    fn rotating_a_hull_rotates_its_footprint() {
        let b = craft(Vec3::ZERO, Vec3::ZERO);
        // Between the two touch distances: outside the combined scaled *width*
        // (nose-on), inside the combined scaled width-plus-length (broadside).
        let nose_on = hull().width * HULL_SCALE;
        let broadside = (hull().width + hull().length) * 0.5 * HULL_SCALE;
        let gap = (nose_on + broadside) * 0.5;
        let mut across = craft(Vec3::new(gap, 0.0, 0.0), Vec3::ZERO);
        assert!(
            overlap(&across.body, &hull(), &b.body, &hull()).is_none(),
            "nose-on at {gap} apart: clear"
        );
        // Now broadside: the 8-unit length lies across the gap.
        across.body.orientation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        assert!(
            overlap(&across.body, &hull(), &b.body, &hull()).is_some(),
            "broadside at {gap} apart, with the hull's length across it: touching"
        );
    }

    #[test]
    fn a_head_on_pair_is_pushed_apart() {
        // Overlapping across the width.
        let mut a = craft(
            Vec3::new(hull().width * HULL_SCALE * 0.75, 0.0, 0.0),
            Vec3::new(-10.0, 0.0, 0.0),
        );
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
        assert!(a.body.position.x > hull().width * HULL_SCALE * 0.75);
        assert!(b.body.position.x < 0.0);
    }

    /// The gate: a pair already coming apart is left alone, which is the one
    /// thing the two-body path does that the one-body path does not.
    #[test]
    fn a_pair_already_separating_is_left_alone() {
        let mut a = craft(
            Vec3::new(hull().width * HULL_SCALE * 0.75, 0.0, 0.0),
            Vec3::new(50.0, 0.0, 0.0),
        );
        let mut b = craft(Vec3::ZERO, Vec3::new(-50.0, 0.0, 0.0));
        assert!(resolve(&mut a, &hull(), &mut b, &hull()).is_none());
    }

    /// Momentum is conserved by the impulse half: equal and opposite, so the sum
    /// of `m*v` does not move. The positional split is a separate correction and
    /// is deliberately not momentum-conserving - it is a position fix, and the
    /// original's is symmetric rather than mass-weighted.
    #[test]
    fn the_impulse_conserves_momentum() {
        let mut a = craft(
            Vec3::new(hull().width * HULL_SCALE * 0.75, 0.0, 0.0),
            Vec3::new(-10.0, 0.0, 0.0),
        );
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
        let start_a = craft(
            Vec3::new(hull().width * HULL_SCALE * 0.75, 0.0, 0.0),
            Vec3::new(-10.0, 0.0, 3.0),
        );
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
        let mut a = craft(
            Vec3::new(hull().width * HULL_SCALE * 0.7, 0.0, 2.0),
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
