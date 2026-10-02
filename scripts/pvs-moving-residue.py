#!/usr/bin/env python3
"""Why does the original not submit a moving draw our section mask passes?

`scripts/pvs-cull-check.py` explains 47 % of the moving draws that pass the
section mask and have no twin by the section's own box. This takes what is left
and asks the question that script cannot: **where was the draw?** A moving
draw's box moves with its `Anim Transform`, and the original's clock is one
global time, so the dump's moving PRIMs pin that time
(`cargo run -p oag-render --example pvs_moving_phase`), and with it the world box
of every moving draw the dump does *not* contain. Each such draw is then either
outside the view (the frustum explains it) or inside it (something else does).

    cargo build -q -p oag-render --example pvs_moving_census \\
        --example pvs_section_boxes --example pvs_moving_phase
    python3 scripts/pvs-moving-residue.py --spline spline.csv --pose-dir DIR \\
        --prims prims.json --entry 'Data\\Environments\\03_Track\\track.vex'

Inputs as `pvs-cull-check.py`: `--pose-dir`'s `trace.csv` and `--spline` give the
craft's and the camera's section, `--prims` is `psp-ge-dump.py census` of the
dump (with `view`/`proj`).

Per moving draw that passes the craft's mask and whose section the view reaches:

  * **present**  - the dump holds a PRIM of the same vertex count whose world
                   box is within `--tol` of ours at the solved time;
  * **in view**  - its own world box, grown by `--tol`, is not culled by the
                   dump's own view and projection (the game's corner outcode).

The falsifier for "the frustum is the whole story" is a row that must read zero:
`present, culled` (a draw the original submits although its own box is outside
the view). The residue a rule would have to explain is `absent, in view`.
"""

