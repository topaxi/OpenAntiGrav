#!/usr/bin/env python3
"""Measure the boost plume in a frame: extent, colour and the orange rim.

The numbers behind `exhaust.md`'s blend table. It exists as a committed script
because that table has now been derived **five** times - in three places, with
three different sets of numbers - and every derivation lived in a scratch file
under gitignored `data/` and then vanished. `race.rs` already carries a "One
home" note about it. This is that home.

The mask is *normalised*: a pixel counts when its magenta excess is a large
enough fraction of its own luminance, not when it is bright. That distinction is
the whole point, and there is a direct demonstration of why. Correcting a frame's
field of view made the plume cover **8 % fewer** pixels by this mask while the
older absolute-threshold mask counted **10 % more** on the very same pair of
images - because the corrected frame is brighter, so more pixels cleared its
`luma > 60` gate even as the plume shrank. The old mask measures brightness and
calls it extent. It is still computed here so old rows can be read, and it should
not be used for anything new.

Definitions, from `exhaust.md`:

    luma      Rec.601, 0.299 R + 0.587 G + 0.114 B
    magenta   min(R, B) - G
    plume     magenta / luma > 0.12  and  luma > 25
    orange    (R - B) / luma > 0.20  and  luma > 25   -- see below
    old       magenta > 25 and luma > 60          (retired, see above)

**`orange` is not a subset of `plume`, and getting that wrong changes it by 14x.**
It is measured over the gated region on its own terms, not inside the plume mask.
Requiring both reproduces `197` where the recorded figure is `425`, and `182`
where it is `2,599`; scoring orange over the region reproduces `425`, `2,599` and
`1,714` exactly. That is the arrangement validated below, and it means the two
populations are largely *disjoint* - the orange rim is mostly the pixels the
magenta mask rejects, which is what makes it a rim.

A region gate drops rows above 300 and columns outside [250, 720), which is what
keeps the track's own lighting and the HUD out of the count.

**Matched-pose use:** a frame of ours is only comparable to a frame of the
original once the projections match. Ours renders at the authored 60 degrees
while the original widens its field with speed, so pass `--camera-fov` computed
from the tick's own forward velocity - see
`docs/rendering/projection-vs-the-original.md`. Every number this script prints
for a *pair* is void without it.

Usage:
    plume-mask.py FRAME [FRAME ...]
"""

import argparse
import sys

import numpy as np
from PIL import Image

ROW_MIN = 300
COL_MIN = 250
COL_MAX = 720

PLUME_MAGENTA_FRACTION = 0.12
PLUME_LUMA_FLOOR = 25.0
ORANGE_FRACTION = 0.20
OLD_MAGENTA_FLOOR = 25.0
OLD_LUMA_FLOOR = 60.0


def measure(path):
    rgb = np.asarray(Image.open(path).convert("RGB"), dtype=np.float32)
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    luma = 0.299 * r + 0.587 * g + 0.114 * b
    magenta = np.minimum(r, b) - g

    region = np.zeros(luma.shape, dtype=bool)
    region[ROW_MIN:, COL_MIN:COL_MAX] = True

    safe = np.where(luma > 0, luma, 1.0)
    plume = region & (luma > PLUME_LUMA_FLOOR) & (magenta / safe > PLUME_MAGENTA_FRACTION)
    old = region & (magenta > OLD_MAGENTA_FLOOR) & (luma > OLD_LUMA_FLOOR)
    # Over the region, *not* within `plume` - see the module docstring. This is
    # the arrangement that reproduces every recorded figure.
    orange = region & (luma > PLUME_LUMA_FLOOR) & ((r - b) / safe > ORANGE_FRACTION)

    n = int(plume.sum())
    if n == 0:
        return {"px": 0, "old": int(old.sum()), "orange": 0}
    mean = (float(r[plume].mean()), float(g[plume].mean()), float(b[plume].mean()))
    ys, xs = np.nonzero(plume)
    return {
        "px": n,
        "old": int(old.sum()),
        "orange": int(orange.sum()),
        "mean": mean,
        "b_minus_r": mean[2] - mean[0],
        "luma": float(luma[plume].mean()),
        "box": (int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())),
        # Every mask measured so far reaches the frame's bottom row, which makes
        # each extent a lower bound. Reported so a reader is not told otherwise.
        "clipped_bottom": bool(ys.max() >= luma.shape[0] - 1),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("frames", nargs="+")
    args = ap.parse_args()

    for path in args.frames:
        m = measure(path)
        if m["px"] == 0:
            print(f"{path}: no plume pixels (old mask {m['old']})")
            continue
        mr, mg, mb = m["mean"]
        x0, y0, x1, y1 = m["box"]
        print(
            f"{path}\n"
            f"  old={m['old']}  px={m['px']}  mean=({mr:.1f},{mg:.1f},{mb:.1f})  "
            f"b-r={m['b_minus_r']:+.1f}  orange={m['orange']}\n"
            f"  mean luma={m['luma']:.1f}  box x[{x0},{x1}] ({x1 - x0} px wide) "
            f"y[{y0},{y1}]{'  BOTTOM-CLIPPED' if m['clipped_bottom'] else ''}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
