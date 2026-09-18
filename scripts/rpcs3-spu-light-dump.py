#!/usr/bin/env python3
"""Reads Wipeout HD's `Enable_spu_vertex_light` double-buffer live, out of a
running race - the evidence reproducer for `docs/ghidra/functions/
ps3-hdfury-eu/renderer.md`'s "`Enable_spu_vertex_light` is read at 14 sites,
two named and reachable, and gates a double-buffered slot" section.

**Read live, twice, reproducibly: `0x008b83b0` never gets relocated.** The
first run of this script guarded on a heap-shaped address
(`0x10000000-0x50000000`, the range `rpcs3-trail-dump.py`'s own allocations
use) and rejected every read, because the pointer's *live* value is
`0x00f4b300` - byte-identical to the static ELF value `scripts/ps3-toc.py
resolve 0x0040d390 -0x5014` already reads offline. This is not a relocated
heap pointer; it is a fixed low address, assigned once and never moved.
Read the pointer, follow it (to whatever it actually is, live), read
`+0x2080` for the current slot index, then `+0x80 + index*0x1000` for that
slot's own 4 KiB - the same double-buffer-reading shape `scripts/
rpcs3-trail-dump.py` already uses for other SPU output.

**What is there: up to 8 records of 8 big-endian floats each,
`(x, y, z, w=1.0, A, A*0.25, A*0.1, D)`** - a world-space position, and one
authored scalar (`A`) with two fixed-ratio derived forms (confirmed exact
across every record read, including scaled ones), plus an independent
per-record scalar `D` that stays stable for a given position across frames
rather than varying like noise. Full account and the open questions this
raised: `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, the dated
addendum after the section this docstring opens with.

    uv run --with evdev python3 scripts/rpcs3-spu-light-dump.py [out_dir]

Needs the decrypted image (`data/images/hdfury-ps3-eu-dec.iso`), the `oag`
input profile and Xvfb :77 - `scripts/rpcs3-drive.py preflight` checks all
three (`uv run --with evdev python3 scripts/rpcs3-drive.py preflight` - the
plain-`python3` invocation checks display only, since `evdev` is not a
system dependency here, only a `uv`-ephemeral one). One debugger session per
emulator launch (`rpcs3_debugger.py`, trap 2), so this boots, drives to a
race, and dumps in one script rather than an interactive session, the same
shape `rpcs3-trail-dump.py` already uses.

**Drives to Amphiseum specifically** (Racebox's `Track Creation` carousel,
8 `right`s - `docs/reverse-engineering/rpcs3-capture.md`'s own read of the
carousel order) rather than wherever a plain `walk_to_race()` lands, since
Amphiseum is already the circuit `crates/render/examples/light_census.rs`
swept for vex-authored `PointLight` nodes and found zero - a circuit this
project already has independent evidence about, not one this script would
need to separately identify from a screenshot.

`Enable_spu_vertex_light`/`Debug.Enable EdgeGeom` both default to `1`
(`Environment_RegisterLightingSchema`, `0x003a83d8`) so no settings
override is needed for the gate to be open - default play already
exercises it if it does anything at all.

Five snapshots two seconds apart, so a moving slot index and changing
per-record positions distinguish "live SPU output, updated every frame"
from "an allocated but inert buffer."

**Also reads the `+0x2084` companion array** (`iRam008b83b0 + 0x2084 +
index*4`, one `u32` per slot index, same index as the buffer above) -
`_opd_FUN_0040d390`'s own return value, and per
`_opd_FUN_004074e0`'s decompile, opcode `0x2d`'s own `value` operand
whenever a chunk's per-chunk bit is set. Unlike the SPU job's own local
store (which this project's RPCS3 tooling cannot reach - RPCS3's GDB stub
only lists PPU threads), this array is ordinary PPU main memory, reachable
the same way the buffer above already is.
"""

