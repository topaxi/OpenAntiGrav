#!/usr/bin/env python3
"""Log every craft of a running Pulse PSP race, one row per opponent tick, to a JSONL file.

The question this answers is the one `psp-ai-thrust.py` cannot: *where does the original's
field go wrong, if it does*. That script samples one driver's thrust law per tick with
everything the law reads; this one samples **every craft** each time the law returns
(`0x088568b0`, about seven ticks in eight): position, velocity, speed, the AI's thrust
output, the throttle the craft actually applied, progress and place, and the driver's `spread` (`driver+0x74`, the lateral slot the law steers to). A fall at a jump, a
respawn (a position that jumps back along the track), or an opponent held below a speed
the circuit needs shows up in the rows, and the sampling is tick-exact because the CPU is
stopped at each hit rather than polled.

Run it attached to an emulator that is already in a race (`psp-drive.py menu`, see
`docs/reverse-engineering/ppsspp-debugger.md`). `--park` holds no buttons (the player
sits on the grid, last). `--skill N` writes `g_skill_level` (`0x08b31044`): do it, then
`psp-drive.py restart`, *then* capture, because the skill scale is read once at race start.
`--seconds` is race-clock seconds to log from the first row.

    uv run --with websocket-client python3 scripts/psp-ai-track-log.py --port 45091 \
        --out data/captures/pulse-13-venom-lvl0-a.jsonl --seconds 150

Rows are `{"t": race_clock, "w": wall_seconds, "slots": [[...] x racers]}`; the first row
carries `meta` (lap length, start offset, skill level, the row layout). Raw captures are
derived game data: write them under `data/`, never commit them.
"""

import argparse
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

THRUST_RETURN = 0x088568B0
SNAP_SET_POSITION = 0x088548DC
RACE_MANAGER = 0x08B317B4
RACERS = 0x08B34420
RACER_STRIDE = 0x370
RACER_COUNT = 0x08B35FA0
PLAYER_RACER = 0x08B34410
G_SKILL_LEVEL = 0x08B31044
G_RACE_LAPS = 0x08B30F9C
START_OFFSET = 0x7AC
RACE_TIME = 0x2B8

BODY_BLOCK = (0x30, 0x3A0)  # position at +0x30, velocity at +0x140, speed at +0x398
CRAFT_BLOCK = (0x2B0, 0x2F0)  # throttle +0x2B8, steer +0x2C0, cached speed +0x2EC
LAYOUT = [
    "x", "y", "z", "vx", "vy", "vz", "speed", "ai_thrust", "throttle", "steer",
    "progress", "place", "finished", "body", "spread",
]


def hits(dbg, address, timeout=120.0):
    """Yield once per time the CPU stops at `address`, leaving it stopped, until closed."""
    dbg.brk()
    dbg.add_breakpoint(address)
    try:
        while True:
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            try:
                yield dbg.wait_for_break_any((address,), timeout=timeout)
            except TimeoutError:
                status = dbg.call("cpu.status")
                if status["stepping"] and status["pc"] == address:
                    yield status
                else:
                    raise
    finally:
        dbg.brk()
        dbg.remove_breakpoint(address)


def regs(dbg, name):
    registers = dbg.call("cpu.getAllRegs")
    category = next(c for c in registers["categories"] if c["name"] == name)
    return dict(zip(category["registerNames"], category["uintValues"]))


def rail_record(dbg, t):
    """`Ai_SnapToLane`'s `Body_SetPosition` call: which driver, where the rail puts it, where it was."""
    r = regs(dbg, "GPR")
    driver = r["s0"]
    target = struct.unpack("<3f", dbg.read(r["a1"], 12))
    current = struct.unpack("<3f", dbg.read(r["a0"] + 0x30, 12))
    return {"rail": [t, driver, *target, *current, dbg.read_f32(driver + 0x48)]}


def pointers(dbg, count):
    """Per racer: entity, driver, craft, body. Re-read now and then: a respawn may move them."""
    block = dbg.read(RACERS, RACER_STRIDE * count)
    rows = []
    for i in range(count):
        entity, driver = struct.unpack_from("<II", block, i * RACER_STRIDE)
        craft = dbg.read_u32(entity + 0x94)
        body = dbg.read_u32(entity + 0x794)
        rows.append((entity, driver, craft, body))
    return rows


