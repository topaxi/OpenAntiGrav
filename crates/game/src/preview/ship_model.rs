//! HD's `Team Selection` craft, framed as the screen's own code frames it.
//!
//! **The widget's authored pose is not what the original draws.**
//! `Team_Selection_Definition.xml`'s `<Model name="ShipModel">` authors
//! `OriginX=1220 OriginY=412`, `z=-24`, `RotX=0.4 RotY=-0.5`, and the screen's
//! update writes over the rotation, the depth, the lens and the origin every
//! frame ([`oag_title::ShipFit`];
//! `docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`). What stays
//! the widget's is `nearZ` and `ObeySafeZone`. A title with a fit is drawn by
//! [`fitted`]; one without (Omega, on a pose it borrows) by [`matrices`].
//!
//! The craft is drawn at its **raw model-space coordinates** either way:
//! nothing re-centres or rescales it.

use oag_core::math::{Mat4, Vec3, camera};
use oag_display::space::Space;
use oag_title::ShipFit;
use oag_ui_screens::picker::hd::ShipModel;

/// The vertical field of view of [`matrices`], radians: **fitted, not
/// authored**, against HD's frames before HD's own camera was read. It frames
/// only a title that borrows a pose in place of a fit. See
/// `docs/ui/selection-screens.md`.
pub const FOV_Y: f32 = 0.41;
/// The pitch of [`matrices`], radians: **fitted**, as [`FOV_Y`] is.
pub const PITCH: f32 = 0.22;
/// A sideways slide of the model before the lens in [`matrices`], model units:
/// **fitted**, as [`FOV_Y`] is.
pub const SLIDE_X: f32 = 0.36;
/// The far plane of [`matrices`] (**chosen**).
pub const FAR: f32 = 1000.0;
/// The near plane the widget authors.
pub const NEAR: f32 = 1.0;

/// The view-projection and model matrices of the craft on the widget's own
/// origin and depth, `scale` times its rest size (the swap's settle,
/// [`super::hull_swap`]), for a title with no [`ShipFit`].
#[must_use]
pub fn matrices(widget: &ShipModel, space: Space, scale: f32) -> (Mat4, Mat4) {
    let projection = camera::perspective(FOV_Y, space.display_aspect, NEAR, FAR);
    let model = Mat4::from_translation(Vec3::new(SLIDE_X, 0.0, widget.z))
        * Mat4::from_rotation_x(PITCH)
        * Mat4::from_rotation_y(widget.rot_y)
        * Mat4::from_scale(Vec3::splat(scale));
    (
        Mat4::from_translation(grid_shift(widget.origin, space)) * projection,
        model,
    )
}

/// Where a grid point sits in normalised device coordinates, as a slide of the
/// finished picture.
fn grid_shift(origin: [f32; 2], space: Space) -> Vec3 {
    Vec3::new(
        2.0 * origin[0] / space.size.0 - 1.0,
        1.0 - 2.0 * origin[1] / space.size.1,
        0.0,
    )
}

/// Where the fit holds a model at rest: how far in front of the lens, and how
/// far the widget's origin is slid, in the screen's grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Framing {
    pub depth: f32,
    pub slide: [f32; 2],
}

/// One frame of the fit at `depth`: the slide that centres the box, and whether
/// the centred box is inside the window.
fn measure(fit: &ShipFit, aspect: f32, box_z: [f32; 2], depth: f32) -> ([f32; 2], bool) {
    let turn = Mat4::from_rotation_x(fit.rotation[0]) * Mat4::from_rotation_y(fit.rotation[1]);
    let lens = 1.0 / fit.fit_half_fov.tan();
    // The original's running minimum and maximum start at zero, not at the
    // first corner.
    let (mut low, mut high) = ([0.0_f32; 2], [0.0_f32; 2]);
    for corner in 0..8 {
        let at = turn.transform_point3(Vec3::new(
            if corner & 2 == 0 {
                -fit.half_box[0]
            } else {
                fit.half_box[0]
            },
            if corner & 1 == 0 {
                -fit.half_box[1]
            } else {
                fit.half_box[1]
            },
            box_z[corner >> 2],
        ));
        let w = depth - at.z;
        let on_screen = [
            at.x * lens / aspect / w * fit.unit[0],
            -(at.y * lens / w) * fit.unit[1],
        ];
        for axis in 0..2 {
            low[axis] = low[axis].min(on_screen[axis]);
            high[axis] = high[axis].max(on_screen[axis]);
        }
    }
    let slide = [-0.5 * (low[0] + high[0]), -0.5 * (low[1] + high[1])];
    let inside = (0..2).all(|axis| {
        low[axis] + slide[axis] >= -fit.window[axis] && high[axis] + slide[axis] <= fit.window[axis]
    });
    (slide, inside)
}

