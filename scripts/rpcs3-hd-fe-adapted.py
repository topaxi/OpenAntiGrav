#!/usr/bin/env python3
"""Read the front end's adapted luma and light block on an attached RPCS3, without pausing it.

    OAG_COMPARE_PLATFORM=hd-fury  (when the serve is started: the 1920x1080 scale-100 profile)
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-fe-adapted.py [reads] [seconds]

Prints, every `seconds` (default 4), `FunkLayer + 0x25c` (the PPU's adapted luma, object `0x00c50ee0`) and
`+0x240` (the raw last ladder mean), the tone constants of the global block `0x00c489f0` (`+0x540` boost,
`+0x544` clamp, `+0x548` maximum, `+0x52c` adaption rate) and the front end's own block at `*(0x009948ec) + 0x60`
(ambient `+0x420`, sun `+0x430`, sun direction `+0x450`, fog `+0x4e0`, fog density `+0x500`). Stand on the screen
first (Ship Select: `right cross right cross cross` from Main Menu). The exposure is
`maximum - min(adapted * boost, clamp)`. `docs/ghidra/functions/ps3-hdfury-eu/front-end-model-lighting.md`,
"The live read".
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

GUEST_BASE = 0x300000000
FUNK_LAYER = 0x00C50EE0
GLOBAL_BLOCK = 0x00C489F0
FE_BLOCK_PTR = 0x009948EC


def main():
    reads = int(sys.argv[1]) if len(sys.argv) > 1 else 6
    seconds = float(sys.argv[2]) if len(sys.argv) > 2 else 4.0
    mem = drive.RemoteMem(drive.read_session()["socket"])

    def read(addr, n):
        mem.seek(GUEST_BASE + addr)
        return mem.read(n)

    def floats(addr, n):
        return [round(x, 4) for x in struct.unpack(">%df" % n, read(addr, 4 * n))]

    for i in range(reads):
        adapted, raw = floats(FUNK_LAYER + 0x25C, 1)[0], floats(FUNK_LAYER + 0x240, 1)[0]
        tone = [floats(GLOBAL_BLOCK + off, 1)[0] for off in (0x540, 0x544, 0x548, 0x52C)]
        print(i, "adapted(+0x25c)=%.6f raw(+0x240)=%.6f boost/clamp/max/rate=%s" % (adapted, raw, tone), flush=True)
        fe = struct.unpack(">I", read(FE_BLOCK_PTR, 4))[0]
        if fe and i == 0:
            block = fe + 0x60
            print("   fe block %#x: ambient %s sun %s dir %s fog %s density %s" % (
                block, floats(block + 0x420, 3), floats(block + 0x430, 4), floats(block + 0x450, 3),
                floats(block + 0x4E0, 3), floats(block + 0x500, 1)), flush=True)
        time.sleep(seconds)


if __name__ == "__main__":
    main()
