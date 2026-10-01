#!/usr/bin/env python3
"""Capture the original's LeachBeam reticle, frame by frame, and replay our law.

`FUN_0881e8c8` is the LeachBeam's own sight update (`docs/ghidra/functions/
psp-pulse-usa/lock-sight.md`, "The LeachBeam's reticle is its own function").
This reads its state off a live PPSSPP at every `HudSight_Update` call and
replays `oag_race::sight`'s law (the same arithmetic, in Python) against the
capture, step by step.

    # a Single Race, once the countdown is over; own HOME, own port
    uv run --with websocket-client scripts/psp-leach-sight-capture.py capture \\
        --port 45683 --frames 140 --fire-at 50 --out leach.json
    python3 scripts/psp-leach-sight-capture.py replay leach.json

What `capture` does to the game, because none of it is a normal way to play:
it writes the LeachBeam id (`10`) into the player's held-weapon slot, holds one
opponent `60` units ahead of the player on every frame (`HudSight_Update`'s
entry is where it writes, so the HUD reads the pinned pose that same call), and
at `--fire-at` sets the fire bit with `craft+0x16c = -1`, the **unlocked** arm
of `Weapon_FireLeachBeam`. The locked arm needs the target's matrix pointer in
`craft+0x168` and halts PPSSPP without one (`scripts/psp-fire-weapon.py`).
Capture output is derived game data: keep it under `data/`, never commit it.

`replay` resyncs to the observation after every step, so it measures the error
of one frame's arithmetic and not of a drifting run.
"""

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

HUD_SIGHT_UPDATE = 0x0881DBCC
SHIP_UPDATE_CRAFT = 0x08849618
G_RACE_MANAGER = 0x08B317B4
DT_GLOBAL = 0x08AB0A40
BODY_POINTER, POS, FWD, VEL = 0x1CC, 0x30, 0x20, 0x140


def gpr(dbg):
    regs = dbg.call("cpu.getAllRegs")
    cat = next(c for c in regs["categories"] if c["name"] == "GPR")
    return dict(zip(cat["registerNames"], cat["uintValues"]))


def capture(args):
    from ppsspp_debugger import Debugger

    dbg = Debugger(args.port)
    entity = dbg.read_u32(dbg.read_u32(G_RACE_MANAGER) + 0x2C0)
    weapon_record = dbg.read_u32(entity + 0x4C)
    crafts = []
    for _, _ in dbg.each_hit(SHIP_UPDATE_CRAFT, 24, timeout=30):
        craft = gpr(dbg)["a0"]
        if craft not in crafts:
            crafts.append(craft)
    player = crafts[0]
    body = dbg.read_u32(player + BODY_POINTER)
    where = dbg.read_f32s(body + POS, 3)
    opponent = min(
        crafts[1:],
        key=lambda c: math.dist(dbg.read_f32s(dbg.read_u32(c + BODY_POINTER) + POS, 3), where),
    )
    opponent_body = dbg.read_u32(opponent + BODY_POINTER)
    dbg.write_u32(weapon_record + 0x1BC, 10)
    dbg.write_u32(weapon_record + 0x16C, 0xFFFFFFFF)
    out = []
    for i, _ in dbg.each_hit(HUD_SIGHT_UPDATE, args.frames, timeout=30):
        hud = gpr(dbg)["a0"]
        pos, fwd = dbg.read_f32s(body + POS, 3), dbg.read_f32s(body + FWD, 3)
        norm = math.sqrt(sum(c * c for c in fwd)) or 1.0
        dbg.write_f32s(opponent_body + POS, tuple(p + f / norm * 60.0 for p, f in zip(pos, fwd)))
        dbg.write_f32s(opponent_body + VEL, (0.0, 0.0, 0.0))
        view = dbg.read_u32(hud + 0x3C)
        out.append(
            {
                "i": i,
                "dt": dbg.read_f32s(DT_GLOBAL, 1)[0],
                "v48": dbg.read_u32(view + 0x48),
                "prev_vis": dbg.read_u8(view + 0xF1),
                "ext": dbg.read_f32s(hud + 0x27C, 1)[0],
                "spin": dbg.read_f32s(hud + 0x104, 1)[0],
                "locked": dbg.read_u8(hud + 0xF8),
                "held": dbg.read_u32(weapon_record + 0x1BC),
            }
        )
        if i == args.fire_at:
            dbg.write_u32(weapon_record + 0x1B8, dbg.read_u32(weapon_record + 0x1B8) | 0x8000)
    dbg.resume()
    dbg.close()
    Path(args.out).write_text(json.dumps(out))
    print(f"{len(out)} frames -> {args.out}")


def replay(args):
    rs = json.loads(Path(args.file).read_text())
    ext, spin, seen = rs[0]["ext"], rs[0]["spin"], bool(rs[0]["prev_vis"])
    worst = [0.0, 0.0, 0]
    for i in range(len(rs) - 1):
        dt, visible, obs = rs[i]["dt"], bool(rs[i + 1]["prev_vis"]), rs[i + 1]
        if visible and not seen:
            ext = 30.0
        wanted, reference = (6.0, 6.0) if visible else (30.0, 9.6)
        step = dt * 50.0
        if ext > reference:
            step *= 1.4
        if not visible:
            step *= 1.5
        ext = min(ext + step, wanted) if ext < wanted else max(ext - step, wanted)
        locked = ext == reference
        spin = spin + dt * (4.0 if locked else 2.0) if visible else spin - 2.0 * dt
        spin = math.fmod(spin, 2 * math.pi)
        worst[0] = max(worst[0], abs(ext - obs["ext"]))
        worst[1] = max(worst[1], abs(spin - obs["spin"]))
        worst[2] += int(locked != bool(obs["locked"]))
        ext, spin, seen = obs["ext"], obs["spin"], visible
    print(
        "%d steps: worst extent error %.5f, worst spin error %.6f rad, lock disagreements %d"
        % (len(rs) - 1, *worst)
    )


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("capture")
    p.add_argument("--port", type=int, required=True)
    p.add_argument("--frames", type=int, default=140)
    p.add_argument("--fire-at", type=int, default=-1)
    p.add_argument("--out", required=True)
    p = sub.add_parser("replay")
    p.add_argument("file")
    args = ap.parse_args()
    capture(args) if args.cmd == "capture" else replay(args)


if __name__ == "__main__":
    main()
