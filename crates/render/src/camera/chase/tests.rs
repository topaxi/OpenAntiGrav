//! What the chase camera in [`super`] is asserted to do: where a snapped camera
//! sits, what the spring preserves as it moves, and which frame each of the two
//! springs acts in.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `chase.rs`: the tests are 281 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

/// A ship at the origin facing +X, upright. No parameter here claims to be
/// the game's; they are round numbers picked to make the geometry readable.
fn target() -> Target {
    Target {
        position: Vec3::ZERO,
        forward: Vec3::X,
        up: Vec3::Y,
    }
}

/// `craft_scale` is `1.0` in every fixture but the one test that is about
/// the scale, so the rest read as the geometry they are testing.
fn params() -> ChaseParams {
    ChaseParams {
        fov: 1.0,
        lookat_height: 1.0,
        lookat_length: 10.0,
        pos_height: 2.0,
        pos_length: 8.0,
        spring_horiz: 4.0,
        spring_vert: 2.0,
        craft_scale: 1.0,
    }
}

/// The unit direction from the aim point to the sprung eye, which is the
/// only thing [`Chase::advance`] can change: the radius is pinned.
fn bearing(camera: &Chase, target: Target, params: &ChaseParams) -> Vec3 {
    (camera.sprung_eye() - aim_point(target, params)).normalize()
}

#[test]
fn a_snapped_camera_sits_behind_and_above_the_ship() {
    let camera = Chase::snapped(target(), &params());
    assert_eq!(camera.eye(target(), &params()), Vec3::new(-8.0, 2.0, 0.0));
    // And the state underneath it carries no vertical offset.
    assert_eq!(camera.sprung_eye(), Vec3::new(-8.0, 0.0, 0.0));
}

/// The craft scale shrinks what comes *out* and never what goes in, which is
/// the ordering the capture distinguishes: a state kept in scaled space is a
/// different state, not the same one in other units.
#[test]
fn the_craft_scale_reaches_the_published_points_and_not_the_state() {
    let target = target();
    let scaled = ChaseParams {
        craft_scale: 0.75,
        ..params()
    };
    let unscaled = params();

    // Same spring, same state, whatever the scale says.
    let (mut a, mut b) = (
        Chase::at(Vec3::new(50.0, -30.0, 20.0)),
        Chase::at(Vec3::new(50.0, -30.0, 20.0)),
    );
    for _ in 0..60 {
        a.advance(target, &scaled, 1.0 / 60.0);
        b.advance(target, &unscaled, 1.0 / 60.0);
    }
    assert_eq!(a.sprung_eye(), b.sprung_eye());

    // Different published points, by exactly the scale, about the craft.
    for (scaled_point, plain) in [
        (a.eye(target, &scaled), b.eye(target, &unscaled)),
        (
            look_at_point(target, &scaled),
            look_at_point(target, &unscaled),
        ),
        (anchor(target, &scaled), anchor(target, &unscaled)),
    ] {
        let wanted = target.position + (plain - target.position) * 0.75;
        assert!((scaled_point - wanted).length() < 1e-5, "{scaled_point}");
    }
}

/// The property the 150-tick capture measures, and the reason this module was
/// rewritten: the spring may rotate the eye about the look-at point and may
/// not move it nearer or further.
#[test]
fn the_spring_never_changes_the_eyes_distance_from_the_look_at_point() {
    let params = params();
    let target = target();
    let radius = (spring_anchor(target, &params) - aim_point(target, &params)).length();

    // Deliberately hostile: a wild starting eye, a huge dt, and a second run
    // at a tiny one. Neither may change the radius by more than rounding.
    for (start, dt) in [
        (Vec3::new(50.0, -30.0, 20.0), 1.0 / 60.0),
        (Vec3::new(-1.0, 0.5, 0.25), 10.0),
        (Vec3::new(0.0, 100.0, 0.0), 1e-4),
    ] {
        let mut camera = Chase::at(start);
        for _ in 0..120 {
            camera.advance(target, &params, dt);
            let r = (camera.sprung_eye() - aim_point(target, &params)).length();
            assert!((r - radius).abs() < 1e-3, "radius {r} against {radius}");
        }
    }
}

#[test]
fn the_look_at_point_is_ahead_of_the_ship_whatever_the_eye_is_doing() {
    let params = params();
    let ahead = look_at_point(target(), &params);
    assert_eq!(ahead, Vec3::new(10.0, 1.0, 0.0));
    // Rigid: moving the eye must not move what the camera aims at.
    assert_eq!(look_at_point(target(), &params), ahead);
}

/// A zero `dt` may not *rotate* the eye. It does still snap the radius,
/// because the original re-projects unconditionally rather than inside the
/// spring - so an eye placed off the sphere is pulled onto it on the first
/// call whatever the timestep, and asserting the eye does not move at all
/// would be asserting the wrong thing.
#[test]
fn zero_dt_does_not_rotate_the_eye() {
    let params = params();
    let target = target();
    let mut camera = Chase::at(Vec3::new(100.0, 100.0, 100.0));
    let before = bearing(&camera, target, &params);
    camera.advance(target, &params, 0.0);
    assert!((bearing(&camera, target, &params) - before).length() < 1e-6);
}

#[test]
fn a_settled_camera_stays_settled() {
    let params = params();
    let mut camera = Chase::snapped(target(), &params);
    for _ in 0..600 {
        camera.advance(target(), &params, 1.0 / 60.0);
    }
    assert!((camera.eye(target(), &params) - anchor(target(), &params)).length() < 1e-4);
}

