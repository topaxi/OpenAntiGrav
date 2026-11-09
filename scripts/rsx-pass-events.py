#!/usr/bin/env python3
"""One line per primitive of a dumped RSX frame, immediate-mode draws included.

    python3 scripts/rsx-pass-events.py <dir> <stem>

`rsx-draw-list.py` lists array and indexed draws only. A full-screen pass drawn
between `SET_BEGIN_END` calls with immediate vertices has no such method, so the
front end's post passes are missing from it. This walks the same dump and prints
every `SET_BEGIN_END` pair: primitive, vertex count (0 for immediate data), target,
surface format, pitch, clip, fragment program, blend state, blend colour, and the
first three bound textures. See docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md,
"The frame, measured".
"""
import glob
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rsx_fifo as r  # noqa: E402

FACTORS = {0: "0", 1: "1", 0x300: "SC", 0x301: "1-SC", 0x302: "SA", 0x303: "1-SA", 0x304: "DA",
           0x305: "1-DA", 0x306: "DC", 0x307: "1-DC", 0x308: "SAS", 0x8001: "CC", 0x8002: "1-CC",
           0x8003: "CA", 0x8004: "1-CA"}


def factor(value):
    return FACTORS.get(value, hex(value))


def main():
    directory, stem = sys.argv[1:3]
    walk = r.Walk(r.Mem.load(sorted(glob.glob(f"{directory}/{stem}-4*.bin"))))
    regs = {}
    prim = 0
    vertices = 0
    number = 0
    for pos, method, non_incrementing, args in walk.run(0x1000):
        for i, value in enumerate(args):
            regs[method if non_incrementing else method + 4 * i] = value
        if method in (0x1814, 0x1824):
            vertices += sum((a >> 24) + 1 for a in args)
        if method != 0x1808:
            continue
        if args[0]:
            prim, vertices = args[0], 0
            continue
        number += 1
        src, dst = regs.get(0x314, 0), regs.get(0x318, 0)
        print(f"#{number:3d} pos {pos:06x} prim {prim:x} cnt {vertices} tgt {regs.get(0x210, 0):08x}"
              f" surf {regs.get(0x208, 0):05x} p {regs.get(0x20c, 0):x}"
              f" clipH {regs.get(0x200, 0) >> 16} clipV {regs.get(0x204, 0) >> 16}"
              f" fp {regs.get(0x8e4, 0):08x} bl {regs.get(0x310, 0)}"
              f" {factor(src & 0xffff)},{factor(dst & 0xffff)}"
              f" bc {regs.get(0x31c, 0):08x} tex {regs.get(0x1a00, 0):x}"
              f" {regs.get(0x1a20, 0):x} {regs.get(0x1a40, 0):x}")


if __name__ == "__main__":
    main()
