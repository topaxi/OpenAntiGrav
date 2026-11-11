#!/usr/bin/env python3
"""Fire one weapon in the original and log every projectile update with both of its collision queries.

Each flying weapon's update (`Rocket_Update`, `Plasma_Update`, `Missile_Update`,
`Shuriken_Update`, `Cannon_UpdateRound`) calls `Collision_SweepSegment` (`0x0883198c`) up to
twice a tick: the surface probe, then the travel sweep. This stops at the update's entry and at
the return of each call, so one run gives, per update: the entity before the update (position,
velocity, carried normal, age), each query's return code (`0x7f` nothing, `0` wall, `1` floor,
`3` mag floor, `4` outside the collision cage), the point and normal it wrote, and what is left of
`Collision_SweepSegment`'s own frame under the caller's stack pointer: the point it handed the
cage test, and the mesh raycast's hit kind (`1` mesh, `3` a craft's box) and collider index.
See `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "code 4 is the collision cage".

One breakpoint at a time (PPSSPP v1.20.4 fires only the most recently added one), rotated:
the update's entry, the probe's return, the sweep's return.

Two ways to reach the place:

- the original's own Autopilot flies the craft and the weapon fires at a spline index
  (`--spline` from `oag-trace track`, `--fire-index`); a Plasma releases 60 frames later, so
  fire it about 85 samples early;
- `--player-fire-frame N`: no Autopilot, thrust held, fire N frames after GO.

    cargo run -q -p oag-trace -- track --source data/images/pulse-psp-usa.chd \\
        --track 'Data\\Environments\\03_Track\\track.vex' > data/scratch/<lane>/03.csv
    scripts/ppsspp-start.sh <lane> 91 45791 data/images/pulse-psp-usa.chd
    uv run --with websocket-client python3 scripts/psp-state.py --port 45791 load 1 \\
        --file data/saves/pulse-psp/time-trial-moa-therma-white-venom-grid.ppst
    uv run --with websocket-client --with evdev python3 scripts/emu-run.py -- \\
        python3 scripts/psp-projectile-sweeps.py --port 45791 --spline data/scratch/<lane>/03.csv \\
        --fire-index 1415 --out data/scratch/<lane>/rocket.json rocket

`--rows-out FILE` also writes one whitespace row per update (entity index, update number, dt,
age, position, velocity, normal, probe code, sweep code, the `+0x48` word as a float) for a
replay through `oag_weapons::projectile::step`.

Raw captures are derived game data: write them under `data/`, never commit them.
"""

import argparse
import csv
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
ENTITY_CRAFT = 0x94
ENTITY_RECORD = 0x4C
ENTITY_BODY = 0x794
AUTOPILOT_BIT = 0x1000
FIRE_WORD = 0x1B8
HELD_WORD = 0x1BC
AUTO_TIMER = 0x148
CYCLES_PER_FRAME = 222_000_000 / 59.940059940059946

# update entry, return of the probe call, return of the sweep call, the in-function stack offset of
# `next` (point and normal follow at +0x10 and +0x20), and the entity's field offsets.
WEAPONS = {
    "rocket": dict(bit=0x80, update=0x0885D2A8, ret1=0x0885D40C, ret2=0x0885D780, stack=0x0,
                   pos=0x90, vel=0xE0, prev=0xF0, normal=0x100, age=0x48),
    "plasma": dict(bit=0x4, update=0x0885C6CC, ret1=0x0885C7F4, ret2=0x0885CACC, stack=0x0,
                   pos=0xA0, vel=0xF0, prev=0x100, normal=0x110, age=0x54),
    "missile": dict(bit=0x40, update=0x0885A918, ret1=0x0885AA5C, ret2=0x0885ACDC, stack=0x10,
                    pos=0xA0, vel=0xB0, prev=0xC0, normal=0xD0, age=0x50),
    "shuriken": dict(bit=0x20000, update=0x08877BDC, ret1=0x08877D1C, ret2=0x08877E90, stack=0x0,
                     pos=0x120, vel=0x130, prev=0x140, normal=0x150, age=0x48),
    # `Cannon_UpdateRound`: one sweep and no probe; it carries no ridden normal (`+0x60` is a basis row).
    "cannon": dict(bit=0x4000, update=0x0886593C, ret1=None, ret2=0x08865A28, stack=0x10,
                   pos=0x80, vel=0x90, prev=0xA0, normal=0x60, age=0x48),
}


def ram(pointer):
    return 0x08800000 <= pointer < 0x0A000000


def f32s(dbg, address, count):
    return list(struct.unpack("<%df" % count, dbg.read(address, 4 * count)))


def regs(dbg):
    out = {}
    for category in dbg.call("cpu.getAllRegs")["categories"]:
        if category["name"] in ("GPR", "FPU"):
            out.update(zip(category["registerNames"], category["uintValues"]))
    return out


