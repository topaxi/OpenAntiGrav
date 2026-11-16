//! The geometry a projectile's tick is resolved with: what its swept segment hits
//! first, how big a craft is to a projectile, and where a segment enters a sphere.
//!
//! Split from `projectile.rs` under the 1,000-line rule; re-exported from
//! [`super`]. Nothing here knows what a weapon is, which is why the two ours
//! readings (the hull sphere and its radius) sit together here.

use crate::Craft;
use oag_core::math::Vec3;
use oag_physics::params::Dimensions;
use oag_physics::{Ray, Raycaster, Surface};

/// What a swept segment met first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SweepHit {
    /// Where along the segment it happened.
    pub point: Vec3,
    /// The ship slot struck, or `None` for track geometry.
    pub struck: Option<u8>,
    /// The surface normal for geometry; the incoming direction reversed for a
    /// hull, which nothing reads - a hull hit always detonates.
    pub normal: Vec3,
    /// Which surface class the geometry is, or `None` for a hull: the original's
    /// collision code. `Collision_SweepSegment` (`0x0883198c`) returns `0x7f` for no
    /// hit and the collider's `+0x6c` type on a world hit (`0` wall, `1` floor, `3` mag
    /// floor), the numbering [`Surface`] carries; its `4`, a segment end outside the
    /// collision cage, is not a surface and is asked of the cage in
    /// [`super::Projectiles::advance`]. See
    /// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
    pub surface: Option<Surface>,
}

/// The nearest of the geometry hit and the hull hits along one tick's step.
///
/// Every geometry hit is reported, whatever its angle: dropping non-face-on hits
/// let a projectile pass through a wall met at a shallow angle and detonate on a
/// steep floor. [`SweepHit::surface`] says which, and
/// [`super::Projectiles::advance`] decides from it, as the original does from its
/// code.
pub(super) fn nearest_hit<R: Raycaster + ?Sized, S: Craft>(
    from: Vec3,
    to: Vec3,
    distance: f32,
    raycaster: &R,
    ships: &[S],
    owner: u8,
) -> Option<SweepHit> {
    let direction = (to - from) / distance;

    // `include_reset` is false: a `Reset Collision` volume is a respawn trigger, not
    // a surface, and the original's query skips it (`Collision_RaycastWorld`'s
    // Reset skip, request bit clear).
    let mut best = Raycaster::raycast(raycaster, Ray::new(from, direction, distance), None, false)
        .map(|hit| {
            (
                hit.distance,
                SweepHit {
                    point: hit.point,
                    struck: None,
                    normal: hit.normal,
                    surface: Some(hit.surface),
                },
            )
        });

    for (slot, ship) in ships.iter().enumerate() {
        if !ship.active() || slot as u8 == owner {
            continue;
        }
        let radius = hull_radius(&ship.dimensions());
        let Some(t) = segment_sphere(from, to, ship.physics().body.position, radius) else {
            continue;
        };
        let travelled = t * distance;
        if best.is_none_or(|(nearest, _)| travelled < nearest) {
            best = Some((
                travelled,
                SweepHit {
                    point: from + direction * travelled,
                    struck: Some(slot as u8),
                    normal: -direction,
                    surface: None,
                },
            ));
        }
    }

    best.map(|(_, hit)| hit)
}

/// The sphere a craft is tested against.
///
/// **Ours.** `<Misc>` authors `length`, `width` and `height` and the physics builds
/// a box (`oag_physics::wall::hull_extent`), but nothing is read about what a
/// projectile is tested against.
///
/// Half the largest dimension circumscribes the longest axis, so it errs
/// generous: a hull of `length 4, width 2, height 1` has half-extents `(2.0, 1.0,
/// 0.5)` and radius `2.0`, bulging past the box on the other two axes (a rocket
/// 1.8 units to the side hits). The smallest half-extent would be conservative and
/// make most visually solid hits miss. A placeholder: whoever recovers the
/// original's test should replace the function, not tune this. A segment-vs-box
/// test would be precision with no evidence under it.
#[must_use]
pub fn hull_radius(dimensions: &Dimensions) -> f32 {
    0.5 * dimensions
        .length
        .max(dimensions.width)
        .max(dimensions.height)
}

