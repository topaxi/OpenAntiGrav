#!/usr/bin/env python3
"""Score a weapon's original-versus-ours frame pairs per phase, with a HUD mask and the two controls.

Inputs are directories of `kNNN.png` frames (480x272), as `scripts/psp-weapon-pair.py` writes
for the original and as a loop of `oag-game --race ... --ticks 391+k --screenshot` writes for
ours (`docs/ghidra/functions/psp-pulse-usa/plasma.md`, "2026-10-30: the matched pair"):

    python3 scripts/weapon-pair-score.py --orig A --orig2 B --orig-ctl C --ours D --ours-ctl E \\
        --phase "head box:200,112,90,80:36-72" --phase "whole frame::76-92" --ks 36,40,...

For each phase it prints the mean absolute grey difference (HUD boxes masked) of
`floor` (orig vs orig2: re-capture noise), `fire` (orig vs ours), `ctl` (the two no-fire
controls: the scene gap that is not the weapon) and `excess = fire - ctl`, with the masked
area. `--rgb K` prints each side's mean RGB per listed frame, which is robust to a position
difference where a grey difference is not.
"""

import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image

HUD = [(0, 0, 60, 45), (380, 0, 100, 45), (200, 0, 85, 72), (0, 200, 175, 72), (290, 200, 190, 72), (130, 243, 230, 29)]


def load(path):
    return np.asarray(Image.open(path).convert("RGB").resize((480, 272)), dtype=np.float32)


def grey(a):
    return a @ np.array([0.299, 0.587, 0.114], dtype=np.float32)


def mask(box):
    m = np.ones((272, 480), bool)
    for x, y, w, h in HUD:
        m[y:y + h, x:x + w] = False
    if box:
        x, y, w, h = box
        keep = np.zeros_like(m)
        keep[y:y + h, x:x + w] = True
        m &= keep
    return m


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    for name in ("orig", "orig2", "orig-ctl", "ours", "ours-ctl"):
        ap.add_argument("--" + name, type=Path, required=True)
    ap.add_argument("--ks", required=True, help="comma-separated frame numbers present in every directory")
    ap.add_argument("--phase", action="append", default=[], help="NAME:x,y,w,h (blank for the whole frame):LO-HI")
    ap.add_argument("--rgb", help="comma-separated frames: print each side's mean RGB")
    a = ap.parse_args()
    ks = [int(v) for v in a.ks.split(",")]

    def frame(d, k):
        return load(d / ("k%03d.png" % k))

    for spec in a.phase:
        name, box, span = spec.split(":")
        lo, hi = (int(v) for v in span.split("-"))
        sel = [k for k in ks if lo <= k <= hi]
        m = mask(tuple(int(v) for v in box.split(",")) if box else None)
        row = {}
        for key, (x, y) in {"floor": (a.orig, a.orig2), "fire": (a.orig, a.ours), "ctl": (a.orig_ctl, a.ours_ctl)}.items():
            row[key] = float(np.mean([np.abs(grey(frame(x, k)) - grey(frame(y, k)))[m].mean() for k in sel]))
        excess = row["fire"] - row["ctl"]
        print("%-34s k%s  area %6d  floor %5.2f  fire %5.2f  ctl %5.2f  excess %5.2f  area*excess %d"
              % (name, span, m.sum(), row["floor"], row["fire"], row["ctl"], excess, excess * m.sum()))
    if a.rgb:
        m = mask(None)
        for k in (int(v) for v in a.rgb.split(",")):
            print("k%03d orig %s ctl %s ours %s" % (k, *(frame(d, k)[m].mean(0).round(0) for d in (a.orig, a.orig_ctl, a.ours))))


if __name__ == "__main__":
    sys.exit(main())
