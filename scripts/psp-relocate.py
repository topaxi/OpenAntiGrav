#!/usr/bin/env python3
"""Resolve a PSP PRX's addresses the way its loader does, outside Ghidra.

Ghidra's PSP import applies no relocation at all (see
`docs/ghidra/workflow.md`), so every address-shaped value read off this
binary's static bytes is pre-relocation: a `jal` target, a `lui`/`lo16` pair
and a plain 32-bit data word alike. This walks the PRX relocation sections
and reports what each of those actually points at once loaded, which makes
cross-references findable again.

Subcommands:

  resolve  <addr>...   what the lui/lo16 pair at each instruction points at
  xrefs    <addr>      every instruction whose relocated operand equals addr
  callers  <addr>      every `jal` into addr
  field    <off>       every load/store using the given struct offset
  andi     <imm>       every `andi`/`ori` with the given immediate
  segments             the PT_LOAD table and the load base of each segment
"""

import argparse
import struct
import sys
from pathlib import Path

# Where the Ghidra databases put segment 0, so addresses printed here match
# the ones written down in docs/ghidra/.
DEFAULT_LOAD_BASE = 0x08804000

R_MIPS_26, R_MIPS_HI16, R_MIPS_LO16, R_MIPS_32 = 4, 5, 6, 2

SHT_PRXRELOC = 0x700000A0
PT_LOAD = 1


class Prx:
    def __init__(self, path: Path, load_base: int = DEFAULT_LOAD_BASE):
        self.data = path.read_bytes()
        self.load_base = load_base
        self.segments = self._program_headers()
        # The PSP loader lays the segments out contiguously from the load
        # base, which for this file is the same as `load_base + p_vaddr`
        # because segment 1's vaddr already sits at segment 0's aligned end.
        self.seg_load = [load_base + s["vaddr"] for s in self.segments]
        self.relocs = self._relocations()
        self.resolved = self._apply()

    def _program_headers(self):
        (phoff,) = struct.unpack_from("<I", self.data, 28)
        phentsize, phnum = struct.unpack_from("<HH", self.data, 42)
        out = []
        for i in range(phnum):
            f = struct.unpack_from("<8I", self.data, phoff + i * phentsize)
            if f[0] != PT_LOAD:
                continue
            out.append({"offset": f[1], "vaddr": f[2], "filesz": f[4], "memsz": f[5]})
        return out

    def _reloc_sections(self):
        (shoff,) = struct.unpack_from("<I", self.data, 32)
        shentsize, shnum = struct.unpack_from("<HH", self.data, 46)
        for i in range(shnum):
            f = struct.unpack_from("<10I", self.data, shoff + i * shentsize)
            if f[1] == SHT_PRXRELOC:
                yield f[4], f[5]

    def _relocations(self):
        out = []
        for off, size in self._reloc_sections():
            for pos in range(off, off + size, 8):
                r_offset, r_info = struct.unpack_from("<II", self.data, pos)
                out.append(
                    (
                        r_offset,
                        r_info & 0xFF,  # type
                        (r_info >> 8) & 0xFF,  # segment the offset is in
                        (r_info >> 16) & 0xFF,  # segment the value refers to
                    )
                )
        return out

    def file_offset(self, vaddr: int):
        for s in self.segments:
            if s["vaddr"] <= vaddr < s["vaddr"] + s["filesz"]:
                return s["offset"] + (vaddr - s["vaddr"])
        return None

    def word_at(self, vaddr: int):
        off = self.file_offset(vaddr)
        if off is None:
            return None
        return struct.unpack_from("<I", self.data, off)[0]

    def _apply(self):
        """Replay the relocations, recording the loaded address each one names.

        HI16 records defer until the LO16 that completes them, exactly as the
        loader does, so a negative LO16's borrow into the HI16 is handled here
        rather than by hand at the call site - which is where hand arithmetic
        on this binary has gone wrong before.
        """
        resolved = {}
        pending = []
        for r_offset, kind, ofs_seg, addr_seg in self.relocs:
            if ofs_seg >= len(self.segments) or addr_seg >= len(self.seg_load):
                continue
            vaddr = self.segments[ofs_seg]["vaddr"] + r_offset
            word = self.word_at(vaddr)
            if word is None:
                continue
            base = self.seg_load[addr_seg]
            if kind == R_MIPS_HI16:
                pending.append((vaddr, word & 0xFFFF, base))
            elif kind == R_MIPS_LO16:
                lo = word & 0xFFFF
                signed_lo = lo - 0x10000 if lo & 0x8000 else lo
                for hi_vaddr, hi, hi_base in pending:
                    resolved[hi_vaddr] = ((hi << 16) + signed_lo) + hi_base
                if pending:
                    resolved[vaddr] = ((pending[-1][1] << 16) + signed_lo) + base
                else:
                    resolved[vaddr] = signed_lo + base
                pending = []
            elif kind == R_MIPS_26:
                resolved[vaddr] = ((word & 0x03FFFFFF) << 2) + base
            elif kind == R_MIPS_32:
                resolved[vaddr] = word + base
        return resolved

    def to_vaddr(self, addr: int):
        return addr - self.load_base if addr >= self.load_base else addr

    def to_loaded(self, vaddr: int):
        return vaddr + self.load_base

    def text(self):
        s = self.segments[0]
        return s["offset"], s["vaddr"], s["filesz"]


