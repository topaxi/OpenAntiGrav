#!/usr/bin/env python3
"""Log Wipeout HD's opponent-thrust law live on RPCS3, and recompute it offline.

The PS3 counterpart of `scripts/psp-ai-thrust.py`. It stops right after
`AI_ComputeOpponentThrust` (`0x000fdf00`) returns to `AI_ComputeControls`
(`0x00100e3c`), reads the result in `f1`, and snapshots every input the law
reads: the driver's wander block, every racer's progress, place, finished flag
and driver `spread`, each craft's `+0x628c`, the race time and the class
record. `--predict` recomputes each sample in `float32` from the log alone.
Evidence page: `docs/ghidra/functions/ps3-hdfury-eu/ai-controls.md`.

Needs an RPCS3 served with `--interpreter` (`Z0` breakpoints fire under no
other decoder) and parked in a race:

    source data/scratch/<lane>/env.sh
    OAG_RPCS3_ATTACH=1 uv run --with numpy python3 scripts/rpcs3-hd-ai-thrust.py \\
        --out data/scratch/<lane>/run.json --samples 120
    uv run --with numpy python3 scripts/rpcs3-hd-ai-thrust.py --predict data/scratch/<lane>/run.json

`--skill` first stops once after `AI_ResolveSkillScale` returns inside the law
(`0x000fe03c`) and records `f1`; the samples need it.

`--rail N` instead counts `Ai_SnapToLane`'s `Body_SetPosition` call
(`0x000ffda4`) over N hits, logging the driver, its rail blend and the move.
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

AFTER_THRUST = 0x00100E3C
AFTER_SKILL = 0x000FE03C
RAIL_SET_POSITION = 0x000FFDA4
RACER_TABLE = 0x009879F0
RACER_STRIDE = 0x9C0
GAME_STATE = 0x00936FE8
MODE_FLAG = 0x009384E1


def f32(raw, at):
    return struct.unpack_from(">f", raw, at)[0]


def u32(raw, at):
    return struct.unpack_from(">I", raw, at)[0]


def fpr(regs, index):
    from rpcs3_debugger import REG_FPR

    return struct.unpack_from(">d", regs, REG_FPR + index * 8)[0]


def gpr(regs, index):
    from rpcs3_debugger import REG_GPR

    return int.from_bytes(regs[REG_GPR + index * 8 : REG_GPR + index * 8 + 8], "big") & 0xFFFFFFFF


def connect():
    """A debugger on the lane's port (`OAG_RPCS3_GDB=host:port`), else the stub's default."""
    import os

    from rpcs3_debugger import Debugger

    server = os.environ.get("OAG_RPCS3_GDB")
    if server:
        host, port = server.rsplit(":", 1)
        return Debugger(port=int(port), host=host)
    return Debugger()


def next_hit(dbg, address, timeout):
    """`(tid, regs)` of a thread parked at `address`.

    Resume, then read the stop reply right away: a pause sent while a
    breakpoint's reply is queued desyncs the stub
    (`docs/reverse-engineering/rpcs3-debugger.md`, the trap set).
    """
    from rpcs3_debugger import REG_PC

    deadline = time.time() + timeout
    while time.time() < deadline:
        dbg.resume()
        if dbg.wait_for_stop(min(10.0, max(deadline - time.time(), 1.0))) is None:
            dbg.pause()
        dbg.drain(0.1)
        for tid in dbg.threads():
            regs = dbg.registers(tid)
            if regs is not None and int.from_bytes(regs[REG_PC:REG_PC + 8], "big") == address:
                return tid, regs
    return None, None


def snapshot(dbg, driver):
    block = dbg.read(driver, 0x180)
    table = dbg.read(RACER_TABLE, 0x10)
    count = u32(dbg.read(RACER_TABLE + 0x4E10, 4), 0)
    player_racer = u32(table, 0)
    racers = []
    for i in range(min(count, 8)):
        base = RACER_TABLE + 0x10 + i * RACER_STRIDE
        head = dbg.read(base, 0x90)
        tail = dbg.read(base + 0x898, 0x100)
        craft, drv = u32(head, 0), u32(head, 4)
        kind = u32(dbg.read(craft + 0x628C, 4), 0) if craft else 0
        spread = f32(dbg.read(drv + 0xA4, 4), 0) if drv else 0.0
        racers.append({
            "addr": base, "craft": craft, "driver": drv, "kind": kind,
            "finished": head[0x82], "progress": f32(tail, 0),
            "place": u32(tail, 0x994 - 0x898), "spread": spread,
        })
    rm = u32(block, 0x68)
    state = dbg.read(GAME_STATE + 0xD0, 0x18)
    return {
        "driver": driver,
        "own_racer": u32(block, 0),
        "player_racer": player_racer,
        "race_time": f32(dbg.read(rm + 0x13DC, 4), 0),
        "wander": [f32(block, o) for o in range(0xA4, 0xCC, 4)],
        "gap": f32(block, 0xCC),
        "accumulator": f32(block, 0xD0),
        "record": u32(block, 0x168),
        "class": u32(state, 0x04),
        "level": u32(state, 0x0C),
        "mode": u32(state, 0x10),
        "humans": state[0x14],
        "flag": dbg.read(MODE_FLAG, 1)[0],
        "racers": racers,
    }


def capture(args):
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    log = {"samples": [], "skill": None, "record": None}
    dbg = connect()
    try:
        dbg.pause()
        if args.skill:
            dbg.add_breakpoint(AFTER_SKILL)
            tid, regs = next_hit(dbg, AFTER_SKILL, 60.0)
            dbg.remove_breakpoint(AFTER_SKILL)
            if tid is None:
                print("no stop after AI_ResolveSkillScale", file=sys.stderr)
                return 1
            log["skill"] = fpr(regs, 1)
            print("skill scale %.6f" % log["skill"], flush=True)
            dbg.step(tid)
            dbg.pause()
        dbg.add_breakpoint(AFTER_THRUST)
        while len(log["samples"]) < args.samples:
            tid, regs = next_hit(dbg, AFTER_THRUST, 60.0)
            if tid is None:
                print("no stop after the law in 60 s", file=sys.stderr)
                break
            thrust = fpr(regs, 1)
            sample = snapshot(dbg, gpr(regs, 25))
            sample["thrust"] = thrust
            if log["record"] is None and sample["record"]:
                log["record"] = dbg.read(sample["record"], 0x118).hex()
            log["samples"].append(sample)
            out.write_text(json.dumps(log, indent=1))
            print("t=%.2f thrust %.3f gap %.2f" % (sample["race_time"], thrust, sample["gap"]),
                  flush=True)
            dbg.remove_breakpoint(AFTER_THRUST)
            dbg.step(tid)
            dbg.pause()
            if args.gap_seconds:
                dbg.run_for(args.gap_seconds)
            dbg.add_breakpoint(AFTER_THRUST)
        dbg.remove_breakpoint(AFTER_THRUST)
    finally:
        out.write_text(json.dumps(log, indent=1))
        try:
            dbg.resume()
        except Exception:
            pass
        dbg.close()
    print("wrote %d samples to %s" % (len(log["samples"]), out))
    return 0


def rail(args):
    out = Path(args.out)
    hits = []
    dbg = connect()
    try:
        dbg.pause()
        dbg.add_breakpoint(RAIL_SET_POSITION)
        while len(hits) < args.rail:
            tid, regs = next_hit(dbg, RAIL_SET_POSITION, 60.0)
            if tid is None:
                break
            driver = gpr(regs, 31)
            racer = u32(dbg.read(driver, 4), 0)
            pos = struct.unpack(">3f", dbg.read(racer + 0x10, 12))
            new = (fpr(regs, 1), fpr(regs, 2), fpr(regs, 3))
            blend = f32(dbg.read(driver + 0x78, 8), 0)
            on = f32(dbg.read(driver + 0x78, 8), 4)
            move = sum((a - b) ** 2 for a, b in zip(pos, new)) ** 0.5
            hits.append({"driver": driver, "blend": blend, "time_on": on, "move": move})
            print("driver %#x blend %.3f move %.4f" % (driver, blend, move), flush=True)
            dbg.remove_breakpoint(RAIL_SET_POSITION)
            dbg.step(tid)
            dbg.pause()
            dbg.add_breakpoint(RAIL_SET_POSITION)
        dbg.remove_breakpoint(RAIL_SET_POSITION)
    finally:
        out.write_text(json.dumps(hits, indent=1))
        try:
            dbg.resume()
        except Exception:
            pass
        dbg.close()
    return 0


def predict_one(sample, record, skill):
    """HD's `AI_ComputeOpponentThrust` in float32, in its own operation order."""
    import numpy as np

    F = np.float32
    rec = bytes.fromhex(record)
    rf = lambda o: F(f32(rec, o))
    a = [rf(0x68 + 4 * i) for i in range(8)]
    s_d = [rf(0x88 + 4 * i) for i in range(8)]
    points = [[rf(0xD0 + 0x18 * n + 4 * k) for k in range(3)] for n in range(3)]
    s = F(skill)
    if s < F(2.0):
        p, q, t = points[0], points[1], s - F(1.0)
    else:
        p, q, t = points[1], points[2], s - F(2.0)
    u = F(1.0) - t
    off = F(F(p[0] * u) + F(t * q[0]))
    mul = F(F(p[1] * u) + F(t * q[1]))
    smul = F(F(p[2] * u) + F(t * q[2]))
    racers = sample["racers"]
    own = next(r for r in racers if r["addr"] == sample["own_racer"])
    player = next(r for r in racers if r["addr"] == sample["player_racer"])
    finished = bool(own["finished"])
    place = player["place"]
    base = a[place - 1]
    spread_dist = F(75.0) if finished else F(smul * s_d[place - 1])
    ref_place = place - 1 if place - 1 > 0 else 2
    lead = trail = F(100000.0)
    ref = None
    for r in racers:
        if r["place"] == ref_place:
            ref = r
        if finished or r["kind"] != 0:
            d = F(F(player["progress"]) - F(r["progress"]))
            lead = min(lead, d)
            trail = min(trail, F(-d))
    lead = max(F(lead - rf(0xAC)), F(0.0))
    trail = max(F(trail - rf(0xB8)), F(0.0))
    ref_progress = F(ref["progress"]) if ref else F(0.0)
    term = F(0.0)
    if ref is not None and ref["driver"]:
        term = F(spread_dist * F(F(own["spread"]) - F(ref["spread"])))
    gap = F(F(ref_progress + term) - F(own["progress"]))
    time_s = F(sample["race_time"])
    gain = max(F(F(time_s * F(-0.08)) + F(2.0)), F(0.5))
    limit = F(base * F(0.3))
    step = min(max(F(gap * gain), -limit), limit)
    x = F(base + step)
    x = F(x + min(F(lead * rf(0xA8)), rf(0xB0)))
    eliminator = sample["mode"] in (8, 0x14) and not sample["flag"]
    if not eliminator:
        x = F(x - min(F(trail * rf(0xB4)), rf(0xBC)))
    cap = F(100.0) if time_s > F(20.0) and abs(F(own["progress"]) - F(player["progress"])) < F(350.0) else F(1000.0)
    thrust = F(F(mul * F(x - a[0])) + F(off + a[0]))
    return float(max(min(thrust, cap), F(1.0))), float(gap)


