//! Pulse PSP's grid walk: where `Race_ComputeGridLayout` puts each of the eight
//! slots, and which way each one faces.
//!
//! **Recovered from the original and checked against it three times over**
//! (2026-10-02, `docs/ghidra/functions/psp-pulse-usa/grid.md`): `Race_ComputeGridLayout`
//! (`0x0882b3b0`) locates the authored node on the track's spline, lays slot 8 on it, and
//! then, for each further slot, steps `19.8` units along the **located record's own
//! tangent** and locates again - `p(k+1) = locate(p(k) + tangent(k) * 19.8)` - so the
//! slots follow the curve and are not on a fixed spacing of samples. The heading is
//! `FUN_0882663c`'s: not the tangent but the unit sum of the left-edge and right-edge
//! chords over the next 20 units. Two properties of the original's locate matter more than
//! the rest, and both were fitted wrongly by earlier attempts before being read:
//!
//! - **It is a projection onto the curve, not a nearest sample.** `AiTrack_LocatePosition`
//!   takes the nearest control point, picks the segment between it and its nearer
//!   neighbour, and runs three Gauss-Newton steps from `t = 0.5` on the uniform cubic
//!   B-spline of the **lifted** control points (`pos - 3 * down`), clamping `t` to
//!   `0..=1` last (`FUN_0887c340`, `FUN_0887c1e0`). Resampling the spline at four samples a
//!   segment, as the rest of this project does, is where the old walk's 1.5-unit sawtooth
//!   came from.
//! - **The record it writes is scaled by `0.999756`.** `FUN_0887c7e8` blends the record
//!   with weights it multiplies by `vfim.s 0x3155`, a half-float immediate - `0.16662598`,
//!   where `1/6` is `0.16666667` - so every field of the located record (position,
//!   tangent, down, lateral, widths, the `w` lane) is `8190/8192` of the true blend. Read
//!   live: the `w` lane of every record on Metropia is exactly `0.999755859375`. The
//!   original's position scales toward the **world origin**, which is `0.00024` of the
//!   coordinate - 0.17 units at `x = -721` - and the step along the shortened tangent
//!   loses `0.00024` of 19.8. Both feed the chain, and dropping them is what put
//!   `01_Track`'s slots up to 0.5 units from the original's instead of 0.001.
//!
//! Measured against the original's eight craft on three circuits (xz, units): `01_Track`
//! 0.001 (it was 1.73), `16_Track` 0.043 (0.62), Metropia reversed 0.070 (1.13); headings
//! to 0.001 degrees on `16_Track` and `01_Track`, where the tangent was 0.045 out and the
//! `01_Track` slot-1 kink (0.22 degrees) was unexplained. See
//! `crates/game/tests/grid_walk_ground_truth.rs`.

use oag_core::math::{Quat, Vec3};
use oag_vex::track::{AiTrack, Path, SplinePoint, basis};

use crate::spawn::{GRID_COLUMN_OFFSET, GRID_ROW_PITCH, GRID_SLOTS, Pose, orientation_from_axes};

/// What `FUN_0887c7e8` multiplies every field of a located record by: `6 * 0x3155`, the
/// weights' own `vfim.s` half-float immediate (`0.1666259765625`) in place of `1/6`.
/// Exact in `f32`, `8190 / 8192`.
pub const RECORD_SCALE: f32 = 8190.0 / 8192.0;

/// How far ahead of a slot the heading looks: the `20.0` (`0x41a00000`) in `FUN_0882663c`.
pub const HEADING_LOOKAHEAD: f32 = 20.0;

/// The most the authored node may sit off its located centreline sample along the
/// sample's own down axis before the walk is not trusted.
///
/// **Chosen, not measured.** Twenty-three of the twenty-four circuit-directions put the
/// node 0.3 to 4.4 units off (it is a lateral stagger, not a height), and
/// `25_Track` reversed puts it 25.9 off: its nearest control point is on the ramp above.
/// What the original does there was not captured, so the caller keeps the node-anchored
/// grid on it.
pub const MAX_NODE_HEIGHT_OFFSET: f32 = 12.0;

