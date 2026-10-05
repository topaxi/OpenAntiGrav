//! The Missile: what it locks onto, how it steers, and how fast it goes.
//!
//! Its *flight* is [`super::Projectiles::advance`]'s, shared with the Rocket -
//! that model was written weapon-agnostic on purpose and the Missile is the first
//! thing to inherit it. What is here is the three parts that are the Missile's
//! own: the lock, the guidance term, and the speed ramp.
//!
//! # What is recovered and what is ours
//!
//! Stated at the top, the way [`crate::pickup`] and [`super`] state it. **This
//! module is unusual for this crate in being almost entirely recovered**, so the
//! short list is the other way round from normal.
//!
//! **Recovered**, all against `/pulse/BOOT-psp-pulse-usa.BIN`, image base `0x08804000`,
//! evidence and per-claim confidences on
//! `docs/ghidra/functions/psp-pulse-usa/missile.md`:
//!
//! - The `<Stats>` block and its two lock distances, at instruction level
//!   (`WeaponStats_ParseMissile`, `0x0880c31c`, confidence 90).
//! - **That a press puts exactly one in the air.** `Weapon_FireMissile`
//!   (`0x088685cc`, fire-request bit `0x40`) makes a single spawn call - no fan,
//!   no burst - unlike the Rocket's three (confidence 90).
//! - The whole of [`lock`], from `Ship_AcquireLock` (`0x08844784`, confidence 88).
//! - The whole of [`steer`], from `Missile_Update` (`0x0885a918`, confidence 95).
//! - The whole of [`speed_kmh`], from `Missile_SpeedNow` (`0x0885a038`,
//!   confidence 90).
//! - The launch speed law in [`launch`], from `Missile_Init` (`0x0885a160`).
//! - **That a press with no lock still fires**, from `Ship_FireHeldWeapon`
//!   (`0x08844ae8`, confidence 90) - see [`lock`].
//! - [`SELF_DETONATE_SECONDS`], from `Projectiles_Update_q` (`0x08869588`,
//!   confidence 90) - which is what makes an unlocked missile end.
//!
//! **Ours**, and it is a short list:
//!
//! - **The launch offset.** [`launch`] pushes the spawn point out to the nose by
//!   the hull's own extent, exactly as [`super::launch`] does and for the same
//!   reason; the original spawns at the craft's pose.
//! - **Dropping one condition of the lock**, deliberately and with the reason
//!   under [`lock`].
//! - **A slot index where the original has a pointer.** The original hands the
//!   missile a raw pointer to the target object; a slot index is what survives
//!   being in a `Copy` world snapshot.

use crate::Craft;
use oag_core::math::Vec3;
use oag_physics::ShipState;
use oag_physics::params::Dimensions;
use oag_tables::weapons::MissileStats;

/// How far the unit heading may swing in one second, as a chord length.
///
/// **Recovered, confidence 95.** `Missile_Update` computes the error between the
/// unit direction to the target and the unit velocity, then clamps the correction
/// to `dt * 4.0` - `local_6b0 = param_1 * 4.0`, the only steering constant in the
/// function.
///
/// **It is a chord on the unit sphere, not an angle.** Nothing in the original's
/// guidance path calls `vsin_s`, `vcos_s` or `vcst_s(5)`; the whole law is vector
/// arithmetic. That is convenient here for a reason the original never cared
/// about: `docs/architecture/determinism.md` forbids platform transcendentals in
/// simulation code, and this needs none.
///
/// A chord of `4.0` per second is a little over a right angle per second at
/// small angles, and it saturates rather than growing without bound - the largest
/// possible error chord is `2.0`, a heading reversal, which this covers in half a
/// second.
pub const TURN_CHORD_PER_SECOND: f32 = 4.0;

/// How far a missile looks toward the surface for something to ride.
///
/// **Recovered, confidence 92**, and it is **double the Rocket's**
/// [`super::SURFACE_PROBE_LENGTH`]: `Missile_Update` scales the stored normal by
/// `0x41400000` = `12.0` where `Rocket_Update` uses `6.0`. Kept as its own
/// constant rather than shared, because the two really are different numbers and
/// a shared one would have to be wrong for one of them.
pub const SURFACE_PROBE_LENGTH: f32 = 12.0;

