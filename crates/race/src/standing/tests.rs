use super::*;
use crate::testing;

fn course() -> Course {
    testing::ring_course()
}

/// A craft that started at the line and has driven to `progress` on `lap`.
///
/// **The gate is part of that claim, not a detail.** A craft that drove here
/// from the line was seen in the near half on the way, so its gate has left
/// [`LapGate::NeedsNearHalf`] - and a standing that says otherwise describes a
/// craft that is three quarters round a lap it has never started, which
/// [`Standing::distance`] now reads as a craft still sitting on the grid. Use
/// [`on_the_grid`] for that craft.
fn at(lap: u32, progress: f32) -> Standing {
    let half = course().length() * 0.5;
    Standing {
        lap,
        progress: Some(progress),
        gate: if progress >= half {
            LapGate::Ready
        } else {
            LapGate::NeedsFarHalf
        },
        ..Standing::default()
    }
}

/// A craft on the grid at `progress`, before its first crossing of the line.
///
/// Which is where a real grid is: the slots straddle the line, so most of the
/// field spawns in the *last* few percent of the circuit. The default gate is
/// what says "has not crossed yet".
fn on_the_grid(progress: f32) -> Standing {
    Standing {
        progress: Some(progress),
        ..Standing::default()
    }
}

/// A point `distance` units round [`testing::ring_course`]'s square, which runs
/// `(0,0,0)` to `(30,0,0)` to `(30,0,30)` to `(0,0,30)` and back. The start line
/// is the origin, so distance zero is the line.
fn on_ring(distance: f32) -> Vec3 {
    let d = distance.rem_euclid(120.0);
    match d {
        d if d < 30.0 => Vec3::new(d, 0.0, 0.0),
        d if d < 60.0 => Vec3::new(30.0, 0.0, d - 30.0),
        d if d < 90.0 => Vec3::new(30.0 - (d - 60.0), 0.0, 30.0),
        d => Vec3::new(0.0, 0.0, 30.0 - (d - 90.0)),
    }
}

/// What one run round the ring did.
///
/// **Ticks, not positions.** [`Course::from_track`] resamples the track and puts
/// its own origin wherever the path data starts, which is four units from
/// [`on_ring`]'s, so the tick a craft crosses the line on is a property of the
/// fixture and not of the rule under test. Every assertion below is on a
/// *duration* or on an ordering, both of which are the rule.
#[derive(Debug, Default)]
struct Run {
    /// The tick the clock started on, i.e. the first crossing of the line.
    started: Option<u64>,
    /// The ticks laps completed on.
    completed: Vec<u64>,
}

/// Drives a craft round the ring at one unit a tick from `from`, updating the
/// standing once a tick as a race does.
///
/// One unit a tick makes every duration read directly in ticks: the ring is 120
/// units round, so a lap is 120 ticks however the course numbers its own origin.
fn drive(standing: &mut Standing, course: &Course, from: f32, ticks: u64) -> Run {
    let mut run = Run::default();
    for tick in 0..ticks {
        let at = from + tick as f32;
        if standing.update(course, on_ring(at), tick, None) {
            run.completed.push(tick);
        }
        if run.started.is_none() {
            run.started = standing.lap_start_tick;
        }
    }
    run
}

#[test]
fn a_fresh_standing_has_no_clock_at_all() {
    let standing = Standing::default();
    assert_eq!(standing.lap_start_tick, None);
    assert_eq!(standing.best_lap_ticks, None);
    // `None` and not `Some(0)`: a craft on the grid has not been driving a lap
    // for no time, it has not started one.
    assert_eq!(standing.lap_ticks(500), None);
}

/// The clock starts when the craft first crosses the line, not when the race
/// does - and until then it does not run at all.
#[test]
fn a_craft_on_the_grid_has_no_clock_until_it_crosses_the_line() {
    let course = course();

    // Ten units behind the line, driven for five ticks: not there yet.
    let mut short = Standing::default();
    let stopped_short = drive(&mut short, &course, 110.0, 5);
    assert_eq!(stopped_short.started, None, "still behind the line");
    assert_eq!(short.lap, 1);

    // The same craft given long enough to reach it.
    let mut over = Standing::default();
    let crossed = drive(&mut over, &course, 110.0, 15);
    let started = crossed.started.expect("the clock started at the line");
    assert!(started < 15, "crossed within the run, on tick {started}");
    // Crossing the line off the grid is not a lap. It is the *start* of one.
    assert_eq!(over.lap, 1);
    assert!(crossed.completed.is_empty());
    assert_eq!(over.best_lap_ticks, None);
}

