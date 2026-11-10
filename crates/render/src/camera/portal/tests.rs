use oag_core::math::{Mat4, Vec3, Vec4, camera};

use super::*;

const EAST: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const NORTH: Vec3 = Vec3::new(0.0, 0.0, -1.0);
const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

fn close(a: Vec3, b: Vec3) {
    assert!((a - b).length() < 1e-3, "{a} against {b}");
}

/// The road runs north through the entrance and east through the exit, banked over.
fn pair() -> (Mat4, Mat4, Vec3, Vec3, Vec3) {
    let entrance_at = Vec3::new(100.0, 5.0, -40.0);
    let exit_at = Vec3::new(-300.0, 60.0, 900.0);
    let banked = (UP + NORTH * 0.3).normalize();
    let entrance = frame(entrance_at, NORTH, UP, Face::Oncoming).unwrap();
    let exit = frame(exit_at, EAST, banked, Face::Departing).unwrap();
    (entrance, exit, entrance_at, exit_at, banked)
}

fn eye_and_axes(view: Mat4) -> (Vec3, Vec3, Vec3, Vec3) {
    let camera = view.inverse();
    (
        camera.w_axis.truncate(),
        camera.x_axis.truncate(),
        camera.y_axis.truncate(),
        -camera.z_axis.truncate(),
    )
}

/// The player's camera, behind the entrance on the road, off to the right and above: it lands
/// behind the exit on the road by the same offsets, looks along the road, right way up and on the
/// same side. Dropping the turn, or building both frames on the road's direction, puts it past
/// the exit looking back.
#[test]
fn a_camera_keeps_its_place_and_heading_through_the_pair() {
    let (entrance, exit, entrance_at, exit_at, banked) = pair();
    let eye = entrance_at - NORTH * 50.0 + EAST * 3.0 + UP * 8.0;
    let main = camera::look_at(eye, eye + NORTH, UP);
    let carried = view(main, through(entrance, exit));
    let (at, right, up, forward) = eye_and_axes(carried);
    let exit_right = EAST.cross(banked).normalize();
    close(at, exit_at - EAST * 50.0 + exit_right * 3.0 + banked * 8.0);
    close(forward, EAST);
    close(up, banked);
    close(right, exit_right);
}

/// A rigid transform: the determinant of `-1` is a mirrored picture and flipped winding.
#[test]
fn the_pair_never_mirrors() {
    let (entrance, exit, ..) = pair();
    let carried = through(entrance, exit);
    assert!((carried.determinant() - 1.0).abs() < 1e-3);
}

/// A point of the entrance's surface lands where the main camera has it: what makes the texture
/// line up with the surface it is drawn on.
#[test]
fn a_point_on_the_entrance_lands_on_the_same_pixel_of_both_views() {
    let (entrance, exit, entrance_at, ..) = pair();
    let eye = entrance_at - NORTH * 80.0 + UP * 6.0;
    let main = camera::look_at(eye, eye + NORTH, UP);
    let projection = camera::perspective(1.2, 1.7, 1.0, 20_000.0);
    let carried = through(entrance, exit);
    let point = (entrance_at + EAST * 4.0 + UP * 2.0).extend(1.0);
    let seen = projection * main * point;
    let drawn = projection * view(main, carried) * (carried * point);
    assert!(
        (seen - drawn).length() < 0.05 * seen.w.abs(),
        "{seen} {drawn}"
    );
}

fn oblique_of(eye_ahead: f32) -> (Mat4, Mat4, Mat4, Vec3) {
    let (_, exit, _, exit_at, _) = pair();
    let eye = exit_at - EAST * eye_ahead + UP * 7.0;
    let carried = camera::look_at(eye, eye + EAST, UP);
    let projection = camera::perspective(1.2, 1.7, 1.0, 20_000.0);
    let plane = eye_plane(carried, exit);
    (
        projection,
        oblique_near(projection, plane).unwrap(),
        carried,
        exit_at,
    )
}

fn depth(matrix: Mat4, point: Vec3) -> (f32, f32) {
    let clip = matrix * point.extend(1.0);
    (clip.z, clip.w)
}

/// What stands between the virtual eye and the exit is clipped, on the near plane side of a
/// 0..1 depth range; the road past it is kept and in order, with `x`, `y` and `w` untouched.
#[test]
fn the_oblique_near_plane_clips_what_is_behind_the_exit_and_nothing_past_it() {
    let (projection, oblique, carried, exit_at) = oblique_of(120.0);
    let behind = carried * (exit_at - EAST * 30.0).extend(1.0);
    assert!((oblique * behind).z < 0.0, "behind the exit is clipped");
    let (near, w_near) = depth(oblique * carried, exit_at + EAST * 10.0);
    let (far, w_far) = depth(oblique * carried, exit_at + EAST * 4000.0);
    assert!(near > 0.0 && near < w_near, "{near} of {w_near}");
    assert!(far > 0.0 && far < w_far, "{far} of {w_far}");
    assert!(near / w_near < far / w_far, "depth order holds");
    let (plain, oblique_clip) = (projection * behind, oblique * behind);
    assert_eq!(
        (plain.x, plain.y, plain.w),
        (oblique_clip.x, oblique_clip.y, oblique_clip.w)
    );
}

/// The plane through the exit's own centre is depth zero, off the axis too.
#[test]
fn the_exit_plane_is_depth_zero() {
    let (_, oblique, carried, exit_at) = oblique_of(120.0);
    let (z, w) = depth(oblique * carried, exit_at + UP * 20.0);
    assert!(z.abs() < 1e-2 * w.abs(), "{z} of {w}");
}

/// Every corner of the plain view volume past the plane stays inside: the far plane's tilt
/// must not eat anything the plain matrix would have drawn.
#[test]
fn the_far_plane_keeps_the_whole_view_volume() {
    let (projection, oblique, ..) = oblique_of(120.0);
    let back = projection.inverse();
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            let corner = back * Vec4::new(x, y, 1.0, 1.0);
            let clip = oblique * corner;
            assert!(clip.z <= clip.w * 1.001, "{clip} at {x} {y}");
        }
    }
}

/// An eye at or past the plane has no oblique matrix: nothing is seen through the portal.
#[test]
fn an_eye_on_the_far_side_has_no_oblique_near_plane() {
    let (_, exit, _, exit_at, _) = pair();
    let projection = camera::perspective(1.2, 1.7, 1.0, 20_000.0);
    let eye = exit_at + EAST * 20.0;
    let past = camera::look_at(eye, eye + EAST, UP);
    assert!(oblique_near(projection, eye_plane(past, exit)).is_none());
}

#[test]
fn a_road_parallel_to_its_up_has_no_frame() {
    assert!(frame(Vec3::ZERO, UP, UP, Face::Departing).is_none());
}
