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
//! own bounding sphere.

/// Radians per second the arrow keys rotate the camera.
pub const ROTATE_RATE: f32 = 1.2;

/// Zoom multiplier change per second while a zoom key is held.
pub const ZOOM_RATE: f32 = 0.8;

/// Keeps the camera short of straight down or up, where `look_at`'s up vector
/// degenerates and the orbit flips.
pub const PITCH_LIMIT: f32 = 1.5;

/// How close and how far the zoom multiplier is allowed to go. 1.0 is the
/// bounding-sphere framing [`crate::mesh_render`] uses by default.
pub const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.3..=4.0;

/// Which camera keys are currently down.
#[derive(Debug, Default)]
pub struct Held {
    pub yaw_neg: bool,
    pub yaw_pos: bool,
    pub pitch_neg: bool,
    pub pitch_pos: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
}

/// The orbit camera's yaw, pitch and zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
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

    Orbit { yaw, pitch, zoom }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(set: impl FnOnce(&mut Held)) -> Held {
        let mut held = Held::default();
        set(&mut held);
        held
    }

    #[test]
    fn no_keys_held_leaves_the_camera_still() {
        let start = Orbit {
            yaw: 0.1,
            pitch: 0.2,
            zoom: 1.5,
        };
        assert_eq!(advance(start, &Held::default(), 1.0), start);
    }

    #[test]
    fn zero_dt_leaves_the_camera_still_even_with_keys_held() {
        let start = Orbit {
            yaw: 0.1,
            pitch: 0.2,
            zoom: 1.5,
        };
        let held = held(|h| {
            h.yaw_pos = true;
            h.pitch_pos = true;
            h.zoom_in = true;
        });
        assert_eq!(advance(start, &held, 0.0), start);
    }

    #[test]
    fn yaw_left_and_right_move_opposite_ways() {
        let start = Orbit {
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
        };
        let right = advance(start, &held(|h| h.yaw_pos = true), 1.0);
        let left = advance(start, &held(|h| h.yaw_neg = true), 1.0);
        assert!(right.yaw > start.yaw);
        assert!(left.yaw < start.yaw);
        assert_eq!(right.yaw, -left.yaw);
    }

    #[test]
    fn opposite_keys_held_together_cancel() {
        let start = Orbit {
            yaw: 0.3,
            pitch: 0.0,
            zoom: 1.0,
        };
        let held = held(|h| {
            h.yaw_neg = true;
            h.yaw_pos = true;
        });
        assert_eq!(advance(start, &held, 1.0), start);
    }

    #[test]
    fn pitch_clamps_short_of_vertical_however_long_held() {
        let start = Orbit {
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
        };
        let up = advance(start, &held(|h| h.pitch_pos = true), 1000.0);
        assert_eq!(up.pitch, PITCH_LIMIT);
        let down = advance(start, &held(|h| h.pitch_neg = true), 1000.0);
        assert_eq!(down.pitch, -PITCH_LIMIT);
    }

    #[test]
    fn zoom_clamps_to_its_range_however_long_held() {
        let start = Orbit {
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
        };
        let close = advance(start, &held(|h| h.zoom_in = true), 1000.0);
        assert_eq!(close.zoom, *ZOOM_RANGE.start());
        let far = advance(start, &held(|h| h.zoom_out = true), 1000.0);
        assert_eq!(far.zoom, *ZOOM_RANGE.end());
    }
}
