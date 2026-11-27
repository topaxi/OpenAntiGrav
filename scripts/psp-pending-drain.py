#!/usr/bin/env python3
"""Catch the original draining a craft's pending impulse, and read which body row it pushes along.

`Ship_ApplyCollisionImpulse` (`0x0883f274`) reaches its `jal Body_ApplyImpulseAtPoint` at
`0x0883f3e0` only when the craft's pending vector (`*(entity+0x4c)+0x110`) is non-zero and no
Shield runs, so a breakpoint there stops on every drain of every craft and on nothing else. At
that stop this reads:

- the raw pending vector, still on the stack at `sp+0x40` (copied at `0x0883f334`), and the
  projected impulse already written back over the pending slot;
- the node `*(entity+0x794)`: rows at `+0x00`, `+0x10`, `+0x20`, position `+0x30`, velocity
  `+0x140`, mass `+0x374`; and the point `Body_ApplyImpulseAtPoint` is handed (`*a1`: the
  function's own second argument, kept in `a0` from `0x0883f2a0` and moved back at `0x0883f3d4`);
- the record `T = *(entity+0x4c)`: `T+0x50` is the nose point and `T+0x90` the centre
  (`contact-response.md`), so `(T+0x50 - T+0x90) / 4.875` names the forward row independently of
  any reading of the rows;
- identity checks: `craft = *(entity+0x94)`, `*(craft+0x1c4) == entity`, and the node equals
  `*(craft+0x1cc)`.

It then moves the breakpoint to `0x0883f3e8` (after the call; PPSSPP v1.20.4 fires only the
newest breakpoint) and reads the node again, so the velocity change is the call's alone.

A blast is made by firing the player's weapon from a parked grid into the craft ahead:

    scripts/ppsspp-start.sh <lane> 97 47895 data/images/pulse-psp-usa.chd
    uv run --with websocket-client python3 scripts/psp-state.py --port 47895 load 1 \\
        --file data/saves/pulse-psp/race-platinum-white-single-grid.ppst
    uv run --with websocket-client --with evdev python3 scripts/emu-run.py -- \\
        python3 scripts/psp-pending-drain.py --port 47895 --fire-frame 120 \\
        --weapon rocket --drains 4 --out data/scratch/<lane>/drain.json

Raw captures are derived game data: write them under `data/`, never commit them.
"""

import argparse
import json
import math
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

WEAPONS_DISPATCH_FIRE = 0x08861814
RACE_MANAGER = 0x08B317B4
PLAYER_OFFSET = 0x2C0
RACE_TIME = 0x2B8
FIRE_WORD = 0x1B8
DRAIN_CALL = 0x0883F3E0
DRAIN_RETURN = 0x0883F3E8
NOSE_OFFSET = 4.875
WEAPON_BITS = {"rocket": 0x80, "plasma": 0x4, "missile": 0x40, "cannon": 0x4000}


def ram(pointer):
    return 0x08800000 <= pointer < 0x0A000000


def f32s(dbg, address, count):
    return list(struct.unpack("<%df" % count, dbg.read(address, 4 * count)))


def regs(dbg):
    out = {}
    for category in dbg.call("cpu.getAllRegs")["categories"]:
        if category["name"] == "GPR":
            out.update(zip(category["registerNames"], category["uintValues"]))
    return out


def step_to(dbg, address, timeout=30.0):
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    try:
        dbg.wait_for_break(address, timeout=timeout)
    except TimeoutError:
        status = dbg.call("cpu.status")
        if not (status["stepping"] and status["pc"] == address):
            raise


def swap(dbg, old, new):
    if old is not None:
        dbg.remove_breakpoint(old)
    dbg.add_breakpoint(new)
    return new


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def norm(a):
    return math.sqrt(dot(a, a))


def analyse(rec):
    rows = [rec["node"][0:3], rec["node"][4:7], rec["node"][8:11]]
    raw = rec["raw_pending"][:3]
    dv = [a - b for a, b in zip(rec["vel_after"], rec["vel_before"])]
    nose = [(a - b) / NOSE_OFFSET for a, b in zip(rec["record_nose"], rec["record_centre"])]
    out = {
        "dv": dv,
        "dv_len": norm(dv),
        "raw_len": norm(raw),
        "dv_dot_row": [dot(dv, r) for r in rows],
        "raw_dot_row": [dot(raw, r) for r in rows],
        "nose_dot_row": [dot(nose, r) for r in rows],
        "vel_dot_row": [dot(rec["vel_before"], r) for r in rows],
        "vel_len": norm(rec["vel_before"]),
        "projected_dot_row": [dot(rec["projected"][:3], r) for r in rows],
    }
    return out


