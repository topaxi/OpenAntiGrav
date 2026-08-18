#!/usr/bin/env python3
"""Resolves PPC64 TOC-relative references in the PS3 `EBOOT.elf`, correctly.

Every function in this binary carries its own TOC in its OPD entry, and there
are two of them. Ghidra uses the ELF entry point's for all 24,155 functions, so
59% of TOC-relative loads resolve to a real string at a real address that is
not the one the code loads - see
docs/ghidra/functions/ps3-hdfury-eu/memory.md. `AssignPs3R2FromOpd.java` fixes
that inside Ghidra; this fixes it from outside, against the file, with no
re-import and no bridge.

    scripts/ps3-toc.py toc      0x003914b0            # which TOC a function uses
    scripts/ps3-toc.py resolve  0x003914b0 -0x6634    # what `lwz rX,disp(r2)` reads
    scripts/ps3-toc.py attrib   0x007a1798            # which functions name a string
    scripts/ps3-toc.py map                            # every .cpp -> its functions
    scripts/ps3-toc.py str      0x007afe68            # a C string at an address
    scripts/ps3-toc.py u32      0x008c0854            # a word at an address

`attrib` and `map` are the `__FILE__` attribution trick done through each
function's own TOC: the C++ base class stores its `__FILE__` at object offset
0x30 and the allocation macro passes it at every call site, so a `.cpp` name
identifies the functions that construct or allocate for that class. It maps
constructors and allocators, not whole classes.

Self-check: `resolve 0x003914b0 -0x6634` must print
"Small is  %3.2f MB (%d bytes)". If it prints "forward" the TOC choice is
wrong, which is exactly the defect this exists to avoid.
"""

from __future__ import annotations

import bisect
import collections
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_ELF = ROOT / "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf"

# The OPD section, read off this image; see the ps3-hdfury-eu README.
OPD_LO, OPD_HI = 0x00870520, 0x008A54D8
# Code lives below this; the data segment above it is not worth scanning.
CODE_HI = 0x00800000


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

    def u32(self, vaddr: int) -> int | None:
        off = self.offset(vaddr)
        return None if off is None else struct.unpack_from(">I", self.raw, off)[0]

    def cstr(self, vaddr: int, limit: int = 160) -> str | None:
        off = self.offset(vaddr)
        if off is None:
            return None
        end = self.raw.find(b"\0", off, off + limit)
        if end < 0:
            return None
        try:
            return self.raw[off:end].decode("ascii")
        except UnicodeDecodeError:
            return None

    def word_addresses(self, value: int) -> list[int]:
        """Every aligned word in the image equal to `value`, as addresses."""
        needle = struct.pack(">I", value)
        hits: list[int] = []
        pos = 0
        while True:
            found = self.raw.find(needle, pos)
            if found < 0:
                return hits
            pos = found + 4
            if found % 4:
                continue
            for lo, hi, off in self.segments:
                if off <= found < off + (hi - lo):
                    hits.append(lo + (found - off))
                    break


class Opd:
    """The function-descriptor table: `{code address, TOC value}` pairs."""

    def __init__(self, image: Image) -> None:
        self.image = image
        self.toc: dict[int, int] = {}
        for entry in range(OPD_LO, OPD_HI, 8):
            func = image.u32(entry)
            toc = image.u32(entry + 4)
            if func is not None and toc is not None:
                self.toc.setdefault(func, toc)
        self.entries = sorted(self.toc)

    def toc_of(self, addr: int) -> tuple[int | None, bool]:
        """The TOC for `addr`; exact when `addr` is itself an entry point,
        otherwise inherited from the entry point it falls inside."""
        if addr in self.toc:
            return self.toc[addr], True
        index = bisect.bisect_right(self.entries, addr) - 1
        if index < 0:
            return None, False
        return self.toc[self.entries[index]], False

    def enclosing(self, addr: int) -> int | None:
        index = bisect.bisect_right(self.entries, addr) - 1
        return None if index < 0 else self.entries[index]


def scan_toc_loads(image: Image, opd: Opd) -> dict[int, list[tuple[int, int]]]:
    """Every `lwz rD,disp(r2)` in code, keyed by the slot it really reads.

    Returns `{slot: [(instruction address, enclosing function), ...]}`.
    """
    loads: dict[int, list[tuple[int, int]]] = collections.defaultdict(list)
    for lo, hi, off in image.segments:
        if lo >= CODE_HI:
            continue
        for addr in range(lo, min(hi, CODE_HI), 4):
            word = struct.unpack_from(">I", image.raw, off + (addr - lo))[0]
            if word >> 26 != 32 or (word >> 16) & 0x1F != 2:
                continue
            disp = word & 0xFFFF
            if disp > 0x7FFF:
                disp -= 0x10000
            func = opd.enclosing(addr)
            if func is None:
                continue
            loads[opd.toc[func] + disp].append((addr, func))
    return loads


def signed16(text: str) -> int:
    value = int(text, 16)
    return value - 0x10000 if value > 0x7FFF else value


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    image = Image(DEFAULT_ELF)
    opd = Opd(image)
    command, args = argv[1], argv[2:]

    if command == "toc":
        for arg in args:
            addr = int(arg, 16)
            toc, exact = opd.toc_of(addr)
            print(f"{addr:#010x} toc={toc:#010x} {'exact' if exact else 'inherited'}")
    elif command == "resolve":
        addr = int(args[0], 16)
        toc, exact = opd.toc_of(addr)
        for arg in args[1:]:
            slot = toc + signed16(arg)
            pointer = image.u32(slot)
            text = image.cstr(pointer) if pointer else None
            note = "" if exact else " (inherited TOC)"
            print(f"{slot:#010x} -> {pointer:#010x} {text!r}{note}")
    elif command == "attrib":
        loads = scan_toc_loads(image, opd)
        for arg in args:
            target = int(arg, 16)
            slots = image.word_addresses(target)
            print(f"== {image.cstr(target)!r} @ {target:#010x}")
            functions = sorted({f for slot in slots for _, f in loads.get(slot, [])})
            for func in functions:
                print(f"   {func:#010x}")
    elif command == "map":
        loads = scan_toc_loads(image, opd)
        by_file: dict[str, set[int]] = collections.defaultdict(set)
        for slot, uses in loads.items():
            pointer = image.u32(slot)
            if not pointer:
                continue
            name = image.cstr(pointer, 80)
            if not name or not name.endswith(".cpp"):
                continue
            by_file[name].update(func for _, func in uses)
        for name in sorted(by_file):
            functions = sorted(by_file[name])
            addresses = " ".join(f"{f:#010x}" for f in functions)
            print(f"{name}\t{len(functions)}\t{addresses}")
    elif command == "str":
        for arg in args:
            addr = int(arg, 16)
            print(f"{addr:#010x} {image.cstr(addr)!r}")
    elif command == "u32":
        for arg in args:
            addr = int(arg, 16)
            print(f"{addr:#010x} {image.u32(addr):#010x}")
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