import argparse
import collections
import csv
import importlib.util
import json
import math
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("cullcheck", ROOT / "scripts" / "pvs-cull-check.py")
cullcheck = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cullcheck)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--spline", required=True, type=Path)
    ap.add_argument("--pose-dir", required=True, type=Path)
    ap.add_argument("--prims", required=True, type=Path)
    ap.add_argument("--entry", required=True)
    ap.add_argument("--image", default="data/images/pulse-psp-usa.chd")
    ap.add_argument("--tol", type=float, default=0.7, help="world units, a box's six numbers")
    ap.add_argument("--grow", type=float, default=1.5, help="units the box is grown by for the view test")
    ap.add_argument("--max-seconds", type=float, default=620.0)
    ap.add_argument("--control", action="store_true", help="view turned a quarter circle")
    ap.add_argument("--list", action="store_true", help="print every moving draw that passes the mask")
    ap.add_argument("--json", type=Path, help="write the per-draw records here")
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

    def run(example, *extra, env=None):
        out = subprocess.run(
            ["cargo", "run", "-q", "-p", "oag-render", "--example", example, "--",
             args.image, args.entry, *extra], cwd=ROOT, capture_output=True, text=True,
            env={**os.environ, **(env or {})})
        if out.returncode:
            sys.exit(out.stderr[-800:])
        return json.loads(out.stdout)

    prims = json.loads(args.prims.read_text())
    main_prims = [p for p in prims if p["proj"][11] == -1.0 and p["proj"][5] > 1.0]
    (view, proj), _ = collections.Counter(
        (tuple(p["view"]), tuple(p["proj"])) for p in main_prims).most_common(1)[0]
    if args.control:
        view = (view[6], view[7], view[8], view[3], view[4], view[5],
                -view[0], -view[1], -view[2], view[9], view[10], view[11])

    folded, seen = [], set()
    for p in prims:
        key = (p["nv"], tuple(round(x, 1) for x in p["mn"]), tuple(round(x, 1) for x in p["mx"]))
        if key not in seen and p.get("diam") is not None:
            seen.add(key)
            folded.append(p)

    obs_path = args.pose_dir / "obs.tsv"
    obs_path.write_text("".join(
        "%d %f %f %f %f %f %f %f\n" % (p["nv"], p["diam"], *p["mn"], *p["mx"]) for p in folded))
    phase_args = [str(obs_path), str(args.max_seconds)]
    phase = run("pvs_moving_phase", *phase_args, env={"TOL": str(args.tol)})
    ours = run("pvs_moving_census", str(craft_sec), str(cam_sec))
    sections = {b["id"]: b for b in run("pvs_section_boxes") if "min" in b}
    boxes = {(b["list"], b["i"]): b for b in phase["boxes"]}

    by_count = collections.defaultdict(list)
    for p in folded:
        by_count[p["nv"]].append(p)

    def present(b):
        for p in by_count.get(b["nv"], []):
            if all(abs(a - c) <= args.tol for a, c in zip(b["mn"] + b["mx"], p["mn"] + p["mx"])):
                return True
        return False

    def section_culled(o):
        mask = int(o["mask"], 16)
        if mask == 0 or mask & (mask - 1):
            return None
        box = sections.get(mask.bit_length() - 1)
        return None if box is None else cullcheck.box_culled(view, proj, box["min"], box["max"])

    def box_verdict(b, prefix=""):
        g = args.grow
        return cullcheck.box_culled(view, proj, [v - g for v in b[prefix + "mn"]],
                                    [v + g for v in b[prefix + "mx"]])

    def corners_culled(corners):
        acc = 31
        for c in corners:
            v = [c[0] * view[k] + c[1] * view[3 + k] + c[2] * view[6 + k] + view[9 + k] for k in range(3)]
            acc &= cullcheck.outcode(v, proj)
            if not acc:
                return False
        return True

    # Where the box is taken: at the dump's own instant, at animation time zero,
    # or in the node's own space with no matrix at all. The original submits a
    # moving draw whose box at the dump's instant the view rejects, so the first
    # is not what its cull reads; the other two are candidates for what is.
    rules = (("at the dump's instant", ""), ("at animation time 0", "rest_"),
             ("anchor space, no matrix", "local_"))

    table = collections.Counter()
    records = []
    for o in ours:
        if not o["moving"] or not o["craft"]:
            continue
        b = boxes.get((o["list"], o["i"]))
        if b is None:
            continue
        sc = section_culled(o)
        if sc is None:
            continue
        pres = present(b)
        verdicts = {name: box_verdict(b, prefix) for name, prefix in rules}
        verdicts["mesh box corners, dump's instant"] = corners_culled(b["mesh_corners"])
        # Hold counter: in view at the dump's instant or on any of the previous
        # 14 frames (the view is taken as unchanged over them).
        verdicts["mesh box, held 14 frames"] = all(
            corners_culled(c) for c in [b["mesh_corners"], *b["history"]])
        rec = {"node": o["node"], "list": o["list"], "i": o["i"], "nv": o["nv"], "diam": o["diam"],
               "section_culled": sc, "present": pres, "box_culled": verdicts,
               "mn": b["mn"], "mx": b["mx"], "rest_mn": b["rest_mn"], "rest_mx": b["rest_mx"]}
        records.append(rec)
        if sc:
            continue
        for name, culled in verdicts.items():
            table[(name, "present" if pres else "absent", "culled" if culled else "in view")] += 1
        if args.list:
            print("  node %4s %-11s #%-4d nv %4d diam %6.1f %-7s %s"
                  % (o["node"], o["list"], o["i"], o["nv"], o["diam"],
                     "present" if pres else "absent",
                     " ".join("culled" if v else "view" for v in verdicts.values())))
    print("pose %s: craft section %d, camera section %d" % (args.pose_dir.name, craft_sec, cam_sec))
    print("  clock: t = %.3f s, %d observed boxes matched; next-best time %.1f s scores %d; "
          "stationary baseline %d" % (phase["t"], phase["score"], phase["top"][1][1],
                                      phase["top"][1][0], phase["top"][-1][0]))
    for key in sorted(table):
        print("  %-24s %-8s %-8s draws %d" % (key + (table[key],)))
    if args.json:
        args.json.write_text(json.dumps({"pose": args.pose_dir.name, "t": phase["t"],
                                         "score": phase["score"], "top": phase["top"],
                                         "records": records}))


if __name__ == "__main__":
    main()
