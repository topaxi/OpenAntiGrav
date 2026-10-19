#!/usr/bin/env python3
"""The original's autopilot against the walls: per-frame lateral offset and contacts.

Arms the player's Autopilot pickup with a raised timer after GO (the method
`scripts/psp-postrace.py` uses: fire bit `0x1000`, then `record+0x148` written
huge), so the original's own player driver flies the race. Then it stops once a
frame inside `Ai_SnapToLane` (`0x088542e4`) for the player's driver, right after
`Ai_LanePoint` has written the lane point to `sp+0x30` (`0x08854340`), and logs:

- the lane point `P` and the body's pose (`body+0x00` rows left, up, forward,
  position), so `d = pos - P` and its component across the track, plus the
  racer's position copy (`racer+0x260+0x30`, what the rail reads);
- the rail's blend and time (`driver+0x48`, `+0x4c`) and the smoothed speed
  (`driver+0x170`);
- the driver's control record (`driver+0x08`: steer, thrust, left and right
  airbrake, pitch), the craft's ground fraction (`craft+0x2b0`) and the
  controls the craft consumed (`craft+0x2b8..`);
- the body's contact count (`body+0x370`, the previous world step's) and each
  contact's 64-byte record (`body+0x170 + i * 0x40`), the records
  `FUN_088418e0`'s contact loop walks.

One breakpoint only: PPSSPP v1.20.4 fires only the most recently added one.

    uv run --with websocket-client python3 scripts/psp-autopilot-rail.py \\
        --port 45696 --out data/scratch/<lane>/talons-venom.json --restart

`--stint SECONDS` arms the authored pickup timer instead of a huge one, after
`--arm-after` frames of the pad holding thrust and steering `--steer` (a crude
off-line handover); the default flies the whole race.

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
from ppsspp_debugger import Debugger  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "pair", Path(__file__).resolve().parent / "psp-weapon-pair.py"
)
pair = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pair)

WEAPONS_DISPATCH_FIRE = 0x08861814
SNAP_AFTER_LANE_POINT = 0x08854340
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


def ram(pointer):
    return 0x08800000 <= pointer < 0x0A000000


def f32s(dbg, address, count):
    return list(struct.unpack("<%df" % count, dbg.read(address, 4 * count)))


def regs(dbg):
    category = dbg.call("cpu.getAllRegs")["categories"][0]
    return dict(zip(category["registerNames"], category["uintValues"]))


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
    return {"entity": entity, "craft": craft, "record": record, "body": body,
            "driver": dbg.read_u32(craft + 0x40) - 8}


def arm(dbg, args, log):
    """Wait for GO on `Weapons_DispatchFire`, then arm the pickup."""
    go = None
    frame = 0
    p = None
    dbg.brk()
    dbg.add_breakpoint(WEAPONS_DISPATCH_FIRE)
    try:
        while True:
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            dbg.wait_for_break(WEAPONS_DISPATCH_FIRE, timeout=60.0)
            p = player(dbg)
            if p is None:
                continue
            throttle = f32s(dbg, p["craft"] + 0x2B8, 1)[0]
            if go is None and throttle > 0.0:
                go = frame
                if args.steer:
                    dbg.hold(cross=True, **{args.steer: True})
            if go is not None and frame - go >= args.arm_after:
                dbg.write_u32(p["record"] + HELD_WORD, 0xFFFFFFFF)
                dbg.write_u32(p["record"] + FIRE_WORD,
                              dbg.read_u32(p["record"] + FIRE_WORD) | AUTOPILOT_BIT)
                break
            frame += 1
            if frame > 3000:
                raise SystemExit("no GO within 3000 frames")
        # Two more dispatches: the handler arms the timer from the stats.
        for _ in range(2):
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            dbg.wait_for_break(WEAPONS_DISPATCH_FIRE, timeout=30.0)
    finally:
        dbg.brk()
        dbg.remove_breakpoint(WEAPONS_DISPATCH_FIRE)
    timer = f32s(dbg, p["record"] + AUTO_TIMER, 1)[0]
    log["events"].append({"armed_frame_after_go": frame - go, "authored_timer": timer})
    print("autopilot armed %d frames after GO, authored timer %.2f s" % (frame - go, timer),
          file=sys.stderr)
    if args.stint is None:
        dbg.write(p["record"] + AUTO_TIMER, struct.pack("<f", 1.0e9))
    else:
        dbg.write(p["record"] + AUTO_TIMER, struct.pack("<f", args.stint))
    if args.steer:
        dbg.hold(cross=True, **{args.steer: False})
    return p


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--restart", action="store_true", help="pause -> RESTART RACE first")
    parser.add_argument("--arm-after", type=int, default=60)
    parser.add_argument("--stint", type=float, help="arm this timer instead of a huge one")
    parser.add_argument("--steer", choices=["left", "right"], help="hold this before the stint")
    parser.add_argument("--frames", type=int, default=12000, help="stop after this many samples")
    parser.add_argument("--budget", type=float, default=540.0, help="wall-clock seconds")
    args = parser.parse_args()

    dbg = Debugger(args.port)
    log = {"args": {k: str(v) for k, v in vars(args).items()}, "events": [], "frames": []}
    start = time.time()
    try:
        if args.restart:
            pair.restart_to_countdown(dbg, hold=True)
        else:
            dbg.resume()
            dbg.hold(cross=True)
        p = arm(dbg, args, log)
        dbg.add_breakpoint(SNAP_AFTER_LANE_POINT, "s0 == 0x%x" % p["driver"])
        try:
            for n in range(args.frames):
                if time.time() - start > args.budget:
                    log["events"].append({"stopped": "budget", "samples": n})
                    break
                dbg.call("cpu.status")
                dbg.pending = []
                dbg.call("cpu.resume")
                try:
                    dbg.wait_for_break(SNAP_AFTER_LANE_POINT, timeout=30.0)
                except TimeoutError:
                    # v1.20.4 can answer a resume with a stop the wait never sees.
                    status = dbg.call("cpu.status")
                    if not (status["stepping"] and status["pc"] == SNAP_AFTER_LANE_POINT):
                        raise
                    print("missed stop recovered", file=sys.stderr)
                status = dbg.call("cpu.status")
                r = regs(dbg)
                sp, s1 = r["sp"], r["s1"]
                lane = f32s(dbg, sp + 0x30, 3)
                pose = f32s(dbg, p["body"], 16)
                racer_pose = f32s(dbg, s1 + 0x30, 3)
                rail = f32s(dbg, p["driver"] + 0x48, 2)
                vel = f32s(dbg, p["body"] + 0x140, 3)
                contacts = dbg.read_u32(p["body"] + 0x370)
                row = {"cycle_frame": int(status["ticks"] / CYCLES_PER_FRAME),
                       "lane": lane, "left": pose[0:3], "up": pose[4:7], "fwd": pose[8:11],
                       "pos": pose[12:15], "racer_pos": racer_pose, "vel": vel, "blend": rail[0], "rail_time": rail[1],
                       "smoothed_speed": f32s(dbg, p["driver"] + 0x170, 1)[0],
                       "flags1c0": dbg.read_u32(p["craft"] + 0x1C0),
                       "blend_1d4": f32s(dbg, p["craft"] + 0x1D4, 1)[0],
                       "timer": f32s(dbg, p["record"] + AUTO_TIMER, 1)[0],
                       "contacts": contacts,
                       "controls": f32s(dbg, p["driver"] + 0x08, 5),
                       "grounded": f32s(dbg, p["craft"] + 0x2B0, 1)[0],
                       "craft_controls": f32s(dbg, p["craft"] + 0x2B8, 5)}
                if 0 < contacts <= 8:
                    raw = dbg.read(p["body"] + 0x170, 0x40 * contacts)
                    row["contact_records"] = [
                        list(struct.unpack_from("<16f", raw, 0x40 * i)) for i in range(contacts)
                    ]
                    row["contact_words"] = [
                        list(struct.unpack_from("<16I", raw, 0x40 * i)) for i in range(contacts)
                    ]
                finished = dbg.read(p["entity"] + 0x912, 1)[0]
                row["finished"] = finished
                log["frames"].append(row)
                if n % 600 == 0:
                    print("sample %d frame %d contacts %d blend %.2f" % (
                        n, row["cycle_frame"], contacts, rail[0]), file=sys.stderr)
                if finished:
                    log["events"].append({"finished_at_sample": n})
                    break
        finally:
            dbg.brk()
            dbg.remove_breakpoint(SNAP_AFTER_LANE_POINT)
            dbg.resume()
    finally:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(log))
        print("wrote %s (%d frames)" % (args.out, len(log["frames"])), file=sys.stderr)


if __name__ == "__main__":
    main()
