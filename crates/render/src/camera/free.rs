//! The free camera: fly anywhere, look anywhere.
//!
//! For inspecting a scene rather than for playing: parking the eye inside a
//! tunnel to see whether the geometry closes, or following a spline by hand. It
//! reproduces nothing from the original game and claims nothing about it.
//!
//! Angles use the same convention as [`super::orbit`]: yaw sweeps around the
//! world Y axis and pitch lifts out of the XZ plane, so yaw and pitch both zero
//! looks along +X with world +Y up.

use oag_core::math::{Mat4, Vec3, camera};

/// Reuses the orbit camera's limit, for the same reason: at exactly straight up
/// or down the `look_at` up vector degenerates and the view flips.
pub use super::orbit::PITCH_LIMIT;

/// How fast a free camera is being asked to move, this frame.
///
/// Translations are world units per second and rotations radians per second, so
/// a caller scales by its own key-repeat or stick deflection and this module
/// stays free of input concepts. All zero is a camera standing still.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Motion {
    /// Along the view direction, positive forwards.
    pub advance: f32,
    /// Across the view direction, positive to the camera's right.
    pub strafe: f32,
    /// Along world +Y. Deliberately not the camera's own up, so holding "rise"
    /// while pitched down does not walk the camera forwards as well.
    pub rise: f32,
    /// Radians per second, positive turning the same way as
    /// [`super::orbit::Held::yaw_pos`].
    pub yaw: f32,
    /// Radians per second, positive tilting the view upwards.
    pub pitch: f32,
}

/// A camera positioned and aimed by hand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Free {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

impl Free {
    /// A camera at `position` looking along +X.
    #[must_use]
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    /// The unit view direction for the current yaw and pitch.
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
    }

    /// The camera's right, in the horizontal plane.
    ///
    /// Zero-length only if the pitch limit were removed and the view went exactly
    /// vertical, which is why the limit exists.
    #[must_use]
    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize_or_zero()
    }

    /// The view matrix, right-handed with world +Y up.
    #[must_use]
    pub fn view(&self) -> Mat4 {
        camera::look_at(self.position, self.position + self.forward(), Vec3::Y)
    }

    /// Moves and turns the camera by `dt` seconds of `motion`.
    ///
    /// Pure, and pitch clamps to [`PITCH_LIMIT`], so both can be checked without
    /// a window or a GPU. Turning is applied before translating, so a frame that
    /// both turns and advances travels along the new heading; at 60 Hz the
    /// difference is invisible, but picking one keeps the result reproducible.
    #[must_use]
    pub fn advance(self, motion: &Motion, dt: f32) -> Self {
        let turned = Self {
            position: self.position,
            yaw: self.yaw + motion.yaw * dt,
            pitch: (self.pitch + motion.pitch * dt).clamp(-PITCH_LIMIT, PITCH_LIMIT),
        };

        let step = turned.forward() * (motion.advance * dt)
            + turned.right() * (motion.strafe * dt)
            + Vec3::Y * (motion.rise * dt);

        Self {
            position: turned.position + step,
            ..turned
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_camera_looks_along_plus_x() {
        let camera = Free::new(Vec3::ZERO);
        assert!((camera.forward() - Vec3::X).length() < 1e-6);
    }

    #[test]
    fn zero_motion_leaves_the_camera_where_it_was() {
        let start = Free {
            position: Vec3::new(1.0, 2.0, 3.0),
            yaw: 0.4,
            pitch: 0.1,
        };
        assert_eq!(start.advance(&Motion::default(), 1.0), start);
    }

    #[test]
    fn zero_dt_leaves_the_camera_where_it_was_however_hard_it_is_pushed() {
        let start = Free {
            position: Vec3::new(1.0, 2.0, 3.0),
            yaw: 0.4,
            pitch: 0.1,
        };
        let motion = Motion {
            advance: 100.0,
            strafe: 100.0,
            rise: 100.0,
            yaw: 10.0,
            pitch: 10.0,
        };
        assert_eq!(start.advance(&motion, 0.0), start);
    }

    #[test]
    fn advancing_moves_along_the_view_direction() {
        let start = Free::new(Vec3::ZERO);
        let moved = start.advance(
            &Motion {
                advance: 2.0,
                ..Motion::default()
            },
            0.5,
        );
        // One world unit along +X, which is where a fresh camera looks.
        assert!((moved.position - Vec3::X).length() < 1e-6, "{moved:?}");
    }

    #[test]
    fn strafing_moves_sideways_and_not_forwards() {
        let start = Free::new(Vec3::ZERO);
        let moved = start.advance(
            &Motion {
                strafe: 1.0,
                ..Motion::default()
            },
            1.0,
        );
        assert!(moved.position.dot(start.forward()).abs() < 1e-6);
        assert!((moved.position.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn rising_is_along_world_up_even_when_pitched() {
        let start = Free {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: -1.0,
        };
        let moved = start.advance(
            &Motion {
                rise: 3.0,
                ..Motion::default()
            },
            1.0,
        );
        assert!((moved.position - Vec3::new(0.0, 3.0, 0.0)).length() < 1e-6);
    }

    #[test]
    fn pitch_clamps_short_of_vertical_however_long_held() {
        let start = Free::new(Vec3::ZERO);
        let up = start.advance(
            &Motion {
                pitch: 1.0,
                ..Motion::default()
            },
            1000.0,
        );
        assert_eq!(up.pitch, PITCH_LIMIT);
        let down = start.advance(
            &Motion {
                pitch: -1.0,
                ..Motion::default()
            },
            1000.0,
        );
        assert_eq!(down.pitch, -PITCH_LIMIT);
    }
}
