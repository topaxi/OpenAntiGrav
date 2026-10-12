#!/usr/bin/env python3
"""Capture Pure's lane planner (`Ai_PlanLane`, `0x08919b6c`) either side of one call, on a running Pure PSP (USA).

`docs/ghidra/functions/psp-pure-usa/ai-controls.md` ("The planner: no ring") reads the
planner as one current lane `driver+0x1c4` moved toward one target `driver+0x1c8` by a
second-order bang-bang: an acceleration of `+/-0.0008` and a position term of `+/-0.0004`
a frame, braking when the lane velocity `driver+0x1cc` would overshoot, times
`k = dt * 60 * s`, `s` (`driver+0x1d8`) the `0.9 / 0.1` smoothing of
`clamp((|v_xz . fwd| - 10) * 0.02, 0, 1)`. The target is the best of nine smoothed
candidate scores (`driver+0x204`), searched from the craft's own lane.

This stops before the `jal` at `0x089196f8` (`s0` the driver) and after it (`0x08919700`),
reading the planner's state both sides; `--predict` recomputes the lane, its velocity,
`s` and the target in `float32` from the "before" state and the "after" scores.
PPSSPP 1.20.4 fires only the most recently added breakpoint, so the two are armed in
turn, never together.

    uv run --with websocket-client python3 scripts/psp-pure-ai-lane.py --port 45096 \
        --out data/scratch/<lane>/lane.json --samples 200
    python3 scripts/psp-pure-ai-lane.py --predict data/scratch/<lane>/lane.json
"""

import argparse
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

BEFORE = 0x089196F8
AFTER = 0x08919700
DT60 = 0x08AB3B64
OBJECT_COUNT = 0x08B72DAC


def regs(dbg, name):
    registers = dbg.call("cpu.getAllRegs")
    category = next(c for c in registers["categories"] if c["name"] == name)
    return dict(zip(category["registerNames"], category["uintValues"]))


def state(dbg, driver):
    racer = dbg.read_u32(driver)
    return {
        "lane": dbg.read_f32(driver + 0x1C4),
        "target": dbg.read_f32(driver + 0x1C8),
        "v": dbg.read_f32(driver + 0x1CC),
        "s": dbg.read_f32(driver + 0x1D8),
        "preferred": dbg.read_f32(driver + 0x1DC),
        "out": dbg.read_f32(driver + 0x44),
        "fresh": dbg.read_f32s(driver + 0x1E0, 9),
        "scores": dbg.read_f32s(driver + 0x204, 9),
        "located": dbg.read_u8(driver + 0x3C),
        "flag80": dbg.read_u8(driver + 0x80),
        "is_player": dbg.read_u8(driver + 0xA0),
        "turn": dbg.read_u8(racer + 0x1B0),
        "vel": dbg.read_f32s(racer + 0x50, 3),
        "fwd": dbg.read_f32s(racer + 0x30, 3),
        "dt60": dbg.read_f32(DT60),
    }


def capture(args):
    from ppsspp_debugger import Debugger

    dbg = Debugger(args.port)
    dbg.brk()
    head = {"objects": dbg.read_u32(OBJECT_COUNT)}
    print("AI track-object count", head["objects"], flush=True)
    samples = []
    # Each round visits a driver not yet seen this round, so every driver is sampled
    # in turn rather than whichever the update reaches first after a resume.
    seen = []
    while len(samples) < args.samples:
        condition = " && ".join(f"s0 != {d:#x}" for d in seen) or None
        dbg.add_breakpoint(BEFORE, condition)
        dbg.call("cpu.status")
        dbg.pending = []
        dbg.call("cpu.resume")
        dbg.wait_for_break(BEFORE, timeout=30.0)
        dbg.remove_breakpoint(BEFORE)
        driver = regs(dbg, "GPR")["s0"]
        before = state(dbg, driver)
        dbg.add_breakpoint(AFTER)
        dbg.call("cpu.status")
        dbg.pending = []
        dbg.call("cpu.resume")
        dbg.wait_for_break(AFTER, timeout=30.0)
        dbg.remove_breakpoint(AFTER)
        after = state(dbg, regs(dbg, "GPR")["s0"])
        samples.append({"driver": driver, "before": before, "after": after})
        seen = [] if len(seen) >= args.drivers - 1 else seen + [driver]
        print(f"{len(samples):4d} driver={driver:#x} lane {before['lane']:+.4f} -> {after['lane']:+.4f} "
              f"target {after['target']:+.3f} v {after['v']:+.5f}", flush=True)
        if args.gap_seconds:
            dbg.resume()
            time.sleep(args.gap_seconds)
            dbg.brk()
    dbg.resume()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({"head": head, "samples": samples}))
    print("wrote", args.out, len(samples), "samples")