def predict(path):
    log = json.loads(Path(path).read_text())
    skill = log["skill"]
    if skill is None:
        print("no skill scale in the log; capture with --skill", file=sys.stderr)
        return 1
    exact = 0
    worst = 0.0
    for sample in log["samples"]:
        thrust, gap = predict_one(sample, log["record"], skill)
        r = abs(thrust - sample["thrust"])
        worst = max(worst, r)
        exact += r < 1e-3
        flag = "" if r < 1e-3 else "  <-- off by %.4f" % r
        print("t=%7.2f logged %8.3f predicted %8.3f gap %9.3f/%9.3f%s"
              % (sample["race_time"], sample["thrust"], thrust, sample["gap"], gap, flag))
    print("%d of %d samples exact to 1e-3, worst residual %.5f"
          % (exact, len(log["samples"]), worst))
    return 0 if exact == len(log["samples"]) else 2


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", default="hd-ai-thrust.json")
    parser.add_argument("--samples", type=int, default=80)
    parser.add_argument("--gap-seconds", type=float, default=0.0)
    parser.add_argument("--skill", action="store_true")
    parser.add_argument("--rail", type=int, default=0)
    parser.add_argument("--predict", metavar="LOG")
    args = parser.parse_args()
    if args.predict:
        return predict(args.predict)
    if args.rail:
        return rail(args)
    return capture(args)


if __name__ == "__main__":
    sys.exit(main())
