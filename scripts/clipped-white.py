#!/usr/bin/env python3
"""Clipped-white share and mean luminance of a capture.

The aggregate the HD/Fury frame-brightness work discriminates on. **Mean
luminance is not the metric**: it matched the rpcs3 reference throughout the
2026-08-20 investigation while the frame was visibly blown out, which is
exactly why that defect survived review. Clipped-white share is what moved.

    clipped white   share of pixels with R, G and B all >= 250
    mean luminance  mean of (0.2126 R + 0.7152 G + 0.0722 B) / 255

Both are aggregates over the whole image, so two captures at different sizes
and near-but-not-identical framings are comparable - which is the only way our
1440x816 captures can be read against rpcs3 grabs at 1280x720. **Never a pixel
diff against a reference grab**; the framings are close, not the same.

Usage:

    python3 scripts/clipped-white.py shot.png [more.png ...]

Needs ImageMagick's `magick` on PATH and nothing else - deliberately, so it
runs in a checkout with no Python packages installed.

**Trim letterboxing before you compare anything.** This script measures the
whole file, and the rpcs3 grabs are not all the same shape:
`talons/00.png` is a 1278x718 frame on a 1600x1200 canvas, 52.2 % of it black,
while `talons-fifo/00.png` is an untrimmed 1278x718. Padding dilutes the share
directly - 8.056 % over that frame's own picture becomes 3.850 % over the
canvas, and 8.056 * (1 - 0.522) = 3.851 closes the arithmetic. A `3.85-6.99 %`
"band" derived from those two figures is comparing a padded grab against an
unpadded one, and puts its real *upper* bound at the bottom.

Reference figures on Talon's Junction, trimmed, for whoever picks this up next:

    17 rpcs3 race grabs, trimmed        2.848-20.549 %  median 7.491
      of those, on the grid              6.40-8.06 %
      of those, in motion               14.41-20.55 %
    ours, --race --ticks 0, bloom on   (2026-09-09)  9.86 %  mean 0.513
    ours, --race --ticks 0, bloom off  (2026-09-09)  4.51 %  mean 0.485

So at a standing start our bloom-on frame sits just above the original's own
grid range and bloom-off sits just below it - not the "off is inside the band,
on is far above it" the padded pair implied.

`data/` is gitignored, so `rg` and `fd` return nothing under it with exit code
0. Use `ls`/`find`, or `--no-ignore`.
"""

import subprocess
import sys


def measure(path):
    """Returns (clipped-white percent, mean luminance 0..1, pixel count)."""
    raw = subprocess.run(
        ["magick", path, "-depth", "8", "rgb:-"],
        check=True,
        capture_output=True,
    ).stdout
    count = len(raw) // 3
    if count == 0:
        raise SystemExit(f"{path}: no pixels")
    clipped = 0
    total = 0.0
    view = memoryview(raw)
    red, green, blue = view[0::3], view[1::3], view[2::3]
    for i in range(count):
        r, g, b = red[i], green[i], blue[i]
        if r >= 250 and g >= 250 and b >= 250:
            clipped += 1
        total += 0.2126 * r + 0.7152 * g + 0.0722 * b
    return 100.0 * clipped / count, total / count / 255.0, count


def main(argv):
    if not argv:
        raise SystemExit(__doc__)
    for path in argv:
        clipped, mean, count = measure(path)
        print(f"{clipped:7.3f} %  mean {mean:.4f}  ({count} px)  {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
