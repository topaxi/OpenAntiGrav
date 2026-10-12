#!/usr/bin/env python3
"""Capture Pure's opponent thrust law, with every input it reads, on a running Pure PSP (USA).

`docs/ghidra/functions/psp-pure-usa/ai-controls.md` states the legacy-block thrust as

    x      = cruise[slot] * tweak * zone_and_fatigue
    thrust = x + (StartSpeed[slot] - x) * launch

with `cruise` the per-slot speed `AiStats_LoadAll` draws at load (`R+0x64 + 4 * slot`),
`tweak` `driver+0x190`, `zone_and_fatigue` `driver+0x54`, `StartSpeed` `R+0x84 + 4 * slot`
and `launch` `driver+0x1c0`, and `driver+0x54 = RM+0x134 * fatigue`, `RM+0x134` the zone
curve at the nearest opponent's gap to the player. This is the falsifier. It stops right
after `Ai_ComputeControls` returns inside `Ai_Update` (`0x08919728`, `s0` the driver), keeps
the stops on the round-robin tick (`racer+0x1b0`), reads the law's output at `sp+0x14`, and
logs every input; `--predict` recomputes each sample in `float32` and reports residuals.

It also reads the block switch `g_ai_use_new_stats` (`0x08ab3b6c`) and, with `--rail N`,
stops `N` times on `Ai_SnapToLane`'s `Body_SetPosition` call (`0x08917f40`) logging the
driver, the body's position before the write and the target in `f12..f14`, and whether
the body's velocity changes across the call (it should not: Pure's rail is position-only).

    uv run --with websocket-client python3 scripts/psp-pure-ai-thrust.py --port 45091 \
        --out data/scratch/<lane>/venom.json --samples 120
    python3 scripts/psp-pure-ai-thrust.py --predict data/scratch/<lane>/venom.json

Captures hold the disc's AI tables (derived game data): keep them under `data/`.
"""

import argparse
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

AFTER_CONTROLS = 0x08919728
RAIL_SET_POSITION = 0x08917F40
USE_NEW_STATS = 0x08AB3B6C
CLASS_INDEX = 0x08B173E0
RACERS = 0x08B71FA0
RACER_STRIDE = 0x1C0
RACER_COUNT = 0x08B72DA0
PLAYER_CRAFT = 0x08B72DA8


def f32(raw):
    return struct.unpack("<f", struct.pack("<I", raw & 0xFFFFFFFF))[0]


def regs(dbg, name):
    registers = dbg.call("cpu.getAllRegs")
    category = next(c for c in registers["categories"] if c["name"] == name)
    return dict(zip(category["registerNames"], category["uintValues"]))


def craft_state(dbg, craft):
    return {
        "craft": craft,
        "progress": dbg.read_f32(craft + 0xA38),
        "place": dbg.read_u32(craft + 0xB44),
        "out": dbg.read_u8(craft + 0xB95),
        "has_point": dbg.read_u32(craft + 0xAC0) != 0,
        "lap": dbg.read_f32(dbg.read_u32(craft + 0xB60)) if dbg.read_u32(craft + 0xB60) else None,
    }


