#!/usr/bin/env python3
"""Capture a per-tick trace of ship state out of Wipeout Pulse running in PCSX2.

The PS2 sibling of `scripts/psp-trace.py`. PPSSPP's capture breaks on
`Ship_UpdateCraft` and reads a register; PCSX2's PINE transport has no
breakpoint and no register read at all
(`docs/reverse-engineering/pcsx2-debugger.md`), so this drives PCSX2's own
**verified** frame-advance instead - `scripts/pcsx2-drive.py frames 1`, once
per tick, which that page measures as bit-exact and pixel-identical across
repeated runs from the same savestate. That is a stronger determinism
guarantee than PPSSPP's breakpoint gives, at a similar cost: about three
verified frames a second, so a few hundred ticks is a few minutes.

Two things this script cannot yet give you, and says so on every capture
rather than inventing them - see `scripts/pcsx2_trace_fields.py` for the
citations:

- **`--craft` is required.** No known static memory location holds the
  current race's craft pointer, so there is nothing to autodetect the way
  PPSSPP's breakpoint hands one over for free. Find it once per savestate (it
  is a heap address, fixed for as long as that particular savestate is what
  every capture starts from - PCSX2's `loadstate` is bit-exact) and pass it
  every time.
- **`steer` and `brake` are always written as `0.0`, and `dt` is always a
  fixed `1/50`.** Neither ramped control's craft offset has been located in
  the PS2 corpus, and whether the game measures a real frame duration or
  integrates a fixed PAL step has not been checked either. A capture taken
  with this script cannot be used to verify steering or braking, and is
  measuring `dt` against an assumption, not a memory read. Both facts are
  written into the CSV's own header comment as well as printed here, because
  a CSV outlives the terminal it came from.

    python3 scripts/pcsx2-trace.py --craft 0x0a1b2c00 --from-state 1 \\
        --script verification/scenarios/steer-both-ways.inputs \\
        --out data/traces/ps2-steer-both-ways.csv

Prerequisite: PCSX2 booted and sitting on the grid, with a savestate already
saved there (`python3 scripts/pcsx2-drive.py boot ...`, drive to the grid by
hand or script, then `python3 scripts/pcsx2-drive.py state save 1`). This
script does not boot anything itself - see `pcsx2-drive.py`'s own module
docstring for that half.
"""

import argparse
import math
import struct
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import input_script
from pcsx2_pine import Pine, PineError
from pcsx2_trace_fields import (
    BODY_FIELDS,
    BODY_POINTER,
    CONTROLS_FIELDS,
    CONTROLS_POINTER,
    CRAFT_FIELDS,
)

DRIVE_SCRIPT = Path(__file__).resolve().parent / "pcsx2-drive.py"

# `l`/`r` (input_script.py's names, shared with the PSP capture) to
# pcsx2-drive.py's own button vocabulary, which spells the shoulders `l1`/`r1`
# rather than `l`/`r`. Every other name is identical on both sides.
DRIVE_BUTTON_NAME = {"l": "l1", "r": "r1"}

# PS2 Pulse boots PAL - confirmed live this session (`UpdateVSyncRate: Mode
# Changed to PAL`, `Frame rate: 50`) - which is the only evidence behind this
# constant. It is a stated assumption about what `dt` the *game* integrates,
# not a measurement of it; see the module docstring.
ASSUMED_DT = 1.0 / 50.0

# Byte lengths of the three blocks this script reads in one `read_bytes` call
# each - past the highest offset any of CRAFT_FIELDS / BODY_FIELDS /
# CONTROLS_FIELDS names, rounded up.
CRAFT_BLOCK_LEN = 0x330
BODY_BLOCK_LEN = 0x170
CONTROLS_BLOCK_LEN = 0x48

# Fields that are integers (flags, a mode enum, a button mask) rather than
# floats. Written to the CSV as their integer value cast to float, the same
# way `psp-trace.py` writes `grounded`/`controller_class`-shaped fields: the
# `%.7g` formatting is uniform across every column regardless of what the
# original type was.
INT_FIELDS = {"grounded_flags", "mode", "controls_buttons"}

REQUIRED_ORDER = [
    "tick", "dt", "grounded", "throttle", "brake", "steer",
    "airbrake_l", "airbrake_r", "speed_cached",
    "right_x", "right_y", "right_z",
    "up_x", "up_y", "up_z",
    "fwd_x", "fwd_y", "fwd_z",
    "pos_x", "pos_y", "pos_z",
    "vel_x", "vel_y", "vel_z",
    "speed",
    "avel_x", "avel_y", "avel_z",
    "omega_x", "omega_y", "omega_z",
]

