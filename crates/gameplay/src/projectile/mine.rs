//! The two weapons that come out of the back: the Mine, a cluster laid one
//! every tenth of a second, and the Bomb, a single bigger one.
//!
//! **One module for both, because they are one weapon in two sizes.** The Bomb
//! authors six of the Mine's seven attributes and every one of them is larger
//! on both shipped tables; a maintainer who plays Pulse describes it as "a
//! single big mine", asked without being shown any of this; and
//! `Weapon_FireBomb` (`0x08863a20`) differs from `Weapon_DropMines`
//! (`0x088675cc`) in exactly one structural way - it spawns once where the
//! other reloads a timer and spawns again. Splitting them would mean two copies
//! of [`triggered_by`] and [`at_rest`] and two places for the same reading to
//! drift.
//!
//! Everything below says "mine" and means both unless it says otherwise; the
//! places they differ are [`CLUSTER`] and [`Drop::count`].
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

use super::{Impact, Projectile};
use oag_core::math::Vec3;
use oag_formats::weapons::{BombStats, MineStats, Weapon};
use oag_physics::ShipState;
use oag_physics::params::Dimensions;

/// What one press of a rear weapon lays: how many, and with what fuse and trip.
///
/// The seam between the two weapons, and it is deliberately small - a count and
/// three floats. Everything else about a Mine and a Bomb in this engine is the
/// same code path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drop {
    /// How many charges the press lays.
    ///
    /// [`CLUSTER`] for the Mine and **one** for the Bomb. The Bomb's one is
    /// recovered - `Weapon_FireBomb` (`0x08863a20`) makes a single spawn call
    /// with no reload timer anywhere in it - which is worth contrasting with the
    /// Mine's, where the count is the invented number on this whole weapon.
    pub count: u8,
    /// Seconds before a laid charge goes off on its own.
    pub fuse: f32,
    /// How close a craft must come to set it off early.
    pub trigger_radius: f32,
}

impl Drop {
    /// The Mine's drop.
    #[must_use]
    pub const fn mine(stats: &MineStats) -> Self {
        Self {
            count: CLUSTER,
            fuse: stats.timetodie,
            trigger_radius: stats.trigger_radius,
        }
    }

    /// The Bomb's, which is the same thing with a count of one.
    #[must_use]
    pub const fn bomb(stats: &BombStats) -> Self {
        Self {
            count: 1,
            fuse: stats.timetodie,
            trigger_radius: stats.trigger_radius,
        }
    }

    /// The drop one weapon makes, or `None` for a weapon that lays nothing and
    /// for a table that authors no block for it.
    ///
    /// The one place the two rear weapons are told apart, so a caller never has
    /// to. A weapon that is not a rear weapon is `None` rather than a panic:
    /// asking is how [`crate::pickup::IMPLEMENTED`]'s callers stay honest.
    ///
    /// **This and [`TriggerRadii::get`] are the same list written twice**, and
    /// neither fails loudly on its own: a third rear weapon missing an arm here
    /// simply lays nothing, and one missing an arm there is laid and can never
    /// be tripped. `every_rear_weapon_can_be_laid_and_tripped` in
    /// [`crate::pickup::tests`] is what makes the pair fail together.
    #[must_use]
    pub fn for_weapon(weapon: Weapon, weapons: &oag_formats::weapons::WeaponStats) -> Option<Self> {
        match weapon {
            Weapon::Mine => weapons.mine().as_ref().map(Self::mine),
            Weapon::Bomb => weapons.bomb().as_ref().map(Self::bomb),
            _ => None,
        }
    }
}

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
/// **The Bomb reaches the same answer by a different route**, and the two should
/// not be confused: everything above is a negative result about the *Mine's*
/// subsystem, and none of it is evidence about the Bomb's. `Weapon_FireBomb`
/// (`0x08863a20`) stages the craft's forward row **negated** and hands it to a
/// spawn helper at `0x0885f188` that could not be resolved statically - two
/// callers jump past its prologue. So the Bomb's direction is recovered and its
/// speed is unfound, which is a weaker position than the Mine's. What settles it
/// is a maintainer who plays Pulse, asked directly: "the bomb should be static,
/// like the mines, just a single big mine". A negated matrix row is also a unit
/// vector, so read as a velocity it would be one unit a second - at racing speed,
/// indistinguishable from static anyway.
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
/// Takes the radius rather than a stats block, because the Bomb's and the
/// Mine's come from two different structs and the rule does not care which.
///
/// Note this is the *trip* only. The blast that follows is
/// [`super::blast`]'s and excludes nobody - a craft that lays a mine and then
/// reverses into its own cluster still takes the damage.
#[must_use]
pub fn triggered_by(mine: Vec3, owner: u8, slot: u8, position: Vec3, trigger_radius: f32) -> bool {
    slot != owner && (position - mine).length() <= trigger_radius
}

