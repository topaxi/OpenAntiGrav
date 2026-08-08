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
//!   lags. Since confirmed against the original - see below.
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
//!
//! # Scored against the original, 2026-08-08
//!
//! `Ship_UpdateCameraRigs` (`0x08845ed0`) was read at instruction level, which
//! turns four of the readings above into measurements. Recorded here rather than
//! only in the docs tree, because the next person to change this file will read
//! this comment and not `camera.md`:
//!
//! - **Confirmed: the two springs split along and across the ship's own up.** The
//!   original closes the up component at `spring_vert` and *both* remaining
//!   components - side and forward - at `spring_horiz`, in the craft's frame, not
//!   the world's. That is exactly [`Chase::advance`].
//! - **Confirmed: the integrator is `error * rate * dt`**, a plain first-order
//!   lag with no transcendental, which is what this module chose without
//!   evidence. The original does **not** clamp the factor; [`lag`] does, and that
//!   is this module's own guard rather than a reproduction.
//! - **Confirmed: only the eye is sprung** and the look-at point is rigid.
//! - **Confirmed and now reproduced: the eye's distance from the look-at point is
//!   rigid.** The original springs the eye and then rewrites it as
//!   `look_at + normalize(eye - look_at) * |anchor - look_at|`, so the spring
//!   moves the eye only *around* the look-at point and never nearer or further
//!   from it. [`Chase::advance`] does that.
//! - **Confirmed and now reproduced: `pos_height` is applied after the spring**,
//!   so the vertical offset is rigid and the spring acts on a forward-only offset
//!   point. [`spring_anchor`] is what the spring chases; [`anchor`] - the settled
//!   eye, with both offsets - is where it ends up, and the two agree at rest,
//!   which is why the change is invisible on a settled frame.
//!
//! # Measured against a 150-tick capture of the original, 2026-08-08
//!
//! Both of the above landed together, and the reason to land them together is
//! that neither is worth much alone. `data/traces/pad0-boost.csv` carries the
//! original's own per-tick craft pose *and* camera position through a speed-pad
//! crossing at 40-154 units/s. Driving this module with the capture's pose and
//! `dt`, seeded from its own tick 0, and comparing the eye against the recorded
//! `cam_pos_*` over all 150 ticks:
//!
//! | Model | RMS error, world units | Max |
//! | --- | ---: | ---: |
//! | free spring, `pos_height` before (what this was) | 4.938 | 6.824 |
//! | `pos_height` after the spring only | 4.954 | 6.831 |
//! | rigid radius only | 0.415 | 0.556 |
//! | both, but the craft scale pre-multiplied into the offsets | 0.121 | 0.332 |
//! | **both, craft scale applied on the way out - what ships** | **0.008** | **0.060** |
//!
//! Three digits of agreement over 150 ticks of an accelerating, rolling craft is
//! not a shape that survives a wrong reading, and it simultaneously confirms the
//! `lookat_*`/`pos_*` sign conventions, the first-order lag, the craft-frame
//! spring split and where `oag_physics::hover::TARGET_GLOBAL_SCALE` belongs.
//! `crates/game/tests/chase_camera_ground_truth.rs` is that comparison, kept as a
//! test rather than as a number in a comment.
//!
//! The fourth row is the one that was not expected, and it is why
//! [`ChaseParams::craft_scale`] exists as a field: `camera.md` recorded scaling
//! the eye about the craft as "the same thing as scaling all four offsets", which
//! is true of the rigid look-at point and false of the sprung eye. See that
//! field's own documentation.
//!
//! The clamp in [`lag`] is the one thing here the original does not do, and at
//! the shipped `spring_horiz`/`spring_vert` of 11-13 per second it never bites at
//! any plausible `dt`.

use oag_core::math::{Mat4, Vec3, camera};

/// The seven values one `<ExternalCameraFar>` or `<ExternalCameraClose>` block
/// carries, plus the one scale the original applies to a rig from outside it.
///
/// Deliberately not `Default`: see the module documentation.
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

