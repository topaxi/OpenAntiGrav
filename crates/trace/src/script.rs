//! Input scripts: a plain-text, committed, per-tick controller state.
//!
//! The third input mode the [verification
//! protocol](../../../docs/reverse-engineering/verification-protocol.md) asks for
//! and the only one that is *independently authored*. The other two are each
//! tied to something else:
//!
//! - [`Inputs::FromTrace`](crate::replay::Inputs::FromTrace) replays whatever a
//!   particular capture happened to record, so it cannot be reused against a
//!   second capture and cannot be written before one exists.
//! - [`Inputs::Held`](crate::replay::Inputs::Held) is one constant input for a
//!   whole run, which is enough for a straight and nothing else.
//!
//! A script is neither. It is a file, committed to the repository, fed
//! *identically* into the emulator capture (`scripts/psp-trace.py --script`) and
//! into our own replay (`oag-trace run --script`), so that the two runs are
//! driven by the same authored intent rather than by each other.
//!
//! Scripts are **authored input, not derived game data**, so unlike traces they
//! are committed. Nothing in one comes off a disc: it is a list of button names
//! somebody chose.
//!
//! # The format
//!
//! One run-length-encoded line per state, `#` to end of line is a comment:
//!
//! ```text
//! # Thrust for a second, then turn left for half of one, then straight again.
//! 60  cross
//! 30  cross left
//! 110 cross
//! ```
//!
//! The first field is how many consecutive ticks the state lasts, and must be a
//! positive integer. The rest are tokens, in any order:
//!
//! | Token | Meaning |
//! | --- | --- |
//! | `cross`, `circle`, `square`, `triangle` | Face buttons |
//! | `up`, `down`, `left`, `right` | D-pad |
//! | `l`, `r` | Shoulders |
//! | `start`, `select` | Both, for menu work |
//! | `stick_x=`, `stick_y=` | Analog stick, `-1..=1` |
//! | `airbrake_left=`, `airbrake_right=` | Airbrakes, `0..=1` |
//! | `none`, `-` | Nothing held. Only token on its line |
//!
//! Run-length encoding rather than one line per tick because a capture is
//! hundreds of ticks long and the interesting thing about a script is where the
//! state *changes*. `1 cross` is a one-tick press, so nothing is lost.
//!
//! # Why the axes have defaults instead of being required
//!
//! A script names buttons because that is what the emulator side can send: PSP
//! hardware has a d-pad and two shoulder buttons, and `input.buttons.send` takes
//! names. Our own side consumes an [`InputSnapshot`], which has axes. The
//! translation between them is not invented here - it is exactly what
//! `oag_input::Input::snapshot` does for a keyboard, and reproducing it is what
//! makes a scripted run and a real one the same thing:
//!
//! - `stick_x` is `right - left`, `stick_y` is `up - down`.
//! - `airbrake_left` is 1 while `l` is held, `airbrake_right` while `r` is.
//!
//! An explicit assignment overrides the derived value, which is how a script
//! expresses something a PSP pad cannot produce - a half-deflected stick. The
//! emulator side sends those through `input.analog.send`; a *fractional airbrake*
//! has no hardware equivalent at all and `psp-trace.py` says so rather than
//! quietly rounding it.
//!
//! [`InputSnapshot`]: oag_gameplay::input::InputSnapshot

use std::fmt;

use oag_gameplay::input::button;

/// The most ticks a script may expand to.
///
/// A guard against a typo'd repeat count allocating the machine's memory, not a
/// meaningful limit: 1,048,576 ticks is nearly five hours of a 60 Hz race.
pub const MAX_TICKS: usize = 1 << 20;

/// The buttons a script may name, in the order [`Script::to_text`] emits them.
///
/// [`button::ANY`] is deliberately absent: it is synthesised from the others by
/// [`oag_gameplay::input::Input::begin_frame`] and is not a thing a pad has.
const SCRIPTABLE: [(&str, u8); 12] = [
    ("cross", button::CROSS),
    ("circle", button::CIRCLE),
    ("square", button::SQUARE),
    ("triangle", button::TRIANGLE),
    ("up", button::UP),
    ("down", button::DOWN),
    ("left", button::LEFT),
    ("right", button::RIGHT),
    ("l", button::L),
    ("r", button::R),
    ("start", button::START),
    ("select", button::SELECT),
];

