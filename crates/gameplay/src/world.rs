//! The whole of gameplay state, in one struct of plain data.
//!
//! Fixed-size arrays and no allocation, so a snapshot is one `memcpy`-shaped
//! operation and a replay or a golden test needs no traversal. See
//! `docs/architecture/adr/0003-no-ecs.md` for why this is not an ECS.

use oag_core::Rng;
use oag_physics::{DamageRules, Handling, ShipState};
use oag_race::{Mode, RaceState};

/// The most ships a race can hold.
///
/// Eight, which is what Pulse grids. A hard array bound rather than a `Vec`
/// capacity: the limit is real, and making it visible is what keeps the world
/// snapshot a fixed size.
pub const MAX_SHIPS: usize = 8;

/// One racer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ship {
    /// Body and suspension state.
    pub physics: ShipState,
    /// The tunables for this ship in the race's speed class, read from the
    /// player's own disc.
    pub handling: Handling,
    /// Which spline segment the ship was last nearest to.
    ///
    /// Carried between ticks so the search for the nearest segment is local
    /// rather than over the whole track. Lap counting is **not** built on this:
    /// where the original counts laps is an open question, and `gate` has no
    /// runtime class at all. See `docs/formats/track.md`.
    ///
    /// **Not the same index as [`Self::driver`]**, which is a position in the
    /// racing line's own point table and is what an opponent actually steers by.
    /// This one is a per-path control-point segment and is still read by nothing.
    pub segment: u16,
    /// What this craft is carrying, if anything.
    ///
    /// Filled by crossing a `Weapon Pad` and emptied by firing or absorbing;
    /// see [`crate::pickup`], which carries the recovered-versus-ours split for
    /// the whole system. Always empty in a mode that races with weapons off,
    /// because such a race has no armed pads at all - the original does not
    /// merely ignore a crossing, it empties the trigger list.
    ///
    /// **Here rather than on `ShipState`**, because it is an
    /// `oag_tables::weapons::Weapon` and `oag-physics` depends on nothing but
    /// `oag-core`. The consequence is that it is *not* covered by
    /// `oag_physics::probe::hash_state`'s compile-time enforcement - what a held
    /// pickup does to the simulation reaches the hash through
    /// `ShipState::turbo_timer` instead, and the inventory itself changes no
    /// force.
    pub pickup: crate::pickup::Held,
    /// Where this craft's driver last found itself on the racing line.
    ///
    /// Only an opponent uses it - slot 0 is flown by the player - and it is the
    /// seed for the next windowed search rather than a lap-counting index.
    ///
    /// **Here rather than beside the world**, because a driver whose state lived
    /// in the composition root would be invisible to a snapshot, and two replays
    /// of one race would not be the same race. It is one `u32` and stays `Copy`,
    /// so the array is still a `memcpy`. The line it indexes into is *track*
    /// data and is not in the world at all; see [`oag_ai::Line`].
    pub driver: oag_ai::Driver,
    /// Where this craft is in the race: its lap, its place on the circuit and
    /// whether it has finished.
    ///
    /// **Every craft, the player included.** [`World::race`] is the *player's*
    /// race - its clock, its Zone counters, its best lap - and stays that; this
    /// is the smaller thing the whole field needs so that eight of them can be
    /// ordered against each other. Slot 0's `lap` is assigned from here rather
    /// than counted twice, so there is one lap rule and not two.
    ///
    /// [`World::race`]: World::race
    pub standing: oag_race::Standing,
    /// Seconds left on an Autopilot pickup, or zero.
    ///
    /// `Autopilot_Fire` (`0x088613bc`) writes
    /// `<Weapon type="Autopilot"><Stats time>` into `craft+0x148` and sets a
    /// running bit; `Autopilot_Update` (`0x08861404`) counts it down and clears
    /// the bit at zero. See
    /// `docs/ghidra/functions/psp-pulse-usa/autopilot.md`.
    ///
    /// **Here rather than on [`ShipState`]**, for the reason [`Self::pickup`]
    /// gives about itself: `oag-physics` depends on nothing but `oag-core`, and
    /// no force reads this. What it changes is *which controls the craft is
    /// handed*, which the composition root decides - so it belongs beside the
    /// pickup that arms it and the driver that flies it, not beside the body.
    ///
    /// Hashed all the same: a craft being driven for is a different race from
    /// one being driven, and a replay that lost this would diverge.
    pub autopilot_timer: f32,
    /// Seconds of weapon slowdown owed to this craft but not yet applied.
    ///
    /// The original's `entity+0x130`. A weapon impact adds its
    /// `<Stats slowdown_time>` here - nine writers, all in the weapon
    /// subsystems - and exactly one consumer drains it, once a tick, into
    /// [`oag_physics::ShipState::slowdown_timer`]. That is
    /// [`crate::slowdown::drain`], and it carries the shield gate: a shielded
    /// craft takes no slowdown, **and this slot is cleared anyway** rather than
    /// banked, so a hit landed one tick before a shield expires is simply lost.
    ///
    /// **Here rather than on [`oag_physics::ShipState`]**, for the reason
    /// [`Self::pickup`] and [`Self::autopilot_timer`] both give about
    /// themselves: what fills it is a figure out of `<WeaponStats>`, and
    /// `oag-physics` depends on `oag-core` and nothing else. It is also where
    /// the original puts it - on the *entity*, not on the craft, which is the
    /// one field of this mechanic that is not a `craft+` offset.
    ///
    /// Hashed: a craft owing slowdown is about to be a craft that has lost its
    /// engine, and a replay that dropped this would diverge one tick later.
    ///
    /// See `oag_physics::slowdown` for the whole law and
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md` for the evidence.
    pub pending_slowdown: f32,
    /// Whether this slot holds a ship at all.
    pub active: bool,
}

impl Default for Ship {
    fn default() -> Self {
        Self {
            physics: ShipState::default(),
            handling: Handling::ZERO,
            segment: 0,
            pickup: crate::pickup::Held::empty(),
            driver: oag_ai::Driver::default(),
            standing: oag_race::Standing::default(),
            autopilot_timer: 0.0,
            pending_slowdown: 0.0,
            active: false,
        }
    }
}

/// The `Weapons` and `Damage` race options a mode runs with.
///
/// **Recovered, not chosen.** `Race_ReadSetupOptions` (`0x08896b84`) takes a
/// per-mode default before consulting the setup strings: `g_weapons_enabled` is
/// cleared for game modes `{5, 9, 10, 0xf, 0x11}` and `g_damage_enabled` for
/// `{5, 7, 10}`. Modes **5 and 10** are in both sets *and* share one constructor
/// in `Race_CreateModeObject` (`0x0882112c`), which is exactly the relationship
/// this project's time trial and speed lap have - speed lap is the time trial
/// without the lap target.
///
/// The disc's own manual text closes it independently: `MAN_P3_PG1_ENER` reads
/// *"The Time Trial and Speed Lap events will recover your ship energy
/// automatically"*, and recovering energy automatically is precisely what
/// clearing `g_damage_enabled` does - `oag_physics::damage::regenerate`, 4 a
/// second with a floor of 20. A shipped string table and a switch statement
/// agreeing is what puts this at confidence **85** rather than the 84 a
/// decompile alone caps at.
///
/// Zone is game mode 6 and is in neither set, so its *damage* default stands
/// and its shield depletes - which is what its unimplemented "destroyed" ending
/// depends on. See `docs/ghidra/functions/psp-pulse-usa/shield.md`. Its
/// **weapons** default does not stand: mode 6 defaults `g_weapons_enabled` to
/// `1` and the front end then always supplies an explicit
/// `<Weapons>Off</Weapons>` for it, which is why selecting ZONE greys the
/// `WEAPONS` row to `OFF` on the running original (confirmed live 2026-08-10,
/// PPSSPP v1.20.4). That is the measurement [`Mode::weapons_enabled`] carries.
///
/// A single race is in neither set either, and unlike Zone the front end does
/// not override it: `MSC_EVENT_SR` calls weapons "optional" and the Custom Race
/// screen leaves both rows selectable, so both defaults stand. That makes it the
/// only mode here whose contact damage is *not* halved - see
/// [`oag_physics::damage::NO_WEAPONS_DAMAGE_SCALE`], which a time trial does get.
///
/// **The weapons flag is read from [`Mode::weapons_enabled`] rather than
/// restated here.** Restating it is how the two disagreed about Zone until the
/// 2026-08-18 review (finding D2): this function said weapons on, `mode.rs`
/// said off with the stronger evidence, and because the flag halves *all*
/// damage rather than only weapon damage, Zone was taking roughly twice the
/// original's contact damage. Survival time is the whole of Zone's score, so
/// that is a scoreboard divergence and not a detail. One source of truth
/// cannot drift from itself.
///
/// **Here rather than on [`oag_race::Mode`]**, because `oag-race` deliberately
/// does not depend on `oag-physics` (see that crate's `Cargo.toml`) and this
/// returns a physics type. This crate is the bridge between the two, which is
/// what it is for.
#[must_use]
pub fn damage_rules(mode: Mode) -> DamageRules {
    DamageRules {
        weapons: mode.weapons_enabled(),
        // `g_damage_enabled` is cleared for modes `{5, 7, 10}`: time trial and
        // speed lap, of the four this engine runs.
        damage: !matches!(mode, Mode::TimeTrial | Mode::SpeedLap),
    }
}

/// Everything the simulation carries from one tick to the next.
#[derive(Debug, Clone)]
pub struct World {
    /// Ticks elapsed since the race began.
    pub tick: u64,
    /// The seeded generator. Every random draw in the simulation comes from
    /// here; nothing reads OS entropy. See `oag_core::rng`.
    pub rng: Rng,
    /// The grid. Slots past [`Self::ship_count`] are inactive, not absent, so
    /// the array's size never depends on the race.
    pub ships: [Ship; MAX_SHIPS],
    /// How many of [`Self::ships`] are in play.
    pub ship_count: u8,
    /// Lap, timing and mode state for the player's ship.
    ///
    /// [ADR-0003] sketched this field when the world was first laid out, and
    /// this is it. Single-ship for now: the modes that need it - time trial,
    /// speed lap, Zone - have one ship on the track, and per-opponent timing
    /// arrives with the grid rather than before it.
    ///
    /// [ADR-0003]: ../../../docs/architecture/adr/0003-no-ecs.md
    pub race: RaceState,
    /// Everything a weapon has put in the air.
    ///
    /// **The field that grows the snapshot**, and it grows it by a constant:
    /// [`crate::projectile::MAX_PROJECTILES`] slots of plain `Copy` data, sized
    /// the same way `[Ship; MAX_SHIPS]` is and for the same reason. A race must
    /// not change the size of a snapshot, so the array holds free slots rather
    /// than shrinking - see [ADR-0003].
    ///
    /// Unlike [`Ship::pickup`] this **is** simulation state that moves every
    /// tick, so it is covered by [`crate::hash::hash_world`] rather than left to
    /// reach the determinism gate indirectly.
    pub projectiles: crate::projectile::Projectiles,
    /// The single travelling Quake wave, or `None` when none is in flight.
    ///
    /// Not part of [`Self::projectiles`] - see `crate::projectile::quake`'s
    /// module doc comment for why: the original's own pool is a single slot,
    /// not an array, and it carries a travelling *distance* rather than a
    /// position and a velocity. Simulation state all the same, so it is
    /// covered by [`crate::hash::hash_world`] exactly as [`Self::projectiles`]
    /// is.
    pub quake: Option<crate::projectile::quake::Wave>,
    /// The single LeachBeam link, or `None` when none is in flight.
    ///
    /// Beside [`Self::quake`] and for the same reason - see
    /// `crate::projectile::leach_beam`'s module doc comment: the original's own
    /// pool cursor allows exactly one beam **in the whole race** at a time, a
    /// stricter gate than any other weapon has, and a beam carries a link
    /// between two craft rather than a position and a velocity. Simulation
    /// state, so [`crate::hash::hash_world`] covers it.
    pub leach_beam: Option<crate::projectile::leach_beam::Beam>,
}

impl World {
    /// An empty world with the given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            tick: 0,
            rng: Rng::new(seed),
            ships: [Ship::default(); MAX_SHIPS],
            ship_count: 0,
            race: RaceState::default(),
            projectiles: crate::projectile::Projectiles::new(),
            quake: None,
            leach_beam: None,
        }
    }

    /// The active ships, in slot order.
    ///
    /// Slot order, not race position: iteration order feeds simulation state, so
    /// it has to be something that cannot vary between runs.
    pub fn active_ships(&self) -> impl Iterator<Item = &Ship> {
        self.ships.iter().take(self.ship_count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The recovered table, pinned per mode rather than per rule, because the
    /// two flags come from two different sets in `Race_ReadSetupOptions` and a
    /// test that only checked "weapons implies damage" would pass on a table
    /// that had them the wrong way round.
    #[test]
    fn each_mode_races_with_the_flags_the_original_gives_it() {
        for (mode, weapons, damage) in [
            (Mode::TimeTrial, false, false),
            (Mode::SpeedLap, false, false),
            (Mode::Zone, false, true),
            (Mode::SingleRace, true, true),
        ] {
            let rules = damage_rules(mode);
            assert_eq!(rules.weapons, weapons, "{mode:?} weapons");
            assert_eq!(rules.damage, damage, "{mode:?} damage");
        }
    }

    /// Finding D2's own guard: the weapons flag here is the one `oag-race`
    /// measured, not a second opinion about it. They contradicted each other
    /// about Zone for as long as both were written out by hand.
    #[test]
    fn the_weapons_flag_is_the_one_oag_race_measured() {
        for mode in Mode::ALL {
            assert_eq!(
                damage_rules(mode).weapons,
                mode.weapons_enabled(),
                "{mode:?} disagrees with Mode::weapons_enabled"
            );
        }
    }

    #[test]
    fn a_new_world_has_no_ships() {
        let world = World::new(1);
        assert_eq!(world.tick, 0);
        assert_eq!(world.ship_count, 0);
        assert_eq!(world.active_ships().count(), 0);
        assert!(world.ships.iter().all(|ship| !ship.active));
    }

    #[test]
    fn a_new_world_is_on_lap_one_of_a_time_trial() {
        let world = World::new(1);
        assert_eq!(world.race.mode, oag_race::Mode::TimeTrial);
        assert_eq!(world.race.lap, 1);
        assert!(!world.race.finished);
        assert_eq!(world.race.progress, None);
    }

    /// The pool starts empty because nothing fills it yet. When collision damage
    /// lands this becomes the ship's maximum and this test changes with it - it
    /// is here so that change is deliberate rather than incidental.
    #[test]
    fn a_default_ship_carries_no_shield_yet() {
        assert_eq!(Ship::default().physics.shield, 0.0);
    }

    /// The array is the whole point: a race must not change the size of a
    /// snapshot, or a replay's frames stop being comparable by memory layout.
    #[test]
    fn the_grid_is_always_eight_slots_wide() {
        let mut world = World::new(1);
        world.ship_count = 3;
        assert_eq!(world.ships.len(), MAX_SHIPS);
        assert_eq!(world.active_ships().count(), 3);
    }
}
