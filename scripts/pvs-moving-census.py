#!/usr/bin/env python3
"""Does our section mask drop a moving draw the original submits? One pose.

The join between the original's GE list (`scripts/psp-ge-dump.py census`, one
record per PRIM) and ours (`cargo run -p oag-render --example
pvs_moving_census`, one record per draw call with its authored section mask).
Written for `docs/rendering/frame-audit.md` section 3, "Repeated on Metropia and
Tech De Ra".

    cargo build -q -p oag-render --example pvs_moving_census
    python3 scripts/pvs-moving-census.py --spline spline.csv --pose-dir DIR \\
        --entry 'Data\\Environments\\03_Track\\track.vex'

`--spline` is `oag-trace track`'s CSV for the same file. `--pose-dir` holds the
`trace.csv` (`psp-trace.py --camera --ticks 3`: the craft and the camera at the
pose) and the `prims.json` (the census of the dump taken at that same pose) that
a `psp-drive.py place` walk leaves. The craft's and the camera's section are the
`section` of the nearest spline row, the camera's within 96 rows of the craft's
as the running game looks.

A moving draw is matched on its vertex count and its **diameter** (the longest
vertex-to-vertex distance, which an animation's rotation cannot change); a
static one on the count and its sorted box extents as well. The original submits
many batches twice at one place with two textures, so its PRIMs are first folded
to one per (vertices, box). Static draws claim their twins first, because they
match on more.

What the output means: `DROPS` counts moving draws our mask hides that still have
an unclaimed twin in the original's list; nonzero is a moving draw the original
submits and we hide. The converse - moving draws we pass with no twin - is a
lower bound on what the original hides that we draw (a coincidence of count and
diameter turns a miss into a hit, never the reverse), and our side applies no
frustum, so some of those are the original's frustum cull.
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


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--spline", required=True, type=Path)
    parser.add_argument("--pose-dir", required=True, type=Path)
    parser.add_argument("--entry", required=True, help="the track's .vex entry name")
    parser.add_argument("--image", default="data/images/pulse-psp-usa.chd")
    parser.add_argument("--tol", type=float, default=0.03, help="relative size tolerance")
    parser.add_argument("--list", action="store_true", help="print every moving draw")
    parser.add_argument(
        "--force-sections",
        metavar="CRAFT,CAMERA",
        help="positive control: use these sections instead of the pose's. A craft "
        "in the wrong section hides what the original draws, so DROPS must go up",
    )
    args = parser.parse_args()

    rows = list(csv.DictReader(l for l in args.spline.open() if not l.startswith("[")))
    pts = [[float(r["pos" + s]) for s in ("_x", "_y", "_z")] for r in rows]
    trace = list(csv.DictReader((args.pose_dir / "trace.csv").open()))[1]
    pos = [float(trace["pos" + s]) for s in ("_x", "_y", "_z")]
    cam = [float(trace["cam_pos" + s]) for s in ("_x", "_y", "_z")]
    craft_row = min(range(len(pts)), key=lambda i: math.dist(pos, pts[i]))
    cam_row = min(
        (k % len(pts) for k in range(craft_row - 96, craft_row + 97)),
        key=lambda i: math.dist(cam, pts[i]),
    )
    craft_sec, cam_sec = int(rows[craft_row]["section"]), int(rows[cam_row]["section"])
    if args.force_sections:
        craft_sec, cam_sec = (int(v) for v in args.force_sections.split(","))
    print(
        "craft row %d (%.1f off) section %d, camera row %d (%.1f off) section %d"
        % (craft_row, math.dist(pos, pts[craft_row]), craft_sec, cam_row,
           math.dist(cam, pts[cam_row]), cam_sec)
    )

    out = subprocess.run(
        ["cargo", "run", "-q", "-p", "oag-render", "--example", "pvs_moving_census", "--",
         args.image, args.entry, str(craft_sec), str(cam_sec)],
        cwd=ROOT, capture_output=True, text=True,
    )
    if out.returncode:
        sys.exit(out.stderr[-800:])
    ours = json.loads(out.stdout)

    folded, seen = [], set()
    prims = json.loads((args.pose_dir / "prims.json").read_text())
    for p in prims:
        key = (p["nv"], tuple(round(x, 1) for x in p["mn"]), tuple(round(x, 1) for x in p["mx"]))
        if key not in seen:
            seen.add(key)
            folded.append(p)
    print("original PRIMs %d, %d after folding second-texture passes; our draws %d, moving %d"
          % (len(prims), len(folded), len(ours), sum(o["moving"] for o in ours)))

    tol = args.tol
    close = lambda a, b: abs(a - b) <= tol * max(abs(a), abs(b), 1.0)
    ext = lambda mn, mx: sorted(b - a for a, b in zip(mn, mx))
    by_count = collections.defaultdict(list)
    for k, p in enumerate(folded):
        by_count[p["nv"]].append(k)

    def match(o, moving, claimed):
        for k in by_count.get(o["nv"], []):
            p = folded[k]
            if k in claimed or p.get("diam") is None or not close(o["diam"], p["diam"]):
                continue
            if not moving and not all(
                close(a, b) for a, b in zip(ext(o["mn"], o["mx"]), ext(p["mn"], p["mx"]))
            ):
                continue
            return k
        return None

    for moving in (False, True):
        for tier, key in (("ours", "ours"), ("craft-only", "craft")):
            claimed, n, hit = set(), 0, 0
            for o in (o for o in ours if o["moving"] == moving and o[key]):
                k = match(o, moving, claimed)
                n += 1
                if k is not None:
                    claimed.add(k)
                    hit += 1
            print("  %s draws passing the %s mask: %d, twin in the original %d, none %d"
                  % ("moving" if moving else "static", tier, n, hit, n - hit))

    claimed = set()
    for o in (o for o in ours if not o["moving"] and o["ours"]):
        k = match(o, False, claimed)
        if k is not None:
            claimed.add(k)
    for o in (o for o in ours if o["moving"] and o["ours"]):
        k = match(o, True, claimed)
        if k is not None:
            claimed.add(k)
    dropped = []
    for o in (o for o in ours if o["moving"] and not o["ours"]):
        k = match(o, True, claimed)
        if k is not None:
            claimed.add(k)
            dropped.append((o, k))
    print("  DROPS: moving draws our mask hides that have an unclaimed twin: %d" % len(dropped))
    for o, k in dropped:
        print("    node %s %s #%d nv %d diam %.1f mask %s -> original PRIM %d (nv %d diam %.1f)"
              % (o["node"], o["list"], o["i"], o["nv"], o["diam"], o["mask"], k,
                 folded[k]["nv"], folded[k]["diam"]))
    if args.list:
        for o in sorted((o for o in ours if o["moving"]), key=lambda o: (not o["ours"], -o["diam"])):
            print("   node %4s %-11s #%-4d nv %4d diam %7.1f ours %-5s craft %s"
                  % (o["node"], o["list"], o["i"], o["nv"], o["diam"], o["ours"], o["craft"]))


if __name__ == "__main__":
    main()