HEADER_COMMENT = (
    "# pcsx2-trace.py: steer and brake are always 0.0 (the ramped control "
    "state's craft offset is not yet located in the PS2 corpus); dt is a "
    "fixed 1/50 (PAL), assumed rather than read; speed is |velocity| computed "
    "at write time, not a memory read. See scripts/pcsx2_trace_fields.py."
)


def unpack_block(block, fields):
    """`{name: float}` for a `[(name, offset)]` table read out of `block`."""
    values = {}
    for name, offset in fields:
        if name in INT_FIELDS:
            (raw,) = struct.unpack_from("<I", block, offset)
            values[name] = float(raw)
        else:
            (raw,) = struct.unpack_from("<f", block, offset)
            values[name] = raw
    return values


def drive_buttons(state):
    """`state.held_names()`, translated to `pcsx2-drive.py`'s own spelling."""
    return [DRIVE_BUTTON_NAME.get(name, name) for name in state.held_names()]


def step_one_frame(pine_slot, counter, buttons, from_state=None):
    """One verified frame, via `pcsx2-drive.py frames 1`. Raises on failure.

    Shelling out rather than importing: `pcsx2-drive.py`'s hyphen makes it
    unimportable (the same reason `psp_trace_fields.py`'s own docstring
    gives), and its `Keyboard`/`advance_frames` machinery is not split into
    an importable sibling the way `pcsx2_pine.py` is. This calls its `frames`
    subcommand exactly the way a human would, which means every capture uses
    the identical, already-measured-deterministic code path - no second
    implementation of frame-stepping to drift from the first.
    """
    command = [
        sys.executable, str(DRIVE_SCRIPT), "--slot", str(pine_slot),
        "frames", "1", "--counter", "0x%x" % counter,
    ]
    if buttons:
        command.append("--hold")
        command.extend(buttons)
    if from_state is not None:
        command.extend(["--from-state", str(from_state)])
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(
            "pcsx2-drive.py frames failed: %s" % (result.stderr.strip() or result.stdout.strip())
        )


def _check_ee_pointer(value, what):
    """The same sanity check `pcsx2-drive.py`'s own `read_pad` uses for
    `g_input`: EE RAM is 32 MiB at `0x00100000`, so a pointer outside that
    range means the walk landed somewhere it should not have - a wrong
    `--craft` address, a struct that moved, or a pointer read before the
    object exists."""
    if not 0x00100000 <= value < 0x02000000:
        raise PineError(
            "%s reads 0x%08x, which is not an EE RAM pointer - wrong "
            "--craft address, or the struct moved" % (what, value)
        )


