//! The Plasma: one bolt, out of the nose, at the class's own speed.
//!
//! The cheapest weapon this project has added since the Bomb, and for the same
//! reason: it shares the whole flight model with [`super::rocket`]. What
//! differs is the *count* - one where the Rocket fires three - and that is
//! read rather than assumed. See
//! `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
//!
//! The flight itself stays in [`super::Projectiles::advance`], which is where
//! it belongs: `Plasma_Update` (`0x0885c6cc`) is the Rocket's own floor
//! follower instruction for instruction - the same 12-unit probe along the
//! carried surface normal, the same redirect that preserves speed, the same
//! fall when the probe finds nothing, the same detonate on a wall.

use oag_core::math::Vec3;
use oag_formats::weapons::PlasmaStats;
use oag_physics::ShipState;
use oag_physics::params::Dimensions;

use super::KMH_PER_UNIT_PER_SECOND;

/// Where a craft launches its plasma bolt from, and how fast.
///
/// # One shot, and it is read twice over
///
/// **The handler.** `Weapon_FirePlasma` (`0x0886a868`), the bit-`0x4` handler
/// in `Weapons_DispatchFire`, takes one slot out of a 16-entry pool, calls
/// `Plasma_Init` once, and clears its own request bit in the same breath. It
/// has neither of the two shapes that make a weapon fire more than once: no
/// three literal spawn calls (the Rocket's) and no reload timer with a round
/// counter (the Mine's).
///
/// **The schema.** The Plasma's `<Stats>` carries no `spread`, and `spread` is
/// what the Rocket's fan is built from. The same absence reads the same way on
/// the Missile, which also fires one.
///
/// # What is shared with the Rocket, and is therefore equally ours
///
/// The launch offset - pushing the origin forward by the hull's own extent so
/// a bolt starts outside the craft that fired it - and the speed being the
/// class's **plus** `launchSpeed` rather than one or the other. Both are
/// [`super::launch`]'s choices, taken here so the two weapons cannot drift
/// apart; both are flagged in that function's own doc comment rather than
/// restated. The authored speeds are km/h for the Rocket's measured reason.
#[must_use]
pub fn launch(
    state: &ShipState,
    dimensions: &Dimensions,
    stats: &PlasmaStats,
    class: oag_formats::handling::SpeedClass,
) -> (Vec3, Vec3) {
    let forward = state.body.forward();
    let nose = state.body.position
        + forward * oag_physics::wall::hull_extent(&state.body, dimensions, forward);
    let speed = (stats.speed_for(class) + stats.launch_speed) / KMH_PER_UNIT_PER_SECOND;
    (nose, forward * speed)
}
