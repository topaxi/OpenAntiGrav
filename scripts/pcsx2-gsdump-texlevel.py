#!/usr/bin/env python3
"""Which texture level can the GS sample in a PCSX2 `.gs` dump (`zstd -d`'d)?

    scripts/pcsx2-gsdump-texlevel.py frame.gs [--draws]

Reports, for every textured primitive, the `TEX1` of the drawing context it uses (`MXL`, `LCM`, `L`, `K`,
`MMAG`, `MMIN`, `MTBA`), every `TEX1`/`MIPTBP1`/`MIPTBP2` write in the frame, how many primitives are *far*
(2 or more texels per screen pixel, where a mipmapping sampler would take level 1 or higher), and every image
upload, split into those a primitive reads as `TEX0` base or CLUT and those nothing reads. `--draws` prints one
line per textured state group.

`docs/rendering/texture-level-per-title.md`, "Pulse PS2 in a race", records what it found. The dump format and the packet walk are `pcsx2-gsdump.py`'s.
"""
import argparse
import collections
import importlib.util
import math
import sys
from pathlib import Path

spec = importlib.util.spec_from_file_location("pcsx2_gsdump", Path(__file__).resolve().parent / "pcsx2-gsdump.py")
gsdump = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gsdump)

MIP_REGISTERS = (0x34, 0x35, 0x36, 0x37)
FAR_TEXELS_PER_PIXEL = 2.0


def tex1_fields(value):
    """`TEX1`'s fields, `K` as the signed 7.4 fixed-point number the GS reads."""
    k = (value >> 32) & 0xFFF
    if k & 0x800:
        k -= 0x1000
    return {"LCM": value & 1, "MXL": (value >> 2) & 7, "MMAG": (value >> 5) & 1, "MMIN": (value >> 6) & 7,
            "MTBA": (value >> 9) & 1, "L": (value >> 19) & 3, "K": k / 16.0}


def show_tex1(value):
    if value is None:
        return "TEX1 never written"
    f = tex1_fields(value)
    return "TEX1 %#014x LCM %d MXL %d MMAG %d MMIN %d MTBA %d L %d K %g" % (
        value, f["LCM"], f["MXL"], f["MMAG"], f["MMIN"], f["MTBA"], f["L"], f["K"])


class Census(gsdump.GS):
    """`GS` that also keeps each primitive's own vertices, context and registers."""

    def __init__(self):
        super().__init__()
        self.primitives = []
        self.writes = collections.Counter()
        self.queue = []
        self.bound = set()

    def setreg(self, address, value):
        if address == 0x00:
            self.queue = []
        if address in (0x14, 0x15) + MIP_REGISTERS:
            self.writes[(address, value)] += 1
        if address in (0x06, 0x07):
            self.bound.update((value & 0x3FFF, (value >> 37) & 0x3FFF))
        super().setreg(address, value)

    def vertex(self, x, y, z, adc):
        kind = self.prim & 7
        self.queue.append((x / 16.0, y / 16.0, self.st, self.q, self.uv))
        drawn = None
        if kind == 3 and len(self.queue) == 3:
            drawn, self.queue = self.queue, []
        elif kind == 4 and len(self.queue) >= 3:
            drawn = self.queue[-3:]
            self.queue = self.queue[-2:]
        elif kind == 5 and len(self.queue) >= 3:
            drawn = [self.queue[0]] + self.queue[-2:]
            self.queue = [self.queue[0], self.queue[-1]]
        elif kind == 6 and len(self.queue) == 2:
            drawn, self.queue = self.queue, []
        elif kind in (0, 1) and len(self.queue) == kind + 1:
            drawn, self.queue = self.queue, []
        elif kind == 2 and len(self.queue) >= 2:
            drawn = self.queue[-2:]
            self.queue = self.queue[-1:]
        if drawn is None or adc:
            return
        # PRMODECONT.AC = 0 takes the attributes from PRMODE instead of PRIM
        attributes = self.prim if self.r.get(0x1A, 1) & 1 else self.r.get(0x1B, 0)
        context = (attributes >> 9) & 1
        self.primitives.append(dict(
            kind=kind, textured=(attributes >> 4) & 1, fst=(attributes >> 8) & 1, context=context,
            tex0=self.r.get(0x06 + context), tex1=self.r.get(0x14 + context),
            frame=self.r.get(0x4C + context), offset=self.r.get(0x18 + context, 0),
            scissor=self.r.get(0x40 + context), alpha=self.r.get(0x42 + context),
            miptbp1=self.r.get(0x34 + context), vertices=drawn, order=len(self.primitives)))