/// One tick of scripted controller state.
///
/// The same five quantities [`oag_gameplay::input::InputSnapshot`] carries, minus
/// the button *edges*, which are not state a script can author: whether a button
/// was pressed this tick follows from the previous tick's state and is computed
/// by [`oag_gameplay::input::Input::begin_frame`] during the run.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct State {
    /// Abstract button indices, as a mask. See [`oag_gameplay::input::button`].
    pub buttons: u32,
    /// Analog stick X, `-1..=1`, positive right.
    pub stick_x: f32,
    /// Analog stick Y, `-1..=1`, positive up.
    pub stick_y: f32,
    /// Left airbrake, `0..=1`.
    pub airbrake_left: f32,
    /// Right airbrake, `0..=1`.
    pub airbrake_right: f32,
}

impl State {
    /// Whether an abstract button index is held.
    #[must_use]
    pub fn is_held(&self, index: u8) -> bool {
        self.buttons & (1u32 << (index & 0x1f)) != 0
    }

    /// The state a set of held buttons alone describes.
    ///
    /// The axis derivation is `oag_input::Input::snapshot`'s, reproduced rather
    /// than imported because no gameplay-side crate may depend on the input
    /// system - rule 1 of `docs/architecture/workspace-layout.md`. The test
    /// `the_axes_follow_the_same_rule_the_input_system_uses` is what keeps the
    /// two honest.
    #[must_use]
    pub fn from_buttons(buttons: u32) -> Self {
        let mut state = Self {
            buttons,
            ..Self::default()
        };
        state.stick_x = axis(state.is_held(button::RIGHT), state.is_held(button::LEFT));
        state.stick_y = axis(state.is_held(button::UP), state.is_held(button::DOWN));
        state.airbrake_left = f32::from(u8::from(state.is_held(button::L)));
        state.airbrake_right = f32::from(u8::from(state.is_held(button::R)));
        state
    }
}

/// `+1`, `-1` or `0`, as a digital pair of directions makes an axis.
fn axis(positive: bool, negative: bool) -> f32 {
    f32::from(u8::from(positive)) - f32::from(u8::from(negative))
}

impl fmt::Display for State {
    /// One state as a script line's tokens, without its repeat count.
    ///
    /// Round-trips: parsing the output gives the same state back, which is what
    /// `a_script_round_trips_through_its_own_text` asserts. An axis is emitted
    /// only when it differs from what the buttons alone would derive, so a
    /// button-only script stays button-only.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let derived = State::from_buttons(self.buttons);
        let mut tokens: Vec<String> = SCRIPTABLE
            .iter()
            .filter(|(_, index)| self.is_held(*index))
            .map(|(name, _)| (*name).to_string())
            .collect();
        for (name, value, from_buttons) in [
            ("stick_x", self.stick_x, derived.stick_x),
            ("stick_y", self.stick_y, derived.stick_y),
            ("airbrake_left", self.airbrake_left, derived.airbrake_left),
            (
                "airbrake_right",
                self.airbrake_right,
                derived.airbrake_right,
            ),
        ] {
            if value != from_buttons {
                tokens.push(format!("{name}={value}"));
            }
        }
        if tokens.is_empty() {
            return f.write_str("none");
        }
        f.write_str(&tokens.join(" "))
    }
}

