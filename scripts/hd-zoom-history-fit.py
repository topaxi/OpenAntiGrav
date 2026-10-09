#!/usr/bin/env python3
"""Fit HD's zoom-history draw on dumped quarter-resolution buffers.

    hd-zoom-history-fit.py <dump-dir> <stem> [E]

The post-chain runner draws the scene copy `S` (the half-resolution scene `0x02150000` box-filtered to quarter size;
`0x02280000` itself is overwritten later in the frame) into `X` with blend off, then the other ping-pong
buffer `Y` over it with `SRC_ALPHA, ONE_MINUS_SRC_ALPHA`, alpha `w` and texture coordinates
`uv * (1 - 2c) + c`.  This scans `w` and `c` (and the pair `X`/`Y` of `0x02300000`/`0x022c0000`) for the least
residual of `X = (1 - w) * S + w * warp(Y, c)` with a bilinear warp, and prints it beside the law
`w = 0.25 E`, `c = 0.05 E` when `E` is given.
"""
import sys
from pathlib import Path

import numpy as np

W, H, PITCH = 320, 180, 0x500


def load(path):
    raw = np.frombuffer(Path(path).read_bytes()[: PITCH * H], dtype=np.uint8).reshape(H, PITCH // 4, 4)
    return raw[:, :W, 1:4].astype(np.float64)


def warp(img, c):
    ys, xs = np.mgrid[0:H, 0:W]
    u = (xs + 0.5) / W * (1 - 2 * c) + c
    v = (ys + 0.5) / H * (1 - 2 * c) + c
    x = u * W - 0.5
    y = v * H - 0.5
    x0 = np.clip(np.floor(x).astype(int), 0, W - 2)
    y0 = np.clip(np.floor(y).astype(int), 0, H - 2)
    fx = np.clip(x - x0, 0, 1)[..., None]
    fy = np.clip(y - y0, 0, 1)[..., None]
    a = img[y0, x0] * (1 - fx) + img[y0, x0 + 1] * fx
    b = img[y0 + 1, x0] * (1 - fx) + img[y0 + 1, x0 + 1] * fx
    return a * (1 - fy) + b * fy


def rms(x, s, y, w, c):
    return float(np.sqrt(np.mean((x - ((1 - w) * s + w * warp(y, c))) ** 2)))


def main():
    d, stem = Path(sys.argv[1]), sys.argv[2]
    e = float(sys.argv[3]) if len(sys.argv) > 3 else None
    half = np.frombuffer((d / ("%s-vram-02150000.bin" % stem)).read_bytes()[: 0xA00 * 360], dtype=np.uint8)
    half = half.reshape(360, 0xA00 // 4, 4)[:, :640, 1:4].astype(np.float64)
    s = half.reshape(180, 2, 320, 2, 3).mean(axis=(1, 3))
    b = {o: load(d / ("%s-vram-%s.bin" % (stem, o))) for o in ("02300000", "022c0000")}
    best = None
    for xo, yo in (("02300000", "022c0000"), ("022c0000", "02300000")):
        x, y = b[xo], b[yo]
        base = rms(x, s, y, 0.0, 0.0)
        for w in np.arange(0, 0.5, 0.005):
            for c in np.arange(0, 0.11, 0.0025):
                r = rms(x, s, y, w, c)
                if best is None or r < best[0]:
                    best = (r, xo, w, c, base)
    r, xo, w, c, base = best
    print("X=%s  best w=%.3f c=%.4f rms=%.2f  (w=0: %.2f)  E from w %.3f, from c %.3f" % (xo, w, c, r, base, w / 0.25, c / 0.05))
    if e is not None:
        yo = "022c0000" if xo == "02300000" else "02300000"
        print("law w=%.4f c=%.4f rms=%.2f" % (0.25 * e, 0.05 * e, rms(b[xo], s, b[yo], 0.25 * e, 0.05 * e)))


if __name__ == "__main__":
    main()
