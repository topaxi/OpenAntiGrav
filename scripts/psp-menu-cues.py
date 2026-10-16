#!/usr/bin/env python3
"""Log every front-end sound cue Pulse (PSP) plays while a button script runs.

Breaks on `Sound_PlayNamedInSlot` (`0x0893a768`), the one entry every named menu
cue passes through, and at each hit records the front end's state name, the cue
name (`a2`), the return address (who asked) and the seconds since the script
started. The CPU is stopped inside the hit, where reads are cheap, then resumed.

    uv run --with websocket-client scripts/psp-menu-cues.py --port 45692 \
        "wait:3 press:start wait:2 state press:down press:cross"

Actions, run in order: `wait:S` (seconds, collecting hits), `press:BUTTON`
(PPSSPP names: cross circle triangle square start select up down left right),
`state` (print the state name). Every press is followed by a 1.5 s collection
window. Only the most recently added breakpoint fires on v1.20.4, so this arms
exactly one.
"""

import argparse
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

PLAY_NAMED = 0x0893A768


def gpr(dbg):
    regs = dbg.call("cpu.getAllRegs")
    cat = next(c for c in regs["categories"] if c["name"] == "GPR")
    return dict(zip(cat["registerNames"], cat["uintValues"]))


def collect(dbg, seconds, t0):
    """Hits for `seconds`; the CPU is left running."""
    end = time.time() + seconds
    while time.time() < end:
        try:
            dbg.wait_for_break(PLAY_NAMED, timeout=max(0.2, min(0.5, end - time.time())))
        except TimeoutError:
            continue
        if not dbg.is_stepping():
            # A rebroadcast of an earlier stop, not a hit.
            continue
        regs = gpr(dbg)
        name = dbg.read_cstring(regs["a2"], 24)
        print(
            f"{time.time() - t0:7.2f}s  state={dbg.state_name()!s:<28} cue={name:<10} "
            f"ra=0x{regs['ra']:08x} pitch=0x{regs['a3']:x}",
            flush=True,
        )
        dbg.pending = []
        dbg.call("cpu.resume")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("script")
    args = parser.parse_args()
    with Debugger(port=args.port) as dbg:
        dbg.pending = []
        if not dbg.is_stepping():
            dbg.call("cpu.stepping")
        try:
            dbg.remove_breakpoint(PLAY_NAMED)
        except Exception:
            pass
        dbg.add_breakpoint(PLAY_NAMED)
        dbg.call("cpu.resume")
        t0 = time.time()
        for action in args.script.split():
            kind, _, arg = action.partition(":")
            if kind == "wait":
                collect(dbg, float(arg), t0)
            elif kind == "press":
                print(f"{time.time() - t0:7.2f}s  press {arg}  (state {dbg.state_name()})", flush=True)
                # Not `dbg.press`: a cue hit stops the CPU before the reply.
                dbg.send("input.buttons.press", button=arg, duration=6)
                collect(dbg, 1.5, t0)
            elif kind == "state":
                print(f"{time.time() - t0:7.2f}s  state {dbg.state_name()}", flush=True)
            else:
                sys.exit(f"unknown action {action}")
        dbg.brk()
        dbg.remove_breakpoint(PLAY_NAMED)
        dbg.call("cpu.resume")


if __name__ == "__main__":
    main()