/// Where a segment first enters a sphere, as a fraction of the segment. `None`
/// when it misses or both roots lie outside `0..=1`; a segment that starts inside
/// returns `0.0`, as a launch overlapping a hull needs.
/// hull needs.
pub(super) fn segment_sphere(p0: Vec3, p1: Vec3, centre: Vec3, radius: f32) -> Option<f32> {
    let d = p1 - p0;
    let m = p0 - centre;
    let a = d.dot(d);
    if a <= 0.0 {
        return None;
    }
    let b = m.dot(d);
    let c = m.dot(m) - radius * radius;

    // Already inside, kept out of the quadratic: with `c <= 0` the near root is
    // negative and a projectile spawned inside a hull would fly out through it.
    if c <= 0.0 {
        return Some(0.0);
    }
    // Heading away, and outside.
    if b >= 0.0 {
        return None;
    }

    let discriminant = b * b - a * c;
    if discriminant < 0.0 {
        return None;
    }
    let t = (-b - discriminant.sqrt()) / a;
    if (0.0..=1.0).contains(&t) {
        Some(t)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;

/// How a flyer's first surface probes treat the firing craft.
///
/// `Collision_RaycastWorld` sees every craft's box collider, the firer's included, so the
/// original's probes miss on their own craft until they are clear of its box
/// (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "The own-craft box is a rotated
/// frame"). A title that has not measured that keeps a plain age window for the Plasma and
/// lets every other flyer probe from the first update.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OwnCraftProbe {
    /// The firing craft's box blocks a probe that crosses it before the mesh does, seen through
    /// the transposed rotation ([`own_box_blocks`]), and the blocked probe falls (Pulse).
    CraftBox,
    /// The same box, but a blocked probe runs neither the fall nor the ride: the projectile keeps
    /// its velocity that update (Pure, `docs/ghidra/functions/psp-pure-usa/own-craft-box.md`).
    CraftBoxHolds,
    /// No box in the probe's query: every flyer probes the track from its first update (2048 and
    /// Omega, `docs/ghidra/functions/vita-2048-eu-v104/own-craft-box.md`).
    NoBox,
    /// No box: a Plasma does not probe for this many seconds after its release (chosen).
    PlasmaWindow(f32),
}

/// Whether the firing craft's box collider hides the track from a probe.
///
/// `Collision_RaycastBox` (`0x08817b1c`) first runs a separating-axis test of the segment
/// against the half dimensions, in a frame built from the collider's rotation rows by
/// `vtfm4.q`, which is `R^T * (p - centre)`, the transpose of the frame that would put a point in
/// the box's own axes; only a segment that passes crosses the box's faces, in world space. So the
/// reach ahead of a craft is 4.875 aligned to the world axes and 2.06 at 45 degrees of yaw
/// (`rocket-visuals.md`, confidence 90: 2,302 recorded probes replayed, 0 mispredicted).
///
/// `start` to `end` is the probe segment; `mesh_distance` is how far along it the track mesh was
/// met, if it was. The box blocks only when its nearest face is strictly nearer.
///
/// The box is centred on the body with the original's rows, `(-right, up, forward)`
/// (row 0 measured as the ship's left, `oag_trace::replay::Basis`), and half dimensions of
/// `(width, height, length) * 0.75 * 0.5` (`TARGET_GLOBAL_SCALE`).
#[must_use]
pub fn own_box_blocks(
    body: &oag_physics::Body,
    dimensions: &Dimensions,
    start: Vec3,
    end: Vec3,
    mesh_distance: Option<f32>,
) -> bool {
    let scale = oag_physics::hover::TARGET_GLOBAL_SCALE * 0.5;
    let extent = [
        dimensions.width * scale,
        dimensions.height * scale,
        dimensions.length * scale,
    ];
    let rows = [-body.right(), body.up(), body.forward()];
    let centre = body.position;
    // The gate's frame: the columns of the row matrix.
    let columns = [
        Vec3::new(rows[0].x, rows[1].x, rows[2].x),
        Vec3::new(rows[0].y, rows[1].y, rows[2].y),
        Vec3::new(rows[0].z, rows[1].z, rows[2].z),
    ];
    let local = |point: Vec3| {
        let offset = point - centre;
        [
            columns[0].dot(offset),
            columns[1].dot(offset),
            columns[2].dot(offset),
        ]
    };
    if !separating_axis_passes(local(start), local(end), extent) {
        return false;
    }
    let Some(t) = face_crossing(&rows, centre, extent, start, end) else {
        return false;
    };
    let length = (end - start).length();
    mesh_distance.is_none_or(|mesh| t * length < mesh)
}

/// The segment's midpoint and half direction against the half dimensions, the three axis
/// tests and the three cross products, as `Collision_RaycastBox` runs them.
fn separating_axis_passes(start: [f32; 3], end: [f32; 3], extent: [f32; 3]) -> bool {
    let mid: [f32; 3] = std::array::from_fn(|i| (start[i] + end[i]) * 0.5);
    let half: [f32; 3] = std::array::from_fn(|i| mid[i] - start[i]);
    let a: [f32; 3] = std::array::from_fn(|i| half[i].abs());
    if (0..3).any(|i| mid[i].abs() > a[i] + extent[i]) {
        return false;
    }
    (mid[1] * half[2] - mid[2] * half[1]).abs() <= a[2] * extent[1] + a[1] * extent[2]
        && (mid[0] * half[2] - mid[2] * half[0]).abs() <= a[2] * extent[0] + a[0] * extent[2]
        && (mid[0] * half[1] - mid[1] * half[0]).abs() <= a[1] * extent[0] + a[0] * extent[1]
}

/// The smallest `t` in `(0, 1]` at which the segment crosses a face of the true oriented box.
fn face_crossing(
    rows: &[Vec3; 3],
    centre: Vec3,
    extent: [f32; 3],
    start: Vec3,
    end: Vec3,
) -> Option<f32> {
    let direction = end - start;
    let offset = start - centre;
    let local_start = [
        rows[0].dot(offset),
        rows[1].dot(offset),
        rows[2].dot(offset),
    ];
    let local_dir = [
        rows[0].dot(direction),
        rows[1].dot(direction),
        rows[2].dot(direction),
    ];
    let mut nearest: Option<f32> = None;
    for axis in 0..3 {
        if local_dir[axis].abs() < 1e-9 {
            continue;
        }
        for sign in [-1.0_f32, 1.0] {
            let t = (sign * extent[axis] - local_start[axis]) / local_dir[axis];
            if t > 0.0 && t <= 1.0 {
                let inside = (0..3)
                    .filter(|&j| j != axis)
                    .all(|j| (local_start[j] + local_dir[j] * t).abs() <= extent[j] + 1e-4);
                if inside && nearest.is_none_or(|best| t < best) {
                    nearest = Some(t);
                }
            }
        }
    }
    nearest
}
