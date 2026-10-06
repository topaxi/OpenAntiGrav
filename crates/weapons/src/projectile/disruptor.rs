//! The Disruptor's bolt: Pure's weapon, and the one projectile on either disc that
//! hurts nobody.
//!
//! A floor follower read off its own functions, not borrowed from the Rocket:
//! `Disruptor_Init` (`0x08859010`), `Disruptor_Update` (`0x0885930c`),
//! `Disruptor_TestHit` (`0x08850274`), `DisruptorPool_Update` (`0x088506c4`), all
//! on `psp-pure-usa` and on `docs/ghidra/functions/psp-pure-usa/weapons.md`,
//! "Fire: a floor-following bolt that homes if the craft had a lock". Every
//! constant is that page's, at its confidence, and none is authored: the table
//! gives `absorb` and `speed`, and this module spends only `speed`.
//!
//! # Why it is its own arm
//!
//! [`super::flight`] carries the generic flight and the Disruptor takes none of
//! it; four differences would be four more `if kind ==` branches:
//!
//! 1. The sweep comes before the probe, and a floor across it is pushed off by
//!    `4.0` without turning the velocity.
//! 2. The probe direction is never re-read: `Disruptor_Update` writes `bolt+0x100`
//!    nowhere, so a bolt fired on a banked corner probes along that bank for life.
//! 3. A floor hit aims the velocity at the ride point, `normalize(ride - position)
//!    * speed`, with [`oag_tables::weapons::DisruptorStats::speed_for_class`]
//!    re-pinned on every floor hit and guided tick; the 500 km/h launch holds
//!    until then.
//! 4. The hit test is a swept cylinder of its own radius, see [`HIT_RADIUS`].
//!
//! # What a hit does
//!
//! Nothing to the pool or body. `Disruptor_TestHit` calls `Disruptor_ApplyEffect`,
//! which writes a kind and timer onto the victim; [`crate::disruption`] is that
//! state, and `blast::apply_impacts` routes the [`Impact`] there.

use oag_core::math::Vec3;
use oag_physics::params::Dimensions;
use oag_physics::{Ray, Raycaster, ShipState};
use oag_tables::weapons::Weapon;

use super::{Impact, Projectile};
use crate::Craft;

/// The speed a bolt leaves the craft at, in km/h.
///
/// **Recovered, confidence 84, a literal.** `Disruptor_Init` seeds `bolt+0xe0` as
/// `forward * 500.0 / 3.6` (`0x43fa0000`). The authored `speed` takes over on the
/// first floor hit, so a bolt fired off a jump flies 500 km/h until it lands.
pub const LAUNCH_KMH: f32 = 500.0;

/// How far the bolt looks along its up for a floor to ride, in units.
///
/// **Recovered, confidence 84.** `Disruptor_Update` scales `bolt+0x100` by
/// `0x41400000` = `12.0`: the Missile's, Plasma's and Shuriken's length, not the
/// Rocket's `6.0`.
pub const SURFACE_PROBE_LENGTH: f32 = 12.0;

/// How far above a floor the bolt aims to ride, in units.
///
/// **Recovered, confidence 84.** The floor arm computes `hit + normal * 6.0`
/// (`0x40c00000`): twice the Rocket's `3.0`.
pub const RIDE_HEIGHT: f32 = 6.0;

/// How far off a floor the bolt is pushed when the sweep, not the probe, meets
/// one, in units.
///
/// **Recovered, confidence 82.** The sweep's non-wall arm writes
/// `hit + normal * 4.0` (`0x40800000`) and leaves the velocity to the probe.
pub const FLOOR_PUSH_OFF: f32 = 4.0;

/// How fast a bolt with nothing under it falls, in units per second squared.
///
/// **Recovered, confidence 84**: `velocity.y -= dt * 50.0`, as the Rocket's.
pub const FALL_ACCELERATION: f32 = 50.0;

