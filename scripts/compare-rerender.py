#!/usr/bin/env python3
"""Re-make comparison pairs in place from the emulator's own savestate, so ours always takes its pose from the original.

    just compare-rerender [--only <substring>] [DIR]       # DIR default data/reference/compare-page

The pose, camera and race clock are NEVER taken from a stored `pair.json` or a stored CSV: for each pair this
re-runs the pair script with the arguments recorded in `pair.json` (`argv`) into a scratch directory, which loads
the savestate in the emulator, reads the ship pose, camera and race clock out of RAM fresh, derives the pose row
and the heading correction from that read, renders ours with the CURRENT oag-game, runs every environment and
parity check, and captures `theirs.png`. The emulator must be up exactly as when the pair was made (PPSSPP started
with the profile's ini, PCSX2 booted with `OAG_COMPARE_PLATFORM`, RPCS3 restored into the state with `--restart`;
docs/tools/compare-page.md, "Capturing a pair per title").

On success `theirs.png`, `ours.png`, `side.png`, the pose CSVs and `pair.json` are replaced and the stored values
become a record: `previous` in the new `pair.json` holds the old pose, camera, clock and binary, and `drift`
the differences (ship position, heading, camera, clock, and the RMSE between the old and new `theirs.png`); a drift
beyond the profile's tolerance is printed as a WARNING. A pair whose fresh run is refused by a check is named, keeps
its old files, and counts in the exit status.
"""

import argparse
import datetime
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402

RUNNER = ["uv", "run", "--with", "websocket-client", "--with", "evdev", "--with", "python-xlib", "python3"]


def camera_of(c):
    if not c:
        return None
    return (c.get("pos") or c.get("eye"), c.get("fwd") or c.get("forward"))


def drift_of(old, new, tol):
    out, warn = {}, []
    if old.get("pose") and new.get("pose"):
        out["ship_units"] = ce.math.dist(old["pose"]["pos"], new["pose"]["pos"])
        out["ship_heading_deg"] = ce.vec_angle(old["pose"]["fwd"], new["pose"]["fwd"])
        if out["ship_units"] > tol["pos_units"] or out["ship_heading_deg"] > tol["heading_deg"]:
            warn.append("ship pose moved %.3f units, %.3f deg" % (out["ship_units"], out["ship_heading_deg"]))
    oc, nc = camera_of(old.get("camera")), camera_of(new.get("camera"))
    if oc and nc:
        out["camera_units"] = ce.math.dist(oc[0], nc[0])
        out["camera_heading_deg"] = ce.vec_angle(oc[1], nc[1])
        if out["camera_units"] > tol["camera_pos_units"] or out["camera_heading_deg"] > tol["heading_deg"]:
            warn.append("camera moved %.3f units, %.3f deg" % (out["camera_units"], out["camera_heading_deg"]))
    if old.get("race_clock_s") is not None and new.get("race_clock_s") is not None:
        out["race_clock_s"] = new["race_clock_s"] - old["race_clock_s"]
    return out, warn


def remake(pj):
    rec = json.loads(pj.read_text())
    argv = rec.get("argv")
    if not argv:
        return ["pair.json has no argv (made before it was recorded)"], []
    plat = ce.platform(rec["profile"])
    tmp = Path(tempfile.mkdtemp(prefix="rerender-", dir=pj.parent.parent))
    args = list(argv)
    args[args.index("--out") + 1] = str(tmp)
    done = subprocess.run(RUNNER + [str(HERE / args[0])] + args[1:], capture_output=True, text=True)
    new_pj = tmp / "pair.json"
    if not new_pj.exists() or json.loads(new_pj.read_text()).get("refused"):
        why = [ln for ln in done.stderr.splitlines() if ln.startswith(("REFUSED", "MISMATCH"))] or done.stderr.splitlines()[-1:]
        shutil.rmtree(tmp, ignore_errors=True)
        return ["the fresh run was refused or failed (old files kept): " + " | ".join(why)], []
    new = json.loads(new_pj.read_text())
    drift, warn = drift_of(rec["original"], new["original"], plat["tolerance"])
    cmp = subprocess.run(["compare", "-metric", "RMSE", str(pj.parent / "theirs.png"), str(tmp / "theirs.png"), "null:"],
                         capture_output=True, text=True)
    drift["theirs_rmse_vs_previous"] = cmp.stderr.strip()
    new["previous"] = {"original": {k: rec["original"].get(k) for k in ("pose", "camera", "race_clock_s", "emulator")},
                       "ours_binary": rec["ours"].get("binary"), "rerendered_from": rec.get("rerendered_at")}
    new["drift"] = drift
    new["rerendered_at"] = datetime.datetime.now().astimezone().isoformat(timespec="seconds")
    new["argv"] = rec["argv"]
    new["argv"][new["argv"].index("--out") + 1] = str(pj.parent)
    new_pj.write_text(json.dumps(new, indent=2) + "\n")
    # the new run's files replace the old ones, the scratch shots directory included
    for f in tmp.iterdir():
        dst = pj.parent / f.name
        if dst.is_dir():
            shutil.rmtree(dst)
        elif dst.exists():
            dst.unlink()
        shutil.move(str(f), str(dst))
    shutil.rmtree(tmp, ignore_errors=True)
    return [], warn


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("dir", nargs="?", type=Path, default=ROOT / "data/reference/compare-page")
    ap.add_argument("--only", help="substring of the pair directory path")
    args = ap.parse_args()
    failed = 0
    for pj in sorted(args.dir.rglob("pair.json")):
        if args.only and args.only not in str(pj.parent):
            continue
        if json.loads(pj.read_text()).get("refused"):
            continue
        problems, warn = remake(pj)
        if problems:
            failed += 1
            print("FAILED %s: %s" % (pj.parent, "; ".join(problems)))
        else:
            print("ok     %s%s" % (pj.parent, "".join("  WARNING: " + w for w in warn)))
    sys.exit(failed)


if __name__ == "__main__":
    main()
