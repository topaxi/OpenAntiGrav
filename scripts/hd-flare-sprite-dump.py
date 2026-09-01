#!/usr/bin/env python3
"""Dump Wipeout HD's live `EngineFlare` object, hunting for the sprite quad.

The sprite's own draw call is unlocated statically - `EngineFlare_Init`
(`0x002a1528`) loads `Engine_Flare_Rich.gtf` and writes four UV corner pairs,
but neither `EngineFlare_PlaceShapes` (`0x002a1f00`) nor `EngineFlare_Update`
(`0x002a3100`) reads `Flare Radius`'s storage back out, and no other function
in the executable references the texture's string
(`scripts/ps3-toc.py attrib 0x0079bdd8` names only the init). See
`handover/hds-sprite-flare-reads-oversized-and-the-tuning.md`.

This is the live half: dump the whole `EngineFlare` object (`craft + 0x5f70`,
reached the same way `rpcs3-trail-dump.py` reaches it - through the trail
block's own `+0x1204` craft pointer) and scan every 4-byte word in it for an
RSX local-memory address (`0xC0000000..0xD0000000`, the same range the
trail's own SPU-built vertex buffers live in). A hit there is the sprite's
own vertex buffer, addressable and dumpable the same way, and its bytes
settle the true world-space half-size independent of the still-open camera
problem: the corners are offsets from the nozzle, not screen pixels.

Two pauses ~1s apart, so a field that jitters (Max Radius Jitter) reads
differently from one that does not (a static handle).

    uv run --with evdev python3 scripts/hd-flare-sprite-dump.py [out_dir]
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
FLARE_OFFSET = 0x5F70
FLARE_VTABLE = 0x00869D30
FLARE_SIZE = 0x400
RSX_LO, RSX_HI = 0xC0000000, 0xD0000000


def u32(b, off=0):
    return struct.unpack_from(">I", b, off)[0]


def f32(b, off=0):
    return struct.unpack_from(">f", b, off)[0]


def dump(dbg, out, addr, size, path=None):
    data = b""
    while len(data) < size:
        n = min(0x200, size - len(data))
        data += dbg.read(addr + len(data), n)
    if path:
        (out / path).write_bytes(data)
    return data


def flare_snapshot(dbg, out, craft, tag):
    flare = u32(dbg.read(craft + FLARE_OFFSET, 4))
    row = {"craft": "%08x" % craft, "flare": "%08x" % flare}
    if not flare or u32(dbg.read(flare, 4)) != FLARE_VTABLE:
        row["error"] = "no flare or vtable mismatch"
        return row
    blk = dump(dbg, out, flare, FLARE_SIZE, "%s_flare.bin" % tag)
    row["known"] = {
        "intensity_e4": f32(blk, 0xE4),
        "speed_104": f32(blk, 0x104),
        "ramp_108": f32(blk, 0x108),
        "engine_on_10c": blk[0x10C],
        "throttle_s_130": f32(blk, 0x130),
        "boost_timer_12c": f32(blk, 0x12C),
        "blend_144": f32(blk, 0x144),
        "blend_150": f32(blk, 0x150),
        "cache_handle_1a0": "%08x" % u32(blk, 0x1A0),
        "jitter_250": f32(blk, 0x250),
        "jitter_254": f32(blk, 0x254),
        "jitter_25c": f32(blk, 0x25C),
    }
    # The four UV corner pairs Init wrote, as both half-words and the raw hex
    # - so a later session can tell a half-float from a fixed-point guess
    # without another race.
    row["uv_corners_280"] = blk[0x280:0x2B4].hex()
    # Every 4-byte word in RSX local-memory range: candidate vertex/index
    # buffers, or anything else GPU-resident this object holds a handle to.
    rsx_hits = []
    for off in range(0, FLARE_SIZE - 3, 4):
        word = u32(blk, off)
        if RSX_LO <= word < RSX_HI:
            rsx_hits.append({"offset": "0x%x" % off, "addr": "%08x" % word})
    row["rsx_pointer_candidates"] = rsx_hits
    # And every 4-byte word that reads as a plausible world-space float - the
    # tuning file's own numbers (Radius 3.0, Min 2.0, Jitter 0.5, Fadeout
    # 15.0 twice, Size Clamp 50.0) are all inside this band, so a field
    # holding one of those is a candidate storage location even without a
    # traced consumer.
    float_hits = []
    for off in range(0, FLARE_SIZE - 3, 4):
        value = f32(blk, off)
        if 0.05 < abs(value) < 60.0 and value == value:  # NaN-safe
            float_hits.append({"offset": "0x%x" % off, "value": round(value, 4)})
    row["plausible_floats"] = float_hits
    return row


def rsx_probe(dbg, out, tag, addr, size=0x200):
    """Dump a candidate RSX buffer and print it as floats, for a human to
    recognise as six vertices' worth of small position deltas."""
    data = dump(dbg, out, addr, size, "%s_rsx_%08x.bin" % (tag, addr))
    return [round(f32(data, i), 4) for i in range(0, min(size, 0x80), 4)]


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/hd-flare-sprite-dump")
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
        metas = {}
        try:
            dbg.pause()
            manager = u32(dbg.read(MANAGER_SLOT, 4))
            alloc = u32(dbg.read(manager + 0x40, 4))
            craft = u32(dbg.read(alloc + 0x84A0 + 0x1204, 4))
            print("manager %08x alloc %08x craft %08x" % (manager, alloc, craft),
                  flush=True)
            metas["t0"] = flare_snapshot(dbg, out, craft, "t0")
            for hit in metas["t0"].get("rsx_pointer_candidates", []):
                addr = int(hit["addr"], 16)
                metas.setdefault("rsx_t0", {})[hit["addr"]] = rsx_probe(
                    dbg, out, "t0", addr)
            dbg.resume()
            time.sleep(1.0)
            dbg.pause()
            metas["t1"] = flare_snapshot(dbg, out, craft, "t1")
            for hit in metas["t1"].get("rsx_pointer_candidates", []):
                addr = int(hit["addr"], 16)
                metas.setdefault("rsx_t1", {})[hit["addr"]] = rsx_probe(
                    dbg, out, "t1", addr)
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