/// How fast a locked bolt turns toward its target, as a chord per second.
///
/// **Recovered, confidence 82.** The guidance arm adds `normalize(error) * 1.0 *
/// dt` (`0x3f800000`) to the unit heading before re-scaling to class speed:
/// [`super::missile::steer`]'s shape at a quarter of its `4.0`, without its clamp,
/// so it can overshoot by a hair as it lines up. Kept as read.
pub const TURN_CHORD_PER_SECOND: f32 = 1.0;

/// How far from the bolt's path a craft's centre may be and still be hit, in
/// units.
///
/// **Recovered, confidence 84.** `Disruptor_TestHit` projects each craft's centre
/// onto the tick's segment, accepting a parameter within `[-1.0, len + 1.0]` and a
/// perpendicular distance within `6.0`: a swept cylinder, not
/// [`super::hull_radius`]'s sphere.
pub const HIT_RADIUS: f32 = 6.0;

/// How far past either end of a tick's segment the cylinder still counts.
///
/// The `1.0` in `[-1.0, len + 1.0]` above, confidence 84.
pub const HIT_END_SLACK: f32 = 1.0;

/// How long a bolt flies before the pool reaps it, in seconds.
///
/// **Recovered, confidence 86.** `DisruptorPool_Update` reaps a bolt whose `+0x4c`
/// exceeds `10.0`, spawning the wall explosion. It equals
/// [`super::MAX_FLIGHT_SECONDS`] by coincidence (that one is ours, this the
/// disc's); the assert below makes a drift a compile-time error.
pub const MAX_FLIGHT_SECONDS: f32 = 10.0;

const _: () = assert!(MAX_FLIGHT_SECONDS == super::MAX_FLIGHT_SECONDS);

/// Where a bolt starts, which way it flies and which way it probes.
///
/// `Disruptor_Init` copies the craft's matrix whole: translation is the position,
/// the forward row times [`LAUNCH_KMH`] the velocity, the up row the probe
/// direction kept for life. No muzzle offset, unlike the Rocket and Missile: the
/// sweep excludes the owner. `dimensions` is unread, matching the other `launch`
/// signatures.
#[must_use]
pub fn launch(state: &ShipState, dimensions: &Dimensions) -> (Vec3, Vec3, Vec3) {
    let _ = dimensions;
    let forward = state.body.forward();
    let up = state.body.up();
    let velocity = forward * (LAUNCH_KMH / super::KMH_PER_UNIT_PER_SECOND);
    (state.body.position, velocity, up)
}

/// Turns `velocity` toward `target` by one tick's chord and re-pins its speed.
///
/// `Disruptor_Update`'s guidance arm in order: unit vector to target, unit
/// heading, unit error, `heading + error * TURN_CHORD * dt`, scale to `speed`. Not
/// renormalised between add and scale, as [`super::missile::steer`] documents.
#[must_use]
pub fn steer(velocity: Vec3, position: Vec3, target: Vec3, dt: f32, speed: f32) -> Vec3 {
    let to_target = target - position;
    if to_target.length_squared() <= 0.0 || velocity.length_squared() <= 0.0 {
        return velocity;
    }
    let desired = to_target.normalize();
    let heading = velocity.normalize();
    let error = desired - heading;
    let turned = if error.length_squared() > 0.0 {
        heading + error.normalize() * (TURN_CHORD_PER_SECOND * dt)
    } else {
        heading
    };
    turned * speed
}

/// Whether the segment `from -> to` passes within [`HIT_RADIUS`] of `centre`:
/// `Disruptor_TestHit`'s cylinder, parameter within [`HIT_END_SLACK`] of either
/// end. A zero-length segment tests the point.
#[must_use]
pub fn cylinder_hit(from: Vec3, to: Vec3, centre: Vec3) -> bool {
    let axis = to - from;
    let length = axis.length();
    let offset = centre - from;
    if length <= 0.0 {
        return offset.length() <= HIT_RADIUS;
    }
    let direction = axis / length;
    let along = offset.dot(direction);
    if along < -HIT_END_SLACK || along > length + HIT_END_SLACK {
        return false;
    }
    let perpendicular = offset - direction * along;
    perpendicular.length() <= HIT_RADIUS
}

