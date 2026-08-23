//! What the input-script parser in [`super`] is asserted to do: the run-length
//! grammar, the axis rules, the errors a malformed line raises, and the
//! committed scenarios.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `script.rs`: the tests are 258 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

#[test]
fn a_run_length_line_expands_to_that_many_ticks() {
    let script = Script::parse("3 cross\n2 none\n").expect("parses");
    assert_eq!(script.len(), 5);
    assert!(script.states[2].is_held(Button::Cross));
    assert_eq!(script.states[3], State::default());
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let script =
        Script::parse("# a comment\n\n  \n2 cross  # thrust\n#2 cross, but commented out\n")
            .expect("parses");
    assert_eq!(script.len(), 2);
}

/// The axis derivation must be the one the real input system uses, or a
/// scripted run and a played one would not be the same experiment. This is
/// the assertion that stands in for a dependency `oag-trace` is not allowed
/// to have.
#[test]
fn the_axes_follow_the_same_rule_the_input_system_uses() {
    let script = Script::parse("1 left\n1 right\n1 up\n1 down\n1 l\n1 r\n").expect("parses");
    assert_eq!(script.states[0].stick_x, -1.0);
    assert_eq!(script.states[1].stick_x, 1.0);
    assert_eq!(script.states[2].stick_y, 1.0);
    assert_eq!(script.states[3].stick_y, -1.0);
    assert_eq!(script.states[4].airbrake_left, 1.0);
    assert_eq!(script.states[5].airbrake_right, 1.0);
}

#[test]
fn an_explicit_axis_overrides_the_one_the_buttons_derive() {
    let script = Script::parse("1 left stick_x=-0.25\n").expect("parses");
    assert_eq!(script.states[0].stick_x, -0.25);
    assert!(
        script.states[0].is_held(Button::Left),
        "the button is still held; only the axis is overridden"
    );
}

/// The lead releases the first ticks **in place**: every later transition
/// has to land on the tick it already landed on, because that is the half of
/// `psp-trace.py --script-lead` that is correct. Delaying the whole script by
/// two instead was tried against `talons-junction-time-trial-lap.csv` and is
/// measurably worse - it pulls the airbrake columns from a mean error of
/// 0.012 to 3.75.
#[test]
fn a_capture_lead_releases_the_first_ticks_without_moving_the_rest() {
    let script = Script::parse("3 cross right\n2 cross\n").expect("parses");
    let led = script.clone().with_capture_lead(2);
    assert_eq!(led.len(), script.len());
    assert_eq!(led.states[0], State::default());
    assert_eq!(led.states[1], State::default());
    assert_eq!(led.states[2], script.states[2], "unmoved");
    assert_eq!(led.states[4], script.states[4], "unmoved");
}

#[test]
fn a_capture_lead_past_the_end_releases_the_whole_script() {
    let script = Script::parse("2 cross\n").expect("parses");
    let led = script.with_capture_lead(9);
    assert_eq!(led.len(), 2);
    assert!(led.states.iter().all(|state| *state == State::default()));
}

#[test]
fn a_zero_capture_lead_changes_nothing() {
    let script = Script::parse("3 cross right\n2 none\n").expect("parses");
    assert_eq!(script.clone().with_capture_lead(0), script);
}

#[test]
fn a_script_round_trips_through_its_own_text() {
    let text = "60 cross\n30 cross left\n1 none\n4 l r stick_x=0.5\n";
    let script = Script::parse(text).expect("parses");
    let again = Script::parse(&script.to_text()).expect("re-parses");
    assert_eq!(script, again);
}

#[test]
fn a_short_script_holds_its_last_state() {
    let script = Script::parse("2 cross\n").expect("parses");
    assert_eq!(script.at(0), script.at(99));
    assert!(script.at(99).is_held(Button::Cross));
}

#[test]
fn an_empty_script_holds_nothing() {
    let script = Script::parse("# nothing but a comment\n").expect("parses");
    assert!(script.is_empty());
    assert_eq!(script.at(0), State::default());
}

#[test]
fn a_missing_or_zero_count_is_an_error_with_a_line_number() {
    assert!(matches!(
        Script::parse("cross\n"),
        Err(ParseError::Count { line: 1, .. })
    ));
    assert!(matches!(
        Script::parse("2 cross\n0 cross\n"),
        Err(ParseError::Count { line: 2, .. })
    ));
}

#[test]
fn an_unknown_token_is_an_error() {
    assert!(matches!(
        Script::parse("1 thrust\n"),
        Err(ParseError::Token { line: 1, .. })
    ));
    assert!(matches!(
        Script::parse("1 wobble=1\n"),
        Err(ParseError::Token { line: 1, .. })
    ));
}