/// The depth the fit moves to from `depth` in one frame: nearer while the box
/// is inside the window, farther while it is not, never nearer than the floor.
fn stepped(fit: &ShipFit, depth: f32, inside: bool) -> f32 {
    let moved = if inside {
        depth - fit.step
    } else {
        depth + fit.step
    };
    moved.max(fit.floor)
}

/// Where the fit comes to rest for a box `box_z` fore and aft.
///
/// The original steps the depth every frame and never stops: at rest it
/// alternates between the last depth the box fits at and the one a step
/// nearer, and the drawn depth and slide ease towards whichever is current, so
/// they sit between the two. This is the mean of that pair.
#[must_use]
pub fn rest(fit: &ShipFit, aspect: f32, box_z: [f32; 2]) -> Framing {
    // Far enough for any depth a model's length can ask for: the fit moves one
    // step a frame, and a box as long as the far plane is deep rests inside it.
    let frames = (2.0 * fit.far / fit.step) as usize;
    let mut depth = fit.start;
    let mut seen = [(depth, [0.0; 2]); 2];
    for frame in 0..frames {
        let (slide, inside) = measure(fit, aspect, box_z, depth);
        seen[frame % 2] = (depth, slide);
        let next = stepped(fit, depth, inside);
        // Back where it was two frames ago: the rest pair, or the floor.
        if frame > 0 && (next - seen[(frame + 1) % 2].0).abs() < 0.5 * fit.step {
            break;
        }
        depth = next;
    }
    Framing {
        depth: 0.5 * (seen[0].0 + seen[1].0),
        slide: [
            0.5 * (seen[0].1[0] + seen[1].1[0]),
            0.5 * (seen[0].1[1] + seen[1].1[1]),
        ],
    }
}

/// The fore and aft extent of the fit's box: the Zone box where the screen
/// draws the Zone hull, else the selected model's own length each way. `None`
/// where neither is known, and the craft is then not fitted.
#[must_use]
pub fn box_z(fit: &ShipFit, zone_hull: bool, length: Option<f32>) -> Option<[f32; 2]> {
    if zone_hull {
        Some(fit.zone_box)
    } else {
        length.map(|length| [-length, length])
    }
}

/// The view-projection and model matrices of the craft as the screen's fit
/// frames a box `box_z` fore and aft, `scale` times its rest size (the swap's
/// settle, [`super::hull_swap`]). `safe_area` is the title's `Safe Area Setting`
/// as a scale, `1.0` for a widget that does not obey it: the original scales the
/// finished picture and the origin's distance from the middle of the screen by it.
#[must_use]
pub fn fitted(
    fit: &ShipFit,
    box_z: [f32; 2],
    space: Space,
    scale: f32,
    safe_area: f32,
) -> (Mat4, Mat4) {
    let framing = rest(fit, space.display_aspect, box_z);
    let projection = camera::perspective(
        fit.fov_degrees.to_radians(),
        space.display_aspect,
        NEAR,
        fit.far,
    );
    let model = Mat4::from_translation(Vec3::new(0.0, 0.0, -framing.depth))
        * Mat4::from_rotation_x(fit.rotation[0])
        * Mat4::from_rotation_y(fit.rotation[1])
        * Mat4::from_scale(Vec3::splat(scale));
    let origin = [
        fit.origin[0] + framing.slide[0],
        fit.origin[1] + framing.slide[1],
    ];
    (
        Mat4::from_translation(grid_shift(origin, space) * safe_area)
            * Mat4::from_scale(Vec3::new(safe_area, safe_area, 1.0))
            * projection,
        model,
    )
}

