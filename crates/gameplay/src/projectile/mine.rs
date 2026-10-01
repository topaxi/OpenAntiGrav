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
//! - **[`CLUSTER`], how many a press lays: five.** Measured on the running
//!   original on 2026-09-15 (three clusters, three craft, the counter at
//!   `craft+0x1ac` reading `5` at the first `Weapon_DropMines` hit every
//!   time) and read off `WeaponPickup_ArmMine` (`0x0886759c`), the grant-path
//!   arm two static sweeps had missed. Confidence 92.
//!
//! **Ours.**
//!
//! - **That a mine does not move.** See [`at_rest`].
//! - **That coming inside `trigger_radius` sets it off.** The attribute is the
//!   disc's and its name is not ambiguous, but the code path that spends it was
//!   not found. See [`triggered_by`].
//! - **That a mine will not be tripped by the craft that laid it**, at any
//!   range. See [`triggered_by`].

use super::{Impact, Projectile};
use oag_core::math::{Quat, Vec3};
use oag_physics::ShipState;
use oag_tables::weapons::{BombStats, MineStats, Weapon};

/// What one press of a rear weapon lays: how many, and with what fuse and trip.
///
/// The seam between the two weapons, and it is deliberately small - a count and
/// three floats. Everything else about a Mine and a Bomb in this engine is the
/// same code path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drop {
    /// How many charges the press lays.
    ///
    /// [`CLUSTER`] for the Mine and **one** for the Bomb. Both are recovered:
    /// the Mine's five is armed by `WeaponPickup_ArmMine` and was measured
    /// live, and the Bomb's one has no counter at all - `WeaponPickup_Grant`
    /// writes its id inline with no arm call, and `Weapon_FireBomb`
    /// (`0x08863a20`) makes a single spawn call with no reload timer anywhere
    /// in it.
    pub count: u8,
    /// Seconds before a laid charge goes off on its own, or [`NO_FUSE`] for
    /// a charge that never does.
    ///
    /// **Pure's Bomb is the one that never does**, and that is read rather
    /// than defaulted: its `<Stats>` authors no `timetodie`, its parser has no
    /// branch for one, and its pool (`BombPool_Update`, `0x0884e968` on
    /// `psp-pure-usa`) spends the charge's age on the model's spin and
    /// compares it to nothing - see
    /// `oag_tables::weapons::BombStats::timetodie`. A `None` there becomes
    /// [`NO_FUSE`] here, so the countdown in [`advance_laid`] runs and never
    /// reaches zero, which is the honest shape of "sits until tripped" in a
    /// field that is otherwise a fuse.
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
            fuse: match stats.timetodie {
                Some(seconds) => seconds,
                None => NO_FUSE,
            },
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
    pub fn for_weapon(weapon: Weapon, weapons: &oag_tables::weapons::WeaponStats) -> Option<Self> {
        match weapon {
            Weapon::Mine => weapons.mine().as_ref().map(Self::mine),
            Weapon::Bomb => weapons.bomb().as_ref().map(Self::bomb),
            _ => None,
        }
    }
}

/// The fuse of a charge that has none: positive infinity.
///
/// `advance_laid` subtracts `dt` from the countdown every tick and detonates
/// at or below zero; infinity minus anything is infinity, so a charge laid
/// with this never times out and only a craft entering `trigger_radius` ends
/// it. Chosen over an `Option` on [`super::Projectile::lifetime`] because that
/// field is hashed for every projectile on every tick and an infinity is one
/// bit pattern where a widened field would move every reference - and because
/// "never" is exactly what the arithmetic already says. **Pure's Bomb is the
/// only weapon that lays one**; see [`Drop::fuse`].
pub const NO_FUSE: f32 = f32::INFINITY;

