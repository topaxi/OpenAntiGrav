//! What the craft-against-craft contact response in [`super`] is asserted to do. Split out of
//! `pair.rs` under the 200-line cap on inline `#[cfg(test)]` modules (`scripts/check-file-size.py`).

use super::*;
use oag_core::math::{Mat3, Quat};

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

/// The regression the play reports produced, twice: craft shoved each other while visibly apart.
/// Two hulls of width `w` touch flank to flank at `w * HULL_SCALE` and not before, written against
/// the constant so retuning the feel moves one number, not a test's meaning.
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

/// Momentum is conserved by the impulse half (equal and opposite). The positional split is a
/// separate correction, deliberately not momentum-conserving: a position fix, symmetric rather
/// than mass-weighted, as in the original.
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

/// A glancing hit does not spin either craft: the original applies the pair impulse at each
/// body's own centre, so the lever arm is zero ([`respond`]). This once asserted the opposite,
/// which was `-267` degrees per second on a fast first contact.
#[test]
fn an_off_centre_hit_spins_neither_craft() {
    let mut a = craft(
        Vec3::new(hull().width * HULL_SCALE * 0.7, 0.0, 2.0),
        Vec3::new(-20.0, 0.0, 0.0),
    );
    let mut b = craft(Vec3::ZERO, Vec3::ZERO);
    let contact = resolve(&mut a, &hull(), &mut b, &hull()).expect("they overlap");
    assert!(contact.impulse > 0.0, "the hit was resolved: {contact:?}");
    assert_eq!(a.body.angular_velocity, Vec3::ZERO);
    assert_eq!(b.body.angular_velocity, Vec3::ZERO);
}

/// A body as PPSSPP read it: the original's basis rows `(up x forward, up, forward)` become this
/// crate's `orientation`, and its raw `body+0x150` triple becomes `angular_velocity`.
///
/// **Both are frame changes, and the second is the point of this fixture.** The rows map by
/// `right = -row0`, `orientation * Z = -row2`, so `R^T v` (the `vtfm4.q C000,E100,C200` the
/// resolvers apply to `+0x150`) is `orientation * (-v.x, v.y, -v.z)` here. `+0x150` holds the
/// rotation rate **negated and in body coordinates** ([`Body::velocity_at`]), so the world rate
/// stored is `-R^T(raw)`. Putting the raw triple into `angular_velocity` builds a `Body` the
/// simulation never produces, which is how `respond`'s sign error survived a green test.
fn captured(
    position: [f32; 3],
    velocity: [f32; 3],
    omega: [f32; 3],
    rows: [[f32; 3]; 3],
) -> ShipState {
    let row = |r: [f32; 3]| Vec3::from_array(r);
    let basis = Mat3::from_cols(-row(rows[0]), row(rows[1]), -row(rows[2]));
    let orientation = Quat::from_mat3(&basis);
    ShipState {
        body: Body {
            position: Vec3::from_array(position),
            orientation,
            linear_velocity: Vec3::from_array(velocity),
            angular_velocity: -(orientation * Vec3::new(-omega[0], omega[1], -omega[2])),
            mass: 1.0,
            inertia: Vec3::new(15.6, 21.6, 15.6),
            ..Body::default()
        },
        ..ShipState::default()
    }
}

