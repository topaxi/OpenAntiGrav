use super::*;
use oag_physics::maglock::ramp;

/// Every hit/miss sequence the probe can produce reads back exactly: the
/// blend's own ramp is the only thing [`contact_this_tick`] sees, so a
/// sequence that rises, saturates, falls and bottoms out covers each branch.
#[test]
fn the_ramp_reads_back_as_the_probe_it_was_given() {
    let probes = [
        false, true, true, true, true, true, true, true, false, false, true, false, false, false,
        false, false, false, true, false, true, true, false,
    ];
    let mut blend = 0.0;
    for (tick, &hit) in probes.iter().enumerate() {
        let next = ramp(blend, hit);
        assert_eq!(
            contact_this_tick(blend, next),
            hit,
            "tick {tick}: {blend} -> {next}"
        );
        blend = next;
    }
}

/// A craft scaled by `0.75` and pitched nose-up: MagEffect1 keeps all of
/// that, and sits `2.5` model units down the craft's own up axis.
#[test]
fn the_riding_model_keeps_the_crafts_frame_and_sits_below_it() {
    let pitch = Quat::from_rotation_x(0.4);
    let position = Vec3::new(10.0, 20.0, 30.0);
    let model = Mat4::from_scale_rotation_translation(Vec3::splat(0.75), pitch, position);
    let ride = ride_matrix(model);
    let up = pitch * Vec3::Y;
    let expected = position - up * (2.5 * 0.75);
    assert!(ride.w_axis.truncate().abs_diff_eq(expected, 1e-5));
    assert!((ride.y_axis.truncate().length() - 0.75).abs() < 1e-6);
    assert!(ride.x_axis.abs_diff_eq(model.x_axis, 1e-6));
    assert!(ride.z_axis.abs_diff_eq(model.z_axis, 1e-6));
}

/// The same pitched craft over a flat track: MagEffect2 takes its up from the
/// track, not the craft, points along the nose flattened onto the road, is
/// unit scale, and shares MagEffect1's position.
#[test]
fn the_lying_model_takes_the_tracks_up_and_the_crafts_heading() {
    let pitch = Quat::from_rotation_x(0.4);
    let model =
        Mat4::from_scale_rotation_translation(Vec3::splat(0.75), pitch, Vec3::new(1.0, 2.0, 3.0));
    let down = Vec3::new(0.0, -3.0, 0.0);
    let lie = lie_matrix(model, down);
    assert!(lie.y_axis.truncate().abs_diff_eq(Vec3::Y, 1e-6));
    assert!(lie.z_axis.truncate().abs_diff_eq(Vec3::Z, 1e-6));
    assert!(lie.x_axis.truncate().abs_diff_eq(Vec3::X, 1e-6));
    for axis in [lie.x_axis, lie.y_axis, lie.z_axis] {
        assert!((axis.truncate().length() - 1.0).abs() < 1e-6);
        assert_eq!(axis.w, 0.0);
    }
    assert!(lie.w_axis.abs_diff_eq(ride_matrix(model).w_axis, 1e-5));
}

/// The two models differ in scale by exactly the craft's own: dropping the
/// orthonormal rebuild for "the craft's matrix again" would fail here.
#[test]
fn only_the_riding_model_carries_the_craft_scale() {
    let model = Mat4::from_scale_rotation_translation(
        Vec3::splat(0.75),
        Quat::from_rotation_z(0.3),
        Vec3::ZERO,
    );
    let ride = ride_matrix(model);
    let lie = lie_matrix(model, Vec3::NEG_Y);
    assert!((ride.x_axis.truncate().length() - 0.75).abs() < 1e-6);
    assert!((lie.x_axis.truncate().length() - 1.0).abs() < 1e-6);
    // A rolled craft's up leans; the lying model's does not.
    assert!(ride.y_axis.truncate().normalize().dot(Vec3::Y) < 0.99);
    assert!(lie.y_axis.truncate().abs_diff_eq(Vec3::Y, 1e-6));
}
