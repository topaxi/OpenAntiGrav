#!/usr/bin/env python3
"""A pair's air brake flap region against the rest of its frame: mean absolute grey difference, original against ours.

    uv run --with pillow --with numpy python3 scripts/compare-flap-region.py <pair dir> --box X0,Y0,X1,Y1 [--json out.json]

The box is the flap region in the image's own pixels (both fins, both sides' drawn positions); the HUD rectangles are left out
of "the rest of the frame" (the original prints its own speed, shield and clock there, ours at a posed rest). The grey is
Rec. 601 luma of the 8-bit RGB. The flap box's difference is also reported as a ratio of the rest of the scene's, which is
the number that says whether the flaps are a worse match than the hull and the circuit around them.
"""

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image

#: HUD rectangles of the PSP frame (x0, y0, x1, y1), 480x272: lap counter, weapon hexagon, record, best/current clock, speed and shield bars.
HUD = [(0, 0, 70, 40), (200, 0, 290, 65), (390, 0, 480, 40), (0, 200, 130, 272), (290, 205, 480, 272)]


def grey(path):
    rgb = np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)
    return rgb @ np.array([0.299, 0.587, 0.114])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pair", type=Path)
    ap.add_argument("--box", required=True, help="X0,Y0,X1,Y1 of the flap region, in image pixels")
    ap.add_argument("--json", type=Path)
    args = ap.parse_args()
    theirs, ours = grey(args.pair / "theirs.png"), grey(args.pair / "ours.png")
    diff = np.abs(theirs - ours)
    x0, y0, x1, y1 = (int(v) for v in args.box.split(","))
    scene = np.ones(diff.shape, dtype=bool)
    for a, b, c, d in HUD:
        scene[b:d, a:c] = False
    flap = np.zeros(diff.shape, dtype=bool)
    flap[y0:y1, x0:x1] = True
    flap &= scene
    rest = scene & ~flap
    out = {
        "box": [x0, y0, x1, y1],
        "flap_pixels": int(flap.sum()),
        "rest_pixels": int(rest.sum()),
        "flap_mean_abs_grey_diff": float(diff[flap].mean()),
        "rest_mean_abs_grey_diff": float(diff[rest].mean()),
        "whole_frame_mean_abs_grey_diff": float(diff.mean()),
    }
    out["flap_over_rest"] = out["flap_mean_abs_grey_diff"] / out["rest_mean_abs_grey_diff"]
    print(json.dumps(out, indent=2))
    if args.json:
        args.json.write_text(json.dumps(out, indent=2) + "\n")


if __name__ == "__main__":
    main()
