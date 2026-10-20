#!/usr/bin/env python3
"""What each of Wipeout Pure's force functions adds, frame by frame, on a running Pure PSP (USA).

Pure keeps its craft's force and torque accumulators in VFPU registers for the whole craft
frame (`0x0892d214`: `S222` thrust, `C230` the body-local torque, `C300` the world force), so
the change in the VFPU register file across one force function's call is exactly what that
function added. This script stops at the frame (`a0` the craft, `a1` the body), reads the
inputs the laws read, then stops at each force function's entry and at its return site in the
frame and records every VFPU register that changed. See `docs/physics/pure-physics.md`.

**This PPSSPP fires only the most recently added breakpoint**, so the capture walks one
breakpoint at a time through the frame's call order. A function the frame does not call this
frame (the brakes without both airbrakes held) costs a missed frame, so it is not in the walk.

    uv run --with websocket-client python3 scripts/psp-pure-force-capture.py \\
        --port 45697 --out data/traces/pure-venom.json --frames 300 --hold cross,ltrigger \\
        [--steer -0.6] [--pitch 1] [--lift 60] [--sideshift left]

`--lift N` raises the body `N` units at the third frame so the craft falls for a while (air
pitch, air drag, air thrust); `--sideshift left|right` writes `0.2` to the shift timer at the
fifth frame (`entity+0x9dc` / `+0x9e0`) and records the body's world force accumulator
(`body+0x140`) across the airbrake function, which is where the shift lands.

Start PPSSPP with `scripts/ppsspp-start.sh`, walk to a Time Trial and leave it racing. The
captures are derived game data: keep them under `data/`.
"""

import argparse
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

FRAME = 0x0892D214
# name: (entry, return site inside the frame). Steering's return site is a `jal`, which
# PPSSPP will not stop on, so it ends at pitch's entry (nothing runs in between); the same for
# the angular damping, ended by the rolling resistance's entry.
FUNCS = {
    "engine": (0x0892F1F0, 0x0892D6F4),
    "airbrakes": (0x0892F64C, 0x0892D740),
    "steering": (0x0892EDFC, 0x0892CAF8),
    "pitch": (0x0892CAF8, 0x0892D754),
    "drag": (0x0892CBA0, 0x0892D774),
    "hover": (0x0892C6B0, 0x0892D828),
    "grip": (0x0892C994, 0x0892D844),
    "weathervane": (0x0892CB58, 0x0892D878),
    "angdamp": (0x0892CC2C, 0x0892CC94),
    "rollres": (0x0892CC94, 0x0892D88C),
    "speedpad": (0x0892CCD0, 0x0892D8D0),
}


def f32(u):
    return struct.unpack("<f", struct.pack("<I", u & 0xFFFFFFFF))[0]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--frames", type=int, default=120)
    ap.add_argument("--hold", default="cross", help="comma-separated PPSSPP button names")
    ap.add_argument("--steer", type=float, default=0.0)
    ap.add_argument("--pitch", type=float, default=0.0)
    ap.add_argument("--lift", type=float)
    ap.add_argument("--sideshift", choices=["left", "right"])
    ap.add_argument("--budget", type=float, default=540.0, help="wall-clock seconds")
    args = ap.parse_args()

    d = Debugger(args.port)
    d.brk()
    buttons = {b: True for b in args.hold.split(",") if b}
    if buttons:
        d.hold(**buttons)
    d.analog(args.steer, args.pitch)

    def regs():
        out = {}
        for c in d.call("cpu.getAllRegs")["categories"]:
            out.update(zip(c["registerNames"], c["uintValues"]))
        return out

    def floats(at, n):
        return list(struct.unpack("<%df" % n, d.read(at, 4 * n)))

    def stop_at(addr, timeout):
        d.add_breakpoint(addr)
        d.pending = []
        d.call("cpu.resume")
        try:
            d.wait_for_break_any([addr], timeout)
            ok = True
        except TimeoutError:
            d.brk()
            ok = False
        d.remove_breakpoint(addr)
        return ok

    rows = []
    start = time.time()
    try:
        while len(rows) < args.frames and time.time() - start < args.budget:
            if not stop_at(FRAME, 20.0):
                print("the craft frame never ran: is a race live?", file=sys.stderr)
                break
            g = regs()
            craft, body = g["a0"], g["a1"]
            entity = d.read_u32(craft + 0x1C4)
            if args.lift is not None and len(rows) == 3:
                p = floats(body + 0x30, 3)
                d.write_f32s(body + 0x30, [p[0], p[1] + args.lift, p[2]])
            if args.sideshift and len(rows) == 4:
                d.write_f32s(entity + (0x9DC if args.sideshift == "left" else 0x9E0), [0.2])
            cb = d.read(craft, 0x300)
            w = lambda o: struct.unpack_from("<f", cb, o)[0]  # noqa: E731
            u = lambda o: struct.unpack_from("<I", cb, o)[0]  # noqa: E731
            inp = u(0x48)
            row = {
                "ticks": d.call("cpu.status")["ticks"], "dt": f32(g["f12"]),
                "flags": u(0x1C0), "state": u(0x25C), "class": u(0x2AC),
                "grounded": w(0x268), "fs": w(0x2A4), "launch_mul": w(0x24C),
                "stun": w(0x248), "slowdown": w(0x298), "steer_state": w(0x278),
                "airbrake_l": w(0x27C), "airbrake_r": w(0x280),
                "input": floats(inp, 5) if inp else None,
                "position": floats(body + 0x30, 3), "velocity": floats(body + 0x180, 3),
                "rows": floats(body, 12), "angular_velocity": floats(body + 0x1A0, 3),
                "speed": floats(body + 0x3D8, 1)[0], "fx": {}, "at_entry": {},
            }
            for name, (entry, ret) in FUNCS.items():
                if not stop_at(entry, 0.6):
                    continue
                before = regs()
                row["at_entry"][name] = {
                    "velocity_cached": floats(craft + 0x190, 3), "fs": floats(craft + 0x2A4, 1)[0],
                    "grounded": floats(craft + 0x268, 1)[0], "flags": d.read_u32(craft + 0x1C0),
                    "angular_velocity": floats(body + 0x1A0, 3),
                    "input": floats(d.read_u32(craft + 0x48), 5),
                    "rows_cached": floats(craft + 0x80, 12), "right": floats(craft + 0x170, 3),
                    "world_force": floats(body + 0x140, 3),
                }
                if not stop_at(ret, 5.0):
                    continue
                after = regs()
                row["fx"][name] = {
                    k: f32(v) - f32(before[k])
                    for k, v in after.items() if k.startswith("S") and before.get(k) != v
                }
                if name == "airbrakes":
                    w0 = row["at_entry"][name]["world_force"]
                    row["fx"]["airbrakes_world"] = [b - a for a, b in zip(w0, floats(body + 0x140, 3))]
            rows.append(row)
            if len(rows) % 20 == 0:
                print("frame %d, %.0f s" % (len(rows), time.time() - start), file=sys.stderr, flush=True)
    finally:
        d.hold(**{b: False for b in buttons})
        d.analog(0.0, 0.0)
        d.resume()
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(rows))
        print("wrote %s (%d frames)" % (args.out, len(rows)), file=sys.stderr)


if __name__ == "__main__":
    main()
