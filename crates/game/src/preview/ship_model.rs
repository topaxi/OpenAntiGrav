//! HD's `Team Selection` craft, framed by the screen's own `ShipModel` widget.
//!
//! **Authored**, on `Team_Selection_Definition.xml`'s `<Model name="ShipModel">`:
//! `OriginX=1220 OriginY=412`, `x=0 y=0 z=-24`, `RotX=0.4 RotY=-0.5`,
//! `nearZ=1`, one pose for every team. The craft is drawn at its **raw
//! model-space coordinates** (nothing re-centres or rescales it), so each team's
//! own hull placement is what frames it differently from its neighbour.
//!
//! `OriginX/OriginY` are an absolute point in the 1920 by 1080 grid (the same
//! reading [`super::track_model`] makes of `TrackModel`): 1220 and 412 sit inside
//! the `SHIP MODEL` frame, which spans 705 to 1757 across.
//!
//! **Not authored**: the field of view ([`FOV_Y`]) and the far plane ([`FAR`],
//! chosen); HD authors only `nearZ`. **Fitted over the authored values**: the
//! pitch ([`PITCH`]) and a small slide ([`SLIDE_X`]).

use oag_core::math::{Mat4, Vec3, camera};
use oag_display::space::Space;
use oag_ui_screens::picker::hd::ShipModel;

/// The vertical field of view, radians: **fitted, not authored** (the widget
/// authors no lens). With [`PITCH`] and [`SLIDE_X`] below it is one
/// coordinate-descent fit of the silhouette against RPCS3's rest frames of
/// Feisar, Qirex and Assegai: overlap 0.45, from 0.31 at the authored pose
/// with the 1.0 the flyer uses, and Icaras (not fitted) lines up by eye. See
/// `docs/ui/selection-screens.md`.
pub const FOV_Y: f32 = 0.41;
/// The pitch, radians: **fitted**, where the widget authors `RotX=0.4`. The
/// authored value draws the craft steeper and taller than the original in
/// every team compared; 0.22 reproduces it. Why the two differ (a rotation
/// order or sign convention, say) is open. `RotY` is used as authored: the fit
/// moved it from -0.5 to -0.44, within what the silhouette can tell apart.
pub const PITCH: f32 = 0.22;
/// A sideways slide of the model before the lens, in model units: **fitted**.
pub const SLIDE_X: f32 = 0.36;
/// The far plane (**chosen**; the widget authors none).
pub const FAR: f32 = 1000.0;
/// The near plane the widget authors.
pub const NEAR: f32 = 1.0;

/// The view-projection and model matrices of the craft, `scale` times its rest
/// size (the swap's settle, [`super::hull_swap`]).
#[must_use]
pub fn matrices(widget: &ShipModel, space: Space, scale: f32) -> (Mat4, Mat4) {
    let projection = camera::perspective(FOV_Y, space.display_aspect, NEAR, FAR);
    let shift = Vec3::new(
        2.0 * widget.origin[0] / space.size.0 - 1.0,
        1.0 - 2.0 * widget.origin[1] / space.size.1,
        0.0,
    );
    let model = Mat4::from_translation(Vec3::new(SLIDE_X, 0.0, widget.z))
        * Mat4::from_rotation_x(PITCH)
        * Mat4::from_rotation_y(widget.rot_y)
        * Mat4::from_scale(Vec3::splat(scale));
    (Mat4::from_translation(shift) * projection, model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_authored_origin_and_depth_place_the_craft() {
        let widget = ShipModel {
            origin: [960.0, 540.0],
            z: -24.0,
            rot_x: 0.4,
            rot_y: -0.5,
        };
        let (_, model) = matrices(&widget, Space::HD, 1.0);
        let at = model.transform_point3(Vec3::ZERO);
        assert!((at.z + 24.0).abs() < 1e-4);
        let (_, bigger) = matrices(&widget, Space::HD, 2.0);
        let nose = bigger.transform_point3(Vec3::X) - bigger.transform_point3(Vec3::ZERO);
        assert!((nose.length() - 2.0).abs() < 1e-4);
    }
}
