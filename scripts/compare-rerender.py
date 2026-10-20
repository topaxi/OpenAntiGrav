#!/usr/bin/env python3
"""Re-render OURS of every recorded comparison pair with the current oag-game build, in place, no emulator.

    just compare-rerender [--only <substring>] [DIR]       # DIR default data/reference/compare-page

For each `pair.json` (written by psp-pulse-pair.py, rpcs3-hd-pair.py, pcsx2-pulse-pair.py) this re-runs
`ours.flags` with the current build: the pose CSV is the pair's own stored `trace-corrected.csv` / `state-row.csv`
(a path in the flags is resolved by its file name inside the pair's directory, so the command works from any
worktree), the tick count is `ours.ticks`, and the heading is NOT re-solved. `theirs.png` and the `original` block
are untouched. The checks that need only ours are re-run (settings read back, image size, pose at the rendered tick
against the stored original pose); a pair that fails is named, keeps its old `ours.png` and the exit status counts
it. On success `ours.png` and `side.png` are rewritten and `ours.binary` and `rerendered_at` updated.
"""

import argparse
import datetime
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402


def resolve_flags(flags, pair_dir):
    """The recorded flags with every path-looking token mapped to the file of that name in the pair's directory;
    `--ticks N` pairs are dropped (re-added from `ours.ticks`)."""
    out, skip = [], False
    for i, tok in enumerate(flags):
        if skip:
            skip = False
            continue
        if tok == "--ticks":
            skip = True
            continue
        if "/" in tok and not tok.startswith("-"):
            here = pair_dir / Path(tok).name
            tok = str(here) if here.exists() else tok
        elif tok.startswith("--pose-from="):
            tok = tok
        out.append(tok)
    return out


def rerender(pj, oag, profile_root):
    rec = json.loads(pj.read_text())
    pair_dir = pj.parent
    plat = ce.platform(rec["profile"])
    image = ROOT / plat["image"]
    flags = resolve_flags(rec["ours"]["flags"], pair_dir)
    ticks = rec["ours"]["ticks"]
    profile = profile_root / rec["profile"]
    ce.write_ours_settings(plat, profile)
    env = dict(os.environ, **ce.ours_env(profile))
    new_png = pair_dir / "ours.new.png"
    csv = pair_dir / "ours-rerender-trace.csv"
    base = [str(oag), str(image)] + flags
    problems = []
    for tail in (["--ticks", str(ticks), "--screenshot", str(new_png)],
                 ["--ticks", str(ticks + 1), "--trace-out", str(csv)]):
        done = subprocess.run(base + tail, env=env, capture_output=True, text=True)
        if done.returncode:
            return ["oag-game failed: " + done.stderr.strip().splitlines()[-1]]
    got, bad = ce.read_back(plat, profile)
    problems += bad
    size = ce.image_size(new_png)
    if size != list(plat["output"]):
        problems.append("ours image is %dx%d, profile %s" % (size[0], size[1], ce.size_text(plat)))
    row = ce.pose_of(ce.csv_row(csv, ticks))
    theirs = rec["original"]["pose"]
    tol = plat["tolerance"]
    d = ce.math.dist(theirs["pos"], row["pos"])
    a = ce.vec_angle(theirs["fwd"], row["fwd"])
    if d > tol["pos_units"]:
        problems.append("ship %.3f units off at the rendered tick (tolerance %.2f)" % (d, tol["pos_units"]))
    if a > tol["heading_deg"]:
        problems.append("heading %.3f deg off at the rendered tick (tolerance %.2f)" % (a, tol["heading_deg"]))
    if problems:
        new_png.unlink(missing_ok=True)
        return problems
    os.replace(new_png, pair_dir / "ours.png")
    subprocess.run(["magick", "montage", str(pair_dir / "theirs.png"), str(pair_dir / "ours.png"), "-tile", "2x1",
                    "-geometry", "+2+2", str(pair_dir / "side.png")], check=True)
    rec["ours"]["binary"] = ce.binary_record(oag)
    rec["ours"]["settings_readback"] = got
    rec["ours"]["pose"] = row
    rec["rerendered_at"] = datetime.datetime.now().astimezone().isoformat(timespec="seconds")
    rec["rerender_checks"] = {"pos_units": d, "heading_deg": a, "image_size": size}
    rec["environment"] = ce.environment_text(plat, rec)
    pj.write_text(json.dumps(rec, indent=2) + "\n")
    return []


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("dir", nargs="?", type=Path, default=ROOT / "data/reference/compare-page")
    ap.add_argument("--only", help="substring of the pair directory path")
    ap.add_argument("--oag-game", default=str(ROOT / "target/debug/oag-game"))
    ap.add_argument("--profile-dir", type=Path, default=ROOT / "data/compare-rerender-profile",
                    help="private XDG tree oag-game writes its settings into")
    args = ap.parse_args()
    failed = 0
    for pj in sorted(args.dir.rglob("pair.json")):
        if args.only and args.only not in str(pj.parent):
            continue
        if json.loads(pj.read_text()).get("refused"):
            continue
        problems = rerender(pj, args.oag_game, args.profile_dir)
        if problems:
            failed += 1
            print("FAILED %s: %s" % (pj.parent, "; ".join(problems)))
        else:
            print("ok     %s" % pj.parent)
    sys.exit(failed)


if __name__ == "__main__":
    main()
