//! The Rocket: three at once, fanned by the angle its own `<Stats>` authors.
//!
//! Split out of `projectile.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, along the seam the other two weapons already
//! use: [`super::missile`] and [`super::mine`] each own their launch rule, and
//! the Rocket's was the one still living in the parent. A move, with no
//! behaviour change - both items are re-exported from [`super`], so
//! `projectile::launch` and `projectile::ROCKET_SHOTS` still resolve and no
//! call site moved.
//!
//! The flight model stays in [`super::Projectiles::advance`], because it is not
//! the Rocket's: it is what every projectile that follows the floor does, and
//! the Missile inherits it.

use oag_core::math::Vec3;
use oag_physics::ShipState;
use oag_tables::weapons::RocketStats;

use super::KMH_PER_UNIT_PER_SECOND;

/// How many rockets one press puts in the air.
///
/// **Recovered, confidence 88.** `Weapon_FireRocket` (`0x0886e104`) makes three
/// literal calls to one spawn helper in a single invocation, with no timer
/// between them: one through the craft's own matrix, one through it rotated by
/// `+spread`, one by `-spread`. See
/// `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`.
pub const ROCKET_SHOTS: usize = 3;

/// How long a rocket that hits nothing stays in the air before the pool reaps
/// it, in seconds.
///
/// **Recovered, confidence 90.** `RocketPool_Update` (`0x0886de60`) tests
/// `5.0 < rocket+0x48` on every live slot in its second pass and sets the
/// same retire bit a wall sets; the teardown that follows releases the trail
/// and plays no explosion and spends no blast, so an expired rocket vanishes
/// rather than detonating - see `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`,
/// "What a rocket hit spends". Tested against the age at the end of the tick,
/// strictly greater, the way the pool does.
pub const LIFETIME_SECONDS: f32 = 5.0;

/// How much slower than the class a rocket leaves, until its first surface hit.
///
/// **Measured 2026-10-01, confidence 88** (Pulse PSP on PPSSPP, Venom, six
/// rockets over two runs): the first four updates read 166.67 units/s, which is
/// 0.75 x `venomspeed` 800 km/h / 3.6. `Rocket_Init` copies the craft's display
/// matrix into the rocket and scales its direction row by `SpeedForClass / 3.6`;
/// that matrix carries `g_craft_scale` (0.75), so the row is 0.75 long. The first
/// surface-probe hit in `Rocket_Update` renormalises it to the class speed (the
/// divide at `0x0885d6c8`), which is the step to 222.22 units/s. `launchSpeed`
/// plays no part. Why the step lands on the fourth update was not separated
/// from the age. Measured on Venom only; Flash, Rapier and Phantom were not run.
/// See `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`'s 2026-10-01 section.
///
/// It is [`oag_physics::hover::TARGET_GLOBAL_SCALE`], the same global.
pub const LAUNCH_SPEED_SCALE: f32 = oag_physics::hover::TARGET_GLOBAL_SCALE;

