//! The chase camera: the external view that flies behind a ship.
//!
//! Its seven parameters are **data, not code**. They come from the ship's own
//! `Data\Ships\<Team>\handlingstats.xml`, which carries two blocks of exactly
//! this shape - `<ExternalCameraFar>` and `<ExternalCameraClose>`, the two
//! external distances the player can toggle between - each with
//! `fov lookat_height lookat_length pos_height pos_length spring_horiz
//! spring_vert`. See `docs/formats/handling-stats.md` for the schema.
//!
//! [`ChaseParams`] therefore has **no `Default`**, and this module hardcodes no
//! value. Every number comes from the caller, which reads it from the user's own
//! disc. A plausible-looking default here would be indistinguishable, later, from
//! a value actually recovered from the game, and that is exactly the confusion
//! the project's confidence rules exist to prevent.
//!
//! # What is a reading of the data and what is this module's own choice
//!
//! The parameter names constrain the *shape* of the camera but not all of its
//! behaviour. Recorded here rather than left implicit in the code:
//!
//! - **Only the eye is sprung.** There are two spring values and they sit
//!   alongside `pos_*`, with nothing of the sort next to `lookat_*`, so the
//!   look-at point is computed rigidly from the ship every frame and only the eye
//!   lags. That is a reading of the parameter set, not a verified observation.
//! - **The horizontal/vertical split is taken in the ship's frame**, along the
//!   `up` vector passed in, not along world Y. Pulse tracks roll and fully
//!   invert, so world Y stops meaning "up" for much of a lap and a world-space
//!   split would make the two springs swap roles upside down. Which one the
//!   original used is unrecovered.
//! - **`pos_length` is behind and `lookat_length` ahead.** An external camera
//!   trails the ship and aims down the track in front of it. A sign error here
//!   would put the camera in front of the ship looking back at it, which is
//!   obvious on screen, so this one is cheap to check once there is a ship to
//!   look at.
//! - **The integrator is a first-order lag**, `eye += error * (rate * dt)` with
//!   the factor clamped to 1, chosen here and not recovered. It uses no
//!   transcendental function, so unlike `1 - exp(-rate * dt)` it gives the same
//!   result on every target - which matters if a replay ever records the camera.
//!   The cost is that the response is not exactly frame-rate independent; at the
//!   fixed 60 Hz this project runs the simulation at (ADR-0007), nothing varies
//!   anyway.
//! - **The unit of `spring_horiz` / `spring_vert` is unrecovered.** They are
//!   treated as a rate per second. If the original's are per-frame at its own
//!   variable timestep, the numbers will need a factor, and the shape here does
//!   not change.

use oag_core::math::{Mat4, Vec3, camera};

/// The seven values one `<ExternalCameraFar>` or `<ExternalCameraClose>` block
/// carries.
///
/// Deliberately not `Default`: see the module documentation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChaseParams {
    /// Field of view, **as authored**. The file's unit (degrees or radians) is
    /// unrecovered, so nothing here converts it; a caller that wants a projection
    /// matrix decides what it means and passes radians to
    /// [`super::projection`].
    pub fov: f32,
    /// How far above the ship the camera aims, along the ship's up.
    pub lookat_height: f32,
    /// How far ahead of the ship the camera aims, along the ship's forward.
    pub lookat_length: f32,
    /// How far above the ship the eye sits, along the ship's up.
    pub pos_height: f32,
    /// How far behind the ship the eye sits, against the ship's forward.
    pub pos_length: f32,
    /// Rate, per second, at which the eye closes the part of its error that lies
    /// across the ship's up axis.
    pub spring_horiz: f32,
    /// Rate, per second, at which the eye closes the part of its error that lies
    /// along the ship's up axis.
    pub spring_vert: f32,
}

/// Where the thing being followed is and how it is oriented.
///
/// Three vectors rather than a ship, so that no gameplay type is visible from the
/// renderer. `forward` and `up` are expected to be unit length and perpendicular;
/// neither is normalised here, because a caller holding an orthonormal frame
/// should not pay for a normalise every frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
}

/// Where the eye would sit with no spring at all: the rigid offset the spring
/// chases.
#[must_use]
pub fn anchor(target: Target, params: &ChaseParams) -> Vec3 {
    target.position + target.up * params.pos_height - target.forward * params.pos_length
}

/// The point the camera aims at, which is rigid: it is recomputed from the ship
/// every frame and never lags.
#[must_use]
pub fn look_at_point(target: Target, params: &ChaseParams) -> Vec3 {
    target.position + target.up * params.lookat_height + target.forward * params.lookat_length
}

/// The external camera. Its only state is where the eye currently is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chase {
    eye: Vec3,
}

impl Chase {
    /// A camera already settled on its rigid offset, with no lag to work off.
    ///
    /// What a race start or a respawn wants: springing in from wherever the eye
    /// happened to be last would sweep the camera across the world.
    #[must_use]
    pub fn snapped(target: Target, params: &ChaseParams) -> Self {
        Self {
            eye: anchor(target, params),
        }
    }

    /// A camera whose eye starts at an explicit point.
    #[must_use]
    pub fn at(eye: Vec3) -> Self {
        Self { eye }
    }

    /// Where the eye currently is.
    #[must_use]
    pub fn eye(&self) -> Vec3 {
        self.eye
    }