/// Flies one bolt one tick. `Some` where it stopped on a wall or through a craft;
/// `None` while going or just aged out (see [`Projectile::lifetime`]).
///
/// `speed` is the class's `speed_for_class`, or `None` for no Disruptor block or
/// an off-ladder class: the bolt keeps its speed. Order is `Disruptor_Update`'s:
/// age, sweep, probe, guide, move.
pub(super) fn advance<R: Raycaster + ?Sized, S: Craft>(
    projectile: &mut Projectile,
    dt: f32,
    raycaster: &R,
    ships: &[S],
    speed: Option<f32>,
) -> Option<Impact> {
    let from = projectile.position;
    let mut to = from + projectile.velocity * dt;
    let stop = |point: Vec3, struck: Option<u8>, projectile: &Projectile| Impact {
        point,
        kind: Weapon::Disruptor,
        owner: projectile.owner,
        struck,
        // A craft takes the effect, a wall nothing; `blast::apply_impacts` reads
        // `effect`, not this flag.
        blast: struck.is_some(),
        effect: struck.and(projectile.effect),
    };

    // 1. The travel sweep. A wall ends the flight; a floor pushes the destination
    //    off and leaves the velocity to the probe.
    let step = to - from;
    let distance = step.length();
    if distance > 0.0 {
        let ray = Ray::new(from, step / distance, distance);
        if let Some(hit) = Raycaster::raycast(raycaster, ray, None, false) {
            if hit.surface.is_hoverable() {
                to = hit.point + hit.normal * FLOOR_PUSH_OFF;
            } else {
                return Some(stop(hit.point, None, projectile));
            }
        }
    }

    // 2. The probe along the fired-with up. Nothing: fall. Floor: aim at the ride
    //    point and re-pin speed. Wall: dead.
    let probe = Raycaster::raycast(
        raycaster,
        Ray::new(to, -projectile.surface, SURFACE_PROBE_LENGTH),
        None,
        false,
    );
    match probe {
        None => projectile.velocity -= Vec3::Y * FALL_ACCELERATION * dt,
        Some(hit) if hit.surface.is_hoverable() => {
            let ride = hit.point + hit.normal * RIDE_HEIGHT;
            let aim = ride - from;
            if aim.length_squared() > 0.0 {
                let pinned = speed.map_or_else(
                    || projectile.velocity.length(),
                    |kmh| kmh / super::KMH_PER_UNIT_PER_SECOND,
                );
                projectile.velocity = aim.normalize() * pinned;
                to = from + projectile.velocity * dt;
            }
        }
        Some(hit) => return Some(stop(hit.point, None, projectile)),
    }

    // 3. Guidance off the tick's start position, then speed re-pinned (the
    //    original's): a locked bolt holds class speed where an unlocked one falls.
    if let Some(target) = projectile.target
        && let Some(ship) = ships.get(target as usize).filter(|s| s.active())
        && let Some(kmh) = speed
    {
        projectile.velocity = steer(
            projectile.velocity,
            from,
            ship.physics().body.position,
            dt,
            kmh / super::KMH_PER_UNIT_PER_SECOND,
        );
    }

    // 4. The hit test over the segment flown this tick, every craft but the owner:
    //    `DisruptorPool_Update` runs `Disruptor_TestHit` after `Disruptor_Update`.
    for (slot, ship) in ships.iter().enumerate() {
        if !ship.active() || slot as u8 == projectile.owner {
            continue;
        }
        if cylinder_hit(from, to, ship.physics().body.position) {
            projectile.position = to;
            return Some(stop(to, Some(slot as u8), projectile));
        }
    }

    projectile.position = to;
    projectile.lifetime -= dt;
    None
}

#[cfg(test)]
mod tests;
