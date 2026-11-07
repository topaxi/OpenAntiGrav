#!/usr/bin/env python3
"""Read what Pulse PSP stamps a weapon pad with, live, in whatever mode the race is.

    uv run --with websocket-client python3 scripts/psp-weapon-pad-stamp.py --port 47810 --pad 8 --hits 4

`WeaponPads_TestCraft` (`0x0888727c`) ends a hit with `swc1 f12, 0x1a0(s7)` at `0x088873dc`: `f12` is the
value (`<WeaponPad elimination_refresh_time>` when `g_game_mode == 8`, `refresh_time` otherwise) and `s7`
is the pad. This puts the player's craft on one weapon pad (written at a `Ship_UpdateCraft` hit, the way
`psp-drive.py place` does), then breaks on that store and prints `g_game_mode`, `f12`, the pad and the
craft's pickup slot as it is. The race has to be up with weapons on
(`psp-drive.py menu --race-type 6 --weapons-on` for Eliminator, `--single-race --weapons-on` for the
control). Pads are read out of the world (`world+0x10c`, count at `world+0x1d4`); each pad's matrix is
`*(*(pad+8)+0x30)`, translation in row 3.
"""

import argparse
import importlib.util
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402

_spec = importlib.util.spec_from_file_location("psp_drive", HERE / "psp-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)

STAMP = 0x088873DC
G_GAME_MODE = 0x08B31048
WORLD = 0x08B32C88
LIFT = 3.0


def regs(dbg):
    registers = dbg.call("cpu.getAllRegs")
    out = {}
    for category in registers["categories"]:
        names = category["registerNames"]
        values = category.get("floatValues") or category.get("uintValues")
        out[category["name"]] = dict(zip(names, values))
    return out, registers


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, default=47810)
    ap.add_argument("--pad", type=int, default=0)
    ap.add_argument("--hits", type=int, default=4)
    ap.add_argument("--lift", type=float, default=LIFT)
    ap.add_argument("--list", action="store_true", help="only list the pads")
    ap.add_argument("--frames", type=int, default=0, help="after the first stamp, read the stamped pad's timer "
                    "(+0x1a0) and colour (+0x6c) at this many of the player's `Ship_UpdateCraft` hits")
    args = ap.parse_args()
    dbg = Debugger(args.port)
    mode = dbg.read_u32(G_GAME_MODE)
    world = dbg.read_u32(WORLD)
    count = dbg.read_u32(world + 0x1D4)
    print("g_game_mode %d, weapons byte %d, %d weapon pads" % (mode, dbg.read_u8(0x08B30FA8), count), flush=True)
    pads = []
    for i in range(count):
        pad = dbg.read_u32(world + 0x10C + 4 * i)
        matrix = dbg.read_u32(dbg.read_u32(pad + 8) + 0x30)
        m = dbg.read_f32s(matrix, 16)
        pads.append((pad, m))
        print("  pad %d 0x%08x at (%.1f, %.1f, %.1f)" % (i, pad, *m[12:15]), flush=True)
    if args.list:
        return
    pad, m = pads[args.pad]
    up = m[4:7]
    forward = m[8:11]
    position = tuple(m[12 + k] + up[k] * args.lift for k in range(3))
    craft = drive.find_craft(dbg)
    print("craft 0x%08x; placing it on pad %d at (%.1f, %.1f, %.1f)" % (craft, args.pad, *position), flush=True)
    row2 = drive._normalized(forward)
    row1 = drive._normalized(up)
    row0 = drive._cross(row1, row2)
    placed = False
    for _, _ in dbg.each_hit(drive.SHIP_UPDATE_CRAFT, 40, timeout=60):
        reg = dbg.call("cpu.getAllRegs")
        gpr = next(c for c in reg["categories"] if c["name"] == "GPR")
        hit = dict(zip(gpr["registerNames"], gpr["uintValues"]))[drive.CRAFT_REGISTER]
        if hit != craft:
            continue
        body = dbg.read_u32(craft + drive.BODY_POINTER)
        dbg.write_f32s(body + drive.BODY_ROW0, row0)
        dbg.write_f32s(body + drive.BODY_UP, row1)
        dbg.write_f32s(body + drive.BODY_FORWARD, row2)
        dbg.write_f32s(body + drive.BODY_POSITION, position)
        for slot in range(3):
            dbg.write_f32s(body + drive.BODY_TRANSPOSE + 0x10 * slot, (row0[slot], row1[slot], row2[slot]))
        dbg.write_f32s(body + drive.BODY_VELOCITY, (0.0, 0.0, 0.0))
        dbg.write_f32s(body + drive.BODY_OMEGA, (0.0, 0.0, 0.0))
        dbg.write_f32s(body + drive.BODY_AVEL, (0.0, 0.0, 0.0))
        placed = True
        break
    if not placed:
        sys.exit("never got a Ship_UpdateCraft hit for the player's craft")
    print("placed; waiting for the stamp store at 0x%08x" % STAMP, flush=True)
    seen = 0
    for _, msg in dbg.each_hit(STAMP, args.hits, timeout=60):
        reg = dbg.call("cpu.getAllRegs")
        cats = {c["name"]: c for c in reg["categories"]}
        gpr = dict(zip(cats["GPR"]["registerNames"], cats["GPR"]["uintValues"]))
        fpr = None
        for name, cat in cats.items():
            if "f12" in cat["registerNames"]:
                fpr = dict(zip(cat["registerNames"], cat.get("floatValues") or cat.get("uintValues")))
        stamped = fpr["f12"] if fpr else None
        pad_hit = gpr["s7"]
        print("stamp hit: g_game_mode %d, f12 %s, pad 0x%08x (index %s)" % (
            dbg.read_u32(G_GAME_MODE), stamped, pad_hit,
            [i for i, (p, _) in enumerate(pads) if p == pad_hit]), flush=True)
        seen += 1
        if args.frames:
            break
    print("%d stamp hit(s)" % seen)
    if args.frames:
        print("frame  timer       colour(+0x6c)")
        frame = 0
        for _, _ in dbg.each_hit(drive.SHIP_UPDATE_CRAFT, args.frames * 12, timeout=60):
            reg = dbg.call("cpu.getAllRegs")
            gpr = next(c for c in reg["categories"] if c["name"] == "GPR")
            if dict(zip(gpr["registerNames"], gpr["uintValues"]))[drive.CRAFT_REGISTER] != craft:
                continue
            timer = dbg.read_f32(pad_hit + 0x1A0)
            colour = dbg.read_u32(pad_hit + 0x6C)
            print("%5d  %.6f  0x%08x" % (frame, timer, colour), flush=True)
            frame += 1
            if frame >= args.frames:
                break


if __name__ == "__main__":
    main()
