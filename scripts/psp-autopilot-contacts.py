#!/usr/bin/env python3
"""The original's own wall contacts, per tick, beside the craft's pose.

`scripts/psp-autopilot-trace.py`'s capture (the player's Autopilot armed after GO,
one stop a frame at `Ship_UpdateCraft`, `0x08849618`, the same CSV columns), plus
a JSON-lines side file with what the previous world step's narrowphase produced:

- every contact in the world's array whose box proxy (`contact+0x28`) is the
  player body's own (`body+0x3a8`): the array is at `world+0x450`, stride
  `0x40`, counted at `world+0x2450`, and `world` is the global at `0x08ab0820`
  (`Body_StepWorld`, `0x0884fc78`/`0x0885000c`). Each record is the sample point
  (`+0x00`), the unit normal (`+0x10`), the proxies and owners (`+0x20..+0x2c`),
  the depth (`+0x30`) and the friction (`+0x34`) (`collision.md`, "What a
  contact holds");
- the box collider (`body+0x3bc`, `Body_SyncBoxCollider` `0x0884db20`): the
  centre and basis it cached (`+0x80..+0xb0`), the three half-axes
  (`+0xc0`/`+0xd0`/`+0xe0`), the ten sample points (`+0x130` up, stride 16)
  and the box dimensions (`+0x1d0`).

The array is empty by `Ship_UpdateCraft` (pass 3 of `Body_StepWorld`), so each
tick swaps the breakpoint to `0x0885001c`, the instruction after
`Collision_StepNarrowphase` returns in pass 5, reads the array and the collider
there, and swaps back (PPSSPP v1.20.4 fires only the newest breakpoint). Row
`k`'s contacts were made by the step from row `k`'s pose, against the
post-integrate pose the collider cached, before any of them resolved.

`--weave N` skips the autopilot and holds thrust while steering left and right
for `N` frames each, with `N` straight frames between, to put the craft into
walls at many angles; `--pulse M` shortens each turn to `M` frames, for
high-speed impacts.

    uv run --with websocket-client python3 scripts/psp-autopilot-contacts.py \\
        --port 45697 --out data/scratch/<lane>/run.csv --frames 2400

Raw captures are derived game data: keep them under `data/`, never commit them.
"""

