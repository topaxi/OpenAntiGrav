#!/usr/bin/env python3
"""Scan a PS3 big-endian ELF's code for D-form loads/stores at given displacements,
or for branches landing on one address, over a raw address range.

Ghidra's function body can stop short of the real function (an inline jump table
defeats its flow), and `search_instructions` then misses whatever sits past that
point. This reads the bytes straight off the program headers instead.

    ps3-dform-scan.py EBOOT.elf 0xe4a30 0xe6060 --disp 0x7001,0x7810
    ps3-dform-scan.py EBOOT.elf 0xe4a30 0xe6060 --target 0xe5938

Used for `docs/ghidra/functions/ps3-hdfury-eu/detonator-mode.md`.
"""

import argparse
import struct

OPS = {
    14: "addi", 32: "lwz", 34: "lbz", 36: "stw", 37: "stwu", 38: "stb",
    39: "stbu", 40: "lhz", 44: "sth", 48: "lfs", 52: "stfs",
}


def segments(data):
    phoff = struct.unpack(">Q", data[0x20:0x28])[0]
    phentsize = struct.unpack(">H", data[0x36:0x38])[0]
    phnum = struct.unpack(">H", data[0x38:0x3A])[0]
    out = []
    for i in range(phnum):
        o = phoff + i * phentsize
        kind, _, off, vaddr, _, filesz, _, _ = struct.unpack(">IIQQQQQQ", data[o : o + 56])
        if kind == 1:
            out.append((vaddr, off, filesz))
    return out


def word(data, segs, va):
    for start, off, size in segs:
        if start <= va < start + size:
            return struct.unpack(">I", data[off + va - start : off + va - start + 4])[0]
    return None


def signed(value, bits):
    return value - (1 << bits) if value & (1 << (bits - 1)) else value


def main():
    p = argparse.ArgumentParser()
    p.add_argument("elf")
    p.add_argument("lo", type=lambda s: int(s, 16))
    p.add_argument("hi", type=lambda s: int(s, 16))
    p.add_argument("--disp", default="")
    p.add_argument("--target", type=lambda s: int(s, 16))
    a = p.parse_args()
    data = open(a.elf, "rb").read()
    segs = segments(data)
    wanted = {int(x, 16) for x in a.disp.split(",") if x}
    for va in range(a.lo, a.hi, 4):
        w = word(data, segs, va)
        if w is None:
            continue
        op = w >> 26
        if wanted and op in OPS:
            disp = signed(w & 0xFFFF, 16)
            if disp in wanted:
                print(f"{va:#x} {OPS[op]} r{(w >> 21) & 31}, {disp:#x}(r{(w >> 16) & 31})")
        if a.target is not None:
            if op == 16 and va + signed(w & 0xFFFC, 16) == a.target:
                print(f"{va:#x} bc -> {a.target:#x}")
            if op == 18 and va + signed(w & 0x3FFFFFC, 26) == a.target:
                print(f"{va:#x} b -> {a.target:#x}")


if __name__ == "__main__":
    main()