/// Why a script would not parse.
///
/// Every variant carries the 1-based line it happened on: a script is a file a
/// person edits, and "bad token" without a line number is not a diagnostic.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParseError {
    /// The repeat count is missing, not a number, or zero.
    #[error("line {line}: {token:?} is not a positive tick count")]
    Count {
        /// 1-based line number.
        line: usize,
        /// What was there instead.
        token: String,
    },
    /// A token is neither a button name nor an axis assignment.
    #[error(
        "line {line}: {token:?} is not a button name or an axis assignment. \
         Buttons: cross, circle, square, triangle, up, down, left, right, l, r, \
         start, select. Axes: stick_x, stick_y, airbrake_left, airbrake_right"
    )]
    Token {
        /// 1-based line number.
        line: usize,
        /// The offending token.
        token: String,
    },
    /// An axis was given a value that is not a finite number.
    #[error("line {line}: {axis} = {value:?} is not a number")]
    Value {
        /// 1-based line number.
        line: usize,
        /// Which axis.
        axis: String,
        /// What was written.
        value: String,
    },
    /// An axis was given a value outside the range the snapshot allows.
    ///
    /// Rejected rather than clamped. [`oag_gameplay::input::InputSnapshot::sanitised`]
    /// clamps because a miscalibrated pad is not the player's fault; a script is
    /// a specification, and a specification that says 2.0 is wrong rather than
    /// saturated.
    #[error("line {line}: {axis} = {value} is outside {low}..={high}")]
    Range {
        /// 1-based line number.
        line: usize,
        /// Which axis.
        axis: String,
        /// The value written.
        value: f32,
        /// The lowest allowed.
        low: f32,
        /// The highest allowed.
        high: f32,
    },
    /// The same axis was assigned twice on one line.
    #[error("line {line}: {axis} is assigned twice")]
    Duplicate {
        /// 1-based line number.
        line: usize,
        /// Which axis.
        axis: String,
    },
    /// `none` shared a line with something that is not nothing.
    #[error("line {line}: {token:?} means nothing is held, so it cannot share a line")]
    NotAlone {
        /// 1-based line number.
        line: usize,
        /// `none` or `-`.
        token: String,
    },
    /// The script expands past [`MAX_TICKS`].
    #[error("line {line}: the script expands past {MAX_TICKS} ticks")]
    TooLong {
        /// 1-based line number.
        line: usize,
    },
}

/// A parsed input script: one [`State`] per tick.
///
/// Held expanded rather than run-length encoded, because every consumer wants
/// "the state at tick n" and a few hundred ticks of five floats is nothing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Script {
    /// The states, in tick order.
    pub states: Vec<State>,
}

impl Script {
    /// Parses a script.
    ///
    /// # Errors
    ///
    /// [`ParseError`], with the line it happened on.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let mut states = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let body = raw.split('#').next().unwrap_or("").trim();
            if body.is_empty() {
                continue;
            }
            let mut tokens = body.split_whitespace();
            let head = tokens.next().unwrap_or_default();
            let count: usize = head
                .parse()
                .ok()
                .filter(|count| *count > 0)
                .ok_or_else(|| ParseError::Count {
                    line,
                    token: head.to_string(),
                })?;
            if states.len() + count > MAX_TICKS {
                return Err(ParseError::TooLong { line });
            }
            let state = parse_state(tokens, line)?;
            states.extend(std::iter::repeat_n(state, count));
        }
        Ok(Self { states })
    }

    /// How many ticks the script covers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Whether the script covers no ticks at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// The state at a tick.
    ///
    /// **A script shorter than the run it drives holds its last state**, rather
    /// than releasing everything or wrapping around. Releasing would put a
    /// deceleration into the middle of a comparison that the script never asked
    /// for; wrapping would put a discontinuity there. Both callers say so on
    /// stderr when it happens, because an over-short script is nearly always a
    /// mistake rather than an intent.
    ///
    /// An empty script is [`State::default`] - nothing held - at every tick.
    #[must_use]
    pub fn at(&self, tick: usize) -> State {
        match self.states.last() {
            Some(last) => *self.states.get(tick).unwrap_or(last),
            None => State::default(),
        }
    }

    /// The script as text, run-length encoded, in the format [`Self::parse`] reads.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        let mut states = self.states.iter().peekable();
        while let Some(state) = states.next() {
            let mut count = 1;
            while states.peek() == Some(&state) {
                states.next();
                count += 1;
            }
            out.push_str(&format!("{count} {state}\n"));
        }
        out
    }

    /// One line per tick, for cross-checking two implementations of the format.
    ///
    /// `scripts/input_script.py --expand` writes the same thing, so
    /// `diff <(oag-trace script --expand f) <(... --expand f)` is a proof that
    /// the emulator side and our side read a script identically. Two parsers in
    /// two languages is the price of driving two runtimes from one file; this is
    /// what keeps them from drifting.
    #[must_use]
    pub fn to_expanded(&self) -> String {
        let mut out = String::from("tick,buttons,stick_x,stick_y,airbrake_left,airbrake_right\n");
        for (tick, state) in self.states.iter().enumerate() {
            out.push_str(&format!(
                "{tick},0x{:08x},{:.6},{:.6},{:.6},{:.6}\n",
                state.buttons,
                state.stick_x,
                state.stick_y,
                state.airbrake_left,
                state.airbrake_right
            ));
        }
        out
    }
}

