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
    /// Which surface class the geometry is, or `None` for a hull.
    /// Which surface class the geometry is, or `None` for a hull: the original's
    /// collision code. `FUN_0883198c` returns `0x7f` for no hit, the collider's
    /// `+0x6c` type on a world hit (`0` wall, `1` floor, `3` mag floor) and `4`
    /// from the craft query, the numbering [`Surface`] carries. See
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
