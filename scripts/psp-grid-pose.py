#!/usr/bin/env python3
"""The original's eight grid craft, read off the racer table while the track description is up.

`RESTART RACE` places the whole field (`Race_PlaceGrid`) before the description
screen is dismissed, so stopping the CPU the moment the front end reports the
description reads every craft on its slot, before the countdown has run a single
frame. The racer table at `0x08b34420` (stride `0x370`, count at `0x08b35fa0`)
holds each craft's position at `+0x10` and a 3x3 of its axes after it; see
`docs/ghidra/functions/psp-pulse-usa/grid.md`.

    python3 scripts/psp-grid-pose.py --port 45681 --out data/scratch/<lane>/grid.json

Raw captures are derived game data: write them under `data/`, never commit them.
"""

import argparse
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

TABLE = 0x08B34420
STRIDE = 0x370
COUNT = 0x08B35FA0


def read_grid(dbg):
    count = dbg.read_u32(COUNT)
    rows = []
    for i in range(count):
        f = struct.unpack("<28f", dbg.read(TABLE + i * STRIDE, 0x70))
        rows.append({"slot_entry": i, "pos": f[4:7], "axis_a": f[8:11], "axis_b": f[12:15],
                     "axis_c": f[16:19], "raw": f})
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--no-restart", action="store_true")
    args = parser.parse_args()
    dbg = Debugger(args.port)
    dbg.resume()
    dbg.hold(cross=False)
    if not args.no_restart:
        if "Pause" not in dbg.state_name():
            dbg.press("start", duration=6)
            time.sleep(1.5)
        for _ in range(4):
            dbg.press("down", duration=4)
            time.sleep(0.35)
        dbg.press("cross", duration=6)
    seen = []
    deadline = time.time() + 90
    while time.time() < deadline:
        state = dbg.state_name()
        seen.append(state)
        if state and "Description" in state:
            break
        time.sleep(0.15)
    else:
        raise SystemExit("never saw the track description; states: %s" % sorted(set(map(str, seen))))
    dbg.brk()
    rows = read_grid(dbg)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({"state": state, "rows": rows}))
    for r in rows:
        print(r["slot_entry"], [round(v, 3) for v in r["pos"]], [round(v, 4) for v in r["axis_b"]])
    dbg.resume()


if __name__ == "__main__":
    main()
