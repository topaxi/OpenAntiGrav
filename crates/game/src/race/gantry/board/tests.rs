use super::*;
use crate::race::gantry::{Clock, GoEdge, HD_PRE_LAP_WINDOW};

use oag_race::COUNTDOWN_TICKS;

fn hd() -> Clock {
    Clock::hd(GoEdge {
        frame: 203,
        settled_frame: 221,
        last_frame_before_exit: 359,
    })
    .expect("before the release")
}

/// The bounds are the EBOOT's TOC floats, bit for bit.
#[test]
fn the_window_bounds_are_the_toc_floats() {
    assert_eq!(HD_BETWEEN_LAPS_WINDOW, (6.017, 9.3));
    assert_eq!(HD_FINAL_LAP_AHEAD_WINDOW, (9.5, 9.9));
    assert_eq!(HD_CHEQUERED_WINDOW, (12.35, 13.3));
}

/// A three-lap race: `GO` on the grid, the `FX-350` board on lap 1,
/// `FINAL LAP` on lap 2, the flag on lap 3 and after the finish.
#[test]
fn a_three_lap_race_walks_the_windows_in_order() {
    use BoardWindow::*;
    assert_eq!(BoardWindow::select(0, 3, false), PreLap);
    assert_eq!(BoardWindow::select(1, 3, false), BetweenLaps);
    assert_eq!(BoardWindow::select(2, 3, false), FinalLapAhead);
    assert_eq!(BoardWindow::select(3, 3, false), Chequered);
    assert_eq!(BoardWindow::select(4, 3, true), Chequered);
}

/// The edge cases the EBOOT's own compares decide: a one-lap race goes
/// straight from `GO` to the flag, a two-lap race has no between-laps
/// board, and a race with no lap count never leaves it.
#[test]
fn short_and_endless_races_follow_the_eboots_compares() {
    use BoardWindow::*;
    assert_eq!(BoardWindow::select(1, 1, false), Chequered);
    assert_eq!(BoardWindow::select(1, 2, false), FinalLapAhead);
    assert_eq!(BoardWindow::select(2, 2, false), Chequered);
    for lap in 1..20 {
        assert_eq!(BoardWindow::select(lap, 0, false), BetweenLaps, "{lap}");
    }
    assert_eq!(BoardWindow::select(0, 0, false), PreLap);
}

/// The counter is 0 until the player's lap clock starts on the first
/// crossing, then this build's own lap.
#[test]
fn the_counter_is_zero_until_the_first_crossing() {
    let mut standing = Standing::default();
    assert_eq!(standing.lap, 1);
    assert_eq!(BoardWindow::of(&standing, Some(3)), BoardWindow::PreLap);
    standing.lap_start_tick = Some(900);
    assert_eq!(
        BoardWindow::of(&standing, Some(3)),
        BoardWindow::BetweenLaps
    );
    standing.lap = 3;
    assert_eq!(BoardWindow::of(&standing, Some(3)), BoardWindow::Chequered);
    standing.lap = 4;
    standing.finish_tick = Some(5000);
    assert_eq!(BoardWindow::of(&standing, Some(3)), BoardWindow::Chequered);
    assert_eq!(BoardWindow::of(&standing, None), BoardWindow::BetweenLaps);
}

/// The window is entered on the crossing tick; a second lap in the same
/// window does not re-enter it.
#[test]
fn the_tracker_enters_on_the_crossing_and_keeps_a_shared_window() {
    let mut tracker = BoardTracker::default();
    assert_eq!(tracker.observe(500, BoardWindow::PreLap, None), None);
    let first = tracker.observe(905, BoardWindow::BetweenLaps, Some(900));
    assert_eq!(
        first,
        Some(WindowEntry {
            window: BoardWindow::BetweenLaps,
            since: 900
        })
    );
    // Lap 2 of 5 is still between laps: same entry, whatever the new crossing.
    assert_eq!(
        tracker.observe(3000, BoardWindow::BetweenLaps, Some(2990)),
        first
    );
    let ahead = tracker.observe(5000, BoardWindow::FinalLapAhead, Some(4998));
    assert_eq!(ahead.map(|e| e.since), Some(4998));
    // A crossing reported later than the frame is clamped to the frame.
    let mut fresh = BoardTracker::default();
    let entry = fresh.observe(10, BoardWindow::Chequered, Some(11));
    assert_eq!(entry.map(|e| e.since), Some(10));
}

/// Every transition is an immediate jump to the new window's start, then a
/// loop of the window's own length - the reset at `0x0005f110`.
#[test]
fn each_transition_jumps_to_the_new_windows_start() {
    let clock = hd();
    for (window, period) in [
        (BoardWindow::BetweenLaps, 197),
        (BoardWindow::FinalLapAhead, 24),
        (BoardWindow::Chequered, 57),
    ] {
        let since = 4321;
        let entry = Some(WindowEntry { window, since });
        let (from, to) = window.bounds().expect("a later window");
        // The tick before the crossing is still the pre-lap loop.
        let before = clock.seconds_in(since - 1, entry);
        assert!((HD_PRE_LAP_WINDOW.from..HD_PRE_LAP_WINDOW.to).contains(&before));
        assert_eq!(clock.seconds_in(since, entry), from, "{window:?}");
        assert_eq!(
            entry.and_then(|e| e.release_window()).map(|w| w.period()),
            Some(period),
            "{window:?}"
        );
        for k in 0..600 {
            let seconds = clock.seconds_in(since + k, entry);
            assert!(
                from <= seconds && seconds < to,
                "{window:?} +{k}: {seconds}"
            );
            assert_eq!(seconds, clock.seconds_in(since + k + period, entry));
        }
    }
}

/// Before the first crossing the window entry changes nothing, and Pulse's
/// clock ignores one altogether.
#[test]
fn no_entry_is_the_pre_lap_clock_and_pulse_ignores_entries() {
    let clock = hd();
    let release = COUNTDOWN_TICKS + 1;
    for tick in [0, 100, release, release + 500] {
        assert_eq!(clock.seconds_in(tick, None), clock.seconds(tick));
    }
    let entry = Some(WindowEntry {
        window: BoardWindow::Chequered,
        since: release + 10,
    });
    for tick in [release, release + 10, release + 900] {
        assert_eq!(
            Clock::PULSE.seconds_in(tick, entry),
            Clock::PULSE.seconds(tick)
        );
    }
}

/// Before the between-laps window the countdown's own set is hidden; after
/// it, the frame's own.
#[test]
fn the_cull_switches_tables_at_the_between_laps_start() {
    let cull = PanelCull {
        countdown: vec![7],
        first_frame: 361,
        frames: vec![vec![1], vec![2], vec![3]],
    };
    assert_eq!(cull.hidden(0.0), &[7]);
    assert_eq!(cull.hidden(5.24), &[7]);
    assert_eq!(cull.hidden(HD_BETWEEN_LAPS_WINDOW.0), &[1]);
    assert_eq!(cull.hidden(362.0 / 60.0), &[2]);
    assert_eq!(cull.hidden(13.29), &[3]);
}
