//! The magstrip attitude hold: a **kinematic basis rewrite**, not a torque.
//!
//! `Ship_UpdateMagLock` (`0x0884ba0c`) is the second half of a design whose first
//! half is already in [`crate::hover`]: the magstrip blend `craft+0x280` fades the
//! suspension out (`1 - blend` on every probe force) while this function takes the
//! attitude over. On a magstrip the spring hands the ship to a hold rather than
//! competing with it.
//!
//! # Why this cannot be a torque, and why that is load-bearing
//!
//! The original writes the three basis rows with `sv.q` and re-orthonormalises
//! them. It touches **no accumulator**, so the rotation it produces is invisible
//! to the angular-velocity column by construction. That is not a stylistic
//! observation: `docs/physics/cornering-ground-truth.md` measures a whole lap of
//! Talon's Junction and finds `body+0x150 = -omega(basis)` holding to `8 %`
//! everywhere *except* the track's inverted section, where it reads `0.023` on
//! pitch and `0.110` on roll - `99.9 %` of the tilt rotation there passes the
//! momentum column by. Routing this mechanism through
//! [`crate::forces::Accumulators`] would put that rotation back into the column
//! and re-break the identity the lap capture just scoped.
//!
//! So this module mutates [`crate::ship::Body`]'s position, velocity and
//! orientation directly, and never adds a force or a torque.
//!
//! # What the original does, in its own order
//!
//! Read instruction by instruction from the decompilation of `0x0884ba0c`;
//! confidence **90** on the writes, see
//! `docs/ghidra/functions/psp-pulse-usa/engine.md`.
//!
//! ```text
//! blend  = craft+0x240 ? min(blend + 0.2, 1) : max(blend - 0.2, 0)   ; per FRAME
//! if blend == 0: return                                              ; ordinary track
//!
//! n1     = unit(-(sample1+0x20))            ; the section's surface normal
//! s1     = sample1+0x00 - 3.0 * n1          ; undo the load-time spline lift
//! d1     = dot(pos - s1, n1)                ; height above section 1
//! h      = dot(n1, pos - craft+0x250)       ; height above the mag ray's own hit
//!
//! if sample2 is valid:                      ; +0x40 != -1024 sentinel
//!     n2, s2, d2 the same way
//!     w2 = max(1 - |d2 - h|, 0);  w1 = max(1 - |d1 - h|, 0)
//!     if w1 == w2 == 0: w1 = w2 = 0.5
//!     w1 /= w1 + w2;  w2 /= w1 + w2
//!     axis = n1*w1 + n2*w2;  d = d1*w1 + d2*w2
//!
//! if |h - d| > 5.0:                         ; the sections disagree with the ray
//!     axis = craft+0x260;  d = dot(pos - craft+0x250, axis)
//!
//! Body_Translate(body, axis * (0.8 * craft+0x2f0 - d) * blend)
//!
//! speed = |v|;  v -= axis * dot(v, axis) * blend;  v = unit(v) * speed
//!
//! forward -= axis * dot(forward, axis) * blend       ; body+0x20
//! left    -= axis * dot(left, axis)    * blend       ; body+0x00
//! up       = cross(forward, left)                    ; body+0x10
//! forward  = unit(forward)
//! up       = unit(up - forward * dot(forward, up))
//! left     = cross(up, forward)
//! ```
//!
//! Row 0 of the original's basis points **left**, not right (see `engine.md`'s
//! "The basis is positively oriented, and row 0 points left"), so the two cross
//! products above are written here in this crate's own `(right, up, forward)`
//! frame as `up = cross(right, forward)` and `right = cross(forward, up)`. Both
//! are the same identity with the row's sign carried through; the skew itself is
//! sign-symmetric, so it transcribes unchanged.
//!
//! # Three things this module does not have
//!
//! - **The original's basis is three rows; this crate's is a quaternion.** The
//!   rewrite is done on the extracted axes and converted back once, at the end.
//!   That conversion is not in the original and it is not free: it re-normalises
//!   through [`Quat::from_mat3`], on top of the explicit Gram-Schmidt above.
//! - **The blend ramps per frame, not per second.** `DAT_08a7bd44` is `0.2` and
//!   nothing multiplies it by `dt`, so a magstrip locks in five frames whatever
//!   the frame rate is. Transcribed as read; see
//!   `docs/architecture/adr/0007-fixed-timestep-vs-original.md` for why a
//!   frame-counted constant is kept frame-counted here.
//! - **`oag-trace` gained a locator (Task #33, `handover/task-33-oag-trace-cannot-exercise-the-mag.md`),
//!   but nothing has driven it far enough to prove a lap reaches a magstrip
//!   yet.** `replay`/`drive`/`drive_with` and `plan::to_gate` all take an
//!   `Option<&[oag_formats::track::Sample]>` now, located fresh every tick
//!   the same way `oag_game::race::Race::tick` locates the player - see
//!   `crate::replay::locate` and its own doc comment for why the resampling is
//!   duplicated here rather than shared. `crates/trace/src/replay/tests.rs`
//!   pins the mechanism itself with a synthetic magstrip fixture (with and
//!   without a locator); a real lap capture through Talon's Junction's
//!   inverted section, the one measured stretch, is still unexercised. The
//!   measurement that *does* exercise the real section is
//!   `crates/game/tests/maglock_ground_truth.rs`, one tick at a time.
//! - **`vrcp.s` is an approximation and this is not.** The original normalises
//!   through the VFPU reciprocal; every normalisation here is an exact `f32`
//!   divide. The project does not claim bit-identity with the hardware - see
//!   `docs/architecture/determinism.md` - and this is one of the places the two
//!   differ by construction.

