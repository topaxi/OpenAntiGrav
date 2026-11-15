//! The free camera: fly anywhere, look anywhere.
//!
//! For inspecting a scene rather than for playing: parking the eye inside a
//! tunnel to see whether the geometry closes, or framing a photograph from
//! anywhere about the craft (`oag_raceplay::photo`). It reproduces nothing from
//! the original game and claims nothing about it.
//!
//! Angles use the same convention as `oag_mesh::orbit`: yaw sweeps around the
//! world Y axis and pitch lifts out of the XZ plane, so yaw and pitch both zero
//! looks along +X with world +Y up. Roll turns the view about its own axis. The
//! camera converts to and from the `-Z` is forward, `Y` is up orientation every
//! override and ship body uses ([`Free::orientation`], [`Free::from_orientation`]).

use oag_core::math::{Mat3, Mat4, Quat, Vec3, camera};

// Reuses the orbit camera's limit, for the same reason: at exactly straight up
// or down the `look_at` up vector degenerates and the view flips.
use oag_mesh::orbit::PITCH_LIMIT;

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
    /// Along the camera's own up, positive lifting the eye. Rolled, it lifts
    /// along the rolled up, so it is the same axis the picture shows as up.
    pub lift: f32,
    /// Radians per second, positive turning the same way as
    /// `oag_mesh::orbit::Held::yaw_pos`.
    pub yaw: f32,
    /// Radians per second, positive tilting the view upwards.
    pub pitch: f32,
    /// Radians per second about the view direction, positive turning the
    /// horizon anticlockwise on screen.
    pub roll: f32,
}

/// A camera positioned and aimed by hand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Free {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    /// Radians about the view direction; zero keeps the camera's right level.
    pub roll: f32,
}

impl Free {
    /// A camera at `position` looking along +X, level.
    #[must_use]
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
        }
    }

    /// The camera at `position` with `orientation` (`-Z` forward, `Y` up), to the bit it can
    /// be expressed in: a pitch past [`PITCH_LIMIT`] is brought back to it.
    #[must_use]
    pub fn from_orientation(position: Vec3, orientation: Quat) -> Self {
        let forward = orientation * Vec3::NEG_Z;
        let up = orientation * Vec3::Y;
        let yaw = forward.z.atan2(forward.x);
        let pitch = forward
            .y
            .clamp(-1.0, 1.0)
            .asin()
            .clamp(-PITCH_LIMIT, PITCH_LIMIT);
        let level = Self {
            position,
            yaw,
            pitch,
            roll: 0.0,
        };
        let right = level.right();
        let level_up = right.cross(level.forward());
        Self {
            roll: up.dot(right).atan2(up.dot(level_up)),
            ..level
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

    /// The camera's right, in the horizontal plane before roll is applied.
    fn level_right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize_or_zero()
    }

    /// The camera's up: the horizon-level up turned by [`Self::roll`] about the view direction.
    #[must_use]
    pub fn up(&self) -> Vec3 {
        let right = self.level_right();
        let level_up = right.cross(self.forward());
        level_up * self.roll.cos() + right * self.roll.sin()
    }

    /// The camera's right, across the view and square to [`Self::up`].
    ///
    /// Zero-length only if the pitch limit were removed and the view went exactly
    /// vertical, which is why the limit exists.
    #[must_use]
    pub fn right(&self) -> Vec3 {
        self.forward().cross(self.up()).normalize_or_zero()
    }

    /// The camera's orientation in the override's convention: `-Z` forward, `Y` up.
    #[must_use]
    pub fn orientation(&self) -> Quat {
        let (right, up, forward) = (self.right(), self.up(), self.forward());
        Quat::from_mat3(&Mat3::from_cols(right, up, -forward))
    }

    /// The view matrix, right-handed with the camera's own up.
    #[must_use]
    pub fn view(&self) -> Mat4 {
        camera::look_at(self.position, self.position + self.forward(), self.up())
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
            roll: self.roll + motion.roll * dt,
        };

        let step = turned.forward() * (motion.advance * dt)
            + turned.right() * (motion.strafe * dt)
            + turned.up() * (motion.lift * dt);

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
            roll: 0.0,
        };
        assert_eq!(start.advance(&Motion::default(), 1.0), start);
    }

    #[test]
    fn zero_dt_leaves_the_camera_where_it_was_however_hard_it_is_pushed() {
        let start = Free {
            position: Vec3::new(1.0, 2.0, 3.0),
            yaw: 0.4,
            pitch: 0.1,
            roll: 0.0,
        };
        let motion = Motion {
            advance: 100.0,
            strafe: 100.0,
            lift: 100.0,
            yaw: 10.0,
            pitch: 10.0,
            roll: 10.0,
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
    fn lifting_is_along_the_cameras_own_up() {
        let start = Free {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: -1.0,
            roll: 0.0,
        };
        let moved = start.advance(
            &Motion {
                lift: 3.0,
                ..Motion::default()
            },
            1.0,
        );
        let expected = start.up() * 3.0;
        assert!((moved.position - expected).length() < 1e-5, "{moved:?}");
        // Pitched down, the camera's up leans forward along +X: it is not world up.
        assert!(moved.position.x > 1.0, "{moved:?}");
    }

    #[test]
    fn a_rolled_camera_strafes_along_its_rolled_right() {
        let start = Free {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            roll: std::f32::consts::FRAC_PI_2,
        };
        let moved = start.advance(
            &Motion {
                strafe: 2.0,
                ..Motion::default()
            },
            1.0,
        );
        // Rolled a quarter turn, the camera's right is world down.
        assert!(
            (moved.position - Vec3::new(0.0, -2.0, 0.0)).length() < 1e-5,
            "{moved:?}"
        );
    }

    #[test]
    fn the_orientation_looks_down_minus_z_with_y_up_and_round_trips() {
        let level = Free::new(Vec3::ZERO);
        let o = level.orientation();
        assert!((o * Vec3::NEG_Z - level.forward()).length() < 1e-5);
        assert!((o * Vec3::Y - Vec3::Y).length() < 1e-5);
        for (yaw, pitch, roll) in [(0.3, 0.2, 0.0), (-2.0, -0.7, 0.9), (3.0, 1.2, -2.5)] {
            let camera = Free {
                position: Vec3::new(4.0, 5.0, 6.0),
                yaw,
                pitch,
                roll,
            };
            let back = Free::from_orientation(camera.position, camera.orientation());
            assert!(
                (back.forward() - camera.forward()).length() < 1e-4,
                "{camera:?} {back:?}"
            );
            assert!(
                (back.up() - camera.up()).length() < 1e-4,
                "{camera:?} {back:?}"
            );
            assert!(
                (back.orientation() * Vec3::X - camera.orientation() * Vec3::X).length() < 1e-4
            );
        }
    }

    #[test]
    fn the_view_matrix_agrees_with_the_orientation() {
        let camera = Free {
            position: Vec3::new(1.0, 2.0, 3.0),
            yaw: 0.8,
            pitch: -0.3,
            roll: 0.4,
        };
        let world = Mat4::from_rotation_translation(camera.orientation(), camera.position);
        let expected = world.inverse();
        let view = camera.view();
        for p in [Vec3::new(5.0, 1.0, -2.0), Vec3::new(-3.0, 4.0, 9.0)] {
            assert!((view.transform_point3(p) - expected.transform_point3(p)).length() < 1e-3);
        }
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
