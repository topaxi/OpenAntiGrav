#!/usr/bin/env python3
"""Which way up HD's particle quads sample their sprites, and which flipbook cell each one reads.

    scripts/hd-particle-uv.py --gtf DIR [--textures textures.json] FILM...

`FILM` is a capture of `rpcs3-hd-particle-ring.py` (or of `rpcs3-hd-rocket-film.py` with `--textures`
naming the `tex/textures.json` of a capture of the same save state: the heap does not move). `DIR` holds
the disc's `/data/psys/tex/*.gtf` (`scripts/psarc.py extract`), searched for each dumped mip's bytes.

Per particle quad of every dumped ring it reports, summed per texture and per frame:

- `edge`: quads whose `u` and `v` axes are within about 6 degrees of parallel on screen, where the sign
  below is noise; they are left out of it.
- `hand`: the sign of the determinant of `d(view xy) / d(uv)`. `+` is `u` rightward with `v` upward
  (stored row 0 on the bottom edge) up to a roll; `-` is a mirrored picture.
- `up`: for quads with no roll (`u` along view x), whether `v` grows upward.
- `cell`: column and row counted from `v = 0` (stored rows), from the quad's own `uv` span, with the
  frame index that cell has counted from the authored top (`top`, a `.gtf`'s last stored row) and from
  the first stored row (`mem`).
- `head`: for stretched quads whose long axis carries `v`, whether the end with the larger `v` is the one
  farther from the median centre of that texture's quads in the frame (the leading end of a streak flying out of a burst).
"""
import argparse
import collections
import glob
import json
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rsx_fifo  # noqa: E402


