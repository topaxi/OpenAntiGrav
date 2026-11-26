#!/usr/bin/env python3
"""The particle quads of HD frames dumped by `rpcs3-hd-rocket-film.py`, measured in view space.

    scripts/hd-particle-quads.py data/reference/hd-capture/rocket-craft-hit/hit1 [STEM...]

Reads each frame's command ring (`ring/`) and takes every `QUADS` draw whose attribute 0 is a half float
triple at stride 16, the layout of HD's particle vertices (view space, the camera looking down -z;
`docs/ghidra/functions/ps3-hdfury-eu/particle-render.md`). Per frame: live quads, the nearest quad's depth,
how many sit nearer than 1 unit, how many sit exactly on a size cap (half height / depth), and the screens
of fill each blend covers after clipping to the frustum. A total line closes a run over several frames.
"""
import collections
import glob
import json
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rsx_fifo  # noqa: E402

CAPS = (0.2, 0.3, 0.5, 1.0, 2.0)
CAP_TOLERANCE = 0.004


def draws(film, stem):
    """`(blend, quads, quads_not_dumped)` per particle draw; a quad is four vertex tuples."""
    files = {int(p.rsplit("-", 1)[1].split(".")[0], 16): Path(p).read_bytes()
             for p in glob.glob("%s/ring/%s-*.bin" % (film, stem))}

    def read(addr, n):
        for base, blob in files.items():
            if base <= addr and addr + n <= base + len(blob):
                return blob[addr - base:addr - base + n]
        return None

    walk = rsx_fifo.Walk(rsx_fifo.Mem.load(sorted(glob.glob("%s/ring/%s-4*.bin" % (film, stem)))))
    regs, prim, batches = {}, 0, []
    for _pos, method, noninc, args in walk.run(0x1000):
        for i, v in enumerate(args):
            regs[method if noninc else method + 4 * i] = v
        if method == 0x1814:
            batches += [((a & 0xFFFFFF), (a >> 24) + 1) for a in args]
        if method != 0x1808:
            continue
        if args[0]:
            prim, batches = args[0], []
            continue
        fmt, off = regs.get(0x1740, 0), regs.get(0x1680, 0)
        if prim != 8 or not batches or (fmt >> 8 & 0xFF, fmt >> 4 & 0xF, fmt & 0xF) != (16, 3, 3):
            continue
        base = (0x40000000 if off >> 31 else 0xC0000000) + (off & 0x7FFFFFFF)
        quads, lost = [], 0
        for first, count in batches:
            blob = read(base + first * 16, count * 16)
            if blob is None:
                lost += count // 4
                continue
            for q in range(count // 4):
                quads.append([struct.unpack_from(">3e2x4B2e", blob, (q * 4 + k) * 16) for k in range(4)])
        yield ("add" if (regs.get(0x318, 0) & 0xFFFF) == 1 else "over"), quads, lost


def clip(poly, axis, sign):
    out = []
    for i, a in enumerate(poly):
        b = poly[(i + 1) % len(poly)]
        fa, fb = sign * a[axis] - 1, sign * b[axis] - 1
        if fa <= 0:
            out.append(a)
        if (fa <= 0) != (fb <= 0):
            t = fa / (fa - fb)
            out.append((a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])))
    return out


def screens(quad, tan_y, aspect):
    """The share of the screen a view-space quad covers; `0.0` when it reaches the eye plane."""
    if any(v[2] > -0.1 for v in quad):
        return 0.0
    poly = [(v[0] / (-v[2] * tan_y * aspect), v[1] / (-v[2] * tan_y)) for v in quad]
    for axis in (0, 1):
        for sign in (1, -1):
            poly = clip(poly, axis, sign)
            if not poly:
                return 0.0
    area = sum(poly[i][0] * poly[(i + 1) % len(poly)][1] - poly[(i + 1) % len(poly)][0] * poly[i][1]
               for i in range(len(poly)))
    return abs(area) / 2 / 4


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    film = Path(sys.argv[1])
    stems = sys.argv[2:] or sorted(p.stem for p in film.glob("*.json") if p.stem[0] in "ap" and ".clock" not in p.name)
    total = collections.Counter()
    nearest = math.inf
    for stem in stems:
        camera = json.loads((film / ("%s.json" % stem)).read_text())["camera"]
        tan_y = math.tan(math.radians(camera["fov_y_deg"]) / 2)
        tally = collections.Counter()
        fill = {"add": 0.0, "over": 0.0}
        depths = []
        for blend, quads, lost in draws(film, stem):
            tally["lost"] += lost
            for quad in quads:
                sides = (math.dist(quad[0][:3], quad[1][:3]), math.dist(quad[1][:3], quad[2][:3]))
                if min(sides) == 0 or not quad[0][6]:
                    continue
                depth = -sum(v[2] for v in quad) / 4
                tally["live"] += 1
                fill[blend] += screens(quad, tan_y, camera["aspect"])
                depths.append(depth)
                tally["under_1"] += 0 < depth < 0.999
                for cap in CAPS:
                    if depth > 0 and abs(min(sides) / 2 / depth - cap) < cap * CAP_TOLERANCE:
                        tally["cap %s" % cap] += 1
        if not depths:
            print("%-6s no particle quads in the dump" % stem)
            continue
        nearest = min(nearest, min(depths))
        total.update(tally)
        print("%-6s live %5d nearest %.3f under 1: %d | on a cap %s | screens add %.1f over %.1f%s" % (
            stem, tally["live"], min(depths), tally["under_1"],
            {c: tally["cap %s" % c] for c in CAPS if tally["cap %s" % c]}, fill["add"], fill["over"],
            " | %d quads not dumped" % tally["lost"] if tally["lost"] else ""))
    if total["live"]:
        print("total  live %5d nearest %.3f under 1: %d | on a cap %s" % (
            total["live"], nearest, total["under_1"], {c: total["cap %s" % c] for c in CAPS if total["cap %s" % c]}))


if __name__ == "__main__":
    main()
