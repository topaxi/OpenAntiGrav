#!/usr/bin/env python3
"""A `psp-trace.py` trace of the player's craft while the original's autopilot flies it.

Arms the player's Autopilot pickup after GO the way `scripts/psp-autopilot-rail.py`
does, then stops once a frame in `Ship_UpdateCraft` (`0x08849618`, `a0` = the
player's craft) and writes the same columns `psp-trace.py` does (`dt`, the ramped
controls, the timers, the body's basis, position, velocity and angular columns),
so `oag_trace::Trace` reads it and `oag_trace::replay::initial_state` seeds a
`ShipState` from any row.

The autopilot's rail (`Ai_SnapToLane`, `0x088542e4`) writes the body's velocity
and position inside the AI update, which runs before the world step in the same
frame (`ai-controls.md`, "One AI update per world step"). So a row's velocity
is already past this frame's rail, and `v(N+1) - v(N)` is this frame's physics
plus frame `N+1`'s rail. Four extra columns undo the rail: `rail_blend` (`driver
+0x48`, the blend frame N's rail used) and `own_lat_x/y/z`, the racer's own
sample's lateral (`racer+0x260+0x30`). The driver's raw output this frame
(`driver+0x08`: steer, thrust, left, right, pitch, already written by the AI
update), the pitch gate's inputs (hull up `craft+0x160`, the located sample's down
`*(craft+0x1c4)+0xb10`) and the flags `craft+0x1c0` follow. The rail keeps the speed and scales the
lateral component by `1 - f/4`, `f = blend^2 * 0.7`, so it inverts exactly:
`v_pre = v + lat * dot(v, lat) * k / (1 - k)`, `k = f / 4`, rescaled to `|v|`.

    uv run --with websocket-client python3 scripts/psp-autopilot-trace.py \\
        --port 45697 --out data/traces/<name>.csv --restart --frames 1300

Raw captures are derived game data: keep them under `data/`, never commit them.
"""

import argparse
import importlib.util
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from psp_trace_fields import BODY_FIELDS, CRAFT_FIELDS, ENTITY_FIELDS  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "rail", Path(__file__).resolve().parent / "psp-autopilot-rail.py"
)
rail = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(rail)

SHIP_UPDATE_CRAFT = 0x08849618
CRAFT_BYTES = 0x400
EXTRA = ["rail_blend", "own_lat_x", "own_lat_y", "own_lat_z", "frame",
         "ai_steer", "ai_thrust", "ai_left", "ai_right", "ai_pitch",
         "hull_up_x", "hull_up_y", "hull_up_z", "spline_down_x", "spline_down_y",
         "spline_down_z", "flags1c0"]


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--restart", action="store_true", help="pause -> RESTART RACE first")
    parser.add_argument("--arm-after", type=int, default=60)
    parser.add_argument("--frames", type=int, default=1300)
    parser.add_argument("--budget", type=float, default=520.0, help="wall-clock seconds")
    args = parser.parse_args()
    args.stint = None
    args.steer = None

    dbg = rail.Debugger(args.port)
    log = {"events": []}
    start = time.time()
    names = ([n for n, _ in CRAFT_FIELDS] + [n for n, _ in ENTITY_FIELDS]
             + [n for n, _ in BODY_FIELDS] + EXTRA)
    rows = []
    try:
        if args.restart:
            rail.pair.restart_to_countdown(dbg, hold=True)
        else:
            dbg.resume()
            dbg.hold(cross=True)
        p = rail.arm(dbg, args, log)
        print(log["events"], file=sys.stderr)
        craft, driver = p["craft"], p["driver"]
        racer = dbg.read_u32(driver)
        entity = dbg.read_u32(craft + 0x1C4)
        entity_lo = min(at for _, at in ENTITY_FIELDS)
        entity_hi = max(at for _, at in ENTITY_FIELDS) + 4
        dbg.add_breakpoint(SHIP_UPDATE_CRAFT, "a0 == 0x%x" % craft)
        try:
            for n in range(args.frames):
                if time.time() - start > args.budget:
                    print("budget spent at %d rows" % n, file=sys.stderr)
                    break
                dbg.call("cpu.status")
                dbg.pending = []
                dbg.call("cpu.resume")
                try:
                    dbg.wait_for_break(SHIP_UPDATE_CRAFT, timeout=30.0)
                except TimeoutError:
                    status = dbg.call("cpu.status")
                    if not (status["stepping"] and status["pc"] == SHIP_UPDATE_CRAFT):
                        raise
                status = dbg.call("cpu.status")
                cb = dbg.read(craft, CRAFT_BYTES)
                body = struct.unpack("<I", cb[0x1CC:0x1D0])[0]
                bb = dbg.read(body, CRAFT_BYTES)
                eb = dbg.read(entity + entity_lo, entity_hi - entity_lo)
                values = [struct.unpack("<f", cb[a:a + 4])[0] for _, a in CRAFT_FIELDS]
                values += [struct.unpack("<f", eb[a - entity_lo:][:4])[0]
                           for _, a in ENTITY_FIELDS]
                values += [struct.unpack("<f", bb[a:a + 4])[0] for _, a in BODY_FIELDS]
                values += rail.f32s(dbg, driver + 0x48, 1)
                values += rail.f32s(dbg, racer + 0x260 + 0x30, 3)
                values.append(int(status["ticks"] / rail.CYCLES_PER_FRAME))
                values += rail.f32s(dbg, driver + 0x08, 5)
                values += rail.f32s(dbg, craft + 0x160, 3)
                values += rail.f32s(dbg, entity + 0xB10, 3)
                values.append(dbg.read_u32(craft + 0x1C0))
                rows.append(values)
                if n % 300 == 0:
                    print("row %d" % n,
                          file=sys.stderr)
        finally:
            dbg.brk()
            dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)
            dbg.resume()
    finally:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        with args.out.open("w") as out:
            print("tick," + ",".join(names), file=out)
            for tick, values in enumerate(rows):
                print("%d,%s" % (tick, ",".join("%.9g" % v for v in values)), file=out)
        print("wrote %s (%d rows)" % (args.out, len(rows)), file=sys.stderr)


if __name__ == "__main__":
    main()
