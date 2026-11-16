#!/usr/bin/env python3
"""Compares Pure's pause band in an original frame and ours, both mapped to 480x272.

    uv run --with pillow --with numpy python3 -I scripts/pause-band-metric.py ORIGINAL.png OURS.png \
        [--rows N] [--orig-box X Y W H]

The original is PPSSPP's 480x272 window (`InternalResolution = 1`, software renderer); ours is a
`--screenshot` or drive `screenshot` of any size, resized by box filter. Prints, for each frame:

- the rows of the two 1-pixel rules (`0xff3192bb`) and their spans (authored: y 75 and 175, x 0 to 480);
- the band tint law: the brightest pixel of a band row with no text on it. The band is `0xc0000010`
  over the scene, so no channel can exceed `0.25 * 255 + (0, 0, 16)` = (64, 64, 80);
- the text rows' ink spans (first and last row with white ink) and the peak ink of each, the
  cursor row against the others;
- the mean absolute colour difference over the first row's box (x 150 to 330, y 100 to 118), where
  both frames hold the same word in the same ink over a band (the backgrounds differ).

`docs/ui/pause-menu.md`, "Pure", has the numbers this gave.
"""
import argparse

import numpy as np
from PIL import Image

RULE = (49, 146, 187)


def load(path, box):
    image = Image.open(path).convert("RGB")
    if box:
        x, y, w, h = box
        image = image.crop((x, y, x + w, y + h))
    if image.size != (480, 272):
        image = image.resize((480, 272), Image.BOX)
    return np.asarray(image).astype(int)


def rule_rows(a):
    close = (np.abs(a - np.array(RULE)).sum(axis=2) < 12)
    out = []
    for y in range(272):
        xs = np.nonzero(close[y])[0]
        if len(xs) > 300:
            out.append((y, int(xs.min()), int(xs.max()) + 1))
    return out


def ink_spans(a, threshold=95):
    region = a[78:172, 150:340].min(axis=2)
    rows = (region > threshold).sum(axis=1) > 0
    spans, start = [], None
    for i, on in enumerate(rows):
        if on and start is None:
            start = i
        if not on and start is not None:
            spans.append((start + 78, i - 1 + 78))
            start = None
    if start is not None:
        spans.append((start + 78, len(rows) - 1 + 78))
    return spans


def peak(a, span):
    block = a[span[0] : span[1] + 1, 150:340].reshape(-1, 3)
    top = block[block.sum(axis=1).argsort()[-12:]]
    return tuple(int(v) for v in top.mean(axis=0).round())


def describe(name, a):
    print(f"== {name}")
    print("rules (row, x0, x1):", rule_rows(a))
    spans = ink_spans(a)
    print("ink rows:", spans)
    print("ink peaks:", [peak(a, span) for span in spans if span[1] - span[0] >= 6])
    text_free = [y for y in range(78, 172) if not any(s[0] - 1 <= y <= s[1] + 1 for s in spans)]
    band = a[text_free, :, :]
    print("brightest text-free band pixel per channel (law: <= 64, 64, 80):", tuple(int(v) for v in band.reshape(-1, 3).max(axis=0)))
    return spans


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("original")
    parser.add_argument("ours")
    parser.add_argument("--orig-box", nargs=4, type=int, default=None)
    args = parser.parse_args()
    original = load(args.original, args.orig_box)
    ours = load(args.ours, None)
    describe("original", original)
    describe("ours", ours)
    box = (slice(100, 118), slice(150, 330))
    diff = np.abs(original[box] - ours[box]).mean()
    print(f"mean absolute difference over the first row's box: {diff:.2f} of 255 per channel")


if __name__ == "__main__":
    main()
