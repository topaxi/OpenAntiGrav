//! The chase rig's rigid geometry: the parameters one external camera block
//! authors and the points they put the eye and the aim at, with no lag.
//!
//! The simulation needs these as well as the picture: the lock-on projects a
//! target through a virtual chase camera that never lags (`oag_raceplay`'s
//! `lock`), and the locks are hashed state. So they live here, below the
//! renderer, and the sprung camera that chases them is
//! `oag_render::camera::chase::Chase`, whose module documentation records what
//! is read off the original and what is this project's choice. Plain `Vec3`
//! arithmetic, no transcendental.

use oag_core::math::Vec3;

/// The seven values one `<ExternalCameraFar>` or `<ExternalCameraClose>` block
/// carries, plus the one scale the original applies to a rig from outside it.
///
/// Deliberately not `Default`: see `oag_render::camera::chase`'s module
/// documentation, which is the rest of this rig.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChaseParams {
    /// The **eighth** value, and the only one here that is not in the block:
    /// `g_craft_scale` (`0x08ab0e1c`), a literal `0.75` written once by the craft
    /// constructor and read by five unrelated consumers as a global scale on
    /// craft-space geometry. `oag_physics::hover::TARGET_GLOBAL_SCALE` is the
    /// same number, and `1.0` turns this off.
    ///
    /// # Why it is a rig parameter and not four pre-multiplied offsets
    ///
    /// The original ends both external rigs with
    ///
    /// ```text
    /// eye  = shipPos + (eye  - shipPos) * g_craft_scale;
    /// look = shipPos + (look - shipPos) * g_craft_scale;
    /// ```
    ///
    /// **after** writing the sprung eye back to its own state. `camera.md`
    /// recorded that as "the same thing as scaling all four offsets", and for the
    /// look-at point - which is rigid - it is. For the eye it is **not**, and the
    /// difference is not academic: the spring carries state across ticks while
    /// `shipPos` moves under it, so a state kept in scaled space is a different
    /// state, not the same one in other units. Measured against the original over
    /// `data/traces/pad0-boost.csv`: pre-multiplying the offsets lands at
    /// `0.121` RMS, applying the scale here at `0.008`. Both are far better than
    /// the free spring's `4.938`, which is why the error was invisible until the
    /// rigid radius made the rest of the model right.
    pub craft_scale: f32,
    /// Field of view, **as authored**, in **vertical degrees** at the authored
    /// 480x272 aspect - settled 2026-08-09 at confidence 94 by reading
    /// `g_camera_fov_degrees` (`0x08b34310`) off the running game. Nothing here
    /// converts it anyway: a caller that wants a projection matrix passes
    /// radians to `oag_render::camera::projection`, which keeps the conversion at a call
    /// site rather than assumed in a library.
    ///
    /// *This field used to say the unit was unrecovered.* It was, from M4 until
    /// that read; see `docs/rendering/projection-vs-the-original.md`.
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

/// Shrinks a craft-space point toward the craft by
/// [`ChaseParams::craft_scale`], which is the last thing the original's rig does
/// to both of the points it hands the tripod.
#[must_use]
pub fn scaled_about(target: Target, params: &ChaseParams, point: Vec3) -> Vec3 {
    target.position + (point - target.position) * params.craft_scale
}

/// Where the eye ends up with no lag left to work off: both offsets applied,
/// and the craft scale with them, so this is a world position.
///
/// **Not what the spring chases** - see [`spring_anchor`], which drops the
/// vertical term because the original applies it after the spring rather than
/// before, and drops the scale because the original applies that after the
/// spring too. The two are the same point once the spring has settled, which is
/// why this is still the right answer for `oag_render::camera::chase::Chase::snapped` and for any caller
/// asking "where would the eye be if nothing were lagging".
#[must_use]
pub fn anchor(target: Target, params: &ChaseParams) -> Vec3 {
    scaled_about(
        target,
        params,
        spring_anchor(target, params) + target.up * params.pos_height,
    )
}

/// The point the spring actually chases: behind the ship, at the ship's own
/// height, **unscaled**.
///
/// `anchorPos` in `Ship_UpdateCameraRigs` (`0x08845ed0`), which is
/// `shipPos + fwd * pos_length` with **no `up` term** - the vertical offset is
/// added to the sprung result afterwards, so it never lags and the spring only
/// ever works in the plane through the ship. See `oag_render::camera::chase`.
#[must_use]
pub fn spring_anchor(target: Target, params: &ChaseParams) -> Vec3 {
    target.position - target.forward * params.pos_length
}

/// The unscaled aim point, which is the centre the spring rotates the eye about.
///
/// [`look_at_point`] is this with [`ChaseParams::craft_scale`] applied, and is
/// what the camera actually aims at. The spring uses this one, because the
/// original's spring runs before the scale.
#[must_use]
pub fn aim_point(target: Target, params: &ChaseParams) -> Vec3 {
    target.position + target.up * params.lookat_height + target.forward * params.lookat_length
}

/// The point the camera aims at, which is rigid: it is recomputed from the ship
/// every frame and never lags.
#[must_use]
pub fn look_at_point(target: Target, params: &ChaseParams) -> Vec3 {
    scaled_about(target, params, aim_point(target, params))
}
