#!/usr/bin/env python3
"""Capture HD's zoom-history buffers at chosen points of a Turbo pulse, from a restored save state.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-zoom-capture.py --out DIR \\
        --delay 0.45 --delay 0.9 [--tries 12] [--damage]

Each `--delay` fires one Turbo (state 4 into the pickup slot, triangle), waits that long, pauses the game through
the GDB stub and reads the half-resolution scene `0x02150000` and the ping-pong pair `0x02300000`/`0x022c0000`
through `/proc/<pid>/mem`, with the `FunkLayer` inputs. A pause can land inside the post chain (one buffer of the
pair one frame behind), so a capture is kept only when `scripts/hd-zoom-history-fit.py` finds a residual near the
`E = 0` floor (about 0.3 to 0.4); a failed pause resumes and tries again, `--tries` times, within the same pulse's
window only for the hold. `--damage` instead writes the bomb state (9) and fires it standing. No pushbuffer is
dumped, so a frame costs seconds, not minutes.
"""
import argparse
import importlib.util
import json
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


poll = load("rpcs3_mem_poll", "rpcs3-mem-poll.py")
drive, place = poll.drive, poll.place
from rpcs3_debugger import Debugger  # noqa: E402

GUEST_BASE = 0x300000000
VRAM = 0xC0000000
FUNK_LAYER = 0x00C50EE0
BUFFERS = [(0x02150000, 0xA00 * 360), (0x02300000, 0x500 * 180), (0x022C0000, 0x500 * 180)]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--delay", type=float, action="append", default=[])
    ap.add_argument("--tries", type=int, default=12)
    ap.add_argument("--max-rms", type=float, default=3.0, help="keep a capture whose best fit is under this")
    ap.add_argument("--damage", action="store_true")
    ap.add_argument("--hold", type=float, default=0.0, help="seconds of throttle before the first pulse")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    mem = drive.RemoteMem(drive.read_session()["socket"])
    pad = drive.RemotePad(drive.read_session()["socket"])

    def read(addr, n):
        mem.seek(GUEST_BASE + addr)
        return mem.read(n)

    with Debugger(port=port) as gdb:
        gdb.pause()
        ship, _body = place.find_player(gdb)
        slot = struct.unpack(">I", gdb.read(ship + poll.SLOT_OFFSET, 4))[0]
        gdb.resume()
        if args.hold:
            pad.set("cross", True)
            time.sleep(args.hold)
        try:
            run(args, gdb, pad, read, slot, out)
        finally:
            try:
                gdb.resume()
            except Exception:
                pass
            pad.set("cross", False)


def run(args, gdb, pad, read, slot, out):
    if True:
        for n, delay in enumerate(args.delay):
            state = 9 if args.damage else 4
            for attempt in range(args.tries):
                gdb.pause()
                gdb.write(slot + poll.STATE, struct.pack(">ii", state, state))
                gdb.resume()
                time.sleep(0.5)
                pad.set("triangle", True)
                time.sleep(0.12)
                pad.set("triangle", False)
                t0 = time.time()
                time.sleep(max(0.0, delay - 0.12))
                gdb.pause()
                stem = "d%02d-a%d" % (n, attempt)
                funk = read(FUNK_LAYER, 0x260)
                for off, size in BUFFERS:
                    t1 = time.time()
                    (out / ("%s-vram-%08x.bin" % (stem, off))).write_bytes(gdb.read(VRAM + off, size))
                    print("  %s %#x read in %.1f s" % (stem, off, time.time() - t1), flush=True)
                gdb.resume()
                f = struct.unpack(">%df" % (0x260 // 4), funk)
                info = {"delay": delay, "attempt": attempt, "E": f[0x64 // 4], "P": f[0x58 // 4],
                        "F18": f[0x18 // 4], "F0": f[0], "since_press": round(time.time() - t0, 2)}
                fit = subprocess.run(["uv", "run", "--with", "numpy", "python3", str(HERE / "hd-zoom-history-fit.py"),
                                      str(out), stem, "%.5f" % info["E"]], capture_output=True, text=True).stdout
                info["fit"] = fit.strip().splitlines()
                (out / ("%s.json" % stem)).write_text(json.dumps(info, indent=1))
                print(stem, json.dumps(info), flush=True)
                good = bool(fit) and float(fit.split("rms=")[1].split()[0]) < args.max_rms
                time.sleep(4.0)
                if good:
                    break
        pad.set("cross", False)


if __name__ == "__main__":
    main()
