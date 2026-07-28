#!/usr/bin/env python3
"""Capture a per-tick trace of ship state out of Wipeout Pulse running in PPSSPP.

This is the emulator side of M3's verification harness: the recording that
`oag-trace` compares a Rust run against. See
`docs/reverse-engineering/ppsspp-debugger.md` for how to get a PPSSPP into a
state where this can run, and `docs/reverse-engineering/verification-protocol.md`
for what the comparison is for.

The capture is breakpoint-driven, and has to be: a memory read costs about 520 ms
while the CPU is running and about 0.3 ms while it is stepping. So this breaks in
`Ship_UpdateCraft` once per tick, reads the whole craft structure in a single
`memory.read`, and resumes.

    uv run --with websocket-client scripts/psp-trace.py --ticks 600 --hold cross

There are two ways to drive the capture. `--hold` holds one input for the whole
run, which is enough for a straight line and nothing else. `--script` reads a
committed input script - `verification/scenarios/*.inputs`, see
`scripts/input_script.py` - and sends a fresh controller state on every tick, so
the capture and `oag-trace run --script <the same file>` are driven by the same
authored intent rather than by each other:

    uv run --with websocket-client scripts/psp-trace.py \\
        --script verification/scenarios/steer-both-ways.inputs --warmup 25

The output is CSV on stdout, one row per tick. It records **derived game data**
and must never be committed; write it under `data/traces/`. The *script* is not
derived data - it is a list of button names somebody chose - and is committed.
"""

import argparse
import struct
import time
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import input_script
from ppsspp_debugger import Debugger
from psp_trace_fields import BODY_FIELDS, CRAFT_FIELDS, SHIP_UPDATE_CRAFT

