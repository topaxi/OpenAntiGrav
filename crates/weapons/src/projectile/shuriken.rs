//! The Shuriken: one blade, thrown twenty degrees off the nose, that bounces.
//!
//! The flight is [`super::Projectiles::advance`]'s, a reading: `Shuriken_Update`
//! (`0x08877bdc`) is `Rocket_Update`'s floor follower (probe along a carried
//! normal, speed-preserving redirect, [`super::FALL_ACCELERATION`]) except that
//! **a wall calls `Shuriken_Bounce` (`0x088778ac`) where a rocket detonates**. See
//! `docs/ghidra/functions/psp-pulse-usa/shuriken.md`. Here is the weapon's own
//! launch: one blade at one of two fixed angles, on a coin flip, carrying the
//! thrower's speed.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_physics::ShipState;
use oag_physics::params::Dimensions;
use oag_tables::weapons::ShurikenStats;

use super::KMH_PER_UNIT_PER_SECOND;

/// How far off the nose a blade leaves, in radians.
///
/// **Recovered, confidence 88.** `Shuriken_Init` (`0x08877280`) rotates the basis
/// by the literal `0x3eb2b8c3` (`0.349066`, `20.000` degrees) or its negation
/// `0xbeb2b8c3`. Not a `spread`: one blade at one of two angles, which the thrower
/// does not choose; the `<Stats>` author no `spread`.
pub const LAUNCH_ANGLE: f32 = 0.349_066;

/// How far out along a wall's normal a bounced blade is placed.
///
/// **Recovered, confidence 88.** `Shuriken_Bounce` adds the hit normal times the
/// literal `0x3dcccccd` (`0.1`) to the contact point, so the next sweep does not
/// start inside the geometry. Its own constant, not
/// [`super::missile::BOUNCE_PUSH_OFF`]: the original writes two numbers for the
/// same law, and unifying them cannot be undone.
pub const BOUNCE_PUSH_OFF: f32 = 0.1;

/// Where a craft throws its blade from, how fast, and which way it veers.
///
/// # What is recovered
///
/// **One blade**: the fire handler spawns once and clears its request bit, as for
/// the Plasma. **[`LAUNCH_ANGLE`], left or right, on a fair coin**:
/// `Weapon_FireShuriken` draws a float and passes `roll > 0.5` to the constructor.
/// **The thrower's speed is added to the class's**: the handler passes
/// `craft_speed * 3.6` and the constructor computes `(craft_kmh + authored) /
/// 3.6`. Measured, and the question `super::launch` records as unmeasured for the
/// Rocket; the two are not assumed to agree.
///
/// # What is ours
///
/// - The axis of rotation. The original builds a forward / re-orthogonalised-up /
///   cross basis and the turning axis was not read off the `vpfx` prefixes; the
///   craft's up is used, with `super::launch`'s caveat for the Rocket's fan.
/// - The launch offset, forward by the hull's extent, shared with the Rocket and
///   Plasma.
///
/// # The draw is one `next_f32`, and it is hashed state
///
/// `rng` is `oag_gameplay::world::World::rng`, the seeded generator
/// (`docs/architecture/determinism.md`). One `f32` is drawn per throw, so the
/// generator state differs after a throw, as in the original (the handler draws
/// unconditionally inside the pool-space check); hence `&mut Rng`, not a `bool`.
#[must_use]
pub fn launch(
    state: &ShipState,
    dimensions: &Dimensions,
    stats: &ShurikenStats,
    class: &str,
    rng: &mut Rng,
) -> Option<(Vec3, Vec3)> {
    let forward = state.body.forward();
    let up = state.body.up();
    let nose = state.body.position
        + forward * oag_physics::wall::hull_extent(&state.body, dimensions, forward);

    // Speed along the nose, floored at zero: a craft shunted backwards throws
    // forwards, not into itself.
    let craft_kmh = state.body.linear_velocity.dot(forward).max(0.0) * KMH_PER_UNIT_PER_SECOND;
    let speed =
        (craft_kmh + stats.speed_for_named(class)? + stats.launch_speed) / KMH_PER_UNIT_PER_SECOND;

    // Drawn before the angle is chosen so it happens exactly once.
    let angle = if rng.next_f32() > 0.5 {
        -LAUNCH_ANGLE
    } else {
        LAUNCH_ANGLE
    };
    let direction = oag_core::math::quat_from_axis_angle(up, angle) * forward;
    Some((nose, direction * speed))
}
