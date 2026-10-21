#!/usr/bin/env python3
"""Dump HD's ship block and body block each frame under a scripted stick (RPCS3), for an offline
correlation of `ship+0x6a5c` against every float near it. Player input only.
Writes `<out>/blocks.npz`-style raw `blocks.bin` (record = t f64, ship 0x6800..0x6c00, body 0..0x600).
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
SHIP_LO, SHIP_N, BODY_N = 0x6800, 0x400, 0x600


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--stick", default="0:0;3:1;6:0;9:-1;12:0.4;15:0;17:-0.7;19:0")
    ap.add_argument("--seconds", type=float, default=22.0)
    args = ap.parse_args()
    steps = [tuple(float(v) for v in s.split(":")) for s in args.stick.split(";")]
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        gdb = Debugger(port=port)
        gdb.pause()
        ship, body = place.find_player(gdb)
        gdb.resume()
        mem = session.open_mem()
        (out / "meta.txt").write_text("ship %#x body %#x\n" % (ship, body))
        session.pad.set("cross", True)
        t0 = time.time()
        i = 0
        with open(out / "blocks.bin", "wb") as f:
            while (t := time.time() - t0) < args.seconds:
                while i < len(steps) and steps[i][0] <= t:
                    session.pad.stick(steps[i][1], 0.0)
                    i += 1
                mem.seek(GUEST + ship + SHIP_LO)
                a = mem.read(SHIP_N)
                mem.seek(GUEST + body)
                b = mem.read(BODY_N)
                f.write(struct.pack("<d", t) + a + b)
        session.pad.stick(0.0, 0.0)
        session.pad.set("cross", False)
        print("done", flush=True)


if __name__ == "__main__":
    main()