/// How many walls a missile may glance off before it gives up and detonates.
///
/// **Recovered, confidence 92.** `Missile_Update` keeps a counter at `self+0x6c`,
/// increments it on every wall hit and tests `< 5`; past that it takes the
/// detonating branch instead.
///
/// **This is the sharpest behavioural difference from the Rocket**, which
/// detonates on its first wall. A missile skitters down a corridor.
pub const MAX_BOUNCES: u8 = 5;

/// How far off a wall a bouncing missile is pushed, so it does not re-hit it.
///
/// **Recovered, confidence 92** - `0x3dcccccd` = `0.1`, applied along the hit
/// normal from the hit point.
pub const BOUNCE_PUSH_OFF: f32 = 0.1;

/// How long a missile flies before it detonates where it is, in seconds.
///
/// **Recovered, confidence 90**, and it is the rule that makes a missile with no
/// target a weapon rather than litter. The missile pool's own per-frame update,
/// `Projectiles_Update_q` (`0x08869588`), runs a second pass over every live
/// slot and takes the destroy branch on `3.0 < self->age`:
///
/// ```text
/// if (3.0 < missile->age && (missile->flags & 1)) {
///     missile->flags |= 4;              // the same bit a wall or a craft sets
///     ...                               // trail released, cue played
/// }
/// ```
///
/// **`self+0x50` is the same age [`speed_kmh`] ramps on** - `Missile_SpeedNow`
/// (`0x0885a038`) reads that field and nothing else for its `age < 1.0` blend,
/// and `Missile_Update` is what accumulates it by `dt`. One field, two consumers,
/// so the ramp's age and the timeout's age cannot drift apart.
///
/// **Bit `4` is "destroy me", and the teardown that acts on it is shared** with
/// the wall and craft paths, which reach it through `flags |= 0x14` and
/// `|= 0x24`. So this is a detonation rather than a reap - see
/// [`super::Impact::blast`] for the one thing it does *not* do.
///
/// The `flags & 1` half of the test is **locally-simulated**, not alive: the
/// network spawn path (`FUN_088687c0`) initialises the same word to `2` instead,
/// so a remote craft's missile is destroyed by its owner's message rather than by
/// this timer. With no networking here the condition is trivially satisfied and
/// is not ported.
pub const SELF_DETONATE_SECONDS: f32 = 3.0;

/// How long the missile takes to reach its class speed, in seconds.
///
/// **Recovered, confidence 95**, and it is a **code literal** rather than the
/// authored `slowdown_time`: `Missile_SpeedNow` tests `age < 1.0` and blends.
/// The Missile's `<Stats>` happens to author `slowdown_time="1.0"` as well, which
/// is a coincidence worth naming so nobody wires the two together - they are
/// different quantities that both read 1.0 on the shipped disc.
///
/// **Shared with the Plasma, and recovered twice over.** `Plasma_SpeedForClass`
/// (`0x0885c5a4`) independently tests `age < 1.0` and blends with the identical
/// operand order - `launch * (1 - age) + class * age` - read weeks after this
/// function and from a different part of the executable
/// (`docs/ghidra/functions/psp-pulse-usa/plasma.md`'s "the launch ramp"
/// section). Two functions agreeing on both the second and the shape is the
/// same bar this project used to fold the Rocket's, the Missile's and the
/// Shuriken's `12.0` surface probe into one constant, so [`speed_kmh`] is
/// spent for the Plasma too rather than the Plasma growing its own copy - see
/// `oag_weapons::projectile::flight`'s `pinned_kmh`. The Plasma does **not**
/// share [`LAUNCH_SPEED_FLOOR_KMH`]: `Plasma_Launch`'s listing has no `vmax_s`
/// clamping a minimum, so a standing-start bolt leaves at `launchspeed` alone.
pub const SPEED_RAMP_SECONDS: f32 = 1.0;

/// The slowest a missile may leave the rail, in km/h.
///
/// **Recovered, confidence 88** - `Missile_Init` takes `vmax_s(speed,
/// 0x41666666)` = `14.4` before building the launch velocity. It matters only for
/// a craft that is barely moving, which on a standing grid is every craft.
pub const LAUNCH_SPEED_FLOOR_KMH: f32 = 14.4;

/// How square-on a target must be to be lockable, as a cosine.
///
/// **Recovered, confidence 88** - `Ship_AcquireLock` tests `0.9 < dot(normalize(to
/// target), forward)`, a code literal. About 26 degrees off the nose.
///
/// A cosine and never an angle, for the reason `oag_ai`'s `WEAPON_CONE` gives:
/// comparing cosines needs no `acos`, and `acos` is a transcendental the
/// determinism rules keep out of simulation code.
pub const LOCK_CONE_COS: f32 = 0.9;

