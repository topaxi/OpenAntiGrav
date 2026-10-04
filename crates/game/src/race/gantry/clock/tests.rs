use super::*;

/// Pulse's own edge through the inherited rule gives Pulse's measured
/// start tick: the rule and the measurement are one thing.
#[test]
fn pulses_edge_through_the_rule_is_the_measured_start() {
    let edge = GoEdge {
        frame: 181,
        settled_frame: 216,
        last_frame_before_exit: 349,
    };
    let clock = Clock::inherited(edge).expect("181 is before the release");
    assert_eq!(clock.start_tick, CLOCK_START_TICK);
    assert_eq!(clock.start_tick, 92);
    assert_eq!(clock.hold, Some((216.0 / 60.0, 349.0 / 60.0)));
}

/// Whatever the edge, it lands on the tick after the thrust gate.
#[test]
fn the_go_edge_lands_one_tick_after_the_release() {
    for frame in [181, 200, 203, 272] {
        let clock = Clock::inherited(GoEdge {
            frame,
            settled_frame: frame,
            last_frame_before_exit: 359,
        })
        .expect("before the release");
        let at_release = clock.seconds(COUNTDOWN_TICKS + 1) * 60.0;
        assert!(
            (at_release - frame as f32).abs() < 1e-3,
            "{frame}: {at_release}"
        );
        let before = clock.seconds(COUNTDOWN_TICKS) * 60.0;
        assert!(
            before < frame as f32,
            "{frame} lit on the release tick itself"
        );
    }
}

/// An edge after the release has no start tick to give.
#[test]
fn an_edge_after_the_release_is_refused() {
    let late = GoEdge {
        frame: (COUNTDOWN_TICKS + 2) as u32,
        settled_frame: (COUNTDOWN_TICKS + 2) as u32,
        last_frame_before_exit: 359,
    };
    assert_eq!(Clock::inherited(late), None);
}

/// Held, the clock never reaches the frame the asset exits on.
#[test]
fn an_inherited_clock_stays_inside_its_hold() {
    let clock = Clock::inherited(GoEdge {
        frame: 203,
        settled_frame: 222,
        last_frame_before_exit: 359,
    })
    .expect("before the release");
    for tick in (0..20_000).step_by(7) {
        let seconds = clock.seconds(tick);
        assert!(seconds < 359.0 / 60.0, "tick {tick}: {seconds}");
    }
    assert!(clock.seconds(COUNTDOWN_TICKS + 5000) >= 222.0 / 60.0);
}

/// HD's window, read from the EBOOT: on the release the clock jumps from
/// the free run's frame 203 to 3.83 s and loops every 86 ticks; the
/// capture's `GO` dark centres at +23, +61 and +109 ticks are 86 apart at
/// the first and third.
#[test]
fn hds_window_jumps_on_the_release_and_loops_86_ticks() {
    let clock = Clock::hd(GoEdge {
        frame: 203,
        settled_frame: 221,
        last_frame_before_exit: 359,
    })
    .expect("before the release");
    assert_eq!(clock.start_tick, 70);
    assert_eq!(HD_PRE_LAP_WINDOW.from, 3.83);
    assert_eq!(HD_PRE_LAP_WINDOW.to, 5.25);
    assert_eq!(HD_PRE_LAP_WINDOW.period(), 86);
    let release = COUNTDOWN_TICKS + 1;
    // Free-running until the release: the board is still red there.
    assert_eq!(clock.seconds(release - 1) * 60.0, 202.0);
    // On the release the clock is at the window's start, past the
    // digits' fade and `GO`'s alpha ramp, not at the edge.
    assert_eq!(clock.seconds(release), 3.83);
    for k in 0..2000 {
        let seconds = clock.seconds(release + k);
        assert!(HD_PRE_LAP_WINDOW.holds(seconds), "+{k}: {seconds}");
        assert_eq!(seconds, clock.seconds(release + k + 86), "+{k}");
    }
    // Frame 252.8, the first measured dip, is +23 ticks and again +109.
    let frame = |k: u64| clock.seconds(release + k) * 60.0;
    assert!((frame(23) - 252.8).abs() < 0.01, "{}", frame(23));
    assert!((frame(109) - 252.8).abs() < 0.01, "{}", frame(109));
    assert!((frame(61) - 290.8).abs() < 0.01, "{}", frame(61));
}

/// A clock already inside the window on its first tick carries on from
/// there and resets only when it leaves.
#[test]
fn a_clock_inside_the_window_is_not_reset() {
    let window = ReleaseWindow {
        from: 1.0,
        to: 2.0,
        from_tick: 0,
    };
    assert_eq!(window.period(), 60);
    assert_eq!(window.seconds(1.5, 0), 1.5);
    assert_eq!(window.seconds(1.5, 29), 1.5 + 29.0 / 60.0);
    assert_eq!(window.seconds(1.5, 30), 1.0);
    assert_eq!(window.seconds(1.5, 90), 1.0);
    assert_eq!(window.seconds(2.5, 0), 1.0);
    assert_eq!(window.seconds(0.5, 0), 1.0);
}

/// Pulse's clock has no window: the HD law never reaches it.
#[test]
fn pulse_has_no_window() {
    assert_eq!(Clock::PULSE.window, None);
    assert_eq!(Clock::PULSE.seconds(COUNTDOWN_TICKS + 1) * 60.0, 181.0);
}

/// Without a hold the clock is the plain timeline, as it was for every
/// title before this rule.
#[test]
fn the_race_start_clock_is_the_plain_timeline() {
    assert_eq!(Clock::FROM_RACE_START.seconds(90), 1.5);
    assert_eq!(Clock::PULSE.seconds(CLOCK_START_TICK + 60), 1.0);
}
