#!/usr/bin/env python3
"""Smoke test for an attached RPCS3 that was restored from a save state.

Checks the three things a state must give back (the pad is checked by pressing
a button yourself): the GDB stub answers through the proxy, `/proc/<pid>/mem`
reads guest memory, and the player's craft can be found. Prints one line each
and exits non-zero on the first failure. Needs a race running (a state taken
in one, after START RACE).

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-state-probe.py
"""
import importlib.util
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

GUEST_BASE = 0x300000000


def main():
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with Debugger(port=port) as gdb:
        gdb.pause()
        try:
            ship, body = place.find_player(gdb)
            via_gdb = gdb.read(ship, 16)
        finally:
            gdb.resume()
    print("gdb: player craft %#x body %#x, first 16 bytes %s" % (ship, body, via_gdb.hex()))
    mem = drive.RemoteMem(drive.read_session()["socket"])
    mem.seek(GUEST_BASE + ship)
    via_mem = mem.read(16)
    print("mem: same 16 bytes through /proc mem: %s (%s)"
          % (via_mem.hex(), "match" if via_mem == via_gdb else "differs while running"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