def f(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def predict_one(b, a):
    """The bang-bang from the "before" state and the target `Ai_PlanLane` chose."""
    target = a["target"]
    e = f(target - b["lane"])
    v = b["v"]
    if 0.0 < e:
        acc, half = 0.0008, 0.0004
        if not v < 0.0:
            k = f(f(v / 0.0008) + 1.0)
            if e < f(f(v * k) + f(f(k * 0.0004) * k)):
                acc, half = -0.0008, -0.0004
    else:
        acc, half = -0.0008, -0.0004
        if not 0.0 < v:
            k = f(f(v / -0.0008) + 1.0)
            if f(f(v * k) + f(f(k * -0.0004) * k)) < e:
                acc, half = 0.0008, 0.0004
    vx, vz = b["vel"][0], b["vel"][2]
    fx, fz = b["fwd"][0], b["fwd"][2]
    raw = f(f(abs(f(f(vx * fx) + f(vz * fz))) - 10.0) * 0.02)
    raw = min(max(raw, 0.0), 1.0)
    s = f(f(b["s"] * 0.9) + f(raw * 0.1))
    k = f(b["dt60"] * s)
    v2 = f(v + f(acc * k))
    lane = f(f(f(v * k) + f(f(half * k) * k)) + b["lane"])
    return lane, v2, s


def bump(scores, centre, half, weight):
    """`Ai_AddLaneBump` (`0x08918ed8`): nothing at all unless `weight > 0.001`."""
    if not 0.001 < weight:
        return
    half = max(half, 0.05)
    inv = f(1.0 / half)
    lo, hi = f(centre - half), f(half + centre)
    for i in range(9):
        x = f(i * 0.25 - 1.0)
        if f(lo - half) < x < f(half + hi):
            if x < lo:
                add = f(f(f(f(x - f(lo - half)) * 0.95) * inv) * weight)
            elif x < centre:
                add = f(weight * f(f(f(f(x - lo) * 0.05) * inv) + 0.95))
            elif x < hi:
                add = f(weight * f(0.95 - f(f(f(x - hi) * 0.05) * inv)))
            else:
                add = f(f(f(f(x - f(half + hi)) * -0.95) * inv) * weight)
            scores[i] = f(scores[i] + add)


def best_of(scores, own_index):
    """Pulse's local search, as Pure's `Ai_PlanLane` ends: up through 7, then down to 0."""
    best, best_score = own_index, -1.0
    for i in range(own_index, 8):
        if scores[i] <= -1.0:
            break
        if scores[i] > best_score:
            best, best_score = i, scores[i]
    for i in range(own_index, -1, -1):
        if scores[i] <= -1.0:
            break
        if scores[i] > best_score:
            best, best_score = i, scores[i]
    return best


def predict(path):
    data = json.loads(Path(path).read_text())
    print("head", data["head"])
    n = exact = 0
    worst = 0.0
    candidates = 0
    fresh_exact = 0
    for s in data["samples"]:
        b, a = s["before"], s["after"]
        if b["is_player"] or not b["located"] == 0 or b["flag80"]:
            continue
        lane, v, sm = predict_one(b, a)
        r = max(abs(lane - a["lane"]), abs(v - a["v"]), abs(sm - a["s"]))
        n += 1
        worst = max(worst, r)
        exact += r < 1e-6
        # The fresh scores must be only the two fixed bumps: no track object, no craft.
        candidates += a["target"] in [i * 0.25 - 1.0 for i in range(9)]
        fresh = [0.0] * 9
        bump(fresh, 0.0, 1.0, 0.05)
        bump(fresh, b["preferred"], 0.15, 0.5)
        fresh_exact += max(abs(x - y) for x, y in zip(fresh, a["fresh"])) < 1e-6
        print(f"lane {a['lane']:+.6f} pred {lane:+.6f}  v {a['v']:+.7f} pred {v:+.7f}  "
              f"s {a['s']:.5f} pred {sm:.5f}  target {a['target']:+.2f}")
    print(f"{exact}/{n} planner calls reproduced (lane, velocity, s) within 1e-6, worst {worst:.2e}; "
          f"{candidates}/{n} targets on a candidate; {fresh_exact}/{n} fresh score sets are the two "
          f"fixed bumps alone (no track-object or other-craft bump)")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=45091)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--samples", type=int, default=100)
    parser.add_argument("--gap-seconds", type=float, default=0.0)
    parser.add_argument("--drivers", type=int, default=8, help="drivers a round visits")
    parser.add_argument("--predict", type=Path)
    args = parser.parse_args()
    if args.predict:
        predict(args.predict)
    else:
        capture(args)


if __name__ == "__main__":
    main()