def sample(dbg, count, ptrs):
    block = dbg.read(RACERS, RACER_STRIDE * count)
    slots = []
    for i in range(count):
        base = i * RACER_STRIDE
        progress = struct.unpack_from("<f", block, base + 0x240)[0]
        place = struct.unpack_from("<I", block, base + 0x344)[0]
        finished = block[base + 0x82]
        _, driver, craft, body = ptrs[i]
        raw = dbg.read(body + BODY_BLOCK[0], BODY_BLOCK[1] - BODY_BLOCK[0])
        x, y, z = struct.unpack_from("<3f", raw, 0)
        vx, vy, vz = struct.unpack_from("<3f", raw, 0x140 - BODY_BLOCK[0])
        speed = struct.unpack_from("<f", raw, 0x398 - BODY_BLOCK[0])[0]
        ai_thrust = struct.unpack_from("<f", dbg.read(driver + 0x0C, 4))[0]
        craft_raw = dbg.read(craft + CRAFT_BLOCK[0], CRAFT_BLOCK[1] - CRAFT_BLOCK[0])
        throttle = struct.unpack_from("<f", craft_raw, 0x2B8 - CRAFT_BLOCK[0])[0]
        steer = struct.unpack_from("<f", craft_raw, 0x2C0 - CRAFT_BLOCK[0])[0]
        spread = struct.unpack_from("<f", dbg.read(driver + 0x74, 4))[0]
        slots.append(
            [x, y, z, vx, vy, vz, speed, ai_thrust, throttle, steer, progress, place, finished, body, spread]
        )
    return slots


def capture(args):
    from ppsspp_debugger import Debugger

    dbg = Debugger(args.port)
    if args.skill is not None:
        dbg.write_u32(G_SKILL_LEVEL, args.skill)
        print("g_skill_level written: %d (restart the race before capturing)" % args.skill)
        dbg.close()
        return
    dbg.resume()
    if args.park:
        dbg.hold(cross=False)
    manager = dbg.read_u32(RACE_MANAGER)
    count = dbg.read_u32(RACER_COUNT)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    out = args.out.open("w")
    started = time.time()
    first = None
    last_t = None
    ptrs = None
    rows = 0
    for _ in hits(dbg, SNAP_SET_POSITION if args.rail else THRUST_RETURN):
        t = dbg.read_f32(manager + RACE_TIME)
        if args.rail and first is not None:
            out.write(json.dumps(rail_record(dbg, t), separators=(",", ":")) + "\n")
        if first is None:
            first = t
            ptrs = pointers(dbg, count)
            driver = ptrs[0][1]
            meta = {
                "layout": LAYOUT,
                "racers": count,
                "skill_level": dbg.read_u32(G_SKILL_LEVEL),
                "laps": dbg.read_u32(G_RACE_LAPS),
                "start_offset": dbg.read_f32(manager + START_OFFSET),
                "lap_length": dbg.read_f32(dbg.read_u32(driver + 0x34)),
                "player_racer": dbg.read_u32(PLAYER_RACER),
                "racer_base": RACERS,
                "racer_stride": RACER_STRIDE,
                "parked": args.park,
            }
        if t == last_t:
            continue
        last_t = t
        if rows % 400 == 399:
            ptrs = pointers(dbg, count)
        row = {"t": t, "w": round(time.time() - started, 2), "slots": sample(dbg, count, ptrs)}
        if rows == 0:
            row["meta"] = meta
        out.write(json.dumps(row, separators=(",", ":")) + "\n")
        rows += 1
        if rows % 100 == 0:
            out.flush()
            lead = min(row["slots"], key=lambda s: s[11])
            print("row %d t=%.2f lead place %d prog %.0f speed %.1f" % (rows, t, lead[11], lead[10], lead[6]), file=sys.stderr, flush=True)
        if t - first >= args.seconds:
            break
    out.close()
    dbg.resume()
    dbg.close()
    print("wrote %s: %d rows, %.1f race seconds" % (args.out, rows, (last_t or 0) - (first or 0)))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument("--out", type=Path, default=Path("track-log.jsonl"))
    parser.add_argument("--seconds", type=float, default=150.0, help="race-clock seconds to log")
    parser.add_argument("--park", action="store_true", help="hold no buttons: the player sits on the grid")
    parser.add_argument("--rail", action="store_true", help="stop at the rail (`Ai_SnapToLane`'s set-position call) instead: one `rail` line per hit, one full row per tick")
    parser.add_argument("--skill", type=int, help="write g_skill_level and exit; restart the race, then capture")
    capture(parser.parse_args())


if __name__ == "__main__":
    main()