/// A located spline sample, in the running game's form: the record
/// `AiTrack_LocatePosition` writes, which is what the walk reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Located {
    /// The centreline point, **lifted** (`pos - 3 * down`) and scaled by [`RECORD_SCALE`].
    pub position: Vec3,
    /// The interpolated tangent, scaled and not renormalised.
    pub tangent: Vec3,
    /// The interpolated down axis (into the surface), scaled.
    pub down: Vec3,
    /// The interpolated lateral axis (to the driver's right), scaled.
    pub lateral: Vec3,
    /// Half-width to the left of the centreline, scaled.
    pub half_width_left: f32,
    /// Half-width to the right, scaled.
    pub half_width_right: f32,
}

impl Located {
    /// The track's left edge at this sample.
    #[must_use]
    pub fn left_edge(&self) -> Vec3 {
        self.position - self.lateral * self.half_width_left
    }

    /// The track's right edge at this sample.
    #[must_use]
    pub fn right_edge(&self) -> Vec3 {
        self.position + self.lateral * self.half_width_right
    }
}

fn lifted(point: &SplinePoint) -> Vec3 {
    Vec3::from_array(point.lifted_pos())
}

fn control(path: &Path, index: isize) -> &SplinePoint {
    let last = path.points.len().saturating_sub(1) as isize;
    &path.points[index.clamp(0, last) as usize]
}

/// The uniform cubic B-spline's derivative weights, the second polynomial
/// `FUN_0887c1e0` evaluates.
fn basis_derivative(t: f32) -> [f32; 4] {
    let t2 = t * t;
    let sixth = 1.0 / 6.0;
    [
        (-3.0 * t2 + 6.0 * t - 3.0) * sixth,
        (9.0 * t2 - 12.0 * t) * sixth,
        (-9.0 * t2 + 6.0 * t + 3.0) * sixth,
        3.0 * t2 * sixth,
    ]
}

/// `AiTrack_LocatePosition(100.0, track, &record, position, 0, -1, 0)` on a fresh cursor:
/// the record of the curve's nearest point to `position`.
///
/// `None` when the track has no control points.
///
/// **Not ported: the hash cache and the junction hop.** The original seeds its search
/// from a spatial hash and steps its window across a junction at a path's end; this
/// scans every control point and clamps a window at the path's ends. The grids of all
/// twenty-four circuit-directions stay within the interior of one path except
/// `03_Track`, whose front slot sits one control point from its path's end, where the
/// clamp moves a slot by under a tenth of a unit.
#[must_use]
pub fn locate(ai: &AiTrack, position: Vec3) -> Option<Located> {
    let mut nearest: Option<(f32, usize, usize)> = None;
    for (path_index, path) in ai.paths.iter().enumerate() {
        for (index, point) in path.points.iter().enumerate() {
            let distance = (lifted(point) - position).length_squared();
            // Strictly nearer, so a tie keeps the earlier point.
            if nearest.is_none_or(|(best, _, _)| distance < best) {
                nearest = Some((distance, path_index, index));
            }
        }
    }
    let (_, path_index, index) = nearest?;
    let path = &ai.paths[path_index];
    let at = index as isize;

    // The segment between the nearest control point and whichever neighbour is nearer.
    let previous = (lifted(control(path, at - 1)) - position).length_squared();
    let next = (lifted(control(path, at + 1)) - position).length_squared();
    let segment = if next < previous { at } else { (at - 1).max(0) };
    let window = [
        control(path, segment - 1),
        control(path, segment),
        control(path, segment + 1),
        control(path, segment + 2),
    ];
    let corners = window.map(lifted);

    // Three Gauss-Newton steps on the squared distance from t = 0.5, unclamped inside the
    // loop and clamped once after it (`FUN_0887c340`).
    let mut t = 0.5f32;
    for _ in 0..3 {
        let weights = basis(t);
        let slope = basis_derivative(t);
        let mut point = Vec3::ZERO;
        let mut derivative = Vec3::ZERO;
        for k in 0..4 {
            point += corners[k] * weights[k];
            derivative += corners[k] * slope[k];
        }
        let speed = derivative.dot(derivative);
        if speed <= 0.0 {
            break;
        }
        t -= (point - position).dot(derivative) / speed;
    }
    let t = t.clamp(0.0, 1.0);

    // The record, blended with the original's own (slightly short) weights.
    let weights = basis(t).map(|w| w * RECORD_SCALE);
    let blend = |field: fn(&SplinePoint) -> Vec3| {
        let mut out = Vec3::ZERO;
        for k in 0..4 {
            out += field(window[k]) * weights[k];
        }
        out
    };
    let scalar = |field: fn(&SplinePoint) -> f32| {
        let mut out = 0.0;
        for k in 0..4 {
            out += field(window[k]) * weights[k];
        }
        out
    };
    Some(Located {
        position: blend(lifted),
        tangent: blend(|p| Vec3::from_array(p.tangent)),
        down: blend(|p| Vec3::from_array(p.down)),
        lateral: blend(|p| Vec3::from_array(p.lateral)),
        half_width_left: scalar(|p| p.half_width_left),
        half_width_right: scalar(|p| p.half_width_right),
    })
}

