#!/usr/bin/env python3
"""Disassembles a byte range of `EBOOT.elf` as Cell SPU machine code.

This project has PPU (stock Ghidra PowerPC) and PSP Allegrex/VFPU (a built
processor module, `just build-allegrex`) covered, but has never had SPU
disassembly for any title - `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s
`Enable_spu_vertex_light` section names this as the barrier past which static
PPU reading cannot go. Ghidra ships no SPU processor module at all, and the one
third-party SLEIGH module found (`aerosoul94/GhidraSPU`) models SPU's 128-bit
registers as 64-bit and stubs `shufb` - the same "decompiler produces readable
fiction" failure `docs/psp/allegrex-vfpu.md` already warns about, so it is not
safe to trust without first fixing that defect.

`spu-elf` is a real, official upstream binutils target (the same one the Cell
SDK's own cross-toolchain uses) - trustworthy where a from-scratch SLEIGH
module is not. This script wraps `spu-objdump` from it, doing only the part
`objdump -b binary` cannot: mapping a virtual address to the ELF's own file
offset, via the program headers, the same way `scripts/ps3-toc.py`'s `Image`
class already does for PPU.

Needs `ps3-spu-binutils` (or equivalent `spu-elf` binutils) installed; this
machine has it at `/opt/ps3dev/spu/bin/spu-objdump` (override with
`--objdump` or `$SPU_OBJDUMP` if installed elsewhere).

    scripts/ps3-spu-disasm.py 0x00811680 0x4d40          # Trails SPU job
    scripts/ps3-spu-disasm.py 0x00816400 0x1240          # WakeTrail SPU job
    scripts/ps3-spu-disasm.py 0x00811680 0x4d40 --out data/extracted/ps3/spu-jobs/trails.bin

Self-check: both jobs above decode as coherent SPU code - a standard
prologue (`stqd $80..$82,$126,$0,$1` register spills, `ai $1,$1,-96` stack
allocation, `rdch $ch8` DMA-completion channel read) starting at file offset
0x30 within the blob, preceded by a 0x30-byte header (four `ila`-shaped
words, then four size/offset-shaped words) neither job's own code branches
into - not proof of what either job computes, only that the disassembly is
real, not garbage, and lines up with `engine-trail.md`'s independent finding
that `WakeTrail` is "structurally the engine trail's twin": both blobs decode
to byte-identical instruction shapes through at least their first 0x80 bytes,
differing only in the embedded literal constants.

Does NOT find a job's own address - `docs/rendering/trail-ribbon.md`'s two
jobs were named ones with a known runtime name string in their registration
call; a job reached only through a generic, shared submission primitive (this
project's own `FUN_005fc728`, see renderer.md) has no such fixed offset
findable this way, since its binary pointer resolves to a runtime-only
address outside any statically-loaded segment - that needs a live RPCS3 read
first, the same way the `Enable_spu_vertex_light` buffer itself was found.
"""

from __future__ import annotations

import argparse
import shutil
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_ELF = ROOT / "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf"
DEFAULT_OBJDUMP_CANDIDATES = ("spu-objdump", "/opt/ps3dev/spu/bin/spu-objdump")


class Image:
    """An ELF64 big-endian image addressed by virtual address."""

    def __init__(self, path: Path) -> None:
        self.raw = path.read_bytes()
        (e_phoff,) = struct.unpack_from(">Q", self.raw, 0x20)
        e_phentsize, e_phnum = struct.unpack_from(">HH", self.raw, 0x36)
        self.segments: list[tuple[int, int, int]] = []
        for i in range(e_phnum):
            base = e_phoff + i * e_phentsize
            p_type, _flags, p_offset, p_vaddr, _paddr, p_filesz, _memsz = struct.unpack_from(
                ">IIQQQQQ", self.raw, base
            )
            if p_type == 1 and p_filesz:
                self.segments.append((p_vaddr, p_vaddr + p_filesz, p_offset))
        self.segments.sort()

    def offset(self, vaddr: int) -> int | None:
        for lo, hi, off in self.segments:
            if lo <= vaddr < hi:
                return off + (vaddr - lo)
        return None


def find_objdump(explicit: str | None) -> str:
    if explicit:
        return explicit
    import os

    env = os.environ.get("SPU_OBJDUMP")
    if env:
        return env
    for candidate in DEFAULT_OBJDUMP_CANDIDATES:
        found = shutil.which(candidate) or (candidate if Path(candidate).exists() else None)
        if found:
            return found
    raise SystemExit(
        "spu-objdump not found - install spu-elf binutils (e.g. the `ps3-spu-binutils` "
        "AUR package) or pass --objdump /path/to/spu-objdump"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("vaddr", type=lambda s: int(s, 0), help="virtual address, e.g. 0x00811680")
    parser.add_argument("size", type=lambda s: int(s, 0), help="byte length, e.g. 0x4d40")
    parser.add_argument("--elf", type=Path, default=DEFAULT_ELF)
    parser.add_argument("--objdump")
    parser.add_argument("--out", type=Path, help="also write the extracted bytes here")
    args = parser.parse_args()

    objdump = find_objdump(args.objdump)
    image = Image(args.elf)
    file_off = image.offset(args.vaddr)
    if file_off is None:
        print(
            f"0x{args.vaddr:08x} is not inside any file-backed segment - this is a "
            "runtime-only address (the loader fills it in, or a live-only heap/job "
            "region), not something embedded in the ELF's own file bytes. A live "
            "RPCS3 read is needed first, the same way rpcs3-spu-light-dump.py reads "
            "its own runtime-only pointer.",
            file=sys.stderr,
        )
        return 1

    blob = image.raw[file_off : file_off + args.size]
    if len(blob) != args.size:
        print(
            f"warning: requested {args.size} bytes but only {len(blob)} available "
            "before the segment ends",
            file=sys.stderr,
        )

    if args.out:
        args.out.write_bytes(blob)
        print(f"wrote {len(blob)} bytes to {args.out}", file=sys.stderr)
        blob_path = args.out
    else:
        import tempfile

        fd, tmp = tempfile.mkstemp(suffix=".spu.bin")
        import os

        os.write(fd, blob)
        os.close(fd)
        blob_path = Path(tmp)

    subprocess.run([objdump, "-D", "-b", "binary", "-m", "spu", str(blob_path)], check=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
