//! Every craft on the grid times its own laps, on a real circuit off a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! cargo nextest run -p oag-game --run-ignored all lap_times
//! ```
//!
//! # What only real data can show here
//!
//! The clock itself is unit-tested in `oag-race`, on a synthetic ring, including
//! the one invariant that matters most - that a [`oag_race::Standing`]'s clock and
//! [`oag_race::RaceState`]'s agree tick for tick, since slot 0 carries both. None
//! of that needs a disc.
//!
//! What needs one is the **grid**. `Standing::update` starts lap 1's clock at the
//! craft's first crossing of the line rather than at the standing start, and the
//! reason is a property of authored track data: a track's `Start Position` node is
//! the *back* of the grid, `Course::START_LINE_OFFSET` behind the line, and the
//! eight slots run forward from it in two staggered columns. So every craft begins
//! a different distance from the line, from rest. A synthetic fixture can only
//! assert that rule against a spacing it invented itself; this one asserts it
//! against the spacing Talon's Junction actually ships.

use std::path::PathBuf;

use oag_game::race;

/// The disc, or `None` on a checkout without one.
fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// What one craft's clock did over a run.
#[derive(Debug, Clone, Copy, Default)]
struct Clock {
    /// The tick this craft's clock started on, i.e. its first crossing.
    started: Option<u64>,
    /// The first lap this craft was *timed* for, captured the first tick a best
    /// lap existed at all.
    ///
    /// Usually lap 1. Not always: the grid straddles the line, so a craft in a
    /// forward slot has its first located fix already past it, and its first wrap
    /// is a lap whose start was never observed. That lap is deliberately given no
    /// time rather than one measured from a start nobody saw, so for those craft
    /// this is lap 2.
    first_lap: Option<u32>,
    /// The tick that lap completed on, which is what timing from the standing
    /// start would have reported as its duration.
    first_lap_ended: Option<u64>,
    /// The tick that lap *began* on - the value of `lap_start_tick` as it stood
    /// on the tick before it completed, since completing restamps it.
    first_lap_began: Option<u64>,
    /// The best lap at the end of the run.
    best: Option<u32>,
    /// Laps reached.
    lap: u32,
}

/// Runs a full grid on one circuit and reports every craft's clock.
fn clocks(track: &str, ticks: u64) -> Option<Vec<Clock>> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        // The one mode that fields a grid, so there are eight clocks to compare
        // rather than one.
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);

    let mut clocks = vec![Clock::default(); oag_gameplay::MAX_SHIPS];
    // Last tick's `lap_start_tick` per slot. Completing a lap restamps the field
    // in the same update that records the time, so the tick the lap *began* on is
    // only readable from the tick before.
    let mut previous_start = [None; oag_gameplay::MAX_SHIPS];
    for _ in 0..ticks {
        race.tick(&oag_gameplay::InputSnapshot::default());
        // **The world's own tick, not the loop counter.** `Race::tick` increments
        // `world.tick` before it advances the standings, so a lap recorded on the
        // n-th call carries tick n+1 - and reconstructing that offset from out
        // here is how a one-tick error gets built into a test rather than caught
        // by one.
        let tick = race.world.tick;
        for (slot, clock) in clocks.iter_mut().enumerate() {
            let standing = &race.world.ships[slot].standing;
            if clock.started.is_none() {
                clock.started = standing.lap_start_tick;
            }
            // The first best lap a craft has is the first lap it was timed for,
            // because there has been nothing else to be better than.
            if clock.first_lap.is_none() && standing.best_lap_ticks.is_some() {
                clock.first_lap = standing.best_lap_ticks;
                clock.first_lap_ended = Some(tick);
                clock.first_lap_began = previous_start[slot];
            }
            clock.best = standing.best_lap_ticks;
            clock.lap = standing.lap;
            previous_start[slot] = standing.lap_start_tick;
        }
    }
    Some(clocks)
}