use oag_core::math::{Mat3, Quat, Vec3};

use crate::collide::{Ray, Raycaster, Surface};
use crate::forces::Environment;
use crate::ship::ShipState;

/// How much the magstrip blend moves in one **frame**, `DAT_08a7bd44`.
///
/// `0.2`, read off `0x08a7bd44` as the bit pattern `0x3e4ccccd`. Added while the
/// mag flag is set and subtracted while it is clear, clamped to `0..=1`, so a
/// strip takes full effect in five frames and releases in five.
pub const BLEND_RAMP: f32 = 0.2;

/// How far the section-derived height may disagree with the measured one,
/// `DAT_08a7bd48`.
///
/// `5.0`, read off `0x08a7bd48` as `0x40a00000`. Past it the hold abandons the
/// spline entirely and uses the mag ray's own hit - see [`Hold::from_sections`].
pub const HEIGHT_MISMATCH_LIMIT: f32 = 5.0;

/// The lift `AiTrack_LoadPathPoints` applies to every control point.
///
/// The loader does `pos -= 3.0 * down`, so the running game's spline sits three
/// units above the surface the exporter wrote, and this function subtracts the
/// same three units back off to recover the surface point. The constant is a
/// literal in both places; `oag_formats::track::HOVER_LIFT` is the same number on
/// the format side, and this crate cannot see that crate.
pub const SPLINE_LIFT: f32 = 3.0;

/// How far the mag-floor probe reaches, in multiples of its own direction vector.
///
/// The ray runs from the craft position to `pos - 5.0 * (up - down)`, where `down`
/// is the section's raw (non-unit) surface axis. With the ship aligned to the
/// track that is about ten units straight into the surface.
pub const PROBE_REACH: f32 = 5.0;

/// The fraction of the hover target height the hold parks the craft at.
///
/// `0.8 * craft+0x2f0`, the same field the hover spring uses as its target - and
/// the same field the blend has already scaled by `1 + 0.2 * blend`, see
/// [`crate::hover::target_height`].
pub const HOLD_HEIGHT_FRACTION: f32 = 0.8;

/// One track-spline sample, as the mag lock reads it.
///
/// `AiTrack_LocatePosition` (`0x0887ce78`) writes two of these into the ship
/// entity, at `+0xaf0` and `+0xb60`, and they are `SplinePt`-shaped: a `0x70`-byte
/// record whose `+0x00` is the position and whose `+0x20` is the axis **into** the
/// surface. Only those two fields are used here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackSample {
    /// The spline position as the running game holds it, i.e. **lifted**
    /// [`SPLINE_LIFT`] units off the surface.
    ///
    /// `oag_formats::track::Sample::pos` is the *unlifted* disc value; a caller
    /// converts with `pos - HOVER_LIFT * down`, which is what the loader does, and
    /// this module subtracts it back off along the normalised axis exactly as the
    /// original does.
    pub position: Vec3,
    /// The frame's `down` axis: a unit vector **into** the track surface,
    /// `(0, -1, 0)` on level ground.
    ///
    /// Not normalised on the way in, deliberately: the original reads the field
    /// raw for the probe direction and normalises only for the hold axis, and an
    /// interpolated frame is not exactly unit length.
    pub down: Vec3,
}

impl TrackSample {
    /// The surface normal: `unit(-down)`.
    #[must_use]
    pub fn normal(&self) -> Vec3 {
        (-self.down).normalize_or_zero()
    }

    /// The point on the track surface under this sample.
    ///
    /// `position - 3.0 * normal`, which undoes the loader's lift.
    #[must_use]
    pub fn surface_point(&self) -> Vec3 {
        self.position - self.normal() * SPLINE_LIFT
    }
}

