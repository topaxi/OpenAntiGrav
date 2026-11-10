//! A forward move along the circuit ([`Course::jump`], [`Standing::jumped`],
//! [`crate::RaceState::jumped`]): the ground moved over counts as driven, so a crossing of
//! the line in it is a lap (or a finish) on the tick of the move, whatever its length
//! under a lap.

use super::*;
use crate::testing;
use crate::{Mode, RaceState, SpeedClass};

/// A point `distance` units round [`testing::ring_course`]'s 120-unit square.
fn on_ring(distance: f32) -> Vec3 {
    let d = distance.rem_euclid(120.0);
    match d {
        d if d < 30.0 => Vec3::new(d, 0.0, 0.0),
        d if d < 60.0 => Vec3::new(30.0, 0.0, d - 30.0),
        d if d < 90.0 => Vec3::new(30.0 - (d - 60.0), 0.0, 30.0),
        d => Vec3::new(0.0, 0.0, 30.0 - (d - 90.0)),
    }
}

/// Drives `standing` one unit a tick from `from` for `ticks` ticks, starting at `tick0`;
/// returns the ticks a lap completed on.
fn drive(standing: &mut Standing, course: &Course, from: f32, tick0: u64, ticks: u64) -> Vec<u64> {
    (0..ticks)
        .filter(|t| standing.update(course, on_ring(from + *t as f32), tick0 + t, Some(3)))
        .map(|t| tick0 + t)
        .collect()
}

/// Moves `standing` `by` units on at `tick`; whether that made a lap, and where it is.
fn jump(standing: &mut Standing, course: &Course, by: f32, tick: u64) -> (bool, Vec3) {
    let from = standing.course_index.expect("located") as usize;
    let to = course.jump(from, by).expect("a move under a lap");
    let lap = standing.jumped(course, to, tick, Some(3));
    (lap, course.position(to).expect("a ring point"))
}

#[test]
fn a_move_of_a_lap_or_more_is_refused() {
    let course = testing::ring_course();
    let lap = course.length();
    assert_eq!(course.max_jump(), lap);
    assert!(course.jump(0, lap - 1.0).is_some());
    for by in [lap, lap + 1.0, 0.0, -5.0, f32::NAN, f32::INFINITY] {
        assert_eq!(course.jump(0, by), None, "{by}");
    }
    assert_eq!(course.index_at_progress(0.0), Some(course.start_index()));
    assert_eq!(course.index_at_progress(lap), None);
    let at = course.index_at_progress(60.0).unwrap();
    assert!((course.progress_at(at).unwrap() - 60.0).abs() < 3.0);
}

/// From the grid to lap 2, 20 units short of the line, then a 40-unit move: the lap
/// completes on the move's tick, and the next update counts nothing more.
#[test]
fn a_move_over_the_line_completes_the_lap_on_its_tick() {
    let course = testing::ring_course();
    let mut standing = Standing::default();
    let laps = drive(&mut standing, &course, 110.0, 0, 230);
    assert_eq!(laps.len(), 1, "{laps:?}");
    assert_eq!(standing.lap, 2);
    let tick = 230;
    let lap_two_started = standing.lap_start_tick.expect("lap 2 running");
    let (lap, landed) = jump(&mut standing, &course, 40.0, tick);
    assert!(lap, "the move crossed the line");
    assert!(
        !standing.update(&course, landed, tick, Some(3)),
        "counted once"
    );
    assert_eq!(standing.lap, 3);
    assert_eq!(standing.lap_start_tick, Some(tick));
    let split = u64::from(standing.lap_splits[1].expect("lap 2 timed"));
    assert_eq!(split, tick - lap_two_started);
}

/// Longer than half a lap, which a driven wrap could not tell from a reverse one: still
/// one lap, and the next real crossing is the next lap.
#[test]
fn a_move_of_most_of_a_lap_counts_one_lap() {
    let course = testing::ring_course();
    let mut standing = Standing::default();
    drive(&mut standing, &course, 110.0, 0, 50);
    assert_eq!(standing.lap, 1);
    let (lap, landed) = jump(&mut standing, &course, 100.0, 50);
    assert!(lap, "over the line");
    assert!(!standing.update(&course, landed, 50, Some(3)));
    assert_eq!((standing.lap, standing.reversed), (2, 0));
    let at = standing.progress.unwrap();
    let laps = drive(&mut standing, &course, at + 1.0 + 4.0, 51, 125);
    assert_eq!(laps.len(), 1, "one lap a lap after: {laps:?}");
}