def particle_quads(film, stem):
    """`(texture offset, [four (x, y, z, r, g, b, a, u, v)])` of every particle quad of one ring."""
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
        for first, count in batches:
            blob = read(base + first * 16, count * 16)
            if blob is None:
                continue
            for q in range(count // 4):
                yield regs.get(0x1A00, 0), [struct.unpack_from(">3e2x4B2e", blob, (q * 4 + k) * 16) for k in range(4)]


#: `NV4097_SET_SURFACE_*`: the render target a draw goes to.
SURFACE = (("clip_h", 0x200), ("clip_v", 0x204), ("format", 0x208), ("color_a", 0x210), ("zeta", 0x214))


def surfaces(film, stem):
    """`{(is particle quads, surface registers): draws}` over every draw of one dumped ring."""
    walk = rsx_fifo.Walk(rsx_fifo.Mem.load(sorted(glob.glob("%s/ring/%s-4*.bin" % (film, stem)))))
    regs, found = {}, collections.Counter()
    for _pos, method, noninc, args in walk.run(0x1000):
        for i, v in enumerate(args):
            regs[method if noninc else method + 4 * i] = v
        if method != 0x1808 or not args[0]:
            continue
        fmt = regs.get(0x1740, 0)
        particle = args[0] == 8 and (fmt >> 8 & 0xFF, fmt >> 4 & 0xF, fmt & 0xF) == (16, 3, 3)
        found[(particle, " ".join("%s=%08x" % (n, regs.get(m, 0)) for n, m in SURFACE))] += 1
    return found


def names(index_path, gtf_dir):
    """Texture offset (as `%08x`) to the `.gtf` stem whose payload holds the dumped bytes."""
    gtfs = {p.stem: p.read_bytes() for p in Path(gtf_dir).rglob("*.gtf")}
    found = {}
    for key in json.loads(index_path.read_text()):
        blob = (index_path.parent / (key + ".bin")).read_bytes()
        hits = sorted(stem for stem, data in gtfs.items() if blob in data)
        found[key] = "|".join(hits) if hits else "unmatched:" + key
    return found


def solve(quad):
    """`d(x, y) / d(u, v)` as `((xu, xv), (yu, yv))`, or `None` for a degenerate quad."""
    p0, p1, p2 = quad[0], quad[1], quad[2]
    du1, dv1 = p1[7] - p0[7], p1[8] - p0[8]
    du2, dv2 = p2[7] - p0[7], p2[8] - p0[8]
    det = du1 * dv2 - du2 * dv1
    if abs(det) < 1e-9:
        return None
    rows = []
    for axis in (0, 1):
        d1, d2 = p1[axis] - p0[axis], p2[axis] - p0[axis]
        rows.append(((d1 * dv2 - d2 * dv1) / det, (d2 * du1 - d1 * du2) / det))
    return rows


def describe(quad, centre):
    jac = solve(quad)
    if jac is None:
        return None
    (xu, xv), (yu, yv) = jac
    us, vs = [c[7] for c in quad], [c[8] for c in quad]
    du, dv = max(us) - min(us), max(vs) - min(vs)
    if du <= 1e-4 or dv <= 1e-4 or du > 1.01 or dv > 1.01:
        return None
    columns, rows = max(round(1 / du), 1), max(round(1 / dv), 1)
    column, row = int((min(us) + du / 2) * columns), int((min(vs) + dv / 2) * rows)
    area = xu * yv - xv * yu
    edge = abs(area) < 0.1 * math.hypot(xu, yu) * math.hypot(xv, yv)
    out = {"hand": "edge" if edge else "+" if area > 0 else "-", "grid": (columns, rows), "cell": (column, row),
           "alpha": quad[0][6], "colour": tuple(quad[0][3:7])}
    along_u = math.hypot(xu, yu) * du
    along_v = math.hypot(xv, yv) * dv
    if abs(yu) * du < 0.02 * along_u and abs(xv) * dv < 0.02 * along_v:
        out["up"] = "v-up" if yv > 0 else "v-down"
    if along_v > 1.5 * along_u and centre is not None:
        low = [c for c in quad if c[8] < min(vs) + dv / 2]
        high = [c for c in quad if c[8] > min(vs) + dv / 2]
        if low and high:
            far = lambda cs: math.dist([sum(c[a] for c in cs) / len(cs) for a in range(3)], centre)  # noqa: E731
            if abs(far(high) - far(low)) > 0.3 * along_v:
                out["head"] = "high-v" if far(high) > far(low) else "low-v"
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("films", nargs="+", type=Path)
    ap.add_argument("--gtf", required=True)
    ap.add_argument("--textures", type=Path, help="textures.json of another capture, for a film with no tex/")
    ap.add_argument("--per-frame", action="store_true", help="print each texture's cells per frame")
    ap.add_argument("--surfaces", action="store_true",
                    help="print each ring's render targets, particle draws against all others, and stop")
    args = ap.parse_args()
    if args.surfaces:
        for film in args.films:
            for stem in sorted({p.name.split("-")[0] for p in (film / "ring").glob("*.bin")}):
                for (particle, regs), draws in surfaces(film, stem).most_common():
                    print("%s %s %-8s x%-4d %s" % (film.name, stem, "particle" if particle else "other", draws, regs))
        return
    totals = collections.defaultdict(collections.Counter)
    for film in args.films:
        index = args.textures or film / "tex" / "textures.json"
        table = names(index, args.gtf)
        stems = sorted({p.name.split("-")[0] for p in (film / "ring").glob("*.bin")},
                       key=lambda s: (s != "pre", s))
        for stem in stems:
            quads = [(t, q) for t, q in particle_quads(film, stem) if q[0][6]]
            if not quads:
                continue
            grouped = collections.defaultdict(list)
            for offset, quad in quads:
                grouped[offset].append([sum(c[a] for c in quad) / 4 for a in range(3)])
            median = {offset: [sorted(c[a] for c in cs)[len(cs) // 2] for a in range(3)]
                      for offset, cs in grouped.items()}
            frame = collections.defaultdict(list)
            for offset, quad in quads:
                key = "%08x" % offset
                name = table.get(key, "unmatched:" + key)
                got = describe(quad, median[offset])
                if got is None:
                    totals[name]["degenerate"] += 1
                    continue
                totals[name]["quads"] += 1
                totals[name]["edge" if got["hand"] == "edge" else "hand " + got["hand"]] += 1
                if "up" in got:
                    totals[name][got["up"]] += 1
                if "head" in got:
                    totals[name]["head " + got["head"]] += 1
                frame[name].append(got)
            if args.per_frame:
                for name, gots in sorted(frame.items()):
                    columns, rows = gots[0]["grid"]
                    if columns * rows == 1:
                        continue
                    top = sorted((rows - 1 - g["cell"][1]) * columns + g["cell"][0] for g in gots)
                    mem = sorted(g["cell"][1] * columns + g["cell"][0] for g in gots)
                    print("%s %s %s %dx%d n=%d top=%s mem=%s" % (
                        film.name, stem, name, columns, rows, len(gots), top[:24], mem[:24]))
    print("%-48s %s" % ("texture", "counts"))
    for name, counter in sorted(totals.items()):
        print("%-48s %s" % (name, " ".join("%s=%d" % kv for kv in sorted(counter.items()))))


if __name__ == "__main__":
    main()
