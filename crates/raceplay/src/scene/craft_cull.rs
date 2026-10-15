//! A craft the camera cannot see is not drawn.
//!
//! The hulls were drawn with no frustum at all, so a grid of twelve cost twelve
//! hulls' vertices and draw calls however many were on screen: HD's craft are a
//! large share of a frame's triangles, and on a field that has spread out most of
//! them are behind the camera or off to a side. The test is the circuit's own, the
//! frustum's sphere test, with a margin; it leaves out only a hull no pixel of
//! which is in the picture, and nothing else about the draw changes.

use super::*;

/// How much wider than the model's own bounding sphere a hull is allowed to
/// reach before it counts as out of view.
///
/// The sphere is fitted to the hull at rest. The airbrake flaps swing about their
/// hinges, a node animation can move a part, and the camera's jitter moves the
/// frustum by a pixel; the margin keeps all of those inside the bound. **Chosen,
/// not measured**: a safety factor, tight enough that it still culls a craft
/// that is a hull's length outside the picture.
const MARGIN: f32 = 1.5;

/// Whether a hull placed by `matrix`, whose model-space bounding sphere is
/// `centre` and `radius`, can touch the picture. With no frustum (the
/// `frustum_culling` setting is off) every hull is in view.
pub(super) fn hull_in_view(
    frustum: Option<&Frustum>,
    matrix: Mat4,
    centre: Vec3,
    radius: f32,
) -> bool {
    let Some(frustum) = frustum else {
        return true;
    };
    if !radius.is_finite() || radius <= 0.0 {
        return true;
    }
    let scale = matrix
        .x_axis
        .truncate()
        .length()
        .max(matrix.y_axis.truncate().length())
        .max(matrix.z_axis.truncate().length());
    frustum.intersects_sphere(matrix.transform_point3(centre), radius * scale * MARGIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn looking_down_negative_z() -> Frustum {
        let projection = oag_core::math::camera::perspective(1.0, 16.0 / 9.0, 0.1, 1000.0);
        Frustum::from_view_projection(projection)
    }

    #[test]
    fn a_hull_in_front_is_drawn_and_one_behind_the_camera_is_not() {
        let frustum = looking_down_negative_z();
        let place = |z: f32| Mat4::from_translation(Vec3::new(0.0, 0.0, z));
        assert!(hull_in_view(Some(&frustum), place(-30.0), Vec3::ZERO, 4.0));
        assert!(!hull_in_view(Some(&frustum), place(30.0), Vec3::ZERO, 4.0));
    }

    #[test]
    fn a_hull_beside_the_view_is_culled_and_one_straddling_its_edge_is_kept() {
        let frustum = looking_down_negative_z();
        let place = |x: f32| Mat4::from_translation(Vec3::new(x, 0.0, -30.0));
        assert!(!hull_in_view(Some(&frustum), place(200.0), Vec3::ZERO, 4.0));
        // At depth 30 the half-width is 30 * tan(0.5) * 16/9, about 29.1.
        assert!(hull_in_view(Some(&frustum), place(31.0), Vec3::ZERO, 4.0));
    }

    #[test]
    fn the_matrix_scale_widens_the_bound() {
        let frustum = looking_down_negative_z();
        let at = |scale: f32| {
            Mat4::from_translation(Vec3::new(60.0, 0.0, -30.0))
                * Mat4::from_scale(Vec3::splat(scale))
        };
        assert!(!hull_in_view(Some(&frustum), at(1.0), Vec3::ZERO, 4.0));
        assert!(hull_in_view(Some(&frustum), at(10.0), Vec3::ZERO, 4.0));
    }

    #[test]
    fn with_culling_off_every_hull_is_in_view() {
        assert!(hull_in_view(
            None,
            Mat4::from_translation(Vec3::splat(1e6)),
            Vec3::ZERO,
            4.0
        ));
    }

    #[test]
    fn a_degenerate_bound_is_never_culled() {
        let frustum = looking_down_negative_z();
        let far = Mat4::from_translation(Vec3::new(0.0, 0.0, 500.0));
        assert!(hull_in_view(Some(&frustum), far, Vec3::ZERO, 0.0));
        assert!(hull_in_view(Some(&frustum), far, Vec3::ZERO, f32::NAN));
    }
}