/// A move that does not reach the line counts nothing, and the gate still lets the next
/// real crossing count: a move across the half-way point is driven ground.
#[test]
fn a_move_short_of_the_line_counts_nothing_and_keeps_the_next_lap() {
    let course = testing::ring_course();
    let mut standing = Standing::default();
    drive(&mut standing, &course, 110.0, 0, 30);
    assert_eq!(standing.lap, 1);
    let (lap, landed) = jump(&mut standing, &course, 50.0, 30);
    assert!(!lap);
    assert!(!standing.update(&course, landed, 30, Some(3)));
    assert_eq!(standing.lap, 1);
    let laps = drive(&mut standing, &course, 71.0, 31, 60);
    assert_eq!(laps.len(), 1, "the next crossing is lap 1's end: {laps:?}");
    assert_eq!(standing.lap, 2);
}

/// The last lap ended by a move: the standing's finish is the move's tick, and the
/// person's clock sees it on the same tick's update.
#[test]
fn a_finish_made_by_a_move_is_timed_on_its_tick() {
    let course = testing::ring_course();
    let mut standing = Standing::default();
    let mut race = RaceState::new(Mode::SingleRace, SpeedClass::VENOM);
    race.laps_target = Some(3);
    let mut tick = 0;
    for d in 0..(10 + 2 * 120 + 50) {
        let at = on_ring(110.0 + d as f32);
        standing.update(&course, at, tick, Some(3));
        race.update(&course, at, tick, 1.0 / 60.0, false);
        tick += 1;
    }
    assert_eq!((standing.lap, race.lap), (3, 3));
    let from = standing.course_index.unwrap() as usize;
    // Ninety units: over half the lap and over the line.
    let to = course.jump(from, 90.0).unwrap();
    assert!(standing.jumped(&course, to, tick, Some(3)));
    race.jumped(&course, to);
    let landed = course.position(to).unwrap();
    assert!(!standing.update(&course, landed, tick, Some(3)));
    let outcome = race.update(&course, landed, tick, 1.0 / 60.0, false);
    assert_eq!(standing.finish_tick, Some(tick));
    assert!(outcome.finished && race.finished && outcome.lap_completed);
}

/// A move over the line that lands past half-way: the ground from the line to where it
/// landed is driven ground too, so the crossing the craft then drives is the next lap, on
/// the standing and on the person's clock alike.
#[test]
fn a_move_over_the_line_into_the_far_half_keeps_the_next_driven_crossing_a_lap() {
    let course = testing::ring_course();
    let mut standing = Standing::default();
    let mut race = RaceState::new(Mode::SingleRace, SpeedClass::VENOM);
    race.laps_target = Some(5);
    let mut tick = 0;
    let mut drive_both = |standing: &mut Standing, race: &mut RaceState, from: f32, ticks: u32| {
        for d in 0..ticks {
            let at = on_ring(from + d as f32);
            standing.update(&course, at, tick, Some(5));
            race.update(&course, at, tick, 1.0 / 60.0, false);
            tick += 1;
        }
    };
    // Off the grid, a whole lap, and on to 99 of 120 on lap 2.
    drive_both(&mut standing, &mut race, 110.0, 230);
    assert_eq!((standing.lap, race.lap), (2, 2));
    // Ninety units on: over the line, to 69, past the half-way point at 60.
    let from = standing.course_index.unwrap() as usize;
    let to = course.jump(from, 90.0).unwrap();
    assert!(standing.jumped(&course, to, 230, Some(5)));
    race.jumped(&course, to);
    let landed = course.progress_at(to).unwrap();
    assert!(landed > 60.0, "landed in the far half: {landed}");
    drive_both(&mut standing, &mut race, landed, 1);
    assert_eq!((standing.lap, race.lap), (3, 3));
    // Driven from there over the line: lap 3 is done.
    drive_both(&mut standing, &mut race, landed + 1.0, 60);
    assert_eq!((standing.lap, race.lap), (4, 4), "the driven crossing");
}
