//! The Shuriken: one blade, thrown twenty degrees off the nose, that bounces.
//!
//! The flight itself is [`super::Projectiles::advance`]'s, and that is a
//! *reading* rather than a convenience. `Shuriken_Update` (`0x08877bdc`) is
//! `Rocket_Update`'s floor follower - the same probe along a carried surface
//! normal, the same speed-preserving redirect onto whatever it finds, the same
//! [`super::FALL_ACCELERATION`] when it finds nothing - and it differs in
//! exactly one way that matters: **a wall calls `Shuriken_Bounce`
//! (`0x088778ac`) where a rocket detonates**. See
//! `docs/ghidra/functions/psp-pulse-usa/shuriken.md`.
//!
//! What lives here is the launch, which is the weapon's own: a single blade at
//! one of two fixed angles, on a coin flip, carrying the throwing craft's own
//! speed.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_physics::ShipState;
use oag_physics::params::Dimensions;
use oag_tables::weapons::ShurikenStats;

use super::KMH_PER_UNIT_PER_SECOND;

/// How far off the nose a blade leaves, in radians.
///
/// **Recovered, confidence 88.** `Shuriken_Init` (`0x08877280`) rotates the
/// launch basis by the literal `0x3eb2b8c3` or by its exact negation
/// `0xbeb2b8c3`, and nothing else. `0x3eb2b8c3` is `0.349066`, which is
/// `20.000` degrees to five significant figures.
///
/// **This is not a `spread`.** The Rocket fans three shots across `-spread`,
/// `0`, `+spread`; the Shuriken throws *one*, at one of these two angles, and
/// the thrower does not choose which. The `<Stats>` say the same thing from the
/// other end by authoring no `spread` at all.
pub const LAUNCH_ANGLE: f32 = 0.349_066;

/// How far out along a wall's normal a bounced blade is placed.
///
/// **Recovered, confidence 88.** `Shuriken_Bounce` scales the hit normal by the
/// code literal `0x3dcccccd` - `0.1` - and adds it to the contact point, so the
/// next tick's sweep does not start inside the geometry it just left.
///
/// Its own constant rather than [`super::missile::BOUNCE_PUSH_OFF`]: the two
/// weapons bounce by the same *law* and the original writes two different
/// numbers, which is the kind of thing that gets quietly unified and then
/// cannot be un-unified.
pub const BOUNCE_PUSH_OFF: f32 = 0.1;

/// Where a craft throws its blade from, how fast, and which way it veers.
///
/// # What is recovered
///
/// **One blade**, from the fire handler spawning once and clearing its own
/// request bit in the same breath - the Plasma's argument exactly.
///
/// **[`LAUNCH_ANGLE`], left or right, on a fair coin.**
/// `Weapon_FireShuriken` draws a float and passes `roll > 0.5` to the
/// constructor, which spends it choosing between the constant and its negation.
///
/// **The throwing craft's own speed is added to the class's.** The handler
/// hands the constructor `craft_speed * 3.6` - km/h - and the constructor
/// computes `(craft_kmh + authored) / 3.6`. That is measured, and it is the
/// same question `super::launch` records as *unmeasured* for the Rocket; the
/// two weapons are not assumed to agree, and this one is the one with a
/// reading.
///
/// # What is ours
///
/// - **The axis the rotation is about.** The original builds a
///   forward / re-orthogonalised-up / cross basis and reads the velocity off the
///   third member, and which of the three the rotation turns about was not read
///   off the `vpfx` prefixes. The craft's **up** axis is what a blade veering
///   left or right wants, and it is what this uses - the same reading, and the
///   same caveat, `super::launch` makes for the Rocket's fan.
/// - **The launch offset**, pushing the origin forward by the hull's own extent
///   so a blade starts outside the craft that threw it. Shared with the Rocket
///   and the Plasma so the three cannot drift apart.
///
/// # The draw is one `next_f32`, and it is hashed state
///
/// `rng` is [`crate::world::World::rng`], the seeded generator every other
/// draw in the simulation goes through - never OS entropy, per
/// `docs/architecture/determinism.md`. One `f32` is drawn per throw, so a race
/// in which a Shuriken is thrown has a different generator state afterwards
/// than one in which it is not. That is the original's own arrangement (the
/// handler draws unconditionally inside the pool-space check) and it is why
/// this takes `&mut Rng` rather than a pre-rolled `bool`.
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

    // The craft's speed along its own nose, floored at zero: a craft being
    // shunted backwards throws a blade forwards, not into itself.
    let craft_kmh = state.body.linear_velocity.dot(forward).max(0.0) * KMH_PER_UNIT_PER_SECOND;
    let speed =
        (craft_kmh + stats.speed_for_named(class)? + stats.launch_speed) / KMH_PER_UNIT_PER_SECOND;

    // The coin, drawn before the angle is chosen so the draw happens exactly
    // once whichever way it lands.
    let angle = if rng.next_f32() > 0.5 {
        -LAUNCH_ANGLE
    } else {
        LAUNCH_ANGLE
    };
    let direction = oag_core::math::quat_from_axis_angle(up, angle) * forward;
    Some((nose, direction * speed))
}
