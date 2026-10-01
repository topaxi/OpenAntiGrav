//! What [`super`] is asserted to do: the original's corner test, and the section
//! narrowing built on it.

use super::*;
use crate::pvs::tests::pvs;

/// The camera at the origin looking down `-z`, 60 degrees vertically at the
/// original's 480:272.
fn view_projection() -> Mat4 {
    let projection =
        oag_core::math::camera::perspective(60f32.to_radians(), 480.0 / 272.0, 1.0, 2000.0);
    projection * oag_core::math::camera::look_at(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y)
}

#[test]
fn a_box_ahead_of_the_camera_is_not_rejected() {
    assert!(!box_outside_view(
        &view_projection(),
        [-5.0, -5.0, -60.0],
        [5.0, 5.0, -40.0]
    ));
}

#[test]
fn a_box_wholly_past_a_side_plane_is_rejected() {
    // At depth 50 the half width is `50 * tan(30deg) * 480/272 = 51`, and
    // 29 the half height.
    let vp = view_projection();
    assert!(box_outside_view(
        &vp,
        [60.0, -5.0, -50.0],
        [90.0, 5.0, -40.0]
    ));
    assert!(box_outside_view(
        &vp,
        [-90.0, -5.0, -50.0],
        [-60.0, 5.0, -40.0]
    ));
    assert!(box_outside_view(
        &vp,
        [-5.0, 40.0, -50.0],
        [5.0, 60.0, -40.0]
    ));
    assert!(box_outside_view(
        &vp,
        [-5.0, -60.0, -50.0],
        [5.0, -40.0, -40.0]
    ));
}

#[test]
fn a_box_straddling_a_side_plane_is_kept() {
    assert!(!box_outside_view(
        &view_projection(),
        [40.0, -5.0, -60.0],
        [90.0, 5.0, -40.0]
    ));
}

/// Corners outside *different* planes do not reject: only a plane every
/// corner is outside does. This box has corners past the right plane and
/// corners past the top plane, but its low corner is inside both.
#[test]
fn corners_outside_different_planes_do_not_reject() {
    assert!(!box_outside_view(
        &view_projection(),
        [30.0, 15.0, -50.0],
        [200.0, 200.0, -48.0]
    ));
}

#[test]
fn there_is_no_far_plane() {
    assert!(!box_outside_view(
        &view_projection(),
        [-10.0, -10.0, -90_000.0],
        [10.0, 10.0, -80_000.0]
    ));
}

#[test]
fn a_box_behind_the_camera_is_rejected() {
    assert!(box_outside_view(
        &view_projection(),
        [-5.0, -5.0, 10.0],
        [5.0, 5.0, 40.0]
    ));
}

/// The `z < 0` outcode is half way between near and far, not the near plane:
/// a box entirely inside it is rejected although it is in front of the camera.
#[test]
fn the_depth_test_is_the_half_way_depth_and_not_the_near_plane() {
    let vp = view_projection();
    assert!(box_outside_view(&vp, [-0.2, -0.2, -2.4], [0.2, 0.2, -1.5]));
    assert!(!box_outside_view(&vp, [-0.2, -0.2, -3.0], [0.2, 0.2, -1.5]));
}

#[test]
fn only_the_sections_the_view_reaches_stay_set() {
    let pvs = pvs(&[
        (0, [-5.0, -5.0, -60.0], [5.0, 5.0, -40.0]),
        (1, [60.0, -5.0, -50.0], [90.0, 5.0, -40.0]),
        (2, [-90.0, -5.0, -50.0], [-60.0, 5.0, -40.0]),
    ]);
    let mask = sections_in_view(&pvs, &view_projection());
    assert_eq!(mask & 0b111, 0b001);
}

#[test]
fn a_section_with_no_box_is_never_narrowed_away() {
    let mask = sections_in_view(&TrackPvs::empty(), &view_projection());
    assert_eq!(mask, u64::MAX);
}

#[test]
fn narrowing_only_removes_what_the_mask_allowed() {
    let pvs = pvs(&[
        (0, [-5.0, -5.0, -60.0], [5.0, 5.0, -40.0]),
        (1, [60.0, -5.0, -50.0], [90.0, 5.0, -40.0]),
    ]);
    let vp = view_projection();
    let set = VisibleSet::everything().within_view(&pvs, &vp);
    assert!(set.allows(1 << 0), "a section in view stays");
    assert!(!set.allows(1 << 1), "a section out of view goes");
    // An unplaced draw carries every bit, so it rides on whatever is left.
    assert!(set.allows(crate::pvs::ALWAYS));
}