/// What the mag-floor probe found.
///
/// The original's third raycast, in `Ship_CastHoverProbes` (`0x08849ed4`), writing
/// its hit into `craft+0x250` (point) and `craft+0x260` (normal) and setting the
/// flag at `craft+0x240` only when the collider's material reads **3**.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MagContact {
    /// Where the ray met the mag floor, `craft+0x250`.
    pub point: Vec3,
    /// The mag floor's own normal there, `craft+0x260`.
    pub normal: Vec3,
}

/// The axis and height the hold ran with, for a caller that wants to see them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hold {
    /// The blend after this frame's ramp, `craft+0x280`.
    pub blend: f32,
    /// The axis the attitude is slaved to. Blended between the sections, and
    /// **not** renormalised afterwards, exactly as the original leaves it.
    pub axis: Vec3,
    /// The craft's height above the surface along [`Self::axis`].
    pub height: f32,
    /// How far the body was displaced this frame.
    pub translation: Vec3,
    /// Whether the section blend was abandoned for the mag ray's own normal.
    pub from_ray: bool,
}

impl Hold {
    /// Builds the axis and height from the two spline samples.
    ///
    /// `contact` supplies the measured height the weights are judged against; it
    /// is the **last** mag-floor hit rather than necessarily this frame's, because
    /// `craft+0x250` is a raycast out-parameter that only a hit overwrites and the
    /// blend takes five frames to decay after the strip ends.
    ///
    /// Returns `None` when there is nothing to build an axis from at all - no
    /// section and no contact - which the original cannot reach (a located ship
    /// always has a primary sample) and a caller that supplies no track data can.
    #[must_use]
    fn from_sections(
        position: Vec3,
        blend: f32,
        env: &Environment,
        contact: Option<MagContact>,
    ) -> Option<Self> {
        let Some(primary) = env.track_sample else {
            // No spline was supplied. The original always has one; what it does
            // when the sections disagree with the ray is use the ray, so that is
            // what a caller without sections gets rather than an invented rule.
            let contact = contact?;
            return Some(Self {
                blend,
                axis: contact.normal,
                height: (position - contact.point).dot(contact.normal),
                translation: Vec3::ZERO,
                from_ray: true,
            });
        };

        let first_normal = primary.normal();
        let first_height = (position - primary.surface_point()).dot(first_normal);
        // The height the ray actually measured, along the first section's normal.
        // Without a contact there is nothing to compare against, and the original
        // would be reading a stale hit; taking the section's own height makes the
        // weights degenerate to the primary and the mismatch test a no-op.
        let measured = contact.map_or(first_height, |contact| {
            first_normal.dot(position - contact.point)
        });

        let (mut axis, mut height) = match env.track_sample_next {
            None => (first_normal, first_height),
            Some(next) => {
                let second_normal = next.normal();
                let second_height = (position - next.surface_point()).dot(second_normal);

                let mut second_weight = 1.0 - (second_height - measured).abs();
                let mut first_weight = 1.0 - (first_height - measured).abs();
                if second_weight < 0.0 {
                    second_weight = 0.0;
                }
                if first_weight < 0.0 {
                    first_weight = 0.0;
                }
                if second_weight == 0.0 && first_weight == 0.0 {
                    first_weight = 0.5;
                    second_weight = 0.5;
                }
                let total = second_weight + first_weight;
                let second_weight = second_weight / total;
                let first_weight = first_weight / total;

                (
                    first_normal * first_weight + second_normal * second_weight,
                    first_height * first_weight + second_height * second_weight,
                )
            }
        };

        let mut from_ray = false;
        if (measured - height).abs() > HEIGHT_MISMATCH_LIMIT
            && let Some(contact) = contact
        {
            axis = contact.normal;
            height = (position - contact.point).dot(axis);
            from_ray = true;
        }

        Some(Self {
            blend,
            axis,
            height,
            translation: Vec3::ZERO,
            from_ray,
        })
    }
}

/// Ramps the magstrip blend one frame.
///
/// `blend +/- 0.2`, clamped to `0..=1`. Per frame and not per second; see the
/// module documentation.
#[must_use]
pub fn ramp(blend: f32, on_strip: bool) -> f32 {
    if on_strip {
        let ramped = blend + BLEND_RAMP;
        if ramped > 1.0 { 1.0 } else { ramped }
    } else {
        let ramped = blend - BLEND_RAMP;
        if ramped <= 0.0 { 0.0 } else { ramped }
    }
}

