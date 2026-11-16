#!/usr/bin/env python3
"""Find a campaign-map node's icon in a Wipeout 2048 (Vita) frame, by normalised cross-correlation.

    python3 scripts/vita-2048-map-node.py FRAME TEMPLATE [--peaks N]

Prints one `x y score` line per peak, best first (x, y: the template's centre in the frame's pixels). The map drifts
sideways while it is idle, so a node's tap coordinate is read off a frame taken a moment before the tap, not remembered.
The templates are crops of the game's own frames and live under `data/reference/vita-2048-map/` (never committed here).
Needs numpy and Pillow.
"""

import argparse
import sys

import numpy as np
from PIL import Image


def grey(path):
    return np.asarray(Image.open(path).convert("L"), dtype=np.float64)


def ncc_map(frame, tpl):
    """Zero-mean normalised cross-correlation of `tpl` over `frame`, 'valid' positions, by FFT."""
    fh, fw = frame.shape
    th, tw = tpl.shape
    t = tpl - tpl.mean()
    tn = np.sqrt((t * t).sum())
    size = (fh + th, fw + tw)
    ff = np.fft.rfft2(frame, size)
    num = np.fft.irfft2(ff * np.conj(np.fft.rfft2(t, size)), size)[: fh - th + 1, : fw - tw + 1]
    ones = np.ones_like(t)
    s1 = np.fft.irfft2(ff * np.conj(np.fft.rfft2(ones, size)), size)[: fh - th + 1, : fw - tw + 1]
    s2 = np.fft.irfft2(np.fft.rfft2(frame * frame, size) * np.conj(np.fft.rfft2(ones, size)), size)[: fh - th + 1, : fw - tw + 1]
    var = np.maximum(s2 - s1 * s1 / t.size, 1e-9)
    return num / (tn * np.sqrt(var))


def peaks(score, count, gap):
    score = score.copy()
    out = []
    for _ in range(count):
        y, x = np.unravel_index(np.argmax(score), score.shape)
        out.append((x, y, float(score[y, x])))
        score[max(0, y - gap): y + gap, max(0, x - gap): x + gap] = -1.0
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("frame")
    ap.add_argument("template")
    ap.add_argument("--peaks", type=int, default=3)
    ap.add_argument("--gap", type=int, default=40, help="pixels around a peak that another may not use")
    args = ap.parse_args()
    frame, tpl = grey(args.frame), grey(args.template)
    th, tw = tpl.shape
    for x, y, s in peaks(ncc_map(frame, tpl), args.peaks, args.gap):
        print("%d %d %.3f" % (x + tw // 2, y + th // 2, s))


if __name__ == "__main__":
    sys.exit(main())