def player(dbg):
    manager = dbg.read_u32(RACE_MANAGER)
    if not ram(manager):
        return None
    entity = dbg.read_u32(manager + PLAYER_OFFSET)
    if not ram(entity):
        return None
    craft = dbg.read_u32(entity + ENTITY_CRAFT)
    record = dbg.read_u32(entity + ENTITY_RECORD)
    body = dbg.read_u32(entity + ENTITY_BODY)
    if not (ram(craft) and ram(record) and ram(body)):
        return None
    return {"entity": entity, "craft": craft, "record": record, "body": body}


def load_spline(path):
    rows = [r for r in csv.DictReader(open(path)) if r["path"] == "0"]
    return [(int(r["index"]), float(r["pos_x"]), float(r["pos_y"]), float(r["pos_z"])) for r in rows]


def nearest(spline, pos):
    best = None
    for index, x, y, z in spline:
        d = (x - pos[0]) ** 2 + (y - pos[1]) ** 2 + (z - pos[2]) ** 2
        if best is None or d < best[1]:
            best = (index, d)
    return best[0], math.sqrt(best[1])


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
    if old == new:
        return
    if old is not None:
        dbg.remove_breakpoint(old)
    dbg.add_breakpoint(new)


def flat_rows(log):
    """One row per update: entity index, update number, dt, age, pos, vel, normal, codes, `+0x48`."""
    entities, counts, lines = [], {}, []
    for update in log["updates"]:
        if update["entity"] not in entities:
            entities.append(update["entity"])
        entity = entities.index(update["entity"])
        counts[entity] = counts.get(entity, 0) + 1
        word = struct.unpack("<f", struct.pack("<I", update["words"][0x48 // 4]))[0]
        fields = [entity, counts[entity], repr(update["dt"]), repr(update["age"])]
        fields += [repr(v) for v in update["pos"] + update["vel"] + update["normal"]]
        fields += [update.get("probe", {}).get("code", -1), update.get("sweep", {}).get("code", -1),
                   repr(word)]
        lines.append(" ".join(str(f) for f in fields))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("weapon", choices=sorted(WEAPONS))
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--spline", help="`oag-trace track` CSV of the circuit (Autopilot route)")
    parser.add_argument("--fire-index", type=int, default=0,
                        help="fire when the craft's nearest path-0 sample reaches this index")
    parser.add_argument("--rows-out", type=Path, help="also write one flat row per update")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--arm-after", type=int, default=30)
    parser.add_argument("--frames", type=float, default=400.0, help="probe this many frames after the fire")
    parser.add_argument("--max-frames", type=int, default=4000)
    parser.add_argument("--budget", type=float, default=520.0)
    parser.add_argument("--no-fire", action="store_true", help="only log the craft's path")
    parser.add_argument("--player-fire-frame", type=int,
                        help="no Autopilot: hold thrust and fire this many frames after GO")
    args = parser.parse_args()
    weapon = WEAPONS[args.weapon]
    if args.player_fire_frame is None and not args.spline:
        parser.error("--spline is needed unless --player-fire-frame is given")
    spline = load_spline(args.spline) if args.spline else []
    dbg = Debugger(args.port)
    log = {"weapon": args.weapon, "fire_index": args.fire_index, "path": [], "updates": []}
    began = time.time()
    current = None
    try:
        dbg.resume()
        dbg.hold(cross=True)
        dbg.brk()
        dbg.add_breakpoint(WEAPONS_DISPATCH_FIRE)
        current = WEAPONS_DISPATCH_FIRE
        go = None
        armed = False
        p = None
        fired = None
        for frame in range(args.max_frames):
            if time.time() - began > args.budget:
                print("budget reached before the fire", flush=True)
                break
            step_to(dbg, WEAPONS_DISPATCH_FIRE, timeout=60.0)
            p = player(dbg)
            if p is None:
                continue
            if go is None:
                if f32s(dbg, p["craft"] + 0x2B8, 1)[0] > 0.0:
                    go = frame
                    print("GO at frame %d" % frame, flush=True)
                continue
            if args.player_fire_frame is not None:
                if frame - go >= args.player_fire_frame:
                    pose = f32s(dbg, p["body"], 16)
                    vel = f32s(dbg, p["body"] + 0x140, 3)
                    log["fire"] = {"frame": frame - go, "index": -1, "pose": pose, "vel": vel}
                    dbg.write_u32(p["record"] + FIRE_WORD,
                                  dbg.read_u32(p["record"] + FIRE_WORD) | weapon["bit"])
                    fired = frame
                    print("player fired %s at frame %d pos (%.1f,%.1f,%.1f)" % (
                        args.weapon, frame - go, pose[12], pose[13], pose[14]), flush=True)
                    break
                continue
            if not armed and frame - go >= args.arm_after:
                dbg.write_u32(p["record"] + HELD_WORD, 0xFFFFFFFF)
                dbg.write_u32(p["record"] + FIRE_WORD,
                              dbg.read_u32(p["record"] + FIRE_WORD) | AUTOPILOT_BIT)
                armed = True
                arm_frame = frame
                continue
            if armed and frame == arm_frame + 2:
                dbg.write(p["record"] + AUTO_TIMER, struct.pack("<f", 1.0e9))
                print("autopilot armed", flush=True)
            pose = f32s(dbg, p["body"], 16)
            index, distance = nearest(spline, pose[12:15]) if spline else (-1, 0.0)
            if frame % 6 == 0:
                log["path"].append({"frame": frame - go, "pos": pose[12:15], "up": pose[4:7],
                                    "fwd": pose[8:11], "index": index, "d": distance})
            if frame % 120 == 0:
                print("frame %d pos (%.0f,%.0f,%.0f) spline#%d d=%.1f" % (
                    frame - go, pose[12], pose[13], pose[14], index, distance), flush=True)
            if armed and frame > arm_frame + 5 and args.fire_index <= index < args.fire_index + 300 \
                    and distance < 60.0:
                if args.no_fire:
                    break
                vel = f32s(dbg, p["body"] + 0x140, 3)
                log["fire"] = {"frame": frame - go, "index": index, "pose": pose, "vel": vel}
                dbg.write_u32(p["record"] + FIRE_WORD,
                              dbg.read_u32(p["record"] + FIRE_WORD) | weapon["bit"])
                fired = frame
                print("fired %s at frame %d spline#%d pos (%.1f,%.1f,%.1f)" % (
                    args.weapon, frame - go, index, pose[12], pose[13], pose[14]), flush=True)
                break
        if fired is None:
            return
        start = dbg.call("cpu.status")["ticks"]
        swap(dbg, current, weapon["update"])
        current = weapon["update"]
        last_print = time.time()
        while time.time() - began < args.budget:
            try:
                step_to(dbg, current, timeout=20.0)
            except TimeoutError:
                print("probe quiet: the projectile is gone", flush=True)
                break
            now = (dbg.call("cpu.status")["ticks"] - start) / CYCLES_PER_FRAME
            if now > args.frames:
                break
            r = regs(dbg)
            if current == weapon["update"]:
                entity = r["a0"]
                blob = dbg.read(entity, 0x180)
                rec = {"frame": round(now, 2), "entity": entity,
                       "dt": struct.unpack("<f", struct.pack("<I", r["f12"]))[0],
                       "flags": struct.unpack_from("<I", blob, 0x3C)[0],
                       "age": struct.unpack_from("<f", blob, weapon["age"])[0],
                       "pos": list(struct.unpack_from("<3f", blob, weapon["pos"])),
                       "vel": list(struct.unpack_from("<3f", blob, weapon["vel"])),
                       "normal": list(struct.unpack_from("<3f", blob, weapon["normal"])),
                       "words": list(struct.unpack("<%dI" % (0x180 // 4), blob)),
                       "craft_pose": f32s(dbg, p["body"], 16),
                       "craft_vel": f32s(dbg, p["body"] + 0x140, 3)}
                log["updates"].append(rec)
                probing = weapon["ret1"] is not None and (
                    args.weapon != "rocket" or not rec["flags"] & 4)
                new = weapon["ret1"] if probing else weapon["ret2"]
            else:
                rec = log["updates"][-1]
                stack = f32s(dbg, r["sp"] + weapon["stack"], 12)
                entry = {"code": r["v0"], "next": stack[0:3], "point": stack[4:7], "normal": stack[8:11]}
                # `Collision_SweepSegment`'s own frame, still intact under the caller's sp: the
                # segment end it handed the cage test (-0x70), the mesh hit's point (-0x60) and
                # normal (-0x50), collider index (-0x40) and kind (-0x38, 1 mesh / 3 box).
                below = dbg.read(r["sp"] - 0x70, 0x70)
                entry["cage_point"] = list(struct.unpack_from("<3f", below, 0x00))
                entry["mesh_point"] = list(struct.unpack_from("<3f", below, 0x10))
                entry["mesh_collider"] = struct.unpack_from("<i", below, 0x30)[0]
                entry["mesh_kind"] = struct.unpack_from("<i", below, 0x38)[0]
                if current == weapon["ret1"]:
                    rec["probe"] = entry
                    new = weapon["ret2"]
                else:
                    rec["sweep"] = entry
                    blob = dbg.read(rec["entity"], 0x180)
                    rec["vel_after"] = list(struct.unpack_from("<3f", blob, weapon["vel"]))
                    rec["normal_after"] = list(struct.unpack_from("<3f", blob, weapon["normal"]))
                    rec["flags_after"] = struct.unpack_from("<I", blob, 0x3C)[0]
                    new = weapon["update"]
            swap(dbg, current, new)
            current = new
            if time.time() - last_print > 30.0:
                last_print = time.time()
                print("frame %.1f, %d updates" % (now, len(log["updates"])), flush=True)
    finally:
        try:
            dbg.brk()
            if current is not None:
                dbg.remove_breakpoint(current)
            dbg.hold(cross=False)
            dbg.resume()
        except Exception as error:  # noqa: BLE001
            print("cleanup: %s" % error, flush=True)
        args.out.write_text(json.dumps(log))
        if args.rows_out:
            args.rows_out.write_text(flat_rows(log))
        print("wrote %s: %d path rows, %d updates" % (args.out, len(log["path"]), len(log["updates"])),
              flush=True)


if __name__ == "__main__":
    main()