def sample(dbg, driver, sp):
    racer = dbg.read_u32(driver)
    rm = dbg.read_u32(driver + 0x38)
    record = dbg.read_u32(driver + 0x138)
    count = dbg.read_u32(RACER_COUNT)
    crafts = [dbg.read_u32(RACERS + i * RACER_STRIDE) for i in range(count)]
    return {
        "driver": driver,
        "craft": dbg.read_u32(driver + 4),
        "law": dbg.read_f32(sp + 0x14),
        "slot": dbg.read_u32(driver + 0x60),
        "is_player": dbg.read_u8(driver + 0xA0),
        "tweak": dbg.read_f32(driver + 0x190),
        "zone_fatigue": dbg.read_f32(driver + 0x54),
        "launch": dbg.read_f32(driver + 0x1C0),
        "flag80": dbg.read_u8(driver + 0x80),
        "fatigue_t": dbg.read_f32(driver + 0xB0),
        "time": dbg.read_f32(driver + 0x64),
        "rr": dbg.read_u8(racer + 0x1B0),
        "rm134": dbg.read_f32(rm + 0x134),
        "launch_phase": dbg.read_u8(rm + 0x1E34),
        "record": record,
        "record_words": [dbg.read_u32(record + 4 * i) for i in range(0x120 // 4)],
        "racers": [craft_state(dbg, c) for c in crafts],
        "player": dbg.read_u32(PLAYER_CRAFT),
        "use_new_stats": dbg.read_u8(USE_NEW_STATS),
    }


def capture(args):
    from ppsspp_debugger import Debugger

    dbg = Debugger(args.port)
    dbg.brk()
    head = {
        "use_new_stats": dbg.read_u8(USE_NEW_STATS),
        "class": dbg.read_u32(CLASS_INDEX),
        "racer_count": dbg.read_u32(RACER_COUNT),
    }
    print("switch", head, flush=True)
    samples, ticks = [], []
    seen = 0
    # One whole-field sample is `racer_count` consecutive stops (every driver runs each
    # frame); between field samples the breakpoint is lifted so the race runs freely.
    while seen < args.samples:
        for _, _msg in dbg.each_hit(AFTER_CONTROLS, head["racer_count"] * args.burst, timeout=60.0):
            gpr = regs(dbg, "GPR")
            driver, sp = gpr["s0"], gpr["sp"]
            racer = dbg.read_u32(driver)
            rr = dbg.read_u8(racer + 0x1B0)
            ticks.append(((racer - RACERS) // RACER_STRIDE, rr))
            if not rr:
                continue
            samples.append(sample(dbg, driver, sp))
            seen += 1
            s = samples[-1]
            print(f"{seen:4d} t={s['time']:.2f} slot={s['slot']} law={s['law']:.3f} "
                  f"rm134={s['rm134']:.3f} zf={s['zone_fatigue']:.3f} launch={s['launch']:.3f}", flush=True)
        if args.gap_seconds:
            dbg.resume()
            time.sleep(args.gap_seconds)
            dbg.brk()
    rail = []
    if args.rail:
        for _, _msg in dbg.each_hit(RAIL_SET_POSITION, args.rail, timeout=60.0):
            gpr = regs(dbg, "GPR")
            fpu = regs(dbg, "FPU")
            body = gpr["a0"]
            rail.append({
                "body": body,
                "pos": [dbg.read_f32(body + 0x30 + 4 * i) for i in range(3)],
                "target": [f32(fpu["f12"]), f32(fpu["f13"]), f32(fpu["f14"])],
            })
    dbg.resume()
    out = {"head": head, "samples": samples, "ticks": ticks, "rail": rail}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(out))
    print("wrote", args.out, len(samples), "samples", len(rail), "rail stops")


def f(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def word(sample, offset):
    return f32(sample["record_words"][offset // 4])


def predict_one(s):
    slot = s["slot"]
    cruise = word(s, 0x64 + 4 * slot)
    start = word(s, 0x84 + 4 * slot)
    x = f(f(cruise * s["tweak"]) * s["zone_fatigue"])
    return f(x + f(f(start - x) * s["launch"]), ), cruise, start


def zone_one(s):
    """`RaceManager_UpdateZoneSpeed` from the logged racers: the curve at the nearest gap."""
    player = next((r for r in s["racers"] if r["craft"] == s["player"]), None)
    if player is None or not player["has_point"]:
        return None
    best = None
    for r in s["racers"]:
        if r["craft"] == s["player"] or r["out"]:
            continue
        g = f(r["progress"] - player["progress"])
        lap = player["lap"]
        while g > lap * 0.5:
            g = f(g - lap)
        while g < -(lap * 0.5):
            g = f(g + lap)
        if best is None or abs(g) < abs(best):
            best = g
    if best is None:
        return 1.0
    lead = player["place"] == 1
    pos = [word(s, (0xD8 if lead else 0xA8) + 4 * i) for i in range(6)]
    spd = [word(s, (0xF0 if lead else 0xC0) + 4 * i) for i in range(6)]
    if best <= pos[0]:
        return spd[0]
    if pos[5] <= best:
        return spd[5]
    i = max(k for k in range(6) if pos[k] <= best)
    return spd[i] + (best - pos[i]) / (pos[i + 1] - pos[i]) * (spd[i + 1] - spd[i])


def predict(path):
    data = json.loads(Path(path).read_text())
    print("switch", data["head"])
    worst, exact, n = 0.0, 0, 0
    zone_worst, zone_n = 0.0, 0
    samples = data["samples"]
    for k, s in enumerate(samples):
        p, cruise, start = predict_one(s)
        if s["is_player"]:
            # The player's own driver (the autopilot) keeps the output default.
            p = 100.0
        else:
            n += 1
            r = abs(p - s["law"])
            worst = max(worst, r)
            exact += r < 1e-3
        z = zone_one(s)
        # The race manager writes `RM+0x134` once a frame from the positions it sees then,
        # so this sample's racers predict the next frame's multiplier.
        later = next((t for t in samples[k + 1:] if t["time"] > s["time"]), None)
        if z is not None and later is not None and later["time"] - s["time"] < 0.02:
            zone_worst = max(zone_worst, abs(z - later["rm134"]))
            zone_n += 1
        print(f"t={s['time']:7.2f} slot={s['slot']} law={s['law']:8.3f} pred={p:8.3f} "
              f"cruise={cruise:6.2f} start={start:6.1f} launch={s['launch']:.3f} "
              f"rm134={s['rm134']:.4f} zone={z if z is None else round(z, 4)} "
              f"zf={s['zone_fatigue']:.4f} ft={s['fatigue_t']:.3f} new={s['use_new_stats']}")
    print(f"{exact}/{n} opponent samples within 1e-3, worst {worst:.5f}; "
          f"zone multiplier worst {zone_worst:.5f} over {zone_n} next-frame pairs")
    period = {}
    last = {}
    for i, (index, rr) in enumerate(data["ticks"]):
        if rr:
            if index in last:
                period.setdefault(index, set()).add(i - last[index])
            last[index] = i
    print("round-robin stops between a racer's own thrust ticks:", period)
    for hit in data["rail"]:
        d = [t - p for t, p in zip(hit["target"], hit["pos"])]
        print("rail move", round(sum(x * x for x in d) ** 0.5, 4))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=45091)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--samples", type=int, default=120)
    parser.add_argument("--gap-seconds", type=float, default=0.0,
                        help="run this long between whole-field samples (every driver runs each frame)")
    parser.add_argument("--burst", type=int, default=1,
                        help="consecutive frames per field sample (2 or more checks the zone multiplier)")
    parser.add_argument("--rail", type=int, default=0)
    parser.add_argument("--predict", type=Path)
    args = parser.parse_args()
    if args.predict:
        predict(args.predict)
    else:
        capture(args)


if __name__ == "__main__":
    main()
