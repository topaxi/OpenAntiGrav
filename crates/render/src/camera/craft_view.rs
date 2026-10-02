//! The spectator director's two craft-relative views: a camera bolted to the craft it watches.
//!
//! The post-finish director (`Camera_UpdateSpectator`, `0x0887fd3c`) cuts between node cameras
//! (modes 5, 6, 7, [`super::destroy`]) and two views that ride on a craft: **mode 2**, a rigid
//! rear view that sits behind and above the craft and looks the way it flies, and **mode 3**,
//! a rigid front view that sits ahead of it and looks back at it. Both are cases of
//! `Camera_UpdateSpectatorView` (`0x08880c04`), reached through the jump table at `0x08a7cf18`
//! (case 2 at `0x0888132c`, case 3 at `0x088816e8`).
//!
//! # The law, as read and as measured
//!
//! The original works on the craft's matrix (`*(entity + 0x794)`, which equals the rigid body's
//! own matrix: unit rows, left / up / forward, then the position), and writes the camera's view
//! matrix from it. In this project's convention (columns right / up / back, `-Z` forward):
//!
//! ```text
//! mode 2 (rear):   orientation = the craft's
//!                  eye = position + 2.5 * u + 6.0 * back
//!                  u   = n / |n . up|,  n = normalize(up + (0, 0.5, 0))
//! mode 3 (front):  orientation = the craft's, turned 180 degrees about its own up
//!                  eye = position + 12.0 * forward + 3.0 * up
//! both:            vertical field of view 65 degrees, whatever the zoom was before
//! ```
//!
//! `u` is the craft's up axis tipped toward the world's up by half a unit and rescaled so it
//! stays 1 unit along the craft's own up: a rolled craft's rear camera is lifted toward the sky
//! rather than into the track. Mode 3 scales its three axes by `(0, 3, 12)` read from three
//! globals (`0x08ab10c8`, `0x08ab10cc`, `0x08ab10d0`), which only this function reads and which
//! hold those values at rest; the zero is the sideways term.
//!
//! Both arms also turn the matrix by a yaw (`camera + 0x344`) and, in mode 3, a pitch
//! (`camera + 0x340`) before offsetting, both `0.0` from the camera's constructor. The only
//! writers are the pause menu's own look-around (`InGame_UpdatePauseInput`), so the director
//! never moves them and **they are not ported**.
//!
//! # Measured
//!
//! 407 frames on PPSSPP (2026-10-02, Pulse PSP, Single Race, the camera object's mode word
//! forced to 2 and 3 while the director followed a craft after the finish): the view matrix the
//! function wrote against the matrix it read, same instant, breakpoint on its exit. The rotation
//! matched exactly and the eye to `6e-5` on every frame, in both modes. The tests below carry
//! the two most banked frames.

use oag_core::math::{Quat, Vec3};

/// The vertical field both views set, in degrees (`g_camera_fov_degrees = 65.0`).
pub const FOV_DEGREES: f32 = 65.0;

/// How far behind the craft the rear view sits.
pub const REAR_BEHIND: f32 = 6.0;
/// How far along the craft's own up the rear view is lifted.
pub const REAR_UP: f32 = 2.5;
/// What is added to the up axis's world `y` before it is normalised.
pub const REAR_LEAN: f32 = 0.5;

/// How far ahead of the craft the front view sits.
pub const FRONT_AHEAD: f32 = 12.0;
/// How far above it the front view sits.
pub const FRONT_UP: f32 = 3.0;

/// Where a camera is and which way it faces: the same pair [`super::destroy`] hands the renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// The eye, world space.
    pub eye: Vec3,
    /// The camera's orientation: `-Z` is the look direction, `Y` is up.
    pub orientation: Quat,
}

/// Mode 2, the rear view: bolted to the craft, [`REAR_BEHIND`] behind and [`REAR_UP`] above,
/// looking the way the craft flies.
#[must_use]
pub fn rear(position: Vec3, orientation: Quat) -> Pose {
    let up = orientation * Vec3::Y;
    let back = orientation * Vec3::Z;
    let mut lean = up;
    lean.y += REAR_LEAN;
    let lean = lean.normalize_or(Vec3::Y);
    // `|n . up|` is 1 for a level craft and falls as it rolls; the original divides by it with
    // no guard, and a craft at 90 degrees of roll would make `n . up` small, not zero, since
    // `n` leans toward the world's up by half a unit.
    let along_up = lean.dot(up).abs().max(f32::EPSILON);
    let lifted = lean / along_up;
    Pose {
        eye: position + lifted * REAR_UP + back * REAR_BEHIND,
        orientation,
    }
}

/// Mode 3, the front view: [`FRONT_AHEAD`] ahead of the craft and [`FRONT_UP`] above it,
/// looking back at it.
#[must_use]
pub fn front(position: Vec3, orientation: Quat) -> Pose {
    let up = orientation * Vec3::Y;
    let forward = orientation * Vec3::NEG_Z;
    Pose {
        eye: position + forward * FRONT_AHEAD + up * FRONT_UP,
        orientation: orientation * Quat::from_rotation_y(std::f32::consts::PI),
    }
}

#[cfg(test)]
mod tests;
