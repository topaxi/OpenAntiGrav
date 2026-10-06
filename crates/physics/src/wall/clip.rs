//! `Body_StepWorld`'s pass 1: the pre-integration clip along the velocity.
//!
//! From `Body_StepWorld` (`0x0884f70c`), the first per-body loop, before any force or
//! integration:
//!
//! ```text
//! v      = body+0x140                              ; this frame's velocity
//! if v != 0:
//!     probe = normalise(v) * 10.0                  ; 0x41200000
//!     half  = FUN_0884dcec(body) * 0.5             ; the box's (w, h, l) / 2
//!     du, dr, df = dot(probe, up), dot(probe, right), dot(probe, forward)
//!     if du >  half.h: probe *= half.h /  du       ; each factor from the
//!     if du < -half.h: probe *= half.h / -du       ; UNSCALED dot, applied
//!     if dr >  half.w: probe *= half.w /  dr       ; multiplicatively, in
//!     if dr < -half.w: probe *= half.w / -dr       ; this order
//!     if df >  half.l: probe *= half.l /  df
//!     if df < -half.l: probe *= half.l / -df
//!     start = body+0x30
//!     end   = start + v * dt + probe
//!     if Collision_RaycastWorld(world, start, end, &hit, self, 1, 2):
//!         body+0x30 = start - normalise(probe) * 0.9 * |end - hit.point|
//! ```
//!
//! `Collision_RaycastWorld`'s last two arguments are `1` (Reset colliders **not** skipped) and
//! `2` (box colliders skipped, meshes only), so the clip runs against every track surface
//! (`Wall`, `Floor`, `MagFloor`, `Reset`). There is no velocity change, only
//! `Body_SetPosition` (`0x0884d840`); the `0.9` is `0x3f666666`.
//!
//! The segment reaches from the centre to the box's own face along the velocity plus this
//! frame's travel, so the clip fires whenever the leading face would reach a surface, and
//! moves the body back by nine-tenths of the overshoot. It lifts a craft whose centre is above
//! a floor but whose hull is through it: measured on `03_Track` at spline index 200, placed
//! 3.6 units into the floor, the first frame with downward velocity lifts it `0.757` and
//! `0.9 * (1.3125 + 0.024 - 0.50) = 0.752`.
//!
//! Confidence **85**: constants and argument order read off the decompile and cross-checked
//! against `Collision_RaycastWorld`'s flag handling. The multiplicative clip is reproduced
//! literally though it over-shrinks a probe exceeding two half-extents at once.

use oag_core::math::Vec3;

use crate::collide::{Ray, Raycaster};
use crate::forces::Environment;
use crate::params::Handling;
use crate::ship::ShipState;

/// `0x41200000`: how far along the velocity the probe starts before the clip.
pub const CLIP_PROBE_REACH: f32 = 10.0;

/// `0x3f666666`: the share of the overshoot the body is moved back by.
pub const CLIP_BACKOFF: f32 = 0.9;

/// What the clip did this frame, for reporting.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Clip {
    /// The position change applied, zero when nothing was hit.
    pub moved: Vec3,
    /// The surface the segment met, if any.
    pub surface: Option<crate::collide::Surface>,
}

/// Pass 1 for one body: clip its position back along its velocity if the box's
/// leading face would reach a surface this frame. See the module docs.
pub fn pre_integration_clip<R: Raycaster + ?Sized>(
    state: &mut ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    dt: f32,
) -> Clip {
    let body = &state.body;
    let velocity = body.linear_velocity;
    if velocity == Vec3::ZERO {
        return Clip::default();
    }
    let speed = velocity.length();
    // Bound bools so a `NaN` lands in the early return.
    let moving = speed > 0.0;
    if !moving {
        return Clip::default();
    }

    let scale = crate::hover::TARGET_GLOBAL_SCALE * 0.5;
    let dimensions = &handling.dimensions;
    let half_width = dimensions.width * scale;
    let half_height = dimensions.height * scale;
    let half_length = dimensions.length * scale;

    let unscaled = velocity * (CLIP_PROBE_REACH / speed);
    let along_up = unscaled.dot(body.up());
    let along_right = unscaled.dot(body.right());
    let along_forward = unscaled.dot(body.forward());

    let mut probe = unscaled;
    for (along, half) in [
        (along_up, half_height),
        (along_right, half_width),
        (along_forward, half_length),
    ] {
        if half < along {
            probe *= half / along;
        }
        if along < -half {
            probe *= half / -along;
        }
    }

    let start = body.position;
    let end = start + velocity * dt + probe;
    let offset = end - start;
    let reach = offset.length();
    let reaches = reach > 0.0;
    if !reaches {
        return Clip::default();
    }
    let Some(hit) = raycaster.raycast(
        Ray::new(start, offset / reach, reach),
        env.self_collider,
        true,
    ) else {
        return Clip::default();
    };

    let probe_length = probe.length();
    let direction = if probe_length > 0.0 {
        probe / probe_length
    } else {
        Vec3::ZERO
    };
    let overshoot = (end - hit.point).length();
    let moved = direction * -(CLIP_BACKOFF * overshoot);
    state.body.position = start + moved;
    Clip {
        moved,
        surface: Some(hit.surface),
    }
}
