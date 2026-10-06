//! The magstrip attitude hold: a **kinematic basis rewrite**, not a torque.
//!
//! `Ship_UpdateMagLock` (`0x0884ba0c`) is the second half of a design whose first half is in
//! [`crate::hover`]: the blend `craft+0x280` fades the suspension out (`1 - blend` on every
//! probe force) while this function takes the attitude over.
//!
//! # Why this cannot be a torque
//!
//! The original writes the three basis rows with `sv.q` and re-orthonormalises them, touching
//! **no accumulator**, so the rotation is invisible to the angular-velocity column by
//! construction. `docs/physics/cornering-ground-truth.md` measures a Talon's Junction lap and
//! finds `body+0x150 = -omega(basis)` to `8 %` everywhere *except* the inverted section, where
//! it reads `0.023` on pitch and `0.110` on roll: `99.9 %` of the tilt rotation bypasses the
//! momentum column. Routing this through [`crate::forces::Accumulators`] would put it back and
//! re-break the identity. So this module mutates [`crate::ship::Body`]'s position, velocity
//! and orientation directly and never adds a force or torque.
//! # What the original does, in its own order
//!
//! Read instruction by instruction from `0x0884ba0c`; confidence **90** on the writes
//! (`docs/ghidra/functions/psp-pulse-usa/engine.md`).
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
//! Row 0 of the original's basis points **left** (`engine.md`, "The basis is positively
//! oriented, and row 0 points left"), so the two cross products are written here in this
//! crate's `(right, up, forward)` frame as `up = cross(right, forward)` and
//! `right = cross(forward, up)`: the same identity with the row's sign carried through (the
//! skew is sign-symmetric, so it transcribes unchanged).
//!
//! # Three things this module does not have
//!
//! - **The original's basis is three rows; this crate's is a quaternion.** The rewrite is done
//!   on extracted axes and converted back once, re-normalising through [`Quat::from_mat3`] on
//!   top of the explicit Gram-Schmidt. Not in the original, and not free.
//! - **The blend ramps per frame, not per second.** `DAT_08a7bd44` is `0.2` and nothing
//!   multiplies it by `dt`, so a strip locks in five frames at any frame rate. Transcribed as
//!   read (`docs/architecture/adr/0007-fixed-timestep-vs-original.md`).
//! - **`vrcp.s` is an approximation and this is not.** The original normalises through the
//!   VFPU reciprocal; every normalisation here is an exact `f32` divide, one place the two
//!   differ by construction (`docs/architecture/determinism.md`).
//!
//! The hold's effect on a trajectory is measured through the `oag-trace` locator; see
//! `docs/physics/cornering-ground-truth.md`, "The hold, now measured as a trajectory".

use oag_core::math::{Mat3, Quat, Vec3};

use crate::collide::{Ray, Raycaster, Surface};
use crate::forces::Environment;
use crate::ship::ShipState;

/// How much the magstrip blend moves in one **frame**, `DAT_08a7bd44`: `0.2` (bit pattern
/// `0x3e4ccccd`), added while the mag flag is set and subtracted while clear, clamped to
/// `0..=1`, so a strip takes full effect in five frames and releases in five.
pub const BLEND_RAMP: f32 = 0.2;

/// How far the section-derived height may disagree with the measured one, `DAT_08a7bd48`:
/// `5.0` (`0x40a00000`). Past it the hold abandons the spline and uses the mag ray's own hit
/// ([`Hold::from_sections`]).
pub const HEIGHT_MISMATCH_LIMIT: f32 = 5.0;

/// The lift `AiTrack_LoadPathPoints` applies to every control point: `pos -= 3.0 * down`, so
/// the running game's spline sits three units above the surface the exporter wrote, and the
/// hold subtracts the same three back off to recover the surface point. A literal in both
/// places; `oag_vex::track::HOVER_LIFT` is the same number on the format side, which this crate
/// cannot see.
pub const SPLINE_LIFT: f32 = 3.0;

/// How far the mag-floor probe reaches, in multiples of its own direction vector: the ray runs
/// from the craft to `pos - 5.0 * (up - down)`, `down` being the section's raw (non-unit)
/// surface axis, about ten units into the surface when aligned.
pub const PROBE_REACH: f32 = 5.0;

/// The fraction of the hover target height the hold parks the craft at: `0.8 * craft+0x2f0`,
/// the hover spring's target field, already scaled by `1 + 0.2 * blend`
/// ([`crate::hover::target_height`]).
pub const HOLD_HEIGHT_FRACTION: f32 = 0.8;

/// One track-spline sample, as the mag lock reads it. `AiTrack_LocatePosition` (`0x0887ce78`)
/// writes two into the ship entity at `+0xaf0` and `+0xb60`: `SplinePt`-shaped `0x70`-byte
/// records whose `+0x00` is the position and `+0x20` the axis **into** the surface. Only
/// those two fields are used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackSample {
    /// The spline position as the running game holds it, **lifted** [`SPLINE_LIFT`] units off
    /// the surface. `oag_vex::track::Sample::pos` is the *unlifted* disc value; a caller
    /// converts with `pos - HOVER_LIFT * down` as the loader does, and this module subtracts it
    /// back along the normalised axis as the original does.
    pub position: Vec3,
    /// The frame's `down` axis: a unit vector **into** the track surface, `(0, -1, 0)` on level
    /// ground. Not normalised on the way in: the original reads it raw for the probe direction
    /// and normalises only for the hold axis, and an interpolated frame is not exactly unit.
    pub down: Vec3,
}