def read_tick(pine, craft, tick, dt):
    craft_block = pine.read_bytes(craft, CRAFT_BLOCK_LEN)
    craft_values = unpack_block(craft_block, CRAFT_FIELDS)

    (body_ptr,) = struct.unpack_from("<I", craft_block, BODY_POINTER)
    _check_ee_pointer(body_ptr, "craft+0x%x (body pointer)" % BODY_POINTER)
    body_values = unpack_block(pine.read_bytes(body_ptr, BODY_BLOCK_LEN), BODY_FIELDS)

    (controls_ptr,) = struct.unpack_from("<I", craft_block, CONTROLS_POINTER)
    _check_ee_pointer(controls_ptr, "craft+0x%x (controls pointer)" % CONTROLS_POINTER)
    controls_values = unpack_block(
        pine.read_bytes(controls_ptr, CONTROLS_BLOCK_LEN), CONTROLS_FIELDS
    )

    speed = math.sqrt(
        body_values["vel_x"] ** 2 + body_values["vel_y"] ** 2 + body_values["vel_z"] ** 2
    )

    row = {
        "tick": float(tick),
        "dt": dt,
        "grounded": craft_values["grounded"],
        "throttle": controls_values["throttle"],
        "brake": 0.0,
        "steer": 0.0,
        "airbrake_l": craft_values["airbrake_l"],
        "airbrake_r": craft_values["airbrake_r"],
        "speed_cached": craft_values["speed_cached"],
        "speed": speed,
    }
    for name in ("right_x", "right_y", "right_z", "up_x", "up_y", "up_z",
                 "fwd_x", "fwd_y", "fwd_z", "pos_x", "pos_y", "pos_z",
                 "avel_x", "avel_y", "avel_z", "omega_x", "omega_y", "omega_z"):
        row[name] = body_values[name]
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--slot", type=int, default=28011, help="PINE slot")
    parser.add_argument(
        "--craft", required=True, type=lambda v: int(v, 0), metavar="ADDRESS",
        help="the craft's EE RAM address for this savestate - see "
             "scripts/pcsx2_trace_fields.py for how to find one",
    )
    parser.add_argument(
        "--from-state", type=int, default=None, metavar="SLOT",
        help="savestate slot to anchor tick 0 from (pauses, loads, re-pauses "
             "before the first step - see pcsx2-drive.py's own `anchor()`)",
    )
    parser.add_argument("--hold", action="append", default=[], metavar="BUTTON",
                        help="hold a button for the whole capture")
    parser.add_argument("--script", type=Path, metavar="FILE",
                        help="drive the capture from a committed input script "
                             "(scripts/input_script.py). Same file `oag-trace "
                             "run --script` replays.")
    parser.add_argument("--ticks", type=int, metavar="N",
                        help="defaults to the script's own length, or 300 without one")
    parser.add_argument("--warmup", type=int, default=0, metavar="TICKS",
                        help="verified-step this many ticks, holding --warmup-hold, "
                             "before tick 0 is recorded")
    parser.add_argument("--warmup-hold", action="append", default=[], metavar="BUTTON")
    parser.add_argument("--counter", type=lambda v: int(v, 0), default=None,
                        help="override the frame-counter address pcsx2-drive.py verifies "
                             "against; defaults to its own FRAME_COUNTER")
    parser.add_argument("--out", type=Path, help="write here instead of stdout")
    args = parser.parse_args()

    if args.script and args.hold:
        parser.error("--hold and --script cannot both drive the capture; "
                     "--warmup-hold is what holds something during --warmup")
    if args.warmup_hold and not args.warmup:
        parser.error("--warmup-hold with no --warmup does nothing")

    states = []
    if args.script:
        try:
            states = input_script.load(args.script)
        except input_script.ScriptError as error:
            parser.error(str(error))
        if not states:
            parser.error("%s: the script has no ticks in it" % args.script)
        for reason in input_script.unrepresentable(states):
            parser.error("%s cannot be sent to PCSX2 - %s" % (args.script, reason))
        for tick, state in enumerate(states):
            if state.explicit & {"stick_x", "stick_y"}:
                parser.error(
                    "%s tick %d sets an explicit stick axis; this harness has "
                    "no analog input path yet, only the digital pad bindings "
                    "pcsx2-drive.py's [Pad1] section carries" % (args.script, tick)
                )

    ticks = args.ticks if args.ticks is not None else (len(states) or 300)

    print("WARNING: steer and brake will be recorded as 0.0 (not yet "
          "located on the PS2); dt is a fixed 1/50, assumed rather than "
          "measured. See scripts/pcsx2_trace_fields.py.", file=sys.stderr)

    # `pcsx2-drive.py`'s own `FRAME_COUNTER` default, duplicated as a literal
    # rather than imported - the hyphenated filename cannot be imported (see
    # the module docstring), and this is the one constant worth a literal
    # copy rather than a second module for.
    counter = args.counter if args.counter is not None else 0x0027A7E8

    out = open(args.out, "w") if args.out else sys.stdout
    try:
        print(HEADER_COMMENT, file=out)
        print(",".join(REQUIRED_ORDER), file=out)

        buttons = [DRIVE_BUTTON_NAME.get(b, b) for b in args.warmup_hold]
        for i in range(args.warmup):
            step_one_frame(args.slot, counter, buttons,
                           from_state=args.from_state if i == 0 else None)
        anchor_state = args.from_state if args.warmup == 0 else None

        pine = Pine(slot=args.slot)
        for tick in range(ticks):
            if args.script:
                state = input_script.at(states, tick)
                buttons = drive_buttons(state)
            else:
                buttons = [DRIVE_BUTTON_NAME.get(b, b) for b in args.hold]
            step_one_frame(args.slot, counter, buttons,
                           from_state=anchor_state if tick == 0 else None)
            row = read_tick(pine, args.craft, tick, ASSUMED_DT)
            print(",".join("%.7g" % row[name] for name in REQUIRED_ORDER), file=out)
            if tick % 50 == 0:
                print("tick %d/%d" % (tick, ticks), file=sys.stderr)
    except (PineError, RuntimeError) as exc:
        print("error at tick: %s" % exc, file=sys.stderr)
        return 1
    finally:
        if args.out:
            out.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
