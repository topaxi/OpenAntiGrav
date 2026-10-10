#!/usr/bin/env python3
"""Dump every trail's full 324-vertex colour/alpha table (read-only, no debugger).

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-trail-colours.py OUT.json [--throttle SECONDS]

Holds throttle first when asked, so the brightness field is non-zero and the alpha ramp
shows. Output per slot: craft flag, brightness (+0x11d8), and `VertexColour1` rgba for
each of the 324 vertices of the buffer the SPU last wrote.
"""
import argparse
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

BASE = 0x300000000
MANAGER_SLOT = 0x00AED460 + 0x6C
mem = drive.RemoteMem(drive.read_session()["socket"])


def rd(a, n):
    mem.seek(BASE + a)
    return mem.read(n)


def u32(a):
    return struct.unpack(">I", rd(a, 4))[0]


ap = argparse.ArgumentParser()
ap.add_argument("out")
ap.add_argument("--throttle", type=float, default=0.0)
a = ap.parse_args()
pad = drive.RemotePad(drive.read_session()["socket"])
if a.throttle:
    pad.set("cross", True)
    time.sleep(a.throttle)
alloc = u32(u32(MANAGER_SLOT) + 0x40)
res = []
for i in range(8):
    b = alloc + 0x84A0 + i * 0x1230
    craft = u32(b + 0x1204)
    flag = rd(craft + 0x7D2C, 1)[0] if craft else None
    data = rd(u32(b + 0x1214), 0x2D90)
    res.append({"slot": i, "flag": flag, "brightness": struct.unpack(">f", rd(b + 0x11D8, 4))[0],
                "u_head": struct.unpack(">f", rd(b + 0x11EC, 4))[0],
                "rgba": [list(data[v * 36 + 0x20: v * 36 + 0x24]) for v in range(324)]})
pad.set("cross", False)
Path(a.out).write_text(json.dumps(res))
t = res[0]
print("slot0 flag", t["flag"], "brightness %.3f" % t["brightness"])
for v in list(range(0, 12)) + [53, 54, 55, 107, 108, 161, 162, 323]:
    print(" v%d" % v, t["rgba"][v])