    /// Moves the eye `dt` seconds closer to the rigid offset behind `target`.
    ///
    /// The error is split along `target.up` and across it, and the two halves
    /// close at `spring_vert` and `spring_horiz` respectively. Pure enough to test
    /// without a window or a GPU: the only state is the eye.
    pub fn advance(&mut self, target: Target, params: &ChaseParams, dt: f32) {
        let error = anchor(target, params) - self.eye;

        // `normalize_or_zero` rather than a branch on a degenerate frame: with a
        // zero up vector the vertical part comes out zero and the whole error is
        // treated as horizontal, which is the harmless reading.
        let up = target.up.normalize_or_zero();
        let vertical = up * error.dot(up);
        let horizontal = error - vertical;

        self.eye +=
            horizontal * lag(params.spring_horiz, dt) + vertical * lag(params.spring_vert, dt);
    }

    /// The view matrix for the current eye, aimed at the rigid look-at point.
    ///
    /// `target.up` is the up vector, so the horizon rolls with the ship, which is
    /// what the original's external view does through a barrel roll.
    #[must_use]
    pub fn view(&self, target: Target, params: &ChaseParams) -> Mat4 {
        camera::look_at(self.eye, look_at_point(target, params), target.up)
    }
}

/// The fraction of its error a first-order lag closes in `dt` seconds at `rate`.
///
/// Clamped to 1 so a large `dt` or a large rate settles on the target instead of
/// overshooting into an oscillation, and to 0 so a negative rate in a data file
/// cannot push the camera away from the ship.
fn lag(rate: f32, dt: f32) -> f32 {
    (rate * dt).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ship at the origin facing +X, upright. No parameter here claims to be
    /// the game's; they are round numbers picked to make the geometry readable.
    fn target() -> Target {
        Target {
            position: Vec3::ZERO,
            forward: Vec3::X,
            up: Vec3::Y,
        }
    }

    fn params() -> ChaseParams {
        ChaseParams {
            fov: 1.0,
            lookat_height: 1.0,
            lookat_length: 10.0,
            pos_height: 2.0,
            pos_length: 8.0,
            spring_horiz: 4.0,
            spring_vert: 2.0,
        }
    }

    #[test]
    fn a_snapped_camera_sits_behind_and_above_the_ship() {
        let camera = Chase::snapped(target(), &params());
        assert_eq!(camera.eye(), Vec3::new(-8.0, 2.0, 0.0));
    }

    #[test]
    fn the_look_at_point_is_ahead_of_the_ship_whatever_the_eye_is_doing() {
        let params = params();
        let ahead = look_at_point(target(), &params);
        assert_eq!(ahead, Vec3::new(10.0, 1.0, 0.0));
        // Rigid: moving the eye must not move what the camera aims at.
        assert_eq!(look_at_point(target(), &params), ahead);
    }

    #[test]
    fn zero_dt_leaves_the_eye_where_it_was() {
        let mut camera = Chase::at(Vec3::new(100.0, 100.0, 100.0));
        let before = camera.eye();
        camera.advance(target(), &params(), 0.0);
        assert_eq!(camera.eye(), before);
    }

    #[test]
    fn a_settled_camera_stays_settled() {
        let params = params();
        let mut camera = Chase::snapped(target(), &params);
        for _ in 0..600 {
            camera.advance(target(), &params, 1.0 / 60.0);
        }
        assert!((camera.eye() - anchor(target(), &params)).length() < 1e-4);
    }

    #[test]
    fn the_eye_converges_on_the_anchor_from_anywhere() {
        let params = params();
        let target = target();
        let mut camera = Chase::at(Vec3::new(50.0, -30.0, 20.0));
        let start = (camera.eye() - anchor(target, &params)).length();

        let mut previous = start;
        for _ in 0..600 {
            camera.advance(target, &params, 1.0 / 60.0);
            let error = (camera.eye() - anchor(target, &params)).length();
            // Monotone: a first-order lag never overshoots.
            assert!(error <= previous, "error grew from {previous} to {error}");
            previous = error;
        }
        assert!(previous < start * 0.01, "{start} to {previous}");
    }

    #[test]
    fn the_two_springs_act_on_their_own_axis_of_the_ships_frame() {
        // Vertical spring off, horizontal on: the eye must close its error across
        // the ship's up and keep all of its error along it.
        let params = ChaseParams {
            spring_vert: 0.0,
            ..params()
        };
        let target = target();
        let mut camera = Chase::at(anchor(target, &params) + Vec3::new(10.0, 10.0, 10.0));
        for _ in 0..600 {
            camera.advance(target, &params, 1.0 / 60.0);
        }
        let error = camera.eye() - anchor(target, &params);
        assert!(
            (error.y - 10.0).abs() < 1e-4,
            "vertical error moved: {error}"
        );
        assert!(error.x.abs() < 1e-3 && error.z.abs() < 1e-3, "{error}");
    }

    #[test]
    fn an_inverted_ship_springs_along_its_own_up_not_the_worlds() {
        // Rolled 180 degrees, so the ship's up is world -Y. With the vertical
        // spring off, the frozen axis must follow the ship, not the world.
        let params = ChaseParams {
            spring_vert: 0.0,
            ..params()
        };
        let target = Target {
            position: Vec3::ZERO,
            forward: Vec3::X,
            up: -Vec3::Y,
        };
        let mut camera = Chase::at(anchor(target, &params) + Vec3::new(0.0, 10.0, 10.0));
        for _ in 0..600 {
            camera.advance(target, &params, 1.0 / 60.0);
        }
        let error = camera.eye() - anchor(target, &params);
        assert!((error.y - 10.0).abs() < 1e-4, "{error}");
        assert!(error.z.abs() < 1e-3, "{error}");
    }
}
