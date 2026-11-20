#!/usr/bin/env python3
"""What `wipeout-pc-bloom-knockout.py` measured: the original's bloom, as whole-frame and per-box grey.

    python3 scripts/omega-bloom-knockout-effect.py <run dir> [<run dir>...]

A knockout grab's effect is its Rec.709 grey (sRGB values, the pair tables' grey) minus the mean of the two base grabs on
either side of it, because the grid camera drifts between grabs (two bases 0.06 s apart differ by 8 to 14 grey per pixel and
by under 1 in the frame mean). A sample is dropped when its two bases are identical (the port had ended: its last frame
stays on screen) or when the poke was overwritten before the grab (`held_after_run` below `pokes`).
"""
import json
import sys

import numpy as np
from PIL import Image

BOXES = {  # the omega-hd-exposure boxes, x, y, w, h of the 1920x1080 frame
    "sebenco": {"snow sky": (1100, 40, 300, 160), "building left": (0, 200, 640, 200), "yellow floor": (700, 480, 560, 100), "wall right": (1200, 380, 600, 200)},
    "modesto": {"sky gap": (800, 160, 240, 100), "left structure": (0, 120, 600, 200), "right wall": (1160, 380, 640, 180), "floor left": (100, 600, 460, 120)},
    "moa": {"sky/city": (1160, 60, 440, 120), "stands left": (0, 200, 500, 200), "floor left": (300, 640, 360, 220), "stripe road": (660, 500, 500, 80)},
    "amph": {"ceiling/sky": (300, 110, 460, 130), "stands right": (1150, 190, 560, 130), "road left": (60, 520, 480, 230), "wall left": (0, 380, 576, 120)},
}


def grey(path):
    a = np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def samples(run, condition):
    rows = json.load(open(run + "/note.json"))["rows"]
    frames = {r["label"]: grey("%s/%s.png" % (run, r["label"])) for r in rows}
    labels = [r["label"] for r in rows]
    for i, r in enumerate(rows):
        if r["condition"] != condition or i == 0 or i + 1 >= len(rows) or r["held_after_run"] < r["pokes"]:
            continue
        before, after = frames[labels[i - 1]], frames[labels[i + 1]]
        if np.array_equal(before, after):
            continue
        yield frames[labels[i]] - (before + after) / 2


def main():
    circuit = sys.argv[1]  # sebenco, modesto, moa or amph
    for run in sys.argv[2:]:
        for condition in ("nobloom", "nomask"):
            diffs = list(samples(run, condition))
            if not diffs:
                continue
            means = [d.mean() for d in diffs]
            print("%s %-8s n=%d whole frame %+.2f sd %.2f" % (run, condition, len(diffs), np.mean(means), np.std(means)))
            for name, (x, y, w, h) in BOXES[circuit].items():
                print("    %-14s %+.2f" % (name, np.mean([d[y:y + h, x:x + w].mean() for d in diffs])))


if __name__ == "__main__":
    main()
