//! What ending a race does to the table it leaves behind.
//!
//! The standings are written onto the craft rather than driven for, which is
//! deliberate: the lap rule is `oag_race`'s and is tested there, and this is
//! about *when* the board is taken and *what* goes on it. The end-to-end
//! version - eight craft actually driving three laps until the flag falls out
//! of the simulation itself - is `crates/game/tests/race_finish_ground_truth.rs`,
//! which needs a disc image.

use super::*;

use crate::tests::race_with_a_grid;
use oag_gameplay::PlayerInputs;

/// A craft that has crossed for the last time, on the tick given.
fn finish(race: &mut Race, slot: usize, tick: u64) {
    let laps_target = race.sim.world.laps_target().unwrap_or(0);
    let standing = &mut race.sim.world.ships[slot].standing;
    standing.lap = laps_target + 1;
    standing.finish_tick = Some(tick);
}

#[test]
fn a_race_in_progress_has_no_results() {
    let mut race = race_with_a_grid();
    race.tick(&PlayerInputs::none());
    assert!(!race.finished());
    assert!(race.results().is_none());
}

#[test]
fn the_board_is_taken_on_the_tick_the_race_finishes() {
    let mut race = race_with_a_grid();
    finish(&mut race, 0, 5_000);
    race.sim.world.primary_race_mut().finished = true;
    race.tick(&PlayerInputs::none());
    let board = race
        .results()
        .expect("the race finished, so it has a board");
    assert_eq!(board.rows.len(), usize::from(race.sim.world.ship_count));
    assert_eq!(board.laps_target, race.sim.world.laps_target());
    assert_eq!(board.tick, oag_race::race_clock_ticks(race.sim.world.tick));
}

/// The board's times are the racing clock: the original's clocks start on the
/// release, so a finish 5,000 ticks into the race is 5,000 minus the
/// countdown on the board, for the player and every other row alike.
#[test]
fn a_finish_time_excludes_the_start_line_countdown() {
    let mut race = race_with_a_grid();
    finish(&mut race, 0, 5_000);
    finish(&mut race, 2, 5_100);
    race.sim.world.primary_race_mut().finished = true;
    race.capture_results();
    let board = race.results().expect("a board");

    let time = |slot| board.rows.iter().find(|row| row.slot == slot)?.finish_tick;
    assert_eq!(time(0), Some(5_000 - oag_race::COUNTDOWN_TICKS));
    assert_eq!(time(2), Some(5_100 - oag_race::COUNTDOWN_TICKS));
}

/// The field is still moving when the player crosses. A board that kept
/// answering from the live world would show a different result a second later,
/// which is the one thing "the race is over" may not do.
#[test]
fn the_board_is_not_revised_after_it_is_taken() {
    let mut race = race_with_a_grid();
    finish(&mut race, 0, 1_000);
    race.sim.world.primary_race_mut().finished = true;
    race.capture_results();
    let taken = race.results().cloned().expect("a board");

    finish(&mut race, 3, 1_200);
    race.capture_results();
    assert_eq!(race.results(), Some(&taken));
}

#[test]
fn a_finisher_is_placed_ahead_of_everyone_still_racing() {
    let mut race = race_with_a_grid();
    finish(&mut race, 5, 900);
    finish(&mut race, 0, 1_000);
    race.sim.world.primary_race_mut().finished = true;
    race.capture_results();
    let board = race.results().expect("a board");

    assert_eq!(board.rows[0].slot, 5, "the earlier crossing takes first");
    assert_eq!(board.rows[1].slot, 0);
    assert_eq!(board.player().map(|row| row.place), Some(2));
    assert!(board.rows[2..].iter().all(|row| !row.finished()));
}

/// A row per grid slot and no more: a single race grids eight, the three
/// single-ship modes grid one, and a slot beyond the count holds no craft.
#[test]
fn there_is_one_row_per_grid_slot() {
    let mut race = race_with_a_grid();
    finish(&mut race, 0, 1_000);
    race.sim.world.primary_race_mut().finished = true;
    race.capture_results();
    let board = race.results().expect("a board");
    assert_eq!(board.rows.len(), 8);
    let mut slots: Vec<u8> = board.rows.iter().map(|row| row.slot).collect();
    slots.sort_unstable();
    assert_eq!(slots, (0..8).collect::<Vec<_>>());
}

/// Every place from `oag_race::places` appears exactly once, so the table
/// cannot show two seconds and no third.
#[test]
fn the_places_are_a_permutation_of_the_field() {
    let mut race = race_with_a_grid();
    finish(&mut race, 2, 800);
    finish(&mut race, 0, 1_000);
    race.sim.world.primary_race_mut().finished = true;
    race.capture_results();
    let board = race.results().expect("a board");
    let mut places: Vec<u8> = board.rows.iter().map(|row| row.place).collect();
    places.sort_unstable();
    assert_eq!(places, (1..=8).collect::<Vec<_>>());
}

