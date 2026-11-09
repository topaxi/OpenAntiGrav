#!/usr/bin/env python3
"""Whole-frame mean luma of Missile-hit films at chosen pool ages, and the normalised excess over the pre-hit frame.

    python3 scripts/hd-missile-film-table.py data/reference/hd-capture/missile-hit/hit0 [more film dirs]

For each `rpcs3-hd-missile-film.py` directory: the luma (Rec. 601, the metric of docs/ghidra/functions/ps3-hdfury-eu/
missile-explosion.md) of `pre.png` and every `aNNNN.png`, linearly interpolated on the pool age (`ship.pool_age`
of each frame's JSON) at the ages asked for, and `e = (L - L_pre) / (L_peak - L_pre)`. Ours' `luma.json`
(`hd-missile-film-pair.py`) is read the same way when `--ours DIR` names its directory.
"""
import argparse
import json
import subprocess
from pathlib import Path

AGES = [0.1, 0.2, 0.33, 0.5, 0.58, 0.6, 0.7, 0.75, 0.83, 0.92, 1.08]


def luma(path):
    out = subprocess.run(["magick", str(path), "-format", "%[fx:(0.299*mean.r+0.587*mean.g+0.114*mean.b)*255]", "info:"],
                         capture_output=True, text=True, check=True).stdout
    return float(out)


def series(film):
    pre = luma(film / "pre.png")
    points = {}
    for js in sorted(film.glob("a[0-9]*.json")):
        age = json.loads(js.read_text())["ship"]["pool_age"]
        points[round(age, 4)] = luma(js.with_suffix(".png"))
    return pre, sorted(points.items())


def at(points, age):
    for (a0, l0), (a1, l1) in zip(points, points[1:]):
        if a0 <= age <= a1:
            return l0 + (l1 - l0) * (age - a0) / (a1 - a0) if a1 > a0 else l0
    return None


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("films", nargs="+", type=Path)
    args = ap.parse_args()
    print("film | pre | peak (age) | " + " | ".join("%.2f" % a for a in AGES))
    for film in args.films:
        pre, points = series(film)
        peak_age, peak = max(points, key=lambda p: p[1])
        cells = []
        for age in AGES:
            value = at(points, age)
            cells.append("-" if value is None else "%.0f (e %.2f)" % (value, (value - pre) / (peak - pre)))
        print("%s | %.1f | %.1f (%.2f) | %s" % (film.name, pre, peak, peak_age, " | ".join(cells)))


if __name__ == "__main__":
    main()
