//! The airbrake flaps on the grid: both fully out on every craft from the race's load until the
//! green light, then folding back at the hull's own `down_speed`
//! (`docs/physics/grid-state.md`, "The flaps on the grid").

use super::*;
use oag_gameplay::PlayerInputs;

/// Every active slot's two drawn flap levels, `0..=100`.
fn levels(race: &Race) -> Vec<(usize, [f32; 2])> {
    (0..race.core.sim.world.ship_count as usize)
        .filter(|&slot| race.core.sim.world.ships[slot].active)
        .map(|slot| (slot, race.view.flaps[slot]))
        .collect()
}

/// One craft at the grid and no more would not show the AI's: the fixture fields eight.
fn a_field() -> Race {
    let race = race_with_a_grid();
    assert!(levels(&race).len() > 1, "the fixture fields opponents");
    race
}

#[test]
fn every_craft_holds_its_flaps_full_from_the_load_until_the_green_light() {
    let mut race = a_field();
    for (slot, flaps) in levels(&race) {
        assert_eq!(flaps, [100.0; 2], "slot {slot} at the load");
        assert!(
            race.airbrake_flaps_of(slot)[0] > 0.0,
            "slot {slot} draws them out at the load"
        );
    }
    for tick in 0..oag_race::COUNTDOWN_TICKS - 1 {
        race.tick(&PlayerInputs::none());
        for (slot, flaps) in levels(&race) {
            assert_eq!(flaps, [100.0; 2], "slot {slot} after tick {tick}");
        }
    }
}

#[test]
fn after_the_green_light_every_craft_folds_at_the_hulls_down_speed() {
    let mut race = a_field();
    for _ in 0..oag_race::COUNTDOWN_TICKS - 1 {
        race.tick(&PlayerInputs::none());
    }
    let step = race.view.flap_graphics.down_speed * race.core.sim.dt;
    assert!(step > 0.0, "the hull authors a down_speed");
    let mut expected = 100.0_f32;
    let mut ticks = 0;
    while expected > 0.0 {
        race.tick(&PlayerInputs::none());
        expected = (expected - step).max(0.0);
        ticks += 1;
        for (slot, flaps) in levels(&race) {
            assert_eq!(
                flaps, [expected; 2],
                "slot {slot}, {ticks} tick(s) after GO"
            );
        }
    }
    assert_eq!(
        ticks,
        (100.0 / step).ceil() as i32,
        "folded in the time the rate gives"
    );
}

#[test]
fn a_posed_capture_stows_the_grid_flaps_and_they_stay_stowed() {
    let mut race = a_field();
    race.stow_grid_flaps();
    for (slot, flaps) in levels(&race) {
        assert_eq!(flaps, [0.0; 2], "slot {slot} stowed at once");
    }
    for tick in 0..oag_race::COUNTDOWN_TICKS {
        race.tick(&PlayerInputs::none());
        for (slot, flaps) in levels(&race) {
            assert_eq!(flaps, [0.0; 2], "slot {slot} after tick {tick}");
        }
    }
}

/// The flaps are drawing state: neither forcing them nor stowing them reaches the state hash.
#[test]
fn the_grid_flaps_reach_no_state_hash() {
    let run = |stowed: bool| {
        let mut race = a_field();
        if stowed {
            race.stow_grid_flaps();
        }
        for _ in 0..oag_race::COUNTDOWN_TICKS + 30 {
            race.tick(&PlayerInputs::none());
        }
        race.core.sim.state_hash()
    };
    assert_eq!(run(false), run(true));
}
