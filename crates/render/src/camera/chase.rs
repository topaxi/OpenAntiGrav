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
//! `crates/raceplay/tests/chase_camera_ground_truth.rs` is that comparison, kept as a
//! test rather than as a number in a comment.
//!
//! The fourth row is the one that was not expected, and it is why
//! [`ChaseParams::craft_scale`] exists as a field: `camera.md` recorded scaling
//! the eye about the craft as "the same thing as scaling all four offsets", which
//! is true of the rigid look-at point and false of the sprung eye. See that
//! field's own documentation.
//!
//! The clamp in [`lag`] is the one thing here the original does not do. At any
//! spring rate a shipped block authors it never bites at any plausible `dt`,
//! which is why the agreement above is not evidence *for* it - the capture never
//! reaches the branch.

use oag_core::math::{Mat4, Vec3, Vec4, camera};
use oag_race::chase_rig::{
    ChaseParams, Target, aim_point, look_at_point, scaled_about, spring_anchor,
};

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
        let eye = self.eye(target, params);
        let aim = look_at_point(target, params);
        if params.view_sheared {
            sheared_look_at(eye, aim, target.up)
        } else {
            camera::look_at(eye, aim, target.up)
        }
    }
}

/// Wipeout Pure's view matrix: the three rows are `forward x up` (**not** renormalised, so its
/// length is the cosine of the angle the forward axis is pitched off the horizontal), `up`
/// as given, and `-forward`, with the eye taken off by dot products.
///
/// Not a rotation: `up` is not perpendicular to `forward`, so a level line stays level and a
/// point ahead of the craft at the eye's height stays on the screen's centre line however far
/// the look-at point is below it. Read live off Pure's GE view matrix, whose rows at the grid
/// are `(0.002, 0.003, 0.995)`, `(0, 1.0, 0.097)`, `(-0.995, 0, 0.002)` - exactly this with
/// `forward` pitched `5.71` degrees down - and off the camera tripod it was copied from.
#[must_use]
pub fn sheared_look_at(eye: Vec3, aim: Vec3, up: Vec3) -> Mat4 {
    let forward = (aim - eye).normalize_or_zero();
    let up = up.normalize_or_zero();
    let side = forward.cross(up);
    Mat4::from_cols(
        Vec4::new(side.x, up.x, -forward.x, 0.0),
        Vec4::new(side.y, up.y, -forward.y, 0.0),
        Vec4::new(side.z, up.z, -forward.z, 0.0),
        Vec4::new(-side.dot(eye), -up.dot(eye), forward.dot(eye), 1.0),
    )
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
mod tests;
