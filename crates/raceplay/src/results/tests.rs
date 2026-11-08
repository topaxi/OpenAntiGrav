//! What ending a race does to the table it leaves behind.
//!
//! The standings are written onto the craft rather than driven for, which is
//! deliberate: the lap rule is `oag_race`'s and is tested there, and this is
//! about *when* the board is taken and *what* goes on it. The end-to-end
//! version - eight craft actually driving three laps until the flag falls out
//! of the simulation itself - is `crates/raceplay/tests/race_finish_ground_truth.rs`,
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

/// A network race names its people on the board it leaves: the name each seat raced under, on a
/// person's row and nowhere else (a name given for a craft the AI flies is not a person's).
#[test]
fn a_person_s_row_carries_the_name_they_raced_under() {
    let mut race = race_with_a_grid();
    race.set_person_names(vec!["Ada".into(), String::new(), "Eve".into()]);
    finish(&mut race, 0, 5_000);
    race.sim.world.primary_race_mut().finished = true;
    race.capture_results();
    let board = race.results().expect("a board");
    let name = |slot| {
        board
            .rows
            .iter()
            .find(|row| row.slot == slot)
            .and_then(|row| row.name.as_deref())
    };
    assert_eq!(name(0), Some("Ada"));
    assert_eq!(name(1), None, "no name given");
    assert_eq!(
        name(2),
        None,
        "slot 2 is flown by the AI, so it is nobody's row"
    );
}

/// A race whose player crossed the line first, the board taken, and the rest of the field
/// still lapping.
fn player_won() -> Race {
    let mut race = race_with_a_grid();
    finish(&mut race, 0, 1_000);
    race.sim.world.primary_race_mut().finished = true;
    race.tick(&PlayerInputs::none());
    assert!(race.runs_on_after_the_line());
    assert!(race.results().is_some());
    race
}

fn row_of(race: &Race, slot: u8) -> scoreboard::Row {
    race.results()
        .and_then(|board| board.rows.iter().find(|row| row.slot == slot))
        .cloned()
        .expect("the slot has a row")
}

/// The claim: a craft still racing when the player finished gets its time and its place on the
/// board once it crosses, while the screens are up. Before, the row held no time for good.
#[test]
fn a_craft_that_finishes_after_the_player_gets_its_time_on_the_board() {
    let mut race = player_won();
    assert_eq!(row_of(&race, 3).finish_tick, None, "still racing");

    race.tick_finished();
    assert_eq!(row_of(&race, 3).finish_tick, None, "nothing changed yet");
    let revision = race.results_revision();

    finish(&mut race, 3, 1_200);
    race.tick_finished();
    let row = row_of(&race, 3);
    assert_eq!(
        row.finish_tick,
        Some(1_200 - oag_race::COUNTDOWN_TICKS),
        "the racing clock, like every other row"
    );
    assert_eq!(
        row.place, 2,
        "behind the player, ahead of those still racing"
    );
    assert_eq!(race.results_revision(), revision + 1);
}

/// The record a screen or the voice already read at the finish must not move: the player's row,
/// the tick the race ended on and the names stay what the finish took, whatever the field does.
#[test]
fn the_players_row_and_the_finish_tick_stay_as_the_finish_took_them() {
    let mut race = race_with_a_grid();
    race.set_person_names(vec!["Ada".into()]);
    finish(&mut race, 0, 1_000);
    race.sim.world.primary_race_mut().finished = true;
    race.tick(&PlayerInputs::none());
    let (player, tick) = (row_of(&race, 0), race.results().expect("a board").tick);

    for (slot, at) in [(4, 1_100), (2, 1_300), (6, 1_500)] {
        finish(&mut race, slot, at);
        race.tick_finished();
        assert_eq!(row_of(&race, 0), player);
        assert_eq!(race.results().expect("a board").tick, tick);
    }
}

/// A board that did not change is not replaced, so a screen rebuilt on a revision change does no
/// work on the ticks nothing happened, and none once every craft is home.
#[test]
fn the_revision_moves_only_when_the_board_does() {
    let mut race = player_won();
    race.tick_finished();
    let settled = race.results_revision();
    race.tick_finished();
    assert_eq!(race.results_revision(), settled);

    for slot in 1..8usize {
        finish(&mut race, slot, 1_000 + slot as u64 * 10);
    }
    race.tick_finished();
    let home = race.results_revision();
    assert_ne!(home, settled);
    assert!(
        race.results()
            .expect("a board")
            .rows
            .iter()
            .all(|row| row.finished())
    );
    race.tick_finished();
    race.tick_finished();
    assert_eq!(race.results_revision(), home, "every craft finished");
}

/// The refresh is a read: ticking a finished race through `tick_finished` leaves the very state
/// hash, every tick, that stepping it with the same neutral input does with no refresh.
#[test]
fn refreshing_the_board_moves_no_state_hash() {
    let mut refreshed = player_won();
    let mut plain = player_won();
    for tick in 0..90 {
        if tick == 30 {
            for race in [&mut refreshed, &mut plain] {
                finish(race, 3, 1_200);
            }
        }
        refreshed.tick_finished();
        plain.tick(&PlayerInputs::none());
        assert_eq!(
            refreshed.sim.state_hash(),
            plain.sim.state_hash(),
            "diverged at run-on tick {tick}"
        );
    }
    assert_eq!(row_of(&refreshed, 3).finish_tick.is_some(), true);
    assert_eq!(
        row_of(&plain, 3).finish_tick,
        None,
        "the plain run keeps the first board"
    );
}

/// A race that ended any other way stands still, so there is nothing to refresh.
#[test]
fn a_race_that_did_not_end_on_the_line_keeps_its_board() {
    let mut race = race_with_a_grid();
    race.sim.world.primary_race_mut().finished = true;
    race.tick(&PlayerInputs::none());
    assert!(race.results().is_some());
    assert!(!race.runs_on_after_the_line());
    let (held, revision) = (race.results().cloned(), race.results_revision());
    finish(&mut race, 3, 1_200);
    race.refresh_results();
    assert_eq!(race.results().cloned(), held);
    assert_eq!(race.results_revision(), revision);
}
