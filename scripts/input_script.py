#!/usr/bin/env python3
"""The committed input-script format, on the emulator side.

An input script is a plain-text, per-tick controller state, committed to the
repository and fed **identically** into the emulator capture
(`scripts/psp-trace.py --script`) and into our own replay
(`oag-trace run --script`). It is the third input mode
`docs/reverse-engineering/verification-protocol.md` asks for, and the only one
that is independently authored: `--hold` is one constant input, and `oag-trace`'s
`FromTrace` replays whatever a particular capture happened to record, so neither
can be written before a capture exists or reused against a second one.

Scripts are **authored input, not derived game data**, so unlike traces they are
committed. Nothing in one comes off a disc. They live in `verification/scenarios/`.

    60  cross            # thrust for a second
    30  cross left       # then half a second of left
    110 cross            # then straight to the end

The first field is how many consecutive ticks the state lasts. The rest are
button names (`cross`, `circle`, `square`, `triangle`, `up`, `down`, `left`,
`right`, `l`, `r`, `start`, `select`), axis assignments (`stick_x=`, `stick_y=`
in `-1..=1`, `airbrake_left=`, `airbrake_right=` in `0..=1`), or `none` / `-` on
a line of its own. `#` starts a comment.

**This is a second implementation of a format `crates/trace/src/script.rs` also
parses**, which is the price of driving two runtimes from one file. What keeps
them from drifting is that both can dump one line per tick in the same shape:

    diff <(cargo run -q -p oag-trace -- script F --expand) \\
         <(uv run scripts/input_script.py F --expand)

`--self-test` checks the parser's own invariants without needing either.
"""

import argparse
import sys
from pathlib import Path

# Abstract button indices, from `Input_BuildState`. The same table
# `oag_gameplay::input::button` holds, and the same one `crates/trace/src/script.rs`
# calls SCRIPTABLE - in the order both emit. The synthetic "any" index 0x14 is
# deliberately absent: it is derived from the others, not a thing a pad has.
BUTTONS = [
    ("cross", 5),
    ("circle", 4),
    ("square", 7),
    ("triangle", 6),
    ("up", 0),
    ("down", 1),
    ("left", 2),
    ("right", 3),
    ("l", 8),
    ("r", 9),
    ("start", 14),
    ("select", 15),
]
BUTTON_INDEX = dict(BUTTONS)

# PPSSPP's own button names, for `input.buttons.send`. Identical to ours for
# every button a script may name, which is not a coincidence - the abstract layer
# was named after the same hardware - but written out rather than assumed, so a
# future divergence is a one-line fix here instead of a silent mis-send.
PPSSPP_NAME = {
    "cross": "cross", "circle": "circle", "square": "square",
    "triangle": "triangle", "up": "up", "down": "down", "left": "left",
    "right": "right", "l": "l", "r": "r", "start": "start", "select": "select",
}

AXES = {
    "stick_x": (-1.0, 1.0),
    "stick_y": (-1.0, 1.0),
    "airbrake_left": (0.0, 1.0),
    "airbrake_right": (0.0, 1.0),
}

# A guard against a typo'd repeat count, not a meaningful limit: 1,048,576 ticks
# is nearly five hours at 60 Hz.
MAX_TICKS = 1 << 20


class ScriptError(ValueError):
    """A script would not parse. Always carries the 1-based line."""


class State:
    """One tick of scripted controller state.

    `explicit` records which axes the script assigned by name rather than
    derived from buttons, and it is not cosmetic: it is what decides whether the
    emulator side sends `input.analog.send` at all. A `left` in a script is a
    d-pad press and must be sent as one; a `stick_x=-0.5` is an analog
    deflection a d-pad cannot express and must be sent as one. Sending both for
    the same intent would be two steering inputs where the script asked for one.
    """

    __slots__ = ("buttons", "stick_x", "stick_y", "airbrake_left",
                 "airbrake_right", "explicit")

    def __init__(self, buttons=0, stick_x=0.0, stick_y=0.0,
                 airbrake_left=0.0, airbrake_right=0.0, explicit=frozenset()):
        self.buttons = buttons
        self.stick_x = stick_x
        self.stick_y = stick_y
        self.airbrake_left = airbrake_left
        self.airbrake_right = airbrake_right
        self.explicit = explicit

    def is_held(self, index):
        return self.buttons & (1 << (index & 0x1F)) != 0

    def held_names(self):
        """The buttons held, by name, in the table's order."""
        return [name for name, index in BUTTONS if self.is_held(index)]

    @classmethod
    def from_buttons(cls, buttons, explicit=frozenset()):
        """The state a set of held buttons alone describes.

        The axis derivation is `oag_input::Input::snapshot`'s - d-pad
        right-minus-left, up-minus-down, and the shoulders as the airbrakes -
        because a scripted run has to be the same experiment as a played one.
        `crates/trace/src/script.rs::State::from_buttons` is the same function.
        """
        state = cls(buttons=buttons, explicit=explicit)

        def axis(positive, negative):
            return float(state.is_held(positive)) - float(state.is_held(negative))

        state.stick_x = axis(BUTTON_INDEX["right"], BUTTON_INDEX["left"])
        state.stick_y = axis(BUTTON_INDEX["up"], BUTTON_INDEX["down"])
        state.airbrake_left = float(state.is_held(BUTTON_INDEX["l"]))
        state.airbrake_right = float(state.is_held(BUTTON_INDEX["r"]))
        return state

    def key(self):
        """Everything that decides what gets sent to the emulator."""
        return (self.buttons, self.stick_x, self.stick_y,
                self.airbrake_left, self.airbrake_right)

    def __eq__(self, other):
        return isinstance(other, State) and self.key() == other.key()

    def __repr__(self):
        return "State(%s)" % " ".join(
            self.held_names()
            + ["%s=%g" % (a, getattr(self, a)) for a in sorted(self.explicit)]
        ) or "State(none)"