# The craft's own address and the two structure layouts live in
# `psp_trace_fields`, because `scripts/psp-autopilot.py` breaks on the same
# function and writes the same columns, and two copies of an offset table drift.
CRAFT_REGISTER = "a0"
CRAFT_BYTES = 0x400
BODY_POINTER = 0x1CC


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument(
        "--ticks",
        type=int,
        metavar="N",
        help="how many ticks to record. Defaults to the script's own length with "
        "--script, and to 300 without one.",
    )
    parser.add_argument(
        "--hold",
        action="append",
        default=[],
        metavar="BUTTON",
        help="hold a button for the whole capture, e.g. --hold cross (thrust)",
    )
    parser.add_argument(
        "--script",
        type=Path,
        metavar="FILE",
        help="drive the capture from a committed input script, e.g. "
        "verification/scenarios/steer-both-ways.inputs. The same file drives "
        "`oag-trace run --script`, which is the point: it is the only input mode "
        "that is authored rather than inferred, so a varying input is known on "
        "both sides. See scripts/input_script.py for the format.",
    )
    parser.add_argument(
        "--script-lead",
        type=int,
        default=0,
        metavar="N",
        help="send the script's tick k+N at the breakpoint for tick k. "
        "**Unverified, and here to be measured rather than assumed**: the "
        "breakpoint is inside the frame, so whether a state set there is seen by "
        "that frame or the next depends on where the game polls input, which has "
        "not been read out of the binary. A capture of steer-both-ways settles it "
        "in one line - the recorded `steer` column is flat until the tick the "
        "game first saw `left`, so the gap between that and the script's own tick "
        "60 is the lead to use.",
    )
    parser.add_argument(
        "--warmup-hold",
        action="append",
        default=[],
        metavar="BUTTON",
        help="hold a button during --warmup only, then hand over to --script. A "
        "scripted capture warms up holding *nothing* by default, which is what "
        "sitting through a countdown needs (thrust through it is a false start). "
        "This is for the other case: reaching the script's starting speed first, "
        "so that a scenario begins the way the reference capture did rather than "
        "from a standstill.",
    )
    parser.add_argument("--out", type=Path, help="write here instead of stdout")
    parser.add_argument(
        "--ship",
        type=int,
        default=0,
        metavar="N",
        help="which craft to follow, by order of first appearance within a tick. "
        "A time trial has one; a race has eight and updates them all from this "
        "same function, so the others' hits are resumed past rather than recorded.",
    )
    parser.add_argument(
        "--craft",
        type=lambda v: int(v, 0),
        metavar="ADDRESS",
        help="follow this craft address instead of selecting one by --ship. The "
        "addresses seen are printed to stderr, so a first run identifies them.",
    )
    parser.add_argument(
        "--max-ships",
        type=int,
        default=8,
        metavar="N",
        help="how many craft a tick may update, which bounds how many breakpoint "
        "hits one tick of the followed craft is allowed to cost.",
    )
    parser.add_argument(
        "--warmup",
        type=float,
        default=0.0,
        metavar="SECONDS",
        help="run free with the buttons held for this long before capturing. A "
        "breakpoint round trip costs far more than a frame, so capture runs the "
        "emulator at a small fraction of real time - too slow to sit through a "
        "start-line countdown. Warm up first, then capture the part that matters.",
    )
    args = parser.parse_args()

    if args.script and args.hold:
        parser.error(
            "--hold holds one input for the whole capture and --script sends a "
            "fresh one every tick, so they cannot both drive it. --warmup-hold is "
            "what holds something during the warmup of a scripted capture."
        )
    if args.warmup_hold and not args.script:
        parser.error("--warmup-hold is for scripted captures; without --script, --hold covers it")

    states = []
    if args.script:
        try:
            states = input_script.load(args.script)
        except input_script.ScriptError as error:
            parser.error(str(error))
        if not states:
            parser.error("%s: the script has no ticks in it" % args.script)
        for reason in input_script.unrepresentable(states):
            parser.error("%s cannot be sent to a PSP - %s" % (args.script, reason))
    ticks = args.ticks if args.ticks is not None else (len(states) or 300)
    if states and ticks > len(states):
        print(
            "note: %s covers %d tick(s) and --ticks asks for %d, so its last state "
            "is held for the remaining %d"
            % (args.script, len(states), ticks, ticks - len(states)),
            file=sys.stderr,
        )

    dbg = Debugger(args.port)
    out = args.out.open("w") if args.out else sys.stdout
    # What the pad was last told, so a tick that changes nothing costs no
    # traffic. `None` means "nothing has been sent yet", which is not the same as
    # "nothing is held" - the first tick must always send.
    sent = {"state": None, "analog": None}

    def send(state):
        """Push one tick of scripted state at the emulator, if it changed."""
        if sent["state"] is not None and state == sent["state"]:
            return
        sent["state"] = state
        dbg.hold(**input_script.button_payload(state))
        analog = input_script.analog_payload(state)
        if analog is None and sent["analog"] not in (None, (0.0, 0.0)):
            # A script that stops asking for analog must recentre the stick, or
            # the last explicit deflection would silently outlive its own line.
            analog = (0.0, 0.0)
        if analog is not None:
            dbg.analog(*analog)
            sent["analog"] = analog

    try:
        if args.hold:
            dbg.resume()
            dbg.hold(**{button: True for button in args.hold})
        if args.warmup_hold:
            dbg.resume()
            dbg.hold(**{button: True for button in args.warmup_hold})
        if args.warmup:
            dbg.resume()
            time.sleep(args.warmup)
        if args.warmup_hold:
            # The script owns the input from here; anything the warmup held that
            # the script's first tick does not is released by the first send().
            dbg.hold(**{button: False for button in args.warmup_hold})

        names = [name for name, _ in CRAFT_FIELDS] + [name for name, _ in BODY_FIELDS]
        print("tick," + ",".join(names), file=out)

        # A race updates every craft from the same function, so the breakpoint
        # fires once per ship per tick and only one of those hits is the ship
        # being traced. Hits for the others are resumed past rather than
        # recorded: interleaving eight ships into one file would produce a trace
        # of nothing in particular, and stopping at the second address - which
        # this did until a real eight-ship capture met it - makes a trace of a
        # race impossible to take at all.
        seen = []
        follow = args.craft
        tick = 0
        hits = 0
        budget = ticks * args.max_ships + args.max_ships
        # `each_hit` owns the break/arm/resume cycle, including the two traps
        # around it, so the filter is a `continue` inside its loop rather than a
        # second copy of that sequence.
        for _, _ in dbg.each_hit(SHIP_UPDATE_CRAFT, budget, timeout=60):
            if tick >= ticks:
                break
            hits += 1

            registers = dbg.call("cpu.getAllRegs")
            gpr = next(c for c in registers["categories"] if c["name"] == "GPR")
            craft = dict(zip(gpr["registerNames"], gpr["uintValues"]))[CRAFT_REGISTER]
            if craft not in seen:
                seen.append(craft)
                print("craft %d at 0x%08x" % (len(seen) - 1, craft), file=sys.stderr)
            if follow is None and len(seen) > args.ship:
                follow = seen[args.ship]
                print("following craft 0x%08x" % follow, file=sys.stderr)
            if craft != follow:
                continue

            craft_blob = dbg.read(craft, CRAFT_BYTES)
            body = struct.unpack("<I", craft_blob[BODY_POINTER : BODY_POINTER + 4])[0]
            body_blob = dbg.read(body, CRAFT_BYTES)
            values = [struct.unpack("<f", craft_blob[at : at + 4])[0] for _, at in CRAFT_FIELDS]
            values += [struct.unpack("<f", body_blob[at : at + 4])[0] for _, at in BODY_FIELDS]
            print("%d,%s" % (tick, ",".join("%.7g" % v for v in values)), file=out)

            # The row above is the craft as this frame's update *found* it, and
            # the input set here is what drives the frame that produces the next
            # row - which is exactly `oag-trace`'s alignment, where a row is
            # emitted before its own step. Whether the emulator's own input poll
            # has already run by the time the breakpoint is reached is not
            # established, so `--script-lead` exists to shift this by whole ticks
            # once a capture has measured it.
            if states:
                send(input_script.at(states, tick + args.script_lead))
            tick += 1
        print(
            "%d tick(s) from %d hit(s) across %d craft" % (tick, hits, len(seen)),
            file=sys.stderr,
        )
    finally:
        try:
            if args.hold:
                dbg.resume()
                dbg.hold(**{button: False for button in args.hold})
            elif states or args.warmup_hold:
                # Leave the pad the way it was found, or the next capture starts
                # with whatever the last line of this script happened to hold.
                dbg.resume()
                dbg.hold(**input_script.button_payload(input_script.State()))
                dbg.analog(0.0, 0.0)
        except Exception:
            pass
        if args.out:
            out.close()
        dbg.close()


if __name__ == "__main__":
    main()
