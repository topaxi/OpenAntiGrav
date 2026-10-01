#!/usr/bin/env python3
"""Put a craft into Ship_SetState(entity, 4) on a running PPSSPP and photograph what follows.

The emulator half of the wreck comparison (`docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md`):
a craft in state 4 explodes for 0.5 s, state 5 swaps its wreck model in (the frame the death
sparks spawn), state 6 follows 1.5 s later. A real race reaches this only by draining a shield,
which is hard to time, so this calls the function itself.

How the call is made: at a `Ship_UpdateCraft` stop for the target craft the registers are saved,
`a0`/`a1`/`ra`/`pc` are pointed at `Ship_SetState(entity, 4)` with `ra` back at the same
breakpoint, the CPU runs until it returns there, and every register is restored. The craft
carries on as if its shield had just run out.

    python3 scripts/psp-wreck-capture.py --port 45682 --display :92 --out DIR \\
        [--restart --nearest --inject-frame 40]            # an opponent, during the countdown
    python3 scripts/psp-wreck-capture.py ... --spawns      # log every Psys_Spawn_q instead

`--restart` walks the pause menu's RESTART RACE first (the same walk
`scripts/psp-weapon-pair.py` uses), so the opponents are still on the grid and the chase camera
sees them. Without it the player's own craft is wrecked and the original's destroy camera
(`Camera_SetMode(5)`) takes over. Photographs are the emulator window at 480x272, the CPU
stopped, so `kNNN.png` shows the world about two frames before the stop frame.

Raw captures are derived game data: write them under `data/`, never commit them.
"""

import argparse
import importlib.util
import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "pair", Path(__file__).resolve().parent / "psp-weapon-pair.py"
)
pair = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pair)

SHIP_UPDATE_CRAFT = 0x08849618
SET_STATE = 0x08844100
PSYS_SPAWN_Q = 0x08915484
RACE_MANAGER = 0x08B317B4
CAMERA_OBJECT = 0x08B32C64
CAMERA_NODE_BASE = 0x08AB10B0
CYCLES_PER_FRAME = 222_000_000 / 59.940059940059946


def regs(dbg):
    c = dbg.call("cpu.getAllRegs")["categories"][0]
    return dict(zip(c["registerNames"], c["uintValues"]))


def camera_row(dbg):
    """The camera controller's own fields (`camera.md`, "destroy camera") and the node it drives."""
    cam = dbg.read_u32(CAMERA_OBJECT)
    row = {"mode": dbg.read_u32(cam + 0x1DC), "subject": hex(dbg.read_u32(cam + 0x1E4))}
    f = lambda off, n=1: list(struct.unpack("<%df" % n, dbg.read(cam + off, 4 * n)))
    row["fov"], row["fov_target"], row["fov_rate"] = f(0x220)[0], f(0x224)[0], f(0x228)[0]
    row["focus"], row["focus_target"] = f(0x230, 3), f(0x240, 3)
    row["focus_rate"], row["frame_size"] = f(0x250)[0], f(0x268)[0]
    node = dbg.read_u32(cam + 0x1D4)
    row["node"] = hex(node)
    if node:
        row["node_floats_0x40_0xc0"] = list(struct.unpack("<32f", dbg.read(node + 0x40, 128)))
    node_base = dbg.read_u32(CAMERA_NODE_BASE)
    row["flags_10e8"] = hex(dbg.read_u32(dbg.read_u32(0x08AB10E8) + 0x2C))
    row["flags_10b0"] = hex(dbg.read_u32(node_base + 0x2C))
    row["view_node_0x40"] = list(struct.unpack("<16f", dbg.read(node_base + 0x40, 64)))
    return row


def position(dbg, entity):
    craft = dbg.read_u32(entity + 0x94)
    return struct.unpack("<3f", dbg.read(dbg.read_u32(craft + 0x1CC) + 0x30, 12))


def pick_entity(dbg, args):
    manager = dbg.read_u32(RACE_MANAGER)
    player = dbg.read_u32(manager + 0x2C0)
    if not args.nearest:
        return dbg.read_u32(manager + 0x78 + 4 * args.slot) if args.slot >= 0 else player
    origin = position(dbg, player)
    best = None
    for index in range(8):
        entity = dbg.read_u32(manager + 0x78 + 4 * index)
        if entity == player or not pair.ram(entity):
            continue
        p = position(dbg, entity)
        distance = sum((p[k] - origin[k]) ** 2 for k in range(3)) ** 0.5
        print("slot %d entity %#x at %.1f" % (index, entity, distance), file=sys.stderr)
        if best is None or distance < best[0]:
            best = (distance, entity)
    return best[1]


