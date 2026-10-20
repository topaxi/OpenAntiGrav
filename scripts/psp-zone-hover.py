#!/usr/bin/env python3
"""Read Pulse's four-corner (Zone) hover live in PPSSPP, one law at a time.

    source data/scratch/<lane>/pp/env.sh
    uv run --with websocket-client --with numpy python3 scripts/psp-zone-hover.py \\
        --port $OAG_PPSSPP_PORT --out data/scratch/<lane>/zone-hover.jsonl \\
        --phase spring --hits 400

Writes `g_game_mode` (`0x08b31048`) to 6 first, so `Ship_UpdateHover` runs
`Ship_HoverFourCorner` (`0x0884ae90`) from the next frame on whatever race is
running (the Time Trial grid state, by default: a fresh profile cannot pick Zone).
Each phase arms one breakpoint on a callee's entry, conditional on the return
address inside `Ship_HoverFourCorner`, so only that call stops. PPSSPP v1.20.4
fires only the newest execution breakpoint, so the phases run one after another.
At the callee's entry the caller's frame is intact, which is where every read
below comes from.

- `spring`: `Body_AddForceAtPoint` (`0x0884d510`) from `0x0884b554`: the force
  (`a1`), the probe point (`a2`), the hit record (`sp+0x50` point, `+0x60`
  normal), the height (`craft+0x308 + 4i`), the point velocity (`sp+0x120`) and
  every input of the spring.
- `torque`: `Body_AddTorqueWorld` (`0x0884d604`) from `0x0884b758`: the torque
  (`a1`), the scaled normal (`sp+0x40`), the contact count (`sp+0xc84`), up.
- `downforce`: `Body_AddForceWorld` (`0x0884d4c8`) from `0x0884b818`: the force
  (`a1`), the normal, mass and the class block's track gravity.
- `snap`: `Body_Translate` (`0x0884d9f8`) from `0x0884b9bc`: the move
  (`f12..f14`), the 10-unit ray's hit (`sp+0x50`, `+0x60`) and position.

Every row is the raw floats; `scripts/psp-zone-hover-check.py` checks them offline.
"""

import argparse
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

G_GAME_MODE = 0x08B31048
G_CLASS = 0x08B31040
K_GLOBAL = 0x08AB0E20
SCALE_GLOBAL = 0x08AB0E1C

PHASES = {
    "spring": (0x0884D510, 0x0884B55C),
    "torque": (0x0884D604, 0x0884B760),
    "downforce": (0x0884D4C8, 0x0884B820),
    "snap": (0x0884D9F8, 0x0884B9C4),
}


def regs(dbg):
    cats = dbg.call("cpu.getAllRegs")["categories"]
    out = {}
    for cat in cats[:2]:
        out.update(zip(cat["registerNames"], cat["uintValues"]))
    return out


def f32(u):
    return struct.unpack("<f", struct.pack("<I", u & 0xFFFFFFFF))[0]


def vec(dbg, addr, n=4):
    return list(struct.unpack("<%df" % n, dbg.read(addr, 4 * n)))


def craft_common(dbg, craft):
    body = dbg.read_u32(craft + 0x1CC)
    return {
        "basis": vec(dbg, craft + 0x80, 16),
        "up": vec(dbg, craft + 0x160),
        "right": vec(dbg, craft + 0x170),
        "vel": vec(dbg, craft + 0x190),
        "pos": vec(dbg, craft + 0x1A0),
        "target": vec(dbg, craft + 0x2F0, 1)[0],
        "grounded": vec(dbg, craft + 0x2B0, 1)[0],
        "maglock": vec(dbg, craft + 0x280, 1)[0],
        "state_2a4": dbg.read_u32(craft + 0x2A4),
        "heights": vec(dbg, craft + 0x308, 4),
        "mass": vec(dbg, body + 0x374, 1)[0],
        "omega_150": vec(dbg, body + 0x150),
        "body_pos": vec(dbg, body + 0x30),
    }


HOVER_FOUR_CORNER = 0x0884AE90


def perturb(dbg, lift, pitch):
    """At `Ship_HoverFourCorner`'s entry, raise the craft by `lift` (body and the craft's copy)
    and tilt the basis rows the probes are placed with (`craft+0x80..0xa0`) by `pitch` degrees
    about the right axis, for this one frame: a frame with fewer than four contacts."""
    for _ in dbg.each_hit(HOVER_FOUR_CORNER, 1, timeout=30.0):
        craft = regs(dbg)["a0"]
        body = dbg.read_u32(craft + 0x1CC)
        up = vec(dbg, craft + 0x160, 3)
        for addr in (body + 0x30, craft + 0x1A0):
            dbg.write_f32s(addr, [p + lift * u for p, u in zip(vec(dbg, addr, 3), up)])
        if pitch:
            import math
            c, s = math.cos(math.radians(pitch)), math.sin(math.radians(pitch))
            up = vec(dbg, craft + 0x90, 3)
            fw = vec(dbg, craft + 0xA0, 3)
            dbg.write_f32s(craft + 0x90, [c * u - s * f for u, f in zip(up, fw)])
            dbg.write_f32s(craft + 0xA0, [s * u + c * f for u, f in zip(up, fw)])
        return craft


