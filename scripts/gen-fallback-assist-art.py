#!/usr/bin/env python3
"""Draws the Pilot Assist indicator's art for the fallback layer.

The art is this project's own: plain geometry (a rounded plate, a ring with a
triangle in it, arrowheads, chevrons) computed here, not traced from or derived
from any game file. It is white shapes in the `OAGMASK` text format
(`crates/texture/src/mask.rs`); the widgets tint it.

Run from the repository root:

    python3 scripts/gen-fallback-assist-art.py

It rewrites `assets/fallback/Data/HUD/Textures/AssistIndicator.pix`, which is
checked in. The atlas layout is mirrored in
`assets/fallback/Data/XML/HUD_assist_indicator.xml`.
"""

import math
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "assets/fallback/Data/HUD/Textures/AssistIndicator.pix"
W, H = 112, 144
SS = 4  # supersampling per axis


def in_polygon(x, y, pts):
    inside = False
    n = len(pts)
    for i in range(n):
        x1, y1 = pts[i]
        x2, y2 = pts[(i + 1) % n]
        if (y1 > y) != (y2 > y) and x < (x2 - x1) * (y - y1) / (y2 - y1) + x1:
            inside = not inside
    return inside


def rounded_rect(x, y, w, h, r):
    """Signed distance, negative inside, of a rounded rectangle at origin."""
    qx = abs(x - w / 2) - (w / 2 - r)
    qy = abs(y - h / 2) - (h / 2 - r)
    return math.hypot(max(qx, 0), max(qy, 0)) + min(max(qx, qy), 0) - r


def plate(x, y):
    d = rounded_rect(x, y, 112, 48, 10)
    if d > 0:
        return 0.0
    return 1.0 if d > -2.5 else 0.8


def ring_and_craft(x, y):
    cx = cy = 20
    r = math.hypot(x - cx, y - cy)
    if 14.5 <= r <= 19:
        return 1.0
    return 1.0 if in_polygon(x, y, [(20, 9), (28.5, 28), (20, 23.5), (11.5, 28)]) else 0.0


def small_left(x, y):
    return 1.0 if in_polygon(x, y, [(3, 10), (17, 2), (17, 18)]) else 0.0


def small_right(x, y):
    return small_left(20 - x, y)


def big_left(x, y):
    pts = [(44, 4), (50, 10), (32, 28), (50, 46), (44, 52), (20, 28)]
    return 1.0 if in_polygon(x, y, pts) else 0.0


def big_right(x, y):
    return big_left(56 - x, y)


# (origin x, origin y, width, height, shape)
PARTS = [
    (0, 0, 56, 56, big_left),
    (56, 0, 56, 56, big_right),
    (0, 56, 112, 48, plate),
    (0, 104, 40, 40, ring_and_craft),
    (40, 104, 20, 20, small_left),
    (60, 104, 20, 20, small_right),
]


def main():
    alpha = [[0.0] * W for _ in range(H)]
    for ox, oy, w, h, shape in PARTS:
        for py in range(h):
            for px in range(w):
                total = 0.0
                for sy in range(SS):
                    for sx in range(SS):
                        total += shape(px + (sx + 0.5) / SS, py + (sy + 0.5) / SS)
                alpha[oy + py][ox + px] = total / (SS * SS)
    lines = [
        f"OAGMASK {W} {H}",
        "# Pilot Assist indicator art for the fallback layer: this project's own",
        "# geometry, drawn by scripts/gen-fallback-assist-art.py. Chosen, not measured.",
    ]
    for row in alpha:
        lines.append("".join("." if a < 1 / 32 else "0123456789abcdef"[min(15, round(a * 15))] for a in row))
    OUT.write_text("\n".join(lines) + "\n")
    print(f"wrote {OUT}", file=sys.stderr)


main()