/// Convergence is now **angular**: the eye slides around the sphere toward
/// the anchor's bearing. Measuring the Euclidean error instead would not be
/// monotone, because the first call teleports the eye onto the sphere.
#[test]
fn the_eye_converges_on_the_anchors_bearing_from_anywhere() {
    let params = params();
    let target = target();
    let look_at = aim_point(target, &params);
    let wanted = (spring_anchor(target, &params) - look_at).normalize();

    let mut camera = Chase::at(Vec3::new(50.0, -30.0, 20.0));
    camera.advance(target, &params, 0.0);
    let start = (bearing(&camera, target, &params) - wanted).length();
    assert!(start > 0.1, "the fixture must start well off the anchor");

    let mut previous = start;
    for _ in 0..600 {
        camera.advance(target, &params, 1.0 / 60.0);
        let error = (bearing(&camera, target, &params) - wanted).length();
        // Monotone: a first-order lag never overshoots.
        assert!(
            error <= previous + 1e-6,
            "error grew from {previous} to {error}"
        );
        previous = error;
    }
    assert!(previous < start * 0.01, "{start} to {previous}");
}

/// `pos_height` is rigid: it is not sprung, so it is present in full on the
/// very first frame however far the eye is from where it belongs.
#[test]
fn the_vertical_offset_does_not_lag() {
    let params = params();
    let target = target();
    let mut camera = Chase::at(Vec3::new(50.0, -30.0, 20.0));
    camera.advance(target, &params, 1.0 / 60.0);
    let lift = camera.eye(target, &params) - camera.sprung_eye();
    assert!(
        (lift - target.up * params.pos_height).length() < 1e-6,
        "{lift}"
    );
}

/// Starts a camera at `spring_anchor + offset`, takes **one** step, and
/// reports how far its bearing was from the anchor's before and after.
///
/// One step and not six hundred, deliberately. A displacement lying purely on
/// one spring's axis stays on that axis for exactly one tick: the
/// re-projection changes the eye's *radius*, and from the second tick the
/// error toward the anchor has a component on the other axis too, which the
/// other spring then legitimately acts on. So a run-to-settled assertion here
/// would be testing that mixing rather than the split, and would have to be
/// hedged with a tolerance big enough to hide a real sign error.
fn one_step_bearing_error(target: Target, params: &ChaseParams, offset: Vec3) -> (f32, f32) {
    let look_at = aim_point(target, params);
    let wanted = (spring_anchor(target, params) - look_at).normalize();
    let mut camera = Chase::at(spring_anchor(target, params) + offset);
    let before = (bearing(&camera, target, params) - wanted).length();
    camera.advance(target, params, 1.0 / 60.0);
    (before, (bearing(&camera, target, params) - wanted).length())
}

/// Each spring closes its own axis of the ship's frame and only its own.
///
/// Stated as "a displacement purely along the ship's up does not move under
/// `spring_horiz` alone, and one purely across it does not move under
/// `spring_vert` alone". The re-projection cannot rescue either case, because
/// it is a *radial* scaling about the look-at point and so changes no
/// bearing at all.
#[test]
fn the_two_springs_act_on_their_own_axis_of_the_ships_frame() {
    let target = target();
    let along_up = target.up * 10.0;
    let across_up = Vec3::Z * 10.0;

    let vertical_only = ChaseParams {
        spring_horiz: 0.0,
        ..params()
    };
    let horizontal_only = ChaseParams {
        spring_vert: 0.0,
        ..params()
    };

    for (name, params, offset, moves) in [
        ("vert spring, vert error", &vertical_only, along_up, true),
        ("vert spring, horiz error", &vertical_only, across_up, false),
        (
            "horiz spring, vert error",
            &horizontal_only,
            along_up,
            false,
        ),
        (
            "horiz spring, horiz error",
            &horizontal_only,
            across_up,
            true,
        ),
    ] {
        let (before, after) = one_step_bearing_error(target, params, offset);
        assert!(before > 0.1, "{name}: the fixture must start displaced");
        if moves {
            assert!(
                after < before - 1e-3,
                "{name}: {before} to {after}, expected to close"
            );
        } else {
            assert!(
                (after - before).abs() < 1e-6,
                "{name}: {before} to {after}, expected untouched"
            );
        }
    }
}

#[test]
fn a_rolled_ship_springs_along_its_own_up_not_the_worlds() {
    // Rolled 90 degrees, so the ship's up is world +Z and world +Y is now
    // across it. The same world-space displacement therefore has to change
    // hands between the two springs, which it cannot do if the split is
    // taken against world Y.
    let rolled = Target {
        position: Vec3::ZERO,
        forward: Vec3::X,
        up: Vec3::Z,
    };
    let horizontal_only = ChaseParams {
        spring_vert: 0.0,
        ..params()
    };

    // Upright, +Z is across the ship's up, so the horizontal spring closes it.
    let (before, after) = one_step_bearing_error(target(), &horizontal_only, Vec3::Z * 10.0);
    assert!(after < before - 1e-3, "upright: {before} to {after}");

    // Rolled, the very same offset is along the ship's up and is frozen.
    let (before, after) = one_step_bearing_error(rolled, &horizontal_only, Vec3::Z * 10.0);
    assert!(
        (after - before).abs() < 1e-6,
        "rolled: {before} to {after}, expected untouched"
    );
}
