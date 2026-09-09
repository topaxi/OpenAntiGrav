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
use oag_physics::ShipState;
use oag_physics::params::Dimensions;
use oag_tables::weapons::PlasmaStats;

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
    class: &str,
) -> Option<(Vec3, Vec3)> {
    let (nose, forward) = muzzle(state, dimensions);
    let speed = (stats.speed_for_named(class)? + stats.launch_speed) / KMH_PER_UNIT_PER_SECOND;
    Some((nose, forward * speed))
}

/// Where a bolt sits on the craft, and which way it is pointing, right now.
///
/// Split out of [`launch`] because a charging bolt needs the same two values
/// every tick of its wind-up and must not re-resolve a speed to get them - see
/// [`CHARGE_SECONDS`] and [`super::Projectiles::advance`]'s charging branch.
/// The offset is [`super::launch`]'s own choice, shared with the Rocket so the
/// two weapons cannot drift apart.
#[must_use]
pub fn muzzle(state: &ShipState, dimensions: &Dimensions) -> (Vec3, Vec3) {
    let forward = state.body.forward();
    let nose = state.body.position
        + forward * oag_physics::wall::hull_extent(&state.body, dimensions, forward);
    (nose, forward)
}

/// How long a plasma bolt winds up on the nose before it flies, in seconds.
///
/// **Recovered, confidence 90, and it is a literal rather than an authored
/// number.** `Plasma_Init` (`0x0885bd18`) marks the fresh pool entry charging
/// and seeds its countdown in two adjacent stores:
///
/// ```text
/// *(undefined1 *)(entity + 0x4c) = 1;              // charging
/// *(undefined4 *)(entity + 0x50) = _DAT_08a7c098;  // 0x3f800000 == 1.0f
/// ```
///
/// `0x08a7c098` was read straight out of `.rodata` as `0x3f800000`. The
/// consumer is the pool walker `Plasmas_Update` (`0x0886b490`), which for a
/// charging entity calls `Plasma_UpdateCharge` (`0x0885c170`) - copying the
/// firing craft's own weapon-node world matrix onto the bolt, so it rides the
/// nose - subtracts `dt` from `+0x50`, and calls `Plasma_Launch`
/// (`0x0885bf84`) the tick that reaches zero. `Plasma_Launch` clears `+0x4c`
/// and builds the velocity from the craft's forward, which is why the shot
/// leaves along the *current* heading.
///
/// **Wipeout Pure hard-codes the same second**, in the same two stores, from
/// the file its own allocator names: `Plasma_Init` there is `FUN_0885df98` and
/// it writes an immediate `0x3f800000`, with
/// `c:/Work/Wipeout/Code/Backend/Weapons/Plasma.cpp` line 63 as its allocation
/// tag. Two titles, one literal.
///
/// # This is **not** `<Plasma charge_time>`, and that is the finding
///
/// The file authors `charge_time="3"` - on Pulse's `WeaponStats_Race.xml`, on
/// its `WeaponStats_Elimination.xml` and on Pure's `weaponstats.xml`, the only
/// weapon that authors it at all - and **nothing reads it in either
/// executable**. `WeaponStats_ParsePlasma` (`0x0880cc2c`) stores it at
/// `stats+0x9c` and that offset is never loaded again: across all 69 functions
/// in `psp-pulse-usa` that reach the weapon-stats table
/// (`*(int *)(&DAT_08b32420 + DAT_08b32428 * 4)`), a sweep of every load and
/// store at `+0x9c` finds **zero**, while the same sweep at the `venomspeed`
/// control offset `+0xac` finds fourteen including the known consumer
/// `Plasma_SpeedForClass` (`0x0885c5a4`). The sweep works; the negative is
/// real. So the wind-up is three times shorter than the authored attribute
/// suggests, and this constant is the measured one rather than the plausible
/// one. See `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
pub const CHARGE_SECONDS: f32 = 1.0;

#[cfg(test)]
mod tests;