/// **The rule this whole clock turns on.** A craft spawns behind the line, so
/// lap 1 timed from the standing start is longer than every other lap by however
/// far back its own grid slot is - and each slot is a different distance back, so
/// the field's lap 1 times would not even be comparable with each other.
///
/// The craft starts ten units behind the line at one unit a tick, so the lap it
/// then drives is **120** ticks - the ring's own length - while the tick it
/// finishes on is ten higher than that. Timing from the grid would report the
/// second number, and it would be a different error for every slot.
#[test]
fn lap_one_is_timed_from_the_line_and_not_from_the_grid() {
    let course = course();
    let mut standing = Standing::default();
    let run = drive(&mut standing, &course, 110.0, 200);

    let started = run.started.expect("the clock started");
    let [finished] = run.completed[..] else {
        panic!("expected exactly one lap, got {:?}", run.completed)
    };
    assert_eq!(standing.lap, 2);
    assert_eq!(
        standing.best_lap_ticks,
        Some(120),
        "the ring is 120 units round at one unit a tick"
    );
    assert_eq!(
        u64::from(standing.best_lap_ticks.unwrap()),
        finished - started,
        "the lap is the line-to-line duration"
    );
    // The distinction the rule exists for: the lap is shorter than the race has
    // been running, by the ticks that were spent getting off the grid.
    assert!(started > 0, "the craft did not spawn on the line");
    assert!(
        u64::from(standing.best_lap_ticks.unwrap()) < finished,
        "timing from the standing start would have read {finished}"
    );
    // And the clock has been restamped for the lap now being driven.
    assert_eq!(standing.lap_start_tick, Some(finished));
    assert_eq!(standing.lap_ticks(finished + 20), Some(20));
}

/// Two laps, and the best is the quicker of them. The second is deliberately the
/// slower one, so a `min` that was really a "latest" would fail.
#[test]
fn the_best_lap_is_the_quickest_and_not_the_last() {
    let course = course();
    let mut standing = Standing::default();
    drive(&mut standing, &course, 110.0, 200);
    assert_eq!(standing.best_lap_ticks, Some(120));

    // A second lap at half the speed. Driven by hand rather than through
    // `drive`, which only knows one pace.
    for (tick, step) in (200u64..).zip(0..300u32) {
        let at = 20.0 + step as f32 * 0.5;
        standing.update(&course, on_ring(at), tick, None);
    }
    assert_eq!(standing.lap, 3, "a second lap completed");
    assert_eq!(
        standing.best_lap_ticks,
        Some(120),
        "the quicker first lap is kept"
    );
}

/// **`lap_splits` is a history, not a second copy of the best.** The rig above
/// proves `best_lap_ticks` keeps the quicker lap; this proves each lap's own
/// slot keeps *its own* time regardless, the way `Lap1Time`/`Lap2Time` on HD's
/// HUD read two different rows rather than the same number twice.
#[test]
fn each_lap_keeps_its_own_time_not_the_best() {
    let course = course();
    let mut standing = Standing::default();
    drive(&mut standing, &course, 110.0, 200);
    assert_eq!(
        standing.lap_splits[0],
        Some(120),
        "lap 1's own slot holds its own time"
    );
    assert_eq!(
        standing.lap_splits[1], None,
        "lap 2 has not been driven yet"
    );

    // A second, slower lap - see `the_best_lap_is_the_quickest_and_not_the_last`
    // for why this pace is driven by hand.
    for (tick, step) in (200u64..).zip(0..300u32) {
        let at = 20.0 + step as f32 * 0.5;
        standing.update(&course, on_ring(at), tick, None);
    }
    assert_eq!(standing.lap, 3, "a second lap completed");
    assert_eq!(
        standing.best_lap_ticks,
        Some(120),
        "the best is still the quicker lap"
    );
    assert_eq!(
        standing.lap_splits[0],
        Some(120),
        "lap 1's own slot is untouched by lap 2 finishing"
    );
    let second = standing.lap_splits[1].expect("lap 2 recorded its own time");
    assert!(
        second > 120,
        "the second lap was driven at half speed and should read slower, got {second}"
    );
}