import argparse
import importlib.util
import json
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
WORLD_GLOBAL = 0x08AB0820
CONTACTS = 0x450
CONTACT_COUNT = 0x2450
BODY_PROXY = 0x3A8
BODY_BOX = 0x3BC
NARROW_DONE = 0x0885001C


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--restart", action="store_true", help="pause -> RESTART RACE first")
    parser.add_argument("--arm-after", type=int, default=60)
    parser.add_argument("--frames", type=int, default=2400)
    parser.add_argument("--weave", type=int, help="no autopilot: steer left/right N frames each")
    parser.add_argument("--pulse", type=int, help="with --weave: steer only this many frames per turn")
    parser.add_argument("--budget", type=float, default=540.0, help="wall-clock seconds")
    args = parser.parse_args()
    args.stint = None
    args.steer = None

    dbg = rail.Debugger(args.port)
    log = {"events": []}
    start = time.time()
    names = ([n for n, _ in CRAFT_FIELDS] + [n for n, _ in ENTITY_FIELDS]
             + [n for n, _ in BODY_FIELDS])
    rows = []
    side = args.out.with_suffix(".contacts.jsonl")
    side.parent.mkdir(parents=True, exist_ok=True)
    sidef = side.open("w")
    try:
        if args.restart:
            rail.pair.restart_to_countdown(dbg, hold=True)
        else:
            dbg.resume()
            dbg.hold(cross=True)
        if args.weave:
            p = None
            while p is None:
                dbg.brk()
                p = rail.player(dbg)
                dbg.resume()
                time.sleep(0.5)
            dbg.brk()
        else:
            p = rail.arm(dbg, args, log)
        print(log["events"], file=sys.stderr)
        craft = p["craft"]
        entity = dbg.read_u32(craft + 0x1C4)
        entity_lo = min(at for _, at in ENTITY_FIELDS)
        entity_hi = max(at for _, at in ENTITY_FIELDS) + 4
        world = dbg.read_u32(WORLD_GLOBAL)
        print("world 0x%08x" % world, file=sys.stderr)
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
                cb = dbg.read(craft, CRAFT_BYTES)
                body = struct.unpack("<I", cb[0x1CC:0x1D0])[0]
                bb = dbg.read(body, CRAFT_BYTES)
                eb = dbg.read(entity + entity_lo, entity_hi - entity_lo)
                values = [struct.unpack("<f", cb[a:a + 4])[0] for _, a in CRAFT_FIELDS]
                values += [struct.unpack("<f", eb[a - entity_lo:][:4])[0]
                           for _, a in ENTITY_FIELDS]
                values += [struct.unpack("<f", bb[a:a + 4])[0] for _, a in BODY_FIELDS]
                rows.append(values)
                if args.weave:
                    pulse = args.pulse or args.weave
                    at = n % (2 * (args.weave + pulse))
                    left = args.weave <= at < args.weave + pulse
                    right = at >= 2 * args.weave + pulse
                    dbg.hold(cross=True, left=left, right=right)
                dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)
                dbg.add_breakpoint(NARROW_DONE)
                dbg.call("cpu.status")
                dbg.pending = []
                dbg.call("cpu.resume")
                try:
                    dbg.wait_for_break(NARROW_DONE, timeout=30.0)
                except TimeoutError:
                    status = dbg.call("cpu.status")
                    if not (status["stepping"] and status["pc"] == NARROW_DONE):
                        raise

                proxy = struct.unpack("<I", bb[BODY_PROXY:BODY_PROXY + 4])[0]
                box = struct.unpack("<I", bb[BODY_BOX:BODY_BOX + 4])[0]
                count = dbg.read_u32(world + CONTACT_COUNT)
                mine = []
                if 0 < count <= 128:
                    raw = dbg.read(world + CONTACTS, 0x40 * count)
                    for i in range(count):
                        f = struct.unpack_from("<16f", raw, 0x40 * i)
                        w = struct.unpack_from("<16I", raw, 0x40 * i)
                        if w[10] != proxy:
                            continue
                        mine.append({"index": i, "point": f[0:3], "normal": f[4:7],
                                     "mesh_proxy": w[8], "mesh_owner": w[9],
                                     "box_owner": w[11], "depth": f[12], "friction": f[13]})
                cr = dbg.read(box, 0x1E0)
                cf = struct.unpack_from("<120f", cr, 0)
                rec = {"tick": n, "count": count, "proxy": proxy, "contacts": mine,
                       "centre": cf[0xB0 // 4:0xB0 // 4 + 3],
                       "rows": [cf[0x80 // 4 + 4 * k:0x80 // 4 + 4 * k + 3] for k in range(3)],
                       "half": [cf[0xC0 // 4 + 4 * k:0xC0 // 4 + 4 * k + 3] for k in range(3)],
                       "samples": [cf[0x130 // 4 + 4 * k:0x130 // 4 + 4 * k + 3]
                                   for k in range(10)],
                       "dims": cf[0x1D0 // 4:0x1D0 // 4 + 3]}
                sidef.write(json.dumps(rec) + "\n")
                dbg.remove_breakpoint(NARROW_DONE)
                dbg.add_breakpoint(SHIP_UPDATE_CRAFT, "a0 == 0x%x" % craft)
                if n % 300 == 0:
                    print("row %d contacts %d" % (n, len(mine)), file=sys.stderr)
        finally:
            dbg.brk()
            for address in (SHIP_UPDATE_CRAFT, NARROW_DONE):
                try:
                    dbg.remove_breakpoint(address)
                except Exception:
                    pass
            dbg.resume()
    finally:
        sidef.close()
        with args.out.open("w") as out:
            print("tick," + ",".join(names), file=out)
            for tick, values in enumerate(rows):
                print("%d,%s" % (tick, ",".join("%.9g" % v for v in values)), file=out)
        print("wrote %s and %s (%d rows)" % (args.out, side, len(rows)), file=sys.stderr)


if __name__ == "__main__":
    main()
