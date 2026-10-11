#!/usr/bin/env python3
"""Capture the original's opponent thrust law, with every input it reads, on a running Pulse PSP.

`docs/ghidra/functions/psp-pulse-usa/ai-controls.md` states `AI_ComputeOpponentThrust`
(`0x08855904`) as a closed formula. This is its falsifier: it stops right after the call
returns inside `Ai_ComputeControls` (`0x088568b0`, the opponent branch), reads the result in
`f0`, and logs the race state the formula takes as input - the race time, the skill scale,
every racer's progress and place, every driver's spread, the player's record and the class
record. `--predict` then recomputes each sample from the log alone and reports the residual.

Phases, all on one attached emulator that is already in a race:

1. `skill`: one stop at `0x088559b8`, right after `AI_ResolveSkillScale` returns, for the
   skill scale in `f0`.
2. `thrust`: `--samples` stops at `0x088568b0`, one per tick (the round-robin `racer+0x360`
   runs one driver's law per tick).
3. `rail`: `--rail-hits` stops at `0x088548dc`, `Ai_SnapToLane`'s `Body_SetPosition` call,
   logging which driver was snapped and the race time, so the snapped fraction of ticks per
   driver falls out.

`--park` holds no buttons (the player sits on the grid, last, so `WhenBehind` saturates);
without it the player is given the autopilot pickup with a huge timer, so the original's own
driver flies it. `--skill N` writes `g_skill_level` before `psp-drive.py restart` is run by
hand; this script does not restart.

    uv run --with websocket-client python3 scripts/psp-ai-thrust.py --port 47994 \
        --out data/scratch/<lane>/cap/easy-park.json --park --samples 160 --rail-hits 400
    python3 scripts/psp-ai-thrust.py --predict data/scratch/<lane>/cap/easy-park.json

Raw captures are derived game data (they hold the disc's AI tables): write them under
`data/`, never commit them.
"""

import argparse
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

RESOLVE_RETURN = 0x088559B8
THRUST_RETURN = 0x088568B0
SNAP_SET_POSITION = 0x088548DC
RACE_MANAGER = 0x08B317B4
RACERS = 0x08B34420
RACER_STRIDE = 0x370
RACER_COUNT = 0x08B35FA0
PLAYER_RACER = 0x08B34410
PLAYER_CRAFT = 0x08B34418
G_SKILL_LEVEL = 0x08B31044
G_RACE_LAPS = 0x08B30F9C
WEAPONS_DISPATCH_FIRE = 0x08861814
AUTOPILOT_BIT = 0x1000
FIRE_WORD = 0x1B8
HELD_WORD = 0x1BC
AUTO_TIMER = 0x148


def f32(raw):
    return struct.unpack("<f", struct.pack("<I", raw & 0xFFFFFFFF))[0]


def regs(dbg, name):
    registers = dbg.call("cpu.getAllRegs")
    category = next(c for c in registers["categories"] if c["name"] == name)
    return dict(zip(category["registerNames"], category["uintValues"]))


def hits(dbg, address, count, timeout=60.0):
    dbg.brk()
    dbg.add_breakpoint(address)
    try:
        for index in range(count):
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            try:
                yield index, dbg.wait_for_break_any((address,), timeout=timeout)
            except TimeoutError:
                status = dbg.call("cpu.status")
                if status["stepping"] and status["pc"] == address:
                    yield index, status
                else:
                    raise
    finally:
        dbg.brk()
        dbg.remove_breakpoint(address)


def floats(dbg, address, count):
    return list(struct.unpack("<%df" % count, dbg.read(address, 4 * count)))


