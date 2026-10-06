//! The Missile: what it locks onto, how it steers, and how fast it goes.
//!
//! Its flight is [`super::Projectiles::advance`]'s, shared with the Rocket. Here:
//! the lock, the guidance term and the speed ramp.
//!
//! # What is recovered and what is ours
//!
//! Almost entirely recovered, against `/pulse/BOOT-psp-pulse-usa.BIN` (image base
//! `0x08804000`); evidence and per-claim confidences on
//! `docs/ghidra/functions/psp-pulse-usa/missile.md`:
//!
//! - The `<Stats>` block and lock distances (`WeaponStats_ParseMissile`,
//!   `0x0880c31c`, confidence 90).
//! - A press puts exactly one in the air: `Weapon_FireMissile` (`0x088685cc`, bit
//!   `0x40`), no fan (confidence 90).
//! - [`lock`], from `Ship_AcquireLock` (`0x08844784`, confidence 88).
//! - [`steer`], from `Missile_Update` (`0x0885a918`, confidence 95).
//! - [`speed_kmh`], from `Missile_SpeedNow` (`0x0885a038`, confidence 90).
//! - The launch speed law in [`launch`], from `Missile_Init` (`0x0885a160`).
//! - A press with no lock still fires, from `Ship_FireHeldWeapon` (`0x08844ae8`,
//!   confidence 90); see [`lock`].
//! - [`SELF_DETONATE_SECONDS`], from `Projectiles_Update_q` (`0x08869588`,
//!   confidence 90).
//!
//! **Ours:** the launch offset to the nose by the hull's extent (the original
//! spawns at the craft's pose, see [`super::launch`]); dropping one lock
//! condition (see [`lock`]); a slot index where the original has a pointer.

use crate::Craft;
use oag_core::math::Vec3;
use oag_physics::ShipState;
use oag_physics::params::Dimensions;
use oag_tables::weapons::MissileStats;

/// How far the unit heading may swing in one second, as a chord length.
///
/// **Recovered, confidence 95.** `Missile_Update` clamps the error between the
/// unit direction to the target and the unit velocity to `dt * 4.0`, the only
/// steering constant in the function.
///
/// A chord on the unit sphere, not an angle: the original's guidance calls no
/// `vsin_s`/`vcos_s`/`vcst_s(5)`, so this needs no platform transcendental
/// (`docs/architecture/determinism.md`). The largest error chord is `2.0`, a
/// reversal, covered in half a second.
pub const TURN_CHORD_PER_SECOND: f32 = 4.0;

/// How far a missile looks toward the surface for something to ride.
///
/// **Recovered, confidence 92**, double the Rocket's
/// [`super::SURFACE_PROBE_LENGTH`]: `Missile_Update` scales the normal by
/// `0x41400000` = `12.0`, `Rocket_Update` by `6.0`.
pub const SURFACE_PROBE_LENGTH: f32 = 12.0;

/// How many walls a missile may glance off before it detonates.
///
/// **Recovered, confidence 92.** `Missile_Update` counts wall hits at `self+0x6c`
/// and tests `< 5`. The Rocket detonates on its first wall.
pub const MAX_BOUNCES: u8 = 5;

/// How far off a wall a bouncing missile is pushed, so it does not re-hit it.
///
/// **Recovered, confidence 92**: `0x3dcccccd` = `0.1` along the hit normal.
pub const BOUNCE_PUSH_OFF: f32 = 0.1;

/// How long a missile flies before it detonates where it is, in seconds.
///
/// **Recovered, confidence 90**; it is what makes a targetless missile a weapon
/// rather than litter. `Projectiles_Update_q` (`0x08869588`) takes the destroy
/// branch on `3.0 < self->age`:
///
/// ```text
/// if (3.0 < missile->age && (missile->flags & 1)) {
///     missile->flags |= 4;              // the same bit a wall or a craft sets
///     ...                               // trail released, cue played
/// }
/// ```
///
/// `self+0x50` is the same age [`speed_kmh`] ramps on. Bit `4` is "destroy me",
/// shared with the wall and craft paths, so this is a detonation rather than a
/// reap; see [`super::Impact::blast`] for what it does not do. The `flags & 1`
/// half means locally-simulated (the network spawn `FUN_088687c0` sets `2`); with
/// no networking here it is not ported.
pub const SELF_DETONATE_SECONDS: f32 = 3.0;

