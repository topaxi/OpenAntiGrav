//! What the race state in [`super`] is asserted to do.
//!
//! Split out of `state.rs` under the 200-line cap on inline `#[cfg(test)]`
//! modules; see `scripts/check-file-size.py`.

use super::{COUNTDOWN_TICKS as COUNTDOWN, Outcome, RaceState};
use crate::testing::square_track;
use crate::{Course, Mode, SpeedClass, zone};

/// The fixed timestep, from ADR-0007.
const DT: f32 = 1.0 / 60.0;

fn course() -> Course {
    Course::from_track(&square_track(2), None).expect("a ring")
}

/// Drives one full lap by stepping the ship along every ring point in order.
///
/// Returns the outcomes, one per tick, so a test can assert on the edge
/// rather than only on the state that survives it.
fn drive_lap(state: &mut RaceState, course: &Course, from_tick: u64) -> Vec<Outcome> {
    (0..course.len())
        .map(|index| {
            let position = course.position(index).expect("in range");
            state.update(course, position, from_tick + index as u64, DT, false)
        })
        .collect()
}

/// The bug this guards, found by driving a time trial on Moa Therma: the
/// ship spawns on the grid slot, the slot is upstream of the start line, so
/// the ship crosses the line seconds into the race. That is a real wrap of
/// the distance-along and a bare wrap test counted it as a completed lap.
#[test]
fn crossing_the_line_just_after_the_start_is_not_a_lap() {
    let course = course();
    let mut state = RaceState::new(Mode::TimeTrial, SpeedClass::Venom);

    // Spawn a few points *before* the line and drive over it.
    let before_line = course.len() - 3;
    for (tick, index) in (0..).zip((before_line..course.len()).chain(0..6)) {
        let position = course.position(index).expect("in range");
        let outcome = state.update(&course, position, tick, DT, false);
        assert!(
            !outcome.lap_completed,
            "counted a lap {tick} ticks into the race, at ring point {index}"
        );
    }
    assert_eq!(state.lap, 1, "the race left the line already on lap 2");
}

/// And having done that, a real lap still counts - the guard must not eat it.
#[test]
fn the_first_real_lap_after_an_early_crossing_still_counts() {
    let course = course();
    let mut state = RaceState::new(Mode::TimeTrial, SpeedClass::Venom);

    let before_line = course.len() - 3;
    let mut tick = 0;
    for index in (before_line..course.len()).chain(0..course.len()) {
        let position = course.position(index).expect("in range");
        state.update(&course, position, tick, DT, false);
        tick += 1;
    }
    // Back over the line after a full circuit.
    let outcome = state.update(&course, course.position(0).unwrap(), tick, DT, false);
    assert!(outcome.lap_completed, "a genuine lap did not count");
    assert_eq!(state.lap, 2);
}

#[test]
fn a_ship_nudged_back_and_forth_over_the_line_cannot_ratchet_the_counter() {
    let course = course();
    let mut state = RaceState::new(Mode::SpeedLap, SpeedClass::Venom);
    for tick in 0..20 {
        let index = if tick % 2 == 0 { course.len() - 1 } else { 0 };
        state.update(&course, course.position(index).unwrap(), tick, DT, false);
    }
    assert_eq!(state.lap, 1, "rocking over the line counted laps");
}

#[test]
fn the_first_fix_of_a_race_is_never_a_lap() {
    let course = course();
    let mut state = RaceState::new(Mode::TimeTrial, SpeedClass::Venom);
    // Spawn most of the way round, so the very first reading is a large
    // distance. Without the "no previous fix" guard this reads as a wrap.
    let position = course.position(course.len() - 1).expect("in range");
    let outcome = state.update(&course, position, 0, DT, false);
    assert!(!outcome.lap_completed);
    assert_eq!(state.lap, 1);
}

#[test]
fn driving_all_the_way_round_completes_exactly_one_lap() {
    let course = course();
    let mut state = RaceState::new(Mode::TimeTrial, SpeedClass::Venom);
    let outcomes = drive_lap(&mut state, &course, 0);
    let laps = outcomes.iter().filter(|o| o.lap_completed).count();
    assert_eq!(
        laps, 0,
        "the lap only closes when the line is crossed again"
    );

    // One more step, back onto the first point: that is the wrap.
    let position = course.position(0).expect("in range");
    let outcome = state.update(&course, position, course.len() as u64, DT, false);
    assert!(outcome.lap_completed);
    assert_eq!(state.lap, 2);
    assert_eq!(state.laps_completed(), 1);
}

