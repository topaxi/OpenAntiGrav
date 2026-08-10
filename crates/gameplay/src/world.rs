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
    pub segment: u16,
    /// Whether this slot holds a ship at all.
    pub active: bool,
}

impl Default for Ship {
    fn default() -> Self {
        Self {
            physics: ShipState::default(),
            handling: Handling::ZERO,
            segment: 0,
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
/// Zone is game mode 6 and is in neither set, so it races with both on - which
/// is what makes its shield deplete, and what its unimplemented "destroyed"
/// ending depends on. See `docs/ghidra/functions/psp-pulse-usa/shield.md`.
///
/// **Here rather than on [`oag_race::Mode`]**, because `oag-race` deliberately
/// does not depend on `oag-physics` (see that crate's `Cargo.toml`) and this
/// returns a physics type. This crate is the bridge between the two, which is
/// what it is for.
#[must_use]
pub fn damage_rules(mode: Mode) -> DamageRules {
    match mode {
        Mode::TimeTrial | Mode::SpeedLap => DamageRules {
            weapons: false,
            damage: false,
        },
        Mode::Zone => DamageRules {
            weapons: true,
            damage: true,
        },
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