def texels_per_pixel(p):
    """Linear texel density of one triangle or sprite, or None when it covers under a pixel on screen."""
    if p["tex0"] is None or p["kind"] not in (3, 4, 5, 6):
        return None
    width, height = 1 << ((p["tex0"] >> 26) & 15), 1 << ((p["tex0"] >> 30) & 15)
    points = []
    for x, y, st, q, uv in p["vertices"]:
        if p["fst"]:
            points.append((x, y, uv[0] / 16.0, uv[1] / 16.0))
        else:
            q = q or 1.0
            points.append((x, y, st[0] / q * width, st[1] / q * height))
    if p["kind"] == 6:
        screen = abs((points[1][0] - points[0][0]) * (points[1][1] - points[0][1]))
        texels = abs((points[1][2] - points[0][2]) * (points[1][3] - points[0][3]))
    else:
        a, b, c = points
        screen = abs((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])) / 2
        texels = abs((b[2] - a[2]) * (c[3] - a[3]) - (c[2] - a[2]) * (b[3] - a[3])) / 2
    if screen < 1.0 or not math.isfinite(texels):
        return None
    return math.sqrt(texels / screen)


def on_screen(p):
    """Whether the primitive's centre is inside its scissor, so it really rasterises there."""
    if p["scissor"] is None:
        return True
    ox, oy = (p["offset"] & 0xFFFF) / 16.0, ((p["offset"] >> 32) & 0xFFFF) / 16.0
    cx = sum(v[0] for v in p["vertices"]) / len(p["vertices"]) - ox
    cy = sum(v[1] for v in p["vertices"]) / len(p["vertices"]) - oy
    s = p["scissor"]
    return (s & 0x7FF) <= cx <= ((s >> 16) & 0x7FF) and ((s >> 32) & 0x7FF) <= cy <= ((s >> 48) & 0x7FF)


def census(path):
    state = Census()
    # `gsdump.run` builds its own `GS`; borrow its walk with ours substituted
    original, gsdump.GS = gsdump.GS, lambda: state
    try:
        gsdump.run(path)
    finally:
        gsdump.GS = original
    return state


def report(state, per_draw):
    textured = [p for p in state.primitives if p["textured"]]
    print("primitives %d, textured %d" % (len(state.primitives), len(textured)))
    print("textured by context: %s" % dict(collections.Counter(p["context"] + 1 for p in textured)))

    print("\nTEX1 as each textured primitive reads it:")
    far = []
    for p in textured:
        density = texels_per_pixel(p)
        p["density"] = density
        if density is not None and density >= FAR_TEXELS_PER_PIXEL and on_screen(p):
            far.append(p)
    by_value = collections.Counter(p["tex1"] for p in textured)
    far_by_value = collections.Counter(p["tex1"] for p in far)
    for value, count in by_value.most_common():
        print("  %-84s prims %6d  far %6d" % (show_tex1(value), count, far_by_value[value]))

    print("\nfar primitives (>= %g texels/pixel, centre inside the scissor): %d" % (FAR_TEXELS_PER_PIXEL, len(far)))
    bins = collections.Counter()
    for p in far:
        bins[min(int(math.log2(p["density"])), 6)] += 1
    print("  by the level a mipmapping sampler would take (log2 of the density, 6 means 6 or more): %s"
          % dict(sorted(bins.items())))
    print("  distinct textures (TBP0/PSM/size) among them: %d" % len({p["tex0"] & 0x3FFFFFFFF for p in far}))
    print("  with MXL > 0: %d" % sum(1 for p in far if p["tex1"] is not None and tex1_fields(p["tex1"])["MXL"]))
    print("textured primitives with MXL > 0: %d, with TEX1 never written: %d" % (
        sum(1 for p in textured if p["tex1"] is not None and tex1_fields(p["tex1"])["MXL"]),
        sum(1 for p in textured if p["tex1"] is None)))

    print("\nTEX1 / MIPTBP register writes in the frame:")
    names = gsdump.REGNAME
    totals = collections.Counter()
    for (address, _), count in state.writes.items():
        totals[names[address]] += count
    print("  totals: %s" % dict(totals))
    for address in (0x14, 0x15):
        for value, count in sorted((v, c) for (a, v), c in state.writes.items() if a == address):
            print("  %-9s x%-5d %s" % (names[address], count, show_tex1(value)))
    for address in MIP_REGISTERS:
        values = [v for a, v in state.writes if a == address]
        if values:
            print("  %-9s %d distinct values" % (names[address], len(values)))
    mip_pointers(textured)
    uploads(state, textured)

    if per_draw:
        draws(textured)


