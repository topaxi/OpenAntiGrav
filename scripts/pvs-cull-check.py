#!/usr/bin/env python3
"""Does the original's section-box view test explain what it does not submit?

The original tests each section's own authored box against the view, after the
section mask (`oag_render::pvs::sections_in_view`, `docs/ghidra/functions/
psp-pulse-usa/section-view-cull.md`). This applies that test, with the view and
projection **the dump's own registers** give, to the section that governs every
draw of the track, and joins the verdict with whether the original's GE list
contains the draw's batch:

  * `section-culled  twin`    - a draw the original submits although the test
                                hides its section: the falsifier, must be 0.
  * `section-culled  no-twin` - absent from the original, and explained.
  * `section-visible no-twin` - absent, and not explained by this test.

    cargo build -q -p oag-render --example pvs_moving_census --example pvs_section_boxes
    python3 scripts/pvs-cull-check.py --spline spline.csv --pose-dir DIR \\
        --prims prims.json --entry 'Data\\Environments\\03_Track\\track.vex'

`prims.json` is `psp-ge-dump.py census` of the dump (it carries `view` and `proj`
per PRIM); `--pose-dir`'s `trace.csv` and `--spline` give the craft's and the
camera's section as `scripts/pvs-moving-census.py` does. A static draw is matched
on its vertex count and world box (to `--pos-tol` units), a moving one on its
vertex count and diameter (exact on both sides, to `--tol`). The original submits
many batches twice with two textures, so its PRIMs are folded first. **Draws in a
visible section claim their twins first**: an instanced copy of a mesh at a
culled section must not take the twin that belongs to the visible one.
"""

import argparse
import collections
import csv
import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NEAR_REJECT = 2.482


def outcode(v, proj):
    """The game's outcode of one view-space point, bit for bit (`FUN_08902894`).

    `z < 0` of the GE's clip space is the depth half way between near and far,
    not the near plane; `proj` is the dump's own projection, so it is exact here.
    """
    x, y, z = v
    cx = x * proj[0] + y * proj[4] + z * proj[8] + proj[12]
    cy = x * proj[1] + y * proj[5] + z * proj[9] + proj[13]
    cz = x * proj[2] + y * proj[6] + z * proj[10] + proj[14]
    w = x * proj[3] + y * proj[7] + z * proj[11] + proj[15]
    return (1 if cx < -w else 0) | (2 if cy < -w else 0) | (4 if cz < 0 else 0) \
        | (8 if cx > w else 0) | (16 if cy > w else 0)


