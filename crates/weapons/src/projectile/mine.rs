//! The two weapons that come out of the back: the Mine, a cluster laid one every
//! tenth of a second, and the Bomb, a single bigger one.
//!
//! One module because they are one weapon in two sizes: the Bomb authors six of
//! the Mine's seven attributes, all larger on both shipped tables, a maintainer
//! who plays Pulse calls it "a single big mine", and `Weapon_FireBomb`
//! (`0x08863a20`) differs from `Weapon_DropMines` (`0x088675cc`) only in spawning
//! once where the other reloads a timer and spawns again. "Mine" below means both
//! unless it says otherwise; they differ in [`CLUSTER`] and [`Drop::count`]. The
//! Mine is the opposite of [`super::missile`]: it does not fly at all.
//!
//! # What is recovered and what is ours
//!
//! Evidence: `docs/ghidra/functions/psp-pulse-usa/mine.md`.
//!
//! **Recovered.**
//!
//! - A press lays a staggered cluster: `Weapon_DropMines` (`0x088675cc`) runs every
//!   tick while its bit is set, spawns one mine each time a `0.1 s` reload
//!   expires and decrements a per-craft counter. See [`DROP_INTERVAL`].
//! - The craft keeps the pickup until the last is out: the handler clears
//!   `craft+0x1bc` and its fire bit at zero, hence [`crate::pickup::Held`] carries
//!   the drop.
//! - They come from `craft+0xa0`, the rear emitter anchor, used only by this and
//!   the Bomb.
//! - The fuse: `Mine_Init` (`0x08859ac8`) loads `stats+0xf4`, which is
//!   `<Weapon type="Mine"><Stats timetodie>`. Confidence 92 on the offset.
//! - `trigger_radius` is a distinct, smaller distance than `blastradius`
//!   (authored, true in both shipped tables).
//! - [`CLUSTER`] is five, measured on the running original 2026-09-15 and read
//!   off `WeaponPickup_ArmMine` (`0x0886759c`). Confidence 92.
//!
//! **Ours.**
//!
//! - That a mine does not move. See [`at_rest`].
//! - That entering `trigger_radius` sets it off: the attribute is the disc's, the
//!   code that spends it was not found. See [`triggered_by`].
//! - The owner's exemption is a window, not permanent: [`OWNER_EXEMPT_SECONDS`],
//!   recovered for both weapons. See [`triggered_by`].

use super::{Impact, Projectile};
use crate::Craft;
use oag_core::math::{Quat, Vec3};
use oag_physics::ShipState;
use oag_tables::weapons::{BombStats, MineStats, Weapon};

/// What one press of a rear weapon lays: how many, and with what fuse and trip.
///
/// The seam between the two weapons: a count and three floats; the rest is one
/// code path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drop {
    /// How many charges the press lays: [`CLUSTER`] for the Mine, **one** for the
    /// Bomb. Both recovered: the Mine's five is armed by `WeaponPickup_ArmMine`
    /// and measured live; the Bomb has no counter (`WeaponPickup_Grant` writes its
    /// id inline and `Weapon_FireBomb` (`0x08863a20`) makes one spawn call).
    pub count: u8,
    /// Seconds before a laid charge goes off on its own, or [`NO_FUSE`].
    ///
    /// Pure's Bomb never does, read rather than defaulted: its `<Stats>` authors
    /// no `timetodie`, and `BombPool_Update` (`0x0884e968` on `psp-pure-usa`)
    /// compares its age to nothing; see `oag_tables::weapons::BombStats::timetodie`.
    /// A `None` becomes [`NO_FUSE`] so the countdown in [`advance_laid`] never
    /// reaches zero.
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

    /// The Bomb's: the same with a count of one.
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

    /// The drop one weapon makes, or `None` for a weapon that lays nothing or a
    /// table with no block for it.
    ///
    /// **This and [`TriggerRadii::get`] are the same list twice** and neither fails
    /// loudly alone: a third rear weapon missing an arm here lays nothing, one
    /// missing there can never be tripped.
    /// `every_rear_weapon_can_be_laid_and_tripped` in `pickup::tests` fails them
    /// together.
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
/// `advance_laid` counts `dt` off each tick and detonates at or below zero, so
/// only a craft in `trigger_radius` ends such a charge. Chosen over an `Option`
/// on [`super::Projectile::lifetime`] because that field is hashed every tick and
/// an infinity is one bit pattern where a widened field would move every
/// reference. Only Pure's Bomb lays one; see [`Drop::fuse`].
pub const NO_FUSE: f32 = f32::INFINITY;