/// Shrinks a craft-space point toward the craft by
/// [`ChaseParams::craft_scale`], which is the last thing the original's rig does
/// to both of the points it hands the tripod.
#[must_use]
fn scaled_about(target: Target, params: &ChaseParams, point: Vec3) -> Vec3 {
    target.position + (point - target.position) * params.craft_scale
}

/// Where the eye ends up with no lag left to work off: both offsets applied,
/// and the craft scale with them, so this is a world position.
///
/// **Not what the spring chases** - see [`spring_anchor`], which drops the
/// vertical term because the original applies it after the spring rather than
/// before, and drops the scale because the original applies that after the
/// spring too. The two are the same point once the spring has settled, which is
/// why this is still the right answer for [`Chase::snapped`] and for any caller
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
/// ever works in the plane through the ship. See the module documentation.
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

/// The external camera. Its only state is where the sprung eye currently is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chase {
    /// The sprung point, **before** `pos_height` is added.
    ///
    /// The original's `craft+0x7e0` / `craft+0x7f0`, which is what it writes
    /// back and reads next frame; it adds the vertical offset only on the way to
    /// the tripod. Storing the published eye instead would feed the offset back
    /// into the spring on the next tick and re-introduce exactly the behaviour
    /// this reproduces away from.
    sprung: Vec3,
}

impl Chase {
    /// A camera already settled on its rigid offset, with no lag to work off.
    ///
    /// What a race start or a respawn wants: springing in from wherever the eye
    /// happened to be last would sweep the camera across the world.
    #[must_use]
    pub fn snapped(target: Target, params: &ChaseParams) -> Self {
        Self {
            sprung: spring_anchor(target, params),
        }
    }

    /// A camera whose sprung point starts explicitly, `pos_height` **not**
    /// included.
    ///
    /// The inverse of [`Self::sprung_eye`], not of [`Self::eye`]: a caller
    /// round-tripping a published eye has to subtract `up * pos_height` itself,
    /// because this type does not hold the target that would let it do so.
    #[must_use]
    pub fn at(sprung: Vec3) -> Self {
        Self { sprung }
    }

    /// Where the camera actually is: the sprung point, plus the rigid
    /// `pos_height` lift along the ship's up, shrunk toward the craft by
    /// [`ChaseParams::craft_scale`] - the original's last two lines, in order.
    #[must_use]
    pub fn eye(&self, target: Target, params: &ChaseParams) -> Vec3 {
        scaled_about(target, params, self.sprung + target.up * params.pos_height)
    }

    /// The raw state, without the vertical offset and without the craft scale.
    /// Mostly of interest to a test that wants to assert the rigid radius, which
    /// is a property of this point and not of [`Self::eye`].
    #[must_use]
    pub fn sprung_eye(&self) -> Vec3 {
        self.sprung
    }

    /// Rotates the eye `dt` seconds further around the look-at point, toward the
    /// rigid offset behind `target`.
    ///
    /// Three steps, all of them the original's, and the order matters:
    ///
    /// 1. **Spring.** The error toward [`spring_anchor`] is split along
    ///    `target.up` and across it, and the two halves close at `spring_vert`
    ///    and `spring_horiz`. The original decomposes the across-up half again,
    ///    into `side` and `forward`, and closes both at `spring_horiz`; for an
    ///    orthonormal frame that is the same vector, and taking the remainder
    ///    directly means a caller's slightly non-orthogonal frame loses nothing
    ///    rather than dropping the leftover component.
    /// 2. **Re-project onto the sphere.** The sprung point is pushed back out to
    ///    `|spring_anchor - look_at|` from the look-at point, so the spring only
    ///    ever *rotates* the eye about the point it is aiming at. This is the
    ///    behaviour the capture measures: see the module documentation.
    /// 3. `pos_height` and [`ChaseParams::craft_scale`] are **not** applied here
    ///    at all - [`Self::eye`] applies both, in that order, and neither ever
    ///    reaches the state.
    ///
    /// Pure enough to test without a window or a GPU: the only state is the
    /// sprung point.
    pub fn advance(&mut self, target: Target, params: &ChaseParams, dt: f32) {
        let look_at = aim_point(target, params);
        let anchor = spring_anchor(target, params);
        let error = anchor - self.sprung;

        // `normalize_or_zero` rather than a branch on a degenerate frame: with a
        // zero up vector the vertical part comes out zero and the whole error is
        // treated as horizontal, which is the harmless reading.
        let up = target.up.normalize_or_zero();
        let vertical = up * error.dot(up);
        let horizontal = error - vertical;

        let sprung = self.sprung
            + horizontal * lag(params.spring_horiz, dt)
            + vertical * lag(params.spring_vert, dt);

        // The one guard the original does not have. It normalises unconditionally,
        // which is a division by zero if the spring ever lands the eye exactly on
        // the look-at point; falling back on the anchor keeps the radius right and
        // is unreachable for any authored parameter set, since `spring_anchor` and
        // `look_at_point` are `pos_length + lookat_length` apart by construction.
        let offset = sprung - look_at;
        self.sprung = if offset.length_squared() > 0.0 {
            look_at + offset.normalize() * (anchor - look_at).length()
        } else {
            anchor
        };
    }

