//! The Mine: a cluster laid out of the back of a craft, one every tenth of a
//! second, each sitting where it was dropped until something trips it.
//!
//! The sibling of [`super::missile`] in shape and its opposite in behaviour -
//! the Missile is the weapon this engine flies hardest, and the Mine is the one
//! that does not fly at all.
//!
//! # What is recovered and what is ours
//!
//! Stated at the top the way [`crate::pickup`] and [`super`] state it. The
//! evidence is `docs/ghidra/functions/psp-pulse-usa/mine.md`, which is also the
//! page that established this is the Mine: two other pages had this handler
//! down as the Cannon and the Bomb's down as the Mine.
//!
//! **Recovered.**
//!
//! - **That a press lays a cluster rather than one**, and that the cluster is
//!   *staggered in time*: `Weapon_DropMines` (`0x088675cc`) is dispatched every
//!   tick while its bit is set, spawns **one** mine each time a `0.1 s` reload
//!   expires, and decrements a per-craft counter. See [`DROP_INTERVAL`].
//! - **That the craft keeps holding the pickup until the last one is out.** The
//!   handler clears `craft+0x1bc` and its own fire bit only when the counter
//!   reaches zero, which is why [`crate::pickup::Held`] carries the drop rather
//!   than the fire path spending the slot outright.
//! - **Where they come from**: `craft+0xa0`, the rear emitter anchor, which
//!   exactly two weapons use - this one and the Bomb.
//! - **The fuse.** `Mine_Init` (`0x08859ac8`) loads `stats+0xf4` into the
//!   entity's own countdown, and `+0xf4` is `<Weapon type="Mine"><Stats
//!   timetodie>`. Confidence 92 on the offset; it is also the reading that
//!   identified the weapon.
//! - **`trigger_radius` being a distinct, smaller distance than
//!   `blastradius`** - authored, and true in both shipped tables.
//!
//! **Ours.**
//!
//! - **[`CLUSTER`], how many a press lays.** Genuinely unfound: nothing writes
//!   `craft+0x1ac` anywhere in the executable outside the handler's own
//!   decrement, and no `<Stats>` attribute counts mines. Asked of a maintainer
//!   who plays the game and they did not want to guess either, so it is flagged
//!   here and in `handover/` rather than dressed up.
//! - **That a mine does not move.** See [`at_rest`].
//! - **That coming inside `trigger_radius` sets it off.** The attribute is the
//!   disc's and its name is not ambiguous, but the code path that spends it was
//!   not found. See [`triggered_by`].
//! - **That a mine will not be tripped by the craft that laid it**, at any
//!   range. See [`triggered_by`].

use oag_core::math::Vec3;
use oag_formats::weapons::MineStats;
use oag_physics::ShipState;
use oag_physics::params::Dimensions;

/// How long between one mine leaving and the next, in seconds.
///
/// **Recovered, confidence 90, and it is a code literal rather than an authored
/// one.** `Weapon_DropMines` (`0x088675cc`) reloads `craft+0x1b0` with the bit
/// pattern `0x3dcccccd` after every spawn. The Mine's `<Stats>` authors no
/// `rate` - the Cannon is the only weapon that does - so there is nothing this
/// could be read from instead, and that is itself part of why the handler was
/// misattributed to the Cannon for months.
pub const DROP_INTERVAL: f32 = 0.1;

/// How many mines one press lays.
///
/// **Ours, and the one number on this weapon that is.** Two searches have now
/// failed to find what writes the original's counter at `craft+0x1ac`: the only
/// store to that offset on a craft base anywhere in the binary is the handler's
/// own `-= 1`. The shipped `<Weapon type="Mine"><Stats>` authors seven
/// attributes and none of them is a count.
///
/// Five, because the *stagger* is what spreads a cluster and the stagger is
/// recovered: at [`DROP_INTERVAL`] apart and a racing speed near 550 km/h, five
/// mines lay about sixty units of track behind the craft - long enough to read
/// as a carpet rather than as one mine, short enough that eight craft dropping
/// at once stay inside even the original's own 64-slot pool.
///
/// **It is simulation state**, so it is in the determinism reference and
/// changing it is a hash move. `handover/` carries the row asking for it to be
/// measured against the running original.
pub const CLUSTER: u8 = 5;

/// Where a craft lays its mines.
///
/// The tail rather than the nose, which is recovered: `Weapon_RequestFire`
/// (`0x08862d9c`) stores `craft+0xa0` as this weapon's emitter anchor, and the
/// only other weapon that uses that anchor is the Bomb. Everything that leaves
/// the front uses `craft+0x20`.
///
/// Pushed back by the hull's own extent for [`super::launch`]'s reason: a mine
/// that started inside the craft would be tripped, or drawn, in the wrong place.
#[must_use]
pub fn drop_point(state: &ShipState, dimensions: &Dimensions) -> Vec3 {
    let back = -state.body.forward();
    state.body.position + back * oag_physics::wall::hull_extent(&state.body, dimensions, back)
}

/// The velocity a mine is laid with.
///
/// **Zero, and that is a reading rather than a shortcut.** `Mine_Init`
/// (`0x08859ac8`) copies the firing craft's velocity into the entity at
/// `+0xa0`, and nothing that was read integrates it: the one function in the
/// Mine subsystem's range that walks the live pool counts a timer down and
/// sweeps for a blast, and never touches a position. A mine that carried the
/// craft's velocity forward would be a slow rocket, which is not what the
/// weapon is; a mine that carried it and was then slowed would need an
/// invented deceleration.
///
/// So the conservative reading is that `+0xa0` is a *heading* the model is
/// drawn along rather than a velocity, and that a mine is placed and stays
/// placed. The cluster still spreads, because [`DROP_INTERVAL`] spreads it:
/// the craft moves between drops and each mine is laid where the craft then
/// was.
///
/// A function rather than a constant so the reasoning has somewhere to live and
/// so a later read that finds the integration has one place to change.
#[must_use]
pub const fn at_rest() -> Vec3 {
    Vec3::ZERO
}

/// Whether a craft at `position` is close enough to set this mine off.
///
/// **Ours, with the disc's own number in it.** `trigger_radius` is authored and
/// its name says what it is for, but what spends it was not found - the fuse
/// sweep that *was* read range-checks candidates only once the fuse has already
/// expired, which is the blast rather than the trip. So the rule is this
/// engine's reading of an authored attribute rather than a recovered code path,
/// and it is the smallest rule that makes a mine a weapon: without it a mine
/// only ever goes off on its own timer, harming whoever happens to be passing
/// by coincidence.
///
/// **The craft that laid it is never the tripper**, whatever the range. A
/// cluster leaves from the tail of a craft that is by definition standing right
/// there, so the alternative is a mine drop that detonates in the dropper's own
/// face on the tick it is pressed. The original's own arrangement is unread;
/// this is the same exclusion [`super::nearest_hit`] already makes for a
/// rocket's owner, applied to a weapon where it is doing more work.
///
/// Note this is the *trip* only. The blast that follows is
/// [`super::blast`]'s and excludes nobody - a craft that lays a mine and then
/// reverses into its own cluster still takes the damage.
#[must_use]
pub fn triggered_by(mine: Vec3, owner: u8, slot: u8, position: Vec3, stats: &MineStats) -> bool {
    slot != owner && (position - mine).length() <= stats.trigger_radius
}