/// `FUN_0882663c`: the heading at a located point.
///
/// Locates the point again, steps [`HEADING_LOOKAHEAD`] along its tangent, locates that,
/// and returns the unit sum of the unit chord of the left edge and the unit chord of the
/// right edge between the two. Forward is therefore the direction the *track's edges*
/// run, which on a widening or bending straight is not the centreline's tangent.
fn heading(ai: &AiTrack, at: Vec3) -> Option<Vec3> {
    let here = locate(ai, at)?;
    let ahead = locate(ai, here.position + here.tangent * HEADING_LOOKAHEAD)?;
    let left = (ahead.left_edge() - here.left_edge()).normalize_or_zero();
    let right = (ahead.right_edge() - here.right_edge()).normalize_or_zero();
    Some((left + right).normalize_or_zero())
}

/// The eight slots of a grid, slot 1 first.
///
/// Positions are the **lifted centreline walk plus the lateral stagger**: the world
/// height is the located sample's, three units above its surface line, and callers drop
/// it onto the collision under it as the original's own raycasts do. The orientation is
/// [`heading`] with the sample's up.
///
/// `node` is the authored `Start Position`'s position. `None` when the track has no
/// control points, or when the node is further than [`MAX_NODE_HEIGHT_OFFSET`] from the
/// sample it locates along that sample's down axis.
///
/// Always steps **forward** along the located tangent, never the decompile's
/// reversed branch (`-19.8`, slot order flipped): Metropia reversed is laid out forward,
/// measured, and no Pulse circuit-direction's node faces against its spline.
#[must_use]
pub fn walk(ai: &AiTrack, node: Vec3) -> Option<[Pose; GRID_SLOTS as usize]> {
    let mut at = locate(ai, node)?;
    let down = at.down.normalize_or_zero();
    if (node - at.position).dot(down).abs() > MAX_NODE_HEIGHT_OFFSET {
        return None;
    }
    // Which edge the node is nearer sets the side of the first `10.0`, and the sides
    // alternate from there.
    let half = 0.5 * GRID_COLUMN_OFFSET;
    let mut sign = if (node - at.left_edge()).length() < (node - at.right_edge()).length() {
        -half
    } else {
        half
    };

    let mut slots = [Pose {
        position: Vec3::ZERO,
        orientation: Quat::IDENTITY,
    }; GRID_SLOTS as usize];
    for index in (0..GRID_SLOTS as usize).rev() {
        // The midpoint of the two edges, and the unit direction from it to the right edge.
        let left = at.left_edge();
        let right = at.right_edge();
        let middle = 0.5 * (left + right);
        let across = right - middle;
        let across = if across.length_squared() > 1e-4 {
            across.normalize_or_zero()
        } else {
            across
        };
        let forward = heading(ai, at.position)?;
        slots[index] = Pose {
            position: middle + across * sign,
            orientation: orientation_from_axes(forward, -at.down),
        };
        if index > 0 {
            at = locate(ai, at.position + at.tangent * GRID_ROW_PITCH)?;
            sign = -sign;
        }
    }
    Some(slots)
}

#[cfg(test)]
mod tests;