/// **The two clocks must agree, because slot 0 has both of them.**
///
/// [`crate::RaceState`] times the player's laps and a [`Standing`] now times
/// every craft's, and the player is a craft - so on slot 0 the same laps are
/// measured twice, by two implementations, from the same positions and ticks.
/// Two clocks that disagreed would put a different time on the HUD than in the
/// results, which is the failure the module doc warns about for the lap *count*.
///
/// Driven here rather than on a disc because both clocks live in this crate and
/// take the same three arguments: a ground-truth run could only reach the same
/// conclusion more slowly, and this one runs in CI.
#[test]
fn the_standings_clock_agrees_with_the_players() {
    let course = course();
    // Unlimited laps, so neither clock stops at a target and several laps are
    // comparable rather than one.
    let mut player = crate::RaceState::new(crate::Mode::SpeedLap, crate::SpeedClass::Venom);
    let mut standing = Standing::default();

    let mut laps = 0u32;
    for tick in 0..500u64 {
        let at = on_ring(110.0 + tick as f32);
        // The same position and the same tick into both, which is exactly how
        // `Race::tick` feeds them.
        let by_player = player
            .update(&course, at, tick, 1.0 / 60.0, false)
            .lap_completed;
        let by_standing = standing.update(&course, at, tick, None);
        assert_eq!(
            by_player, by_standing,
            "the two disagreed about whether tick {tick} completed a lap"
        );
        if by_standing {
            laps += 1;
        }
    }

    assert!(laps >= 3, "several laps were driven, got {laps}");
    assert_eq!(
        standing.best_lap_ticks, player.best_lap_ticks,
        "the field's clock and the player's read the same lap"
    );
    assert_eq!(standing.lap, player.lap, "and count the same laps");
    assert_eq!(
        standing.lap_start_tick,
        Some(player.lap_start_tick),
        "and start the lap being driven on the same tick"
    );
}

/// Reversing over the line takes the lap count back and deliberately leaves the
/// clock alone, exactly as `RaceState::uncomplete_lap` does. Inventing a time for
/// a lap that was un-driven would put a wrong number on a results table.
#[test]
fn driving_backwards_over_the_line_invents_no_lap_time() {
    let course = course();
    let mut standing = Standing::default();
    drive(&mut standing, &course, 110.0, 200);
    let (best, started) = (standing.best_lap_ticks, standing.lap_start_tick);
    assert_eq!(standing.lap, 2);

    // Back over the line the way it came.
    for (tick, step) in (200u64..).zip(0..30u32) {
        standing.update(&course, on_ring(20.0 - step as f32), tick, None);
    }
    assert_eq!(standing.lap, 1, "the lap count went back");
    assert_eq!(standing.best_lap_ticks, best, "the best lap is untouched");
    assert_eq!(standing.lap_start_tick, started, "and so is the clock");
}

