//! The Rocket: three at once, fanned by the angle its own `<Stats>` authors.
//!
//! Split from `projectile.rs` under the 1,000-line rule, like [`super::missile`]
//! and [`super::mine`]. The flight model stays in [`super::Projectiles::advance`]:
//! it is what every floor-following projectile does.

use oag_core::math::Vec3;
use oag_physics::ShipState;
use oag_tables::weapons::RocketStats;

use super::KMH_PER_UNIT_PER_SECOND;

/// How many rockets one press puts in the air.
///
/// **Recovered, confidence 88.** `Weapon_FireRocket` (`0x0886e104`) makes three
/// calls to one spawn helper, no timer between them: the craft's matrix, rotated
/// `+spread`, rotated `-spread`. See
/// `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`.
pub const ROCKET_SHOTS: usize = 3;

/// How long a rocket that hits nothing stays in the air, in seconds.
///
/// **Recovered, confidence 90.** `RocketPool_Update` (`0x0886de60`) tests
/// `5.0 < rocket+0x48` in its second pass and sets the wall's retire bit; the
/// teardown releases the trail with no explosion or blast, so an expired rocket
/// vanishes (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "What a
/// rocket hit spends"). Tested after the tick, strictly greater.
pub const LIFETIME_SECONDS: f32 = 5.0;

/// How much slower than the class a rocket leaves, until its first surface hit.
///
/// **Measured 2026-10-01, confidence 88** (Pulse PSP on PPSSPP, Venom, six rockets
/// over two runs): the first four updates read 166.67 units/s, 0.75 x `venomspeed`
/// 800 km/h / 3.6. `Rocket_Init` copies the craft's display matrix, carrying
/// `g_craft_scale` (0.75), and scales its direction row by `SpeedForClass / 3.6`;
/// the first surface-probe hit in `Rocket_Update` renormalises to the class speed
/// (the divide at `0x0885d6c8`), 222.22 units/s. `launchSpeed` plays no part. Why
/// the step lands on the fourth update was not separated from the age. Venom only;
/// Flash, Rapier and Phantom not run. See `rocket-visuals.md`, 2026-10-01 section.
///
/// It is [`oag_physics::hover::TARGET_GLOBAL_SCALE`], the same global.
pub const LAUNCH_SPEED_SCALE: f32 = oag_physics::hover::TARGET_GLOBAL_SCALE;

/// Where a craft launches its rockets from, and how fast.
///
/// Returns [`ROCKET_SHOTS`] `(position, velocity)` pairs in the original's order
/// (straight, `+spread`, `-spread`): it decides each projectile's slot, and the
/// slot is hashed state.
///
/// # What is recovered here, and what is not
///
/// **Recovered.** Three leave together, sharing the craft's position and differing
/// by a rotation from `<Rocket spread>` and its negation; `spread` is radians
/// (from the `vcst_s(5)` = `2/pi` the original multiplies by before
/// `vcos_s`/`vsin_s`, Allegrex's radians-to-quarter-turns).
///
/// **Ours.**
///
/// - The axis the fan rotates about. The original builds its rotation through four
///   `vpfxs`-prefixed lanes and the axis was not read back; the craft's up is what
///   a lateral spread wants.
/// - **The launch point is the craft's own position, measured 2026-10-01**: a live
///   `Rocket_Update` probe reads the spawn at `124.5, -47.9, -196.9` against the
///   body's `124.5, -47.9, -197.0`. An earlier forward push by the hull extent was
///   this engine's. [`super::nearest_hit`] excludes the owner already.
/// - **The speed is 0.75 x the class speed at launch, then the class's alone,
///   measured 2026-10-01** (Pulse PSP on PPSSPP, Venom, Time Trial on Talon's
///   Junction): the probe reads 222.22 units/s, `venomspeed` 800 km/h / 3.6, held.
///   This took `class + launchSpeed` (277.78) before; `launchSpeed` plays no part
///   in `Rocket_Init` or `Rocket_Update` and what it is for on a Rocket is not
///   found. Pulse PSP only: HD and Pure author the attribute and were not probed.
///   The craft's velocity is not inherited. See [`LAUNCH_SPEED_SCALE`] and
///   `rocket-visuals.md`, 2026-10-01 section.
///
/// # The authored speeds are km/h, not units per second
///
/// **Recovered, confidence 84**
/// (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`).
/// `Rocket_SpeedForClass` (`0x0885d1b0`) returns the authored float with no
/// arithmetic and both callers divide by `3.6`. Spending them as units per second
/// flew a Venom rocket at `1000` units/s (3600 km/h on the HUD) instead of `222`.
/// The conversion is at the call site, not in [`RocketStats::speed_for`],
/// mirroring the original.
#[must_use]
pub fn launch(
    state: &ShipState,
    stats: &RocketStats,
    class: &str,
) -> Option<[(Vec3, Vec3); ROCKET_SHOTS]> {
    let forward = state.body.forward();
    let up = state.body.up();
    // The craft's own position, shared by all three (`Rocket_Init` places the
    // rocket where it is handed).
    let origin = state.body.position;
    let speed = stats.speed_for_named(class)? / KMH_PER_UNIT_PER_SECOND * LAUNCH_SPEED_SCALE;

    // The original's order. A zero `spread` collapses all three onto one ray: a
    // file authoring no fan, not a broken weapon.
    Some([0.0, stats.spread, -stats.spread].map(|angle| {
        let direction = oag_core::math::quat_from_axis_angle(up, angle) * forward;
        (origin, direction * speed)
    }))
}

/// Puts a volley in the air, each rocket carrying the class speed it is
/// renormalised to on its first surface hit.
///
/// [`launch`] leaves at [`LAUNCH_SPEED_SCALE`] x the class speed; the class speed
/// rides in [`super::Projectile::launch_speed_kmh`] (see
/// [`super::Projectiles::advance`]'s Rocket arm). One entry point for both halves,
/// so dropping either fails a test. Each rocket rides the craft's up until its
/// first probe adopts a surface: `Rocket_Init` seeds the normal from the craft
/// (measured: live rows read `(-0.03, 1.0, 0.05)`, not `(0, 1, 0)`).
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
        if projectiles.spawn_riding(
            oag_tables::weapons::Weapon::Rocket,
            position,
            velocity,
            owner,
            state.body.up(),
            class_kmh,
        ) {
            fired += 1;
        }
    }
    Some(fired)
}

#[cfg(test)]
mod tests;
