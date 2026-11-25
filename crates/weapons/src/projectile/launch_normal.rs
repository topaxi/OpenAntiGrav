//! Pure's Rocket and Plasma flight: the travel sweep first, then a surface probe along the
//! normal the projectile was launched with, which it carries unchanged to the end.
//!
//! `Rocket_Update` (`0x0885f28c`) and `Plasma_Update` (`0x0885e348`) on Pure's executable
//! write the carried normal nowhere: only `Rocket_Init` (`+0xd0`) and `Plasma_Launch` (`+0x110`)
//! store it, as the launch matrix's up row, which carries the craft's display scale (length
//! 0.75). Every length taken along it is therefore three quarters of its literal. Watched live on
//! PPSSPP: the stored normal held for every update of ten flights, over a floor 15 degrees off it
//! on Sebenco Climb (`docs/ghidra/functions/psp-pure-usa/floor-follow.md`).

use super::{FALL_ACCELERATION, Impact, Projectile, nearest_hit, own_box_blocks};
use crate::Craft;
use oag_core::math::Vec3;
use oag_physics::{Ray, Raycaster};
use oag_tables::weapons::Weapon;

/// The length of the launch matrix's up row: the craft's display scale.
pub const NORMAL_LENGTH: f32 = oag_physics::hover::TARGET_GLOBAL_SCALE;

/// The probe's literal, times the carried normal's length (`0x41400000` at `0x0885f51c` and
/// `0x0885e498`).
pub const PROBE_LENGTH: f32 = 12.0 * NORMAL_LENGTH;

/// The travel sweep's push-out along the hit normal (unit), both weapons (`0x40400000` at
/// `0x0885f4b8` and `0x0885e44c`).
pub const SWEEP_RIDE_HEIGHT: f32 = 3.0;

/// The probe arm's ride height along the carried normal: the Rocket's `3.0` (`0x0885f6dc`) and
/// the Plasma's `6.0` (`0x0885e534`), each times the normal's length.
#[must_use]
pub fn probe_ride_height(kind: Weapon) -> f32 {
    let literal = if kind == Weapon::Plasma { 6.0 } else { 3.0 };
    literal * NORMAL_LENGTH
}

/// One update of a Pure Rocket or Plasma: moves `projectile` and returns the impact if it ended.
///
/// `from` is its position at the start of the update; `pinned_kmh` the speed the probe's floor
/// arm renormalises to. A probe that meets the firing craft's box runs no arm (`box_holds`).
#[allow(clippy::too_many_arguments)]
pub(super) fn advance<R: Raycaster + ?Sized, S: Craft>(
    projectile: &mut Projectile,
    kind: Weapon,
    from: Vec3,
    dt: f32,
    raycaster: &R,
    ships: &[S],
    box_holds: bool,
    pinned_kmh: Option<f32>,
) -> Option<Impact> {
    let mut to = from + projectile.velocity * dt;
    let wall = |point: Vec3, struck: Option<u8>| Impact {
        point,
        kind,
        owner: projectile.owner,
        struck,
        blast: struck.is_some(),
        effect: None,
    };

    // The travel sweep: a floor across the step pushes the end out along the hit's own normal
    // and writes nothing else; a wall ends the flight.
    let step = to - from;
    let distance = step.length();
    if distance > 0.0
        && let Some(hit) = nearest_hit(from, to, distance, raycaster, ships, projectile.owner)
    {
        if hit.struck.is_none() && hit.surface.is_some_and(oag_physics::Surface::is_hoverable) {
            to = hit.point + hit.normal * SWEEP_RIDE_HEIGHT;
        } else {
            return Some(wall(hit.point, hit.struck));
        }
    }

    // The probe, from the swept end along the carried normal.
    let probe_end = to - projectile.surface * PROBE_LENGTH;
    let mesh = Raycaster::raycast(
        raycaster,
        Ray::new(to, -projectile.surface, PROBE_LENGTH),
        None,
        false,
    );
    let boxed = box_holds
        && ships.get(projectile.owner as usize).is_some_and(|ship| {
            ship.active()
                && own_box_blocks(
                    &ship.physics().body,
                    &ship.dimensions(),
                    to,
                    probe_end,
                    mesh.map(|hit| hit.distance),
                )
        });
    match mesh {
        _ if boxed => {}
        Some(hit) if hit.surface.is_hoverable() => {
            // The ride point is offset along the carried normal, not the floor's.
            let ride = hit.point + projectile.surface * probe_ride_height(kind);
            let toward = (ride - from) / dt;
            if toward.length_squared() > 0.0 {
                let speed = pinned_kmh
                    .map_or_else(|| toward.length(), super::missile::speed_units_on_surface);
                projectile.velocity = toward.normalize() * speed;
                to = from + projectile.velocity * dt;
            }
        }
        Some(hit) => return Some(wall(hit.point, None)),
        // Both weapons fall along world down (`0x0885f814`, `0x0885e668`).
        None => projectile.velocity -= Vec3::Y * (FALL_ACCELERATION * dt),
    }
    projectile.position = to;
    None
}