/// **A craft shoved backwards over the line and straight back over it forwards
/// earns no lap either.** Reversing over the line resets the lap count, but
/// until 2026-09-07 it left `self.gate` at `Ready` - the value the *forward*
/// drive above had already earned - so the very next forward crossing read
/// `Ready` and completed "lap 1" again on the spot, against the clock still
/// timing the lap actually being driven. That is not a short lap, it is no
/// lap: the craft never left the near half of the one it is being credited
/// for.
///
/// This is the scenario `crates/game/tests/lap_times_ground_truth.rs` hit for
/// real: a corrected craft-pair narrowphase threw an opponent back across
/// Talon's Junction's line in one tick, and its very next ordinary forward
/// crossing - seconds later, nothing anomalous about it - was booked as a
/// 316-tick lap.
#[test]
fn an_immediate_re_crossing_after_a_backward_wrap_earns_no_lap() {
    let course = course();
    let mut standing = Standing::default();
    drive(&mut standing, &course, 110.0, 200);
    let (best, started) = (standing.best_lap_ticks, standing.lap_start_tick);
    assert_eq!(standing.lap, 2);

    // Back over the line the way it came - the same 30 ticks the sibling test
    // uses, which is enough to cross it: `on_ring(20.0 - step)` reaches
    // negative distances, i.e. the far side of the line.
    let mut tick = 200u64;
    for step in 0..30u32 {
        standing.update(&course, on_ring(20.0 - step as f32), tick, None);
        tick += 1;
    }
    assert_eq!(standing.lap, 1, "the lap count went back");

    // And straight forward again, back across the same line, continuing from
    // exactly where the reversal left off.
    let mut completed_on = None;
    for step in 1..=20u32 {
        let position = on_ring(-9.0 + step as f32);
        if standing.update(&course, position, tick, None) {
            completed_on = Some(tick);
        }
        tick += 1;
    }

    assert_eq!(
        completed_on, None,
        "a lap completed at tick {completed_on:?}, ticks after a backward wrap - \
         the gate was not re-earned"
    );
    assert_eq!(standing.lap, 1, "no lap re-completed itself");
    assert_eq!(
        standing.best_lap_ticks, best,
        "a bogus short lap overwrote the real one"
    );
    assert_eq!(
        standing.lap_start_tick, started,
        "the clock was not restamped"
    );
}

#[test]
fn a_fresh_standing_is_on_lap_one_and_has_not_finished() {
    let standing = Standing::default();
    assert_eq!(standing.lap, 1);
    assert!(!standing.finished());
    assert_eq!(standing.progress, None);
}

#[test]
fn further_round_is_a_better_place() {
    let course = course();
    let standings = [at(1, 10.0), at(1, 90.0), at(2, 5.0)];
    assert_eq!(places(&standings, &course), [3, 2, 1]);
}

#[test]
fn a_finisher_beats_anyone_still_racing() {
    let course = course();
    let mut done = at(4, 0.0);
    done.finish_tick = Some(500);
    // Further round the circuit, and still racing, so still second.
    let standings = [at(9, 0.0), done];
    assert_eq!(places(&standings, &course), [2, 1]);
}

#[test]
fn finishers_are_placed_by_when_they_finished() {
    let course = course();
    let early = Standing {
        finish_tick: Some(100),
        ..Standing::default()
    };
    let late = Standing {
        finish_tick: Some(200),
        ..Standing::default()
    };
    assert_eq!(places(&[late, early], &course), [2, 1]);
}

/// **The first lap of every race**: the grid straddles the start line, so some
/// craft are a few units *before* it and some a few units after, and the ones
/// before must not be placed a whole lap ahead.
///
/// Three craft a unit apart across the line - one just short of it, two just
/// over - in the order a grid puts them, back to front. The right answer is that
/// the one short of the line is last; the arithmetic this replaced made it first
/// by nearly a whole circuit.
#[test]
fn a_craft_that_has_not_reached_the_line_is_behind_one_just_past_it() {
    let course = course();
    let behind = on_the_grid(course.length() - 1.0);
    let over = on_the_grid(0.5);
    let further = on_the_grid(1.0);
    assert_eq!(places(&[behind, over, further], &course), [3, 2, 1]);
    assert_eq!(behind.distance(&course), -1.0);
}

/// The gate is what separates the two, not the distance: a craft that *has*
/// crossed and come round to the far half of lap 1 is ahead of the field, not
/// behind it.
#[test]
fn a_craft_already_round_to_the_far_half_of_lap_one_is_not_read_as_on_the_grid() {
    let course = course();
    let far = course.length() * 0.75;
    let round = at(1, far);
    let grid = on_the_grid(far);
    assert_eq!(round.distance(&course), far);
    assert_eq!(grid.distance(&course), far - course.length());
    assert_eq!(places(&[grid, round], &course), [2, 1]);
}

/// Eight craft on the grid have no fix at all and are all at distance zero.
/// The order has to be *something*, and it has to be the same something every
/// run.
#[test]
fn craft_that_are_exactly_level_are_placed_by_slot() {
    let course = course();
    let standings = [Standing::default(); 8];
    assert_eq!(places(&standings, &course), [1, 2, 3, 4, 5, 6, 7, 8]);
}
