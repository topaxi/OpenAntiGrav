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

The output is CSV on stdout, one row per tick. It records **derived game data**
and must never be committed; write it under `data/traces/`.
"""

import argparse
import struct
import time
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

# Ship_UpdateCraft, docs/ghidra/functions/psp-pulse/engine.md. **The craft is a0
# and the body is a1**, not a1/a2 as that page's signature says: at a breakpoint
# on entry, `craft+0x1cc` holds exactly the value in a1, dt at `craft+0x1c8`
# reads as a plausible frame time and `grounded` at `craft+0x2b0` as 1.0, while
# the same offsets off a1 are nonsense. a2 holds the function's own address,
# which is what the vtable dispatch at the call site leaves there.
SHIP_UPDATE_CRAFT = 0x08849618
CRAFT_REGISTER = "a0"
CRAFT_BYTES = 0x400
BODY_POINTER = 0x1CC

# Offsets into the craft, from engine.md.
CRAFT_FIELDS = [
    ("dt", 0x1C8),
    ("grounded", 0x2B0),
    ("throttle", 0x2B8), ("brake", 0x2BC), ("steer", 0x2C0),
    ("airbrake_l", 0x2C4), ("airbrake_r", 0x2C8),
    ("speed_cached", 0x2EC),
]

# Offsets into the rigid body, measured at runtime by diffing successive frames.
# Rows 0/1/2 are orthonormal, and velocity times the frame time reproduces the
# position delta to within 0.007 units per tick over a 200-tick capture.
#
# **`speed` at +0x398 is not the velocity's own length**, which an earlier pass
# recorded here and a real capture disproved: over 200 ticks of Talon's Junction
# it runs a steady 3.67 % high (ratio 1.0367, sd 0.0026), about 0.86 units/s.
# `speed_cached` on the craft *is* the velocity's length, one tick stale. What
# +0x398 actually holds is not established, and the capture's own speed range is
# only 3 %, so a constant offset and a constant factor cannot be told apart from
# this data. Both columns are recorded; neither is assumed.
BODY_FIELDS = [
    ("right_x", 0x000), ("right_y", 0x004), ("right_z", 0x008),
    ("up_x", 0x010), ("up_y", 0x014), ("up_z", 0x018),
    ("fwd_x", 0x020), ("fwd_y", 0x024), ("fwd_z", 0x028),
    ("pos_x", 0x030), ("pos_y", 0x034), ("pos_z", 0x038),
    ("vel_x", 0x140), ("vel_y", 0x144), ("vel_z", 0x148),
    ("speed", 0x398),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument("--ticks", type=int, default=300)
    parser.add_argument(
        "--hold",
        action="append",
        default=[],
        metavar="BUTTON",
        help="hold a button for the whole capture, e.g. --hold cross (thrust)",
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

    dbg = Debugger(args.port)
    out = args.out.open("w") if args.out else sys.stdout
    try:
        if args.hold:
            dbg.resume()
            dbg.hold(**{button: True for button in args.hold})
        if args.warmup:
            dbg.resume()
            time.sleep(args.warmup)

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
        budget = args.ticks * args.max_ships + args.max_ships
        # `each_hit` owns the break/arm/resume cycle, including the two traps
        # around it, so the filter is a `continue` inside its loop rather than a
        # second copy of that sequence.
        for _, _ in dbg.each_hit(SHIP_UPDATE_CRAFT, budget, timeout=60):
            if tick >= args.ticks:
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
            tick += 1
        print(
            "%d tick(s) from %d hit(s) across %d craft" % (tick, hits, len(seen)),
            file=sys.stderr,
        )
    finally:
        if args.hold:
            try:
                dbg.resume()
                dbg.hold(**{button: False for button in args.hold})
            except Exception:
                pass
        if args.out:
            out.close()
        dbg.close()


if __name__ == "__main__":
    main()
