#!/usr/bin/env python3
"""Place the player's craft at a pose, then log it every tick: how a fall and its reset look.

`psp-drive.py place` teleports and reports one settled pose; this keeps logging, so a craft
put over a gap (or at the lip of a jump, at a chosen speed) falls and the game's reset shows
as a row where the position jumps. Each row is one `Ship_Update` of the craft: CPU cycle
counter (so a frame count gone between rows can be read), body position, forward row, velocity,
body speed, and the craft words `0x2b0..0x2f0` (the grounded fraction at `+0x2b0`, throttle
`+0x2b8`, steer `+0x2c0`, cached speed `+0x2ec`).

    uv run --with websocket-client python3 scripts/psp-fall-log.py --port 45091 \
        --pos -640,21,241 --tangent -1,0,0 --up 0,1,0 --speed 40 --ticks 300 --out fall.jsonl

Raw logs are derived game data: keep them under `data/`, never commit them.
"""

import argparse
import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

SHIP_UPDATE_CRAFT = 0x08849618
BODY_POINTER = 0x1CC


def vec(text):
    return tuple(float(p) for p in text.split(","))


def norm(v):
    n = sum(c * c for c in v) ** 0.5
    return tuple(c / n for c in v)


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def run(args):
    dbg = Debugger(args.port)
    row2 = norm(args.tangent)
    up = norm(args.up)
    lean = sum(a * b for a, b in zip(up, row2))
    row1 = norm(tuple(u - lean * f for u, f in zip(up, row2)))
    row0 = cross(row1, row2)
    velocity = tuple(c * args.speed for c in row2)
    out = args.out.open("w")
    craft = None
    placed = False
    logged = 0
    dbg.brk()
    dbg.add_breakpoint(SHIP_UPDATE_CRAFT)
    try:
        while logged < args.ticks:
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            dbg.wait_for_break_any((SHIP_UPDATE_CRAFT,), timeout=120)
            registers = dbg.call("cpu.getAllRegs")
            gpr = next(c for c in registers["categories"] if c["name"] == "GPR")
            hit = dict(zip(gpr["registerNames"], gpr["uintValues"]))["a0"]
            if craft is None:
                craft = hit
            if hit != craft:
                continue
            body = dbg.read_u32(craft + BODY_POINTER)
            if not placed:
                dbg.write(body + 0x00, struct.pack("<3f", *row0))
                dbg.write(body + 0x10, struct.pack("<3f", *row1))
                dbg.write(body + 0x20, struct.pack("<3f", *row2))
                dbg.write(body + 0x30, struct.pack("<3f", *args.pos))
                for slot in range(3):
                    dbg.write(body + 0xC0 + 0x10 * slot, struct.pack("<3f", row0[slot], row1[slot], row2[slot]))
                dbg.write(body + 0x140, struct.pack("<3f", *velocity))
                dbg.write(body + 0x150, struct.pack("<3f", 0, 0, 0))
                dbg.write(body + 0x160, struct.pack("<3f", 0, 0, 0))
                placed = True
                print("placed at %s facing %s speed %.1f" % (args.pos, row2, args.speed), file=sys.stderr)
            raw = dbg.read(body + 0x20, 0x398 + 4 - 0x20)
            fwd = struct.unpack_from("<3f", raw, 0)
            pos = struct.unpack_from("<3f", raw, 0x30 - 0x20)
            vel = struct.unpack_from("<3f", raw, 0x140 - 0x20)
            speed = struct.unpack_from("<f", raw, 0x398 - 0x20)[0]
            words = struct.unpack("<16f", dbg.read(craft + 0x2B0, 0x40))
            row = {
                "cycles": dbg.call("cpu.status")["ticks"],
                "pos": pos, "fwd": fwd, "vel": vel, "speed": speed,
                "grounded": words[0], "throttle": words[2], "steer": words[4], "cached_speed": words[15],
            }
            out.write(json.dumps(row, separators=(",", ":")) + "\n")
            logged += 1
            if logged % 60 == 0:
                out.flush()
                print("tick %d pos (%.1f, %.1f, %.1f) speed %.1f" % (logged, *pos, speed), file=sys.stderr, flush=True)
    finally:
        out.close()
        dbg.brk()
        dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)
        dbg.resume()
        dbg.close()
    print("wrote %s: %d ticks" % (args.out, logged))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument("--pos", type=vec, required=True, metavar="X,Y,Z")
    parser.add_argument("--tangent", type=vec, required=True, metavar="X,Y,Z")
    parser.add_argument("--up", type=vec, default=(0.0, 1.0, 0.0), metavar="X,Y,Z")
    parser.add_argument("--speed", type=float, default=0.0)
    parser.add_argument("--ticks", type=int, default=300)
    parser.add_argument("--out", type=Path, required=True)
    run(parser.parse_args())


if __name__ == "__main__":
    main()
