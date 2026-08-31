#!/usr/bin/env python3
"""Arm read watchpoints on a live DirectionalLight list, to answer "does
anything ever read it" the way `docs/reverse-engineering/ppsspp-debugger.md`'s
watchpoint section says a memory watchpoint answers best.

Context: `docs/ghidra/functions/psp-pulse-eu/lighting.md` traced every static
mechanism this project could find for reaching the world object that holds
Wipeout Pulse's collected `DirectionalLight` list (`world+0x40` count,
`world+0x7c` the 4-pointer list) and found zero consumers, across the
class-key enumeration and a full characterization of the one lead outside
it. This script is the "genuinely different angle" that page's thread file
asks for: a live read watchpoint on the actual runtime addresses, for the
whole rest of a race, not just track load.

`World_CollectMarkerLists` (EU `0x0887a1c4`) is what populates the fields;
its USA counterpart was confirmed by a 100% structural diff (98/98
instructions equal, `diff_functions` via ghidra-mcp) to be `0x0887a368` -
this script runs against the USA disc because the driving scripts
(`psp-drive.py`) are calibrated for it, not because the EU binary is any
less the target of record.

Sequence: break at `World_CollectMarkerLists`'s entry to learn the world
pointer from `$a0` (this is a load-time function - it needs a *fresh* track
load to fire, so run `psp-drive.py restart` from a second shell while this
script is waiting), then arm all three watchpoints (`enabled: False, log:
False` - counting only, no halting, no log-flood risk) and exit. Read the
results later with `memory.breakpoint.list`.

**Does not break at `Ship_UpdateCraft` itself, on purpose.** `psp-drive.py
restart`'s own `settle_into_race` already breaks there internally to print
its own progress line, and PPSSPP execution breakpoints are global CPU
state shared across every connected client - two connections racing to add,
resume past, and remove the *same* address produced a garbage body-pointer
read here once (`craft+0x1CC` came back `0`, arming a watch on address
`0x30` instead of a real one). `--craft` takes the address `psp-drive.py
restart`/`state` already prints instead, sidestepping the race entirely.
**A stale watchpoint on an invalid address is not harmless** - one left
armed here long enough tripped PPSSPP's own bad-memory-access safety halt
("Bad memory access detected! 00000030 ... Stopping emulation") and killed
the whole session. Always verify a computed control address is in
`0x08000000`-`0x0A000000` before arming it, and remove a bad one
immediately rather than leaving it at 0 hits.

    uv run --with websocket-client scripts/psp-drive.py restart --port 47810
    # prints e.g. "craft 0x09a0c590 at ..."
    uv run --with websocket-client scripts/psp-watch-directionallight-consumer.py \\
        --port 47810 --craft 0x09a0c590
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

WORLD_COLLECT_MARKER_LISTS = 0x0887A368  # USA; EU 0x0887a1c4, 100% diff-matched
BODY_POINTER = 0x1CC
BODY_POSITION = 0x030

DIRECTIONAL_LIGHT_COUNT = 0x40  # 4 bytes, an int
DIRECTIONAL_LIGHT_LIST = 0x7C  # 16 bytes, 4 pointers


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=47800)
    parser.add_argument(
        "--craft",
        type=lambda s: int(s, 0),
        required=True,
        help="a live craft address, from psp-drive.py restart/state's own output "
        "(not re-learned here, to avoid racing psp-drive.py's own "
        "Ship_UpdateCraft breakpoint on the same global CPU state)",
    )
    parser.add_argument(
        "--load-timeout",
        type=float,
        default=90.0,
        help="how long to wait for World_CollectMarkerLists to fire "
        "(run psp-drive.py restart from another shell to trigger it)",
    )
    args = parser.parse_args()

    dbg = Debugger(args.port)
    dbg.brk()
    dbg.add_breakpoint(WORLD_COLLECT_MARKER_LISTS)
    dbg.resume()
    print("waiting for a track (re)load - run `psp-drive.py restart` now", file=sys.stderr)
    dbg.wait_for_break(WORLD_COLLECT_MARKER_LISTS, timeout=args.load_timeout)
    registers = dbg.call("cpu.getAllRegs")
    gpr = next(c for c in registers["categories"] if c["name"] == "GPR")
    world = dict(zip(gpr["registerNames"], gpr["uintValues"]))["a0"]
    print("world object: 0x%08x" % world)
    dbg.remove_breakpoint(WORLD_COLLECT_MARKER_LISTS)

    body = dbg.read_u32(args.craft + BODY_POINTER)
    if not 0x08000000 <= body < 0x0A000000:
        print(
            "body pointer 0x%08x is not a plausible PSP RAM address - refusing "
            "to arm a control watch on garbage (see the module docstring's "
            "'not harmless' warning)" % body,
            file=sys.stderr,
        )
        dbg.resume()
        dbg.close()
        raise SystemExit(1)
    control_addr = body + BODY_POSITION
    print("control (rigid body position of craft 0x%08x): 0x%08x" % (args.craft, control_addr))

    dbg.call(
        "memory.breakpoint.add",
        address=world + DIRECTIONAL_LIGHT_COUNT,
        size=4,
        enabled=False,
        log=False,
        read=True,
        write=False,
        change=False,
    )
    print("armed read watch on world+0x40 (DirectionalLight count), 0x%08x" % (world + DIRECTIONAL_LIGHT_COUNT))

    dbg.call(
        "memory.breakpoint.add",
        address=world + DIRECTIONAL_LIGHT_LIST,
        size=16,
        enabled=False,
        log=False,
        read=True,
        write=False,
        change=False,
    )
    print("armed read watch on world+0x7c (DirectionalLight list), 0x%08x" % (world + DIRECTIONAL_LIGHT_LIST))

    dbg.call(
        "memory.breakpoint.add",
        address=control_addr,
        size=12,
        enabled=False,
        log=False,
        read=True,
        write=False,
        change=False,
    )
    print("armed positive control read watch, 0x%08x" % control_addr)

    dbg.resume()
    print("resumed; let the race run, then check hits with memory.breakpoint.list")
    dbg.close()


if __name__ == "__main__":
    main()