#[test]
fn the_lap_time_is_recorded_and_the_best_is_kept() {
    let course = course();
    let mut state = RaceState::new(Mode::SpeedLap, SpeedClass::Venom);
    let mut tick = 0;

    // Three laps. The clock is driven by the tick we pass in, not by how
    // many points we stepped through, so the laps can be given different
    // durations without moving the ship differently.
    let mut lap_times = Vec::new();
    for gap in [0u64, 100, 40] {
        drive_lap(&mut state, &course, tick);
        tick += course.len() as u64 + gap;
        let position = course.position(0).expect("in range");
        let outcome = state.update(&course, position, tick, DT, false);
        assert!(outcome.lap_completed, "lap did not close");
        lap_times.push(outcome.lap_ticks.expect("a completed lap has a time"));
        tick += 1;
    }

    assert_eq!(
        state.best_lap_ticks,
        Some(*lap_times.iter().min().expect("three laps")),
        "best lap is not the fastest of {lap_times:?}"
    );
    // Each lap keeps its own time too, not just whichever was fastest -
    // `Lap1Time`-`Lap4Image` on HD's HUD show the history, not a running best.
    for (index, &ticks) in lap_times.iter().enumerate() {
        assert_eq!(
            state.lap_splits[index],
            Some(ticks),
            "lap {} did not keep its own time",
            index + 1
        );
    }
    assert_eq!(
        state.lap_splits[3], None,
        "a fourth lap was never driven, and must not read as one"
    );
}

/// A fifth lap has nowhere to go: [`super::MAX_RECORDED_LAPS`] is four, read
/// off the widget count HD's own `HUD_lap_times.xml` authors, and the array
/// does not grow. It must not panic and must not silently overwrite an
/// earlier lap either.
#[test]
fn a_lap_past_the_recorded_maximum_is_not_recorded_and_does_not_panic() {
    let course = course();
    let mut state = RaceState::new(Mode::SpeedLap, SpeedClass::Venom);
    let mut tick = 0;

    for _ in 0..5 {
        drive_lap(&mut state, &course, tick);
        tick += course.len() as u64;
        let position = course.position(0).expect("in range");
        let outcome = state.update(&course, position, tick, DT, false);
        assert!(outcome.lap_completed, "lap did not close");
        tick += 1;
    }

    assert_eq!(state.lap, 6, "five laps completed");
    assert!(
        state.lap_splits.iter().all(Option::is_some),
        "all four recordable slots should have filled: {:?}",
        state.lap_splits
    );
}

#[test]
fn a_time_trial_finishes_after_three_laps_and_a_speed_lap_never_does() {
    for (mode, expect_finish) in [(Mode::TimeTrial, true), (Mode::SpeedLap, false)] {
        let course = course();
        let mut state = RaceState::new(mode, SpeedClass::Venom);
        let mut tick = 0;
        let mut finished_on = None;

        for lap in 0..4 {
            drive_lap(&mut state, &course, tick);
            tick += course.len() as u64;
            let position = course.position(0).expect("in range");
            let outcome = state.update(&course, position, tick, DT, false);
            tick += 1;
            if outcome.finished {
                finished_on = Some(lap + 1);
                break;
            }
        }

        if expect_finish {
            assert_eq!(finished_on, Some(3), "{mode:?} should end on lap 3");
            assert!(state.finished);
        } else {
            assert_eq!(finished_on, None, "{mode:?} should never end on its own");
            assert!(!state.finished);
        }
    }
}

#[test]
fn a_finished_race_stops_counting() {
    let course = course();
    let mut state = RaceState::new(Mode::TimeTrial, SpeedClass::Venom);
    state.finished = true;
    let before = state;
    let outcome = state.update(&course, course.position(5).unwrap(), 999, DT, false);
    assert_eq!(outcome, Outcome::default());
    assert_eq!(state, before, "a finished race kept simulating");
}

#[test]
fn crossing_the_line_backwards_takes_the_lap_back() {
    let course = course();
    let mut state = RaceState::new(Mode::SpeedLap, SpeedClass::Venom);
    drive_lap(&mut state, &course, 0);
    let position = course.position(0).expect("in range");
    state.update(&course, position, course.len() as u64, DT, false);
    assert_eq!(state.lap, 2);

    // Reverse back over the line.
    let first_tick = course.len() as u64 + 1;
    for (tick, index) in (first_tick..).zip((course.len() - 4..course.len()).rev()) {
        let position = course.position(index).expect("in range");
        state.update(&course, position, tick, DT, false);
    }
    assert_eq!(state.lap, 1, "reversing over the line did not undo the lap");
}

