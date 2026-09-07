#!/usr/bin/env python3
"""Resolves a PSP module's import stubs to library function names.

A PSP executable calls system libraries through 8-byte stubs. The ELF carries no
symbol for them: each imported function is identified only by a 32-bit **NID**,
and the NID is the first four bytes of `SHA-1(name)`, little-endian. That is a
one-way function, so a NID cannot be turned back into a name, but a *candidate*
name can be hashed and compared, which turns naming imports into a check rather
than a guess.

`scripts/psp-import-names.txt` holds the candidate names (the public PSP SDK
surface). Every match here is therefore verified, not inferred: a wrong name
cannot produce the right NID.

Layout, all of it from the ELF's own section headers:

- `.lib.stub`, 20 bytes per imported module:
  `{u32 name, u16 version, u16 flags, u8 len, u8 var_count, u16 func_count,
    u32 nid_table, u32 stub_table}`
- `.rodata.sceNid`, one `u32` NID per imported function
- `.sceStub.text.<module>`, 8 bytes per stub, parallel to that module's NIDs

Pointers in the file are unrelocated, so `--base` is added on output to match the
address the binary is analysed at.

This is region-blind by design - a NID is a hash of a *function name*, not of
which disc it shipped on, so the same candidate list matches any PSP build.
What is region-specific is the *addresses* a run reports, which belong to
whichever `BOOT.BIN` was actually pointed at. The conventional
`data/extracted/psp/PSP_GAME/SYSDIR/BOOT.BIN` path is not itself region-tagged
and has always held the USA disc's extract in practice, matching `justfile`'s
`psp_image` default and `apply-ghidra-names.py`'s pairing of its output,
`psp-imports.tsv`, with `psp-pulse-usa` specifically - unaffected by
[ADR-0048](../docs/architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md)
making `psp-pulse-eu` the Ghidra target of record, since nothing here forces
that path to mean "the target of record's binary". Point this script at the
EU `BOOT.BIN` instead (`just resolve-imports boot=<eu path>
out=data/ghidra/psp-imports-eu.tsv`) to build the EU-addressed counterpart;
see `docs/reverse-engineering/methodology.md#where-to-start` for why that is
not simply the new default.

Usage:
    scripts/resolve-psp-imports.py data/extracted/psp/PSP_GAME/SYSDIR/BOOT.BIN
    scripts/resolve-psp-imports.py <boot.bin> -o data/ghidra/psp-imports.tsv
"""

from __future__ import annotations

import argparse
import hashlib
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CANDIDATES = ROOT / "scripts" / "psp-import-names.txt"

STUB_ENTRY_SIZE = 20
STUB_SIZE = 8


class Elf:
    """Just enough ELF32 little-endian to find sections by name."""

    def __init__(self, data: bytes) -> None:
        if data[:4] != b"\x7fELF":
            raise ValueError("not an ELF file")
        self.data = data
        e_shoff, = struct.unpack_from("<I", data, 0x20)
        e_shentsize, e_shnum, e_shstrndx = struct.unpack_from("<HHH", data, 0x2E)
        self.sections = []
        for i in range(e_shnum):
            off = e_shoff + i * e_shentsize
            name, _type, _flags, addr, offset, size = struct.unpack_from(
                "<IIIIII", data, off
            )
            self.sections.append({"name_off": name, "addr": addr, "offset": offset, "size": size})
        strtab = self.sections[e_shstrndx]
        for s in self.sections:
            start = strtab["offset"] + s["name_off"]
            s["name"] = data[start : data.index(b"\0", start)].decode("ascii")

    def section(self, name: str) -> dict:
        for s in self.sections:
            if s["name"] == name:
                return s
        raise KeyError(name)

    def at(self, vaddr: int, length: int) -> bytes:
        """Read `length` bytes from a virtual address, via the section table."""
        for s in self.sections:
            if s["addr"] and s["addr"] <= vaddr < s["addr"] + s["size"]:
                start = s["offset"] + (vaddr - s["addr"])
                return self.data[start : start + length]
        raise ValueError(f"no section contains 0x{vaddr:08x}")

    def cstr(self, vaddr: int, limit: int = 128) -> str:
        raw = self.at(vaddr, limit)
        return raw.split(b"\0", 1)[0].decode("ascii", "replace")