def inject(dbg, entity, state):
    """Call Ship_SetState(entity, state) from the current Ship_UpdateCraft stop."""
    saved = regs(dbg)
    for name, value in (("a0", entity), ("a1", state), ("ra", SHIP_UPDATE_CRAFT), ("pc", SET_STATE)):
        dbg.call("cpu.setReg", name=name, value=value)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    dbg.wait_for_break(SHIP_UPDATE_CRAFT, timeout=30.0)
    for name, value in saved.items():
        if name != "zero":
            dbg.call("cpu.setReg", name=name, value=value)


def log_spawns(dbg, entity, seconds):
    start = dbg.call("cpu.status")["ticks"]
    log = []
    try:
        for _ in dbg.each_hit(PSYS_SPAWN_Q, 100000, timeout=20.0):
            now = (dbg.call("cpu.status")["ticks"] - start) / CYCLES_PER_FRAME
            if now > seconds * 60:
                break
            r = regs(dbg)
            name = dbg.read_cstring(r["a1"], 40) if pair.ram(r["a1"]) else None
            log.append(
                {
                    "frame": round(now, 1),
                    "name": name,
                    "parent": hex(dbg.read_u32(r["a0"] + 8)),
                    "state": dbg.read_u32(entity + 0x8C),
                }
            )
    except TimeoutError:
        pass
    return log


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--display", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--slot", type=int, default=-1, help="manager grid slot to wreck (default the player)")
    ap.add_argument("--nearest", action="store_true", help="wreck the opponent nearest the player")
    ap.add_argument("--restart", action="store_true", help="RESTART RACE first; inject during the countdown")
    ap.add_argument("--inject-frame", type=int, default=5)
    ap.add_argument("--state", type=int, default=4)
    ap.add_argument("--shots", default="0,5,10,20,30,36,40,50,60,75,90,105,120,140,160,180")
    ap.add_argument("--spawns", action="store_true", help="log Psys_Spawn_q for 3.3 s instead of photographing")
    ap.add_argument("--place-window", action="store_true")
    ap.add_argument("--camera", action="store_true", help="log the camera controller's fields each frame")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    shots = sorted({int(v) for v in args.shots.split(",")})
    if args.place_window:
        pair.place_window(args.display)

    dbg = Debugger(args.port)
    log = []
    try:
        if args.restart:
            pair.restart_to_countdown(dbg, hold=False)
        dbg.brk()
        entity = pick_entity(dbg, args)
        craft = dbg.read_u32(entity + 0x94)
        print("entity %#x craft %#x" % (entity, craft), file=sys.stderr)
        injected = None
        frame = 0
        for _ in dbg.each_hit(SHIP_UPDATE_CRAFT, 100000, timeout=30.0):
            if regs(dbg)["a0"] != craft:
                continue
            if injected is None and frame >= args.inject_frame:
                inject(dbg, entity, args.state)
                injected = frame
                print("Ship_SetState(%d) injected at frame %d" % (args.state, frame), file=sys.stderr)
                if args.spawns:
                    break
            row = {"frame": frame, "state": dbg.read_u32(entity + 0x8C)}
            live, hull, wreck = (dbg.read_u32(entity + o) for o in (0x8B0, 0x8B4, 0x8B8))
            row["live"] = "wreck" if live == wreck else "hull"
            row["hull_flags"] = dbg.read_u32(hull + 0x2C)
            row["wreck_flags"] = dbg.read_u32(wreck + 0x2C) if wreck else None
            row["timer"] = struct.unpack("<f", dbg.read(entity + 0x874, 4))[0]
            if args.camera:
                row["camera"] = camera_row(dbg)
            if injected is not None:
                k = frame - injected
                row["since"] = k
                if k in shots:
                    name = "k%03d.png" % k
                    row["shot"] = name if pair.shoot(args.display, args.out / name) else None
                if k >= max(shots):
                    log.append(row)
                    break
            log.append(row)
            frame += 1
        if args.spawns:
            log = log_spawns(dbg, entity, 3.3)
        dbg.resume()
    finally:
        (args.out / ("spawns.json" if args.spawns else "log.json")).write_text(json.dumps(log, indent=1))
        dbg.close()
    print("wrote", args.out)


if __name__ == "__main__":
    main()