/// How long between one mine leaving and the next, in seconds.
///
/// **Recovered, confidence 92, a code literal.** `Weapon_DropMines`
/// (`0x088675cc`) reloads `craft+0x1b0` with `0x3dcccccd` after every spawn, and
/// the running original drops at six-frame intervals (seven when `dt` jitter
/// left the timer at `0.0002`), measured 2026-09-15. The Mine's `<Stats>` authors
/// no `rate` (only the Cannon does), which is why the handler was long
/// misattributed to the Cannon.
pub const DROP_INTERVAL: f32 = 0.1;

/// How many mines one press lays.
///
/// **Recovered, confidence 92.** Two static sweeps had failed to find the writer
/// of `craft+0x1ac`, so five was chosen; on 2026-09-15 the running original
/// settled it (`scripts/psp-count-mines.py`, PPSSPP, a VENOM Single Race): three
/// clusters from three craft read **5** at the first `Weapon_DropMines` hit,
/// counting down one per drop. The write watch caught the arming store in
/// `WeaponPickup_ArmMine` (`0x0886759c`, EU `0x088673f8`):
/// `li a0,0x5; sw a0,0x1ac(a1)`, called from `WeaponPickup_Grant`. The count is
/// armed at grant, not at fire, and the same function zeroes the reload timer, so
/// the first mine leaves on the press frame. See
/// `docs/ghidra/functions/psp-pulse-usa/mine.md`, 2026-09-15 section. The shipped
/// `<Stats>` authors no count; it is a code literal like [`DROP_INTERVAL`].
///
/// Simulation state: in the determinism reference, so changing it is a hash move.
pub const CLUSTER: u8 = 5;

/// Where a craft lays its mines and its bomb: **the craft's own position.**
///
/// **Measured on the running original, 2026-10-01, confidence 90.** A stationary
/// craft's five `Mine_PoseNode` matrices carry the body position to the
/// hundredth (`6.08, -50.07, -196.03`), and `Bomb_Init`'s `a1` (the drop point
/// `Weapon_FireBomb` hands it) equals the body position to the last bit,
/// stationary (`6.0764699, -50.0664253, -196.0265045`) and at 106.2 u/s
/// (`124.4720764, -47.9094696, -196.9474182`, the fire-frame position, not
/// advanced by velocity). See `docs/ghidra/functions/psp-pulse-usa/mine.md`,
/// 2026-10-01 section.
///
/// This replaced a drop point pushed back by the hull's extent (chosen); the
/// original's charge starts inside the hull and is left behind. What `craft+0xa0`
/// feeds is not established.
///
/// **Measured on Pulse's PSP build only.** Pure and HD inherit this law, chosen
/// for those titles.
#[must_use]
pub fn drop_point(state: &ShipState) -> Vec3 {
    state.body.position
}