/// How long the missile takes to reach its class speed, in seconds.
///
/// **Recovered, confidence 95**, a code literal and not the authored
/// `slowdown_time` (`Missile_SpeedNow` tests `age < 1.0`). The shipped
/// `slowdown_time="1.0"` is a coincidence; do not wire the two together.
///
/// **Shared with the Plasma, recovered twice.** `Plasma_SpeedForClass`
/// (`0x0885c5a4`) tests `age < 1.0` with the same operand order
/// (`docs/ghidra/functions/psp-pulse-usa/plasma.md`, "the launch ramp"), so
/// [`speed_kmh`] serves both (see `oag_weapons::projectile::flight`'s
/// `pinned_kmh`). The Plasma does **not** share [`LAUNCH_SPEED_FLOOR_KMH`]:
/// `Plasma_Launch` has no `vmax_s`, so a standing-start bolt leaves at
/// `launchspeed` alone.
pub const SPEED_RAMP_SECONDS: f32 = 1.0;

/// The slowest a missile may leave the rail, in km/h.
///
/// **Recovered, confidence 88**: `Missile_Init` takes `vmax_s(speed, 0x41666666)`
/// = `14.4`. It matters on a standing grid.
pub const LAUNCH_SPEED_FLOOR_KMH: f32 = 14.4;

/// How square-on a target must be to be lockable, as a cosine.
///
/// **Recovered, confidence 88**: `Ship_AcquireLock` tests
/// `0.9 < dot(normalize(to target), forward)`, about 26 degrees. A cosine, as in
/// `oag_ai`'s `WEAPON_CONE`, to avoid `acos`.
pub const LOCK_CONE_COS: f32 = 0.9;

/// How far round the houses a target may be, as along-track gap over range.
///
/// **Recovered, confidence 80**: `Ship_AcquireLock`'s last test is
/// `fabs(gap) / range < 1.4`, `gap` from `FUN_0883dbf4`, which subtracts the two
/// craft's along-track distances (`entity+0x91c`) and wraps around the lap. The
/// purpose (rejecting a craft near in space but far along the road, as on a
/// hairpin) is the reading, confidence 80; the arithmetic is plain.
pub const LOCK_GAP_RATIO: f32 = 1.4;

/// km/h to world units per second, as the original's own bit pattern.
///
/// `Missile_Update` scales by `0x3e8e38e4`, which is the correctly rounded `f32`
/// for `1.0 / 3.6` (checked; a first disassembly reading claimed it was not).
/// The operation differs, not the value: this path multiplies, the
/// surface-contact path divides by `3.6` ([`speed_units_on_surface`]), and
/// `x * (1/3.6)` and `x / 3.6` differ in `f32`.
pub const KMH_TO_UNITS_PER_SECOND: f32 = f32::from_bits(0x3e8e_38e4);

/// The divisor the original's surface-contact path uses, as a divide. See
/// [`KMH_TO_UNITS_PER_SECOND`].
pub const KMH_PER_UNIT_PER_SECOND: f32 = 3.6;

/// How fast a missile is travelling right now, in km/h.
///
/// **Recovered whole** from `Missile_SpeedNow` (`0x0885a038`), confidence 90: a
/// linear blend from the launch speed to the class speed over
/// [`SPEED_RAMP_SECONDS`], flat thereafter.
///
/// ```text
/// age < 1.0  ->  launch * (1 - age) + class * age
/// age >= 1.0 ->  class
/// ```
///
/// The speed is pinned, never integrated: both velocity writes normalise and
/// rescale to this, so the `dt * 50.0` fall term changes direction only.
#[must_use]
pub fn speed_kmh(launch_kmh: f32, class_kmh: f32, age: f32) -> f32 {
    if age < SPEED_RAMP_SECONDS {
        launch_kmh * (1.0 - age) + class_kmh * age
    } else {
        class_kmh
    }
}

/// The same speed in world units per second, as the guidance path spends it.
#[must_use]
pub fn speed_units_guided(speed_kmh: f32) -> f32 {
    speed_kmh * KMH_TO_UNITS_PER_SECOND
}

/// The same, as the surface-contact path spends it: a divide, not a multiply.
#[must_use]
pub fn speed_units_on_surface(speed_kmh: f32) -> f32 {
    speed_kmh / KMH_PER_UNIT_PER_SECOND
}

