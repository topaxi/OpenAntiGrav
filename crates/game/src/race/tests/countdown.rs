//! The start-line countdown: thrust reads zero for the measured span and
//! releases at the full held value with no ramp.
//!
//! See [`oag_race::RaceState::thrust_gated`] and
//! `docs/gameplay/race-modes.md#the-countdown-is-measured` for the live
//! capture this pins. Split out under the 200-line rule in
//! `scripts/check-file-size.py`; shared fixtures live in the parent
//! `tests.rs`.

use super::*;
use oag_gameplay::PlayerInputs;

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
        let evaluated = race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
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
        race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    }
    let released = race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
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
        let evaluated = race.tick(&PlayerInputs::single(buttons.tick(0)));
        assert_eq!(evaluated.engine.thrust, 0.0);
    }
}

/// The opponents are held at the line for the same span as the player.
///
/// Before this, `Race::step_opponents` never consulted `RaceState` at all -
/// `oag-ai` has no dependency on `oag-race` and cannot, so the gate has to
/// live in the `game` crate that already owns both - and an opponent held
/// full throttle from tick 0 while the player sat through "3, 2, 1, go" on
/// screen, which reads as the AI cheating off the line rather than as a
/// missing feature.
#[test]
fn opponents_are_held_at_the_line_through_the_gated_span() {
    let mut race = race_with_a_grid();
    for tick in 0..oag_race::COUNTDOWN_TICKS {
        race.tick(&PlayerInputs::none());
        for slot in 1..race.ship_count() as usize {
            assert_eq!(
                race.sim.world.ships[slot].physics.thrust, 0.0,
                "opponent {slot} was moving on tick {tick}, before the gate released"
            );
        }
    }
    race.tick(&PlayerInputs::none());
    for slot in 1..race.ship_count() as usize {
        assert_eq!(
            race.sim.world.ships[slot].physics.thrust,
            oag_physics::controls::CONTROL_RANGE,
            "opponent {slot} did not release at the same tick as the player"
        );
    }
}

/// Every `ready` and `go` a race raises across its first 400 ticks, with the
/// tick each was raised on.
fn voiced_start(countdown_voice: bool) -> Vec<(u64, crate::audio::sfx::Cue)> {
    let mut setup = setup(hulled_handling());
    setup.countdown_voice = countdown_voice;
    let mut race = without_player_rescue(Race::start(setup));
    let mut raised = Vec::new();
    for _ in 0..400 {
        race.tick(&PlayerInputs::none());
        let stepped = race.sim.world.tick - 1;
        for event in race.drain_cues() {
            if matches!(
                event.cue,
                crate::audio::sfx::Cue::Ready | crate::audio::sfx::Cue::Go
            ) {
                raised.push((stepped, event.cue));
            }
        }
    }
    raised
}

/// The measured shape, as literals so `race::countdown`'s own constants cannot
/// agree with themselves: `ready`, then `go` 180 ticks later on the last tick
/// the thrust gate holds, and nothing else.
#[test]
fn a_voiced_start_raises_ready_then_go_on_the_measured_ticks() {
    use crate::audio::sfx::Cue;
    assert_eq!(voiced_start(true), vec![(91, Cue::Ready), (271, Cue::Go)]);
    assert_eq!(
        271,
        oag_race::COUNTDOWN_TICKS - 1,
        "go is the last gated tick"
    );
}

/// A title whose start has not been measured plays nothing rather than borrow
/// Pulse's ticks.
#[test]
fn an_unmeasured_title_raises_no_start_voice() {
    assert!(voiced_start(false).is_empty());
}
