#!/usr/bin/env python3
"""Drive Pulse's Time Trial craft under player input and log what its hover escape reads.

`Ship_HoverTwoPoint` (`0x0884a658`) stores each probe's height along up in `craft+0x308 + 4i`
and, when it is under `1.0` and the hit record's shape tag `craft+0x208 + 0x30i` is `1`, moves
the body by `up * (1 - h)` (`0x0884a808`-`0x0884a86c`) before it tests the collider's class
(`*(*(craft+0x300) + 12 * craft+0x200 + 0x2454) + 0x6c`, `1` Floor or `3` MagFloor,
`0x0884a878`-`0x0884a8c4`). This parks on `Ship_UpdateCraft` (`0x08849618`) once a tick, so it
reads the previous tick's hover state: per probe the hit byte, height, tag and class, plus the
magnet-lock blend `craft+0x280`, the body's position and velocity and the frame's `dt`. The
escape a tick made is then visible in the next tick's rows as
`dot(pos(t+1) - pos(t) - v(t+1) * dt, up)`.

Player input only: the throttle is held below `--cap` and the stick is this project's
pure-pursuit controller on the authored racing line (`psp-autopilot.py`'s `steer_for`), sent
through the virtual pad. The original's Autopilot is never armed.

    uv run --with websocket-client python3 scripts/psp-magstrip-escape.py --port 9315 \\
        --spline 16_track.csv --out escape.jsonl

Start it with the Time Trial's countdown over (a restored grid state, then GO). Raw logs are
derived game data: keep them under `data/`.
"""

import argparse
import importlib.util
import json
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import input_script  # noqa: E402
from ppsspp_debugger import Debugger  # noqa: E402

spec = importlib.util.spec_from_file_location("psp_autopilot", HERE / "psp-autopilot.py")
autopilot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(autopilot)

SHIP_UPDATE_CRAFT = 0x08849618


def buttons(held):
    names = ("cross", "left", "right", "l", "r")
    return {input_script.PPSSPP_NAME[name]: name in held for name in names}


def probes(dbg, craft):
    raw = dbg.read(craft, 0x320)
    table = struct.unpack_from("<I", raw, 0x300)[0]
    out = []
    for i in range(2):
        rec = 0x1E0 + 0x30 * i
        hit = raw[0x1D8 + i]
        index = struct.unpack_from("<i", raw, rec + 0x20)[0]
        surface = None
        if hit and 0 <= index < 4096:
            obj = dbg.read_u32(table + 12 * index + 0x2454)
            surface = dbg.read_u32(obj + 0x6C) if obj else None
        out.append({
            "hit": hit,
            "h": struct.unpack_from("<f", raw, 0x308 + 4 * i)[0],
            "tag": struct.unpack_from("<I", raw, rec + 0x28)[0],
            "surface": surface,
            "point": struct.unpack_from("<3f", raw, rec),
        })
    return out, struct.unpack_from("<f", raw, 0x280)[0], struct.unpack_from("<3f", raw, 0x160)


def run(args):
    line = autopilot.Line(args.spline, "line")
    dbg = Debugger(args.port)
    out = args.out.open("w")
    craft = None
    index = None
    ticks = 0
    mag_ticks = 0
    dbg.brk()
    dbg.add_breakpoint(SHIP_UPDATE_CRAFT)
    try:
        while ticks < args.max_ticks and mag_ticks < args.mag_ticks:
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            dbg.wait_for_break_any((SHIP_UPDATE_CRAFT,), timeout=120)
            regs = dbg.call("cpu.getAllRegs")["categories"]
            gpr = next(c for c in regs if c["name"] == "GPR")
            fpu = next(c for c in regs if c["name"] == "FPU")
            hit = dict(zip(gpr["registerNames"], gpr["uintValues"]))["a0"]
            craft = craft or hit
            if hit != craft:
                continue
            dt = struct.unpack("<f", struct.pack("<I", dict(zip(fpu["registerNames"], fpu["uintValues"]))["f12"]))[0]
            body = dbg.read_u32(craft + 0x1CC)
            raw = dbg.read(body, 0x3A0)
            state = {
                "position": struct.unpack_from("<3f", raw, 0x30),
                "velocity": struct.unpack_from("<3f", raw, 0x140),
                "row0": struct.unpack_from("<3f", raw, 0x00),
                "up": struct.unpack_from("<3f", raw, 0x10),
                "forward": struct.unpack_from("<3f", raw, 0x20),
            }
            if index is None:
                index, _ = line.nearest(state["position"], 0, back=len(line), ahead=0)
            index, _ = line.nearest(state["position"], index)
            speed = autopilot.length(state["velocity"])
            held, _, _ = autopilot.steer_for(state, line, index, args)
            held = [b for b in held if b != "cross"] + (["cross"] if speed < args.cap else [])
            dbg.hold(**buttons(held))
            probe, blend, hover_up = probes(dbg, craft)
            if any(p["surface"] == 3 for p in probe):
                mag_ticks += 1
            row = {
                "tick": ticks, "row": index, "dt": dt,
                "pos": state["position"], "vel": state["velocity"], "up": state["up"],
                "hover_up": hover_up, "speed": speed, "held": held,
                "mag_blend": blend, "probes": probe,
            }
            out.write(json.dumps(row, separators=(",", ":")) + "\n")
            ticks += 1
            if ticks % 200 == 0:
                out.flush()
                print("tick %d row %d speed %.1f mag ticks %d" % (ticks, index, speed, mag_ticks),
                      file=sys.stderr, flush=True)
    finally:
        out.close()
        dbg.brk()
        dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)
        dbg.hold(**buttons([]))
        dbg.resume()
        dbg.close()
    print("wrote %s: %d ticks, %d over a magstrip" % (args.out, ticks, mag_ticks))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument("--spline", type=Path, required=True, help="`oag-trace track` output")
    parser.add_argument("--cap", type=float, default=999.0, help="hold the throttle only below this body speed")
    parser.add_argument("--max-ticks", type=int, default=4000)
    parser.add_argument("--mag-ticks", type=int, default=400, help="stop after this many magstrip ticks")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--look-min", type=float, default=18.0)
    parser.add_argument("--look-speed", type=float, default=0.55)
    parser.add_argument("--look-max", type=float, default=90.0)
    parser.add_argument("--deadband", type=float, default=0.045)
    parser.add_argument("--brake-at", type=float, default=0.32)
    parser.add_argument("--reverse-at", type=float, default=-0.2)
    run(parser.parse_args())


if __name__ == "__main__":
    main()