/// Every opponent that got round has a lap time of its own.
///
/// The thing this crate could not say before: a `Standing` counted laps and
/// nothing timed them, so a results table could rank the field and not report a
/// single number about it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_opponent_that_laps_has_a_lap_time() {
    let Some(clocks) = clocks("Data\\Environments\\16_Track\\track.vex", 9_000) else {
        return;
    };

    for (slot, clock) in clocks.iter().enumerate() {
        println!(
            "slot {slot}: lap {:<3} clock started {:?}  lap 1 {:?}  best {:?}",
            clock.lap,
            clock.started,
            clock.first_lap.map(|t| format!("{:.2}s", t as f32 / 60.0)),
            clock.best.map(|t| format!("{:.2}s", t as f32 / 60.0)),
        );
    }

    // Slot 0 is the player, issued no input here, so it drives no lap and has no
    // lap time. Asserting that keeps this test honest about which craft it is
    // measuring.
    //
    // **Its clock does start, though, and that is not a defect.** Measured at
    // tick 5,028 on this run: craft-to-craft collision is implemented, the grid
    // straddles the line, and seven opponents leaving a standing start shove the
    // parked craft over it. So "the clock started" means the craft crossed the
    // line, which it did - being pushed across one is still crossing it.
    assert_eq!(clocks[0].lap, 1, "the parked player drove a lap");
    assert_eq!(
        clocks[0].best, None,
        "the parked player recorded a lap time"
    );

    let lapped: Vec<usize> = (1..oag_gameplay::MAX_SHIPS)
        .filter(|slot| clocks[*slot].lap >= 2)
        .collect();
    assert!(
        lapped.len() >= 6,
        "only {} of seven opponents completed a lap",
        lapped.len()
    );
    for slot in lapped {
        let best = clocks[slot].best.unwrap_or_else(|| {
            panic!("slot {slot} completed a lap and has no time for it");
        });
        // A wide bracket on purpose. The point is that the number is a lap and
        // not a race clock or a tick count, not that Talon's Junction takes any
        // particular time - `race_ground_truth` measures the pace itself.
        assert!(
            (20 * 60..90 * 60).contains(&best),
            "slot {slot}'s best lap is {best} ticks, which is not a lap time"
        );
    }
}

/// **Lap 1 is timed from the line, and the grid is what makes that visible.**
///
/// Every craft starts from rest a different distance behind the line, so a lap 1
/// timed from the standing start would be longer than the lap actually driven -
/// by a different amount for each slot, which is what would make the field's
/// first laps incomparable with each other and with the player's.
///
/// The two numbers this separates are on the same craft: the lap's own duration,
/// and the tick it finished on. Timing from the standing start would make them
/// equal, and the assertion is exact rather than a margin - lap 1 *is* genuinely
/// slower than a craft's best, by up to 44 % on a real grid fighting through the
/// first corner, so any ratio bound here would measure the traffic rather than
/// the clock.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn lap_one_is_not_inflated_by_the_distance_from_the_grid_to_the_line() {
    let Some(clocks) = clocks("Data\\Environments\\16_Track\\track.vex", 9_000) else {
        return;
    };

    let mut spread = Vec::new();
    for (slot, clock) in clocks.iter().enumerate().skip(1) {
        let (Some(started), Some(first), Some(ended), Some(began)) = (
            clock.started,
            clock.first_lap,
            clock.first_lap_ended,
            clock.first_lap_began,
        ) else {
            continue;
        };
        spread.push(started);

        // Nobody's clock starts with the race. Each craft is behind the line at
        // tick zero and has to drive to it from rest.
        assert!(
            started > 0,
            "slot {slot}'s clock started on tick 0, i.e. at the standing start"
        );
        // The lap is shorter than the race had been running when it ended, by
        // the ticks that craft spent getting to the line.
        assert!(
            u64::from(first) < ended,
            "slot {slot}'s first timed lap is {first} ticks and it finished on \
             tick {ended}: the clock is measuring the race, not the lap"
        );
        assert_eq!(
            u64::from(first),
            ended - began,
            "slot {slot}'s first timed lap is not the line-to-line duration \
             (began {began}, ended {ended})"
        );
    }

    assert!(spread.len() >= 6, "too few craft to compare");
    // The clocks do not all start together, because the slots are not all the
    // same distance back. This is the authored grid geometry showing through, and
    // it is the reason the rule cannot be "start every clock on tick zero".
    let (first, last) = (
        spread.iter().min().copied().unwrap(),
        spread.iter().max().copied().unwrap(),
    );
    println!("clocks started between tick {first} and tick {last}");
    assert!(
        last > first,
        "every craft crossed the line on the same tick, so the grid has no depth"
    );
}
