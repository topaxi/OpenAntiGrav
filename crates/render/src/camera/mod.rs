//! Cameras.
//!
//! Four of them, all pure maths over plain vectors so they can be tested
//! without a window or a GPU:
//!
//! - [`orbit`], which an asset viewer wants: yaw, pitch and zoom around a point.
//! - [`free`], which a debug fly-through wants.
//! - [`chase`], the external view the game flies behind a ship, whose parameters
//!   come from the ship's own `handlingstats.xml`.
//! - [`internal`], the cockpit view, whose parameters come from the same file.
//!
//! The last two are the ones a player cycles between with a button, and both take
//! the same [`chase::Target`] so the caller describes the ship once. Which of them
//! is live is `crate::display::CameraView` in `oag-game`, not a decision made
//! here: this crate offers cameras and picks none.
//!
//! Every view and projection matrix here comes from [`oag_core::math::camera`],
//! which is right-handed with a 0..1 depth range. That is what wgpu, Metal and
//! DX12 expect; the OpenGL -1..1 convention would put everything at the wrong
//! depth. Nothing in this module builds a matrix by hand.
//!
//! No camera takes a simulation type. The chase camera is handed a position and
//! an orientation as three vectors, so `oag-gameplay` never has to be visible
//! from here - see the dependency rules in
//! `docs/architecture/workspace-layout.md`.

pub mod chase;
pub mod free;
pub mod internal;
pub mod orbit;

use oag_core::math::{Mat4, camera};

/// The projection every camera in this crate uses.
///
/// `fov_radians` is the *vertical* field of view. The parameter is named for its
/// unit deliberately: the fov values recovered from the game's data files are of
/// unrecovered unit (see [`chase::ChaseParams::fov`]), and converting them is the
/// caller's decision, not this function's.
#[must_use]
pub fn projection(fov_radians: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    camera::perspective(fov_radians, aspect, near, far)
}

/// Adapts a vertical field of view authored for one viewport shape to another.
///
/// A field of view in a data file is only defined at the shape the authors saw.
/// The original renders into exactly one, and says nothing about any other, so
/// what a window of some different shape should show is a **presentation choice**.
/// It is made here, in one function with a test, rather than left as an
/// unexplained expression at a call site.
///
/// The choice: **at or above the authored aspect nothing changes** - the vertical
/// field is kept and a wider window simply sees more to the sides. **Below it the
/// vertical field opens up** by exactly enough to keep the horizontal field the
/// authored shape would have had.
///
/// The alternative - holding the vertical field whatever the window - crops the
/// sides away as a window narrows. A tiling compositor handing out a portrait tile
/// then leaves the ship filling a slot with the track edges outside the frame,
/// which reads as a broken camera when the camera and its data are both fine.
///
/// A degenerate `aspect` of zero or less returns the input unchanged. The result
/// is always below half a turn, since `atan` is bounded.
#[must_use]
pub fn fit_vertical_fov(fov_radians: f32, authored_aspect: f32, aspect: f32) -> f32 {
    if aspect <= 0.0 || aspect >= authored_aspect {
        return fov_radians;
    }
    // Half-angle tangents: the horizontal one is what is being held fixed, and the
    // vertical one follows from the viewport's own aspect.
    let half_horizontal = (fov_radians * 0.5).tan() * authored_aspect;
    2.0 * (half_horizontal / aspect).atan()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The PSP's screen, which is what makes this a realistic case rather than a
    /// round-number one.
    const AUTHORED: f32 = 480.0 / 272.0;

    #[test]
    fn a_viewport_of_the_authored_shape_is_left_alone() {
        let fov = 60.0f32.to_radians();
        assert_eq!(fit_vertical_fov(fov, AUTHORED, AUTHORED), fov);
    }

    #[test]
    fn a_wider_viewport_is_left_alone_and_sees_more_to_the_sides() {
        let fov = 60.0f32.to_radians();
        assert_eq!(fit_vertical_fov(fov, AUTHORED, 21.0 / 9.0), fov);
    }

    /// The property the whole function exists for.
    #[test]
    fn a_narrower_viewport_keeps_the_horizontal_field() {
        let fov = 60.0f32.to_radians();
        let authored_horizontal = (fov * 0.5).tan() * AUTHORED;

        // A portrait tile, which is what a tiling compositor hands out.
        for aspect in [1.0, 948.0 / 1152.0, 0.5] {
            let fitted = fit_vertical_fov(fov, AUTHORED, aspect);
            let horizontal = (fitted * 0.5).tan() * aspect;
            assert!(
                (horizontal - authored_horizontal).abs() < 1e-5,
                "aspect {aspect}: {horizontal} vs {authored_horizontal}"
            );
            assert!(fitted > fov, "the vertical field has to open up");
            assert!(fitted < std::f32::consts::PI);
        }
    }

    #[test]
    fn a_degenerate_viewport_changes_nothing() {
        let fov = 60.0f32.to_radians();
        assert_eq!(fit_vertical_fov(fov, AUTHORED, 0.0), fov);
        assert_eq!(fit_vertical_fov(fov, AUTHORED, -1.0), fov);
    }
}