#[test]
fn the_lap_count_never_goes_below_one() {
    let course = course();
    let mut state = RaceState::new(Mode::SpeedLap, SpeedClass::Venom);
    // Sit just past the line, then reverse over it repeatedly.
    for _ in 0..3 {
        state.update(&course, course.position(1).unwrap(), 0, DT, false);
        state.update(
            &course,
            course.position(course.len() - 1).unwrap(),
            1,
            DT,
            false,
        );
    }
    assert_eq!(state.lap, 1);
}

#[test]
fn a_zone_steps_every_ten_seconds_and_the_timer_resets_to_zero() {
    let course = course();
    let mut state = RaceState::new(Mode::Zone, SpeedClass::Venom);
    let position = course.position(0).expect("in range");

    // Ten seconds at the fixed 60 Hz of ADR-0007 is 600 ticks, so the 600th
    // accumulation is the one that steps. Counting the ticks it actually
    // took says that directly, and does not quietly pass if `dt` accumulates
    // to slightly under 10.0 and the step slips a tick.
    // Counted from the release: the zone machine holds through the countdown
    // (see `a_zone_holds_through_the_start_line_countdown`).
    let mut stepped_after = None;
    for tick in COUNTDOWN..COUNTDOWN + 2_000 {
        if state
            .update(&course, position, tick, DT, false)
            .zone_advanced
        {
            stepped_after = Some(tick - COUNTDOWN + 1);
            break;
        }
    }
    assert_eq!(stepped_after, Some(600), "a zone is not 600 ticks long");
    assert_eq!(state.zone, 1);

    // Reset to zero, not `-= STEP_SECONDS`. The original stores 0.0 and so
    // drops the frame overshoot; keeping the remainder would be a different
    // clock. See `zone::STEP_SECONDS`.
    assert_eq!(state.zone_timer, 0.0);
}

#[test]
fn only_zone_mode_scores_or_steps() {
    let course = course();
    for mode in [Mode::TimeTrial, Mode::SpeedLap] {
        let mut state = RaceState::new(mode, SpeedClass::Venom);
        let position = course.position(0).expect("in range");
        for tick in 0..1_000 {
            state.update(&course, position, tick, DT, false);
        }
        assert_eq!(state.zone, 0, "{mode:?} stepped a zone");
        assert_eq!(state.score, 0, "{mode:?} scored");
    }
}

#[test]
fn a_clean_zone_pays_the_bonus_and_a_dirty_one_does_not() {
    let course = course();
    let position = course.position(0).expect("in range");
    let ticks_per_zone = (zone::STEP_SECONDS * 60.0) as u64;

    let mut clean = RaceState::new(Mode::Zone, SpeedClass::Venom);
    let mut dirty = RaceState::new(Mode::Zone, SpeedClass::Venom);
    let mut perfect = (false, false);
    for tick in COUNTDOWN..=COUNTDOWN + ticks_per_zone {
        perfect.0 |= clean
            .update(&course, position, tick, DT, false)
            .perfect_zone;
        // One wall contact anywhere in the zone spoils it.
        let hit = tick == COUNTDOWN + 3;
        perfect.1 |= dirty.update(&course, position, tick, DT, hit).perfect_zone;
    }

    assert!(perfect.0, "a clean zone did not pay out");
    assert!(!perfect.1, "a zone with a wall contact paid out anyway");
    assert_eq!(
        clean.score - dirty.score,
        zone::CLEAN_ZONE_BONUS,
        "the bonus is not the recovered 500"
    );
}

#[test]
fn the_dirty_flag_clears_at_each_zone_step() {
    let course = course();
    let position = course.position(0).expect("in range");
    let ticks_per_zone = (zone::STEP_SECONDS * 60.0) as u64;
    let mut state = RaceState::new(Mode::Zone, SpeedClass::Venom);

    // Dirty the first zone only.
    state.update(&course, position, COUNTDOWN, DT, true);
    assert!(state.zone_dirty);
    for tick in COUNTDOWN + 1..=COUNTDOWN + ticks_per_zone {
        state.update(&course, position, tick, DT, false);
    }
    assert_eq!(state.zone, 1);
    assert!(!state.zone_dirty, "the flag survived the step");

    // The second zone is clean, so it pays.
    let mut paid = false;
    for tick in COUNTDOWN + ticks_per_zone + 1..=COUNTDOWN + ticks_per_zone * 2 + 1 {
        paid |= state
            .update(&course, position, tick, DT, false)
            .perfect_zone;
    }
    assert!(paid, "the zone after a dirty one could not be perfect");
}

