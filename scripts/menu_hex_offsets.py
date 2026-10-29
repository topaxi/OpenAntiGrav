#!/usr/bin/env python3
"""Per-axis offset of ours against the original on patches of a menu pair, by gradient cross-correlation.

    scripts/menu_hex_offsets.py theirs.png ours.png NAME:CX:CY:W:H [NAME:CX:CY:W:H ...]
    scripts/menu_hex_offsets.py [--search=N] theirs.png ours.png --ps2-grid NAME:GX:GY:W:H ...   # centre in PS2 GS frame units (640x448, origin at the centre)

A patch is a box (centre and size in the pair's pixels) around one hex. Both images are reduced to a gradient
magnitude (Gaussian sigma 1, then only the strongest edges kept), so a brightness change (the selector pulses between
cyan and white) and a halo do not move the peak; the printed `dx dy` is where ours' edges lie against theirs, in pixels
of the pair, to a tenth of a pixel (parabola through the correlation peak). Positive `dx` means ours is right of the
original, positive `dy` below it. `theirs.png` is resized to ours' size first (one Lanczos resize, like the pair does).
"""

import sys

import numpy as np
from PIL import Image
from scipy import ndimage, signal

SEARCH = 12  # largest shift tried, in pixels (`--search N`)


def edges(img):
    gray = np.asarray(img.convert("L"), dtype=np.float64)
    blur = ndimage.gaussian_filter(gray, 1.0)
    gy, gx = np.gradient(blur)
    mag = np.hypot(gx, gy)
    return np.where(mag > 0.35 * mag.max(), mag, 0.0) if mag.max() > 0 else mag


def peak_offset(a, b):
    """The shift that carries `b` onto `a`, `(dx, dy)`, to sub-pixel."""
    a = a - a.mean()
    b = b - b.mean()
    corr = signal.fftconvolve(a, b[::-1, ::-1], mode="full")
    cy0, cx0 = b.shape[0] - 1, b.shape[1] - 1
    window = corr[cy0 - SEARCH:cy0 + SEARCH + 1, cx0 - SEARCH:cx0 + SEARCH + 1]
    iy, ix = np.unravel_index(np.argmax(window), window.shape)

    def refine(minus, centre, plus):
        denom = minus - 2 * centre + plus
        return 0.0 if denom == 0 else 0.5 * (minus - plus) / denom

    fx = refine(window[iy, ix - 1], window[iy, ix], window[iy, ix + 1]) if 0 < ix < window.shape[1] - 1 else 0.0
    fy = refine(window[iy - 1, ix], window[iy, ix], window[iy + 1, ix]) if 0 < iy < window.shape[0] - 1 else 0.0
    return ix - SEARCH + fx, iy - SEARCH + fy


def measure(theirs_png, ours_png, patches, ps2_grid=False):
    ours = Image.open(ours_png).convert("RGB")
    theirs = Image.open(theirs_png).convert("RGB")
    if theirs.size != ours.size:
        theirs = theirs.resize(ours.size, Image.LANCZOS)
    width, height = ours.size
    out = []
    for name, cx, cy, w, h in patches:
        if ps2_grid:
            cx, cy = (cx + 320.0) * width / 640.0, (cy + 224.0) * height / 448.0
            w, h = w * width / 640.0, h * height / 448.0
        x0, y0 = int(round(cx - w / 2)), int(round(cy - h / 2))
        x1, y1 = int(round(cx + w / 2)), int(round(cy + h / 2))
        box = (x0, y0, x1, y1)
        a, b = edges(theirs.crop(box)), edges(ours.crop(box))
        dx, dy = peak_offset(a, b)
        # `peak_offset(a, b)` carries b onto a: ours moves by +(dx, dy) to meet theirs, so ours sits at -(dx, dy) from theirs
        out.append((name, -dx, -dy))
    return out


def main(argv):
    global SEARCH
    ps2 = "--ps2-grid" in argv
    for a in argv:
        if a.startswith("--search="):
            SEARCH = int(a.split("=")[1])
    argv = [a for a in argv if a != "--ps2-grid" and not a.startswith("--search=")]
    if len(argv) < 4:
        sys.exit(__doc__)
    patches = []
    for item in argv[3:]:
        name, *nums = item.split(":")
        patches.append((name, *(float(n) for n in nums)))
    for name, dx, dy in measure(argv[1], argv[2], patches, ps2):
        print("%-14s dx %+6.2f  dy %+6.2f" % (name, dx, dy))


if __name__ == "__main__":
    main(sys.argv)
