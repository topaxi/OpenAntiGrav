//! The Disruptor's bolt - Pure's weapon, and the one projectile on either
//! disc that hurts nobody.
//!
//! A floor follower like the Rocket, read off its own functions rather than
//! borrowed from the Rocket's: `Disruptor_Init` (`0x08859010`),
//! `Disruptor_Update` (`0x0885930c`), `Disruptor_TestHit` (`0x08850274`) and
//! `DisruptorPool_Update` (`0x088506c4`), all on `psp-pure-usa` and all on
//! `docs/ghidra/functions/psp-pure-usa/weapons.md`, "Fire: a floor-following
//! bolt that homes if the craft had a lock". Every constant below is that
//! page's, at that page's confidence, and none of them is authored: the
//! table gives the Disruptor two numbers, `absorb` and `speed`, and this
//! module spends exactly one of them.
//!
//! # How it differs from the Rocket, and why it is its own arm
//!
//! [`super::flight`] carries the generic flight - probe, ride, fall, sweep,
//! bounce, guide - and the Disruptor takes none of it, the way a Mine takes
//! none of it. Four things are different enough that folding them in would
//! have meant four more `if kind ==` branches in a function that already has
//! six:
//!
//! 1. **The sweep comes before the probe**, the opposite order to the
//!    Rocket's, and a floor across the sweep is pushed off by `4.0` without
//!    turning the velocity.
//! 2. **The probe direction is never re-read.** `Disruptor_Update` writes
//!    `bolt+0x100` nowhere; the up the bolt was fired with is the up it
//!    probes along for its whole life. A bolt fired on a banked corner keeps
//!    probing along that bank.
//! 3. **A floor hit aims the velocity at the ride point** - `normalize(ride -
//!    position) * speed` - rather than turning it parallel and keeping the
//!    speed. The speed is [`oag_tables::weapons::DisruptorStats::
//!    speed_for_class`], and it is re-pinned on every floor hit and on every
//!    guided tick, so the authored `speed` is what the bolt flies at once it
//!    has found the track, and the 500 km/h launch is what it flies at until
//!    then.
//! 4. **The hit test is a swept cylinder of its own radius**, not the hull
//!    sphere the other weapons share - see [`HIT_RADIUS`].
//!
//! # What a hit does
//!
//! Nothing to the pool, nothing to the body. `Disruptor_TestHit` calls
//! `Disruptor_ApplyEffect` and that function writes a kind and a timer onto
//! the victim; [`crate::disruption`] is that state and its law, and
//! `blast::apply_impacts` routes a Disruptor's [`Impact`] there instead of
//! to a blast the table does not author.

use oag_core::math::Vec3;
use oag_physics::params::Dimensions;
use oag_physics::{Ray, Raycaster, ShipState};
use oag_tables::weapons::Weapon;

use super::{Impact, Projectile};
use crate::world::Ship;

/// The speed a bolt leaves the craft at, in km/h.
///
/// **Recovered, confidence 84, and a literal.** `Disruptor_Init` seeds
/// `bolt+0xe0` as `forward * 500.0 / 3.6` - the `0x43fa0000` is `500.0` and
/// the reciprocal it multiplies by is `1 / 3.6`. The authored `speed` is
/// **not** read here; it takes over on the first floor hit. So a Disruptor
/// fired off a jump flies at 500 km/h until it lands, whatever the class.
pub const LAUNCH_KMH: f32 = 500.0;

/// How far the bolt looks along its up for a floor to ride, in units.
///
/// **Recovered, confidence 84.** `Disruptor_Update` scales `bolt+0x100` by
/// `0x41400000` = `12.0` and probes from the swept position back along it -
/// the Missile's, Plasma's and Shuriken's length rather than the Rocket's
/// `6.0`.
pub const SURFACE_PROBE_LENGTH: f32 = 12.0;

/// How far above a floor the bolt aims to ride, in units.
///
/// **Recovered, confidence 84.** The same function's floor arm computes `hit
/// + normal * 6.0` (`0x40c00000`) and aims the velocity at it - twice the
/// Rocket's `3.0`, and the Disruptor's own.
pub const RIDE_HEIGHT: f32 = 6.0;

/// How far off a floor the bolt is pushed when the sweep, not the probe, meets
/// one, in units.
///
/// **Recovered, confidence 82.** The sweep's non-wall arm writes
/// `hit + normal * 4.0` (`0x40800000`) as the tick's destination and leaves
/// the velocity alone; the probe from that point then does the aiming.
pub const FLOOR_PUSH_OFF: f32 = 4.0;

/// How fast a bolt with nothing under it falls, in units per second squared.
///
/// **Recovered, confidence 84**: `velocity.y -= dt * 50.0` on the probe's
/// nothing branch, the same figure the Rocket's own read gives.
pub const FALL_ACCELERATION: f32 = 50.0;

/// How fast a locked bolt turns toward its target, as a chord per second.
///
/// **Recovered, confidence 82.** The guidance arm normalises `target -
/// position` and the heading, normalises their difference, scales it by
/// `1.0 * dt` (`0x3f800000`) and adds it to the unit heading before
/// re-scaling to the class speed - [`super::missile::steer`]'s shape at a
/// quarter of the Missile's `4.0`, and without its clamp: the original takes
/// the whole chord even when the error is shorter, which can overshoot by a
/// hair on the tick it lines up. Kept, because that is the reading.
pub const TURN_CHORD_PER_SECOND: f32 = 1.0;