/// The synthetic "any button" bit is computed, not authored.
#[test]
fn the_any_button_is_not_scriptable() {
    assert!(matches!(
        Script::parse("1 any\n"),
        Err(ParseError::Token { line: 1, .. })
    ));
}

#[test]
fn an_out_of_range_axis_is_rejected_rather_than_clamped() {
    assert!(matches!(
        Script::parse("1 stick_x=2\n"),
        Err(ParseError::Range { line: 1, .. })
    ));
    assert!(matches!(
        Script::parse("1 airbrake_left=-1\n"),
        Err(ParseError::Range { line: 1, .. })
    ));
}

#[test]
fn a_non_numeric_or_repeated_axis_is_an_error() {
    assert!(matches!(
        Script::parse("1 stick_x=hard\n"),
        Err(ParseError::Value { line: 1, .. })
    ));
    assert!(matches!(
        Script::parse("1 stick_x=nan\n"),
        Err(ParseError::Value { line: 1, .. })
    ));
    assert!(matches!(
        Script::parse("1 stick_x=0.1 stick_x=0.2\n"),
        Err(ParseError::Duplicate { line: 1, .. })
    ));
}

#[test]
fn none_cannot_share_a_line() {
    assert!(matches!(
        Script::parse("1 none cross\n"),
        Err(ParseError::NotAlone { line: 1, .. })
    ));
}

#[test]
fn a_runaway_repeat_count_is_refused() {
    assert!(matches!(
        Script::parse("99999999 cross\n"),
        Err(ParseError::TooLong { line: 1 })
    ));
}

/// The committed example scripts are part of the deliverable, so they are
/// checked here rather than only by whoever next runs a capture.
#[test]
fn the_committed_scenarios_parse_and_are_two_hundred_ticks() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../verification/scenarios/");
    for name in ["straight-line.inputs", "steer-both-ways.inputs"] {
        let text = std::fs::read_to_string(format!("{root}{name}"))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let script = Script::parse(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(script.len(), 200, "{name}");
        assert!(
            script.states.iter().all(|s| s.is_held(Button::Cross)),
            "{name}: both scenarios hold thrust throughout"
        );
    }
}

/// Every committed scenario parses, whatever it is for.
///
/// The named check above is about the two 200-tick protocol scenarios
/// specifically. This one is the guard that matters as the directory grows:
/// a scenario that does not parse is a scenario nobody can replay, and the
/// whole-lap recording is 3,146 ticks of run-length lines that no human
/// proof-read.
#[test]
fn every_committed_scenario_parses() {
    let root = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../verification/scenarios/"
    ));
    let mut seen = 0;
    for entry in std::fs::read_dir(root).expect("the scenarios directory is committed") {
        let path = entry.expect("a readable entry").path();
        if path.extension().is_none_or(|e| e != "inputs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a readable scenario");
        let script = Script::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(!script.is_empty(), "{}: no ticks", path.display());
        seen += 1;
    }
    assert!(seen >= 6, "only {seen} scenario(s) found");
}

/// The whole-lap recording, which is a different kind of artefact from the
/// hand-authored scenarios: it is what a closed-loop autopilot actually sent
/// the emulator over a lap the game itself timed. Length and thrust are
/// asserted because both are load-bearing - a truncated recording would
/// still parse, and a recording that had stopped holding thrust would be a
/// coast rather than a lap.
#[test]
fn the_whole_lap_scenario_is_a_lap_of_thrust() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../verification/scenarios/talons-junction-time-trial-lap.inputs"
    ))
    .expect("the lap scenario is committed");
    let script = Script::parse(&text).expect("parses");
    assert_eq!(script.len(), 3146);
    let thrusting = script
        .states
        .iter()
        .filter(|s| s.is_held(Button::Cross))
        .count();
    assert!(
        thrusting * 20 > script.len() * 19,
        "{thrusting} of {} tick(s) hold thrust",
        script.len()
    );
}

/// A script's whole reason to exist is that the emulator and our replay read
/// the *same* file, so the two parsers must agree tick for tick. This is the
/// Rust half of that; `scripts/input_script.py`'s self-test is the other.
#[test]
fn the_expanded_dump_is_one_line_per_tick_plus_a_header() {
    let script = Script::parse("2 cross\n1 left\n").expect("parses");
    let dump = script.to_expanded();
    assert_eq!(dump.lines().count(), 4);
    assert!(dump.lines().nth(1).is_some_and(|l| l.starts_with("0,0x")));
    assert!(
        dump.lines().nth(3).is_some_and(|l| l.contains("-1.000000")),
        "{dump}"
    );
}
