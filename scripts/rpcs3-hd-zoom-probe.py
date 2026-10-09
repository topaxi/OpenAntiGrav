#!/usr/bin/env python3
"""Sample HD's zoom-ring inputs from an attached RPCS3 without pausing it.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-zoom-probe.py [--fire turbo] [--seconds 3]

Reads, through `/proc/<pid>/mem`, the `FunkLayer` object (`0x00c50ee0`: `E` at `+0x64`, `P` at `+0x58`, the
per-viewport weight scale at `+0x00`, the history crop at `+0x18`, `+0x250`) and the environment block's `+0x54c`
(the global pointer is `0x008b6fb4`), and prints them every 50 ms together with the accumulation weight
`min(0.95, 10 * E * F[0] + 10 * F[0x250] + Env[0x54c])` the post-chain runner builds. `--fire turbo` writes the
Turbo state into the pickup slot and presses triangle first (the recipe of `rpcs3-hd-postchain.py`).
"""
import argparse
import importlib.util
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)

GUEST_BASE = 0x300000000


def drive_load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod
FUNK_LAYER = 0x00C50EE0
ENV_PTR = 0x008B6FB4
GLOW_STATE = 0x00AD7880


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--fire", choices=["turbo"], default=None)
    ap.add_argument("--seconds", type=float, default=3.0)
    ap.add_argument("--shots", default=None, help="with --onsets: a directory for a screenshot 0.25 s after each onset")
    ap.add_argument("--onsets", action="store_true",
                    help="print only a pulse's start (the glow state `0x00ad7880` jumping up) with the speed read at the craft")
    args = ap.parse_args()
    mem = drive.RemoteMem(drive.read_session()["socket"])

    def read(addr, n):
        mem.seek(GUEST_BASE + addr)
        return mem.read(n)

    if args.fire:
        poll = drive_load("rpcs3_mem_poll", "rpcs3-mem-poll.py")
        place = poll.place
        from rpcs3_debugger import Debugger
        port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
        with Debugger(port=port) as gdb:
            gdb.pause()
            ship, _body = place.find_player(gdb)
            slot = struct.unpack(">I", gdb.read(ship + poll.SLOT_OFFSET, 4))[0]
            gdb.write(slot + poll.STATE, struct.pack(">ii", 4, 4))
            gdb.resume()
        time.sleep(0.5)
        pad = drive.RemotePad(drive.read_session()["socket"])
        pad.set("triangle", True)
        time.sleep(0.12)
        pad.set("triangle", False)
    env = struct.unpack(">I", read(ENV_PTR, 4))[0]
    if args.onsets:
        last = 0.0
        t0 = time.time()
        while time.time() - t0 < args.seconds:
            glow = struct.unpack(">4f", read(GLOW_STATE, 16))
            if glow[0] > last + 0.3:
                print("onset t=%.2f glow=%s" % (time.time() - t0, [round(x, 3) for x in glow]), flush=True)
                if args.shots:
                    time.sleep(0.25)
                    n = len(list(Path(args.shots).glob("onset*.png")))
                    drive.screenshot(Path(args.shots) / ("onset%d.png" % n), trim=True)
            last = glow[0]
            time.sleep(0.01)
        return
    t0 = time.time()
    while time.time() - t0 < args.seconds:
        f = struct.unpack(">%df" % (0x260 // 4), read(FUNK_LAYER, 0x260))
        e54c = struct.unpack(">f", read(env + 0x54C, 4))[0]
        e, p = f[0x64 // 4], f[0x58 // 4]
        w = min(0.95, 10 * e * f[0] + 10 * f[0x250 // 4] + e54c)
        mesh = ""
        obj = struct.unpack(">I", read(FUNK_LAYER + 0x130, 4))[0]
        if obj:
            data = struct.unpack(">I", read(obj + 0x10, 4))[0]
            rec = struct.unpack(">40f", read(data, 0xA0))
            mesh = " mesh0 " + " ".join("v%d=(%.3f,%.3f,%.3f,%.3f)" % (v, *rec[10 * v + 6:10 * v + 10]) for v in range(4))
        print("t=%.2f E=%.4f P=%.4f F0=%.4f F18=%.5f F250=%.5f env54c=%.5f weight=%.4f%s"
              % (time.time() - t0, e, p, f[0], f[0x18 // 4], f[0x250 // 4], e54c, w, mesh), flush=True)
        time.sleep(0.05)


if __name__ == "__main__":
    main()
