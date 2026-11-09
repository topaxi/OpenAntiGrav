#!/usr/bin/env python3
"""Score an absorb frame against its no-absorb control on a craft box: mean luma and blue-minus-red of the difference.

    scripts/absorb-score.py --box 150,150,330,272 absorb.png control.png [absorb2.png control2.png ...]

Each pair is `absorb`, `control`; the output per pair is the mean Rec. 709 luma of the box in the absorb frame minus the
control's, and the mean (blue - red) of the same difference (`docs/rendering/weapon-absorb-pairs.md`, "Method"). `--box` is
`x0,y0,x1,y1` in the frame's own pixels (a frame of another size is resized to `--size`, default the box's frame, 480x272).
"""
import argparse
import subprocess


def pixels(path, size):
    w, h = size
    raw = subprocess.run(["magick", path, "-resize", "%dx%d!" % (w, h), "-depth", "8", "rgb:-"], capture_output=True,
                         check=True).stdout
    return raw, w, h


def box_mean(raw, w, box):
    x0, y0, x1, y1 = box
    r = g = b = 0.0
    n = 0
    for y in range(y0, y1):
        row = raw[(y * w + x0) * 3:(y * w + x1) * 3]
        r += sum(row[0::3])
        g += sum(row[1::3])
        b += sum(row[2::3])
        n += x1 - x0
    return r / n, g / n, b / n


def luma(rgb):
    return 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--box", required=True)
    ap.add_argument("--size", default="480x272")
    ap.add_argument("files", nargs="+")
    args = ap.parse_args()
    box = tuple(int(v) for v in args.box.split(","))
    size = tuple(int(v) for v in args.size.split("x"))
    files = args.files
    for a, c in zip(files[0::2], files[1::2]):
        (ra, w, _), (rc, _, _) = pixels(a, size), pixels(c, size)
        ma, mc = box_mean(ra, w, box), box_mean(rc, w, box)
        d = [x - y for x, y in zip(ma, mc)]
        print("%s luma %+.1f  rgb(%+.1f %+.1f %+.1f)  b-r %+.1f" % (a, luma(ma) - luma(mc), d[0], d[1], d[2], d[2] - d[0]))


if __name__ == "__main__":
    main()
