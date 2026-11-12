#!/usr/bin/env python3
"""Per draw, the vertex records of an `hd-draw-dump.py` frame (no HOOK_LIGHT): decoded normals and UV sets.

    uv run --with numpy python3 scripts/hd-draw-vertices.py <dump-dir> <stem> <target-hex> -- [draw-number ...]

Importable: `load(dir, stem)` returns the draw registers, `vertices(mem, draw)` the arrays of one draw.
Attribute word `0x1740 + 4 * a` is `stride << 8 | size << 4 | type`; the scenery layout read here is
position `s16 x3` (type 5), normal `CMP` 11-11-10 (type 6), UV sets `f16 x2` (type 3), colour `u8 x4` (type 4).
"""
import glob, os, struct, sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import rsx_fifo as r
import importlib.util
spec = importlib.util.spec_from_file_location("hook", os.path.join(HERE, "rpcs3_draw_hook.py"))
hook = importlib.util.module_from_spec(spec); spec.loader.exec_module(hook)


def load(out, stem):
    m, wk, draws = hook.draws_of(out, stem)
    return draws


def read(out, stem, addr, n):
    for p in glob.glob("%s/%s-*.bin" % (out, stem)):
        base = int(p.rsplit("-", 1)[1].split(".")[0], 16)
        sz = os.path.getsize(p)
        if base <= addr and addr + n <= base + sz:
            with open(p, "rb") as f:
                f.seek(addr - base); return f.read(n)
    return None


def cmp_normal(w):
    def s(v, bits):
        v &= (1 << bits) - 1
        if v >= 1 << (bits - 1): v -= 1 << bits
        return v / ((1 << (bits - 1)) - 1)
    return np.array([s(w, 11), s(w >> 11, 11), s(w >> 22, 10)], dtype=np.float64)


def vertices(out, stem, d):
    """Return (indices, {attr: array}) for one draw, or None if its streams were not dumped."""
    batches = d.get("batches") or [(d[0x1824] & 0xffffff, ((d[0x1824] >> 24) & 0xff) + 1)]
    ix = []
    for first, cnt in batches:
        blob = read(out, stem, 0x40000000 + d[0x181c] + first * 2, cnt * 2)
        if blob is None: return None
        ix += struct.unpack(">%dH" % cnt, blob)
    lo, hi = min(ix), max(ix)
    attrs = {}
    for a in range(16):
        fmt = d.get(0x1740 + 4 * a); off = d.get(0x1680 + 4 * a)
        if not fmt or not off or (fmt & 0xf) == 0 or ((fmt >> 4) & 0xf) == 0: continue
        stride = (fmt >> 8) & 0xff
        base = (0x40000000 if off >> 31 else 0xC0000000) + (off & 0x7fffffff)
        blob = read(out, stem, base + lo * stride, (hi - lo + 1) * stride)
        if blob is None: return None
        attrs[a] = (fmt, stride, blob)
    return ix, lo, attrs
