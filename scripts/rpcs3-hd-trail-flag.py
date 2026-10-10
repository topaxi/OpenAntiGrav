#!/usr/bin/env python3
"""Read, and optionally overwrite, the player craft's Fury-skin byte (`craft + 0x7d2c`).

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-trail-flag.py [--set 0|1]

`Trail_BuildDrawState` reads that byte every frame to patch the `engineTrail`
colour-mix vec4 (0.0 blue, 1.0 red), so writing it on a running race flips the
player's trail between the two variants with the hull unchanged. Also prints the
model-variant name at `*(craft+0x6298)+0x78`. One GDB session per emulator launch:
this connects, pauses, acts, resumes and disconnects, so run it once.
"""
import argparse
import importlib.util
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", type=int, choices=[0, 1])
    a = ap.parse_args()
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with Debugger(port=port) as gdb:
        gdb.pause()
        try:
            ship, _body = place.find_player(gdb)
            before = gdb.read(ship + 0x7D2C, 1)[0]
            p = struct.unpack(">I", gdb.read(ship + 0x6298, 4))[0]
            q = struct.unpack(">I", gdb.read(p + 0x78, 4))[0]
            name = gdb.read(q, 16).split(b"\0")[0].decode("latin1")
            if a.set is not None:
                gdb.write(ship + 0x7D2C, bytes([a.set]))
            after = gdb.read(ship + 0x7D2C, 1)[0]
            print("craft %#x variant %r flag %d -> %d" % (ship, name, before, after))
        finally:
            gdb.resume()


if __name__ == "__main__":
    sys.exit(main())
