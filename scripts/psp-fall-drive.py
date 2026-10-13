#!/usr/bin/env python3
"""Drive the Time Trial craft along the spline at a capped speed and log what a fall does.

`psp-fall-log.py` teleports; a teleport lays no crumbs on the way (the reset ring only fills
while the craft is grounded and moving), so its resets can come from stale crumbs. This drives
in for real: every tick it steers the craft at the authored racing line with
`psp-autopilot.py`'s pure-pursuit controller, holds the throttle only while the body speed is
under `--cap` (a crawl up the ramp) and, with `--stop-row`, lets go of it from that spline row
on (brake to a stop on the lip and drift off). It logs the same rows as `psp-fall-log.py --ring`
(pose, velocity, the entity's reset crumb ring) until `--after` ticks after the first reset.

    uv run --with websocket-client python3 scripts/psp-fall-drive.py --port 45091 \
        --spline 13_track.csv --cap 75 --out drive.jsonl

Start it with the Time Trial's countdown over (`psp-drive.py menu`, then wait). Raw logs are
derived game data: keep them under `data/`.
"""

import argparse
import importlib.util
import json
import math
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import input_script
from ppsspp_debugger import Debugger

spec = importlib.util.spec_from_file_location("psp_autopilot", HERE / "psp-autopilot.py")
autopilot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(autopilot)

SHIP_UPDATE_CRAFT = 0x08849618


def buttons(held):
    names = ("cross", "left", "right", "l", "r")
    return {input_script.PPSSPP_NAME[name]: name in held for name in names}


def run(args):
    line = autopilot.Line(args.spline, "line")
    dbg = Debugger(args.port)
    out = args.out.open("w")
    craft = None
    index = None
    ticks = 0
    reset_at = None
    prev = None
    dbg.brk()
    dbg.add_breakpoint(SHIP_UPDATE_CRAFT)
    try:
        while ticks < args.max_ticks:
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            dbg.wait_for_break_any((SHIP_UPDATE_CRAFT,), timeout=120)
            gpr = next(c for c in dbg.call("cpu.getAllRegs")["categories"] if c["name"] == "GPR")
            hit = dict(zip(gpr["registerNames"], gpr["uintValues"]))["a0"]
            craft = craft or hit
            if hit != craft:
                continue
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
            throttle = speed < args.cap and not (args.stop_row is not None and args.stop_row <= index < args.stop_row + 400)
            held = [b for b in held if b != "cross"] + (["cross"] if throttle else [])
            dbg.hold(**buttons(held))

            entity = dbg.read_u32(craft + 0x1C4)
            blk = dbg.read(entity + 0x380, 0x200)
            head, oldest = struct.unpack("<II", dbg.read(entity + 0x780, 8))
            row = {
                "cycles": dbg.call("cpu.status")["ticks"],
                "row": index,
                "pos": state["position"], "fwd": state["forward"], "vel": state["velocity"],
                "speed": speed,
                "grounded": struct.unpack("<f", dbg.read(craft + 0x2B0, 4))[0],
                "held": held,
                "ring": {
                    "crumbs": [struct.unpack_from("<3f", blk, i * 0x40 + 0x30) for i in range(8)],
                    "head": head, "oldest": oldest,
                    "latch": struct.unpack("<f", dbg.read(entity + 0x78C, 4))[0],
                    "flag": dbg.read_u8(entity + 0xB51),
                },
            }
            out.write(json.dumps(row, separators=(",", ":")) + "\n")
            ticks += 1
            if prev and math.dist(prev, state["position"]) > 12 and reset_at is None:
                reset_at = ticks
                print("reset at tick %d" % ticks, file=sys.stderr, flush=True)
            prev = state["position"]
            if ticks % 100 == 0:
                out.flush()
                print("tick %d row %d speed %.1f" % (ticks, index, speed), file=sys.stderr, flush=True)
            if reset_at is not None and ticks - reset_at >= args.after:
                break
    finally:
        out.close()
        dbg.brk()
        dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)
        dbg.hold(**buttons([]))
        dbg.resume()
        dbg.close()
    print("wrote %s: %d ticks, reset %s" % (args.out, ticks, reset_at))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument("--spline", type=Path, required=True, help="`oag-trace track` output")
    parser.add_argument("--cap", type=float, default=75.0, help="hold the throttle only below this body speed")
    parser.add_argument("--stop-row", type=int, help="release the throttle from this spline row on (400 rows)")
    parser.add_argument("--after", type=int, default=40, help="ticks to keep logging after the reset")
    parser.add_argument("--max-ticks", type=int, default=8000)
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
