#!/usr/bin/env python3
"""Dump the ship-effects tuning block `EngineFlare_RenderTick` reads live,
plus the flare object's own fade-scale storage.

Third session's static read of `EngineFlare_RenderTick` (`0x002a08a8`, see
`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`, "Third session") found
a `saturate()`-based distance fade that reads twelve floats plus one gate byte
out of the same shared global `Trail_RenderTick` also reads (`0x00AEA540`),
multiplies the fade by a per-instance value at `this+0x194` (or a `1.0`
default), clamps the result against one of those global floats, and stores it
at `this+0x18c` right before the function's early-out gate.

**`0x00AEA540` is a pointer *variable*, not the struct base** -
`EngineFlare_RenderTick`'s own disassembly dereferences it twice (`lwz
r9,0x5a48(r2)` resolves the TOC slot to this address; `lwz r9,0x0(r9)` then
loads the pointer it holds, a heap allocation whose address moves between
runs). A first version of this script skipped the second load and read every
offset as `0.0`; fixed, `struct_base + 0x460`..`+0x4b4` matches
`Data/ships/shipeffectstweaks.txt`'s remaining unread rows almost one for
one, live and exact: `Flare Highlight Power` (32.0, `+0x470`), `Flare
Fadeout Dist`/`Range` (15.0/15.0, `+0x484`/`+0x488`), `Spikes Thrust/Boost
Max Scale` (1.5/2.0, `+0x490`/`+0x480`), `Shockwave Cycle Speed` (0.01,
`+0x46c`), `Engine Flare Particles Min Alpha` (0.25, `+0x464`), `Flare Size
Clamp` (50.0, `+0x4b0`) and `Flare Depth Bias` (-0.46, `+0x4b4`) - see
"Fourth session" on engine-trail.md for the full table and what is still
ambiguous (three rows share the value `1.0`).

Then reads `this+0x18c`/`this+0x194` for one craft across a few frames, so a
changing `0x18c` and a static `0x194` would tell the fade output from a base
value - what it actually found (`0x18c` reading `0.0` on every sample of a
visibly-thrusting, on-screen-flared craft) is the open question the next
session inherits: whether `EngineFlare_RenderTick` really executes for that
craft this build, or the visible sprite draws through a branch this script's
static reading did not trace.

    uv run --with evdev python3 scripts/hd-flare-tuning-dump.py [out_dir]
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
# The pointer variable `EngineFlare_RenderTick` and `Trail_RenderTick` both
# dereference, resolved from `PTR_DAT_008b2f20`/`PTR_DAT_008b44d0` (two
# different TOC slots, same pointer value, read directly with the ghidra-mcp
# tool in the static pass). What it points at is a heap allocation, so
# `struct_base` below is read live rather than hardcoded. Dumped wide
# (struct_base+0x420..0x4d0) to catch neighbouring rows the static read did
# not name a consumer for yet.
GLOBAL_BLOCK = 0x00AEA540
GLOBAL_DUMP_LO = 0x420
GLOBAL_DUMP_HI = 0x4d0

# The tuning file's twelve still-unread rows past what earlier sessions
# matched (order from `Data/ships/shipeffectstweaks.txt`, "All 41 rows" on
# engine-trail.md), so a printed float can be eyeballed against this list
# without cross-referencing the doc mid-run.
UNREAD_ROWS = [
    ("Spikes Thrust Max Scale", 1.5),
    ("Spikes Boost Max Scale", 2.0),
    ("Engine Flare Particles Min Alpha", 0.25),
    ("Enable Engine Flare Particles", 1.0),
    ("Shockwave Cycle Speed", 0.01),
    ("Flare Highlight Power", 32.0),
    ("Flare Highlight Boost", 1.0),
    ("Flare Fadeout Dist", 15.0),
    ("Flare Fadeout Range", 15.0),
    ("Flare Occluder Radius", 1.0),
    ("Flare Size Clamp", 50.0),
    ("Flare Depth Bias", -0.46),
]


def u32(b, off=0):
    return struct.unpack_from(">I", b, off)[0]


def f32(b, off=0):
    return struct.unpack_from(">f", b, off)[0]


def dump(dbg, addr, size):
    data = b""
    while len(data) < size:
        n = min(0x200, size - len(data))
        data += dbg.read(addr + len(data), n)
    return data


def annotate(value):
    hits = [name for name, expect in UNREAD_ROWS if abs(value - expect) < 1e-3]
    return " <- %s" % ", ".join(hits) if hits else ""


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/hd-flare-tuning-dump")
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
        result = {}
        try:
            dbg.pause()
            manager = u32(dbg.read(MANAGER_SLOT, 4))
            alloc = u32(dbg.read(manager + 0x40, 4))
            craft = u32(dbg.read(alloc + 0x84A0 + 0x1204, 4))
            flare = u32(dbg.read(craft + FLARE_OFFSET, 4))
            print("manager %08x alloc %08x craft %08x flare %08x" %
                  (manager, alloc, craft, flare), flush=True)
            result["craft"] = "%08x" % craft
            result["flare"] = "%08x" % flare
            if not flare or u32(dbg.read(flare, 4)) != FLARE_VTABLE:
                print("no flare object / vtable mismatch", file=sys.stderr)
                return 1

            # 0x00AEA540 is a pointer *variable*, not the struct base -
            # EngineFlare_RenderTick's own disassembly dereferences it twice
            # (`lwz r9,0x5a48(r2)` resolves the TOC slot to this address;
            # `lwz r9,0x0(r9)` then loads the pointer it holds). Missing this
            # second load was the first run's bug - every offset read as 0.0.
            struct_base = u32(dbg.read(GLOBAL_BLOCK, 4))
            result["global_struct_base"] = "%08x" % struct_base
            gate_byte = dbg.read(struct_base + 0x494, 1)[0]
            result["gate_byte_0x494"] = gate_byte
            # EngineFlare_RenderTick's other gate: ble if
            # *(craft+0x5fa4)-4 <= 2, i.e. skips unless this field is >= 7.
            count_field = u32(dbg.read(craft + 0x5fa4, 4))
            result["count_field_craft+0x5fa4"] = count_field
            block = dump(dbg, struct_base + GLOBAL_DUMP_LO,
                         GLOBAL_DUMP_HI - GLOBAL_DUMP_LO)
            (out / "global_block.bin").write_bytes(block)
            floats = []
            for off in range(0, len(block) - 3, 4):
                value = f32(block, off)
                if value == value and abs(value) < 1e6:  # NaN-safe
                    floats.append({
                        "offset": "0x%x" % (GLOBAL_DUMP_LO + off),
                        "value": round(value, 4),
                        "match": annotate(value),
                    })
            result["global_floats"] = floats

            samples = []
            for i in range(4):
                blk = dump(dbg, flare + 0x180, 0x100)
                samples.append({
                    "t": i,
                    "0x18c_fade_scale": round(f32(blk, 0x0C), 6),
                    "0x194_base": round(f32(blk, 0x14), 6),
                    "0x250": round(f32(blk, 0xD0), 6),
                    "0x254": round(f32(blk, 0xD4), 6),
                    "0x25c": round(f32(blk, 0xDC), 6),
                })
                dbg.resume()
                time.sleep(0.5)
                dbg.pause()
            result["flare_samples"] = samples
            dbg.resume()
        finally:
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
        (out / "meta.json").write_text(json.dumps(result, indent=1))
        session.pad.set("cross", False)
        drive.screenshot(out / "race.png")
        print(json.dumps(result, indent=1))
        print("done; artefacts in %s" % out, flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
