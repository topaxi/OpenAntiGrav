#!/usr/bin/env python3
"""Put the original's craft into a recorded state and read the contacts its next step makes.

For each `--tick`, stops at `Ship_UpdateCraft` (`0x08849618`) for the player's
craft, overwrites the body with that row of a `psp-trace.py`-shaped capture
(basis rows `+0x00..+0x28`, position `+0x30`, velocity `+0x140`, `+0x150` and
`+0x160`, every field `scripts/psp_trace_fields.py` records, written back to the
offset it was read from), then swaps the breakpoint to `0x0885001c`, the
instruction after `Collision_StepNarrowphase` returns, and reads the world's
contact array and the box collider the way `scripts/psp-autopilot-contacts.py`
does. One more stop at the next `Ship_UpdateCraft` reads the body the step left,
to set against the capture's next row.

The craft's own controls, timers and spline state are whatever the live race
holds, so only what one integrate and one narrowphase do with the written body is
comparable; a row's force terms are not reproduced.

    uv run --with websocket-client python3 scripts/psp-contact-at.py --port 45697 \\
        --csv data/traces/talons-junction-clean-lap.csv --tick 1583 --tick 1584 \\
        --out data/scratch/<lane>/at.jsonl

Raw captures are derived game data: keep them under `data/`, never commit them.
"""

import argparse
import csv
import importlib.util
import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from psp_trace_fields import BODY_FIELDS  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "rail", Path(__file__).resolve().parent / "psp-autopilot-rail.py"
)
rail = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(rail)

SHIP_UPDATE_CRAFT = 0x08849618
NARROW_DONE = 0x0885001C
WORLD_GLOBAL = 0x08AB0820
CONTACTS = 0x450
CONTACT_COUNT = 0x2450
BODY_PROXY = 0x3A8
BODY_BOX = 0x3BC
WRITTEN = [(name, at) for name, at in BODY_FIELDS if name != "speed"]


def stop(dbg, address, condition=None):
    dbg.add_breakpoint(address, condition)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    try:
        dbg.wait_for_break(address, timeout=30.0)
    except TimeoutError:
        status = dbg.call("cpu.status")
        if not (status["stepping"] and status["pc"] == address):
            raise
    dbg.remove_breakpoint(address)


def watch(dbg, body, cond, limit=40):
    """Resume to the next `Ship_UpdateCraft`, logging every write to `body+0x140`."""
    writers = []
    dbg.call("memory.breakpoint.add", address=body + 0x140, size=12, enabled=True,
             log=False, read=False, write=True, change=False)
    dbg.add_breakpoint(SHIP_UPDATE_CRAFT, cond)
    try:
        for _ in range(limit):
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            while True:
                msg = dbg._recv()
                if msg is not None and msg.get("event") == "cpu.stepping":
                    break
            pc = msg.get("pc")
            if pc == SHIP_UPDATE_CRAFT:
                break
            r = rail.regs(dbg)
            vel = rail.f32s(dbg, body + 0x140, 3)
            writers.append({"pc": pc, "ra": r["ra"], "vel": vel})
            print("   write pc 0x%08x ra 0x%08x vel %s" % (
                pc, r["ra"], [round(x, 2) for x in vel]), file=sys.stderr)
    finally:
        dbg.call("memory.breakpoint.remove", address=body + 0x140, size=12)
        dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)
    return writers


CAGE_RESOLVE_CALL = 0x08831FF0
CAGE_SLOT = 0x08AE4CD8
CAGE_SIBLING_SLOT = 0x08AE4CDC


def cage_contacts(dbg, body, limit=8, timeout=3.0):
    """Every contact the cage resolves on `body` this frame, with its section slot."""
    out = []
    start = None
    dbg.add_breakpoint(CAGE_RESOLVE_CALL, "s0 == 0x%x" % body)
    try:
        for _ in range(limit):
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            try:
                dbg.wait_for_break(CAGE_RESOLVE_CALL, timeout=timeout)
            except TimeoutError:
                dbg.brk()
                break
            frame = int(dbg.call("cpu.status")["ticks"] / rail.CYCLES_PER_FRAME)
            if start is None:
                start = frame
            elif frame > start:
                break
            r = rail.regs(dbg)
            contact = list(struct.unpack("<16f", dbg.read(r["a0"], 0x40)))
            slot = dbg.read_u32(CAGE_SLOT)
            sibling = dbg.read_u32(CAGE_SIBLING_SLOT)
            item = {"frame": frame, "contact": contact,
                    "slot": list(struct.unpack("<36f", dbg.read(slot, 0x90))),
                    "sibling": (list(struct.unpack("<36f", dbg.read(sibling, 0x90)))
                                if rail.ram(sibling) else None),
                    "pos": rail.f32s(dbg, body + 0x30, 3),
                    "vel": rail.f32s(dbg, body + 0x140, 3)}
            out.append(item)
            print("   cage contact point %s normal %s depth %.3f friction %.3f; body %s vel %s" % (
                [round(x, 3) for x in contact[0:3]], [round(x, 3) for x in contact[4:7]],
                contact[12], contact[13], [round(x, 3) for x in item["pos"]],
                [round(x, 2) for x in item["vel"]]), file=sys.stderr)
    finally:
        dbg.remove_breakpoint(CAGE_RESOLVE_CALL)
    return out