def parse(text):
    """Parses a script into one `State` per tick.

    Raises `ScriptError` with a line number. Rejects rather than clamps an
    out-of-range axis: a pad is clamped because a miscalibration is not the
    player's fault, but a script is a specification and a specification that
    says 2.0 is wrong rather than saturated.
    """
    states = []
    for number, raw in enumerate(text.splitlines(), start=1):
        body = raw.split("#", 1)[0].strip()
        if not body:
            continue
        head, *tokens = body.split()
        try:
            count = int(head)
        except ValueError:
            count = 0
        if count <= 0:
            raise ScriptError("line %d: %r is not a positive tick count" % (number, head))
        if len(states) + count > MAX_TICKS:
            raise ScriptError("line %d: the script expands past %d ticks" % (number, MAX_TICKS))
        states.extend([_parse_state(tokens, number)] * count)
    return states


def _parse_state(tokens, line):
    buttons = 0
    overrides = {}
    nothing = None
    for token in tokens:
        if token in ("none", "-"):
            nothing = token
            continue
        if "=" in token:
            name, _, value = token.partition("=")
            if name not in AXES:
                raise ScriptError(
                    "line %d: %r is not a button name or an axis assignment. "
                    "Buttons: %s. Axes: %s"
                    % (line, token, ", ".join(n for n, _ in BUTTONS), ", ".join(AXES))
                )
            if name in overrides:
                raise ScriptError("line %d: %s is assigned twice" % (line, name))
            try:
                parsed = float(value)
            except ValueError:
                raise ScriptError("line %d: %s = %r is not a number" % (line, name, value)) from None
            if parsed != parsed or parsed in (float("inf"), float("-inf")):
                raise ScriptError("line %d: %s = %r is not a number" % (line, name, value))
            low, high = AXES[name]
            if not low <= parsed <= high:
                raise ScriptError(
                    "line %d: %s = %g is outside %g..=%g" % (line, name, parsed, low, high)
                )
            overrides[name] = parsed
            continue
        if token not in BUTTON_INDEX:
            raise ScriptError(
                "line %d: %r is not a button name or an axis assignment. "
                "Buttons: %s. Axes: %s"
                % (line, token, ", ".join(n for n, _ in BUTTONS), ", ".join(AXES))
            )
        buttons |= 1 << (BUTTON_INDEX[token] & 0x1F)

    if nothing is not None and (buttons or overrides):
        raise ScriptError(
            "line %d: %r means nothing is held, so it cannot share a line" % (line, nothing)
        )
    state = State.from_buttons(buttons, explicit=frozenset(overrides))
    for name, value in overrides.items():
        setattr(state, name, value)
    return state


def at(states, tick):
    """The state at a tick.

    A script shorter than the run it drives **holds its last state**, rather
    than releasing everything or wrapping around: releasing would put a
    deceleration into the middle of a capture that the script never asked for,
    and wrapping would put a discontinuity there. Callers warn when it happens,
    because an over-short script is nearly always a mistake.
    """
    if not states:
        return State()
    return states[tick] if tick < len(states) else states[-1]


def expanded(states):
    """One line per tick, byte-identical to `oag-trace script --expand`.

    Two parsers in two languages is the price of driving two runtimes from one
    file; diffing this against the Rust dump is what stops them drifting.
    """
    out = ["tick,buttons,stick_x,stick_y,airbrake_left,airbrake_right"]
    for tick, state in enumerate(states):
        out.append(
            "%d,0x%08x,%.6f,%.6f,%.6f,%.6f"
            % (tick, state.buttons, state.stick_x, state.stick_y,
               state.airbrake_left, state.airbrake_right)
        )
    return "\n".join(out) + "\n"