impl TrackSample {
    /// The surface normal: `unit(-down)`.
    #[must_use]
    pub fn normal(&self) -> Vec3 {
        (-self.down).normalize_or_zero()
    }

    /// The point on the track surface under this sample: `position - 3.0 * normal`, undoing
    /// the loader's lift.
    #[must_use]
    pub fn surface_point(&self) -> Vec3 {
        self.position - self.normal() * SPLINE_LIFT
    }
}

/// What the mag-floor probe found: the original's third raycast, in `Ship_CastHoverProbes`
/// (`0x08849ed4`), writing its hit to `craft+0x250` (point) and `craft+0x260` (normal) and
/// setting the flag at `craft+0x240` only when the collider's material reads **3**.
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
    /// The axis the attitude is slaved to. Blended between the sections and **not**
    /// renormalised, as the original leaves it.
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
    /// `contact` supplies the measured height the weights are judged against: the **last**
    /// mag-floor hit, not necessarily this frame's, because `craft+0x250` is a raycast
    /// out-parameter only a hit overwrites and the blend takes five frames to decay after the
    /// strip ends.
    ///
    /// Returns `None` with no section and no contact, which the original cannot reach (a
    /// located ship always has a primary sample) and a caller supplying no track data can.
    #[must_use]
    fn from_sections(
        position: Vec3,
        blend: f32,
        env: &Environment,
        contact: Option<MagContact>,
    ) -> Option<Self> {
        let Some(primary) = env.track_sample else {
            // No spline supplied. The original always has one and, when the sections disagree
            // with the ray, uses the ray, so a caller without sections gets that, not an
            // invented rule.
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
        // The height the ray measured, along the first section's normal. With no contact the
        // original would read a stale hit; the section's own height makes the weights
        // degenerate to the primary and the mismatch test a no-op.
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

/// Ramps the magstrip blend one frame: `blend +/- 0.2`, clamped to `0..=1`. Per frame, not
/// per second (module docs).
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
/// From the craft position along `-(up - down)`, reaching [`PROBE_REACH`] times that vector's
/// length. A hit counts only for [`Surface::MagFloor`] (the original compares the collider's
/// material against the literal `3`). Returns `None` without a track sample, as the direction
/// is built from the section's own `down` axis.
#[must_use]
pub fn probe<R: Raycaster + ?Sized>(
    state: &ShipState,
    env: &Environment,
    raycaster: &R,
) -> Option<MagContact> {
    let sample = env.track_sample?;
    let origin = state.body.position;
    // The original passes two endpoints; `Ray` is an origin, unit direction and length.
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
/// `target_height` is [`crate::hover::target_height`]'s result for this frame, which the
/// original computed at the top of `Ship_UpdateCraft` from the **previous** frame's blend:
/// pass the value the hover spring got, not one recomputed from the blend this function is
/// about to write. Returns `None` on ordinary track, where the original returns immediately
/// with the blend at zero.
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

    // The reposition. `Body_Translate` moves the body and nothing else (no velocity change or
    // impulse): the craft is *held* at four fifths of its hover target, not sprung toward it.
    hold.translation = axis * (HOLD_HEIGHT_FRACTION * target_height - hold.height) * blend;
    state.body.position += hold.translation;

    // The velocity is projected off the axis and renormalised to its magnitude, which is why
    // `docs/ghidra/functions/psp-pulse-usa/rigid-body.md` ruled this out as the source of the
    // missing linear resistance: it turns the velocity, never shortens it.
    let velocity = state.body.linear_velocity;
    let speed = velocity.length();
    let projected = velocity - axis * velocity.dot(axis) * blend;
    state.body.linear_velocity = projected.normalize_or_zero() * speed;

    hold_basis(state, axis, blend);

    Some(hold)
}

/// The basis rewrite, in the original's operation order. Separate from [`update`] so the one
/// thing this mechanism exists to do can be read and tested without the ramp, probe or
/// reposition.
pub fn hold_basis(state: &mut ShipState, axis: Vec3, blend: f32) {
    let body = &mut state.body;

    let mut forward = body.forward();
    let mut right = body.right();

    // Skew both axes toward the surface plane; at `blend == 1` the component along the axis
    // is removed outright, which slaves the attitude.
    forward -= axis * forward.dot(axis) * blend;
    right -= axis * right.dot(axis) * blend;

    // `up = cross(forward, left)` in the original, whose row 0 is **left**; here
    // `cross(right, forward)`.
    let up = right.cross(forward);

    // Gram-Schmidt with forward primary, as the original: the skew leaves the axes neither
    // unit nor orthogonal.
    let forward = forward.normalize_or_zero();
    let up = (up - forward * forward.dot(up)).normalize_or_zero();
    // `left = cross(up, forward)` there, `right = cross(forward, up)` here.
    let right = forward.cross(up);

    if forward == Vec3::ZERO || up == Vec3::ZERO {
        // A degenerate basis would give a `NaN` quaternion from `from_mat3` and poison every
        // later frame. The original's `vcmovt.s` guard substitutes `MaxFloat` for a zero length
        // and carries on with a zero vector; keeping the previous orientation is the same
        // refusal to rotate, without the infinity.
        return;
    }

    // Body `+X` is right, `+Y` up and `-Z` forward, per `Body::forward`.
    body.orientation = Quat::from_mat3(&Mat3::from_cols(right, up, -forward));
}

#[cfg(test)]
mod tests;
