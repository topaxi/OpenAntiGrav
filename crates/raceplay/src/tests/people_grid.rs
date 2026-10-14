//! Where the people of a split-screen race start on the grid.
//!
//! PS2 Pulse and HD/Fury both spawn a two-person race the same way: the first person on the
//! back row, the second on the row before it, the AI ahead (`oag_gameplay::rank_from_back`).
//! The fixture is the straight grid of [`race_with_a_grid`], whose forward axis is `+X`, so a
//! craft's grid row is read off its `x`.

use super::*;

/// A grid race with `people` people, built the way the load builds it.
fn race_with(people: u8) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.people = people;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(setup)
}

/// Ships ordered from the back of the grid to the front.
fn back_to_front(race: &Race) -> Vec<usize> {
    let mut ships: Vec<usize> = (0..usize::from(race.sim.world.ship_count)).collect();
    ships.sort_by(|&a, &b| {
        let x = |ship: usize| race.sim.world.ships[ship].physics.body.position.x;
        x(a).total_cmp(&x(b))
    });
    ships
}

/// One person: the player on the back row and the opponents ahead, ship 1 at the front. This
/// is the grid every earlier test and ground truth was built on.
#[test]
fn one_person_starts_last_with_the_opponents_ahead() {
    let race = race_with(1);
    assert_eq!(race.sim.world.ship_count, 8);
    assert_eq!(back_to_front(&race), [0, 7, 6, 5, 4, 3, 2, 1]);
}

/// Two people: person 1 on the back row, person 2 on the row before it, not on the front.
/// Fails if the second person is left on an opponent's front row.
#[test]
fn two_people_start_on_the_last_two_rows() {
    let race = race_with(2);
    assert_eq!(back_to_front(&race), [0, 1, 7, 6, 5, 4, 3, 2]);
}

/// Three to eight people take the rows from the back in order (chosen, not measured: the
/// originals ship two people).
#[test]
fn more_people_take_the_next_rows_forward() {
    for people in 3..=8_u8 {
        let race = race_with(people);
        let order = back_to_front(&race);
        let people = usize::from(people);
        assert_eq!(
            order[..people],
            (0..people).collect::<Vec<_>>(),
            "{people} people"
        );
        let opponents: Vec<usize> = order[people..].to_vec();
        let mut ahead = opponents.clone();
        ahead.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(
            opponents, ahead,
            "the opponents keep their order, {people} people"
        );
    }
}

/// The people change who stands where, never the grid: the same eight places are occupied at
/// every head count, with the same heading and height.
#[test]
fn the_grid_is_the_same_eight_places_at_every_head_count() {
    let places = |race: &Race| {
        let mut places: Vec<[u32; 3]> = race
            .sim
            .world
            .ships
            .iter()
            .take(usize::from(race.sim.world.ship_count))
            .map(|ship| ship.physics.body.position.to_array().map(f32::to_bits))
            .collect();
        places.sort_unstable();
        places
    };
    let one = places(&race_with(1));
    for people in 2..=8 {
        assert_eq!(places(&race_with(people)), one, "{people} people");
    }
}

/// A person's craft starts exactly where an opponent's would have: the pose is the row's,
/// whoever stands on it, so the second person is level with the row's own column.
#[test]
fn a_person_stands_on_the_row_an_opponent_stood_on() {
    let one = race_with(1);
    let two = race_with(2);
    // Person 2 now stands where ship 7 stood.
    assert_eq!(
        two.sim.world.ships[1].physics.body.position,
        one.sim.world.ships[7].physics.body.position
    );
    // And ship 7 moved forward one row, to where ship 6 stood.
    assert_eq!(
        two.sim.world.ships[7].physics.body.position,
        one.sim.world.ships[6].physics.body.position
    );
    // Person 1 did not move.
    assert_eq!(
        two.sim.world.ships[0].physics.body.position,
        one.sim.world.ships[0].physics.body.position
    );
}

/// A Head to Head race with `people` people, built the way the load builds it.
fn head_to_head_with(people: u8) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::Head2Head;
    setup.people = people;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(setup)
}

/// The `x` of grid row `row` (1 front, 8 back) of the full grid, read off an eight-craft race.
fn full_grid_x(row: usize) -> f32 {
    let race = race_with(1);
    let order = back_to_front(&race);
    race.sim.world.ships[order[8 - row]].physics.body.position.x
}

/// One person: the AI on place 1 and the player on place 2, the two FRONT places (PPSSPP,
/// measured). Fails on the build that packed the field to the back rows 8 and 7.
#[test]
fn head_to_head_one_person_starts_on_the_front_two_places() {
    let race = head_to_head_with(1);
    assert_eq!(race.sim.world.ship_count, 2);
    let x = |ship: usize| race.sim.world.ships[ship].physics.body.position.x;
    assert!((x(1) - full_grid_x(1)).abs() < 1e-3, "AI on place 1");
    assert!((x(0) - full_grid_x(2)).abs() < 1e-3, "player on place 2");
}

/// Two people: person 2 on place 1 and person 1 on place 2 (PS2, measured).
#[test]
fn head_to_head_two_people_start_on_the_front_two_places() {
    let race = head_to_head_with(2);
    assert_eq!(race.sim.world.ship_count, 2);
    let x = |ship: usize| race.sim.world.ships[ship].physics.body.position.x;
    assert!((x(1) - full_grid_x(1)).abs() < 1e-3, "person 2 on place 1");
    assert!((x(0) - full_grid_x(2)).abs() < 1e-3, "person 1 on place 2");
}

/// The rule behind it, with no mode in it: a front-anchored field of N fills places 1..=N and
/// a packed one keeps its rank from the back of the eight.
#[test]
fn the_row_of_a_field_is_its_rank_from_the_front_or_the_back() {
    use crate::spawn::grid_row;
    assert_eq!(grid_row(0, 2, true), 2);
    assert_eq!(grid_row(1, 2, true), 1);
    assert_eq!(grid_row(0, 2, false), 8);
    assert_eq!(grid_row(1, 2, false), 7);
    for rank in 0..8 {
        assert_eq!(grid_row(rank, 8, true), grid_row(rank, 8, false));
    }
}