/// The velocity a mine is laid with.
///
/// **Zero, a reading.** `Mine_Init` (`0x08859ac8`) copies the craft's velocity to
/// `+0xa0`, but nothing read integrates it: the one function walking the live
/// pool counts a timer down and sweeps for a blast, never touching a position.
/// So `+0xa0` is read as a heading the model is drawn along. The cluster still
/// spreads, as the craft moves between drops.
///
/// **The Bomb reaches the same answer by another route.** The above is a
/// negative result about the Mine's subsystem only. `Weapon_FireBomb`
/// (`0x08863a20`) stages the craft's forward row negated and hands it to a spawn
/// helper at `0x0885f188` that could not be resolved statically (two callers
/// jump past its prologue): direction recovered, speed unfound. A maintainer who
/// plays Pulse says "the bomb should be static, like the mines"; a negated unit
/// row would be one unit a second anyway.
///
/// A function so the reasoning has a home and a later read that finds the
/// integration has one place to change.
#[must_use]
pub const fn at_rest() -> Vec3 {
    Vec3::ZERO
}

/// The pose a laid charge is drawn with, frozen at the instant it lands.
///
/// **Recovered as a mechanism, chosen as to which basis.** `Mine_Init`
/// (`0x08859ac8`, confidence 90) copies a matrix (`entity+0x60..+0x9c`) once at
/// drop (`docs/ghidra/functions/psp-pulse-usa/mine.md#mine_init-scatters-and-takes-its-fuse-from-timetodie`)
/// and nothing read updates it. Applied to the Bomb too: `Weapon_FireBomb`'s
/// spawn helper (`0x0885f188`) is unresolved, so nothing says it differs, and
/// "one weapon in two sizes" is the standing reading.
///
/// **Chosen, not measured, no confidence score**: the copied matrix is
/// `craft->anchor`, the rear emitter's own transform (`craft+0xa0`), not
/// necessarily the hull's orientation used here (the Shuriken's trail anchor
/// carries a non-identity local rotation, `docs/ghidra/functions/psp-pulse-usa/shuriken.md`).
/// No rear-emitter transform is located, so [`oag_physics::Body::orientation`]
/// is the closest available reading.
///
/// At the draw site, `Pulse_Mine.vex` (viewed with `oag-view --mesh`) is a
/// caltrop of three spikes at about 120° around one axis, so a `MODEL_YAW`-style
/// error about it has nothing to be wrong relative to; whether the axis agrees
/// with [`oag_physics::Body::forward`] is open, lower-stakes. `Pulse_Bomb.vex`
/// was not viewed. See `oag_raceplay::weapons::visuals::projectile_model_matrices`.
#[must_use]
pub const fn frozen_pose(orientation: Quat) -> Quat {
    orientation
}

/// How long after laying the owner is exempt from its own Mine or Bomb, seconds.
///
/// **Recovered, confidence 85 on Pulse, a code literal**, the same `0x3f000000`
/// (`0.5`) read twice. `Bomb_UpdateTrigger` (`0x08863d7c`) skips the owner
/// (`bomb+0x48 == i`) while `Bomb_InArmingDelay` (`0x08863440`) says `age < 0.5`;
/// `Mine_SweepCraftTrigger` (`0x08867b50`) does the same through `FUN_08859f04`
/// (`mine+0xd4 < 0x08ab0efc`, whose word is `0.5`), and `Mine_Update` is what
/// adds `dt` to `+0xd4`. After the window the layer trips its own charge like
/// any craft.
///
/// HD: its trip was filmed on RPCS3 and the owner's own bomb goes off about
/// `0.6 s` after laying in bomb age (`0.5 s` plus a frame or two), so the same
/// constant is used. The trigger itself is not in `NormalBomb_Update`
/// (`0x001443f8`); see `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`. Applied to
/// Pure and the other titles on the "unmeasured titles inherit Pulse" rule.
pub const OWNER_EXEMPT_SECONDS: f32 = 0.5;