/// How far round the houses a target may be, as along-track gap over straight-line
/// range.
///
/// **Recovered, confidence 80** - `Ship_AcquireLock`'s last test is
/// `fabs(gap) / range < 1.4`, where `gap` comes from a helper
/// (`FUN_0883dbf4`) that subtracts the two craft's along-track distances
/// (`entity+0x91c` each) and wraps the difference around the lap.
///
/// **What it is for**, and this is the reading rather than the reading of the
/// arithmetic: it rejects a craft that is close in space but far away along the
/// road. On a hairpin the car coming the other way is thirty units from the nose
/// and three hundred units of tarmac away, and locking it would send a missile
/// into the barrier. Confidence 80 on the *purpose*; the arithmetic itself is
/// plain.
pub const LOCK_GAP_RATIO: f32 = 1.4;

/// km/h to world units per second, as the original's own bit pattern.
///
/// `Missile_Update` scales by the literal `0x3e8e38e4`, and that **is** the
/// correctly-rounded `f32` for `1.0 / 3.6` - checked, because a first reading of
/// the disassembly claimed it was not and that claim is the sort that survives
/// review by sounding precise. Written as the bit pattern anyway, so the constant
/// in this file is the constant in the executable rather than something a
/// compiler happened to agree on.
///
/// **What is genuinely two things is the operation, not the value.** This path
/// multiplies by the reciprocal; the surface-contact path *divides* by `3.6`
/// ([`speed_units_on_surface`]). `x * (1/3.6)` and `x / 3.6` are not the same
/// function in `f32`, so both are reproduced where they occur.
pub const KMH_TO_UNITS_PER_SECOND: f32 = f32::from_bits(0x3e8e_38e4);

/// The divisor the original's surface-contact path uses, as a divide.
///
/// See [`KMH_TO_UNITS_PER_SECOND`] for why this is a separate constant spent a
/// separate way.
pub const KMH_PER_UNIT_PER_SECOND: f32 = 3.6;

/// How fast a missile is travelling right now, in km/h.
///
/// **Recovered whole** from `Missile_SpeedNow` (`0x0885a038`), confidence 90: a
/// linear blend from the speed it left the rail at to its class speed, over
/// [`SPEED_RAMP_SECONDS`], and the class speed flat thereafter.
///
/// ```text
/// age < 1.0  ->  launch * (1 - age) + class * age
/// age >= 1.0 ->  class
/// ```
///
/// **The speed is pinned, never integrated.** Both of the original's velocity
/// writes normalise the direction and rescale to this, which has a consequence
/// worth stating because it looks like a bug otherwise: the `dt * 50.0` fall term
/// on the no-surface branch contributes only a *direction* change, never a
/// magnitude one. A missile that flies off the edge of the track curves downward
/// without speeding up.
#[must_use]
pub fn speed_kmh(launch_kmh: f32, class_kmh: f32, age: f32) -> f32 {
    if age < SPEED_RAMP_SECONDS {
        // The original's own order: `launch * (1 - age) + class * age`.
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

/// The same speed in world units per second, as the surface-contact path spends
/// it - **a divide, not the multiply above**. See [`KMH_TO_UNITS_PER_SECOND`].
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
/// # Three things that look like bugs and are not
///
/// 1. **`dir` is not renormalised before the speed scale, so a turning missile
///    flies slower than [`speed_kmh`] says.** The correction always points partly
///    *against* the current heading - `normalize(d - v)` has a dot product of
///    `-sin(θ/2)` with `v` for a turn of `θ` - so `|dir|` is at most one and
///    reaches about `0.954` for a right-angle turn and `0.933` for a reversal. It
///    returns to exactly one as the missile lines up. The original does not
///    renormalise and neither does this; "fixing" it would speed up every turning
///    missile by up to 7 %.
///
///    Worth stating because the intuition runs the other way - a correction
///    added to a unit vector *sounds* like it should overshoot - and a first
///    reading of the disassembly recorded exactly that. It is wrong, and
///    `a_turning_missile_flies_slower_than_its_pinned_speed` is the arithmetic
///    that says so.
/// 2. **The clamp is on a chord, not an angle**, so a missile already pointing
///    nearly at its target takes the whole error in one tick (the `else` arm) and
///    a missile pointing away turns at a constant rate. That is what the
///    `|step|^2 <= |err|^2` test decides.
/// 3. **`position` must be the missile's position at the *start* of the tick.**
///    The original runs this block after the move is already committed and writes
///    only the velocity, so the correction takes effect on the tick *after* the
///    one that measured it. Feeding it the post-move position would tighten the
///    loop by one tick and is not what the original does. See
///    [`super::Projectiles::advance`], which calls this with `from`.
///
/// Returns `velocity` unchanged when either vector is degenerate - the original
/// guards both with explicit component-wise tests against zero before
/// normalising.
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
        // Already pointing exactly at it. The original's normalise guard takes
        // the same branch, leaving the step at the whole (zero) error.
        return heading * speed;
    }

    let step = error.normalize() * (dt * TURN_CHORD_PER_SECOND);
    let movement = if step.length_squared() <= error_squared {
        step
    } else {
        error
    };

    // Not normalised - see the doc comment.
    (heading + movement) * speed
}

