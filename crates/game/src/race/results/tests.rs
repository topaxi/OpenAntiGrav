//! What ending a race does to the table it leaves behind.
//!
//! The standings are written onto the craft rather than driven for, which is
//! deliberate: the lap rule is `oag_race`'s and is tested there, and this is
//! about *when* the board is taken and *what* goes on it. The end-to-end
//! version - eight craft actually driving three laps until the flag falls out
//! of the simulation itself - is `crates/game/tests/race_finish_ground_truth.rs`,
//! which needs a disc image.

use super::*;

use crate::race::tests::race_with_a_grid;

/// A craft that has crossed for the last time, on the tick given.
fn finish(race: &mut Race, slot: usize, tick: u64) {
    let standing = &mut race.sim.world.ships[slot].standing;
    standing.lap = race.sim.world.race.laps_target.unwrap_or(0) + 1;
    standing.finish_tick = Some(tick);
}

#[test]
fn a_race_in_progress_has_no_results() {
    let mut race = race_with_a_grid();
    race.tick(&InputSnapshot::default());
    assert!(!race.finished());
    assert!(race.results().is_none());
}

#[test]
fn the_board_is_taken_on_the_tick_the_race_finishes() {
    let mut race = race_with_a_grid();
    finish(&mut race, 0, 5_000);
    race.sim.world.race.finished = true;
    race.tick(&InputSnapshot::default());
    let board = race
        .results()
        .expect("the race finished, so it has a board");
    assert_eq!(board.rows.len(), usize::from(race.sim.world.ship_count));
    assert_eq!(board.laps_target, race.sim.world.race.laps_target);
    assert_eq!(board.tick, race.sim.world.tick);
}

/// The field is still moving when the player crosses. A board that kept
/// answering from the live world would show a different result a second later,
/// which is the one thing "the race is over" may not do.
#[test]
fn the_board_is_not_revised_after_it_is_taken() {
    let mut race = race_with_a_grid();
    finish(&mut race, 0, 1_000);
    race.sim.world.race.finished = true;
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
    race.sim.world.race.finished = true;
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
    race.sim.world.race.finished = true;
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
    race.sim.world.race.finished = true;
    race.capture_results();
    let board = race.results().expect("a board");
    let mut places: Vec<u8> = board.rows.iter().map(|row| row.place).collect();
    places.sort_unstable();
    assert_eq!(places, (1..=8).collect::<Vec<_>>());
}