def read_body(dbg, body):
    raw = dbg.read(body, 0x400)
    return {name: struct.unpack_from("<f", raw, at)[0] for name, at in BODY_FIELDS}


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--csv", type=Path, required=True)
    parser.add_argument("--tick", type=int, action="append", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--bisect", type=lambda t: int(t, 0), action="append", default=[],
                        help="after the narrowphase, stop at each of these pcs in order and log "
                             "the body's velocity there")
    parser.add_argument("--cage", action="store_true",
                        help="after the narrowphase, stop at the collision cage's "
                             "Body_ResolveContact call (0x08831ff0) for this body, read each "
                             "contact and the cross-section slot it was built from")
    parser.add_argument("--watch-velocity", action="store_true",
                        help="between the narrowphase and the next step, halt on every write "
                             "to the body's velocity and log the writer's pc and ra")
    args = parser.parse_args()

    rows = {int(r["tick"]): r for r in csv.DictReader(args.csv.open())}
    dbg = rail.Debugger(args.port)
    dbg.brk()
    p = rail.player(dbg)
    if p is None:
        raise SystemExit("no player craft: is a race running?")
    craft = p["craft"]
    world = dbg.read_u32(WORLD_GLOBAL)
    cond = "a0 == 0x%x" % craft
    out = args.out.open("w")
    try:
        for tick in args.tick:
            row = rows[tick]
            stop(dbg, SHIP_UPDATE_CRAFT, cond)
            body = dbg.read_u32(craft + 0x1CC)
            for name, at in WRITTEN:
                dbg.write(body + at, struct.pack("<f", float(row[name])))
            stop(dbg, NARROW_DONE)
            proxy = dbg.read_u32(body + BODY_PROXY)
            box = dbg.read_u32(body + BODY_BOX)
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
                                 "depth": f[12], "friction": f[13]})
            cf = struct.unpack_from("<120f", dbg.read(box, 0x1E0), 0)
            rec = {"tick": tick, "contacts": mine,
                   "narrow_velocity": rail.f32s(dbg, body + 0x140, 3),
                   "centre": cf[0xB0 // 4:0xB0 // 4 + 3],
                   "rows": [cf[0x80 // 4 + 4 * k:0x80 // 4 + 4 * k + 3] for k in range(3)],
                   "samples": [cf[0x130 // 4 + 4 * k:0x130 // 4 + 4 * k + 3] for k in range(10)]}
            for at in args.bisect:
                stop(dbg, at)
                vel = rail.f32s(dbg, body + 0x140, 3)
                r = rail.regs(dbg)
                rec.setdefault("bisect", []).append({"pc": at, "vel": vel, "ra": r["ra"],
                                                     "a0": r["a0"], "a1": r["a1"], "a2": r["a2"]})
                print("   at 0x%08x velocity %s ra 0x%08x a0 0x%08x a1 0x%08x a2 0x%08x" % (
                    at, [round(x, 2) for x in vel], r["ra"], r["a0"], r["a1"], r["a2"]),
                    file=sys.stderr)
            if args.cage:
                rec["cage"] = cage_contacts(dbg, body)
            if args.watch_velocity:
                rec["writers"] = watch(dbg, body, cond)
            else:
                stop(dbg, SHIP_UPDATE_CRAFT, cond)
            rec["after"] = read_body(dbg, body)
            out.write(json.dumps(rec) + "\n")
            print("tick %d: %d contacts, centre %s, velocity at the narrowphase %s, after %s" % (
                tick, len(mine), [round(x, 3) for x in rec["centre"]],
                [round(x, 2) for x in rec["narrow_velocity"]],
                [round(rec["after"]["vel_" + a], 2) for a in "xyz"]), file=sys.stderr)
            for c in mine:
                print("   point %s n %s depth %.3f friction %.3f proxy %d" % (
                    [round(x, 3) for x in c["point"]], [round(x, 3) for x in c["normal"]],
                    c["depth"], c["friction"], c["mesh_proxy"]), file=sys.stderr)
    finally:
        out.close()
        dbg.resume()


if __name__ == "__main__":
    main()