def snapshot(dbg):
    manager = dbg.read_u32(RACE_MANAGER)
    count = dbg.read_u32(RACER_COUNT)
    racers = []
    for i in range(count):
        base = RACERS + i * RACER_STRIDE
        craft = dbg.read_u32(base)
        driver = dbg.read_u32(base + 4)
        row = {
            "index": i,
            "base": base,
            "craft": craft,
            "driver": driver,
            "finished": dbg.read_u8(base + 0x82),
            "progress": dbg.read_f32(base + 0x240),
            "place": dbg.read_u32(base + 0x344),
            "tick": dbg.read_u8(base + 0x360),
            "speed": sum(v * v for v in floats(dbg, base + 0x50, 3)) ** 0.5,
        }
        if 0x08800000 <= driver < 0x0A000000:
            d = floats(dbg, driver, 0x2C)
            row.update(
                {
                    "thrust_out": d[3],
                    "steer_out": d[2],
                    "consumed": dbg.read_u8(driver + 0x21),
                    "authority": d[0x19],
                    "spread": d[0x1D],
                    "spread_start": d[0x1E],
                    "spread_end": d[0x1F],
                    "phase": d[0x22],
                    "segments": d[0x23],
                    "amplitude": d[0x24],
                    "wander_prev": d[0x25],
                    "wander_next": d[0x26],
                    "gap": d[0x27],
                    "accum": d[0x28],
                    "snap_blend": dbg.read_f32(driver + 0x48),
                    "second_controller": dbg.read_u8(driver + 0x880),
                    "record": dbg.read_u32(driver + 0x138),
                    "is_player_driver": dbg.read_u8(driver + 0xA8),
                }
            )
        if 0x08800000 <= craft < 0x0A000000:
            body = dbg.read_u32(craft + 0x94)  # racer+0x00 is the entity; its craft is entity+0x94
            row["craft_input"] = dbg.read_u32(body + 0x3C)
            row["craft_autopilot_input"] = dbg.read_u32(body + 0x40)
            row["autopilot_blend"] = dbg.read_f32(body + 0x1D4)
        if 0x08800000 <= driver < 0x0A000000:
            row["lap_length"] = dbg.read_f32(dbg.read_u32(driver + 0x34))  # *("AI track data")
        racers.append(row)
    return {
        "manager": manager,
        "time": dbg.read_f32(manager + 0x2B8),
        "start_offset": dbg.read_f32(manager + 0x7AC),
        "skill_level": dbg.read_u32(G_SKILL_LEVEL),
        "laps": dbg.read_u32(G_RACE_LAPS),
        "player_racer": dbg.read_u32(PLAYER_RACER),
        "player_craft": dbg.read_u32(PLAYER_CRAFT),
        "racers": racers,
    }


def teleport(dbg, distance, log):
    """Move the player's body `distance` units along the leader's velocity from the leader.

    Only the law's inputs change (progress is re-located from the position each frame), which
    is the point: the player leading or in mid-pack is otherwise minutes of 2-3 % emulation.
    """
    dbg.brk()
    snap = snapshot(dbg)
    leader = min(snap["racers"], key=lambda x: x["place"] if x["base"] != snap["player_racer"] else 99)
    lb = dbg.read_u32(leader["craft"] + 0x794)
    pos = floats(dbg, lb + 0x30, 3)
    vel = floats(dbg, lb + 0x140, 3)
    norm = sum(v * v for v in vel) ** 0.5 or 1.0
    target = [p + v / norm * distance for p, v in zip(pos, vel)]
    pb = dbg.read_u32(snap["player_racer"] and dbg.read_u32(snap["player_racer"]) + 0x794)
    dbg.write(pb + 0x30, struct.pack("<3f", *target))
    dbg.write(pb + 0x140, struct.pack("<3f", *vel))
    log["events"].append({"teleport": distance, "from": pos, "to": target})
    print("player moved %.0f units off the leader" % distance, file=sys.stderr)
    dbg.resume()
    time.sleep(1.0)


