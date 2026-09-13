//! The geometry a projectile's tick is resolved with: what its swept segment
//! hits first, how big a craft is to a projectile, and where a segment enters a
//! sphere.
//!
//! Split out of `projectile.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py` on the day the Plasma's wind-up took that file
//! past it; a move, with no behaviour change. All three items are re-exported
//! from [`super`], so `projectile::hull_radius` and friends still resolve and
//! no call site moved.
//!
//! The seam is a real one: nothing here knows what a weapon is. It takes
//! points, segments and dimensions and answers questions about them, which is
//! why the two `ours`-rather-than-recovered readings in this module - the hull
//! sphere and its radius - sit together rather than beside the flight model
//! they are consumed by.

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
    ///
    /// **This is the original's collision code.** `FUN_0883198c`, the query
    /// every projectile update runs, returns `0x7f` for no hit, the struck
    /// collider's `+0x6c` surface type on a world hit - `0` wall, `1` floor,
    /// `3` mag floor - and `4` from a second query against the craft. The
    /// physics layer carries the same numbering in [`Surface`]; a hull is the
    /// `4`. See `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
    pub surface: Option<Surface>,
}

/// The nearest of the geometry hit and the hull hits along one tick's step.
///
/// **Every geometry hit is reported, whatever its angle.** This used to drop
/// a hit that was not face-on, on the reasoning that a projectile riding the
/// floor clips it constantly; that let a projectile pass *through a wall* it
/// met at a shallow angle and leave the circuit, and it detonated on a floor
/// it met steeply. Which of the two a hit is comes back in
/// [`SweepHit::surface`], and [`super::Projectiles::advance`] decides from
/// that, the way the original decides from its code.
pub(super) fn nearest_hit<R: Raycaster + ?Sized>(
    from: Vec3,
    to: Vec3,
    distance: f32,
    raycaster: &R,
    ships: &[crate::world::Ship],
    owner: u8,
) -> Option<SweepHit> {
    let direction = (to - from) / distance;

    // `include_reset` is false: a `Reset Collision` volume is a respawn trigger
    // rather than a surface, and a rocket detonating on one would blow up in
    // mid-air over the run-off. The original's query skips them the same way
    // - `Collision_RaycastWorld`'s Reset skip, with the request bit clear.
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
        if !ship.active || slot as u8 == owner {
            continue;
        }
        let radius = hull_radius(&ship.handling.dimensions);
        let Some(t) = segment_sphere(from, to, ship.physics.body.position, radius) else {
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
/// **Ours.** `<Misc>` authors a `length`, a `width` and a `height` and the
/// physics builds a box from them (`oag_physics::wall::hull_extent`), but
/// nothing has been read about what a *projectile* is tested against.
///
/// Half the largest dimension is the sphere that **circumscribes** the box's
/// longest axis, and it is worth being exact about which way that errs. A hull
/// of `length 4, width 2, height 1` has half-extents `(2.0, 1.0, 0.5)` and a
/// radius of `2.0`, so the sphere matches the box nose-to-tail and **bulges
/// past it on the other two axes**: a rocket passing 1.8 units to the side hits,
/// where the box would have missed. So this is the *generous* reading, not the
/// conservative one - it favours the shooter, and a near miss can register as a
/// hit.
///
/// That is a defensible placeholder rather than the right answer: the smallest
/// half-extent (`0.5` here) would be conservative and would make most visually
/// solid hits miss, which reads as a broken weapon. Sizing to the hull's length
/// keeps a craft-sized target. Whoever recovers what the original tests should
/// replace the whole function rather than tune this number.
///
/// Reusing the box would mean a segment-vs-oriented-box test for a mechanic
/// where nothing is known about the original's own shape, which is precision
/// with no evidence under it.
#[must_use]
pub fn hull_radius(dimensions: &Dimensions) -> f32 {
    0.5 * dimensions
        .length
        .max(dimensions.width)
        .max(dimensions.height)
}

/// Where a segment first enters a sphere, as a fraction of the segment.
///
/// `None` when it misses, or when both roots lie outside `0..=1`. A segment that
/// *starts* inside returns `0.0`, which is the answer a launch that overlaps a
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

    // Already inside. Not folded into the quadratic below: with `c <= 0` the
    // near root is negative and would be rejected, which would let a projectile
    // spawned inside a hull fly out through it.
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
