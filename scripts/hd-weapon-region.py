#!/usr/bin/env python3
"""Region metric for a weapon visual: the same box of an original frame and of ours, side by side.

    python3 scripts/hd-weapon-region.py --box X,Y,W,H theirs.png ours.png [more.png ...]

Reads each PNG through ImageMagick (`magick ... rgb:-`), so it needs nothing from pip. In the box it counts

* `red`: pixels that are red (R >= 170, G <= 0.6 R, B <= 0.75 R) - the Missile's shell and halo,
* `white`: pixels with every channel >= 240 - a blown-out core,
* `lit`: red or white,

and prints the lit pixels' centroid and the 5th to 95th percentile extent in x and y, which is the size a
player reads as the effect's width and height. Mean RGB is over the lit pixels. The box is in the frame's own
pixels, so both images must be the same size.
"""
import argparse
import subprocess
import sys


def load(path, box):
    x, y, w, h = box
    raw = subprocess.run(["magick", path, "-crop", "%dx%d+%d+%d" % (w, h, x, y), "+repage", "-depth", "8", "rgb:-"],
                         capture_output=True, check=True).stdout
    return raw


def measure(path, box):
    w, h = box[2], box[3]
    raw = load(path, box)
    if len(raw) < w * h * 3:
        sys.exit("%s: the box runs outside the frame" % path)
    red = white = 0
    xs, ys, sums = [], [], [0, 0, 0]
    for i in range(w * h):
        r, g, b = raw[i * 3], raw[i * 3 + 1], raw[i * 3 + 2]
        is_white = min(r, g, b) >= 240
        is_red = r >= 170 and g <= 0.6 * r and b <= 0.75 * r
        if not (is_white or is_red):
            continue
        red += is_red and not is_white
        white += is_white
        xs.append(i % w)
        ys.append(i // w)
        sums[0] += r
        sums[1] += g
        sums[2] += b
    lit = len(xs)
    if not lit:
        return dict(red=0, white=0, lit=0)
    xs.sort()
    ys.sort()
    p = lambda v, q: v[min(len(v) - 1, int(q * len(v)))]
    return dict(red=red, white=white, lit=lit,
                cx=sum(xs) / lit + box[0], cy=sum(ys) / lit + box[1],
                w90=p(xs, 0.95) - p(xs, 0.05), h90=p(ys, 0.95) - p(ys, 0.05),
                rgb=tuple(round(s / lit) for s in sums))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--box", required=True, help="X,Y,W,H in frame pixels")
    ap.add_argument("images", nargs="+")
    args = ap.parse_args()
    box = tuple(int(v) for v in args.box.split(","))
    print("%-40s %6s %6s %6s %7s %7s %5s %5s  %s" % ("image", "red", "white", "lit", "cx", "cy", "w90", "h90", "mean rgb"))
    for path in args.images:
        m = measure(path, box)
        if not m["lit"]:
            print("%-40s %6d %6d %6d  (nothing lit)" % (path[-40:], 0, 0, 0))
            continue
        print("%-40s %6d %6d %6d %7.1f %7.1f %5d %5d  %s" % (path[-40:], m["red"], m["white"], m["lit"], m["cx"], m["cy"],
                                                           m["w90"], m["h90"], m["rgb"]))


if __name__ == "__main__":
    main()