/// How long between one mine leaving and the next, in seconds.
///
/// **Recovered, confidence 92, and it is a code literal rather than an authored
/// one.** `Weapon_DropMines` (`0x088675cc`) reloads `craft+0x1b0` with the bit
/// pattern `0x3dcccccd` after every spawn - and the running original drops at
/// six-frame intervals (seven where a frame's `dt` jitter left the timer at
/// `0.0002`), measured 2026-09-15 alongside [`CLUSTER`]. The Mine's `<Stats>` authors no
/// `rate` - the Cannon is the only weapon that does - so there is nothing this
/// could be read from instead, and that is itself part of why the handler was
/// misattributed to the Cannon for months.
pub const DROP_INTERVAL: f32 = 0.1;

/// How many mines one press lays.
///
/// **Recovered, confidence 92 - and it was the invented number, unchanged.**
/// Two static sweeps had failed to find what writes the original's counter at
/// `craft+0x1ac`, so this was chosen at five for the carpet it lays. On
/// 2026-09-15 it was measured on the running original instead
/// (`scripts/psp-count-mines.py`, PPSSPP, a VENOM Single Race with the AI
/// collecting and firing through the game's own grant path): three clusters
/// from three different craft, and `craft+0x1ac` read **5** at the first
/// `Weapon_DropMines` hit of every one, counting down by one per drop to the
/// frame the fire bit cleared. The write watch that ran alongside caught the
/// arming store's PC, and it is `WeaponPickup_ArmMine` (`0x0886759c`, EU
/// `0x088673f8`): `li a0,0x5; sw a0,0x1ac(a1)`, called from
/// `WeaponPickup_Grant` when the `<Pickupodds>` roll lands on the Mine. **The
/// count is armed when the pickup is granted, not when fire is pressed**, and
/// the same function zeroes the reload timer, so the first mine leaves on the
/// press frame. Both pressings carry the same literal. See
/// `docs/ghidra/functions/psp-pulse-usa/mine.md`'s 2026-09-15 section for the
/// trail and the disassembly.
///
/// The shipped `<Weapon type="Mine"><Stats>` still authors no count; this is a
/// code literal in the executable, like [`DROP_INTERVAL`].
///
/// **It is simulation state**, so it is in the determinism reference and
/// changing it is a hash move - which the measurement did not require.
pub const CLUSTER: u8 = 5;

