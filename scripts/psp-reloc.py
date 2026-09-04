#!/usr/bin/env python3
"""What a PSP PRX's `lui`/`addiu` pair actually points at, relocations included.

  psp-reloc.py sections <BOOT.BIN>                 the ELF's segments and sections
  psp-reloc.py at       <BOOT.BIN> <address>...    relocation entries on an instruction
  psp-reloc.py pair     <BOOT.BIN> <address>       resolve the pair starting here
  psp-reloc.py refs     <BOOT.BIN> <address>...    every site in .text forming this address
  psp-reloc.py read     <BOOT.BIN> <address> [n]   n bytes there, as u32 and f32

Addresses are Ghidra's, at this project's `0x08804000` image base, unless they
are below it - then they are taken as raw segment-0 offsets.

# Why this exists

A PSP executable is a **relocatable PRX with two segments**, and a `lui`/`addiu`
pair that reaches the second one stores its addend relative to *that* segment.
The loader adds the segment base; Ghidra applies it to nothing. So the obvious
reading - `(hi << 16) + lo` - lands inside `.text` for every data reference to
segment 1, decodes as instructions, and reads exactly like "there are no bytes
at this address".

Three of this project's own pages record being caught by it
(`shield-pickup.md`, `autopilot.md`, `positional-audio.md`), and a fourth spent
a pass recording the shadow direction constant as unfindable for the same
reason - see
`docs/ghidra/functions/psp-pulse-usa/shadow-occluder.md`. The relocation table
answers it outright: `.rel.text`'s entries carry an `addr_base` naming the
segment whose base is added.

The disassembly here is deliberately partial. It decodes address formation and
nothing else, because that is the question; anything more belongs in Ghidra.
"""

import struct
import sys

# This project's image base for the PSP executables - see
# docs/psp/allegrex-vfpu.md, "On the value".
BASE = 0x08804000

# The opcodes that take a `lui`-formed register as a base. Address formation
# only: this is not a disassembler.
LOADS_AND_STORES = {
    0x09: "addiu",
    0x20: "lb",
    0x21: "lh",
    0x23: "lw",
    0x24: "lbu",
    0x25: "lhu",
    0x28: "sb",
    0x29: "sh",
    0x2B: "sw",
    0x31: "lwc1",
    0x32: "lv.s",
    0x36: "lv.q",
    0x39: "swc1",
    0x3A: "sv.s",
    0x3E: "sv.q",
}

# The quad/single VFPU transfers encode the low two bits of the offset field as
# part of the register number, so their offset is the field masked to a word.
VFPU = {0x32, 0x36, 0x3A, 0x3E}

RELOC_TYPES = {0x00: "NONE", 0x04: "MIPS26", 0x05: "HI16", 0x06: "LO16", 0x07: "GPREL"}