/// How far from the bolt's path a craft's centre may be and still be hit, in
/// units.
///
/// **Recovered, confidence 84.** `Disruptor_TestHit` projects each craft's
/// centre onto the tick's segment, accepts a parameter within `[-1.0, len +
/// 1.0]` of the ends and a perpendicular distance within `6.0`. A swept
/// cylinder rather than [`super::hull_radius`]'s sphere, and a generous one:
/// six units is a hull and a half either side.
pub const HIT_RADIUS: f32 = 6.0;

/// How far past either end of a tick's segment the cylinder still counts.
///
/// The `1.0` in `[-1.0, len + 1.0]` above, confidence 84.
pub const HIT_END_SLACK: f32 = 1.0;

/// How long a bolt flies before the pool reaps it, in seconds.
///
/// **Recovered, confidence 86.** `DisruptorPool_Update` reaps any bolt whose
/// `+0x4c` exceeds `10.0`, spawning the wall explosion where it was. That it
/// equals [`super::MAX_FLIGHT_SECONDS`] is a coincidence worth stating: the
/// shared constant is *ours*, this one is the disc's, and a change to the
/// shared one must not drag this along. [`super::Projectiles::fire_disruptor`]
/// asserts the two agree so that a drift is a compile-time question.
pub const MAX_FLIGHT_SECONDS: f32 = 10.0;

const _: () = assert!(MAX_FLIGHT_SECONDS == super::MAX_FLIGHT_SECONDS);

/// Where a bolt starts, which way it flies and which way it probes.
///
/// `Disruptor_Init` copies the firing craft's matrix whole: the translation
/// is the bolt's position, the forward row scaled by [`LAUNCH_KMH`] is its
/// velocity, and the up row is the probe direction it keeps for life. **No
/// muzzle offset**, unlike the Rocket and the Missile - the bolt starts at the
/// craft's centre, and the sweep excludes its owner so that costs nothing.
///
/// `dimensions` is taken and not read, so the signature matches the other
/// weapons' `launch` and a caller that does grow a nose offset changes one
/// line rather than a call site.
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
/// The guidance arm of `Disruptor_Update`, in its order: unit vector to the
/// target, unit heading, unit error, `heading + error * TURN_CHORD * dt`, then
/// scaled to `speed`. Not renormalised between the add and the scale, so the
/// result is very slightly faster than `speed` while turning - the original's
/// own rounding, as [`super::missile::steer`] documents for the Missile.
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

/// Whether the segment `from -> to` passes within [`HIT_RADIUS`] of `centre`.
///
/// `Disruptor_TestHit`'s cylinder: the centre's parameter along the segment
/// must lie within [`HIT_END_SLACK`] of either end, and its distance from the
/// line within the radius. A zero-length segment tests the point.
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

/// Flies one bolt one tick. `Some` where it stopped - on a wall or through a
/// craft - and `None` where it is still going or has just aged out, which the
/// caller tells apart by [`Projectile::lifetime`].
///
/// `speed` is the class's `speed_for_class`, or `None` where the table
/// authors no Disruptor or the class is on no ladder; the bolt then keeps
/// whatever speed it has rather than borrowing a number. The order is
/// `Disruptor_Update`'s: age, sweep, probe, guide, move.
pub(super) fn advance<R: Raycaster + ?Sized>(
    projectile: &mut Projectile,
    dt: f32,
    raycaster: &R,
    ships: &[Ship],
    speed: Option<f32>,
) -> Option<Impact> {
    let from = projectile.position;
    let mut to = from + projectile.velocity * dt;
    let stop = |point: Vec3, struck: Option<u8>, projectile: &Projectile| Impact {
        point,
        kind: Weapon::Disruptor,
        owner: projectile.owner,
        struck,
        // A craft takes the effect; a wall takes nothing. `blast::apply_impacts`
        // reads `effect` and not this for a Disruptor, but the flag still
        // says which of the two this was for anyone drawing it.
        blast: struck.is_some(),
        effect: struck.and(projectile.effect),
    };

    // 1. The travel sweep, first. A wall ends the flight; a floor across the
    //    step pushes the destination off it and leaves the velocity to the
    //    probe below.
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

    // 2. The probe, along the up the bolt was fired with. Nothing: fall. A
    //    floor: aim at the ride point and re-pin the speed. A wall: dead.
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

    // 3. Guidance, off the tick's *start* position, then the speed re-pinned
    //    - both the original's, and the second is what makes a locked bolt
    //    hold class speed in the air where an unlocked one only falls.
    if let Some(target) = projectile.target
        && let Some(ship) = ships.get(target as usize).filter(|s| s.active)
        && let Some(kmh) = speed
    {
        projectile.velocity = steer(
            projectile.velocity,
            from,
            ship.physics.body.position,
            dt,
            kmh / super::KMH_PER_UNIT_PER_SECOND,
        );
    }

    // 4. The hit test, over the segment actually flown this tick, every craft
    //    but the owner. `DisruptorPool_Update` runs `Disruptor_TestHit` after
    //    `Disruptor_Update`, so it sees the moved bolt; the segment is the
    //    tick's, from where it started to where it ended.
    for (slot, ship) in ships.iter().enumerate() {
        if !ship.active || slot as u8 == projectile.owner {
            continue;
        }
        if cylinder_hit(from, to, ship.physics.body.position) {
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