/// Casts the mag-floor probe: the **third** raycast of `Ship_CastHoverProbes`.
///
/// From the craft position along `-(up - down)`, reaching [`PROBE_REACH`] times
/// that vector's length. A hit counts only when the surface is
/// [`Surface::MagFloor`] - the original compares the collider's material against
/// the literal `3` - so an ordinary floor under the ship produces nothing.
///
/// Returns `None` without a track sample, because the direction is built from the
/// section's own `down` axis and there is nothing to substitute for it.
#[must_use]
pub fn probe<R: Raycaster + ?Sized>(
    state: &ShipState,
    env: &Environment,
    raycaster: &R,
) -> Option<MagContact> {
    let sample = env.track_sample?;
    let origin = state.body.position;
    // The original passes two endpoints; this crate's `Ray` is an origin, a unit
    // direction and a length. Same segment, different spelling.
    let delta = (state.body.up() - sample.down) * -PROBE_REACH;
    let length = delta.length();
    if length == 0.0 {
        return None;
    }

    let hit = raycaster.raycast(
        Ray::new(origin, delta / length, length),
        env.self_collider,
        false,
    )?;
    if hit.surface != Surface::MagFloor {
        return None;
    }

    Some(MagContact {
        point: hit.point,
        normal: hit.normal,
    })
}

/// Runs the whole mag lock for one frame: the ramp, and the hold it gates.
///
/// `target_height` is [`crate::hover::target_height`]'s result for this frame,
/// which the original computed at the top of `Ship_UpdateCraft` from the
/// **previous** frame's blend - so pass the same value the hover spring was given,
/// not one recomputed from the blend this function is about to write.
///
/// Returns `None` on ordinary track, where the original returns immediately with
/// the blend at zero and nothing else touched.
pub fn update(
    state: &mut ShipState,
    env: &Environment,
    contact: Option<MagContact>,
    target_height: f32,
) -> Option<Hold> {
    let blend = ramp(state.mag_lock_blend, contact.is_some());
    state.mag_lock_blend = blend;
    if let Some(contact) = contact {
        state.mag_contact = Some(contact);
    }
    if blend == 0.0 {
        return None;
    }

    let mut hold = Hold::from_sections(state.body.position, blend, env, state.mag_contact)?;
    let axis = hold.axis;

    // The reposition. `Body_Translate` moves the body and nothing else - no
    // velocity change, no impulse - so the craft is *held* at four fifths of its
    // hover target off the strip rather than sprung toward it.
    hold.translation = axis * (HOLD_HEIGHT_FRACTION * target_height - hold.height) * blend;
    state.body.position += hold.translation;

    // The velocity is projected off the axis and renormalised to the magnitude it
    // had. `docs/ghidra/functions/psp-pulse-usa/rigid-body.md` already ruled this out
    // as the source of the missing linear resistance for exactly that reason: it
    // turns the velocity, it never shortens it.
    let velocity = state.body.linear_velocity;
    let speed = velocity.length();
    let projected = velocity - axis * velocity.dot(axis) * blend;
    state.body.linear_velocity = projected.normalize_or_zero() * speed;

    hold_basis(state, axis, blend);

    Some(hold)
}

/// The basis rewrite, in the original's operation order.
///
/// Separated from [`update`] so the one thing this whole mechanism exists to do
/// can be read - and tested - without the ramp, the probe or the reposition
/// around it.
pub fn hold_basis(state: &mut ShipState, axis: Vec3, blend: f32) {
    let body = &mut state.body;

    let mut forward = body.forward();
    let mut right = body.right();

    // Skew both axes toward the surface plane. At `blend == 1` the component
    // along the axis is removed outright, which is what slaves the attitude.
    forward -= axis * forward.dot(axis) * blend;
    right -= axis * right.dot(axis) * blend;

    // `up = cross(forward, left)` in the original, whose row 0 is the ship's
    // **left**. In this crate's frame that is `cross(right, forward)`.
    let up = right.cross(forward);

    // Gram-Schmidt with forward primary, exactly the original's ordering: the
    // skew has left the three axes neither unit nor orthogonal.
    let forward = forward.normalize_or_zero();
    let up = (up - forward * forward.dot(up)).normalize_or_zero();
    // `left = cross(up, forward)` there, `right = cross(forward, up)` here.
    let right = forward.cross(up);

    if forward == Vec3::ZERO || up == Vec3::ZERO {
        // A degenerate basis would come back out of `from_mat3` as a `NaN`
        // quaternion and poison every later frame. The original's `vcmovt.s`
        // guard substitutes `MaxFloat` for a zero length and carries on with a
        // zero vector; keeping the previous orientation is the same refusal to
        // produce a rotation, without the infinity.
        return;
    }

    // Body `+X` is right, `+Y` up and `-Z` forward, per `Body::forward`.
    body.orientation = Quat::from_mat3(&Mat3::from_cols(right, up, -forward));
}

#[cfg(test)]
mod tests;