/// The trip radius of each weapon that has one, so [`super::Projectiles::advance`] can
/// be handed both without a whole weapon table.
///
/// A pair of `Option<f32>` rather than the table itself, because `advance` is
/// the deterministic core and the table is a loaded asset: the less of it
/// crosses that line the fewer ways a race can differ from a replay of itself.
/// `None` is a weapon the table does not author, and a charge of that kind
/// cannot be tripped - see [`advance_laid`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TriggerRadii {
    /// `<Weapon type="Mine"><Stats trigger_radius>`.
    pub mine: Option<f32>,
    /// `<Weapon type="Bomb"><Stats trigger_radius>`.
    pub bomb: Option<f32>,
}

impl TriggerRadii {
    /// Both, out of a weapon table, or both `None` for a table that did not
    /// load.
    #[must_use]
    pub fn from_table(weapons: Option<&oag_formats::weapons::WeaponStats>) -> Self {
        let Some(weapons) = weapons else {
            return Self::default();
        };
        Self {
            mine: weapons.mine().map(|s| s.trigger_radius),
            bomb: weapons.bomb().map(|s| s.trigger_radius),
        }
    }

    /// One weapon's, or `None` for a weapon that is not laid at all.
    ///
    /// The twin of [`Drop::for_weapon`]; see that function for why the pair is
    /// guarded by a test rather than by the type system.
    #[must_use]
    pub fn get(self, kind: Weapon) -> Option<f32> {
        match kind {
            Weapon::Mine => self.mine,
            Weapon::Bomb => self.bomb,
            _ => None,
        }
    }
}

/// One tick of a laid charge - a mine or a bomb: the trip test, then the fuse.
///
/// Returns the blast, if this is the tick it goes off. Two ways it can be: a
/// craft came inside `trigger_radius`, or the authored fuse ran out. The trip is
/// tested **first**, so a charge tripped on the same tick its fuse expires is
/// recorded as having struck the craft that tripped it - which is the more
/// informative of two answers that do the same damage.
///
/// A charge whose weapon the table does not author cannot be tripped and simply
/// expires without a blast - the same "no authored numbers, spend no blast" rule
/// [`super::blast_stats`] follows, and reachable only through a bug upstream in
/// [`crate::pickup::IMPLEMENTED`].
pub(super) fn advance_laid(
    projectile: &mut Projectile,
    kind: Weapon,
    dt: f32,
    ships: &[crate::world::Ship],
    trigger_radii: TriggerRadii,
) -> Option<Impact> {
    let here = projectile.position;
    let owner = projectile.owner;

    if let Some(trigger_radius) = trigger_radii.get(kind) {
        // Slot order, and slot order only - the first craft in the array that is
        // close enough trips it. Which craft is credited when two are inside the
        // radius on one tick is therefore the array's business rather than the
        // geometry's, and that is deliberate: the alternative is a nearest-first
        // search whose result depends on distances that are equal often enough
        // to matter. See `docs/architecture/determinism.md`.
        for (slot, ship) in ships.iter().enumerate() {
            if !ship.active {
                continue;
            }
            let slot = slot as u8;
            if triggered_by(
                here,
                owner,
                slot,
                ship.physics.body.position,
                trigger_radius,
            ) {
                return Some(Impact {
                    point: here,
                    kind,
                    owner,
                    struck: Some(slot),
                    blast: true,
                });
            }
        }
    }

    projectile.lifetime -= dt;
    if projectile.lifetime > 0.0 {
        return None;
    }
    // **Both ways out of here spend a blast**, which is the opposite of the
    // Missile's expiry and is recovered rather than assumed. `Impact::blast`
    // exists because a missile that runs out of time spawns its explosion and
    // hurts nobody - the pool's teardown calls the spawner and neither the
    // damage nor the force. A mine's fuse running out is the other shape
    // entirely: the chain that reads it (`FUN_08867370` counts `+0x48` down,
    // `FUN_08867b50` sweeps, `Weapon_PostBlastImpulse` posts) *is* the blast,
    // and there is no path where it goes off quietly. A mine that expired
    // harmlessly would be a mine nobody ever needed to drive around.
    // The fuse. An unauthored weapon is the one path that reaches here with
    // nothing to spend, and it produces an impact whose blast lookup then finds
    // nothing - one free slot and no damage, rather than a charge that lives for
    // ever.
    Some(Impact {
        point: here,
        kind,
        owner,
        struck: None,
        blast: true,
    })
}
