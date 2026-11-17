#!/usr/bin/env python3
"""Read HD's planar sea reflection off one `hd-draw-dump.py` frame (HOOK_LIGHT=1 HOOK_NO_HEADS=1 HOOK_FP_LEN=0x900).

    hd-sea-reflect-read.py <dump-dir> <stem> <rcsmaterial-dir>

`<rcsmaterial-dir>` holds the circuit's extracted `materials/*.rcsmaterial` (`psarc.py extract`); the draws are named
by `rsx_fp_match.py`, so the sea is `reflectplane_dc_seawater#N`. Prints, in order:

* every sea draw, and the render target its `screenSpaceReflectionTex` unit (the one whose program reads a unit bound
  to a surface the frame draws into) points at;
* that target's surface registers (format word, pitch, clip, viewport, clear value and mask), its draw count, and
  how many main-view draws bind it against how many sample it;
* the mirror: the target's view-projection (`c[260..263]` of its draws) against the main one (`c[256..259]` of a
  sea draw), fitted as `M_target = Flip(h) M_main`, with `h` and the residual;
* the sea program's patched constants: the projection rows it multiplies by (`reflectProject`) against the main
  matrix, and the `distortion` scale it multiplies the normal by.

A fragment program's address is reused, so a constant is read only off a draw whose program disassembles to the sea.
"""
import glob
import importlib.util
import io
import contextlib
import re
import struct
import subprocess
import sys
from collections import Counter
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


rdl = load("rsx_draw_list", "rsx-draw-list.py")
mc = load("ps3_microcode", "ps3-microcode.py")


def fl(w):
    return struct.unpack(">f", struct.pack(">I", w))[0]


def fp_text(dump, stem, draw):
    fp = draw.get(0x8E4, 0) & 0xFFFFFFF0
    paths = glob.glob("%s/%s-c%07x.bin" % (dump, stem, fp))
    if not paths:
        return ""
    raw = open(paths[0], "rb").read() + bytes(64)
    out = io.StringIO()
    try:
        with contextlib.redirect_stdout(out):
            mc.fp_code(raw, 0, len(raw), {})
    except Exception:
        pass
    return out.getvalue()


def matrix(draw, base):
    return np.array([[fl(w) for w in draw["c"][base + i]] for i in range(4)])


