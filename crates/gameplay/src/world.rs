//! The whole of gameplay state, in one struct of plain data.
//!
//! Fixed-size arrays and no allocation, so a snapshot is one `memcpy`-shaped
//! operation and a replay or a golden test needs no traversal. See
//! `docs/architecture/adr/0003-no-ecs.md` for why this is not an ECS.

use oag_core::Rng;
use oag_physics::{Handling, ShipState};
use oag_race::RaceState;

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
    /// Shield energy remaining.
    ///
    /// **Nothing depletes this yet, deliberately.** Collision damage and the
    /// weapons that would drain it are M5 work, so today the only thing that
    /// writes it is Zone mode's perfect-zone recharge - which clamps to the
    /// ship's maximum, so a full pool recharges to full and the HUD bar reads
    /// full for the whole race. It is here rather than waiting because the
    /// recharge itself is recovered (`docs/gameplay/race-modes.md`) and dropping
    /// it would mean re-deriving it later.
    ///
    /// Zero on a default ship. [`Ship::place_at`] does not reset it: the pool is
    /// a race-scale quantity, not part of a spawn pose.
    pub shield: f32,
    /// Whether this slot holds a ship at all.
    pub active: bool,
}

impl Default for Ship {
    fn default() -> Self {
        Self {
            physics: ShipState::default(),
            handling: Handling::ZERO,
            segment: 0,
            shield: 0.0,
            active: false,
        }
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
        assert_eq!(Ship::default().shield, 0.0);
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
