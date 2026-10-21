#!/usr/bin/env python3
"""Poll HD's sway inputs (`ship+0x6a54/58/5c` and neighbours) under a scripted player stick (RPCS3).

Attach to a running `rpcs3-drive.py serve` sitting in a race (`OAG_RPCS3_ATTACH=1`). Holds cross,
plays `--stick` steps (`seconds:x[:y]`, left stick -1..1), and logs one CSV row per read of the
quad `ship+0x6a50..+0x6a5f` plus the cockpit pair `+0x6a44/+0x6a48`. Player input only.
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
OFFS = (0x6A44, 0x6A48, 0x6A50, 0x6A54, 0x6A58, 0x6A5C, 0x6A60)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--stick", default="0:0;3:1;7:0;10:-1;14:0;16:0.5;19:0")
    ap.add_argument("--seconds", type=float, default=20.0)
    ap.add_argument("--no-cross", action="store_true")
    args = ap.parse_args()
    steps = []
    for item in args.stick.split(";"):
        p = [float(v) for v in item.split(":")]
        steps.append((p[0], p[1], p[2] if len(p) > 2 else 0.0))
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        gdb = Debugger(port=port)
        gdb.pause()
        ship, _ = place.find_player(gdb)
        gdb.resume()
        print("ship %#x" % ship, flush=True)
        mem = session.open_mem()

        def f32(addr):
            mem.seek(GUEST + addr)
            return struct.unpack(">f", mem.read(4))[0]

        if not args.no_cross:
            session.pad.set("cross", True)
        t0 = time.time()
        i = 0
        rows = ["t,sx,sy," + ",".join("%x" % o for o in OFFS)]
        sx = sy = 0.0
        while (t := time.time() - t0) < args.seconds:
            while i < len(steps) and steps[i][0] <= t:
                sx, sy = steps[i][1], steps[i][2]
                session.pad.stick(sx, sy)
                i += 1
            rows.append("%.3f,%.2f,%.2f," % (t, sx, sy) + ",".join("%.5f" % f32(ship + o) for o in OFFS))
        session.pad.stick(0.0, 0.0)
        session.pad.set("cross", False)
        (out / "poll.csv").write_text("\n".join(rows) + "\n")
        print("rows", len(rows) - 1, flush=True)


if __name__ == "__main__":
    main()