/// Which craft a missile fired from `origin` along `forward` locks onto.
///
/// **Recovered** from `Ship_AcquireLock` (`0x08844784`), confidence 88. The
/// original runs this for the firing craft and stores the result on the *entity*,
/// then the fire handler passes the target through; running it at the moment of
/// firing is the same thing for a weapon that acquires and launches in one press.
///
/// # The four conditions, in the original's own order
///
/// 1. **Not the firer, and racing.** The loop skips `other == self` outright, so a
///    missile cannot lock the craft that fired it. That is recovered rather than a
///    guard added here, and it matters: the blast does not exclude its owner, so a
///    self-lock is a craft firing at its own back.
/// 2. **Longitudinal distance inside the authored window.** `along = dot(target -
///    origin, forward)`, tested against [`MissileStats::lock_min_dist`] and
///    [`MissileStats::lock_max_dist`]. **Not straight-line range** - a craft
///    directly alongside sits near zero on this axis however close it is, and is
///    excluded by the near bound rather than by an angle.
/// 3. **Inside the cone**, by [`LOCK_CONE_COS`].
/// 4. **Not round the houses**, by [`LOCK_GAP_RATIO`] - the along-track gap over
///    the straight-line range. Skipped entirely when `circuit_length` is `None`,
///    which is the honest thing for a caller with no closed course to measure a
///    lap against; a synthetic straight has no "round the houses" to speak of.
///
/// The winner is the **nearest by longitudinal distance**, not by range and not by
/// bearing.
///
/// # `None` is not a refusal to fire
///
/// **Recovered, confidence 90**, and it used to be read the other way here.
/// `Ship_FireHeldWeapon` (`0x08844ae8`) branches on the lock and fires either
/// way - with a lock it passes the target's address, and with none it passes a
/// null and a target index of `-1`:
///
/// ```c
/// if ((self->target == -1) || ((self->lock_flags & 1) == 0)) {
///     Weapon_RequestFire(craft, pose, 0, 0xffffffff);
/// } else {
///     Weapon_RequestFire(craft, pose, entity_table[self->target] + 0x794);
/// }
/// ```
///
/// `Weapon_FireMissile` (`0x088685cc`) then tests the target not at all: it
/// clears the held slot, and spawns whatever `craft+0x160` holds. A null reaches
/// `Missile_Init`, and `Missile_Update` skips its whole guidance block on
/// `self+0xe0 == 0`, so the missile flies ballistically - riding the floor,
/// glancing off walls - until [`SELF_DETONATE_SECONDS`] ends it.
///
/// So a caller wanting the original's behaviour launches on `None` rather than
/// declining. `oag_game`'s `Race::fire_missile` does.
///
/// # The condition deliberately not ported
///
/// The original also skips any craft whose `weapon_record+0x12c` is above zero.
/// That field is a **count of hits landed on it but not yet resolved** - the same
/// function that increments it also adds the weapon's `damage` and `slowdown_time`
/// into two accumulators beside it - so the rule is "do not lock somebody who is
/// already taking a hit this frame".
///
/// This engine has no such state: [`super::step`] applies a blast the moment it
/// lands, so the count would be zero on every tick it could be read. Porting it
/// would mean inventing the deferred-damage queue it counts. Recorded here rather
/// than silently dropped, and it is the one place this function departs from the
/// original.
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

