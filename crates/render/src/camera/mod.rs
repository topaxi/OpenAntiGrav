//! Cameras.
//!
//! Three of them, all pure maths over plain vectors so they can be tested
//! without a window or a GPU:
//!
//! - [`orbit`], which an asset viewer wants: yaw, pitch and zoom around a point.
//! - [`free`], which a debug fly-through wants.
//! - [`chase`], the external view the game flies behind a ship, whose parameters
//!   come from the ship's own `handlingstats.xml`.
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