def mip_pointers(textured):
    """Where `MIPTBP1`'s three level addresses point, against the same primitive's `TEX0`."""
    bound = [p for p in textured if p["miptbp1"] is not None and p["tex0"] is not None]
    at_clut = at_base = 0
    for p in bound:
        base, clut = p["tex0"] & 0x3FFF, (p["tex0"] >> 37) & 0x3FFF
        at_clut += (p["miptbp1"] & 0x3FFF) == clut
        at_base += ((p["miptbp1"] >> 40) & 0x3FFF) == base
    print("  MIPTBP1 under %d textured primitives: level 1 points at their own CLUT on %d, level 3 at their own "
          "base on %d" % (len(bound), at_clut, at_base))


def uploads(state, textured):
    """Every image upload, and whether some primitive after it reads that base or CLUT address."""
    bases = {p["tex0"] & 0x3FFF for p in textured if p["tex0"] is not None}
    palettes = {(p["tex0"] >> 37) & 0x3FFF for p in textured if p["tex0"] is not None}
    seen = collections.Counter()
    for u in state.uploads:
        if u["bitblt"] is None or u["trxreg"] is None:
            continue
        key = ((u["bitblt"] >> 32) & 0x3FFF, (u["bitblt"] >> 56) & 0x3F,
               u["trxreg"] & 0xFFF, (u["trxreg"] >> 32) & 0xFFF)
        seen[key] += 1
    other = [k for k in seen if k[0] not in bases and k[0] not in palettes]
    print("\nimage uploads %d to %d distinct (DBP, DPSM, w, h): base of a drawn TEX0 %d, CLUT %d, neither %d" % (
        len(state.uploads), len(seen), sum(1 for k in seen if k[0] in bases),
        sum(1 for k in seen if k[0] in palettes and k[0] not in bases), len(other)))
    unbound = sorted(k for k in other if k[0] not in state.bound)
    if other:
        print("  of the %d neither: named as TBP0 or CBP by a TEX0 write whose batch drew nothing %d, never named %d"
              % (len(other), len(other) - len(unbound), len(unbound)))
    for k in unbound:
        print("  never bound: dbp %#06x psm %#04x %dx%d x%d" % (k + (seen[k],)))


def draws(textured):
    """One line per run of textured primitives under one context, TEX0 and TEX1."""
    print("\nper draw:")
    groups = []
    for p in textured:
        key = (p["context"], p["tex0"], p["tex1"], p["frame"], p["alpha"], p["kind"])
        if not groups or groups[-1][0] != key:
            groups.append((key, []))
        groups[-1][1].append(p)
    far_groups = 0
    for i, ((context, tex0, tex1, frame, alpha, kind), prims) in enumerate(groups):
        densities = sorted(p["density"] for p in prims if p["density"] is not None and on_screen(p))
        n_far = sum(1 for d in densities if d >= FAR_TEXELS_PER_PIXEL)
        far_groups += bool(n_far)
        f = tex1_fields(tex1) if tex1 is not None else None
        print("%4d ctx %d fbp %#05x prim %d x%-5d tbp %#06x psm %#04x %4dx%-4d %s max density %s far %d" % (
            i, context + 1, (frame or 0) & 0x1FF, kind, len(prims), tex0 & 0x3FFF, (tex0 >> 20) & 63,
            1 << ((tex0 >> 26) & 15), 1 << ((tex0 >> 30) & 15),
            "MXL %d LCM %d L %d K %g MMAG %d MMIN %d" % (f["MXL"], f["LCM"], f["L"], f["K"], f["MMAG"], f["MMIN"])
            if f else "TEX1 never written", "%.1f" % densities[-1] if densities else "-", n_far))
    print("draws %d, of them with a far primitive %d" % (len(groups), far_groups))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("dump")
    ap.add_argument("--draws", action="store_true", help="one line per textured state group")
    args = ap.parse_args()
    report(census(args.dump), args.draws)


if __name__ == "__main__":
    sys.exit(main())
