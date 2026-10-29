#!/usr/bin/env python3
"""Switch Wipeout HD's trail-hit burst off (or back on) by its looked-up name, on an attached RPCS3.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-trail-hit-names.py red|blue|both|restore

`Trail_HitShipEffect` (`0x002d9ec0`) resolves the system by hashing a bare name at call time
(`0x0027f178`), and each name has one TOC slot read only there, so overwriting its first byte makes
the lookup fail and the spawn draws nothing. A data write, so it works under the recompiler;
writing a NOP over the call instead stalls the PPU interpreter on that word. The A/B the
trail-hit page records (`engine-trail.md`).
"""
import importlib.util, sys
from pathlib import Path
REPO = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO))
spec = importlib.util.spec_from_file_location("rpcs3_drive", REPO / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger
RED, BLUE = 0x007A1CC8, 0x007A1CE0
port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
with Debugger(port=port) as gdb:
    gdb.pause()
    sel = {"red": [RED], "blue": [BLUE], "both": [RED, BLUE], "restore": []}[sys.argv[1]]
    for a in (RED, BLUE):
        gdb.write(a, b"X" if a in sel else b"W")
        print(hex(a), gdb.read(a, 24).split(b"\0")[0])