def main():
    dump, stem, mats = sys.argv[1:4]
    draws, _events = rdl.draws(dump, stem)
    names = {}
    out = subprocess.run([sys.executable, str(HERE / "rsx_fp_match.py"), dump, stem, mats], capture_output=True, text=True).stdout
    for line in out.splitlines():
        p = line.split()
        if p and p[0].isdigit():
            names[int(p[0])] = p[-1]
    sea = [d for d in draws if "reflectplane_dc_seawater#" in names.get(d["n"], "")]
    print("draws %d, sea draws %d: %s" % (len(draws), len(sea), [d["n"] for d in sea]))
    if not sea:
        print("no sea draw in the frame")
        return
    sea_by_target = Counter()
    for d in sea:
        text = fp_text(dump, stem, d)
        read = {int(m.group(1)) for m in re.finditer(r"unit(\d+)", text)}
        for u in sorted(read):
            off = d.get(0x1A00 + 0x20 * u, 0)
            if off and any(x.get(0x210, 0) == off for x in draws):
                sea_by_target[off] += 1
    print("sea draws reading a rendered surface:", {"%08x" % k: v for k, v in sea_by_target.items()})
    for d in sea:
        units = [(u, d.get(0x1A00 + 0x20 * u, 0)) for u in range(16) if d.get(0x1A00 + 0x20 * u + 0xC, 0) & 0x80000000]
        print("  sea draw %d fp %08x units %s" % (d["n"], d.get(0x8E4, 0), " ".join("%d:%08x" % x for x in units)))
    if not sea_by_target:
        return
    tgt = sea_by_target.most_common(1)[0][0]
    members = [d for d in draws if d.get(0x210, 0) == tgt]
    first = members[0]
    print("\ntarget %08x: %d draws" % (tgt, len(members)))
    print("  surface format %08x pitch %08x clip %08x/%08x viewport %08x/%08x offset %g,%g scale %g,%g" % (
        first.get(0x208, 0), first.get(0x20C, 0), first.get(0x200, 0), first.get(0x204, 0), first.get(0xA00, 0), first.get(0xA04, 0),
        fl(first.get(0xA20, 0)), fl(first.get(0xA24, 0)), fl(first.get(0xA30, 0)), fl(first.get(0xA34, 0))))
    print("  depth surface %08x pitch %08x; clear colour %08x, depth/stencil %08x, mask %08x; colour mask %08x, depth test %d func %x mask %d, cull %d" % (
        first.get(0x214, 0), first.get(0x21C, 0), first.get(0x1D90, 0), first.get(0x1D8C, 0), first.get(0x1D94, 0),
        first.get(0x324, 0), first.get(0xA74, 0) & 1, first.get(0xA6C, 0), first.get(0xA70, 0) & 1, first.get(0x183C, 0) & 1))
    main = sea[0]
    print("  main view: surface %08x clip %08x/%08x" % (main.get(0x210, 0), main.get(0x200, 0), main.get(0x204, 0)))
    bound = sampled = shared = 0
    last_use = {}
    for d in draws:
        last_use[d.get(0x8E4, 0) & 0xFFFFFFF0] = d["n"]
    for d in draws:
        if d.get(0x210, 0) == tgt:
            continue
        us = [u for u in range(16) if d.get(0x1A00 + 0x20 * u + 0xC, 0) & 0x80000000 and d.get(0x1A00 + 0x20 * u, 0) == tgt]
        if not us:
            continue
        bound += 1
        fp = d.get(0x8E4, 0) & 0xFFFFFFF0
        reused = last_use[fp] > d["n"]
        read = {int(m.group(1)) for m in re.finditer(r"unit(\d+)", fp_text(dump, stem, d))}
        if reused:
            shared += 1
        if any(u in read for u in us):
            sampled += 1
            print("    samples it: draw %d %s" % (d["n"], names.get(d["n"], "?")))
    print("  main-view draws binding it: %d; sampling it: %d; of the %d that do not sample it, %d have a program address a later draw reuses "
          "(the bytes read are the later draw's, so for those 'never reads unit 3' is not established)" % (bound, sampled, bound - sampled, shared))
    t_names = Counter(names.get(d["n"], "?").split("#")[0] for d in members)
    print("  materials drawn into it (fp-matched, a match lists the families that share a program):", dict(t_names.most_common(8)))

    m_main = matrix(main, 256)
    seen_m = Counter()
    for d in members:
        if all(260 + i in d["c"] for i in range(4)):
            seen_m[tuple(np.round(matrix(d, 260), 4).flatten())] += 1
    print("distinct c[260..263] over the target's draws (count): %s" % sorted(seen_m.values(), reverse=True))
    m_tgt = np.array(seen_m.most_common(1)[0][0]).reshape(4, 4)
    print("\nmain view-projection c[256..259] of a sea draw:\n", np.round(m_main, 5))
    print("target view-projection c[260..263] of a target draw:\n", np.round(m_tgt, 5))
    print("main sea draw's own c[260..263] equals the target's:", np.allclose(matrix(main, 260), m_tgt, atol=1e-3))
    flip_rows = np.allclose(m_tgt[[0, 2]], m_main[[0, 2]], atol=1e-4) and np.allclose(m_tgt[1], -m_main[1], atol=1e-4)
    print("rows x and z equal, row y negated:", flip_rows)
    delta = m_tgt[3] - m_main[3]
    two_h = float(delta @ m_main[1] / (m_main[1] @ m_main[1]))
    resid = float(np.abs(delta - two_h * m_main[1]).max())
    print("row 3 difference = K * main row y, least squares K = %.4f (max residual %.5f)" % (two_h, resid))
    print("  the mirror is y' = K - y: plane y = K / 2 = %.4f" % (two_h / 2))

    print("\nsea programs (patched constants; the rows of the main view-projection and of the target's are looked for among them):")
    seen = set()
    rows = {"main": m_main, "target": m_tgt}
    for d in sea:
        fp = d.get(0x8E4, 0) & 0xFFFFFFF0
        if fp in seen:
            continue
        seen.add(fp)
        text = fp_text(dump, stem, d)
        vecs = [[float(v) for v in m.group(1).split(",")] for m in re.finditer(r"\{([-0-9.e, ]+)\}", text) if len(m.group(1).split(",")) == 4]
        found = []
        for name, mat in rows.items():
            for r in range(4):
                if any(np.allclose(sorted(v), sorted(mat[r]), rtol=2e-3, atol=2e-3) for v in vecs):
                    found.append("%s row %d" % (name, r))
        dist = sorted({v[2] for v in vecs if v[0] == 0 and v[1] == 0 and v[3] == 0 and v[2] not in (1.0, 0.0)})
        print("  draw %d fp %08x: %d instructions; rows found %s; {0,0,K,0} constants %s" % (
            d["n"], d.get(0x8E4, 0), len([1 for l in text.splitlines() if l.startswith("@")]), found, dist))


if __name__ == "__main__":
    main()