def capture(args):
    from ppsspp_debugger import Debugger

    dbg = Debugger(args.port)
    log = {"skill_scale": None, "samples": [], "rail": [], "records": {}, "events": []}
    if args.skill is not None:
        dbg.write_u32(G_SKILL_LEVEL, args.skill)
        log["events"].append({"skill_level_written": args.skill})
    dbg.resume()
    if args.park:
        dbg.hold(cross=False)
    else:
        dbg.hold(cross=True)

    for _, _ in hits(dbg, RESOLVE_RETURN, 1):
        log["skill_scale"] = f32(regs(dbg, "FPU")["f0"])
    print("skill scale %.4f" % log["skill_scale"], file=sys.stderr)

    if args.autopilot:
        manager = dbg.read_u32(RACE_MANAGER)
        player = dbg.read_u32(manager + 0x2C0)
        record = dbg.read_u32(player + 0x4C)
        dbg.write_u32(record + HELD_WORD, 0xFFFFFFFF)
        dbg.write_u32(record + FIRE_WORD, dbg.read_u32(record + FIRE_WORD) | AUTOPILOT_BIT)
        for _, _ in hits(dbg, WEAPONS_DISPATCH_FIRE, 3):
            pass
        dbg.write(record + AUTO_TIMER, struct.pack("<f", 1.0e9))
        log["events"].append({"autopilot_armed": True})
        print("autopilot armed", file=sys.stderr)
        dbg.resume()
        time.sleep(args.settle)

    if args.teleport is not None:
        teleport(dbg, args.teleport, log)

    for index, _ in hits(dbg, THRUST_RETURN, args.samples):
        r = regs(dbg, "GPR")
        thrust = f32(regs(dbg, "FPU")["f0"])
        snap = snapshot(dbg)
        driver = r["s1"]
        record = next(x["record"] for x in snap["racers"] if x.get("driver") == driver)
        if str(record) not in log["records"]:
            log["records"][str(record)] = floats(dbg, record, 0x10C // 4)
        log["samples"].append({"driver": driver, "thrust": thrust, "state": snap})
        if index % 20 == 0:
            print("sample %d t=%.2f driver %#x thrust %.3f" % (index, snap["time"], driver, thrust), file=sys.stderr)
        if args.gap_seconds:
            dbg.resume()
            time.sleep(args.gap_seconds)

    for index, _ in hits(dbg, SNAP_SET_POSITION, args.rail_hits):
        r = regs(dbg, "GPR")
        manager = dbg.read_u32(RACE_MANAGER)
        driver = r["s0"]
        # The stop is on the `jal` itself, whose delay slot loads `f14`: the target's z is not in a
        # register yet. Read the whole target from the stack buffer `a1` points at instead.
        target = floats(dbg, r["a1"], 3)
        current = floats(dbg, r["a0"] + 0x30, 3)
        log["rail"].append(
            {
                "driver": driver,
                "time": dbg.read_f32(manager + 0x2B8),
                "blend": dbg.read_f32(driver + 0x48),
                "on_rail_for": dbg.read_f32(driver + 0x4C),
                "move": sum((a - b) ** 2 for a, b in zip(target, current)) ** 0.5,
            }
        )
    log["final"] = snapshot(dbg)
    dbg.resume()
    dbg.close()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(log, indent=1))
    print("wrote %s: %d samples, %d rail hits" % (args.out, len(log["samples"]), len(log["rail"])))


def lerp(a, b, t):
    return a * (1.0 - t) + t * b


