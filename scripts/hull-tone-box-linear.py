#!/usr/bin/env python3
"""Mean of a box of PNGs in linear light (sRGB decoded per pixel, then averaged), per channel.

    python3 scripts/hull-tone-box-linear.py x0,y0,x1,y1 a.png [b.png ...]
"""
import sys

import numpy as np
from PIL import Image


def lin(a):
    a = a / 255.0
    return np.where(a <= 0.04045, a / 12.92, ((a + 0.055) / 1.055) ** 2.4)


def box_linear(path, box):
    x0, y0, x1, y1 = box
    a = np.asarray(Image.open(path).convert("RGB"), dtype=float)[y0:y1, x0:x1]
    return lin(a).mean((0, 1))


if __name__ == "__main__":
    box = tuple(int(x) for x in sys.argv[1].split(","))
    for p in sys.argv[2:]:
        print(p, box_linear(p, box).round(5))
