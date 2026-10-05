//! The orbit camera: yaw, pitch and zoom around a fixed point.
//!
//! What an asset viewer wants, and useless in a race. It is here rather than in
//! `oag-view` because it is camera maths and belongs with the other cameras, and
//! because [`advance`] is a pure function of the held keys and `dt` and so can be
//! tested without a window or a GPU. The window, the event loop and the mapping
//! from a real keyboard to [`Held`] stay in the viewer.
//!
//! The framing itself - how far out the eye sits for a given zoom - lives with
//! the pipeline in [`crate::mesh_render`], because it is derived from the model's
//! own bounding sphere. [`Orbit::pan`] is in those same units for the same
//! reason: this module has no idea how big the thing being looked at is, and
//! keeping it that way is what lets every function here be tested without one.

use oag_core::math::Vec3;

/// Radians per second the arrow keys rotate the camera.
pub const ROTATE_RATE: f32 = 1.2;

/// Zoom multiplier change per second while a zoom key is held.
pub const ZOOM_RATE: f32 = 0.8;

/// Bounding-sphere radii per second the pan keys move the look-at point.
///
/// In radii rather than world units, so panning covers the same fraction of any
/// model in the same time - which is the point, given the things this viewer
/// opens run from a 13-unit ship to a 2,370-unit circuit.
pub const PAN_RATE: f32 = 0.9;

/// Keeps the camera short of straight down or up, where `look_at`'s up vector
/// degenerates and the orbit flips.
pub const PITCH_LIMIT: f32 = 1.5;

/// How close and how far the zoom multiplier is allowed to go. 1.0 is the
/// bounding-sphere framing [`crate::mesh_render`] uses by default.
pub const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.3..=4.0;

/// How far the look-at point may be dragged from the model's own centre, in
/// bounding-sphere radii.
///
/// A limit rather than none, because the eye is placed *relative* to the
/// look-at point: pan far enough with no clamp and the model leaves the frustum
/// with nothing on screen to say which way to come back. Three radii reaches
/// well past any corner of the bounding sphere and still keeps a return
/// plausible.
pub const PAN_LIMIT: f32 = 3.0;

/// Which camera keys are currently down.
#[derive(Debug, Default)]
pub struct Held {
    pub yaw_neg: bool,
    pub yaw_pos: bool,
    pub pitch_neg: bool,
    pub pitch_pos: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub pan_left: bool,
    pub pan_right: bool,
    pub pan_up: bool,
    pub pan_down: bool,
}

/// The orbit camera's yaw, pitch, zoom and look-at point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    /// Where the camera is looking, offset from the model's own centre, in
    /// **bounding-sphere radii**.
    ///
    /// Stored as the accumulated 3-D offset rather than as the two screen-plane
    /// numbers that produced it, so rotating after panning swings the eye
    /// around the point you panned to instead of dragging that point with the
    /// view. That is what makes it possible to pick a corner of a circuit and
    /// then look at it from anywhere.
    pub pan: Vec3,
}

impl Default for Orbit {
    /// The default framing: looking at the model's own centre from `yaw` and
    /// `pitch` zero, at the bounding-sphere distance.
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
            pan: Vec3::ZERO,
        }
    }
}

impl Orbit {
    /// The camera's own right and up axes at this yaw and pitch.
    ///
    /// The screen plane, in world space, which is what panning has to move
    /// along for a drag to go where the cursor went. Derived from the eye
    /// direction [`crate::mesh_render`] builds, so the two cannot drift: `right`
    /// is the horizon-parallel axis and `up` completes the frame.
    ///
    /// Well defined for every pitch [`advance`] permits - `right` divides out a
    /// `cos(pitch)` that [`PITCH_LIMIT`] keeps away from zero.
    #[must_use]
    pub fn basis(&self) -> (Vec3, Vec3) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let right = Vec3::new(sin_yaw, 0.0, -cos_yaw);
        let up = Vec3::new(-cos_yaw * sin_pitch, cos_pitch, -sin_yaw * sin_pitch);
        (right, up)
    }

    /// Moves the look-at point by `right` and `up` radii along the screen
    /// plane, clamped to [`PAN_LIMIT`].
    #[must_use]
    pub fn panned(self, right: f32, up: f32) -> Self {
        let (right_axis, up_axis) = self.basis();
        let pan = self.pan + right_axis * right + up_axis * up;
        Self {
            pan: pan.clamp_length_max(PAN_LIMIT),
            ..self
        }
    }

    /// Puts the look-at point back on the model's centre, leaving the angle
    /// and the zoom alone.
    #[must_use]
    pub fn recentred(self) -> Self {
        Self {
            pan: Vec3::ZERO,
            ..self
        }
    }
}

/// Advances `orbit` by `dt` seconds according to which keys are held.
///
/// Pitch clamps short of straight down or up, where [`crate::mesh_render`]'s
/// `look_at` up vector degenerates and the orbit flips. Zoom clamps to
/// [`ZOOM_RANGE`]. Pure so the clamping can be tested without a window or a GPU.
#[must_use]
pub fn advance(orbit: Orbit, held: &Held, dt: f32) -> Orbit {
    let mut yaw = orbit.yaw;
    let mut pitch = orbit.pitch;
    let mut zoom = orbit.zoom;

    if held.yaw_neg {
        yaw -= ROTATE_RATE * dt;
    }
    if held.yaw_pos {
        yaw += ROTATE_RATE * dt;
    }
    if held.pitch_pos {
        pitch += ROTATE_RATE * dt;
    }
    if held.pitch_neg {
        pitch -= ROTATE_RATE * dt;
    }
    pitch = pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);

    if held.zoom_in {
        zoom -= ZOOM_RATE * dt;
    }
    if held.zoom_out {
        zoom += ZOOM_RATE * dt;
    }
    zoom = zoom.clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());

    let step = PAN_RATE * dt;
    let right = f32::from(held.pan_right) - f32::from(held.pan_left);
    let up = f32::from(held.pan_up) - f32::from(held.pan_down);
    // Panned *after* the rotation, so a frame that both turns and pans moves
    // along the plane the viewer is about to see rather than the one it just
    // left. Either order is defensible; this one is the one that does not lag.
    Orbit {
        yaw,
        pitch,
        zoom,
        pan: orbit.pan,
    }
    .panned(right * step, up * step)
}

#[cfg(test)]
mod tests;
