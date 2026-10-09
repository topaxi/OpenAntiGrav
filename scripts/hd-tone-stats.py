#!/usr/bin/env python3
"""Clipped-white share, blob pixels, mean RGB and mean luma of renders against their reference frames.

    uv run --with scipy --with numpy --with pillow python3 scripts/hd-tone-stats.py \\
        --pair metropia-bloom:00:RENDER.png [--pair talons-matched:01:RENDER.png ...] [--label name]

Each `--pair DIR:POSE:RENDER` prints the capture's own frame (`orig`) and the render. The clipped
share and blob pixels (components of min-channel >= 250, >= 20 px) are taken over the ceiling band
(top 420/1058 of the frame) for `metropia-bloom` and over the whole frame for every other pair;
the mean luma (0.3, 0.59, 0.11 over the encoded bytes) is always the whole frame.
"""
import argparse, pathlib
import numpy as np
from PIL import Image
from scipy import ndimage

REF = pathlib.Path(__file__).resolve().parent.parent / "data" / "reference" / "hd-capture"
W = np.array([0.3, 0.59, 0.11])


def stats(path, band):
    a = np.asarray(Image.open(path).convert("RGB")).astype(float)
    sub = a if band is None else a[: int(a.shape[0] * band)]
    low = sub.min(2)
    labels, n = ndimage.label(low >= 250)
    sizes = ndimage.sum(np.ones_like(low), labels, range(1, n + 1)) if n else []
    return ((low >= 250).mean() * 100, sum(s for s in sizes if s >= 20),
            sub.mean((0, 1)).round(1).tolist(), (a / 255 * W).sum(2).mean())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pair", action="append", required=True)
    args = ap.parse_args()
    for spec in args.pair:
        pair, pose, render = spec.split(":", 2)
        band = 420 / 1058 if pair == "metropia-bloom" else None
        print(pair, pose)
        for name, path in (("orig", REF / pair / f"{pose}.png"), ("ours", render)):
            clip, blobs, mean, luma = stats(path, band)
            print(f"  {name}: clip {clip:5.2f}%  blob px {blobs:7.0f}  mean {mean}  luma {luma:.3f}")


if __name__ == "__main__":
    main()