/// The matrices the screen draws its craft with: the title's fit when it has
/// one and the box is known, else the widget's own pose. `safe_area` is the
/// title's `Safe Area Setting` as a scale, which the fit's picture takes when
/// the widget authors `ObeySafeZone`.
#[must_use]
pub fn framed(
    widget: &ShipModel,
    fit: Option<(&ShipFit, [f32; 2])>,
    space: Space,
    scale: f32,
    safe_area: f32,
) -> (Mat4, Mat4) {
    match fit {
        Some((fit, box_z)) => {
            let safe_area = if widget.obey_safe_zone {
                safe_area
            } else {
                1.0
            };
            fitted(fit, box_z, space, scale, safe_area)
        }
        None => matrices(widget, space, scale),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIT: ShipFit = oag_hd::frontend::SHIP_FIT;
    const ASPECT: f32 = 16.0 / 9.0;

    #[test]
    fn the_authored_origin_and_depth_place_the_craft() {
        let widget = ShipModel {
            origin: [960.0, 540.0],
            z: -24.0,
            rot_x: 0.4,
            rot_y: -0.5,
            obey_safe_zone: true,
        };
        let (_, model) = matrices(&widget, Space::HD, 1.0);
        let at = model.transform_point3(Vec3::ZERO);
        assert!((at.z + 24.0).abs() < 1e-4);
        let (_, bigger) = matrices(&widget, Space::HD, 2.0);
        let nose = bigger.transform_point3(Vec3::X) - bigger.transform_point3(Vec3::ZERO);
        assert!((nose.length() - 2.0).abs() < 1e-4);
    }

    /// The rest pair straddles the window's edge: the box fits a twentieth of a
    /// unit farther than the rest depth and not a twentieth nearer.
    #[test]
    fn the_fit_rests_where_the_box_just_fits_the_window() {
        for length in [3.8, 4.1, 4.5] {
            let box_z = [-length, length];
            let framing = rest(&FIT, ASPECT, box_z);
            assert!(framing.depth > FIT.floor, "{length}: {framing:?}");
            let half = 0.5 * FIT.step;
            assert!(measure(&FIT, ASPECT, box_z, framing.depth + half).1);
            assert!(!measure(&FIT, ASPECT, box_z, framing.depth - half).1);
        }
    }

    /// A longer model rests farther away, and a box short enough to fit at the
    /// floor rests on it (the disc's shortest, `length="3.2"`, does).
    #[test]
    fn a_longer_model_rests_farther_and_a_short_one_on_the_floor() {
        let depth = |length: f32| rest(&FIT, ASPECT, [-length, length]).depth;
        assert!(depth(4.5) > depth(3.8) + 1.0);
        assert!(depth(3.8) > depth(3.2) + 0.5);
        assert!((depth(3.2) - FIT.floor).abs() < 1e-4);
    }

    /// The window is in the fit's own unit, a device coordinate times the whole
    /// screen: Feisar's box (`length="3.8"`) rests 535 of them wide, which is
    /// 535 pixels of a 1920 grid across both halves, not 1070.
    #[test]
    fn the_window_is_measured_in_device_coordinates_times_the_screen() {
        let box_z = [-3.8, 3.8];
        let framing = rest(&FIT, ASPECT, box_z);
        let (view_projection, model) = fitted(&FIT, box_z, Space::HD, 1.0, 1.0);
        let (mut low, mut high) = (f32::MAX, f32::MIN);
        for corner in 0..8 {
            let at = Vec3::new(
                if corner & 2 == 0 { -1.125 } else { 1.125 },
                if corner & 1 == 0 { -0.75 } else { 0.75 },
                box_z[corner >> 2],
            );
            let x = (view_projection * model).project_point3(at).x * 960.0;
            low = low.min(x);
            high = high.max(x);
        }
        let wide = high - low;
        assert!((wide - 535.0).abs() < 8.0, "{wide} px at {framing:?}");
    }

    /// The origin the fit slides is the screen's own constant, not the
    /// widget's authored one, and the Zone hull is fitted to the Zone box.
    #[test]
    fn the_fit_uses_its_own_origin_and_the_zone_box() {
        assert_eq!(FIT.origin, [1220.0, 500.0]);
        assert_eq!(box_z(&FIT, true, Some(3.8)), Some([-4.2, 4.2]));
        assert_eq!(box_z(&FIT, false, Some(3.8)), Some([-3.8, 3.8]));
        assert_eq!(box_z(&FIT, false, None), None);
        let widget = ShipModel {
            origin: [1220.0, 412.0],
            z: -24.0,
            rot_x: 0.4,
            rot_y: -0.5,
            obey_safe_zone: true,
        };
        let authored = framed(&widget, None, Space::HD, 1.0, 1.0);
        let fit = framed(&widget, Some((&FIT, [-3.8, 3.8])), Space::HD, 1.0, 1.0);
        assert_ne!(authored, fit);
        assert_eq!(fit, fitted(&FIT, [-3.8, 3.8], Space::HD, 1.0, 1.0));
    }

    /// `ObeySafeZone` shrinks the picture and pulls its origin towards the
    /// middle of the screen by the Safe Area; a widget without it is untouched.
    #[test]
    fn an_obeying_widget_is_drawn_inside_the_safe_area() {
        let mut widget = ShipModel {
            origin: [1220.0, 412.0],
            z: -24.0,
            rot_x: 0.4,
            rot_y: -0.5,
            obey_safe_zone: true,
        };
        let fit = Some((&FIT, [-3.8, 3.8]));
        let at =
            |matrices: (Mat4, Mat4), point: Vec3| (matrices.0 * matrices.1).project_point3(point);
        let full = framed(&widget, fit, Space::HD, 1.0, 1.0);
        let inset = framed(&widget, fit, Space::HD, 1.0, 0.96);
        for point in [Vec3::ZERO, Vec3::new(1.0, 0.5, -3.0)] {
            let (full, inset) = (at(full, point), at(inset, point));
            assert!((inset.x - 0.96 * full.x).abs() < 1e-5);
            assert!((inset.y - 0.96 * full.y).abs() < 1e-5);
        }
        widget.obey_safe_zone = false;
        assert_eq!(framed(&widget, fit, Space::HD, 1.0, 0.96), full);
    }
}
