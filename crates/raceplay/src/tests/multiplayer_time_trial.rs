//! A Time Trial with two or more people is Pulse's Multiplayer Time Trial: the people and
//! nobody else on the grid, craft that do not touch, and a board ranked by total time.
//!
//! The mapping from "Time Trial and a head count" to the original's mode `17` is chosen; the
//! rules are measured (`docs/ghidra/functions/psp-pulse-usa/mp-time-trial.md`). The disc-backed
//! half, on a real circuit, is `tests/mp_time_trial_ground_truth.rs`.

use super::*;
use oag_gameplay::Controller;

/// A grid on the synthetic straight in `mode`, `people` of them flown by a person.
fn race_of(mode: Mode, people: u8, opponents: bool, no_opponents: bool) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = mode;
    setup.people = people;
    setup.opponents = opponents;
    setup.no_opponents = no_opponents;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let mut race = Race::start(setup);
    for slot in 0..usize::from(people).min(usize::from(race.sim.world.ship_count)) {
        race.sim.world.controllers[slot] = Controller::Local;
    }
    race
}

/// The people fly the craft and nobody else is on the grid, whatever the escape hatch or a
/// class with no driver says. Fails on the build before this one, which fielded the AI's seven
/// beside the people (the hatch) or none at all for the second person (`no_opponents`).
#[test]
fn the_field_is_the_people_and_nobody_else() {
    for people in 2..=8_u8 {
        for (opponents, no_opponents) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let race = race_of(Mode::TimeTrial, people, opponents, no_opponents);
            let world = &race.sim.world;
            assert_eq!(
                world.ship_count, people,
                "{people} people (hatch {opponents}, no driver {no_opponents})"
            );
            let active = (0..8).filter(|&slot| world.ships[slot].active).count();
            assert_eq!(active, usize::from(people), "{people} people");
            assert_eq!(world.human_slots().count(), usize::from(people));
        }
    }
}

/// Each person has a grid row of their own, the back rows in seat order (`Race_SpawnGrid`'s
/// "a short grid packs to the back"): no two craft start on one spot.
#[test]
fn every_person_starts_on_a_row_of_their_own() {
    let race = race_of(Mode::TimeTrial, 4, false, false);
    let mut rows: Vec<f32> = (0..4)
        .map(|slot| race.sim.world.ships[slot].physics.body.position.x)
        .collect();
    assert!(
        rows.windows(2).all(|pair| pair[0] < pair[1]),
        "seat order is back to front: {rows:?}"
    );
    rows.dedup();
    assert_eq!(rows.len(), 4);
}

/// One person is the ordinary Time Trial: alone on the grid, as it was.
#[test]
fn one_person_is_the_ordinary_time_trial() {
    let race = race_of(Mode::TimeTrial, 1, false, false);
    assert_eq!(race.sim.world.ship_count, 1);
}

/// Speed Lap has no multiplayer twin in the original and keeps what it had: two people in
/// one fielded the grid by the same hatch every split race uses.
#[test]
fn speed_lap_with_two_people_is_left_as_it_was() {
    let race = race_of(Mode::SpeedLap, 2, true, false);
    assert_eq!(race.sim.world.ship_count, 8);
    let race = race_of(Mode::SpeedLap, 2, false, false);
    assert_eq!(race.sim.world.ship_count, 1);
}

/// Two craft on top of each other and closing.
fn overlap(race: &mut Race) {
    let at = race.sim.world.ships[0].physics.body.position;
    race.sim.world.ships[1].physics.body.position = at + Vec3::new(0.0, 0.0, 1.0);
    race.sim.world.ships[0].physics.body.linear_velocity = Vec3::new(0.0, 0.0, 8.0);
    race.sim.world.ships[1].physics.body.linear_velocity = Vec3::new(0.0, 0.0, -8.0);
}

/// Craft pass through each other in a Multiplayer Time Trial, and the same overlap in a Single
/// Race of two people is a contact: dropping the gate in `resolve_craft_pairs` fails the first,
/// breaking the contact itself fails the second.
#[test]
fn people_in_a_multiplayer_time_trial_pass_through_each_other() {
    let mut race = race_of(Mode::TimeTrial, 2, false, false);
    overlap(&mut race);
    let before = [
        race.sim.world.ships[0].physics.body,
        race.sim.world.ships[1].physics.body,
    ];
    race.resolve_craft_pairs();
    assert!(
        race.pair_contacts.is_empty(),
        "no contact between the people"
    );
    assert_eq!(race.sim.world.ships[0].physics.body, before[0]);
    assert_eq!(race.sim.world.ships[1].physics.body, before[1]);

    let mut race = race_of(Mode::SingleRace, 2, false, false);
    overlap(&mut race);
    race.resolve_craft_pairs();
    assert_eq!(
        race.pair_contacts,
        [(0, 1)],
        "a Single Race's craft collide"
    );
}

/// One person in a Time Trial still meets the opponents the escape hatch fields: the rule
/// needs a second *person*, not a second craft.
#[test]
fn one_person_among_hatch_opponents_still_collides() {
    let mut race = race_of(Mode::TimeTrial, 1, true, false);
    assert!(race.sim.world.ship_count > 1);
    overlap(&mut race);
    race.resolve_craft_pairs();
    assert_eq!(race.pair_contacts, [(0, 1)]);
}

/// The board is ordered by total time: whoever finished on the earlier tick is first, whatever
/// seat or grid row they had. Fails if the order came from the grid.
#[test]
fn the_board_ranks_the_people_by_total_time() {
    let mut race = race_of(Mode::TimeTrial, 3, false, false);
    let laps = race.sim.world.laps_target().unwrap_or(0);
    for (slot, tick) in [(0_usize, 9_000_u64), (1, 8_500), (2, 8_800)] {
        let standing = &mut race.sim.world.ships[slot].standing;
        standing.lap = laps + 1;
        standing.finish_tick = Some(tick);
    }
    for slot in 0..3 {
        race.sim.world.race[slot].finished = true;
    }
    race.capture_results();
    let board = race
        .results()
        .expect("every person finished, so there is a board");
    let order: Vec<u8> = board.rows.iter().map(|row| row.slot).collect();
    assert_eq!(order, [1, 2, 0], "fastest total time first");
    assert_eq!(board.persons(), 3, "every row is a person's");
    assert!(board.shows_place(), "three craft have a place to show");
}
