#!/usr/bin/env python3
"""Where a region of ours sits against the original's: the (dx, dy) shift of ours that best matches the original, by masked
normalised cross-correlation of high-passed gradient magnitude (no brightness threshold; the two renderers light differently).
    frame-shift.py THEIRS OURS [region ...]   -> one line per region (hull, ground_right, ground_left, far_scenery)
A positive dx means the ORIGINAL's feature is that many px to the right of ours' (original minus ours). The boxes are Wipeout 2048's
960x544 grid frame (hull under the chase camera, floor either side, scenery above the horizon, HUD outside all four); the scenery box is a direction
rather than a number where its NCC is under about 0.4."""
import sys
import numpy as np
from PIL import Image
from scipy import ndimage

REGIONS = {
    "hull": (390, 255, 600, 420),
    "ground_right": (600, 225, 880, 335),
    "ground_left": (60, 235, 400, 420),
    "far_scenery": (230, 40, 760, 200),
}

def feat(path):
    g = np.asarray(Image.open(path).convert("L"), dtype=np.float64)
    g = g - ndimage.gaussian_filter(g, 6.0)
    g = ndimage.gaussian_filter(g, 0.8)
    gx, gy = ndimage.sobel(g, axis=1), ndimage.sobel(g, axis=0)
    return np.hypot(gx, gy)

def ncc(a, b):
    a = a - a.mean(); b = b - b.mean()
    d = np.sqrt((a * a).sum() * (b * b).sum())
    return float((a * b).sum() / d) if d > 0 else 0.0

def best_shift(ft, fo, box, rng=40, sub=True):
    x0, y0, x1, y1 = box
    ref = ft[y0:y1, x0:x1]
    best = (-2, 0, 0)
    surf = {}
    for dy in range(-rng, rng + 1):
        for dx in range(-rng, rng + 1):
            # ours moved by (dx, dy) so that it lies on the original: ours at (x - dx, y - dy) matches theirs at (x, y)
            ys, xs = y0 - dy, x0 - dx
            if ys < 0 or xs < 0 or ys + (y1 - y0) > fo.shape[0] or xs + (x1 - x0) > fo.shape[1]:
                continue
            c = ncc(ref, fo[ys:ys + (y1 - y0), xs:xs + (x1 - x0)])
            surf[(dx, dy)] = c
            if c > best[0]:
                best = (c, dx, dy)
    return best, surf

def main():
    ft, fo = feat(sys.argv[1]), feat(sys.argv[2])
    only = sys.argv[3:] 
    for name, box in REGIONS.items():
        if only and name not in only: continue
        (c, dx, dy), surf = best_shift(ft, fo, box)
        # second best away from the peak, to tell a sharp peak from a flat band
        far = max((v for (x, y), v in surf.items() if abs(x - dx) > 3 or abs(y - dy) > 3), default=0)
        print("%-13s original minus ours: dx %+3d dy %+3d   ncc %.3f  (best elsewhere %.3f)" % (name, dx, dy, c, far))

if __name__ == "__main__":
    main()
