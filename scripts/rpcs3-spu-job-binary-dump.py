#!/usr/bin/env python3
"""Reads the shared SPU job binary pointer `FUN_005fc728` uses, live, out of a
running race - the reproducer for `docs/ghidra/functions/ps3-hdfury-eu/
renderer.md`'s "SPU disassembly is now unblocked, tooling-wise" section.

`FUN_005fc728` is the generic SPU job-submission primitive every shadow-
redraw and "Zone Stage" draw compiler in this project's `Enable_spu_vertex_light`
thread calls (see renderer.md, "`FUN_005fc728` is confirmed as a genuine SPU
job dispatch"). Its own builder references a job "binary" pointer at the
fixed address `0x008bf6cc`, shared by every one of its seven callers - one
SPU program services all of them. Read statically off the ELF, that address
holds `0x00f73c00` - outside every file-backed segment
(`scripts/ps3-spu-disasm.py` refuses it), the same shape `0x008b83b0` (the
`Enable_spu_vertex_light` buffer's own pointer) had before its live value was
read. `get_xrefs_to 0x008bf6cc` finds no write, so whether this is set once
at boot by an unanalysed loader routine or is itself a static placeholder is
not established - only a live read settles it.

    uv run --with evdev python3 scripts/rpcs3-spu-job-binary-dump.py [out_dir]

Needs the decrypted image (`data/images/hdfury-ps3-eu-dec.iso`), the `oag`
input profile and Xvfb :77 - `scripts/rpcs3-drive.py preflight` checks all
three. Drives to Amphiseum (the same circuit `rpcs3-spu-light-dump.py`
already reads) so the two captures are directly comparable, and so the
Zone-Stage material draw calls that trigger `FUN_005fc728` are definitely
running by the time this reads.

Dumps up to `DUMP_LEN` bytes starting at the resolved pointer - large enough
to comfortably cover a `Trails`-sized job (`0x4d40` bytes) with headroom -
so the result can be fed straight to `spu-objdump -D -b binary -m spu` (or
`scripts/ps3-spu-disasm.py`'s own `Image`-based extraction does not apply
here, since this is already raw bytes, not a virtual address inside
`EBOOT.elf`).
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

# `PTR_DAT_008bf6cc`'s own address, read directly off the ELF - see renderer.md.
BINARY_POINTER_SLOT = 0x008BF6CC
# `PTR_DAT_008bf6b8`, the job-system struct's own pointer, read alongside for
# context - if it turns out `0x008bf6cc` is really `*0x008bf6b8 + 0x14` at
# runtime rather than an independent global, the two reads will show it.
STRUCT_POINTER_SLOT = 0x008BF6B8
DUMP_LEN = 0x8000

AMPHISEUM_PLAN = {
    "Main Menu": ["right"],
    "Track Creation": ["right"] * 8,
}


def u32(b, off=0):
    return struct.unpack_from(">I", b, off)[0]


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/hd-spu-job-binary-dump")
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

        dbg = Debugger()
        meta = {}
        try:
            dbg.pause()
            binary_ptr = u32(dbg.read(BINARY_POINTER_SLOT, 4))
            struct_ptr = u32(dbg.read(STRUCT_POINTER_SLOT, 4))
            meta["binary_pointer_slot"] = "%08x" % BINARY_POINTER_SLOT
            meta["binary_ptr"] = "%08x" % binary_ptr
            meta["struct_pointer_slot"] = "%08x" % STRUCT_POINTER_SLOT
            meta["struct_ptr"] = "%08x" % struct_ptr
            meta["matches_struct_plus_0x14"] = binary_ptr == (struct_ptr + 0x14)
            if 0x00010000 <= binary_ptr < 0x50000000:
                data = dbg.read(binary_ptr, DUMP_LEN)
                (out / "job_binary.bin").write_bytes(data)
                meta["dumped_bytes"] = len(data)
                meta["first_32_bytes_hex"] = data[:32].hex()
            else:
                meta["note"] = "binary_ptr outside plausible main-RAM range"
            dbg.resume()
        finally:
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
        (out / "meta.json").write_text(json.dumps(meta, indent=1))
        drive.screenshot(out / "race.png")
        print("done; artefacts in %s" % out, flush=True)
        print(meta)
    return 0


if __name__ == "__main__":
    sys.exit(main())
