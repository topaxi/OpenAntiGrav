#!/usr/bin/env python3
"""Live-check the SAP broadphase clamp globals during a real race.

`docs/ghidra/functions/psp-pulse-usa/collision.md`'s clamp section reads
`g_sap_clamp_min` (`0x08ab0c50`) / `g_sap_clamp_max` (`0x08ab0c60`) as all-zero
in shipped `.data` with no writer found by any static scan, and stops at
confidence 45 rather than calling the clamp degenerate, because a zero clamp
"sits badly with the game's frame rate" - a plausibility argument, not
evidence. This script replaces it with a live read: `Sap_Insert`, `Sap_Update`
and `Sap_QueryAabb` each `lv.q` these two addresses every call, so a *read*
watchpoint on `g_sap_clamp_min` must fire during a running race - that is the
positive control that proves the read below actually observed the clamp path
executing, not a quiet or dead one.

Requires a PPSSPP already running with the debugger enabled and a race
already loaded and moving (`psp-drive.py restart` gets you there).

    uv run --with websocket-client scripts/psp-check-sap-clamp.py --port 47810
"""

import argparse
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

CLAMP_MIN = 0x08AB0C50
CLAMP_MAX = 0x08AB0C60


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument(
        "--settle", type=float, default=3.0, help="seconds to let the race run with the read watch armed"
    )
    args = parser.parse_args()

    dbg = Debugger(args.port)

    dbg.brk()
    before_min = dbg.read(CLAMP_MIN, 16)
    before_max = dbg.read(CLAMP_MAX, 16)
    dbg.call(
        "memory.breakpoint.add",
        address=CLAMP_MIN,
        size=16,
        enabled=True,
        log=True,
        read=True,
        write=True,
        change=False,
    )
    dbg.resume()

    time.sleep(args.settle)

    dbg.brk()
    hits = dbg.call("memory.breakpoint.list")
    after_min = dbg.read(CLAMP_MIN, 16)
    after_max = dbg.read(CLAMP_MAX, 16)
    dbg.call("memory.breakpoint.remove", address=CLAMP_MIN, size=16)
    dbg.resume()
    dbg.close()

    print("g_sap_clamp_min 0x%08x before: %s" % (CLAMP_MIN, before_min.hex()))
    print("g_sap_clamp_min 0x%08x after:  %s" % (CLAMP_MIN, after_min.hex()))
    print("g_sap_clamp_max 0x%08x before: %s" % (CLAMP_MAX, before_max.hex()))
    print("g_sap_clamp_max 0x%08x after:  %s" % (CLAMP_MAX, after_max.hex()))
    print("breakpoint.list: %s" % hits)


if __name__ == "__main__":
    main()
