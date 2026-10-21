#!/usr/bin/env python3
"""Which way does Wipeout HD draw its steering lean? (RPCS3, attached to a `serve` in a race.)

Holds cross and the stick (`--stick`) for `--hold` seconds, pauses, and reads the drawn lean
`ship+0x6a54`, the barrel roll's eased phase `ship+0x6a90`, the body's rows (`body+0x1d0`,
`+0x1e0`, `+0x1f0`) and the hull node's matrix, `*(ship+0x6adc) + 0x80` (`0x00323780` is that
accessor; `Ship_UpdateEntity` stores the composed rows there at `0x000ebc60`-`0x000ebc6c`).
Prints the node's rows expressed in the body's basis, so the rotation and its sign read off
directly against `angle = lean * 0.5 + roll * 6.28`
(`docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md`).

    OAG_RPCS3_ATTACH=1 python3 scripts/rpcs3-hd-lean-display.py --image <iso> --out <dir>
"""
import argparse
import importlib.util
import math
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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--stick", type=float, action="append", default=[])
    ap.add_argument("--hold", type=float, default=3.0)
    args = ap.parse_args()
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(Path(args.out) / "logs")) as session:
        gdb = Debugger(port=port)
        gdb.pause()
        ship, body = place.find_player(gdb)
        gdb.resume()
        for stick in args.stick or [1.0, -1.0]:
            session.pad.set("cross", True)
            session.pad.stick(stick, 0.0)
            time.sleep(args.hold)
            gdb.pause()
            f = lambda a, n: struct.unpack(">%df" % n, gdb.read(a, 4 * n))  # noqa: E731
            lean, roll = f(ship + 0x6A54, 1)[0], f(ship + 0x6A90, 1)[0]
            node = struct.unpack(">I", gdb.read(ship + 0x6ADC, 4))[0] + 0x80
            b = [f(body + 0x1D0 + 0x10 * i, 3) for i in range(3)]
            n = [f(node + 0x10 * i, 3) for i in range(3)]
            gdb.resume()
            angle = lean * 0.5 + roll * 6.28
            print("stick %+.1f lean %+.4f roll %+.4f angle %+.4f (cos %.4f sin %.4f)"
                  % (stick, lean, roll, angle, math.cos(angle), math.sin(angle)), flush=True)
            for i, row in enumerate(n):
                coords = [sum(row[k] * b[j][k] for k in range(3)) for j in range(3)]
                print("  node row %d in body basis: %s" % (i, ["%+.4f" % c for c in coords]),
                      flush=True)
            print("  body rows %s" % [["%+.4f" % v for v in r] for r in b], flush=True)
            print("  node rows %s" % [["%+.4f" % v for v in r] for r in n], flush=True)
        session.pad.stick(0.0, 0.0)
        session.pad.set("cross", False)


if __name__ == "__main__":
    main()
