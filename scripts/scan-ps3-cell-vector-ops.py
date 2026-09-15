#!/usr/bin/env python3
"""Count the Cell/PPC970 vector load/store forms stock Ghidra cannot decode.

`lvlx`, `lvrx`, `stvlx`, `stvrx` and their `l` (LRU-hint) variants are the
"Load/Store Vector Left/Right Indexed" instructions Ghidra's `altivec.sinc`
does not implement, so every one is an undecodable word that stops the
disassembler's fall-through and truncates the function it sits in. This scans
a decrypted PS3 `EBOOT.elf` offline - no Ghidra, no project lock - and prints
the count per form plus every site address, so the number in
docs/reverse-engineering/toolchain.md#ps3 is re-derivable from the file.

All eight forms share primary opcode 31 and are told apart by the 10-bit
extended opcode in bits 21-30 (Power ISA 2.06 Book I, section 6.7.2):

    lvlx  519   lvlxl  775   lvrx  551   lvrxl  807
    stvlx 647   stvlxl 903   stvrx 679   stvrxl 935

Only sections flagged executable are scanned, one aligned 32-bit word at a
time, so a data-section word that happens to match is never counted. Run
against the extracted Wipeout HD / Fury executable:

    python3 scripts/scan-ps3-cell-vector-ops.py \\
        data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf

Mapping a site to the Ghidra function it truncates needs the live project;
the inline-script recipe for that is on the same toolchain.md page.
"""

import struct
import sys

FORMS = {
    "lvlx": 519,
    "lvlxl": 775,
    "lvrx": 551,
    "lvrxl": 807,
    "stvlx": 647,
    "stvlxl": 903,
    "stvrx": 679,
    "stvrxl": 935,
}
BY_XO = {xo: name for name, xo in FORMS.items()}
SHF_EXECINSTR = 0x4


def executable_sections(elf):
    """Yield (virtual address, file offset, size) for every executable section."""
    if elf[:4] != b"\x7fELF" or elf[4] != 2 or elf[5] != 2:
        sys.exit("not a big-endian ELF64 - is this the decrypted EBOOT.elf?")
    (shoff,) = struct.unpack(">Q", elf[0x28:0x30])
    shentsize, shnum = struct.unpack(">HH", elf[0x3A:0x3E])
    for i in range(shnum):
        header = elf[shoff + i * shentsize :][:40]
        _name, _type, flags, addr, off, size = struct.unpack(">IIQQQQ", header)
        if flags & SHF_EXECINSTR and size:
            yield addr, off, size


def main(path):
    with open(path, "rb") as f:
        elf = f.read()
    counts = {name: 0 for name in FORMS}
    sites = []
    words = 0
    for addr, off, size in executable_sections(elf):
        for p in range(off, off + size - 3, 4):
            (w,) = struct.unpack(">I", elf[p : p + 4])
            words += 1
            if w >> 26 != 31:
                continue
            name = BY_XO.get((w >> 1) & 0x3FF)
            if name is None:
                continue
            counts[name] += 1
            sites.append((addr + p - off, name, w))
    print(f"{words} instruction words across executable sections")
    for name, n in counts.items():
        print(f"{name:7} {n}")
    for site, name, w in sites:
        v_d = (w >> 21) & 0x1F
        r_a = (w >> 16) & 0x1F
        r_b = (w >> 11) & 0x1F
        print(f"{site:08x}\t{name} v{v_d},r{r_a},r{r_b}\t{w:08x}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