def box_culled(view, proj, mn, mx):
    """`FUN_08902594`: culled when every one of the 8 corners shares an outcode bit."""
    acc = 31
    for cx in (mn[0], mx[0]):
        for cy in (mn[1], mx[1]):
            for cz in (mn[2], mx[2]):
                v = [cx * view[k] + cy * view[3 + k] + cz * view[6 + k] + view[9 + k] for k in range(3)]
                acc &= outcode(v, proj)
                if not acc:
                    return False
    return True


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--spline", required=True, type=Path)
    ap.add_argument("--pose-dir", required=True, type=Path)
    ap.add_argument("--prims", required=True, type=Path)
    ap.add_argument("--entry", required=True)
    ap.add_argument("--image", default="data/images/pulse-psp-usa.chd")
    ap.add_argument("--tol", type=float, default=0.03)
    ap.add_argument("--pos-tol", type=float, default=0.5)
    ap.add_argument("--control", action="store_true",
                    help="positive control: test against a view turned a quarter circle "
                    "about the vertical, where the falsifier count must rise")
    ap.add_argument("--list", action="store_true", help="print the falsifier draws")
    args = ap.parse_args()

    rows = list(csv.DictReader(l for l in args.spline.open() if not l.startswith("[")))
    pts = [[float(r["pos" + s]) for s in ("_x", "_y", "_z")] for r in rows]
    trace = list(csv.DictReader((args.pose_dir / "trace.csv").open()))[1]
    pos = [float(trace["pos" + s]) for s in ("_x", "_y", "_z")]
    cam = [float(trace["cam_pos" + s]) for s in ("_x", "_y", "_z")]
    craft_row = min(range(len(pts)), key=lambda i: math.dist(pos, pts[i]))
    cam_row = min((k % len(pts) for k in range(craft_row - 96, craft_row + 97)),
                  key=lambda i: math.dist(cam, pts[i]))
    craft_sec, cam_sec = int(rows[craft_row]["section"]), int(rows[cam_row]["section"])

    def run(example, *extra):
        out = subprocess.run(
            ["cargo", "run", "-q", "-p", "oag-render", "--example", example, "--",
             args.image, args.entry, *extra], cwd=ROOT, capture_output=True, text=True)
        if out.returncode:
            sys.exit(out.stderr[-800:])
        return json.loads(out.stdout)

    ours = run("pvs_moving_census", str(craft_sec), str(cam_sec))
    sections = {b["id"]: b for b in run("pvs_section_boxes") if "min" in b}

    prims = json.loads(args.prims.read_text())
    main_prims = [p for p in prims if p["proj"][11] == -1.0 and p["proj"][5] > 1.0]
    (view, proj), _ = collections.Counter(
        (tuple(p["view"]), tuple(p["proj"])) for p in main_prims).most_common(1)[0]

    if args.control:
        # Rows are the view's x, y, z axes: swap x and z to turn the camera 90 degrees.
        view = (view[6], view[7], view[8], view[3], view[4], view[5],
                -view[0], -view[1], -view[2], view[9], view[10], view[11])

    folded, seen = [], set()
    for p in prims:
        key = (p["nv"], tuple(round(x, 1) for x in p["mn"]), tuple(round(x, 1) for x in p["mx"]))
        if key not in seen:
            seen.add(key)
            folded.append(p)
    by_count = collections.defaultdict(list)
    for k, p in enumerate(folded):
        by_count[p["nv"]].append(k)
    close = lambda a, b: abs(a - b) <= args.tol * max(abs(a), abs(b), 1.0)

    def match(o, claimed):
        for k in by_count.get(o["nv"], []):
            p = folded[k]
            if k in claimed or p.get("diam") is None or not close(o["diam"], p["diam"]):
                continue
            if not o["moving"] and not all(
                    abs(a - b) <= args.pos_tol for a, b in zip(o["mn"] + o["mx"], p["mn"] + p["mx"])):
                continue
            return k
        return None

    def section_culled(o):
        mask = int(o["mask"], 16)
        if mask == 0 or mask & (mask - 1):
            return None
        box = sections.get(mask.bit_length() - 1)
        return None if box is None else box_culled(view, proj, box["min"], box["max"])

    verdict = {(o["list"], o["i"]): section_culled(o) for o in ours}
    claimed, twin = set(), {}
    for o in sorted(ours, key=lambda o: (o["moving"], bool(verdict[(o["list"], o["i"])]))):
        k = match(o, claimed)
        if k is not None:
            claimed.add(k)
        twin[(o["list"], o["i"])] = k is not None

    table = collections.Counter()
    for o in ours:
        key = (o["list"], o["i"])
        if not o["craft"] or verdict[key] is None:
            continue
        label = "section-culled" if verdict[key] else "section-visible"
        table[("moving" if o["moving"] else "static", label, "twin" if twin[key] else "no-twin")] += 1
        if args.list and verdict[key] and twin[key]:
            print("  FALSIFIER: node %s %s #%d nv %d diam %.1f mask %s"
                  % (o["node"], o["list"], o["i"], o["nv"], o["diam"], o["mask"]))
    print("pose %s: craft section %d, camera section %d" % (args.pose_dir.name, craft_sec, cam_sec))
    for key in sorted(table):
        print("  %-7s %-15s %-8s draws %d" % (key + (table[key],)))


if __name__ == "__main__":
    main()