    /// The view matrix for the current eye, aimed at the rigid look-at point.
    ///
    /// `target.up` is the up vector, so the horizon rolls with the ship, which is
    /// what the original's external view does through a barrel roll.
    #[must_use]
    pub fn view(&self, target: Target, params: &ChaseParams) -> Mat4 {
        camera::look_at(
            self.eye(target, params),
            look_at_point(target, params),
            target.up,
        )
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

    /// `craft_scale` is `1.0` in every fixture but the one test that is about
    /// the scale, so the rest read as the geometry they are testing.
    fn params() -> ChaseParams {
        ChaseParams {
            fov: 1.0,
            lookat_height: 1.0,
            lookat_length: 10.0,
            pos_height: 2.0,
            pos_length: 8.0,
            spring_horiz: 4.0,
            spring_vert: 2.0,
            craft_scale: 1.0,
        }
    }

    /// The unit direction from the aim point to the sprung eye, which is the
    /// only thing [`Chase::advance`] can change: the radius is pinned.
    fn bearing(camera: &Chase, target: Target, params: &ChaseParams) -> Vec3 {
        (camera.sprung_eye() - aim_point(target, params)).normalize()
    }

    #[test]
    fn a_snapped_camera_sits_behind_and_above_the_ship() {
        let camera = Chase::snapped(target(), &params());
        assert_eq!(camera.eye(target(), &params()), Vec3::new(-8.0, 2.0, 0.0));
        // And the state underneath it carries no vertical offset.
        assert_eq!(camera.sprung_eye(), Vec3::new(-8.0, 0.0, 0.0));
    }

    /// The craft scale shrinks what comes *out* and never what goes in, which is
    /// the ordering the capture distinguishes: a state kept in scaled space is a
    /// different state, not the same one in other units.
    #[test]
    fn the_craft_scale_reaches_the_published_points_and_not_the_state() {
        let target = target();
        let scaled = ChaseParams {
            craft_scale: 0.75,
            ..params()
        };
        let unscaled = params();

        // Same spring, same state, whatever the scale says.
        let (mut a, mut b) = (
            Chase::at(Vec3::new(50.0, -30.0, 20.0)),
            Chase::at(Vec3::new(50.0, -30.0, 20.0)),
        );
        for _ in 0..60 {
            a.advance(target, &scaled, 1.0 / 60.0);
            b.advance(target, &unscaled, 1.0 / 60.0);
        }
        assert_eq!(a.sprung_eye(), b.sprung_eye());

        // Different published points, by exactly the scale, about the craft.
        for (scaled_point, plain) in [
            (a.eye(target, &scaled), b.eye(target, &unscaled)),
            (
                look_at_point(target, &scaled),
                look_at_point(target, &unscaled),
            ),
            (anchor(target, &scaled), anchor(target, &unscaled)),
        ] {
            let wanted = target.position + (plain - target.position) * 0.75;
            assert!((scaled_point - wanted).length() < 1e-5, "{scaled_point}");
        }
    }

    /// The property the 150-tick capture measures, and the reason this module was
    /// rewritten: the spring may rotate the eye about the look-at point and may
    /// not move it nearer or further.
    #[test]
    fn the_spring_never_changes_the_eyes_distance_from_the_look_at_point() {
        let params = params();
        let target = target();
        let radius = (spring_anchor(target, &params) - aim_point(target, &params)).length();

        // Deliberately hostile: a wild starting eye, a huge dt, and a second run
        // at a tiny one. Neither may change the radius by more than rounding.
        for (start, dt) in [
            (Vec3::new(50.0, -30.0, 20.0), 1.0 / 60.0),
            (Vec3::new(-1.0, 0.5, 0.25), 10.0),
            (Vec3::new(0.0, 100.0, 0.0), 1e-4),
        ] {
            let mut camera = Chase::at(start);
            for _ in 0..120 {
                camera.advance(target, &params, dt);
                let r = (camera.sprung_eye() - aim_point(target, &params)).length();
                assert!((r - radius).abs() < 1e-3, "radius {r} against {radius}");
            }
        }
    }

    #[test]
    fn the_look_at_point_is_ahead_of_the_ship_whatever_the_eye_is_doing() {
        let params = params();
        let ahead = look_at_point(target(), &params);
        assert_eq!(ahead, Vec3::new(10.0, 1.0, 0.0));
        // Rigid: moving the eye must not move what the camera aims at.
        assert_eq!(look_at_point(target(), &params), ahead);
    }

    /// A zero `dt` may not *rotate* the eye. It does still snap the radius,
    /// because the original re-projects unconditionally rather than inside the
    /// spring - so an eye placed off the sphere is pulled onto it on the first
    /// call whatever the timestep, and asserting the eye does not move at all
    /// would be asserting the wrong thing.
    #[test]
    fn zero_dt_does_not_rotate_the_eye() {
        let params = params();
        let target = target();
        let mut camera = Chase::at(Vec3::new(100.0, 100.0, 100.0));
        let before = bearing(&camera, target, &params);
        camera.advance(target, &params, 0.0);
        assert!((bearing(&camera, target, &params) - before).length() < 1e-6);
    }

    #[test]
    fn a_settled_camera_stays_settled() {
        let params = params();
        let mut camera = Chase::snapped(target(), &params);
        for _ in 0..600 {
            camera.advance(target(), &params, 1.0 / 60.0);
        }
        assert!((camera.eye(target(), &params) - anchor(target(), &params)).length() < 1e-4);
    }

    /// Convergence is now **angular**: the eye slides around the sphere toward
    /// the anchor's bearing. Measuring the Euclidean error instead would not be
    /// monotone, because the first call teleports the eye onto the sphere.
    #[test]
    fn the_eye_converges_on_the_anchors_bearing_from_anywhere() {
        let params = params();
        let target = target();
        let look_at = aim_point(target, &params);
        let wanted = (spring_anchor(target, &params) - look_at).normalize();

        let mut camera = Chase::at(Vec3::new(50.0, -30.0, 20.0));
        camera.advance(target, &params, 0.0);
        let start = (bearing(&camera, target, &params) - wanted).length();
        assert!(start > 0.1, "the fixture must start well off the anchor");

        let mut previous = start;
        for _ in 0..600 {
            camera.advance(target, &params, 1.0 / 60.0);
            let error = (bearing(&camera, target, &params) - wanted).length();
            // Monotone: a first-order lag never overshoots.
            assert!(
                error <= previous + 1e-6,
                "error grew from {previous} to {error}"
            );
            previous = error;
        }
        assert!(previous < start * 0.01, "{start} to {previous}");
    }

    /// `pos_height` is rigid: it is not sprung, so it is present in full on the
    /// very first frame however far the eye is from where it belongs.
    #[test]
    fn the_vertical_offset_does_not_lag() {
        let params = params();
        let target = target();
        let mut camera = Chase::at(Vec3::new(50.0, -30.0, 20.0));
        camera.advance(target, &params, 1.0 / 60.0);
        let lift = camera.eye(target, &params) - camera.sprung_eye();
        assert!(
            (lift - target.up * params.pos_height).length() < 1e-6,
            "{lift}"
        );
    }

    /// Starts a camera at `spring_anchor + offset`, takes **one** step, and
    /// reports how far its bearing was from the anchor's before and after.
    ///
    /// One step and not six hundred, deliberately. A displacement lying purely on
    /// one spring's axis stays on that axis for exactly one tick: the
    /// re-projection changes the eye's *radius*, and from the second tick the
    /// error toward the anchor has a component on the other axis too, which the
    /// other spring then legitimately acts on. So a run-to-settled assertion here
    /// would be testing that mixing rather than the split, and would have to be
    /// hedged with a tolerance big enough to hide a real sign error.
    fn one_step_bearing_error(target: Target, params: &ChaseParams, offset: Vec3) -> (f32, f32) {
        let look_at = aim_point(target, params);
        let wanted = (spring_anchor(target, params) - look_at).normalize();
        let mut camera = Chase::at(spring_anchor(target, params) + offset);
        let before = (bearing(&camera, target, params) - wanted).length();
        camera.advance(target, params, 1.0 / 60.0);
        (before, (bearing(&camera, target, params) - wanted).length())
    }

    /// Each spring closes its own axis of the ship's frame and only its own.
    ///
    /// Stated as "a displacement purely along the ship's up does not move under
    /// `spring_horiz` alone, and one purely across it does not move under
    /// `spring_vert` alone". The re-projection cannot rescue either case, because
    /// it is a *radial* scaling about the look-at point and so changes no
    /// bearing at all.
    #[test]
    fn the_two_springs_act_on_their_own_axis_of_the_ships_frame() {
        let target = target();
        let along_up = target.up * 10.0;
        let across_up = Vec3::Z * 10.0;

        let vertical_only = ChaseParams {
            spring_horiz: 0.0,
            ..params()
        };
        let horizontal_only = ChaseParams {
            spring_vert: 0.0,
            ..params()
        };

        for (name, params, offset, moves) in [
            ("vert spring, vert error", &vertical_only, along_up, true),
            ("vert spring, horiz error", &vertical_only, across_up, false),
            (
                "horiz spring, vert error",
                &horizontal_only,
                along_up,
                false,
            ),
            (
                "horiz spring, horiz error",
                &horizontal_only,
                across_up,
                true,
            ),
        ] {
            let (before, after) = one_step_bearing_error(target, params, offset);
            assert!(before > 0.1, "{name}: the fixture must start displaced");
            if moves {
                assert!(
                    after < before - 1e-3,
                    "{name}: {before} to {after}, expected to close"
                );
            } else {
                assert!(
                    (after - before).abs() < 1e-6,
                    "{name}: {before} to {after}, expected untouched"
                );
            }
        }
    }

    #[test]
    fn a_rolled_ship_springs_along_its_own_up_not_the_worlds() {
        // Rolled 90 degrees, so the ship's up is world +Z and world +Y is now
        // across it. The same world-space displacement therefore has to change
        // hands between the two springs, which it cannot do if the split is
        // taken against world Y.
        let rolled = Target {
            position: Vec3::ZERO,
            forward: Vec3::X,
            up: Vec3::Z,
        };
        let horizontal_only = ChaseParams {
            spring_vert: 0.0,
            ..params()
        };

        // Upright, +Z is across the ship's up, so the horizontal spring closes it.
        let (before, after) = one_step_bearing_error(target(), &horizontal_only, Vec3::Z * 10.0);
        assert!(after < before - 1e-3, "upright: {before} to {after}");

        // Rolled, the very same offset is along the ship's up and is frozen.
        let (before, after) = one_step_bearing_error(rolled, &horizontal_only, Vec3::Z * 10.0);
        assert!(
            (after - before).abs() < 1e-6,
            "rolled: {before} to {after}, expected untouched"
        );
    }
}