/// The same acquisition, against a window given directly rather than off the
/// Missile's block.
///
/// **`Ship_AcquireLock` (`0x08844784`) is one function serving two weapons**,
/// and this is that shape written out. The original switches on the held weapon
/// id to pick which pair of offsets the window comes from -
/// `stats+0x50`/`+0x54` for the Missile, `stats+0x114`/`+0x118` for the
/// LeachBeam - and everything after that switch is shared: the same `0.9` cone,
/// the same along-track screen, the same nearest-by-longitudinal-distance
/// tie-break, the same "not the firer, and racing" skip.
///
/// So the two weapons differ by **two numbers and nothing else**, which is why
/// this takes the numbers instead of a second stats type. [`lock`] is the
/// Missile's spelling of it and is what every existing caller uses.
///
/// The LeachBeam's window is authored shorter at the far end than the Missile's
/// on both shipped tables - see `oag_tables::weapons::LeachBeamStats`, which is
/// where that finding lives.
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
        // Nearest by longitudinal distance. Tested before the two remaining
        // conditions because the original tests it there, and a candidate that
        // loses on distance never pays for the normalise below.
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

/// A signed along-track difference folded into `-length/2 ..= length/2`.
///
/// The original's helper takes the raw subtraction and the circuit length and
/// does the same; a craft one metre ahead of the line and one metre behind it are
/// two metres apart, not a lap.
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
/// Returns the spawn point, the launch velocity, and the launch speed in km/h -
/// the third because [`speed_kmh`] ramps *from* it and it is per-shot rather than
/// per-weapon.
///
/// # The speed law, and what it settles about `launchSpeed`
///
/// **Recovered, confidence 88.** `Missile_Init` (`0x0885a160`) computes
///
/// ```text
/// speed_kmh = |craft velocity| * 3.6 + launchSpeed
/// ```
///
/// then builds the velocity as `direction * max(speed_kmh, 14.4) / 3.6`.
///
/// **This answers a question [`super::launch`] records as open.** That function's
/// doc says of the Rocket that "the original's flight speed is the class's alone
/// ... what `launchSpeed` *is* for has not been found". For the Missile it is
/// found: it is an additive muzzle velocity **over the launcher's own speed**, and
/// the class speed is where the ramp ends rather than where it starts.
///
/// **It is deliberately not propagated to the Rocket.** The Rocket has its own
/// `Rocket_Init` and nothing has re-read it, so the Rocket keeps the sum it has
/// always had and keeps its note saying so. Two weapons parsed by two functions
/// may genuinely differ.
///
/// # The launch direction
///
/// The craft's forward, and the offset out to the nose is **ours** - the original
/// spawns at the craft's pose and lets the first tick sort it out. Same choice
/// [`super::launch`] makes, same reason: a projectile that starts inside the hull
/// that fired it is a projectile the sweep has to special-case.
#[must_use]
pub fn launch(
    state: &ShipState,
    dimensions: &Dimensions,
    stats: &MissileStats,
    // **Deliberately unused, and kept in the signature.** A rocket's launch speed
    // is its class speed plus `launchSpeed`; a missile's is its *launcher's*
    // speed plus `launchSpeed`, and the class speed is where the ramp ends rather
    // than where it starts (`speed_kmh`). Dropping the parameter would make the
    // two launchers look gratuitously different and invite somebody to "fix" this
    // one back to the rocket's rule.
    _class: &str,
) -> (Vec3, Vec3, f32) {
    let forward = state.body.forward();
    let nose = state.body.position
        + forward * oag_physics::wall::hull_extent(&state.body, dimensions, forward);

    let launch_kmh =
        state.body.linear_velocity.length() * KMH_PER_UNIT_PER_SECOND + stats.launch_speed;

    // **The floor applies to the launch velocity and not to the ramp's base**,
    // which is the original's arrangement rather than a simplification of it:
    // `Missile_Init` stores the unfloored sum at `self+0x48` - which is what
    // `speed_kmh` blends from ever after - and applies `vmax_s(_, 14.4)` only to
    // the value it turns into the initial velocity. The two differ only for a
    // launch under 14.4 km/h, which the shipped `launchSpeed="200"` makes
    // unreachable; reproduced anyway, because a table that authored a small one
    // would otherwise diverge silently.
    let speed = speed_units_guided(launch_kmh.max(LAUNCH_SPEED_FLOOR_KMH));
    (nose, forward * speed, launch_kmh)
}

#[cfg(test)]
mod tests;