def capture(dbg, phase, hits, out, lift=0.0, pitch=0.0):
    callee, ret = PHASES[phase]
    cond = "ra == 0x%08x" % ret
    n = 0
    if lift or pitch:
        hits_iter = []
        for trial in range(hits):
            perturb(dbg, lift, pitch)
            try:
                hits_iter.extend(_rows(dbg, phase, 1, cond, callee, out, trial))
            except TimeoutError:
                print("trial %d: %s not reached" % (trial, phase), file=sys.stderr, flush=True)
            dbg.resume()
            time.sleep(1.0)
            dbg.brk()
        return len(hits_iter)
    return len(list(_rows(dbg, phase, hits, cond, callee, out, 0)))


def _rows(dbg, phase, hits, cond, callee, out, trial):
    for index, _ in dbg.each_hit(callee, hits, timeout=5.0 if trial else 30.0, condition=cond):
        r = regs(dbg)
        sp, s0 = r["sp"], r["s0"]
        row = {"phase": phase, "hit": index, "trial": trial, "craft": hex(s0)}
        row.update(craft_common(dbg, s0))
        row["K"] = vec(dbg, K_GLOBAL, 1)[0]
        row["scale"] = vec(dbg, SCALE_GLOBAL, 1)[0]
        if phase == "spring":
            row["probe"] = r["s1"]
            row["force"] = vec(dbg, r["a1"])
            row["point"] = vec(dbg, r["a2"])
            row["offset"] = vec(dbg, sp + 0xB0)
            row["hit_point"] = vec(dbg, sp + 0x50)
            row["hit_normal"] = vec(dbg, sp + 0x60)
            row["h"] = vec(dbg, r["s2"] + 0x308, 1)[0]
            row["point_vel"] = vec(dbg, sp + 0x120)
            row["grav"] = vec(dbg, sp + 0xC88, 1)[0]
            row["contacts_so_far"] = dbg.read_u32(sp + 0xC80)
        elif phase == "torque":
            row["torque"] = vec(dbg, r["a1"])
            row["normal"] = vec(dbg, sp + 0x40)
            row["inv"] = vec(dbg, sp + 0x6D0, 1)[0]
            row["contacts"] = dbg.read_u32(sp + 0xC84)
            row["contacts_50"] = dbg.read_u32(sp + 0xC80)
        elif phase == "downforce":
            row["force"] = vec(dbg, r["a1"])
            row["normal"] = vec(dbg, sp + 0x40)
            cls = dbg.read_u32(G_CLASS)
            block = dbg.read_u32(s0 + 0x6C)
            row["class"] = cls
            row["track_gravity"] = vec(dbg, block + cls * 0x80 + 0x100, 1)[0]
            row["contacts"] = dbg.read_u32(sp + 0xC84)
        elif phase == "snap":
            row["move"] = [f32(r["f12"]), f32(r["f13"]), f32(r["f14"])]
            row["hit_point"] = vec(dbg, sp + 0x50)
            row["hit_normal"] = vec(dbg, sp + 0x60)
            row["d"] = vec(dbg, sp + 0x910, 1)[0]
        out.write(json.dumps(row) + "\n")
        out.flush()
        yield row


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--phase", choices=sorted(PHASES), action="append", required=True)
    ap.add_argument("--hits", type=int, default=200)
    ap.add_argument("--accelerate", action="store_true", help="hold cross throughout")
    ap.add_argument("--steer", type=int, default=0, help="analog x, -128..127, held throughout")
    ap.add_argument("--lift", type=float, default=0.0, help="per trial: raise the craft this far")
    ap.add_argument("--pitch", type=float, default=0.0, help="per trial: tilt the probe basis")
    args = ap.parse_args()
    with Debugger(port=args.port) as dbg, open(args.out, "a") as out:
        dbg.brk()
        mode = dbg.read_u32(G_GAME_MODE)
        print("g_game_mode was %d" % mode, file=sys.stderr, flush=True)
        dbg.write_u32(G_GAME_MODE, 6)
        if args.accelerate:
            dbg.hold(cross=True)
        if args.steer:
            dbg.analog(args.steer / 128.0, 0.0)
        for phase in args.phase:
            t = time.time()
            n = capture(dbg, phase, args.hits, out, args.lift, args.pitch)
            print("%s: %d rows in %.1fs" % (phase, n, time.time() - t), file=sys.stderr, flush=True)
        dbg.resume()


if __name__ == "__main__":
    main()
