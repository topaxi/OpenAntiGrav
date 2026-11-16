#!/usr/bin/env python3
"""Census of the big draws in dumped RSX frames, for an emulator limit that only a large count reaches.

    python3 scripts/rsx-large-draws.py [--min N] <dir> [<dir> ...]

Walks every `<dir>/<stem>-4*.bin` set that `rpcs3_draw_hook.py` wrote, the same walk as
`rsx-pass-events.py`, and prints one line per frame: primitives in the frame, the largest count per
primitive type, then every primitive of at least `--min` vertices (default 65,536) with its method
(`arrays` is `0x1814`, `indexed` is `0x1824`). Inline vertices (`0x1818`) are counted in words, an upper
bound on vertices, and named `inline words`. A count sums
every batch between one `SET_BEGIN_END` pair, which is what RPCS3's draw clause sums too. RPCS3's Vulkan
backend draws the primitive types marked `emul` through a generated index buffer
(`docs/reverse-engineering/rpcs3-index-wrap.md`).
"""
import argparse
import glob
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rsx_fifo as r  # noqa: E402

PRIMITIVES = {1: "POINTS", 2: "LINES", 3: "LINE_LOOP", 4: "LINE_STRIP", 5: "TRIANGLES", 6: "TRIANGLE_STRIP",
              7: "TRIANGLE_FAN", 8: "QUADS", 9: "QUAD_STRIP", 10: "POLYGON"}
EMULATED = {3, 8, 10}


def primitives(mem):
    """Every `SET_BEGIN_END` pair of a frame: `(number, primitive, arrays, indexed, inline, target)`."""
    walk = r.Walk(mem)
    regs, out = {}, []
    prim, arrays, indexed, inline = 0, 0, 0, 0
    for pos, method, non_incrementing, args in walk.run(0x1000):
        if (mem.word(pos) >> 13) & 7:
            continue
        for i, value in enumerate(args):
            regs[method if non_incrementing else method + 4 * i] = value
        if method == 0x1814:
            arrays += sum((a >> 24) + 1 for a in args)
        elif method == 0x1824:
            indexed += sum((a >> 24) + 1 for a in args)
        elif method == 0x1818:
            inline += len(args)
        if method != 0x1808:
            continue
        if args[0]:
            prim, arrays, indexed, inline = args[0], 0, 0, 0
        else:
            out.append((len(out) + 1, prim, arrays, indexed, inline, regs.get(0x210, 0)))
    return out, len(walk.missing)


def stems(directory):
    found = {}
    for path in glob.glob(f"{directory}/*-4*.bin"):
        match = re.fullmatch(r"(.+)-4[0-9a-f]{7}\.bin", os.path.basename(path))
        if match:
            found.setdefault(match.group(1), []).append(path)
    return found


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--min", type=int, default=0x10000)
    parser.add_argument("dirs", nargs="+")
    options = parser.parse_args()
    for directory in options.dirs:
        for stem, paths in sorted(stems(directory).items()):
            frame, missing = primitives(r.Mem.load(sorted(paths)))
            peak = {}
            for _, prim, arrays, indexed, inline, _ in frame:
                peak[prim] = max(peak.get(prim, 0), arrays + indexed + inline)
            peaks = " ".join(f"{PRIMITIVES.get(p, hex(p))}={c}" for p, c in sorted(peak.items()))
            print(f"{directory}/{stem}: {len(frame)} primitives, {missing} walk gaps, peak {peaks}")
            for number, prim, arrays, indexed, inline, target in frame:
                count = arrays + indexed + inline
                if count >= options.min:
                    kind = "arrays" if arrays else "indexed" if indexed else "inline words"
                    path = "emul" if prim in EMULATED else "native"
                    print(f"  #{number} {PRIMITIVES.get(prim, hex(prim))} {kind} {count} {path} target {target:08x}")


if __name__ == "__main__":
    main()
