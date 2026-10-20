#!/usr/bin/env python3
"""Does a craft resting after GO turn in ours the way it turns in the original?

For every pair under a compare-page title directory (`pair.json`, written by
`psp-pulse-pair.py` or `rpcs3-hd-pair.py`), runs ours UNPOSED from its own grid
with no input for the pair's own tick count (`GO_TICK + clock * 60`), and prints
our heading at GO, at that tick, and the original's heading read out of RAM at
the same race clock. `drift` is ours from GO; `diff` is ours minus the original
at the rendered clock; `correction` is the heading turn the pair script applied
to its tick-0 pose.

    python3 scripts/compare-resting-yaw.py --title pulse-psp --image data/images/pulse-psp-usa.chd
    python3 scripts/compare-resting-yaw.py --title hd --image data/images/hdfury-ps3-eu-dec.iso

Findings: docs/physics/grid-state.md, "A resting craft after GO turns, in the original too".
"""

import argparse
import csv
import glob
import json
import math
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GO_TICK = 276


def yaw(fwd):
    return math.degrees(math.atan2(float(fwd[0]), -float(fwd[2])))


def wrap(a):
    return (a + 180.0) % 360.0 - 180.0


def original_pose(pair, title):
    if title == "hd":
        return pair["original"]["pose"]["fwd"], pair["original"]["pose"]["pos"]
    # The PSP pair keeps the original's RAM row 0 in trace.csv beside pair.json.
    return None, None


def correction(pair, title):
    if title == "hd":
        c = pair["ours"].get("yaw_corrections") or []
        return c[-1]["yaw"] - c[0]["yaw"] if c else 0.0
    c = pair["ours"].get("heading_corrections") or []
    return c[-1]["turn_deg"] + c[-1]["heading_error_deg"] if c else 0.0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--title", choices=("pulse-psp", "hd"), required=True)
    ap.add_argument("--image", required=True)
    ap.add_argument("--pairs", default=None, help="default data/reference/compare-page/<title>")
    ap.add_argument("--oag-game", default=str(ROOT / "target/debug/oag-game"))
    args = ap.parse_args()
    pairs = Path(args.pairs or ROOT / "data/reference/compare-page" / args.title)
    scratch = Path(tempfile.mkdtemp(prefix="resting-yaw-"))
    env = dict(os.environ, XDG_CONFIG_HOME=str(scratch / "c"), XDG_DATA_HOME=str(scratch / "d"),
               XDG_STATE_HOME=str(scratch / "s"))
    try:
        run(args, pairs, scratch, env)
    finally:
        shutil.rmtree(scratch, ignore_errors=True)


def run(args, pairs, scratch, env):
    for path in sorted(glob.glob(str(pairs / "*/pair.json"))):
        pair = json.load(open(path))
        flags = pair["ours"]["flags"]
        ticks = int(pair["ours"]["ticks"])
        extra = []
        for key in ("--track", "--mode", "--class", "--team"):
            if key in flags:
                extra += [key, flags[flags.index(key) + 1]]
        name = Path(path).parent.name
        out = scratch / (name + ".csv")
        subprocess.run([args.oag_game, args.image, "--race", "--no-audio", "--ticks", str(ticks + 1),
                        "--trace-out", str(out)] + extra,
                       env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        rows = {int(r["tick"]): r for r in csv.DictReader(open(out))}
        at_go, at_t = rows[GO_TICK], rows[ticks]
        fwd, pos = original_pose(pair, args.title)
        if fwd is None:
            row = next(csv.DictReader(open(Path(path).parent / "trace.csv")))
            fwd = [row["fwd_" + a] for a in "xyz"]
            pos = [float(row["pos_" + a]) for a in "xyz"]
        ours_go = yaw([at_go["fwd_" + a] for a in "xyz"])
        ours_t = yaw([at_t["fwd_" + a] for a in "xyz"])
        theirs = yaw(fwd)
        dist = math.dist([float(at_t["pos_" + a]) for a in "xyz"], pos)
        clock = (ticks - GO_TICK) / 60.0
        print("%-20s clock %6.2f s  drift %8.3f  diff %7.3f  correction %8.3f  position %.3f" % (
            name, clock, wrap(ours_t - ours_go), wrap(ours_t - theirs), correction(pair, args.title), dist))


if __name__ == "__main__":
    main()
