#!/usr/bin/env python3
"""The original's grid walk, read live: the located record at each of the eight heading calls.

`Race_ComputeGridLayout` (`0x0882b3b0`) walks the located curve once per slot, slot 8 first, and
calls `FUN_0882663c` for each slot's heading at `0x0882bb54`. `s7` is the walk's own `0x70`-byte
record (`sp + 0x60`: position at `+0x00`, tangent `+0x10`, down `+0x20`, lateral `+0x30`, the
half-widths `+0x44` and `+0x48`), the sample `AiTrack_LocatePosition` wrote for that slot. A
breakpoint on that `jal` and one read of `s7` give the eight records, whose positions are the
centreline chain `p(k+1) = locate(p(k) + tangent * 19.8)` and whose `w` lane is the record scale.

    python3 scripts/psp-grid-walk.py --port 45491 --out data/scratch/<lane>/walk.json

Run it from a race (any Single Race; Metropia is two track-downs from Talon's Junction, see
`psp-drive.py menu --single-race --track-down 2`). It opens the pause menu, restarts the race,
and stops eight times at the breakpoint, so the grid is placed under it.

**Only the most recently added execution breakpoint fires** (`ppsspp_debugger.py`), so this
arms exactly one. Raw captures are derived game data: write them under `data/`, never commit
them.
"""

import argparse
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

HEADING_CALL = 0x0882BB54


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--address", type=lambda s: int(s, 0), default=HEADING_CALL)
    args = parser.parse_args()

    dbg = Debugger(args.port)
    dbg.resume()
    dbg.hold(cross=False)
    dbg.brk()
    dbg.add_breakpoint(args.address)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    if "Pause" not in dbg.state_name():
        dbg.press("start", duration=6)
        time.sleep(1.5)
    for _ in range(4):
        dbg.press("down", duration=4)
        time.sleep(0.35)
    # The confirm on RESTART RACE is routinely swallowed: press until the state leaves the
    # pause menu.
    for _ in range(6):
        dbg.press("cross", duration=6)
        time.sleep(1.5)
        if "Pause" not in dbg.state_name():
            break

    rows = []
    for hit in range(8):
        dbg.wait_for_break(args.address, timeout=120)
        registers = dbg.call("cpu.getAllRegs")
        gpr = next(c for c in registers["categories"] if c["name"] == "GPR")
        record = dict(zip(gpr["registerNames"], gpr["uintValues"]))["s7"]
        fields = struct.unpack("<28f", dbg.read(record, 0x70))
        rows.append({"hit": hit, "record": record, "fields": fields})
        print(hit, hex(record), [round(f, 4) for f in fields[0:8]])
        dbg.call("cpu.status")
        dbg.pending = []
        dbg.call("cpu.resume")
    dbg.brk()
    dbg.remove_breakpoint(args.address)
    dbg.resume()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(rows))
    print("wrote", args.out)


if __name__ == "__main__":
    main()