class Elf:
    """A PSP PRX, enough of it to map an address to a file offset."""

    def __init__(self, path):
        self.data = open(path, "rb").read()
        if self.data[:4] != b"\x7fELF":
            raise SystemExit(f"{path}: not an ELF")
        (e_phoff, e_shoff) = struct.unpack_from("<II", self.data, 28)
        (e_phentsize, e_phnum, e_shentsize, e_shnum, e_shstrndx) = struct.unpack_from(
            "<HHHHH", self.data, 42
        )
        self.segments = []
        for i in range(e_phnum):
            at = e_phoff + i * e_phentsize
            _, offset, vaddr, _, filesz, memsz, _, _ = struct.unpack_from("<8I", self.data, at)
            self.segments.append((offset, vaddr, filesz, memsz))
        shstr = struct.unpack_from("<I", self.data, e_shoff + e_shstrndx * e_shentsize + 16)[0]
        self.sections = []
        for i in range(e_shnum):
            at = e_shoff + i * e_shentsize
            sh_name, sh_type, _, sh_addr, sh_offset, sh_size = struct.unpack_from(
                "<6I", self.data, at
            )
            end = self.data.index(b"\0", shstr + sh_name)
            name = self.data[shstr + sh_name : end].decode()
            # `sh_type == 8` is NOBITS: `.bss` has an address and no bytes.
            self.sections.append((name, sh_addr, sh_offset, sh_size, sh_type == 8))

    def section_of(self, vaddr):
        """`(name, file offset or None)` for a segment-0 virtual address."""
        for name, addr, offset, size, nobits in self.sections:
            if size and addr <= vaddr < addr + size:
                return name, (None if nobits else offset + (vaddr - addr))
        return None, None

    def word(self, vaddr):
        _, offset = self.section_of(vaddr)
        if offset is None:
            raise SystemExit(f"{vaddr:#010x}: no bytes there")
        return struct.unpack_from("<I", self.data, offset)[0]

    def relocations(self):
        """Every `.rel.*` entry, as `(offset, type, ofs_base, addr_base)`."""
        for name, _, offset, size, _ in self.sections:
            if not name.startswith(".rel") or not size:
                continue
            for i in range(size // 8):
                where, info = struct.unpack_from("<II", self.data, offset + i * 8)
                yield where, info & 0xFF, (info >> 8) & 0xFF, (info >> 16) & 0xFF

    def segment_base(self, index):
        return self.segments[index][1] if index < len(self.segments) else 0


def raw(elf, text):
    """A command-line address as a segment-0 virtual address."""
    value = int(text, 16) if text.lower().startswith("0x") else int(text, 0)
    return value - BASE if value >= BASE else value


def decode(word):
    """`(op, rs, rt, offset)` for an instruction that may form an address."""
    op = word >> 26
    rs = (word >> 21) & 31
    rt = (word >> 16) & 31
    imm = word & 0xFFFF
    if op in VFPU:
        imm &= 0xFFFC
    offset = imm - 0x10000 if imm & 0x8000 else imm
    return op, rs, rt, offset


def relocation_map(elf):
    out = {}
    for where, rtype, ofs_base, addr_base in elf.relocations():
        out.setdefault(where, []).append((rtype, ofs_base, addr_base))
    return out


def describe(elf, target, addr_base):
    """Where an encoded address really lands, once its segment base is added."""
    resolved = target + elf.segment_base(addr_base)
    name, _ = elf.section_of(resolved)
    return resolved, name


def cmd_sections(elf, args):
    for index, (offset, vaddr, filesz, memsz) in enumerate(elf.segments):
        print(
            f"segment {index}: file {offset:#x} vaddr {vaddr:#010x} "
            f"filesz {filesz:#x} memsz {memsz:#x}"
        )
    for name, addr, offset, size, nobits in elf.sections:
        if size:
            kind = "NOBITS" if nobits else "      "
            print(f"  {name:<24} {kind} addr {addr:#010x} off {offset:#010x} size {size:#x}")


def cmd_at(elf, args):
    relocations = relocation_map(elf)
    for text in args:
        where = raw(elf, text)
        entries = relocations.get(where, [])
        if not entries:
            print(f"{where + BASE:#010x}: no relocation")
            continue
        for rtype, ofs_base, addr_base in entries:
            print(
                f"{where + BASE:#010x}: {RELOC_TYPES.get(rtype, hex(rtype)):<6} "
                f"ofs_base {ofs_base} addr_base {addr_base} "
                f"(adds {elf.segment_base(addr_base):#x})"
            )


def cmd_pair(elf, args):
    """Resolve the `lui` at this address against the next instruction using it."""
    start = raw(elf, args[0])
    relocations = relocation_map(elf)
    word = elf.word(start)
    op, _, rt, offset = decode(word)
    if op != 0x0F:
        raise SystemExit(f"{start + BASE:#010x}: not a lui")
    high = word & 0xFFFF
    for step in range(1, 16):
        at = start + step * 4
        op, rs, _, offset = decode(elf.word(at))
        if op == 0x0F and (elf.word(at) >> 16) & 31 == rt:
            print(f"{at + BASE:#010x}: the register is rebuilt before it is used")
            return
        if op not in LOADS_AND_STORES or rs != rt:
            continue
        target = (high << 16) + offset
        # The relocation on the *low* half names the segment; without one the
        # pair is segment-0 relative and the naive reading is already right.
        entries = relocations.get(at, [(0x06, 0, 0)])
        for rtype, _, addr_base in entries:
            resolved, name = describe(elf, target, addr_base)
            naive_name, _ = elf.section_of(target)
            print(
                f"{start + BASE:#010x}/{at + BASE:#010x}  {LOADS_AND_STORES[op]:<5} "
                f"encoded {target:#010x} ({naive_name}) "
                f"addr_base {addr_base} -> {resolved:#010x} ({name}) "
                f"= {resolved + BASE:#010x} at this base"
            )
        return
    print(f"{start + BASE:#010x}: no use of the register within 16 instructions")


def cmd_refs(elf, args):
    """Every site in `.text` whose pair forms one of these resolved addresses."""
    wanted = {raw(elf, text) for text in args}
    relocations = relocation_map(elf)
    text = next((s for s in elf.sections if s[0] == ".text"), None)
    if text is None:
        raise SystemExit("no .text")
    _, addr, offset, size, _ = text
    high = {}
    for i in range(size // 4):
        at = addr + i * 4
        word = struct.unpack_from("<I", elf.data, offset + i * 4)[0]
        op, rs, rt, imm = decode(word)
        if op == 0x0F:
            high[rt] = word & 0xFFFF
            continue
        if op not in LOADS_AND_STORES or rs not in high:
            continue
        target = (high[rs] << 16) + imm
        for _, _, addr_base in relocations.get(at, [(0x06, 0, 0)]):
            resolved, name = describe(elf, target, addr_base)
            if resolved in wanted:
                print(
                    f"{at + BASE:#010x}: {LOADS_AND_STORES[op]:<5} "
                    f"{resolved:#010x} ({name}) = {resolved + BASE:#010x}"
                )


def cmd_read(elf, args):
    vaddr = raw(elf, args[0])
    count = int(args[1]) if len(args) > 1 else 16
    name, offset = elf.section_of(vaddr)
    if offset is None:
        print(f"{vaddr + BASE:#010x}: in {name}, which has no bytes in the file")
        return
    blob = elf.data[offset : offset + count]
    print(f"{vaddr + BASE:#010x}: {name}, file offset {offset:#x}")
    print("  bytes " + blob.hex(" ", 4))
    for i in range(0, len(blob) - 3, 4):
        (word,) = struct.unpack_from("<I", blob, i)
        (value,) = struct.unpack_from("<f", blob, i)
        print(f"  +{i:#04x}  u32 {word:#010x}  f32 {value:+.8g}")


COMMANDS = {
    "sections": cmd_sections,
    "at": cmd_at,
    "pair": cmd_pair,
    "refs": cmd_refs,
    "read": cmd_read,
}


def main(argv):
    if len(argv) < 3 or argv[1] not in COMMANDS:
        print(__doc__)
        return 2
    elf = Elf(argv[2])
    COMMANDS[argv[1]](elf, argv[3:])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
