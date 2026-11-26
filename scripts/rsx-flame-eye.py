#!/usr/bin/env python3
"""HD's engine-flame draws in one dumped RSX frame: the live vertex program, and where its eye comes from.

    python3 scripts/rsx-flame-eye.py <dir> <stem> [--program]

`<dir>/<stem>-4*.bin` are the spans `rpcs3_draw_hook.py` writes. The flame draws are picked by
their draw state (target 0xf50000, blend on, colour mask 0x00010101, cull off, alpha func 0x204,
blend dst ONE), as `rsx-flare-shapes.py` picks them. Per draw it prints the distance between
`c465` (the program's `c[209]`, the constant the rim's view vector is taken from) and the camera
centre solved from the view-projection at `c260`-`c263` (the point whose clip x, y and w are all
zero), the distance from the eye to the draw's translation `c259`, the row lengths of the model
matrix `c256`-`c258` and `c464`. With `--program` it also prints the first flame draw's vertex
program, rebuilt from the stream's `TRANSFORM_PROGRAM` uploads (`0x0b80`, load slot `0x1e9c`,
start `0x1ea0`) and rendered with `scripts/ps3-microcode.py`. The program's `c[N]` is `c[256 + N]`.
See `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`, "The live vertex program".
"""
import argparse
import glob
import importlib.util
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, file))
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def vec4(words):
    return struct.unpack(">4f", struct.pack(">4I", *words))


def solve3(a, b):
    """Cramer's rule for a 3x3 system."""
    def det(m):
        return (m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]))
    d = det(a)
    out = []
    for col in range(3):
        m = [row[:] for row in a]
        for r in range(3):
            m[r][col] = b[r]
        out.append(det(m) / d)
    return out


def is_flame(d):
    return (d.get(0x210) == 0xF50000 and d.get(0x310, 0) & 1 and d.get(0x324) == 0x00010101
            and not d.get(0x183C, 0) & 1 and d.get(0x308) == 0x204 and d.get(0x318) == 0x10001)


def programs(directory, stem):
    """The vertex program each draw ran, in draw order, as a list of 4-word instructions."""
    fifo = load("rsx_fifo", "rsx_fifo.py")
    memory = fifo.Mem.load(sorted(glob.glob("%s/%s-4*.bin" % (directory, stem))))
    store, slot, start, out = {}, 0, 0, []
    for position, method, _noinc, args in fifo.Walk(memory).run(0x1000):
        if (memory.word(position) >> 13) & 7:
            continue
        if method == 0x1E9C:
            slot = args[0]
        elif method == 0xB80:
            for i in range(0, len(args) - 3, 4):
                store[slot] = tuple(args[i:i + 4])
                slot += 1
        elif method == 0x1EA0:
            start = args[0] & 0xFFF
        elif method in (0x1814, 0x1824):
            words, s = [], start
            while s in store:
                words.append(store[s])
                s += 1
                if store[s - 1][3] & 1:
                    break
            out.append(words)
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("dir")
    parser.add_argument("stem")
    parser.add_argument("--program", action="store_true")
    args = parser.parse_args()
    draw_list = load("rsx_draw_list", "rsx-draw-list.py")
    draws, _ = draw_list.draws(args.dir, args.stem)
    flames = [d for d in draws if is_flame(d)]
    print("%d flame draws" % len(flames))
    if args.program and flames:
        microcode = load("ps3_microcode", "ps3-microcode.py")
        for i, words in enumerate(programs(args.dir, args.stem)[flames[0]["n"]]):
            print("  %3d %s" % (i, microcode.vp_render(*words)))
    worst = 0.0
    for d in flames:
        c = d["c"]
        if not all(k in c for k in (256, 257, 258, 259, 260, 261, 262, 263, 465)):
            print("draw %4d constants missing" % d["n"])
            continue
        vp = [vec4(c[k]) for k in (260, 261, 262, 263)]
        centre = solve3([[vp[0][i], vp[1][i], vp[2][i]] for i in (0, 1, 3)], [-vp[3][i] for i in (0, 1, 3)])
        eye = vec4(c[465])[:3]
        t = vec4(c[259])[:3]
        gap = sum((eye[k] - centre[k]) ** 2 for k in range(3)) ** 0.5
        worst = max(worst, gap)
        rows = [sum(x * x for x in vec4(c[k])[:3]) ** 0.5 for k in (256, 257, 258)]
        print("draw %4d |c465 - camera| %.5f  |c465 - c259| %8.3f  model rows %s  c464 %s" % (
            d["n"], gap, sum((eye[k] - t[k]) ** 2 for k in range(3)) ** 0.5,
            " ".join("%.3f" % r for r in rows), vec4(c[464]) if 464 in c else None))
    print("largest |c465 - camera|: %.5f" % worst)


if __name__ == "__main__":
    main()
