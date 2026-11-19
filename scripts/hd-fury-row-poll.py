#!/usr/bin/env python3
"""Which `<ScreenSetting>` row the Fury menu backdrop is using, read live.

Attach to a long-lived `rpcs3-drive.py serve` on the Fury disc
(`OAG_RPCS3_ATTACH=1`). Finds the widget's row table by scanning guest memory
for the 0x60-byte row named `default` followed by `Main Menu`, then the item
that points at it (`hd_fury_backdrop.Widget`; `item+0xb8` is the table's start, `+0xc4` the current row,
`+0xc8` the `default` row, `+0x1648..+0x1654` the `OnEnable` pulse's frame,
frames, seconds and rate), and prints the screen, the current row's name and tint
and the pulse block every 0.1 s. Rows hold the tint's four floats at `+0x40`.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/hd-fury-row-poll.py [seconds]

2026-10-10: `Team Selection`, `Single Player`, `Track Creation` and the race
`HUD` are on `default` (0.12549); `Main Menu` and `Additional` are white; the
pulse block restarts on every screen change and not on a team step. See
docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md.
"""
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
import hd_fury_backdrop as fb  # noqa: E402

GUEST_BASE = 0x300000000
ROW = 0x60


def main():
    seconds = float(sys.argv[1]) if len(sys.argv) > 1 else 5.0
    mem = drive.RemoteMem(drive.read_session()["socket"])

    def read(address, size):
        mem.seek(GUEST_BASE + address)
        return mem.read(size)

    def name(row):
        return row[:0x40].split(b"\0")[0].decode(errors="replace")

    try:
        widget = fb.Widget(mem)
    except LookupError as error:
        sys.exit(str(error))
    table, item = widget.table, widget.item
    print("rows %#x item %#x" % (table, item), flush=True)
    start = time.time()
    while time.time() - start < seconds:
        current = struct.unpack(">I", read(item + 0xC4, 4))[0]
        row = read(current, ROW)
        tint = struct.unpack(">4f", row[0x40:0x50])
        pulse = struct.unpack(">IIfI", read(item + 0x1648, 16))
        print("%6.2f %s row=%s tint=%.4f pulse(frame, frames, seconds, rate)=%s"
              % (time.time() - start, drive.current_screen(), name(row), tint[0], pulse),
              flush=True)
        time.sleep(0.1)


if __name__ == "__main__":
    main()
