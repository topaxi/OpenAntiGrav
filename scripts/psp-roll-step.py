#!/usr/bin/env python3
"""Roll step response of a grounded craft in Wipeout Pulse under PPSSPP.

Breaks in `Ship_UpdateCraft` once per frame, lets the craft settle, then rotates
the rigid body's rows 0 and 1 about row 2 (the nose) by `--roll` radians with
both rate columns zeroed, and records the basis, both rate columns, `dt` and
`grounded` on every following frame. `docs/physics/roll-step-response.md` is
what the capture is for.

    uv run --with websocket-client python3 scripts/psp-roll-step.py \\
        --port 45100 --roll 0.25 --frames 300 --out data/traces/roll-step.csv

The output is derived game data: write it under `data/`, never commit it.
"""

import argparse
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger
from psp_trace_fields import SHIP_UPDATE_CRAFT

BODY_POINTER = 0x1CC
COLUMNS = [
    ("dt", "craft", 0x1C8), ("grounded", "craft", 0x2B0),
    ("r0x", "body", 0x00), ("r0y", "body", 0x04), ("r0z", "body", 0x08),
    ("r1x", "body", 0x10), ("r1y", "body", 0x14), ("r1z", "body", 0x18),
    ("r2x", "body", 0x20), ("r2y", "body", 0x24), ("r2z", "body", 0x28),
    ("w150x", "body", 0x150), ("w150y", "body", 0x154), ("w150z", "body", 0x158),
    ("w160x", "body", 0x160), ("w160y", "body", 0x164), ("w160z", "body", 0x168),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--roll", type=float, default=0.25, help="radians about the nose")
    parser.add_argument("--settle", type=int, default=20, help="frames recorded before the step")
    parser.add_argument("--frames", type=int, default=300, help="frames recorded after the step")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    with Debugger(port=args.port) as dbg:
        rows = []
        craft = None
        budget = (args.settle + args.frames) * 16
        for _, _ in dbg.each_hit(SHIP_UPDATE_CRAFT, budget):
            gpr = dbg.call("cpu.getAllRegs")["categories"][0]
            a0 = dict(zip(gpr["registerNames"], gpr["uintValues"]))["a0"]
            if craft is None:
                craft = a0
            if a0 != craft:
                continue
            body = dbg.read_u32(craft + BODY_POINTER)
            if len(rows) == args.settle:
                r0 = dbg.read_f32s(body + 0x00, 3)
                r1 = dbg.read_f32s(body + 0x10, 3)
                r2 = dbg.read_f32s(body + 0x20, 3)
                c, s = math.cos(args.roll), math.sin(args.roll)
                n0 = tuple(c * a - s * b for a, b in zip(r0, r1))
                n1 = tuple(s * a + c * b for a, b in zip(r0, r1))
                dbg.write_f32s(body + 0x00, n0)
                dbg.write_f32s(body + 0x10, n1)
                for slot in range(3):
                    dbg.write_f32s(body + 0xC0 + 0x10 * slot, (n0[slot], n1[slot], r2[slot]))
                dbg.write_f32s(body + 0x150, (0.0, 0.0, 0.0))
                dbg.write_f32s(body + 0x160, (0.0, 0.0, 0.0))
            source = {"craft": dbg.read(craft, 0x2C0), "body": dbg.read(body, 0x170)}
            rows.append([struct.unpack_from("<f", source[w], off)[0] for _, w, off in COLUMNS])
            if len(rows) >= args.settle + args.frames:
                break
        dbg.resume()

    with args.out.open("w") as out:
        out.write("frame," + ",".join(name for name, _, _ in COLUMNS) + "\n")
        for index, row in enumerate(rows):
            out.write(f"{index - args.settle}," + ",".join(repr(v) for v in row) + "\n")
    print(f"craft {craft:#x}: {len(rows)} frames to {args.out}")


if __name__ == "__main__":
    main()
