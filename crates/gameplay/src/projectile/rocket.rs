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
use oag_physics::params::Dimensions;
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
/// - **The launch offset.** The original passes the craft's pose for the
///   position and varies only the matrix, so all three share an origin - that
///   part is recovered. Pushing that origin forward by the hull's own extent,
///   so a rocket starts outside the craft that fired it, is this engine's, and
///   it uses the craft's *unrotated* forward so the three still share it.
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
    dimensions: &Dimensions,
    stats: &RocketStats,
    class: &str,
) -> Option<[(Vec3, Vec3); ROCKET_SHOTS]> {
    let forward = state.body.forward();
    let up = state.body.up();
    let nose = state.body.position
        + forward * oag_physics::wall::hull_extent(&state.body, dimensions, forward);
    let speed = stats.speed_for_named(class)? / KMH_PER_UNIT_PER_SECOND;

    // The original's own order. A zero `spread` collapses all three onto the
    // same ray rather than erroring: that is a file that authors no fan, not a
    // broken weapon.
    Some([0.0, stats.spread, -stats.spread].map(|angle| {
        let direction = oag_core::math::quat_from_axis_angle(up, angle) * forward;
        (nose, direction * speed)
    }))
}