#[test]
fn a_new_race_starts_on_lap_one_with_no_best() {
    let state = RaceState::new(Mode::TimeTrial, SpeedClass::Venom);
    assert_eq!(state.lap, 1);
    assert_eq!(state.laps_completed(), 0);
    assert_eq!(state.best_lap_ticks, None);
    assert_eq!(state.progress, None);
    assert!(!state.finished);
}

#[test]
fn the_lap_target_comes_from_the_mode() {
    assert_eq!(
        RaceState::new(Mode::TimeTrial, SpeedClass::Venom).laps_target,
        Some(3)
    );
    assert_eq!(
        RaceState::new(Mode::SpeedLap, SpeedClass::Venom).laps_target,
        None
    );
    assert_eq!(
        RaceState::new(Mode::Zone, SpeedClass::Venom).laps_target,
        None
    );
}

#[test]
fn the_default_is_a_time_trial() {
    assert_eq!(
        RaceState::default(),
        RaceState::new(Mode::TimeTrial, SpeedClass::Venom)
    );
}

#[test]
fn a_lap_clock_before_its_own_start_reads_zero() {
    let mut state = RaceState::new(Mode::TimeTrial, SpeedClass::Venom);
    state.lap_start_tick = COUNTDOWN + 100;
    assert_eq!(state.lap_ticks(COUNTDOWN + 140), 40);
    assert_eq!(state.lap_ticks(COUNTDOWN + 100), 0);
    assert_eq!(state.lap_ticks(COUNTDOWN + 99), 0);
}

/// The measured fact: the original's lap clock (`racer+0x920`) reads `0.0`
/// through the whole countdown and takes its first `dt` on the release tick.
/// Lap 1's clock is `lap_start_tick = 0` until the first crossing, so it is
/// the floor that keeps the countdown out of it.
#[test]
fn the_lap_clock_starts_on_the_release_not_on_the_grid() {
    let state = RaceState::new(Mode::TimeTrial, SpeedClass::Venom);
    assert_eq!(state.lap_ticks(0), 0);
    assert_eq!(state.lap_ticks(COUNTDOWN - 1), 0);
    assert_eq!(state.lap_ticks(COUNTDOWN), 0);
    assert_eq!(state.lap_ticks(COUNTDOWN + 1), 1);
    assert_eq!(state.lap_ticks(COUNTDOWN + 600), 600);
}

#[test]
fn the_race_clock_is_zero_through_the_countdown_and_counts_from_the_release() {
    assert_eq!(super::race_clock_ticks(0), 0);
    assert_eq!(super::race_clock_ticks(COUNTDOWN - 1), 0);
    assert_eq!(super::race_clock_ticks(COUNTDOWN), 0);
    assert_eq!(super::race_clock_ticks(COUNTDOWN + 1), 1);
}

/// Zone's score and dwell timer are not running on the grid: the original's
/// Zone machine resets both on leaving its countdown state, so counting them
/// through it would bring the first zone in `COUNTDOWN_TICKS` early.
#[test]
fn a_zone_holds_through_the_start_line_countdown() {
    let course = course();
    let position = course.position(0).expect("in range");
    let mut state = RaceState::new(Mode::Zone, SpeedClass::Venom);
    for tick in 0..COUNTDOWN {
        state.update(&course, position, tick, DT, false);
    }
    assert_eq!(state.score, 0, "scored on the grid");
    assert_eq!(state.zone_timer, 0.0, "the zone timer ran on the grid");
    state.update(&course, position, COUNTDOWN, DT, false);
    assert!(state.score > 0, "did not start scoring on the release");
}

/// The exact boundary the live capture measured: gated through tick 271,
/// released at tick 272. See [`super::COUNTDOWN_TICKS`]'s doc comment for the
/// capture this pins.
#[test]
fn thrust_is_gated_for_exactly_the_measured_span() {
    assert!(RaceState::thrust_gated(0));
    assert!(RaceState::thrust_gated(super::COUNTDOWN_TICKS - 1));
    assert!(!RaceState::thrust_gated(super::COUNTDOWN_TICKS));
    assert!(!RaceState::thrust_gated(super::COUNTDOWN_TICKS + 1));
}