/// Two contacts the original was caught resolving, 2026-09-10, PPSSPP v1.20.4, an eight-craft
/// SINGLE RACE on `pulse-psp-usa.chd`: every input read off both bodies and the contact record at
/// the `jal` into `Body_ApplyImpulseAtPoint`, and `j` from the impulse vector on the stack (`s4`);
/// recipe `scripts/psp-pair-capture.py`.
///
/// The first is a staged rear-end hit at `vn = -78.5`. The second is a craft tumbling at 84 rad/s
/// with its basis well off level, which separates the original's point velocity from the same
/// expression fed this crate's unmapped `angular_velocity` (`0.184` against `-0.085`) and the
/// world-diagonal denominator from the rotated one (`0.18440` against `0.18398`); tolerances sit
/// between those alternatives, not at float noise.
///
/// **The frame mapping in [`captured`] is what these numbers check.** `omega_ours == -R^T(raw)`
/// makes `omega_ours x r` identically `cross(r, R^T raw)` (the same products subtracted the other
/// way, bit for bit under IEEE-754), so the corrected `respond` reproduces the original's `j` from
/// its own bytes as the literal transcription did. If either moves, the mapping is wrong. It did
/// not: `43.1697235107` and `0.1843950450` before and after the correction, to the last digit.
#[test]
fn the_response_reproduces_two_contacts_the_original_was_caught_resolving() {
    let mut a = captured(
        [-220.81857, -50.145325, -581.0928],
        [-65.15091, 0.51419795, 60.969162],
        [0.003608909, -0.0074495664, -0.03581129],
        [
            [0.6698566, -0.050424457, 0.7407764],
            [0.037723415, 0.9987142, 0.03387033],
            [-0.74153185, 0.005256349, 0.6708975],
        ],
    );
    let mut b = captured(
        [-227.55688, -50.293213, -574.8641],
        [-7.265_033, -5.7860765, 6.084974],
        [-7.874016, -0.43041557, -51.043682],
        [
            [0.68709743, -0.36553824, 0.6279159],
            [0.2958006, 0.9300953, 0.21777053],
            [-0.66362506, 0.03610833, 0.7471925],
        ],
    );
    // The original's `+0x150` did not move across the call, bit for bit. Read
    // back against what `captured()` built rather than a literal triple: the
    // claim is "the pair impulse changed nothing", not "it holds these bytes".
    let spin_before = a.body.angular_velocity;
    let contact = respond(
        &mut a,
        &mut b,
        Vec3::new(0.74153167, -0.005256348, -0.6708973),
        Vec3::new(-224.18773, -50.21927, -577.97845),
        0.8196907,
    )
    .expect("a rear-end hit at 80 units/s resolves");
    assert!(
        (contact.impulse - 43.16971).abs() < 0.01,
        "rear-end j {} against the original's 43.16971",
        contact.impulse
    );
    assert_eq!(a.body.angular_velocity, spin_before);

    let mut a = captured(
        [-121.563614, -49.78744, -193.69652],
        [119.600815, -0.6304198, -0.16429144],
        [-0.006623617, 0.0322605, 0.025514752],
        [
            [0.004427044, -0.007387802, -0.9999629],
            [0.0046109925, 0.9999623, -0.0073673837],
            [0.9999796, -0.0045782058, 0.004460942],
        ],
    );
    let mut b = captured(
        [-119.225075, -49.789764, -193.641],
        [-99.24166, 1.3238686, -0.4299373],
        [56.963963, -0.11366701, 62.044594],
        [
            [-0.082835935, -0.3124286, 0.9463227],
            [0.29989827, 0.8977535, 0.32264495],
            [-0.950368, 0.31052715, 0.019330546],
        ],
    );
    let contact = respond(
        &mut a,
        &mut b,
        Vec3::new(0.004427044, -0.007387802, -0.9999629),
        Vec3::new(-120.39435, -49.788605, -193.66876),
        4.3339205,
    )
    .expect("the tumbling pair resolves");
    assert!(
        (contact.impulse - 0.18440).abs() < 2.0e-4,
        "tumbling j {} against the original's 0.18440",
        contact.impulse
    );
}

/// The contact point is the plain midpoint of the two bodies' positions whatever axis the SAT
/// chose: `Collision_BoxAgainstBox` writes `(colliderA.centre + colliderB.centre) * 0.5` and a
/// collider's centre is its body's position.
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

/// Only the six face axes ever choose the normal; the nine edge-edge cross products are
/// reject-only in the original, so two boxes whose true shallowest separating axis is an
/// edge-edge product must still resolve to a face normal.
///
/// This pose was found by sweeping orientations and offsets against [`overlap_old`] (the
/// pre-2026-09-07 all-fifteen-compete version, kept only to diff against): here the true minimum
/// SAT axis is an edge-edge cross product, which a port letting all fifteen compete picks and
/// this function must not.
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

    // A face axis is `+-right`, `+-up` or `+-forward` of one hull, each a unit vector, so a match
    // dots to (near) +-1. The true minimum axis here is an edge-edge product, well short of that.
    let is_face_axis = axes(&a.body)
        .into_iter()
        .chain(axes(&b.body))
        .any(|face| face.dot(normal).abs() > 0.999);
    assert!(is_face_axis, "normal {normal:?} is not a face axis");
}

/// Each hull's own "up" axis needs to beat *half* the reigning best depth to become the normal;
/// a plain strict-`<` SAT would pick the shallowest axis. Built so `up` is shallower than `right`
/// (a plain-`<` port picks it) but not shallower than half of it, so the original's rule keeps
/// `right`.
#[test]
fn the_up_axis_needs_to_beat_half_the_best_depth_to_win() {
    // width == length, both boxes identical and axis-aligned, offset only in y: `right`'s and
    // `forward`'s depths are `2 * half_width` (9.0 below) and `up`'s is `2 * half_height - |dy|`
    // (5.0): shallower than `right`, not under half of it.
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
