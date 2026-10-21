#!/usr/bin/env python3
"""Find HD's two swaying HUD containers in RAM and log their offsets (RPCS3, player input).

`Hud_Update` (0x0009e710-0x0009e754) stores `-15 * ship+0x6a58` at `+0x40` and `ship+0x6a5c * 15`
at `+0x44` of the nodes `*(hud+0x4e4)` ("HUD") and `*(hud+0x4e8)` ("InGame"). Holds cross and the
stick for a few seconds so the drawn lean's follower settles away from zero, then scans guest
memory for big-endian floats `-15 * follower` at `+0x40`, and prints each hit with the
follower. Attach to a running `rpcs3-drive.py serve` in a race (`OAG_RPCS3_ATTACH=1`).
docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md, "The HUD sway".
"""
import argparse
import importlib.util
import struct
import sys
import time
from pathlib import Path


REPO = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", REPO / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

GUEST = 0x300000000
K = 15.0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--stick", type=float, default=1.0)
    ap.add_argument("--hold", type=float, default=4.0)
    ap.add_argument("--lo", type=lambda s: int(s, 0), default=0x10000000)
    ap.add_argument("--hi", type=lambda s: int(s, 0), default=0x80000000)
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        gdb = Debugger(port=port)
        gdb.pause()
        ship, _ = place.find_player(gdb)
        gdb.resume()
        mem = session.open_mem()
        session.pad.set("cross", True)
        session.pad.stick(args.stick, 0.0)
        time.sleep(args.hold)

        def f32(addr):
            mem.seek(GUEST + addr)
            return struct.unpack(">f", mem.read(4))[0]

        follower = f32(ship + 0x6A58)
        want = -K * follower
        print("follower %.5f lean %.5f want +0x40 = %.4f" % (follower, f32(ship + 0x6A54), want), flush=True)
        # The node's +0x40 float is -15 * follower exactly (fnmsubs against zero); +0x44 is 15 * ship+0x6a5c.
        pattern = struct.pack(">f", want)
        hits = []
        chunk = 0x1000000
        for base in range(args.lo, args.hi, chunk):
            mem.seek(GUEST + base)
            try:
                raw = mem.read(chunk)
            except OSError:
                continue
            start = 0
            while (i := raw.find(pattern, start)) >= 0:
                if i % 4 == 0 and i >= 0x40:
                    hits.append(base + i - 0x40)
                start = i + 1
        session.pad.stick(0.0, 0.0)
        session.pad.set("cross", False)
        sway_y = K * f32(ship + 0x6A5C)
        print("ship+0x6a5c %.5f -> +0x44 expected %.4f" % (f32(ship + 0x6A5C), sway_y), flush=True)
        for addr in hits:
            if abs(f32(addr + 0x44) - sway_y) > 0.05:
                continue
            print("node %#x  +0x40 %.4f +0x44 %.4f" % (addr, f32(addr + 0x40), f32(addr + 0x44)))
        print("hits", len(hits), "(kept those whose +0x44 matches)", flush=True)


if __name__ == "__main__":
    main()