/// Whether a craft at `position` is close enough to set this mine off.
///
/// **Ours, with the disc's number in it.** `trigger_radius` is authored; the
/// original tests a cube and then a sphere of it (`Mine_SweepCraftTrigger`,
/// `Bomb_UpdateTrigger`), which for a sphere test is the sphere alone.
///
/// **The laying craft is exempt only while the charge is younger than
/// [`OWNER_EXEMPT_SECONDS`]**: a cluster leaves from the tail of a craft standing
/// right there. After that it trips its own charge like any craft.
///
/// Takes the radius, as the two weapons' come from different structs. This is the
/// trip only: the following blast excludes nobody.
#[must_use]
pub fn triggered_by(
    mine: Vec3,
    owner: u8,
    age: f32,
    slot: u8,
    position: Vec3,
    trigger_radius: f32,
) -> bool {
    (slot != owner || age >= OWNER_EXEMPT_SECONDS) && (position - mine).length() <= trigger_radius
}

/// The trip radius of each weapon that has one, so [`super::Projectiles::advance`]
/// takes both without a whole weapon table: `advance` is the deterministic core
/// and the table a loaded asset. `None` is a weapon the table does not author,
/// which cannot be tripped; see [`advance_laid`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TriggerRadii {
    /// `<Weapon type="Mine"><Stats trigger_radius>`.
    pub mine: Option<f32>,
    /// `<Weapon type="Bomb"><Stats trigger_radius>`.
    pub bomb: Option<f32>,
}

impl TriggerRadii {
    /// Both, out of a weapon table, or both `None` if it did not load.
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

    /// One weapon's, or `None` for a weapon that is not laid. The twin of
    /// [`Drop::for_weapon`], guarded by a test rather than the type system.
    #[must_use]
    pub fn get(self, kind: Weapon) -> Option<f32> {
        match kind {
            Weapon::Mine => self.mine,
            Weapon::Bomb => self.bomb,
            _ => None,
        }
    }
}

/// One tick of a laid charge: the trip test, then the fuse.
///
/// Returns the blast if it goes off this tick. The trip is tested first, so a
/// charge tripped on the tick its fuse expires is credited to the tripper. A
/// weapon the table does not author cannot be tripped and expires without a
/// blast (reachable only through a bug upstream in
/// [`crate::pickup::IMPLEMENTED`]).
pub(super) fn advance_laid<S: Craft>(
    projectile: &mut Projectile,
    kind: Weapon,
    dt: f32,
    ships: &[S],
    trigger_radii: TriggerRadii,
) -> Option<Impact> {
    let here = projectile.position;
    let owner = projectile.owner;
    // Age first for a Mine (`Mine_Update` runs before its sweep), after the trip
    // for a Bomb (`BombPool_Update` triggers, then `Bomb_AdvanceFuse` ages).
    if kind == Weapon::Mine {
        projectile.age += dt;
    }
    let age = projectile.age;

    if let Some(trigger_radius) = trigger_radii.get(kind) {
        // Slot order only: the first close craft trips it, so which is credited
        // when two are inside the radius is the array's business, not a
        // nearest-first search over often-equal distances. See
        // `docs/architecture/determinism.md`.
        for (slot, ship) in ships.iter().enumerate() {
            if !ship.active() {
                continue;
            }
            let slot = slot as u8;
            if triggered_by(
                here,
                owner,
                age,
                slot,
                ship.physics().body.position,
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

    if kind == Weapon::Bomb {
        projectile.age += dt;
    }
    projectile.lifetime -= dt;
    if projectile.lifetime > 0.0 {
        return None;
    }
    // A mine whose fuse runs out goes off quietly; a bomb's does not.
    // `MinePool_Update` (`0x08867370`), read in full 2026-09-16: the first pass
    // counts `+0x48` down and at zero only raises the destroy bit;
    // `Weapon_PostBlastImpulse` has one caller, `Mine_SweepCraftTrigger`'s trip
    // branch; the second pass's teardown is `Mine_SpawnExplosion` and the slot
    // swap. See `docs/ghidra/functions/psp-pulse-usa/mine.md`, "the fuse and the
    // trip spend differently". `BombPool_Update` calls `Bomb_Detonate` on the fuse.
    Some(Impact {
        point: here,
        kind,
        owner,
        struck: None,
        blast: kind != Weapon::Mine,
        effect: None,
    })
}
