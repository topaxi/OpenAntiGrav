//! The maths of a planar portal: a world seen through a pair of frames on the circuit.
//!
//! A portal's frame has its `+z` out of the face a craft sees, its `+y` the road's up and its
//! `+x` to the side that makes it right-handed. So the entrance faces back along the road
//! ([`Face::Oncoming`]) and the exit forward along it ([`Face::Departing`]), and
//! [`through`] is the camera transform of the spec, `exit * rotation_180 * entrance^-1`. The turn
//! about the local up is what takes a point in front of the entrance to a point behind the exit:
//! with both frames on the road's own direction a camera would land past the exit looking back.
//!
//! Every matrix here is right-handed with a 0..1 depth range, [`oag_core::math::camera`]'s.

use oag_core::math::{Mat4, Vec3, Vec4};

/// Which way a portal's face looks along the road.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// Back along the road, at the craft coming up to it: an entrance.
    Oncoming,
    /// Forward along the road: an exit.
    Departing,
}

/// A portal's frame at `centre` on a road going `along` with `up` its normal, the road's own
/// banking kept. `along` need not be square to `up`; `None` when they are parallel.
#[must_use]
pub fn frame(centre: Vec3, along: Vec3, up: Vec3, face: Face) -> Option<Mat4> {
    let up = up.try_normalize()?;
    let along = (along - up * along.dot(up)).try_normalize()?;
    let normal = match face {
        Face::Oncoming => -along,
        Face::Departing => along,
    };
    Some(Mat4::from_cols(
        up.cross(normal).extend(0.0),
        up.extend(0.0),
        normal.extend(0.0),
        centre.extend(1.0),
    ))
}

/// The rigid transform that carries a point in front of `entrance` to where it is seen behind
/// `exit`: `exit * rotation_180 * entrance^-1`, the turn about the portal's local up.
#[must_use]
pub fn through(entrance: Mat4, exit: Mat4) -> Mat4 {
    let turn = Mat4::from_cols(
        Vec4::new(-1.0, 0.0, 0.0, 0.0),
        Vec4::Y,
        Vec4::new(0.0, 0.0, -1.0, 0.0),
        Vec4::W,
    );
    exit * turn * entrance.inverse()
}

/// The view matrix of the camera `main_view` seen through `through`. The spec's `virtual_camera =
/// through * main_camera` is on camera-to-world transforms, a view matrix is the inverse.
#[must_use]
pub fn view(main_view: Mat4, through: Mat4) -> Mat4 {
    main_view * through.inverse()
}

/// The plane through `frame`'s origin square to its `+z`, in `view`'s eye space, as `(n, d)` with
/// `n.dot(p) + d` positive in front of the face.
#[must_use]
pub fn eye_plane(view: Mat4, frame: Mat4) -> Vec4 {
    let normal = frame.z_axis.truncate();
    let point = frame.w_axis.truncate();
    let world = normal.extend(-normal.dot(point));
    view.inverse().transpose() * world
}

/// `projection` with its near plane swung onto `plane`, an eye-space plane with the eye behind
/// it, as Lengyel's oblique near-plane clipping for a 0..1 depth range: what lies behind the
/// plane is clipped, every other row of the matrix is untouched, so a point lands on the same
/// `x`, `y` and `w`. Only the depth changes, and the far plane tilts with it: cull with the plain
/// matrix. `None` when the eye is not behind the plane by a usable margin.
#[must_use]
pub fn oblique_near(projection: Mat4, plane: Vec4) -> Option<Mat4> {
    // Below this the depth row degenerates.
    const MARGIN: f32 = 0.01;
    if plane.w > -MARGIN {
        return None;
    }
    // The corner of the view volume opposite the plane, on the far plane.
    let corner = projection.inverse() * Vec4::new(plane.x.signum(), plane.y.signum(), 1.0, 1.0);
    let w_row = projection.row(3);
    let scale = w_row.dot(corner) / plane.dot(corner);
    let z_row = plane * scale;
    let mut oblique = projection;
    oblique.x_axis.z = z_row.x;
    oblique.y_axis.z = z_row.y;
    oblique.z_axis.z = z_row.z;
    oblique.w_axis.z = z_row.w;
    Some(oblique)
}

#[cfg(test)]
mod tests;
