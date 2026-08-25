#!/usr/bin/env python3
"""Read `entity + 0x368` off every live craft and classify human vs AI.

`Craft_Construct_q` (`0x08840c74`) stores its own second constructor argument
verbatim at `entity + 0x368` (see
`docs/ghidra/functions/psp-pulse-usa/shield.md#entity--0x368-is-craft_construct_qs-own-second-argument`),
but no static caller of that constructor exists to read the argument's real
values off. This rig reads the field the settled way instead: after a race has
loaded, break at `Ship_UpdateCraft` (fires once per craft per tick, per-craft
in `a0`), collect every distinct craft address, and for each one read:

  - `craft + 0x2b8`  (throttle) - digital (0 or 100) for the human, fractional
    for AI, per `ppsspp-debugger.md`'s own reading of a Single Race capture.
  - `craft + 0x1c4`  -> entity, the walk `shield.md` already uses.
  - `entity + 0x368` - the field this rig exists to classify.

Requires a PPSSPP already running with the debugger enabled and a **Single
Race** already loaded and settled at the start line (`psp-drive.py menu
--single-race` gets you there; a Time Trial has only one craft and proves
nothing).

    uv run --with websocket-client scripts/psp-watch-controller-kind.py --port 47830
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

SHIP_UPDATE_CRAFT = 0x08849618
CRAFT_THROTTLE = 0x2B8
ENTITY_POINTER = 0x1C4
CONTROLLER_KIND = 0x368


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=47800)
    parser.add_argument(
        "--hits",
        type=int,
        default=40,
        help="breakpoint hits to sample while collecting craft entities "
        "(a full grid needs at least 8 distinct addresses; several hits "
        "per craft since not every craft updates every tick)",
    )
    args = parser.parse_args()

    dbg = Debugger(args.port)
    dbg.brk()
    dbg.add_breakpoint(SHIP_UPDATE_CRAFT)

    crafts = []
    seen = set()
    for _ in range(args.hits):
        dbg.call("cpu.resume")
        dbg.wait_for_break(SHIP_UPDATE_CRAFT, timeout=15.0)
        registers = dbg.call("cpu.getAllRegs")
        gpr = next(c for c in registers["categories"] if c["name"] == "GPR")
        a0 = dict(zip(gpr["registerNames"], gpr["uintValues"]))["a0"]
        if a0 not in seen:
            seen.add(a0)
            crafts.append(a0)

    dbg.brk()
    dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)
    dbg.resume()

    print("distinct craft entities seen: %d" % len(crafts))
    print("%-12s %8s %-12s %14s" % ("craft", "throttle", "entity", "entity+0x368"))
    for craft in crafts:
        throttle = dbg.read_f32(craft + CRAFT_THROTTLE)
        entity = dbg.read_u32(craft + ENTITY_POINTER)
        kind = None
        if 0x08000000 <= entity < 0x0A000000:
            kind = dbg.read_u32(entity + CONTROLLER_KIND)
            if kind >= 0x80000000:
                kind -= 0x100000000  # signed: -1 is a real value here
        controller = str(kind) if kind is not None else "?"
        print(
            "0x%08x %8.2f 0x%08x %14s"
            % (craft, throttle, entity, controller)
        )

    dbg.close()


if __name__ == "__main__":
    main()
