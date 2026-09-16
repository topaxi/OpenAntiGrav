//! Eliminator's per-lap refill: a fifth of the maximum, clamped, with the
//! absorb feedback - `Ship_RefillLapShield` (`0x0883de30`), see
//! `docs/ghidra/functions/psp-pulse-usa/shield.md`.

use super::*;
use crate::audio::sfx::Cue;
use crate::race::eliminator::LAP_REFILL_FRACTION;
use oag_gameplay::PlayerInputs;

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

/// A wrecked opponent in a single race comes back: state 5's dwell, state
/// 6's wait, then `Ship_ResetShield` and a place on the line
/// (`Ship_UpdateDestroyed`, `Ship_UpdateRespawn` - shield.md). The player's
/// own destruction still ends the race, so it is the opponent that is put
/// down here.
#[test]
fn a_destroyed_opponent_in_a_single_race_returns_after_the_two_dwells() {
    use crate::race::eliminator::{AI_RESPAWN_WAIT, DESTROYED_DWELL};
    use oag_physics::CraftState;

    let mut race = race_with_a_grid();
    assert!(
        race.sim.world.ships[1].active,
        "the grid fixture fields opponents"
    );
    let dimensions = race.sim.world.ships[1].handling.dimensions;
    // Straight to the out-of-the-race state, the explosion already run.
    race.sim.world.ships[1].physics.craft_state = CraftState::Eliminated;
    race.sim.world.ships[1].physics.shield = 0.0;

    let dwell_ticks = ((DESTROYED_DWELL + AI_RESPAWN_WAIT) / race.dt()).round() as usize;
    for _ in 0..dwell_ticks - 2 {
        race.tick(&PlayerInputs::none());
        assert_eq!(
            race.sim.world.ships[1].physics.craft_state,
            CraftState::Eliminated,
            "the opponent came back before its dwell had run"
        );
    }
    for _ in 0..4 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(
        race.sim.world.ships[1].physics.craft_state,
        CraftState::Racing
    );
    assert_eq!(race.sim.world.ships[1].physics.shield, dimensions.shield);
    assert!(
        !race.finished(),
        "an opponent's death must not end the player's race"
    );
}

/// And the player's own destruction is still the race's end, not a respawn.
#[test]
fn the_players_destruction_in_a_single_race_still_ends_it() {
    use oag_physics::CraftState;

    let mut race = race_with_a_grid();
    race.sim.world.ships[0].physics.craft_state = CraftState::Eliminated;
    for _ in 0..300 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(
        race.sim.world.ships[0].physics.craft_state,
        CraftState::Eliminated
    );
    assert!(race.finished());
}