def predict_one(sample, skill, record):
    import numpy as np

    F = np.float32
    st = sample["state"]
    racers = st["racers"]
    me = next(x for x in racers if x.get("driver") == sample["driver"])
    player = next(x for x in racers if x["base"] == st["player_racer"])
    R = [F(v) for v in record]
    A = R[0x68 // 4 : 0x68 // 4 + 8]
    S = R[0x88 // 4 : 0x88 // 4 + 8]
    lead_mul, lead_dead, lead_max = R[0xA8 // 4], R[0xAC // 4], R[0xB0 // 4]
    trail_mul, trail_dead, trail_max = R[0xB4 // 4], R[0xB8 // 4], R[0xBC // 4]
    s = F(skill)
    if s < 2:
        p, q, base = 0xC4, 0xDC, F(1)
    else:
        p, q, base = 0xDC, 0xF4, F(2)
    t = s - base
    off = lerp(R[p // 4], R[q // 4], t)
    mul = lerp(R[p // 4 + 1], R[q // 4 + 1], t)
    smul = lerp(R[p // 4 + 2], R[q // 4 + 2], t)
    r = player["place"]
    finished = me["finished"]
    base_thrust = A[r - 1]
    spread_dist = F(50) if finished else S[r - 1] * smul
    ref_place = r - 1 if r - 1 >= 1 else 2
    lead = trail = F(100000)
    ref = None
    for k in racers:
        if k["place"] == ref_place:
            ref = k
        if finished or k["craft"] != st["player_craft"]:
            d = F(player["progress"]) - F(k["progress"])
            lead = min(lead, d)
            trail = min(trail, -d)
    lead = max(lead - lead_dead, F(0))
    trail = max(trail - trail_dead, F(0))
    if ref is None:
        ref = me
    ref_spread = F(ref["spread"]) if "spread" in ref else F(me["spread"])
    gap = F(ref["progress"]) + (F(me["spread"]) - ref_spread) * spread_dist - F(me["progress"])
    T = F(st["time"])
    k_gain = max(F(2) - T * F(0.08), F(0.5))
    x = base_thrust + min(max(gap * k_gain, -base_thrust * F(0.3)), base_thrust * F(0.3))
    band_lead = min(lead * lead_mul, lead_max)
    band_trail = min(trail * trail_mul, trail_max)
    x = x + band_lead - band_trail
    thrust = off + (x - A[0]) * mul + A[0]
    cap = F(100) if (T > 20 and abs(F(me["progress"]) - F(player["progress"])) < 350) else F(1000)
    thrust = max(min(thrust, cap), F(1))
    return {
        "predicted": float(thrust),
        "gap": float(gap),
        "logged_gap": me.get("gap"),
        "place": r,
        "lead": float(lead),
        "trail": float(trail),
        "band": float(band_lead - band_trail),
        "capped": float(cap),
        "time": float(T),
    }


def wander_one(sample):
    """`Ai_UpdateSpreadWander`'s output for the sampled driver, from its logged state."""
    import numpy as np

    F = np.float32
    st = sample["state"]
    me = next(x for x in st["racers"] if x.get("driver") == sample["driver"])
    laps = st["laps"]
    f = F(0)
    if 0 < laps < 20:
        f = (F(me["progress"]) - F(st["start_offset"])) / (F(laps) * F(me["lap_length"]))
    f = min(max(f, F(0)), F(1))
    phase = F(me["phase"])
    low = phase - F(1) / F(me["segments"])
    w = (f - low) / (phase - low)
    spread = (F(me["wander_prev"]) * (F(1) - w) + F(me["wander_next"]) * w
              + F(me["spread_start"]) * (F(1) - f) + F(me["spread_end"]) * f)
    return abs(float(spread) - me["spread"])


def predict(path):
    log = json.loads(Path(path).read_text())
    skill = log["skill_scale"]
    worst = 0.0
    rows = []
    for sample in log["samples"]:
        me = next(x for x in sample["state"]["racers"] if x.get("driver") == sample["driver"])
        record = log["records"][str(me["record"])]
        out = predict_one(sample, skill, record)
        out["read"] = sample["thrust"]
        out["residual"] = out["read"] - out["predicted"]
        rows.append(out)
        worst = max(worst, abs(out["residual"]))
    exact = sum(1 for r in rows if abs(r["residual"]) < 1e-3)
    print("%s: skill scale %.4f, %d samples, %d within 1e-3, worst residual %.5f" % (
        path, skill, len(rows), exact, worst))
    bands = sorted({round(r["band"], 3) for r in rows})
    print("  rubber-band terms seen: %s" % bands[:12])
    print("  player places seen: %s" % sorted({r["place"] for r in rows}))
    print("  gap/logged gap max diff: %.5f" % max(abs(r["gap"] - r["logged_gap"]) for r in rows))
    for r in rows[:: max(1, len(rows) // 8)]:
        print("  t=%6.2f place %d gap %9.2f band %7.3f cap %6.0f predicted %8.3f read %8.3f" % (
            r["time"], r["place"], r["gap"], r["band"], r["capped"], r["predicted"], r["read"]))
    # Only the sampled driver: its spread was rewritten at the law's entry this tick, the others'
    # can be up to seven ticks stale against the snapshot's progress.
    if all("lap_length" in x for s in log["samples"] for x in s["state"]["racers"] if x.get("driver") == s["driver"]):
        w = [wander_one(s) for s in log["samples"]]
        print("  wander: %d of %d samples within 1e-4, worst %.2g" % (sum(d < 1e-4 for d in w), len(w), max(w)))
    rail = log.get("rail", [])
    if rail:
        span = rail[-1]["time"] - rail[0]["time"]
        per = {}
        for h in rail:
            per[h["driver"]] = per.get(h["driver"], 0) + 1
        print("  rail: %d Body_SetPosition hits over %.2f s race time, per driver %s" % (
            len(rail), span, sorted(per.values())))
        print("  rail blends: min %.3f max %.3f" % (min(h["blend"] for h in rail), max(h["blend"] for h in rail)))
        moves = sorted(h["move"] for h in rail if "move" in h)
        if moves:
            print("  rail move per call: min %.4f median %.4f max %.4f" % (moves[0], moves[len(moves) // 2], moves[-1]))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=47994)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--samples", type=int, default=160)
    parser.add_argument("--rail-hits", type=int, default=400)
    parser.add_argument("--park", action="store_true")
    parser.add_argument("--autopilot", action="store_true")
    parser.add_argument("--settle", type=float, default=0.0)
    parser.add_argument("--gap-seconds", type=float, default=0.0,
                        help="let the race run this long between thrust samples")
    parser.add_argument("--skill", type=int)
    parser.add_argument("--teleport", type=float, metavar="UNITS",
                        help="place the player this far along the leader's heading first (negative: behind it)")
    parser.add_argument("--predict", type=Path)
    args = parser.parse_args()
    if args.predict:
        predict(args.predict)
    else:
        if args.out is None:
            parser.error("--out is required for a capture")
        capture(args)


if __name__ == "__main__":
    main()
