#!/usr/bin/env python3
"""How far a HUD moved between two frames: gradient cross-correlation per region, sub-pixel.

    hud-shift.py <base.png> <frame.png> [<frame.png> ...]

Prints, per frame, the mean shift in x and y of five HUD regions (the 1280x720 positions of HD's
Time Trial HUD: record panel, clock, lap counter, speed bar, and the top centre box) against
`base.png`, and the spread between the regions. A whole-HUD translation reads the same in all of
them (spread under 0.3 px); a region that disagrees is not part of the slide. The search is plus or
minus 26 px with a parabolic fit round the best integer. Used for the HD HUD sway
(`docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md`, "The HUD sway").
"""
import sys

import numpy as np
from PIL import Image

REGIONS = {
    "record": (1010, 40, 1200, 110),
    "time": (40, 585, 350, 680),
    "lap": (40, 60, 190, 160),
    "speed": (910, 545, 1220, 680),
    "topc": (545, 40, 710, 170),
}
RADIUS = 26


def load(path):
    return np.asarray(Image.open(path).convert("L"), dtype=np.float64)


def grad(a):
    return np.abs(np.diff(a, axis=1))[:-1, :] + np.abs(np.diff(a, axis=0))[:, :-1]


def estimate(base, other, box):
    x0, y0, x1, y1 = box
    ga = base[y0:y1, x0:x1]
    score = {}
    for dy in range(-RADIUS, RADIUS + 1):
        for dx in range(-RADIUS, RADIUS + 1):
            gb = other[y0 + dy:y1 + dy, x0 + dx:x1 + dx]
            score[(dx, dy)] = ((ga - ga.mean()) * (gb - gb.mean())).sum() / (
                ga.std() * gb.std() * ga.size + 1e-9)
    dx, dy = max(score, key=score.get)

    def sub(c, m, p):
        d = m - 2 * c + p
        return 0.5 * (m - p) / d if d else 0.0

    c = score[(dx, dy)]
    sx = sub(c, score.get((dx - 1, dy), 0), score.get((dx + 1, dy), 0)) if abs(dx) < RADIUS else 0.0
    sy = sub(c, score.get((dx, dy - 1), 0), score.get((dx, dy + 1), 0)) if abs(dy) < RADIUS else 0.0
    return dx + sx, dy + sy, c


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    base = grad(load(sys.argv[1]))
    for path in sys.argv[2:]:
        other = grad(load(path))
        found = [estimate(base, other, box) for box in REGIONS.values()]
        dx = [f[0] for f in found]
        dy = [f[1] for f in found]
        print("%s dx %.2f dy %.2f spread dx %.2f dy %.2f" % (
            path.rsplit("/", 1)[-1], np.mean(dx), np.mean(dy), np.ptp(dx), np.ptp(dy)))


if __name__ == "__main__":
    main()
