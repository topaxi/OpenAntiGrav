//! What a finger does to the photo camera: the gestures of
//! `oag_input::gesture`, already boiled down to a [`Touch`], turned onto the
//! lens that is up.
//!
//! **Chosen, not measured.** The picture follows the fingers, as in a 3D
//! viewer or a map (`docs/ui/photo-mode.md`, "Touch"):
//!
//! | Gesture | `close` | `track` | `free` | `internal` |
//! | --- | --- | --- | --- | --- |
//! | one-finger drag | orbit | takes the free lens | looks | takes the free lens |
//! | pinch | field of view | field of view | dollies | nothing |
//! | two-finger drag | pans the eye | nothing | slides and lifts | nothing |
//! | twist | tilts | tilts | rolls | tilts |
//!
//! This crate keeps its own plain vocabulary and does not name `oag-input`.

use super::*;

/// Radians of orbit per screen height of drag on `close`.
pub const TOUCH_ORBIT: f32 = std::f32::consts::PI;
/// Units the `close` eye is panned per screen height of two-finger drag.
pub const TOUCH_PAN: f32 = 8.0;
/// Units the free lens slides or lifts per screen height of two-finger drag.
pub const TOUCH_SLIDE: f32 = 30.0;
/// Units the free lens dollies per e-fold of pinch.
pub const TOUCH_DOLLY: f32 = 40.0;
/// Units the free lens moves per screen height of one-finger drag with the drag set to move.
pub const TOUCH_MOVE: f32 = 30.0;

/// What the fingers did this tick. Travel is a fraction of the window's height, `y` down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Touch {
    /// One-finger travel.
    pub drag: (f32, f32),
    /// Two-finger centroid travel.
    pub pan: (f32, f32),
    /// Distance ratio of the pair this tick: above 1 they spread. `0` is read as none.
    pub pinch: f32,
    /// Radians the pair turned, positive clockwise on screen.
    pub twist: f32,
}

impl Default for Touch {
    fn default() -> Self {
        Self {
            drag: (0.0, 0.0),
            pan: (0.0, 0.0),
            pinch: 1.0,
            twist: 0.0,
        }
    }
}

impl Touch {
    fn ratio(&self) -> f32 {
        if self.pinch > 0.0 { self.pinch } else { 1.0 }
    }

    fn is_none(&self) -> bool {
        *self == Self::default()
            || *self
                == Self {
                    pinch: 0.0,
                    ..Self::default()
                }
    }
}

impl Photo {
    /// A tap on the empty picture: hides the help and the panel when either is up, else shows
    /// the help.
    pub fn toggle_chrome(&mut self) {
        if self.help || self.panel {
            self.help = false;
            self.panel = false;
        } else {
            self.help = true;
        }
    }

    /// Applies one tick of fingers to the lens that is up.
    pub(super) fn apply_touch(&mut self, touch: Touch) {
        if touch.is_none() {
            return;
        }
        let ratio = touch.ratio();
        match self.lens {
            Lens::Internal | Lens::Track if touch.drag != (0.0, 0.0) => {
                // The picture does not jump: the session seeds the free lens from it.
                self.lens = Lens::Free;
                self.free = None;
                self.free_home = None;
            }
            Lens::Internal => self.tilt = wrap(self.tilt + touch.twist),
            Lens::Close => {
                self.yaw = wrap(self.yaw - touch.drag.0 * TOUCH_ORBIT);
                self.pitch = wrap(self.pitch + touch.drag.1 * TOUCH_ORBIT);
                self.pan.0 = (self.pan.0 - touch.pan.0 * TOUCH_PAN).clamp(-PAN_LIMIT, PAN_LIMIT);
                self.pan.1 = (self.pan.1 + touch.pan.1 * TOUCH_PAN).clamp(-PAN_LIMIT, PAN_LIMIT);
                self.close_fov = (self.close_fov / ratio).clamp(CLOSE_FOV.0, CLOSE_FOV.1);
                self.tilt = wrap(self.tilt + touch.twist);
            }
            Lens::Track => {
                self.track_fov = (self.track_fov / ratio).clamp(TRACK_FOV.0, TRACK_FOV.1);
                self.tilt = wrap(self.tilt + touch.twist);
            }
            Lens::Free => self.fly_touch(touch, ratio),
        }
    }

    fn fly_touch(&mut self, touch: Touch, ratio: f32) {
        let Some(mut rig) = self.free else {
            return;
        };
        let look = rig.fov.to_radians();
        let (mut yaw, mut pitch) = (0.0, 0.0);
        let (mut strafe, mut advance) = (0.0, ratio.ln() * TOUCH_DOLLY);
        if self.drag_moves {
            strafe -= touch.drag.0 * TOUCH_MOVE;
            advance += touch.drag.1 * TOUCH_MOVE;
        } else {
            yaw = -touch.drag.0 * look;
            pitch = touch.drag.1 * look;
        }
        strafe -= touch.pan.0 * TOUCH_SLIDE;
        rig.cam = rig.cam.advance(
            &Motion {
                advance,
                strafe,
                lift: touch.pan.1 * TOUCH_SLIDE,
                yaw,
                pitch,
                roll: -touch.twist,
            },
            1.0,
        );
        rig.cam.position = rig.cam.position.clamp_length_max(rig.reach);
        self.free = Some(rig);
    }
}

#[cfg(test)]
mod tests;
