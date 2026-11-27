#!/usr/bin/env python3
"""HD's per-craft view flags `ship + 0x5f42` and `ship + 0x5f43`, read live while the view changes, on an attached RPCS3.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-camera-flags.py --image ISO \\
        [--press select --presses 4 | --hold l3,r3] [--settle 1.5]

In a race (restore any HD race state, `emu-restore-state.sh`), it reads the byte pair for all eight craft, taps
`--press` `--presses` times with `--settle` seconds between, and reads again after each tap. The player's craft
is the one whose role word (`ship + 0x7a60`) is 0. Reads go through `/proc/<pid>/mem`, never pausing the game.
Documented on docs/ghidra/functions/ps3-hdfury-eu/particle-triggers.md ("The camera-mode flags").
"""
import argparse
import importlib.util
import struct
import sys
import time
from pathlib import Path
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import oag_tmp  # noqa: E402

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_hd_field  # noqa: E402
import rpcs3_place as place  # noqa: E402


def read_flags(mem):
    rows = []
    for ship in struct.unpack(">8I", mem.rd(place.CRAFT_ARRAY, 32)):
        if not ship:
            continue
        role = struct.unpack(">I", mem.rd(ship + place.OFF_ROLE, 4))[0]
        flags = mem.rd(ship + 0x5F40, 6)
        rows.append(("player" if role == 0 else "rival ", ship, flags.hex()))
    return rows


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--log-dir", default=oag_tmp.output("camera-flags"))
    ap.add_argument("--press", default="select")
    ap.add_argument("--presses", type=int, default=4)
    ap.add_argument("--settle", type=float, default=1.5)
    ap.add_argument("--hold", default="", help="comma separated buttons, each held for --settle seconds and read while held")
    args = ap.parse_args()
    with drive.Session(args.image, args.log_dir) as session:
        mem = rpcs3_hd_field.Mem(session.open_mem())

        def show(label):
            print(label, flush=True)
            for who, ship, flags in read_flags(mem):
                print("  %s %08x  +0x5f40..45 = %s" % (who, ship, flags), flush=True)

        show("before")
        for name in [b for b in args.hold.split(",") if b]:
            session.pad.set(name, True)
            time.sleep(args.settle)
            show("holding %s" % name)
            session.pad.set(name, False)
            time.sleep(args.settle)
            show("released %s" % name)
        for n in range(args.presses if not args.hold else 0):
            session.pad.press(args.press)
            time.sleep(args.settle)
            show("after %s %d" % (args.press, n + 1))


if __name__ == "__main__":
    main()