/// One line's tokens, after the repeat count.
fn parse_state<'a>(
    tokens: impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<State, ParseError> {
    let mut buttons = 0u32;
    let mut overrides: Vec<(&str, f32)> = Vec::new();
    let mut nothing: Option<&str> = None;
    let mut any = false;

    for token in tokens {
        any = true;
        if matches!(token, "none" | "-") {
            nothing = Some(token);
            continue;
        }
        if let Some((name, value)) = token.split_once('=') {
            let (low, high) = axis_range(name).ok_or_else(|| ParseError::Token {
                line,
                token: token.to_string(),
            })?;
            let parsed: f32 = value
                .parse()
                .ok()
                .filter(|v: &f32| v.is_finite())
                .ok_or_else(|| ParseError::Value {
                    line,
                    axis: name.to_string(),
                    value: value.to_string(),
                })?;
            if !(low..=high).contains(&parsed) {
                return Err(ParseError::Range {
                    line,
                    axis: name.to_string(),
                    value: parsed,
                    low,
                    high,
                });
            }
            if overrides.iter().any(|(seen, _)| *seen == name) {
                return Err(ParseError::Duplicate {
                    line,
                    axis: name.to_string(),
                });
            }
            overrides.push((name, parsed));
            continue;
        }
        let index = SCRIPTABLE
            .iter()
            .find(|(name, _)| *name == token)
            .map(|(_, index)| *index)
            .ok_or_else(|| ParseError::Token {
                line,
                token: token.to_string(),
            })?;
        buttons |= 1u32 << (index & 0x1f);
    }

    if let Some(token) = nothing
        && (buttons != 0 || !overrides.is_empty())
    {
        return Err(ParseError::NotAlone {
            line,
            token: token.to_string(),
        });
    }
    if !any {
        // A bare count with no tokens is the same statement as `none`, and
        // reading it as anything else would be a trap.
        return Ok(State::default());
    }

    let mut state = State::from_buttons(buttons);
    for (name, value) in overrides {
        match name {
            "stick_x" => state.stick_x = value,
            "stick_y" => state.stick_y = value,
            "airbrake_left" => state.airbrake_left = value,
            "airbrake_right" => state.airbrake_right = value,
            _ => unreachable!("axis_range accepted a name this does not handle"),
        }
    }
    Ok(state)
}

/// The range an axis name allows, or `None` if it is not an axis.
fn axis_range(name: &str) -> Option<(f32, f32)> {
    match name {
        "stick_x" | "stick_y" => Some((-1.0, 1.0)),
        "airbrake_left" | "airbrake_right" => Some((0.0, 1.0)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_length_line_expands_to_that_many_ticks() {
        let script = Script::parse("3 cross\n2 none\n").expect("parses");
        assert_eq!(script.len(), 5);
        assert!(script.states[2].is_held(button::CROSS));
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
            script.states[0].is_held(button::LEFT),
            "the button is still held; only the axis is overridden"
        );
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
        assert!(script.at(99).is_held(button::CROSS));
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
                script.states.iter().all(|s| s.is_held(button::CROSS)),
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
            .filter(|s| s.is_held(button::CROSS))
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
}