def iter_instructions(prx):
    off, vaddr, size = prx.text()
    data = prx.data
    for i in range(0, size - 3, 4):
        yield vaddr + i, struct.unpack_from("<I", data, off + i)[0]


def cmd_segments(prx, _args):
    for i, s in enumerate(prx.segments):
        print(
            f"segment {i}: vaddr=0x{s['vaddr']:08x} filesz=0x{s['filesz']:x} "
            f"memsz=0x{s['memsz']:x} -> loads at 0x{prx.seg_load[i]:08x}"
        )


def cmd_resolve(prx, args):
    for a in args.addresses:
        addr = int(a, 0)
        vaddr = prx.to_vaddr(addr)
        target = prx.resolved.get(vaddr)
        word = prx.word_at(vaddr)
        if target is None:
            print(f"0x{addr:08x}: no relocation record (word=0x{word:08x})")
        else:
            print(f"0x{addr:08x}: -> 0x{target:08x}  (word=0x{word:08x})")


def cmd_xrefs(prx, args):
    target = int(args.address, 0)
    hits = [(v, t) for v, t in prx.resolved.items() if t == target]
    for vaddr, _ in sorted(hits):
        word = prx.word_at(vaddr)
        print(f"0x{prx.to_loaded(vaddr):08x}  word=0x{word:08x}")
    print(f"{len(hits)} reference(s) to 0x{target:08x}")


def cmd_callers(prx, args):
    target_v = prx.to_vaddr(int(args.address, 0))
    encoded = (target_v >> 2) & 0x03FFFFFF
    n = 0
    for vaddr, word in iter_instructions(prx):
        op = word >> 26
        if op in (2, 3) and (word & 0x03FFFFFF) == encoded:
            kind = "j" if op == 2 else "jal"
            print(f"0x{prx.to_loaded(vaddr):08x}  {kind}")
            n += 1
    print(f"{n} call site(s) into 0x{prx.to_loaded(target_v):08x}")


LOADS = {
    0x20: "lb",
    0x21: "lh",
    0x23: "lw",
    0x24: "lbu",
    0x25: "lhu",
    0x28: "sb",
    0x29: "sh",
    0x2B: "sw",
    0x31: "lwc1",
    0x39: "swc1",
}


def cmd_field(prx, args):
    want = int(args.offset, 0)
    n = 0
    for vaddr, word in iter_instructions(prx):
        op = word >> 26
        if op not in LOADS:
            continue
        imm = word & 0xFFFF
        imm = imm - 0x10000 if imm & 0x8000 else imm
        if imm != want:
            continue
        base = (word >> 21) & 0x1F
        rt = (word >> 16) & 0x1F
        print(f"0x{prx.to_loaded(vaddr):08x}  {LOADS[op]} r{rt}, 0x{want:x}(r{base})")
        n += 1
    print(f"{n} access(es) at struct offset 0x{want:x}")


def cmd_andi(prx, args):
    want = int(args.immediate, 0)
    n = 0
    for vaddr, word in iter_instructions(prx):
        op = word >> 26
        # andi (0x0c) and ori (0x0d) share the I-type shape.
        if op not in (0x0C, 0x0D):
            continue
        if (word & 0xFFFF) != want:
            continue
        rs, rt = (word >> 21) & 0x1F, (word >> 16) & 0x1F
        mnem = "andi" if op == 0x0C else "ori"
        print(f"0x{prx.to_loaded(vaddr):08x}  {mnem} r{rt}, r{rs}, 0x{want:x}")
        n += 1
    print(f"{n} andi/ori site(s) with immediate 0x{want:x}")


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--binary", required=True, type=Path)
    p.add_argument("--load-base", default=hex(DEFAULT_LOAD_BASE))
    sub = p.add_subparsers(dest="cmd", required=True)

    sub.add_parser("segments")
    q = sub.add_parser("resolve")
    q.add_argument("addresses", nargs="+")
    q = sub.add_parser("xrefs")
    q.add_argument("address")
    q = sub.add_parser("callers")
    q.add_argument("address")
    q = sub.add_parser("field")
    q.add_argument("offset")
    q = sub.add_parser("andi")
    q.add_argument("immediate")

    args = p.parse_args()
    prx = Prx(args.binary, int(args.load_base, 0))
    {
        "segments": cmd_segments,
        "resolve": cmd_resolve,
        "xrefs": cmd_xrefs,
        "callers": cmd_callers,
        "field": cmd_field,
        "andi": cmd_andi,
    }[args.cmd](prx, args)
    return 0


if __name__ == "__main__":
    sys.exit(main())
