#!/usr/bin/env python3
"""Score a Fury menu pair with the animated backdrop masked out.

    python3 scripts/fury-menu-masked-score.py [--freeze DIR | --mask DIR] PAIR_DIR...

A PAIR_DIR holds `theirs.png` and `ours.png`. The reference frames carry RPCS3's 16-bit QUADS wrap
(`docs/reverse-engineering/rpcs3-index-wrap.md`), so the backdrop in `theirs.png` is wrong and must
not steer page work. The backdrop is additive, warm (orange/gold/brown) light over a near-black page,
and neither frame can be rendered without it, so the mask is read off the pixels:

  warm pixel = R - B >= 8 and G >= 0.30 * R and R >= 20
  (the page's red hexes and text have G < 0.25 * R; the grey circuit model and white text have R ~ B)

The mask is the union of the warm pixels of BOTH frames, dilated by `GROW` pixels to take the glow's
skirt. `--freeze DIR` writes each pair's mask to `DIR/<pair>-mask.png`; `--mask DIR` scores with those
frozen masks instead of recomputing, so a before and an after run cover the same pixels (a fix that adds
warm page content must not grow the mask over it). Per-pair `KEEP` rectangles (x0, y0, x1, y1 in output
pixels) are page content the mask never covers. Prints whole-frame and masked mean absolute grey difference (0-255, Rec. 709 luma) and the
masked fraction. Scores differ from compare_env.overlap_score's by ImageMagick's gamma handling only.
"""
import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image

GROW = 5
WARM_RB = 8
WARM_GR = 0.30
WARM_R_MIN = 20


def luma(a):
    return a[..., 0] * 0.2126 + a[..., 1] * 0.7152 + a[..., 2] * 0.0722


def warm(a):
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    return (r - b >= WARM_RB) & (g >= WARM_GR * r) & (r >= WARM_R_MIN)


def dilate(m, n):
    out = m.copy()
    for _ in range(n):
        p = np.pad(out, 1)
        out = (p[1:-1, 1:-1] | p[:-2, 1:-1] | p[2:, 1:-1] | p[1:-1, :-2] | p[1:-1, 2:]
               | p[:-2, :-2] | p[:-2, 2:] | p[2:, :-2] | p[2:, 2:])
    return out


KEEP = {
    "menu-track-select": [(519, 208, 770, 366)],
}


def mask_of(theirs, ours, name=""):
    m = dilate(warm(theirs) | warm(ours), GROW)
    for x0, y0, x1, y1 in KEEP.get(name, []):
        m[y0:y1, x0:x1] = False
    return m


def score(pair, frozen=None):
    t = np.asarray(Image.open(pair / "theirs.png").convert("RGB"), float)
    o = np.asarray(Image.open(pair / "ours.png").convert("RGB"), float)
    d = np.abs(luma(t) - luma(o))
    if frozen:
        m = np.asarray(Image.open(frozen / (pair.name + "-mask.png"))) > 0
    else:
        m = mask_of(t, o, pair.name)
    return d.mean(), (d[~m].mean() if (~m).any() else 0.0), m.mean(), m


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--freeze", type=Path, help="write each pair's mask here")
    ap.add_argument("--mask", type=Path, help="score with the frozen masks in this directory")
    ap.add_argument("pairs", nargs="+", type=Path)
    a = ap.parse_args()
    for p in a.pairs:
        whole, masked, frac, m = score(p, a.mask)
        print("%-22s whole %.2f  masked %.2f  mask %.1f%%" % (p.name, whole, masked, frac * 100))
        if a.freeze:
            a.freeze.mkdir(parents=True, exist_ok=True)
            Image.fromarray((m * 255).astype("uint8")).save(a.freeze / (p.name + "-mask.png"))


if __name__ == "__main__":
    sys.exit(main())