def capture_drain(dbg, rec_entity_filter=None):
    r = regs(dbg)
    entity, sp, point_ptr = r["s0"], r["sp"], r["a1"]
    node = dbg.read_u32(entity + 0x794)
    record = dbg.read_u32(entity + 0x4C)
    craft = dbg.read_u32(entity + 0x94)
    rec = {
        "entity": entity,
        "node_ptr": node,
        "record_ptr": record,
        "craft_ptr": craft,
        "craft_entity_back": dbg.read_u32(craft + 0x1C4) if ram(craft) else None,
        "craft_body_ptr": dbg.read_u32(craft + 0x1CC) if ram(craft) else None,
        "raw_pending": f32s(dbg, sp + 0x40, 4),
        "row0_copy": f32s(dbg, sp + 0x00, 4),
        "projected": f32s(dbg, record + 0x110, 4),
        "node": f32s(dbg, node, 16),
        "vel_before": f32s(dbg, node + 0x140, 3),
        "mass": f32s(dbg, node + 0x374, 1)[0],
        "point_ptr": point_ptr,
        "point": f32s(dbg, point_ptr, 4) if ram(point_ptr) else None,
        "record_nose": f32s(dbg, record + 0x50, 3),
        "record_centre": f32s(dbg, record + 0x90, 3),
        "node_block_before": dbg.read(node, 0x380).hex(),
    }
    if ram(rec["craft_body_ptr"] or 0):
        rec["craft_body_rows"] = f32s(dbg, rec["craft_body_ptr"], 12)
    return rec


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--fire-frame", type=int, default=120, help="fire this many frames after GO")
    parser.add_argument("--weapon", choices=sorted(WEAPON_BITS), default="rocket")
    parser.add_argument("--thrust", action="store_true", help="hold thrust instead of firing parked")
    parser.add_argument("--drains", type=int, default=4, help="stop after this many drains")
    parser.add_argument("--drain-timeout", type=float, default=60.0)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    dbg = Debugger(args.port)
    log = {"weapon": args.weapon, "fire_frame": args.fire_frame, "drains": []}
    current = None
    try:
        dbg.resume()
        dbg.hold(cross=args.thrust)
        dbg.brk()
        current = swap(dbg, None, WEAPONS_DISPATCH_FIRE)
        go = None
        for frame in range(4000):
            step_to(dbg, WEAPONS_DISPATCH_FIRE, timeout=60.0)
            manager = dbg.read_u32(RACE_MANAGER)
            if not ram(manager):
                continue
            if go is None:
                if f32s(dbg, manager + RACE_TIME, 1)[0] > 0.0:
                    go = frame
                    print("GO at frame %d" % frame, flush=True)
                continue
            if frame - go >= args.fire_frame:
                entity = dbg.read_u32(manager + PLAYER_OFFSET)
                record = dbg.read_u32(entity + 0x4C)
                dbg.write_u32(record + FIRE_WORD, dbg.read_u32(record + FIRE_WORD) | WEAPON_BITS[args.weapon])
                log["player_entity"] = entity
                print("fired %s at frame %d (player entity 0x%08x)" % (args.weapon, frame - go, entity), flush=True)
                break
        current = swap(dbg, current, DRAIN_CALL)
        while len(log["drains"]) < args.drains:
            try:
                step_to(dbg, DRAIN_CALL, timeout=args.drain_timeout)
            except TimeoutError:
                print("no further drain within %.0f s" % args.drain_timeout, flush=True)
                break
            rec = capture_drain(dbg)
            current = swap(dbg, current, DRAIN_RETURN)
            step_to(dbg, DRAIN_RETURN, timeout=10.0)
            rec["vel_after"] = f32s(dbg, rec["node_ptr"] + 0x140, 3)
            rec["node_block_after"] = dbg.read(rec["node_ptr"], 0x380).hex()
            rec["analysis"] = analyse(rec)
            log["drains"].append(rec)
            a = rec["analysis"]
            print("drain %d entity 0x%08x |raw| %.3f |dv| %.3f mass %.3f dv.rows %s raw.rows %s nose.rows %s vel.rows %s" % (
                len(log["drains"]), rec["entity"], a["raw_len"], a["dv_len"], rec["mass"],
                ["%.3f" % x for x in a["dv_dot_row"]], ["%.3f" % x for x in a["raw_dot_row"]],
                ["%.3f" % x for x in a["nose_dot_row"]], ["%.3f" % x for x in a["vel_dot_row"]]), flush=True)
            current = swap(dbg, current, DRAIN_CALL)
    finally:
        try:
            dbg.brk()
            if current is not None:
                dbg.remove_breakpoint(current)
            dbg.hold(cross=False)
            dbg.resume()
        finally:
            args.out.parent.mkdir(parents=True, exist_ok=True)
            args.out.write_text(json.dumps(log, indent=1))
            print("wrote %s (%d drains)" % (args.out, len(log["drains"])), flush=True)
            dbg.close()


if __name__ == "__main__":
    main()
