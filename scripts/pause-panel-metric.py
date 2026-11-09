#!/usr/bin/env python3
"""Compares the pause panel of an original frame and ours, both mapped to the 1280x720 game area.

    uv run --with pillow python3 -I scripts/pause-panel-metric.py ORIGINAL.png OURS.png \
        [--orig-box X0 Y0 W H] [--rows N] [--inset 0.9625]

The original's RPCS3 capture holds the game area at (722, 24, 1278, 718) of a 2000x1200 image;
ours is the whole 1280x720 frame. Prints the two white rules' rows and spans, the focused row's
span, and the mean absolute colour difference over each row's box centre (three columns inside
the box, 0 to 255 per channel) - the backgrounds differ, so only block interiors are compared.
`docs/ui/pause-menu.md`, "HD and Fury", has the numbers this gave.
"""
import argparse
import sys

from PIL import Image


def game_area(path, box):
    image = Image.open(path).convert("RGB")
    if box:
        x, y, w, h = box
        image = image.crop((x, y, x + w, y + h))
    if image.size != (1280, 720):
        image = image.resize((1280, 720), Image.LANCZOS)
    return image


def rules(image):
    pixels = image.load()
    found = []
    for y in range(720):
        run = best = start = best_start = 0
        for x in range(1280):
            if pixels[x, y] == (255, 255, 255):
                if run == 0:
                    start = x
                run += 1
                if run > best:
                    best, best_start = run, start
            else:
                run = 0
        if best >= 300:
            found.append((y, best_start, best_start + best))
    merged = []
    for y, a, b in found:
        if merged and y - merged[-1][1] <= 1:
            merged[-1][1] = y
        else:
            merged.append([y, y, a, b])
    return merged


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("original")
    parser.add_argument("ours")
    parser.add_argument("--orig-box", nargs=4, type=int, default=[722, 24, 1278, 718])
    parser.add_argument("--rows", type=int, default=8)
    parser.add_argument("--inset", type=float, default=0.9625)
    args = parser.parse_args()
    original = game_area(args.original, args.orig_box)
    ours = game_area(args.ours, None)
    for label, image in (("original", original), ("ours", ours)):
        print(label, "rules (y0, y1, x0, x1):", rules(image))
    # Row k of the screen: authored y = 310 + 50 k (1080 grid), box height 40, inset about centre.
    scale = 2.0 / 3.0

    def to_px(y):
        return (540 + (y - 540) * args.inset) * scale

    errors = []
    for k in range(args.rows):
        y = int(to_px(310 + 50 * k + 20))
        row = []
        for x in (540, 580, 620):
            a, b = original.getpixel((x, y)), ours.getpixel((x, y))
            row.append(sum(abs(p - q) for p, q in zip(a, b)) / 3)
        errors.extend(row)
        print("row %d (y=%d): original %s ours %s, mean abs difference %.1f"
              % (k, y, original.getpixel((580, y)), ours.getpixel((580, y)), sum(row) / 3))
    print("block interiors: mean abs colour difference %.1f over %d samples" % (sum(errors) / len(errors), len(errors)))


if __name__ == "__main__":
    sys.exit(main())
