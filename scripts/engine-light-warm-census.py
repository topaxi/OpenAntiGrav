#!/usr/bin/env python3
"""Count warm-lit pixels on the grid craft of a 1920x1080 Omega frame: R > 1.35 * max(G, B) and R > 50, in the craft
region minus the flame box. One line per glob: the count's distribution over the frames, and the summed R excess.

    python3 scripts/engine-light-warm-census.py 'frames/a-shipped-*.png' 'frames/b-suppressed-*.png' ours.png

The boxes fit the Modesto Heights grid pair (Feisar_c1, chase camera); `docs/ghidra/functions/ps4-omega-eu/engine-light.md`,
"Live check", has the original's numbers.
"""
import sys, glob
import numpy as np
from PIL import Image
X0, Y0, X1, Y1 = 640, 560, 1280, 960
FLAME = (900, 840, 1020, 1000)  # the nozzle flame and its floor glow, full-frame px
def census(path):
    a = np.asarray(Image.open(path).convert("RGB"), dtype=np.float32)
    m = (a[..., 0] > 1.35 * np.maximum(a[..., 1], a[..., 2])) & (a[..., 0] > 50)
    m[FLAME[1]:FLAME[3], FLAME[0]:FLAME[2]] = False
    reg = m[Y0:Y1, X0:X1]
    return int(reg.sum()), float((a[..., 0] - np.maximum(a[..., 1], a[..., 2]))[Y0:Y1, X0:X1][reg].sum()) if reg.any() else 0.0
for pat in sys.argv[1:]:
    files = sorted(glob.glob(pat))
    v = np.array([census(f) for f in files])
    if len(files) == 1:
        print(pat, "pixels", int(v[0, 0]), "excess", round(v[0, 1]))
    else:
        px = v[:, 0]
        print(pat, "n", len(files), "pixels mean", px.mean().round(0), "median", np.median(px), "pct10/90", np.percentile(px, [10, 90]).round(0), "max", px.max(), "frames over 1500px", int((px > 1500).sum()), "excess mean", v[:, 1].mean().round(0))
        print("   per frame:", " ".join(str(int(p)) for p in px))
