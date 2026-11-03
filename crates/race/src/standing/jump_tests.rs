//! A forward jump along the circuit ([`Course::jump`], [`Standing::jumped`],
//! [`crate::RaceState::jumped`]): a crossing of the line made by the jump is a lap,
//! timed on the tick of the update after the jump, and a finish made by it lands there.

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

/// Jumps `standing` `by` units from where it is located and returns where it lands.
fn jump(standing: &mut Standing, course: &Course, by: f32) -> Vec3 {
    let from = standing.course_index.expect("located") as usize;
    let to = course.jump(from, by).expect("a jump below half a lap");
    standing.jumped(to);
    course.position(to).expect("a ring point")
}

#[test]
fn a_jump_of_half_a_lap_or_more_is_refused() {
    let course = testing::ring_course();
    let half = course.length() * 0.5;
    assert_eq!(course.max_jump(), half);
    assert!(course.jump(0, half - 1.0).is_some());
    for by in [half, half + 1.0, 0.0, -5.0, f32::NAN, f32::INFINITY] {
        assert_eq!(course.jump(0, by), None, "{by}");
    }
}

/// From the grid to lap 2, 20 units short of the line, then a 40-unit jump: the lap
/// completes on the tick of the update after the jump, and is timed to it.
#[test]
fn a_jump_over_the_line_completes_the_lap_on_the_jump_tick() {
    let course = testing::ring_course();
    let mut standing = Standing::default();
    // Off the grid ten behind the line, round once, and on to 100 into lap 2.
    let laps = drive(&mut standing, &course, 110.0, 0, 230);
    assert_eq!(laps.len(), 1, "{laps:?}");
    assert_eq!(standing.lap, 2);
    let tick = 230;
    let lap_two_started = standing.lap_start_tick.expect("lap 2 running");
    let landed = jump(&mut standing, &course, 40.0);
    assert!(
        standing.update(&course, landed, tick, Some(3)),
        "the jump crossed the line"
    );
    assert_eq!(standing.lap, 3);
    assert_eq!(standing.lap_start_tick, Some(tick));
    // Timed to the jump's tick: the 40 units it covered took one tick.
    let split = u64::from(standing.lap_splits[1].expect("lap 2 timed"));
    assert_eq!(split, tick - lap_two_started);
    assert!(split < 120, "shorter than a driven lap: {split}");
}

/// A jump that does not reach the line counts nothing, and the gate still lets the
/// next real crossing count: a jump across the half-way point is driven ground.
#[test]
fn a_jump_short_of_the_line_counts_nothing_and_keeps_the_next_lap() {
    let course = testing::ring_course();
    let mut standing = Standing::default();
    drive(&mut standing, &course, 110.0, 0, 30);
    assert_eq!(standing.lap, 1);
    // Twenty into lap 1, jumped to 70: over the half-way point.
    let landed = jump(&mut standing, &course, 50.0);
    assert!(!standing.update(&course, landed, 30, Some(3)));
    assert_eq!(standing.lap, 1);
    let laps = drive(&mut standing, &course, 71.0, 31, 60);
    assert_eq!(laps.len(), 1, "the next crossing is lap 1's end: {laps:?}");
    assert_eq!(standing.lap, 2);
}

/// The last lap ended by a jump: the finish is the tick of the update after it.
#[test]
fn a_finish_made_by_a_jump_is_timed_on_the_jump_tick() {
    let course = testing::ring_course();
    let mut standing = Standing::default();
    let mut race = RaceState::new(Mode::SingleRace, SpeedClass::VENOM);
    race.laps_target = Some(3);
    let mut tick = 0;
    // Drive the person's clock and the standing together to 20 short of the finish.
    for d in 0..(10 + 2 * 120 + 100) {
        let at = on_ring(110.0 + d as f32);
        standing.update(&course, at, tick, Some(3));
        race.update(&course, at, tick, 1.0 / 60.0, false);
        tick += 1;
    }
    assert_eq!((standing.lap, race.lap), (3, 3));
    let from = standing.course_index.unwrap() as usize;
    let to = course.jump(from, 40.0).unwrap();
    standing.jumped(to);
    race.jumped(to);
    let landed = course.position(to).unwrap();
    assert!(standing.update(&course, landed, tick, Some(3)));
    let outcome = race.update(&course, landed, tick, 1.0 / 60.0, false);
    assert_eq!(standing.finish_tick, Some(tick));
    assert!(outcome.finished && race.finished);
}
