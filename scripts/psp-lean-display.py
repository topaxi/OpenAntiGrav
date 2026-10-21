#!/usr/bin/env python3
"""Does the steering lean turn the drawn hull, and which way? (Pulse PSP USA.)

Holds a stick deflection on a free-running PPSSPP, then stops at `0x08842140`
(`FUN_088418e0`'s display-matrix block, the player's entity in `s2`) on
consecutive ticks and reads the lean `entity+0x854`, the eased roll
`entity+0x880`, the body basis `*(entity+0x794)` and the drawn node's matrix
`*(*(entity+0x8b0)+0x3c)+0x40`. The node read at hit `k + 1` holds what hit `k`
composed, so each line predicts it from hit `k`'s angle,
`angle = lean * 0.5 + eased * 6.28`, as
`row0 = 0.75 * (cos(angle) * body_row0 + sin(angle) * body_row1)`
(the rollsign capture's law, `input-bindings.md`, Stage 2a) and prints the
residual. A positive lean must read the same way a positive roll did there.

    uv run --with websocket-client python3 scripts/psp-lean-display.py \\
        --port 9315 --entity 0x09a0cee0 --stick 1.0
"""

import argparse
import math
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

DISPLAY_BLOCK = 0x08842140


def f32s(dbg, address, count):
    return list(struct.unpack("<%df" % count, dbg.read(address, 4 * count)))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--entity", type=lambda v: int(v, 0), required=True)
    ap.add_argument("--stick", type=float, default=1.0, help="analog x, -1..1")
    ap.add_argument("--settle", type=float, default=2.0, help="seconds held before reading")
    ap.add_argument("--hits", type=int, default=6)
    args = ap.parse_args()
    with Debugger(args.port) as dbg:
        dbg.resume()
        dbg.analog(args.stick, 0.0)
        time.sleep(args.settle)
        dbg.brk()
        dbg.add_breakpoint(DISPLAY_BLOCK, condition="s2 == 0x%08x" % args.entity)
        previous = None
        try:
            for _, _ in dbg.each_hit(DISPLAY_BLOCK, args.hits):
                e = args.entity
                lean, eased = f32s(dbg, e + 0x854, 1)[0], f32s(dbg, e + 0x880, 1)[0]
                body = f32s(dbg, dbg.read_u32(e + 0x794), 12)
                node = dbg.read_u32(dbg.read_u32(e + 0x8B0) + 0x3C)
                display = f32s(dbg, node + 0x40, 4)
                if previous is not None:
                    angle, b = previous
                    c, s = math.cos(angle), math.sin(angle)
                    want = [0.75 * (c * b[i] + s * b[4 + i]) for i in range(3)]
                    flip = [0.75 * (c * b[i] - s * b[4 + i]) for i in range(3)]
                    err = max(abs(want[i] - display[i]) for i in range(3))
                    err_flip = max(abs(flip[i] - display[i]) for i in range(3))
                    print("angle %+.4f  row0 %s  residual %.5f  (opposite sign %.5f)"
                          % (angle, ["%+.4f" % v for v in display[:3]], err, err_flip))
                print("lean %+.5f eased %+.5f" % (lean, eased))
                previous = (lean * 0.5 + eased * 6.28, body)
        finally:
            dbg.remove_breakpoint(DISPLAY_BLOCK)
            dbg.analog(0.0, 0.0)
            dbg.resume()


if __name__ == "__main__":
    main()