import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("rpcs3_drive", ROOT / "scripts" / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger  # noqa: E402

IMAGE = ROOT / "data/images/hdfury-ps3-eu-dec.iso"

# `FUN_0040d390`'s own `lwz r11,-0x5014(r2)`, resolved against its own TOC
# (`scripts/ps3-toc.py toc 0x0040d390` -> exact) to this address - a pointer
# slot, not the buffer itself.
POINTER_SLOT = 0x008B83B0
INDEX_OFFSET = 0x2080
SLOT_BASE_OFFSET = 0x80
SLOT_STRIDE = 0x1000
SLOT_DUMP_LEN = 0x1000  # the whole slot

# `_opd_FUN_0040d390`'s own read: `*(int *)(iRam008b83b0 + 0x2080) * 4 +
# iRam008b83b0 + 0x2084` - a per-slot-index array parallel to the `+0x80`
# double-buffer above, sharing the same running index. Never read before
# 2026-09-18 - this is opcode `0x2d`'s own `value` operand, per
# `_opd_FUN_004074e0`'s decompile (`docs/ghidra/functions/ps3-hdfury-eu/
# renderer.md`, "Opcode `0x2d`'s `(address, value)` operand pair is
# decompiled at its own PPU-side source").
COMPANION_OFFSET = 0x2084


def u32(b, off=0):
    return struct.unpack_from(">I", b, off)[0]


def snapshot(dbg, out, tag):
    ptr = u32(dbg.read(POINTER_SLOT, 4))
    result = {"pointer_slot": "%08x" % POINTER_SLOT, "ptr": "%08x" % ptr}
    # Not the 0x10000000-0x50000000 heap range rpcs3-trail-dump.py's own
    # allocations use - this pointer never gets relocated at runtime (a
    # first live read found it still exactly its own static ELF value,
    # 0x00f4b300), so the guard is just "looks like a plausible main-RAM
    # address," not "looks like a heap allocation."
    if not (0x00010000 <= ptr < 0x50000000):
        result["note"] = "pointer outside plausible main-RAM range"
        return result
    index = u32(dbg.read(ptr + INDEX_OFFSET, 4))
    result["index_raw"] = index
    # A guard, not a measurement: an index this large is a sign the pointer
    # or offset is wrong, not a real slot to read - matches this project's
    # own MAX_INSTANCES-shaped guards elsewhere.
    if index > 0xFFFF:
        result["note"] = "index implausibly large, not reading a slot"
        return result
    companion_addr = ptr + COMPANION_OFFSET + index * 4
    companion_raw = dbg.read(companion_addr, 4)
    result["companion_addr"] = "%08x" % companion_addr
    result["companion_value_u32"] = u32(companion_raw)
    result["companion_value_f32"] = struct.unpack(">f", companion_raw)[0]

    slot_addr = ptr + SLOT_BASE_OFFSET + index * SLOT_STRIDE
    result["slot_addr"] = "%08x" % slot_addr
    data = dbg.read(slot_addr, SLOT_DUMP_LEN)
    (out / f"{tag}_slot.bin").write_bytes(data)
    nonzero = sum(1 for b in data if b != 0)
    result["slot_nonzero_bytes"] = nonzero
    # The first record read as position.xyz,w + 4 more floats - print every
    # 32-byte record as 8 floats, stopping at the first all-zero one, to see
    # whether that shape repeats (an array of light-like records) or was one
    # coincidence.
    records = []
    for i in range(0, len(data) - 31, 32):
        chunk = data[i : i + 32]
        if chunk == b"\x00" * 32:
            break
        records.append(struct.unpack(">8f", chunk))
    result["record_count"] = len(records)
    result["records"] = records[:16]
    return result


# Racebox's Track Creation carousel, 8 `right`s from its default highlight -
# `docs/reverse-engineering/rpcs3-capture.md`'s own read of the carousel
# order. Reaches Amphiseum specifically rather than whatever a plain
# `walk_to_race()` lands on (the Fury campaign's own default, unidentified
# by this script) - chosen because it is already the circuit this project's
# own `light_census.rs` swept for vex-authored PointLight nodes and found
# zero, so a live light-list here would be a clean "not from .vex" result
# rather than a circuit this session would need to separately identify.
AMPHISEUM_PLAN = {
    "Main Menu": ["right"],
    "Track Creation": ["right"] * 8,
}


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/hd-spu-light-dump")
    out.mkdir(parents=True, exist_ok=True)
    with drive.Session(str(IMAGE), str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", 180.0):
            print("never reached the Main Menu", file=sys.stderr)
            return 1
        time.sleep(12.0)
        if session.walk_to_race(plan=AMPHISEUM_PLAN) not in drive.RACE_ARRIVED:
            print("did not reach a race", file=sys.stderr)
            return 1
        print("in race; waiting for the load", flush=True)
        time.sleep(50.0)
        session.pad.set("cross", True)
        time.sleep(16.0)

        dbg = Debugger()
        metas = []
        try:
            for i, wait in enumerate((0.0, 2.0, 2.0, 2.0, 2.0)):
                if wait:
                    time.sleep(wait)
                tag = "s%d" % i
                dbg.pause()
                metas.append({"tag": tag, "wall": time.time(), **snapshot(dbg, out, tag)})
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
        for m in metas:
            print(m)
    return 0


if __name__ == "__main__":
    sys.exit(main())
