"""Nose-box metric for the trail-hit pairs `rpcs3-hd-trail-hit-pair.py` writes.

    uv run --with numpy --with pillow --with scipy python3 -I scripts/hd-trail-hit-box-metric.py PAIRDIR...

For each pair dir (`capture/00.json`, `field.csv`, `theirs.png`, `ours.png`, optionally `ours_off.png`, and
`ours_fired_on.png`/`ours_fired_off.png`: ours re-rendered on a shifted `field_fired.csv` with the burst on and with
every emitter dropped) prints one JSON line. Inside a 300x300 box centred on the player's nose (4.45 units ahead of
the hull centre) projected through the captured camera: the near-white count (min channel >= 200), the largest
8-connected component's bounding box, and whether a streak (>= 20 px, aspect >= 4) exists; for the fired pair, the
same over `on & !off`, which is ours' burst exactly. The metric and its limits:
docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md, "The trail-hit burst as a matched pair".
"""
import csv
import json
import math
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

NOSE = 4.45
BOX = 300


def project(cam, p, w, h):
    f = np.array(cam["forward"], float); f /= np.linalg.norm(f)
    u = np.array(cam["up"], float)
    r = np.cross(f, u); r /= np.linalg.norm(r)
    u = np.cross(r, f)
    d = np.array(p, float) - np.array(cam["eye"], float)
    z = d @ f
    t = math.tan(math.radians(cam["fov_y_deg"]) / 2)
    x = (d @ r) / (z * t) * (h / w)
    y = (d @ u) / (z * t)
    return (x + 1) / 2 * w, (1 - y) / 2 * h, z


def stats(img, cx, cy):
    a = np.asarray(Image.open(img).convert("RGB")).astype(int)
    h, w, _ = a.shape
    x0, y0 = int(max(0, cx - BOX / 2)), int(max(0, cy - BOX / 2))
    box = a[y0:y0 + BOX, x0:x0 + BOX]
    mask = box.min(axis=2) >= 200
    lab, n = ndimage.label(mask, structure=np.ones((3, 3)))
    best, streak = (0, 0), False
    for sl in ndimage.find_objects(lab):
        bh, bw = sl[0].stop - sl[0].start, sl[1].stop - sl[1].start
        if bw * bh > best[0] * best[1]:
            best = (bw, bh)
        lng, sht = max(bw, bh), max(1, min(bw, bh))
        if lng >= 20 and lng / sht >= 4:
            streak = True
    return int(mask.sum()), best, streak, mask


def main():
    for d in map(Path, sys.argv[1:]):
        rec = json.loads((d / "capture" / "00.json").read_text())
        cam = rec["camera"]
        row = next(csv.DictReader(open(d / "field.csv")))
        pos = [float(row["pos_" + a]) for a in "xyz"]
        fwd = [float(row["fwd_" + a]) for a in "xyz"]
        nose = [pos[i] + fwd[i] * NOSE for i in range(3)]
        out = {"dir": d.name}
        if (d / "ours_fired_on.png").exists() and (d / "ours_fired_off.png").exists():
            w, h = Image.open(d / "ours_fired_on.png").size
            cx, cy, z = project(cam, nose, w, h)
            on = stats(d / "ours_fired_on.png", cx, cy)[3]
            off = stats(d / "ours_fired_off.png", cx, cy)[3]
            burst = on & ~off
            lab, n = ndimage.label(burst, structure=np.ones((3, 3)))
            best, streak = (0, 0), False
            for sl in ndimage.find_objects(lab):
                bh, bw = sl[0].stop - sl[0].start, sl[1].stop - sl[1].start
                if bw * bh > best[0] * best[1]:
                    best = (bw, bh)
                lng, sht = max(bw, bh), max(1, min(bw, bh))
                if lng >= 20 and lng / sht >= 4:
                    streak = True
            ys, xs = np.nonzero(burst)
            extent = (int(xs.max() - xs.min() + 1), int(ys.max() - ys.min() + 1)) if len(xs) else (0, 0)
            out["ours_fired_burst"] = {"bright": int(burst.sum()), "largest": best, "extent": extent, "streak": streak,
                                       "on_total": int(on.sum()), "off_total": int(off.sum())}
        for side in ("theirs", "ours", "ours_off"):
            img = d / (side + ".png")
            if not img.exists():
                continue
            w, h = Image.open(img).size
            cx, cy, z = project(cam, nose, w, h)
            n, best, streak, mask = stats(img, cx, cy)
            out[side] = {"bright": n, "largest": best, "streak": streak, "nose_px": [round(cx), round(cy)], "z": round(z, 1)}
            if side == "ours_off" and "ours" in out:
                on = stats(d / "ours.png", cx, cy)[3]
                out["ours_diff"] = int((on & ~mask).sum())
        print(json.dumps(out))


if __name__ == "__main__":
    main()