/// One tick of guidance: the new velocity for a missile chasing `target`.
///
/// **Recovered whole** from `Missile_Update` (`0x0885a918`), confidence 95.
///
/// ```text
/// d    = normalize(target - position)
/// v    = normalize(velocity)
/// err  = d - v
/// step = normalize(err) * (dt * TURN_CHORD_PER_SECOND)
/// move = if |step|^2 <= |err|^2 { step } else { err }
/// dir  = v + move
/// out  = dir * speed
/// ```
///
/// Three things that look like bugs and are not:
///
/// 1. `dir` is not renormalised, so a turning missile flies slower than
///    [`speed_kmh`] says. `normalize(d - v)` has dot `-sin(θ/2)` with `v`, so
///    `|dir|` reaches about `0.954` for a right angle and `0.933` for a reversal,
///    returning to one as it lines up. Renormalising would speed turns up to 7 %;
///    `a_turning_missile_flies_slower_than_its_pinned_speed` is the arithmetic.
/// 2. The clamp is on a chord: a missile nearly on target takes the whole error
///    in one tick (the `else` arm), one pointing away turns at a constant rate.
/// 3. `position` must be the position at the **start** of the tick: the original
///    runs this after the move is committed and writes only velocity, so the
///    correction lands a tick later. See [`super::Projectiles::advance`], which
///    passes `from`.
///
/// Returns `velocity` unchanged when either vector is degenerate, as the original
/// guards.
#[must_use]
pub fn steer(velocity: Vec3, position: Vec3, target: Vec3, dt: f32, speed: f32) -> Vec3 {
    let to_target = target - position;
    if to_target.length_squared() <= 0.0 || velocity.length_squared() <= 0.0 {
        return velocity;
    }
    let desired = to_target.normalize();
    let heading = velocity.normalize();

    let error = desired - heading;
    let error_squared = error.length_squared();
    if error_squared <= 0.0 {
        // Already on target; the original's normalise guard does the same.
        return heading * speed;
    }

    let step = error.normalize() * (dt * TURN_CHORD_PER_SECOND);
    let movement = if step.length_squared() <= error_squared {
        step
    } else {
        error
    };

    // Not normalised, see the doc comment.
    (heading + movement) * speed
}

/// Which craft a missile fired from `origin` along `forward` locks onto.
///
/// **Recovered** from `Ship_AcquireLock` (`0x08844784`), confidence 88. The
/// original stores the result on the entity; running it at the moment of firing
/// is equivalent for a weapon that acquires and launches in one press.
///
/// # The four conditions, in the original's order
///
/// 1. **Not the firer, and racing.** `other == self` is skipped outright. The
///    blast does not exclude its owner, so a self-lock would hit the firer's back.
/// 2. **Longitudinal distance inside the authored window**:
///    `along = dot(target - origin, forward)` against
///    [`MissileStats::lock_min_dist`] and [`MissileStats::lock_max_dist`]. Not
///    straight-line range: a craft alongside sits near zero on this axis.
/// 3. **Inside the cone**, [`LOCK_CONE_COS`].
/// 4. **Not round the houses**, [`LOCK_GAP_RATIO`]. Skipped when `circuit_length`
///    is `None` (a synthetic straight has no lap).
///
/// The winner is the nearest by longitudinal distance.
///
/// # `None` is not a refusal to fire
///
/// **Recovered, confidence 90.** `Ship_FireHeldWeapon` (`0x08844ae8`) fires either
/// way, passing a null and target index `-1` when unlocked:
///
/// ```c
/// if ((self->target == -1) || ((self->lock_flags & 1) == 0)) {
///     Weapon_RequestFire(craft, pose, 0, 0xffffffff);
/// } else {
///     Weapon_RequestFire(craft, pose, entity_table[self->target] + 0x794);
/// }
/// ```
///
/// `Weapon_FireMissile` (`0x088685cc`) tests the target not at all, and
/// `Missile_Update` skips guidance on `self+0xe0 == 0`, so the missile flies
/// ballistically until [`SELF_DETONATE_SECONDS`]. A caller launches on `None`, as
/// `oag_game`'s `Race::fire_missile` does.
///
/// # The condition not ported
///
/// The original also skips any craft whose `weapon_record+0x12c` is above zero, a
/// count of hits landed on it but not yet resolved (the same function adds
/// `damage` and `slowdown_time` into two accumulators beside it): "do not lock
/// somebody already taking a hit this frame". [`super::step`] applies a blast the
/// moment it lands, so the count would always be zero; porting it would mean
/// inventing a deferred-damage queue. The one departure from the original here.
#[must_use]
pub fn lock<S: Craft>(
    ships: &[S],
    firer: u8,
    origin: Vec3,
    forward: Vec3,
    stats: &MissileStats,
    circuit_length: Option<f32>,
) -> Option<u8> {
    lock_window(
        ships,
        firer,
        origin,
        forward,
        stats.lock_min_dist,
        stats.lock_max_dist,
        circuit_length,
    )
}

