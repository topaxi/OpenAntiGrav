#!/usr/bin/env python3
"""Dump Wipeout HD's live engine-trail state out of a running race.

The evidence reproducer for
`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`: boots the HD disc in
RPCS3 on the virtual display, walks the front end into the first Fury race,
drives, then attaches the GDB stub and dumps - for all eight craft - the
`TrailEffectManager` per-trail block (the 54-sample history ring and its
per-frame context), both double-buffered SPU-output vertex buffers, and the
`EngineFlare` timer fields. Two snapshots, so accumulators and the double
buffer show their movement.

    uv run --with evdev python3 scripts/rpcs3-trail-dump.py [out_dir]

Needs the decrypted image (`data/images/hdfury-ps3-eu-dec.iso`), the `oag`
input profile and Xvfb :77 - `scripts/rpcs3-drive.py preflight` checks all
three. One debugger session per emulator launch (rpcs3_debugger.py, trap 2),
which is why this is one script rather than an interactive session.

Addresses, all from the Ghidra corpus (EBOOT.elf, BCES-00664):

- `0x00AED460 + 0x6c`: the manager global - `Trail_ConstructManager` stores
  `this` there; `this + 0x40` is the 0x11b00 allocation.
- alloc `+ 0x84a0 + slot * 0x1230`: one craft's trail block. Bytes 0..0x10e0
  are the ring (54 x 0x50: three basis rows, position, colour); the tail is
  the per-frame context and header (craft pointer at +0x1204, RSX vertex
  buffers at +0x1214/+0x1218, brightness at +0x11d8, u-head at +0x11ec,
  scroll phase at +0x1210).
- The RSX buffers read back through the stub at their `0xC...` addresses:
  0x2d90 bytes = 324 vertices of 36 (position f32x3, normal f32x3, Uv1
  f32x2, VertexColour1 u8x4 - the manager's own attribute table at
  alloc+0x11a00).
- `craft + 0x5f70`: the `EngineFlare`; `+0x12c` its boost timer, `+0x144`
  and `+0x150` its two boost blends. `craft + 0x7d2c`: the Fury-skin flag.
"""

import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location(
    "rpcs3_drive", ROOT / "scripts" / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger  # noqa: E402

IMAGE = ROOT / "data/images/hdfury-ps3-eu-dec.iso"
MANAGER_SLOT = 0x00AED460 + 0x6C
FLARE_VTABLE = 0x00869D30
FLARE_OFFSET = 0x5F70


def u32(b, off=0):
    return struct.unpack_from(">I", b, off)[0]


def dump(dbg, out, addr, size, path=None):
    data = b""
    while len(data) < size:
        n = min(0x200, size - len(data))
        data += dbg.read(addr + len(data), n)
    if path:
        (out / path).write_bytes(data)
    return data


def snapshot(dbg, out, alloc, tag):
    meta = {"wall": time.time(), "trails": []}
    flares = []
    for i in range(8):
        base = alloc + 0x84A0 + i * 0x1230
        blk = dump(dbg, out, base, 0x1230, "%s_trail%d.bin" % (tag, i))
        craft = u32(blk, 0x1204)
        meta["trails"].append({
            "craft": "%08x" % craft,
            "vA": "%08x" % u32(blk, 0x1214),
            "vB": "%08x" % u32(blk, 0x1218),
        })
        dump(dbg, out, u32(blk, 0x1214), 0x2D90, "%s_vtx%dA.bin" % (tag, i))
        dump(dbg, out, u32(blk, 0x1218), 0x2D90, "%s_vtx%dB.bin" % (tag, i))
        flare = u32(dbg.read(craft + FLARE_OFFSET, 4))
        row = {"flare": "%08x" % flare,
               "fury": dbg.read(craft + 0x7D2C, 1)[0]}
        if flare and u32(dbg.read(flare, 4)) == FLARE_VTABLE:
            fd = dbg.read(flare + 0x120, 0x40)
            row.update({
                "timer_12c": struct.unpack_from(">f", fd, 0x0C)[0],
                "blend_144": struct.unpack_from(">f", fd, 0x24)[0],
                "blend_150": struct.unpack_from(">f", fd, 0x30)[0],
            })
        flares.append(row)
    (out / ("%s_flares.json" % tag)).write_text(json.dumps(flares, indent=1))
    return meta


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/hd-trail-dump")
    out.mkdir(parents=True, exist_ok=True)
    with drive.Session(str(IMAGE), str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", 180.0):
            print("never reached the Main Menu", file=sys.stderr)
            return 1
        time.sleep(12.0)
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            print("did not reach a race", file=sys.stderr)
            return 1
        print("in race; waiting for the load", flush=True)
        time.sleep(50.0)
        session.pad.set("cross", True)
        time.sleep(16.0)

        dbg = Debugger()
        metas = []
        try:
            dbg.pause()
            manager = u32(dbg.read(MANAGER_SLOT, 4))
            alloc = u32(dbg.read(manager + 0x40, 4))
            print("manager %08x alloc %08x" % (manager, alloc), flush=True)
            metas.append(snapshot(dbg, out, alloc, "s0"))
            dbg.resume()
            time.sleep(1.2)
            dbg.pause()
            metas.append(snapshot(dbg, out, alloc, "s1"))
            dbg.resume()
        finally:
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
        (out / "meta.json").write_text(json.dumps(metas, indent=1))
        session.pad.set("cross", False)
        drive.screenshot(out / "race.png")
        print("done; artefacts in %s" % out, flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