/// With two people the race is over when both are, not when the first is: the
/// first to cross sees the world run on (their craft the driver's) until the
/// second does, and the board marks both of their rows.
#[test]
fn a_race_with_two_people_ends_when_both_have_finished() {
    let mut race = race_with_a_grid();
    race.sim.world.controllers[1] = oag_gameplay::Controller::Local;
    assert!(race.sim.world.person_flown(1));

    finish(&mut race, 0, 1_000);
    race.sim.world.race[0].finished = true;
    race.tick(&PlayerInputs::none());
    assert!(
        !race.finished(),
        "the race ended with a person still racing"
    );
    assert!(!race.runs_on_after_the_line());
    assert!(race.results().is_none(), "the board was taken early");

    finish(&mut race, 1, 1_200);
    race.sim.world.race[1].finished = true;
    race.tick(&PlayerInputs::none());
    assert!(race.finished(), "both people finished and the race did not");
    assert!(race.runs_on_after_the_line());
    let board = race.results().expect("the last person finished");
    let players: Vec<u8> = board
        .rows
        .iter()
        .filter(|row| row.player)
        .map(|row| row.slot)
        .collect();
    assert_eq!(players, vec![0, 1], "every person's row is a player's row");
}

/// The second person alone finishing does not end it either: the primary's
/// flag was the whole condition before, and this is what guards its being
/// *every* person's and not "any".
#[test]
fn the_second_person_finishing_first_does_not_end_the_race() {
    let mut race = race_with_a_grid();
    race.sim.world.controllers[1] = oag_gameplay::Controller::Local;
    race.sim.world.race[1].finished = true;
    assert!(!race.finished());
}

/// Two people on the straight, split in two, the first having crossed the line on
/// `tick`: the view of the person who finished while the other is still racing.
fn first_person_finished(tick: u64) -> Race {
    let mut race = race_with_a_grid();
    race.sim.world.controllers[1] = oag_gameplay::Controller::Local;
    assert_eq!(race.set_viewport_count(2), 2);
    finish(&mut race, 0, tick);
    race.sim.world.race[0].finished = true;
    race.tick(&PlayerInputs::none());
    race
}

/// What a person's own view says after their crossing and before the race ends:
/// the standing line, the lap counter held at the target and the clock held at
/// that crossing. The other person's view, still racing, says none of it.
#[test]
fn a_person_who_has_finished_sees_it_while_the_other_races_on() {
    let race = first_person_finished(5_000);
    assert!(!race.finished(), "the second person is still racing");
    assert_eq!(race.views_drawn(), 2, "the split stays until the race ends");

    let line = |readout: &oag_hud::Readout| {
        readout.messages[oag_hud::messages::SLOTS - 1]
            .as_ref()
            .map(|line| line.text.clone())
    };
    let finished = race.readout_for(0);
    assert_eq!(line(&finished).as_deref(), Some("OAG_SPLIT_FINISHED"));
    assert_eq!(
        finished.race_ticks,
        oag_race::race_clock_ticks(5_000),
        "the clock is held at the crossing, not running on"
    );
    assert_eq!(line(&race.readout_for(1)), None, "still racing");
}

/// The line is a split's alone: with one person the HUD is hidden at the finish
/// and nothing here may add a line to it.
#[test]
fn one_person_gets_no_finished_line() {
    let mut race = race_with_a_grid();
    finish(&mut race, 0, 5_000);
    race.sim.world.race[0].finished = true;
    race.tick(&PlayerInputs::none());
    assert!(
        race.readout_for(0).messages[oag_hud::messages::SLOTS - 1].is_none(),
        "a one-person race changed its HUD"
    );
}

/// The race ends with both people finished, and then one view is drawn over the
/// end screens while the split itself stays what says a split raced: records, the
/// ghost and a restart all read `viewport_count`.
#[test]
fn the_end_screens_sit_over_one_view_and_the_split_is_kept() {
    let mut race = first_person_finished(5_000);
    finish(&mut race, 1, 5_300);
    race.sim.world.race[1].finished = true;
    race.tick(&PlayerInputs::none());
    assert!(race.finished());
    assert_eq!(race.views_drawn(), 1);
    assert_eq!(race.viewport_count(), 2);
}

/// A craft the autopilot flies is the primary's, and in a split every other local
/// seat's, so a headless run can get both people over the line; an AI slot is not
/// "taken" by it and a one-person race is flown as it always was.
#[test]
fn the_operators_autopilot_flies_every_local_seat_and_no_other() {
    let mut race = race_with_a_grid();
    race.set_autopilot(true);
    assert!(race.flown_for_the_player(0));
    assert!(!race.flown_for_the_player(1), "an AI slot");
    race.sim.world.controllers[1] = oag_gameplay::Controller::Local;
    assert!(race.flown_for_the_player(1), "a second local seat");
    race.sim.world.controllers[1] = oag_gameplay::Controller::Remote;
    assert!(!race.flown_for_the_player(1), "somebody else's craft");
}
