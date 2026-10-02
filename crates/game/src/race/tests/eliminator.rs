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

/// A wrecked opponent in a single race stays down: state 5's dwell, then
/// state 6, a bare timer nothing revives (measured live on PPSSPP, 2026-10-02,
/// 13 s past the timer's expiry - the module doc of `race::eliminator`). The
/// opponent keeps its wreck, its empty pool and its place off the circuit for
/// as long as the race runs, and the player's race goes on.
#[test]
fn a_destroyed_opponent_in_a_single_race_never_comes_back() {
    use crate::race::eliminator::DESTROYED_DWELL;
    use oag_physics::CraftState;

    let mut race = race_with_a_grid();
    assert!(
        race.sim.world.ships[1].active,
        "the grid fixture fields opponents"
    );
    // Straight to the out-of-the-race state, the explosion already run.
    race.sim.world.ships[1].physics.craft_state = CraftState::Eliminated;
    race.sim.world.ships[1].physics.shield = 0.0;

    // Past state 5's 1.5 s, state 6's 0.8 s, the Eliminator's longest return
    // (2.5 s) and then the 13 s the live capture watched.
    let ticks = ((DESTROYED_DWELL + 0.8 + 2.5 + 13.0) / race.dt()).round() as usize;
    for tick in 0..ticks {
        race.tick(&PlayerInputs::none());
        assert_eq!(
            race.sim.world.ships[1].physics.craft_state,
            CraftState::Eliminated,
            "the opponent came back at tick {tick}"
        );
    }
    assert_eq!(race.sim.world.ships[1].physics.shield, 0.0);
    assert_eq!(race.sim.world.ships[1].standing.deaths, 0);
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

/// Two craft in play, `victim` recorded as last struck by `killer`, its last
/// weapon hit `age` ticks ago.
fn struck(age: u64) -> Race {
    let mut race = eliminator();
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.tick = 5000;
    race.sim.last_damager[1] = Some(0);
    race.sim.last_weapon_hit[1] = 5000 + 1 - age;
    race
}

/// `Ship_Damage` credits the kill on a weapon's fatal blow.
#[test]
fn a_kill_is_credited_when_a_weapon_hit_finished_the_craft() {
    let mut race = struck(29);
    race.credit_kill(1);
    assert_eq!(race.sim.world.ships[0].standing.kills, 1);
}

/// A wall that finishes a craft off, long after the last weapon hit, credits
/// nobody - and, unlike the rule this replaced, the earlier hit is still what
/// decides it rather than a scrape having wiped the attacker.
#[test]
fn a_death_long_after_the_last_weapon_hit_credits_nobody() {
    let mut race = struck(1800);
    race.credit_kill(1);
    assert_eq!(race.sim.world.ships[0].standing.kills, 0);
}

/// Nobody is credited with their own death.
#[test]
fn a_craft_is_never_credited_with_its_own_death() {
    let mut race = struck(10);
    race.sim.last_damager[1] = Some(1);
    race.credit_kill(1);
    assert_eq!(race.sim.world.ships[1].standing.kills, 0);
}