/// The same acquisition, against a window given directly.
///
/// `Ship_AcquireLock` (`0x08844784`) serves two weapons: it switches on the held
/// weapon id to pick the window (`stats+0x50`/`+0x54` for the Missile,
/// `+0x114`/`+0x118` for the LeachBeam), and everything after is shared (cone,
/// along-track screen, nearest tie-break, firer skip). So this takes the two
/// numbers rather than a second stats type; [`lock`] is the Missile's spelling.
/// The LeachBeam's window is shorter at the far end on both shipped tables, see
/// `oag_tables::weapons::LeachBeamStats`.
#[must_use]
pub fn lock_window<S: Craft>(
    ships: &[S],
    firer: u8,
    origin: Vec3,
    forward: Vec3,
    lock_min_dist: f32,
    lock_max_dist: f32,
    circuit_length: Option<f32>,
) -> Option<u8> {
    let mut best: Option<(f32, u8)> = None;

    for (slot, ship) in ships.iter().enumerate() {
        if slot as u8 == firer || !ship.active() {
            continue;
        }
        let to_them = ship.physics().body.position - origin;
        let along = to_them.dot(forward);
        if along <= lock_min_dist || along >= lock_max_dist {
            continue;
        }
        // Nearest first, as the original tests it; a loser skips the normalise.
        if best.is_some_and(|(nearest, _)| along >= nearest) {
            continue;
        }
        let range = to_them.length();
        if range <= 0.0 || to_them.normalize().dot(forward) <= LOCK_CONE_COS {
            continue;
        }
        if let (Some(length), Some(here), Some(there)) = (
            circuit_length,
            ships[firer as usize].progress(),
            ship.progress(),
        ) {
            let gap = wrapped_gap(there - here, length);
            if gap.abs() / range >= LOCK_GAP_RATIO {
                continue;
            }
        }
        best = Some((along, slot as u8));
    }

    best.map(|(_, slot)| slot)
}

/// A signed along-track difference folded into `-length/2 ..= length/2`, as the
/// original's helper does.
fn wrapped_gap(difference: f32, length: f32) -> f32 {
    if length <= 0.0 || !length.is_finite() {
        return difference;
    }
    let half = length * 0.5;
    let mut gap = difference;
    while gap > half {
        gap -= length;
    }
    while gap < -half {
        gap += length;
    }
    gap
}

/// Where a craft launches a missile from, how fast, and the speed to ramp from.
///
/// Returns the spawn point, the launch velocity, and the launch speed in km/h
/// (per-shot, [`speed_kmh`] ramps from it).
///
/// **Recovered, confidence 88.** `Missile_Init` (`0x0885a160`) computes
///
/// ```text
/// speed_kmh = |craft velocity| * 3.6 + launchSpeed
/// ```
///
/// then `direction * max(speed_kmh, 14.4) / 3.6`. This answers what
/// [`super::launch`] records as open for the Rocket: for the Missile
/// `launchSpeed` is an additive muzzle velocity over the launcher's own speed,
/// and the class speed is where the ramp ends. **Not propagated to the Rocket**:
/// `Rocket_Init` is a different function nobody re-read.
///
/// The direction is the craft's forward; the offset to the nose is **ours**, as
/// in [`super::launch`], so a projectile does not start inside its firer's hull.
#[must_use]
pub fn launch(
    state: &ShipState,
    dimensions: &Dimensions,
    stats: &MissileStats,
    // Deliberately unused, kept in the signature: a rocket's launch speed is the
    // class speed plus `launchSpeed`, a missile's the launcher's speed plus it.
    // Dropping the parameter would invite "fixing" this to the rocket's rule.
    _class: &str,
) -> (Vec3, Vec3, f32) {
    let forward = state.body.forward();
    let nose = state.body.position
        + forward * oag_physics::wall::hull_extent(&state.body, dimensions, forward);

    let launch_kmh =
        state.body.linear_velocity.length() * KMH_PER_UNIT_PER_SECOND + stats.launch_speed;

    // The floor applies to the launch velocity, not the ramp's base:
    // `Missile_Init` stores the unfloored sum at `self+0x48` and applies
    // `vmax_s(_, 14.4)` only to the initial velocity. They differ only under
    // 14.4 km/h, unreachable with the shipped `launchSpeed="200"`.
    let speed = speed_units_guided(launch_kmh.max(LAUNCH_SPEED_FLOOR_KMH));
    (nose, forward * speed, launch_kmh)
}

#[cfg(test)]
mod tests;