def nid(name: str) -> int:
    return struct.unpack("<I", hashlib.sha1(name.encode()).digest()[:4])[0]


def modules(elf: Elf) -> list[dict]:
    stub = elf.section(".lib.stub")
    count, extra = divmod(stub["size"], STUB_ENTRY_SIZE)
    if extra:
        raise ValueError(f".lib.stub size {stub['size']} is not a multiple of 20")
    out = []
    for i in range(count):
        entry = elf.data[stub["offset"] + i * STUB_ENTRY_SIZE :][:STUB_ENTRY_SIZE]
        name_ptr, version, flags, _len, var_count, func_count, nid_table, stub_table = (
            struct.unpack("<IHHBBHII", entry)
        )
        out.append(
            {
                "module": elf.cstr(name_ptr),
                "version": version,
                "flags": flags,
                "var_count": var_count,
                "func_count": func_count,
                "nid_table": nid_table,
                "stub_table": stub_table,
            }
        )
    return out


def resolve(elf: Elf, base: int) -> tuple[list[tuple[int, str, str]], list[tuple[int, str, int]], list[dict]]:
    by_nid = {}
    for line in CANDIDATES.read_text().splitlines():
        name = line.strip()
        if name and not name.startswith("#"):
            by_nid[nid(name)] = name

    known: list[tuple[int, str, str]] = []
    unknown: list[tuple[int, str, int]] = []
    mods = modules(elf)
    for mod in mods:
        if not mod["func_count"]:
            continue
        nids = elf.at(mod["nid_table"], mod["func_count"] * 4)
        for i in range(mod["func_count"]):
            (value,) = struct.unpack_from("<I", nids, i * 4)
            addr = base + mod["stub_table"] + i * STUB_SIZE
            name = by_nid.get(value)
            if name:
                known.append((addr, name, mod["module"]))
            else:
                unknown.append((addr, mod["module"], value))
    return known, unknown, mods


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("boot", type=Path, help="the unencrypted BOOT.BIN")
    ap.add_argument("-o", "--output", type=Path, help="write a names TSV here")
    ap.add_argument(
        "--base",
        type=lambda s: int(s, 0),
        default=0x08804000,
        help="image base to add to stub addresses (default 0x08804000)",
    )
    ap.add_argument("--modules", action="store_true", help="print the module inventory")
    args = ap.parse_args()

    if not args.boot.is_file():
        print(f"{args.boot}: not found. See docs/ghidra/workflow.md.", file=sys.stderr)
        return 1

    elf = Elf(args.boot.read_bytes())
    known, unknown, mods = resolve(elf, args.base)
    total = len(known) + len(unknown)

    if args.modules:
        print(f"{len(mods)} imported modules, {total} stubs")
        for mod in sorted(mods, key=lambda m: m["module"]):
            named = sum(1 for _, _, m in known if m == mod["module"])
            print(
                f"  {mod['module']:<24} {mod['func_count']:>3} functions, "
                f"{named:>3} resolved, flags 0x{mod['flags']:04x}"
            )
        print()

    print(f"{len(known)} of {total} import stubs resolved by NID")
    if unknown:
        print(f"{len(unknown)} unresolved:")
        for addr, module, value in unknown:
            print(f"  0x{addr:08x} {module:<20} nid 0x{value:08X}")

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("w") as f:
            f.write("# Generated by scripts/resolve-psp-imports.py. Do not edit.\n")
            f.write("# Every name here matches its stub's NID, so all are confidence 99.\n")
            f.write("# address\tkind\tname\tconfidence\tevidence\n")
            for addr, name, _module in sorted(known):
                f.write(f"0x{addr:08x}\tfunction\t{name}\t99\t-\n")
        print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