/// Where a craft launches its rockets from, and how fast.
///
/// Returns [`ROCKET_SHOTS`] `(position, velocity)` pairs in the original's own
/// order - straight ahead, `+spread`, `-spread` - because that order decides
/// which projectile slot each lands in, and the slot is hashed state.
///
/// # What is recovered here, and what is not
///
/// **Recovered.** That there are three; that they leave *together* rather than
/// as a burst; that they share the craft's position and differ only by a
/// rotation built from `<Rocket spread>` and its negation; and that `spread` is
/// an angle in radians (from the `vcst_s(5)` = `2/pi` the original multiplies by
/// before `vcos_s`/`vsin_s`, which is Allegrex's radians-to-quarter-turns
/// conversion).
///
/// **Ours.**
///
/// - **The axis the fan rotates about.** The original builds its rotation
///   through four `vpfxs`-prefixed lanes and reading the axis back off the
///   prefixes was not attempted. The craft's **up** axis is what a lateral
///   spread of forward-firing rockets wants, and it is what this uses.
/// - **The launch point is the craft's own position, measured 2026-10-01.**
///   A live `Rocket_Update` probe reads the rocket's spawn at `124.5, -47.9,
///   -196.9` against the body's `124.5, -47.9, -197.0`, to 0.1 unit. This pushed
///   the origin forward by the hull's own extent, so a rocket started outside
///   the craft that fired it; that was this engine's, and the original does not.
///   All three share it. The rocket is no longer clear of its owner's hull at
///   the first tick, and [`super::nearest_hit`] excludes the owner already.
/// - **The launch speed is 0.75 x the class speed, and the class speed alone
///   afterwards, measured 2026-10-01.** See [`LAUNCH_SPEED_SCALE`].
/// - **The speed is the class's alone, measured 2026-10-01** (Pulse PSP on
///   PPSSPP, Venom, Time Trial on Talon's Junction): a live `Rocket_Update`
///   probe reads 222.22 units/s, which is `venomspeed` 800 km/h divided by 3.6,
///   and held. This took `class + launchSpeed` (277.78) before; `launchSpeed`
///   plays no part in `Rocket_Init` or `Rocket_Update`. What `launchSpeed` *is*
///   for on a Rocket has not been found. Measured on Pulse PSP only: HD and
///   Pure author the same attribute and were not probed. The craft's own
///   velocity is not inherited either, which the same probe agrees with. See
///   `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`'s 2026-10-01 section.
///
/// # The authored speeds are km/h, not units per second
///
/// **Recovered, confidence 84** - `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
/// `Rocket_SpeedForClass` (`0x0885d1b0`) returns the authored float for the
/// current class and does no arithmetic, and **both** of its callers divide by
/// `3.6` before it becomes a velocity. Spending the numbers as units per
/// second, which this did until 2026-08-11, flies a rocket **3.6x too fast**:
/// the disc authors `venomspeed="800" launchSpeed="200"`, so a Venom rocket ran
/// at `1000` units/s, which the HUD's own `* 3.6` would read as **3600 km/h**
/// against a craft that tops out near 600. Converted it is `222` units/s, or
/// 800 km/h - faster than the craft, which is what a rocket should be.
///
/// The conversion is at the call site rather than inside
/// [`RocketStats::speed_for`] on purpose, mirroring the original: the lookup
/// hands back the authored figure and the consumer spends it.
#[must_use]
pub fn launch(
    state: &ShipState,
    stats: &RocketStats,
    class: &str,
) -> Option<[(Vec3, Vec3); ROCKET_SHOTS]> {
    let forward = state.body.forward();
    let up = state.body.up();
    // The craft's own position, shared by all three: `Rocket_Init` places the
    // rocket at the position it is handed, which is the craft's.
    let origin = state.body.position;
    let speed = stats.speed_for_named(class)? / KMH_PER_UNIT_PER_SECOND * LAUNCH_SPEED_SCALE;

    // The original's own order. A zero `spread` collapses all three onto the
    // same ray rather than erroring: that is a file that authors no fan, not a
    // broken weapon.
    Some([0.0, stats.spread, -stats.spread].map(|angle| {
        let direction = oag_core::math::quat_from_axis_angle(up, angle) * forward;
        (origin, direction * speed)
    }))
}

/// Puts a volley in the air, each rocket carrying the class speed it will be
/// renormalised to on its first surface hit.
///
/// [`launch`] leaves at [`LAUNCH_SPEED_SCALE`] x the class speed; the class speed
/// itself rides in [`super::Projectile::launch_speed_kmh`] so the flight can
/// pin to it (see [`super::Projectiles::advance`]'s Rocket arm). One entry point
/// for both halves, so dropping either is a failing test and not a quiet change.
///
/// Returns how many left, or `None` where the table authors no speed for this
/// race's class.
pub fn fire(
    projectiles: &mut super::Projectiles,
    state: &ShipState,
    stats: &RocketStats,
    class: &str,
    owner: u8,
) -> Option<usize> {
    let class_kmh = stats.speed_for_named(class)?;
    let shots = launch(state, stats, class)?;
    let mut fired = 0;
    for (position, velocity) in shots {
        if projectiles.spawn_guided(
            oag_tables::weapons::Weapon::Rocket,
            position,
            velocity,
            owner,
            None,
            class_kmh,
        ) {
            fired += 1;
        }
    }
    Some(fired)
}

#[cfg(test)]
mod tests;