/// Where a craft lays its mines and its bomb: **the craft's own position.**
///
/// **Measured on the running original, 2026-10-01, confidence 90.** A stationary
/// craft's five `Mine_PoseNode` matrices carry the body position to the
/// hundredth (`6.08, -50.07, -196.03` on both), and `Bomb_Init`'s own `a1` - the
/// drop point `Weapon_FireBomb` hands it - equals the rigid body's position to
/// the last bit, both stationary (`6.0764699, -50.0664253, -196.0265045`) and at
/// 106.2 u/s (`124.4720764, -47.9094696, -196.9474182`, the body's position on
/// the fire frame, not advanced by the velocity). `docs/ghidra/functions/
/// psp-pulse-usa/mine.md`'s 2026-10-01 section has the probe.
///
/// This replaced a drop point pushed back by the hull's own extent, which was
/// chosen (so a charge did not start inside the craft) and which the original
/// does not do: its charge starts inside the hull and is left behind by the
/// craft's own motion. What `craft+0xa0`, the anchor `Weapon_RequestFire` stores
/// for these two weapons, feeds is not established; the measured drop point is the
/// body position.
///
/// **Measured on Pulse's PSP build only.** Pure and HD inherit this law, which is
/// chosen for those titles, not measured.
#[must_use]
pub fn drop_point(state: &ShipState) -> Vec3 {
    state.body.position
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

/// The pose a laid charge is drawn with, frozen at the instant it lands.
///
/// **Recovered as a mechanism, chosen as to which basis.** `Mine_Init`
/// (`0x08859ac8`, confidence 90) copies a whole matrix - `entity+0x60` through
/// `+0x9c` - into the entity once, at drop
/// (`docs/ghidra/functions/psp-pulse-usa/mine.md#mine_init-scatters-and-takes-its-fuse-from-timetodie`),
/// and nothing this project has read updates it afterward: a laid mine keeps
/// the pose it landed with, not the world's up, for the rest of its life.
/// Applied here to the Bomb too, on the same footing this module already
/// applies [`at_rest`] to both: `Weapon_FireBomb`'s own spawn helper
/// (`0x0885f188`) could not be resolved statically, so nothing says the Bomb
/// does the same copy, but nothing says it does not either, and "one weapon in
/// two sizes" is this module's standing reading rather than a per-field one.
///
/// **Chosen, not measured, and carries no confidence score**: the matrix
/// `Mine_Init` copies comes from `craft->anchor`, the *rear emitter's own*
/// transform (`craft+0xa0`) - not necessarily the hull's own orientation this
/// takes instead. An emitter's basis is not guaranteed to equal the body's:
/// the Shuriken's trail anchor is known to carry a non-identity local rotation
/// of its own (`docs/ghidra/functions/psp-pulse-usa/shuriken.md`, the
/// constructor rotating it by `-pi/2`). This engine has no located rear-emitter
/// transform to read instead, so the firing craft's whole
/// [`oag_physics::Body::orientation`] is what it uses - the closest available
/// reading, not the recovered one.
///
/// **A second, separate question lives at the draw site, not here, and it is
/// mostly closed by the model's own shape.** `Pulse_Mine.vex` (viewed with
/// `oag-view --mesh`, the same way `MODEL_YAW`'s own measurement of
/// `Ship.vex` was made) is a caltrop - three spikes at roughly 120° around
/// one axis - not a directional hull, so a `MODEL_YAW`-style yaw error about
/// that axis has nothing to be wrong *relative to*: any of the three spikes
/// reads as "the front" equally well. What that measurement does not reach
/// is whether the axis itself - which way the caltrop's own points face
/// versus [`oag_physics::Body::forward`]'s convention - agrees, a coarser
/// and lower-stakes question than the ship's nose/tail one. `Pulse_Bomb.vex`
/// was not separately viewed. See
/// `oag_game::race::weapons::visuals::projectile_model_matrices`'s own doc
/// comment.
#[must_use]
pub const fn frozen_pose(orientation: Quat) -> Quat {
    orientation
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
/// face on the tick it is pressed. This is the same exclusion
/// [`super::nearest_hit`] already makes for a rocket's owner, applied to a
/// weapon where it is doing more work. **For the Bomb the original's own
/// arrangement is now read and it is narrower than this**: `Bomb_UpdateTrigger`
/// (`0x08863d7c`) skips the owner only while the bomb's age is under `0.5 s`
/// (`Bomb_InArmingDelay`, `0x08863440`), after which the layer can trip its
/// own bomb like anyone else. The Mine's own sweep (`Mine_SweepCraftTrigger`) has not
/// been read for the same point, so the permanent exclusion stays this
/// engine's for both, labelled as such - see
/// `docs/ghidra/functions/psp-pulse-usa/mine.md`'s 2026-09-15 Bomb section.
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
    pub fn from_table(weapons: Option<&oag_tables::weapons::WeaponStats>) -> Self {
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
                    effect: None,
                });
            }
        }
    }

    projectile.lifetime -= dt;
    if projectile.lifetime > 0.0 {
        return None;
    }
    // **A mine whose fuse runs out goes off quietly; a bomb's does not.** This
    // comment used to say both ways out of here spend a blast, from the shape
    // of the Mine's chain; `MinePool_Update` (`0x08867370`) read in full on
    // 2026-09-16 says otherwise. Its first pass counts `+0x48` down and, at
    // zero, only raises the destroy bit; `Weapon_PostBlastImpulse` is called
    // from one place, `Mine_SweepCraftTrigger`'s trip branch, and the second
    // pass's teardown is `Mine_SpawnExplosion` and the slot swap - the same
    // explosion-and-nothing a Missile's expiry gets. See
    // `docs/ghidra/functions/psp-pulse-usa/mine.md`, "the fuse and the trip
    // spend differently". The Bomb keeps its blast: `BombPool_Update` calls
    // `Bomb_Detonate` on the fuse, which is its own read on the same page.
    Some(Impact {
        point: here,
        kind,
        owner,
        struck: None,
        blast: kind != Weapon::Mine,
        effect: None,
    })
}
