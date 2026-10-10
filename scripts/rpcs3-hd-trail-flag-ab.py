#!/usr/bin/env python3
"""Does the SPU's vertex colour ramp depend on the Fury-skin byte (`craft + 0x7d2c`)?

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-trail-flag-ab.py OUT.json

Attached to a running race. Reads every trail's SPU output buffer (324 vertices of 36
bytes, `VertexColour1` u8x4 at +0x20) through the emulator's memory (RSX local memory
is readable there, no debugger needed), then - one GDB session - overwrites the byte
to 0 on all eight craft, resumes, waits, and reads the buffers again. Unchanged colours
mean the ramp is universal and the flag only drives the fragment-side mix.
"""
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger  # noqa: E402

BASE = 0x300000000
MANAGER_SLOT = 0x00AED460 + 0x6C
mem = drive.RemoteMem(drive.read_session()["socket"])


def rd(a, n):
    mem.seek(BASE + a)
    return mem.read(n)


def u32(a):
    return struct.unpack(">I", rd(a, 4))[0]


def snapshot():
    alloc = u32(u32(MANAGER_SLOT) + 0x40)
    out = []
    for i in range(8):
        b = alloc + 0x84A0 + i * 0x1230
        craft = u32(b + 0x1204)
        flag = rd(craft + 0x7D2C, 1)[0] if craft else None
        buf = u32(b + 0x1214)
        data = rd(buf, 0x2D90)
        ring = [list(data[v * 36 + 0x20: v * 36 + 0x24]) for v in range(0, 54)]
        out.append({"slot": i, "craft": craft, "flag": flag, "buf": buf,
                    "alpha_scale": struct.unpack(">f", rd(b + 0x11D8, 4))[0],
                    "colour_fin0": ring})
    return alloc, out


def main():
    alloc, before = snapshot()
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with Debugger(port=port) as gdb:
        gdb.pause()
        try:
            for t in before:
                if t["craft"]:
                    gdb.write(t["craft"] + 0x7D2C, bytes([0]))
        finally:
            gdb.resume()
    time.sleep(1.0)
    _, after = snapshot()
    res = {"before": before, "after": after}
    Path(sys.argv[1]).write_text(json.dumps(res))
    for b, a in zip(before, after):
        same = b["colour_fin0"] == a["colour_fin0"]
        print("slot %d flag %s -> %s  speed/bright %.3f  colours identical: %s"
              % (b["slot"], b["flag"], a["flag"], a["alpha_scale"], same))
        print("   ring k=0,6,13,27,40,53 after:", [a["colour_fin0"][k] for k in (0, 6, 13, 27, 40, 53)])


if __name__ == "__main__":
    sys.exit(main())
