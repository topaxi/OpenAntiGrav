//! Eliminator's per-lap refill: a fifth of the maximum, clamped, with the
//! absorb feedback - `Ship_RefillLapShield` (`0x0883de30`), see
//! `docs/ghidra/functions/psp-pulse-usa/shield.md`.

use super::*;
use crate::audio::sfx::Cue;
use crate::race::eliminator::LAP_REFILL_FRACTION;

fn eliminator() -> Race {
    race_with_weapon_table(Mode::Eliminator, enveloping_pad(), 1.0, one_mine_table())
}

#[test]
fn a_completed_lap_gives_back_a_fifth_of_the_maximum_and_plays_absorb() {
    let mut race = eliminator();
    let maximum = race.sim.world.ships[0].handling.dimensions.shield;
    race.sim.world.ships[0].physics.shield = maximum * 0.3;
    race.drain_cues();

    race.eliminator_lap_health_refill(0);

    assert_eq!(
        race.sim.world.ships[0].physics.shield,
        maximum * 0.3 + maximum * LAP_REFILL_FRACTION
    );
    assert!(
        race.pending_cues().iter().any(|cue| cue.cue == Cue::Absorb),
        "the refill must play the absorb feedback"
    );
}

#[test]
fn the_refill_is_clamped_at_the_maximum() {
    let mut race = eliminator();
    let maximum = race.sim.world.ships[0].handling.dimensions.shield;
    race.sim.world.ships[0].physics.shield = maximum * 0.9;
    race.eliminator_lap_health_refill(0);
    assert_eq!(race.sim.world.ships[0].physics.shield, maximum);
}

#[test]
fn a_craft_that_is_not_racing_and_a_mode_that_is_not_eliminator_get_nothing() {
    let mut race = eliminator();
    let maximum = race.sim.world.ships[0].handling.dimensions.shield;
    race.sim.world.ships[0].physics.shield = maximum * 0.3;
    race.sim.world.ships[0].physics.craft_state = oag_physics::damage::CraftState::Destroyed;
    race.eliminator_lap_health_refill(0);
    assert_eq!(race.sim.world.ships[0].physics.shield, maximum * 0.3);

    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_mine_table());
    race.sim.world.ships[0].physics.shield = maximum * 0.3;
    race.eliminator_lap_health_refill(0);
    assert_eq!(race.sim.world.ships[0].physics.shield, maximum * 0.3);
}