def button_payload(state):
    """`input.buttons.send`'s argument: every scriptable button, held or not.

    Every button is named on every send, not only the held ones, because the
    command sets state rather than adding to it and a release has to be said out
    loud. The shoulders carry the airbrakes: a PSP pad has no analog trigger, so
    `airbrake_left = 1` and `l` are the same hardware event, and a script that
    says either produces the same capture.
    """
    payload = {PPSSPP_NAME[name]: state.is_held(index) for name, index in BUTTONS}
    payload[PPSSPP_NAME["l"]] = payload[PPSSPP_NAME["l"]] or state.airbrake_left > 0.0
    payload[PPSSPP_NAME["r"]] = payload[PPSSPP_NAME["r"]] or state.airbrake_right > 0.0
    return payload


def analog_payload(state):
    """`input.analog.send`'s x and y, or `None` if the script asked for no analog.

    Only *explicit* stick assignments are sent. The stick values a d-pad press
    derives are for our own side, which has no d-pad; sending both here would be
    two steering inputs where the script asked for one.
    """
    if not state.explicit & {"stick_x", "stick_y"}:
        return None
    return (
        state.stick_x if "stick_x" in state.explicit else 0.0,
        state.stick_y if "stick_y" in state.explicit else 0.0,
    )


def unrepresentable(states):
    """The reasons a script cannot be sent to a PSP, if any.

    One reason so far: a **fractional airbrake**. The force law ramps each side
    toward its analog input and on anything but a PSP that input need not be 0 or
    1, so the format allows it - but PSP hardware has two shoulder *buttons* and
    the emulator can only press or release them. Said out loud rather than
    quietly rounded, because a rounded input makes a capture that silently is not
    the script's.
    """
    reasons = []
    for tick, state in enumerate(states):
        for side in ("airbrake_left", "airbrake_right"):
            value = getattr(state, side)
            if value not in (0.0, 1.0):
                reasons.append(
                    "tick %d: %s = %g, and a PSP shoulder is a button" % (tick, side, value)
                )
    return reasons


def load(path):
    """Parses a script file, raising `ScriptError` prefixed with its name."""
    text = Path(path).read_text(encoding="utf-8")
    try:
        return parse(text)
    except ScriptError as error:
        raise ScriptError("%s: %s" % (path, error)) from None


def _self_test():
    """The invariants, without needing a Rust toolchain or an emulator."""
    states = parse("3 cross\n2 none\n")
    assert len(states) == 5, states
    assert states[2].is_held(BUTTON_INDEX["cross"])
    assert states[3] == State()

    states = parse("1 left\n1 right\n1 up\n1 down\n1 l\n1 r\n")
    assert [s.stick_x for s in states[:2]] == [-1.0, 1.0]
    assert [s.stick_y for s in states[2:4]] == [1.0, -1.0]
    assert states[4].airbrake_left == 1.0 and states[5].airbrake_right == 1.0

    states = parse("1 left stick_x=-0.25\n")
    assert states[0].stick_x == -0.25
    assert states[0].is_held(BUTTON_INDEX["left"]), "the button is still held"
    assert analog_payload(states[0]) == (-0.25, 0.0)
    assert analog_payload(parse("1 left\n")[0]) is None, "a d-pad press is not analog"

    assert button_payload(parse("1 airbrake_left=1\n")[0])["l"] is True
    assert unrepresentable(parse("1 airbrake_left=0.5\n")), "a fractional airbrake"
    assert not unrepresentable(parse("1 l\n"))

    for bad in ("cross\n", "0 cross\n", "1 thrust\n", "1 any\n", "1 wobble=1\n",
                "1 stick_x=2\n", "1 airbrake_left=-1\n", "1 stick_x=hard\n",
                "1 stick_x=nan\n", "1 stick_x=0.1 stick_x=0.2\n", "1 none cross\n",
                "99999999 cross\n"):
        try:
            parse(bad)
        except ScriptError:
            continue
        raise AssertionError("%r parsed and should not have" % bad)

    states = parse("2 cross\n")
    assert at(states, 99) == states[-1], "a short script holds its last state"
    assert at([], 0) == State()

    root = Path(__file__).resolve().parent.parent / "verification" / "scenarios"
    for name in ("straight-line.inputs", "steer-both-ways.inputs"):
        committed = load(root / name)
        assert len(committed) == 200, "%s: %d ticks" % (name, len(committed))
        assert all(s.is_held(BUTTON_INDEX["cross"]) for s in committed), name
        assert not unrepresentable(committed), name
    print("input_script.py: self-test passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("script", nargs="?", type=Path)
    parser.add_argument("--expand", action="store_true",
                        help="one line per tick, to diff against `oag-trace script --expand`")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        _self_test()
        return
    if args.script is None:
        parser.error("a script, or --self-test")
    try:
        states = load(args.script)
    except ScriptError as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from None
    if args.expand:
        sys.stdout.write(expanded(states))
        return
    print("%d tick(s)" % len(states))
    for reason in unrepresentable(states):
        print("cannot be sent to a PSP - %s" % reason, file=sys.stderr)


if __name__ == "__main__":
    main()
