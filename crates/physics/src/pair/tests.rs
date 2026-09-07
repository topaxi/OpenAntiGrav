//! What the craft-against-craft contact response in [`super`] is asserted to do.
//!
//! Split out of `pair.rs` under the 200-line cap on inline `#[cfg(test)]`
//! modules; see `scripts/check-file-size.py`.

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

/// The contact point is the plain midpoint of the two bodies' positions,
/// whatever axis the SAT chose - `Collision_BoxAgainstBox` writes
/// `(colliderA.centre + colliderB.centre) * 0.5` and a collider's centre is
/// its body's own position, unconditionally.
#[test]
fn the_contact_point_is_the_midpoint_of_the_two_positions() {
    let a = craft(
        Vec3::new(hull().width * HULL_SCALE * 0.7, 0.0, 2.0),
        Vec3::ZERO,
    );
    let b = craft(Vec3::ZERO, Vec3::ZERO);
    let (_, point, _) = overlap(&a.body, &hull(), &b.body, &hull()).expect("they overlap");
    assert_eq!(point, (a.body.position + b.body.position) * 0.5);
}

/// Only the six face axes ever choose the normal - the nine edge-edge cross
/// products are reject-only in the original, so two boxes whose true
/// shallowest separating axis is an edge-edge cross product must still
/// resolve to a face normal, never that edge one.
///
/// This exact pose was found by sweeping orientations and offsets against
/// [`overlap_old`] (the pre-2026-09-07 all-fifteen-compete version, kept
/// around only long enough to diff against): at this pose the true minimum
/// SAT axis genuinely is one of the nine edge-edge cross products, not a
/// face axis of either hull, so a port that let all fifteen compete (as this
/// function used to) picks it - and this function must not.
#[test]
fn only_face_axes_ever_become_the_normal() {
    let long_hull = Dimensions {
        width: 2.0,
        height: 3.0,
        length: 20.0,
        ..Dimensions::default()
    };
    let mut a = craft(Vec3::new(1.5, 0.3, -1.5), Vec3::ZERO);
    a.body.orientation = Quat::from_rotation_y(0.9);
    let mut b = craft(Vec3::ZERO, Vec3::ZERO);
    b.body.orientation =
        Quat::from_rotation_y(34.0_f32.to_radians()) * Quat::from_rotation_z(19.0_f32.to_radians());

    let (normal, _, _) = overlap(&a.body, &long_hull, &b.body, &long_hull).expect("they overlap");

    // A face axis is `+-right`, `+-up` or `+-forward` of one of the two
    // hulls - each is a unit vector, so a match dots to (near) +-1. The true
    // minimum axis at this pose is an edge-edge cross product, which would
    // land well short of that.
    let is_face_axis = axes(&a.body)
        .into_iter()
        .chain(axes(&b.body))
        .any(|face| face.dot(normal).abs() > 0.999);
    assert!(is_face_axis, "normal {normal:?} is not a face axis");
}

/// Each hull's own "up" axis needs to beat *half* the reigning best depth to
/// become the normal; a plain SAT (compare every axis with a strict `<`)
/// would pick whichever axis is shallowest, full stop. Built so `up` is
/// genuinely shallower than `right` - a plain-`<` port would pick it - but not
/// shallower than half of it, so the original's own rule must keep `right`.
#[test]
fn the_up_axis_needs_to_beat_half_the_best_depth_to_win() {
    // width == length, both boxes identical and axis-aligned, offset only in
    // y: `right`'s depth and `forward`'s depth are then both `2 * half_width`
    // (9.0 below), and `up`'s is `2 * half_height - |dy|` (5.0) - shallower
    // than `right`, but not under half of it.
    let square_hull = Dimensions {
        width: 12.0,
        height: 8.0,
        length: 12.0,
        ..Dimensions::default()
    };
    let a = craft(Vec3::new(0.0, 1.0, 0.0), Vec3::ZERO);
    let b = craft(Vec3::ZERO, Vec3::ZERO);

    let (normal, _, depth) =
        overlap(&a.body, &square_hull, &b.body, &square_hull).expect("they overlap");
    assert_eq!(
        normal,
        Vec3::X,
        "up's depth (5.0) is shallower than right's (9.0) but does not clear \
         half of it (4.5), so a plain `<` SAT would wrongly pick up here"
    );
    assert_eq!(depth, 9.0);
}
