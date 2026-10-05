//! The track tube the LeachBeam's chain is kept inside.
//!
//! Recovered from `LeachBeam_KeepInTrack` (`0x088734d0`) and
//! `LeachBeam_ReaimChain` (`0x08873328`), both re-read from disassembly on
//! 2026-09-30 - see `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
//! "2026-09-30: the track tube". `LeachBeam_Advance` walks the chain one
//! undisplaced point at a time (`base += step`) and hands every point to
//! `KeepInTrack` **before** the amplitude displacement is added on top, so the
//! arc bends round a corner with its jitter kept, rather than the jittered line
//! being clipped.
//!
//! This is a pure function of a locator: `oag-render` owns no track, so the
//! caller supplies `Fn(Vec3) -> Option<TubeFrame>`, the stand-in for
//! `AiTrack_LocatePosition(100.0, track, &frame, point, ...)`.

use oag_core::math::Vec3;

/// How far inside each edge the tube's wall sits: the `2.0` literal
/// `KeepInTrack` subtracts from both half-widths. Confidence **85**, read
/// directly (`0x4000_0000` at `0x08873648`, `0x08873818`).
pub const EDGE_MARGIN: f32 = 2.0;

/// The fields of the located `SplinePt` that `KeepInTrack` reads: the record
/// `AiTrack_LocatePosition` fills at `+0x00`/`+0x20`/`+0x30`/`+0x44`/`+0x48`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TubeFrame {
    /// `+0x00`, the **lifted** centre line: the runtime record has had
    /// `pos -= 3.0 * down` applied at load, so callers lift the authored
    /// sample before handing it over.
    pub pos: Vec3,
    /// `+0x20`, the unit axis pointing into the road surface.
    pub down: Vec3,
    /// `+0x30`, the unit axis across the road, towards the right edge.
    pub lateral: Vec3,
    /// `+0x44`.
    pub half_width_left: f32,
    /// `+0x48`.
    pub half_width_right: f32,
}

/// `LeachBeam_ReaimChain`. `scale` is the failed plane test's own (negative)
/// distance, which the caller leaves in `f12` at the call.
///
/// Read literally from `0x0887334c`-`0x08873460`: the point moves to
/// `point - scale * dir`, and the step becomes `(target - (point + scale *
/// dir)) / remaining`, measured from the point **displaced the other way**, so
/// a chain re-aimed once ends `2 * scale * dir` off the target. Divided only
/// when `remaining` is non-zero. Not "corrected": that is what the code does.
fn reaim(point: &mut Vec3, step: &mut Vec3, dir: Vec3, scale: f32, target: Vec3, remaining: u32) {
    let scaled = dir * scale;
    let old_step = target - *point;
    let new_step = target - (*point + scaled);
    *point += new_step - old_step;
    *step = if remaining == 0 {
        new_step
    } else {
        new_step * (1.0 / remaining as f32)
    };
}

/// `dir = normalize(pos - point)`, with `KeepInTrack`'s own zero guard: a zero
/// length divides by `f32::MAX` (`vcst.s MaxFloat`), giving a near-zero vector.
fn towards_centre(pos: Vec3, point: Vec3) -> Vec3 {
    let to_centre = pos - point;
    let length = to_centre.length();
    to_centre * (1.0 / if length == 0.0 { f32::MAX } else { length })
}

/// `LeachBeam_KeepInTrack`, given the located frame.
///
/// Three plane tests in sequence against the one frame, each reading the point
/// the one before may have moved:
///
/// 1. below the road - `(pos - point) . down < 0` - re-aims along `-down`, an
///    exact projection onto the road plane;
/// 2. past the right wall - `(pos + lateral * (right - 2) - point) . lateral
///    < 0`;
/// 3. past the left wall, the mirror with `-lateral` and the left half-width.
///
/// The wall cases re-aim along `normalize(pos - point)`, straight at the
/// centre line rather than along `lateral`. `remaining` is how many chain
/// points are still to place after this one. Confidence **92**, measured against PPSSPP on 2026-10-01: the flow is a
/// direct read, and the frame layout matches `SplinePt` (`docs/formats/track.md`)
/// field for field, the sign of the floor test included.
pub fn keep_in_track(
    tube: &TubeFrame,
    point: &mut Vec3,
    step: &mut Vec3,
    target: Vec3,
    remaining: u32,
) {
    let below = (tube.pos - *point).dot(tube.down);
    if below < 0.0 {
        reaim(point, step, -tube.down, below, target, remaining);
    }

    let right_edge = tube.pos + tube.lateral * (tube.half_width_right - EDGE_MARGIN);
    let past_right = (right_edge - *point).dot(tube.lateral);
    if past_right < 0.0 {
        let dir = towards_centre(tube.pos, *point);
        reaim(point, step, dir, past_right, target, remaining);
    }

    let left_edge = tube.pos + -tube.lateral * (tube.half_width_left - EDGE_MARGIN);
    let past_left = (left_edge - *point).dot(-tube.lateral);
    if past_left < 0.0 {
        let dir = towards_centre(tube.pos, *point);
        reaim(point, step, dir, past_left, target, remaining);
    }
}

/// The undisplaced chain points `LeachBeam_Advance` walks: the shooter's
/// origin, then `segments` points ending near the target, each one
/// `base += step` and then kept in the tube. Element `i` is the point the
/// original's loop holds as `local_5f0` at iteration `i`, which is also where
/// it writes `WO_LEACHBEAM_ENERGY`'s matrix.
///
/// A point the locator cannot place is left alone: `AiTrack_LocatePosition`
/// then leaves its zero-initialised frame (`DAT_08a90a20` reads sixteen zero
/// bytes) in the record, and every test above computes `0`, which is not `< 0`.
pub fn walk(
    owner: Vec3,
    target: Vec3,
    segments: u32,
    locate: &dyn Fn(Vec3) -> Option<TubeFrame>,
) -> Vec<Vec3> {
    let mut step = (target - owner) / segments as f32;
    let mut base = owner;
    let mut bases = Vec::with_capacity(segments as usize + 1);
    bases.push(base);
    for i in 0..segments {
        base += step;
        if let Some(tube) = locate(base) {
            keep_in_track(&tube, &mut base, &mut step, target, segments - i - 1);
        }
        bases.push(base);
    }
    bases
}

#[cfg(test)]
mod tests;
