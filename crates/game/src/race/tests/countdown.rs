//! The start-line countdown: thrust reads zero for the measured span and
//! releases at the full held value with no ramp.
//!
//! See [`oag_race::RaceState::thrust_gated`] and
//! `docs/gameplay/race-modes.md#the-countdown-is-measured` for the live
//! capture this pins. Split out under the 200-line rule in
//! `scripts/check-file-size.py`; shared fixtures live in the parent
//! `tests.rs`.

use super::*;

/// [`hulled_handling`] with a real engine, the same numbers
/// `race_with_weapon_table` uses - invented, large enough that a thrust of
/// zero and a thrust that fired are never close to each other by accident.
fn thrust_capable_race() -> Race {
    let mut handling = hulled_handling();
    handling.engine.amount = 20.0;
    handling.engine.accelcap = 1000.0;
    without_player_rescue(Race::start(setup(handling)))
}

/// Cross held from tick 0: every tick inside the measured 272-tick span
/// still reports zero thrust, exactly like the capture's flat `throttleState`.
#[test]
fn thrust_reads_zero_for_the_whole_gated_span() {
    let mut race = thrust_capable_race();
    let mut buttons = Buttons::new();
    for tick in 0..oag_race::COUNTDOWN_TICKS {
        let evaluated = race.tick(&buttons.tick(CROSS));
        assert_eq!(
            evaluated.engine.thrust, 0.0,
            "tick {tick} should still be gated"
        );
    }
}

/// The capture showed a step, not a ramp: the first ungated tick already
/// reads the full held thrust rather than climbing to it.
#[test]
fn thrust_releases_at_the_full_held_value_with_no_ramp() {
    let mut race = thrust_capable_race();
    let mut buttons = Buttons::new();
    for _ in 0..oag_race::COUNTDOWN_TICKS {
        race.tick(&buttons.tick(CROSS));
    }
    let released = race.tick(&buttons.tick(CROSS));
    assert!(
        released.engine.thrust > 0.0,
        "the first ungated tick should thrust immediately, not ramp in: {}",
        released.engine.thrust
    );
}

/// A countdown with nothing held produces no thrust either, which is the
/// ordinary case a player who waits for green sees - this only shows the
/// gate is not what a bored tester would notice by holding nothing.
#[test]
fn an_untouched_pad_produces_no_thrust_through_or_past_the_gate() {
    let mut race = thrust_capable_race();
    let mut buttons = Buttons::new();
    for _ in 0..(oag_race::COUNTDOWN_TICKS + 10) {
        let evaluated = race.tick(&buttons.tick(0));
        assert_eq!(evaluated.engine.thrust, 0.0);
    }
}
