#!/usr/bin/env python3
"""Recompute each `scripts/psp-zone-hover.py` row's output from its recorded inputs, in `f32`.

    uv run --with numpy python3 -I scripts/psp-zone-hover-check.py data/reference/zone-hover/*.jsonl

Prints, per phase, the row count and the largest difference between the original's output and
`Ship_HoverFourCorner`'s law as written on
`docs/ghidra/functions/psp-pulse-usa/zone-hover.md`.
"""
import json, sys
import numpy as np
F = np.float32
def v(a): return np.array(a[:3], dtype=F)
def cross(a, b): return np.array([a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]], dtype=F)
rows = [json.loads(l) for f in sys.argv[1:] for l in open(f)]
stats = {}
for r in rows:
    p = r["phase"]
    if p == "spring":
        up = v(r["up"]); n = v(r["hit_normal"])
        # probe point = basis * offset + pos
        B = np.array(r["basis"], dtype=F).reshape(4, 4)
        off = np.array(r["offset"], dtype=F)
        pt = (B[0]*off[0] + B[1]*off[1] + B[2]*off[2] + B[3]*off[3])[:3] + v(r["pos"])
        h = F(np.dot(v(r["point"]) - v(r["hit_point"]), up))
        vn = F(np.dot(v(r["point_vel"]), n))
        d = F(vn * F(-0.1))
        d = max(d, F(-1.0)); d = F(0.8) if d > 1 else F(d * F(0.8))
        s = F(F(r["target"]) - F(r["h"])); s = F(s*F(r["K"])); s = F(s*F(r["grav"])); s = F(s*F(0.15)); s = F(s*F(r["mass"]))
        s = F(s * F(d + F(1.0)))
        f = up * s
        err = np.abs(f - v(r["force"])).max()
        e_pt = np.abs(pt - v(r["point"])).max()
        stats.setdefault(p, []).append((err, e_pt, abs(h - r["h"]), r["probe"], r["offset"][:3], vn, r["h"], r["grounded"]))
    elif p == "torque":
        up = v(r["up"]); n = v(r["normal"])
        t = cross(up, n) * F(-400.0)
        right = v(r["right"])
        stats.setdefault(p, []).append((np.abs(t - v(r["torque"])).max(), float(np.dot(v(r["torque"]), right)), r["contacts"], float(np.linalg.norm(n)), r["inv"]))
    elif p == "downforce":
        n = v(r["normal"]); k = F(-(F(r["track_gravity"]) * F(r["mass"])))
        f = n * k
        stats.setdefault(p, []).append((np.abs(f - v(r["force"])).max(), r["contacts"], r["grounded"], r["maglock"], r["track_gravity"], r["mass"]))
    elif p == "snap":
        n = v(r["hit_normal"]); d = F(np.dot(v(r["pos"]) - v(r["hit_point"]), n))
        m = n * F(F(3.0) - d); m = m * F(0.25)
        stats.setdefault(p, []).append((np.abs(m - np.array(r["move"], dtype=F)).max(), float(d), r["d"]))
for p, s in stats.items():
    print(p, len(s), "max err", max(x[0] for x in s))
    for x in s[:: max(1, len(s)//8)]:
        print("   ", x)
