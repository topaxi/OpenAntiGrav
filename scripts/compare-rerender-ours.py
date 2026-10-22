#!/usr/bin/env python3
"""Re-render only OUR side of every comparison pair, leaving each original untouched.

    just compare-rerender-ours [--only <substring>] [--no-build] [DIR]   # DIR default data/reference/compare-page

No emulator runs. Each pair's `pair.json` records the exact `oag-game` flags that drew `ours.png` (`ours.flags`), and
the pose, camera, field and clock in those flags were read out of the original's RAM when `theirs.png` was captured.
Since `theirs.png` is kept, those recorded values are still the ones that match it, so this is the one place a stored
pose is reused (`compare-rerender` re-reads it because it re-captures the original too).

Per pair: a recorded path under a lane's scratch directory (`{scratch}/.../trace.csv`) or a reaped worktree is
resolved to the file of the same name in the pair directory; a pair whose input cannot be resolved is skipped and
named. The current `oag-game` (built first, the dev profile the pair scripts use) renders into a temporary file with
a private settings directory written from the pair's profile; on success `ours.png`, `side.png`, `ours.binary`,
`ours.rendered_at`, `overlap` and the environment are replaced, and the old overlap is kept as `previous_ours`. The
original's fields, provenance and checks are not touched. Then the page manifests are rewritten from the pairs they
already list and the page is rebuilt.
"""

import argparse
import datetime
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402

BINARY = ROOT / "target/debug/oag-game"
MANIFESTS = ("fresh.toml", "race-shots.toml", "zone.toml", "pure.toml")
#: A recorded path that no longer exists: a lane's scratch directory or a reaped worktree.
STALE = re.compile(r"\{scratch\}/|OpenAntiGrav-worktrees/|/data/scratch/")


def resolve_flags(flags, pair_dir):
    """`(flags, problems)`: every stale path replaced by the pair directory's file of the same name."""
    out, problems = [], []
    for f in flags:
        f = str(f)
        if STALE.search(f):
            local = pair_dir / Path(f).name
            if local.exists():
                f = str(local)
            else:
                problems.append("%s: no %s in the pair directory" % (f, Path(f).name))
        out.append(f)
    return out, problems


def rerender(pj, binary):
    rec = json.loads(pj.read_text())
    pair_dir = pj.parent
    if rec.get("refused"):
        return "skipped: refused pair"
    flags = (rec.get("ours") or {}).get("flags")
    if not flags:
        return "skipped: no recorded ours.flags"
    flags, problems = resolve_flags(flags, pair_dir)
    # The HD and PS2 pair scripts record the run length beside the flags (`ours.ticks`), not in them; without it the
    # frame is tick 0 (a clock of 0.00.0, no GO board).
    if "--ticks" not in flags and rec["ours"].get("ticks") is not None:
        flags += ["--ticks", str(rec["ours"]["ticks"])]
    # Pairs recorded before `--no-hull-shake` existed drew no shake; keep it that way (`compare_env.ours_flags`).
    if "--no-hull-shake" not in flags:
        flags += ["--no-hull-shake"]
    if problems:
        return "skipped: " + "; ".join(problems)
    plat = ce.platform(rec["profile"])
    # The pair scripts record the flags after the disc image (`base[2:]`): the image is the pair's `--image`, else its
    # profile's, the same rule the scripts apply.
    argv = rec.get("argv") or []
    image = argv[argv.index("--image") + 1] if "--image" in argv else str(ce.MAIN / plat["image"])
    with tempfile.TemporaryDirectory(prefix="oag-ours-") as tmp:
        tmp = Path(tmp)
        ce.write_ours_settings(plat, tmp / "profile")
        shot = tmp / "ours.png"
        env = dict(os.environ, **ce.ours_env(tmp / "profile"))
        done = subprocess.run([str(binary), image] + flags + ["--screenshot", str(shot)], env=env, cwd=ROOT,
                              capture_output=True, text=True, timeout=600)
        if done.returncode != 0 or not shot.exists():
            tail = (done.stderr.strip().splitlines() or ["no output"])[-1]
            return "FAILED (rc %d): %s" % (done.returncode, tail)
        if ce.image_size(shot) != ce.image_size(pair_dir / "theirs.png"):
            return "FAILED: ours is %s, theirs %s" % (ce.image_size(shot), ce.image_size(pair_dir / "theirs.png"))
        shutil.move(str(shot), pair_dir / "ours.png")
    subprocess.run(["magick", "montage", pair_dir / "theirs.png", pair_dir / "ours.png", "-tile", "2x1",
                    "-geometry", "+2+2", pair_dir / "side.png"], check=True)
    old = rec.get("overlap") or {}
    rec["previous_ours"] = {"overlap": old, "binary": rec["ours"].get("binary"), "rendered_at": rec["ours"].get("rendered_at")}
    rec["ours"]["flags"] = flags
    rec["ours"]["image"] = image
    rec["ours"]["binary"] = ce.binary_record(binary)
    rec["ours"]["rendered_at"] = datetime.datetime.now().astimezone().isoformat(timespec="seconds")
    rec["overlap"] = ce.overlap_score(pair_dir / "theirs.png", pair_dir / "ours.png")
    rec["environment"] = ce.environment_text(plat, rec)
    pj.write_text(json.dumps(rec, indent=2) + "\n")
    return "ok: " + changes(old, rec["overlap"])


#: `(key, label, format, lower is better)`, in the order printed.
SCORES = (("mean_grey", "mean grey", "%.2f", True), ("rmse", "rmse", "%.3f", True), ("edge_ncc", "edge", "%.2f", False))
COLOUR = (sys.stdout.isatty() and "NO_COLOR" not in os.environ) or bool(os.environ.get("FORCE_COLOR"))
GREEN, RED, DIM, RESET = "\033[32m", "\033[31m", "\033[2m", "\033[0m"
TALLY = {"better": 0, "worse": 0, "same": 0}


def paint(text, code):
    return code + text + RESET if COLOUR else text


def changes(old, new):
    """Each score old -> new, green where it improved, red where it regressed and dim where the printed value did not
    move; the pair is tallied by its mean grey difference."""
    parts = []
    for key, label, fmt, lower in SCORES:
        if key not in old or key not in new:
            continue
        a, b = fmt % old[key], fmt % new[key]
        verdict = "same" if a == b else "better" if (float(b) < float(a)) == lower else "worse"
        if key == "mean_grey":
            TALLY[verdict] += 1
        parts.append(paint("%s %s -> %s" % (label, a, b), {"same": DIM, "better": GREEN, "worse": RED}[verdict]))
    return ", ".join(parts) if parts else "no previous scores"


def rewrite_manifests(d):
    for name in MANIFESTS:
        toml = d / name
        if not toml.exists():
            continue
        dirs = [str((toml.parent / e["original"]).parent) for e in tomllib.loads(toml.read_text()).get("entry", [])]
        if dirs:
            ce.manifest(toml, dirs)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("dir", nargs="?", type=Path, default=ROOT / "data/reference/compare-page")
    ap.add_argument("--only", help="substring of the pair directory path")
    ap.add_argument("--no-build", action="store_true", help="use target/debug/oag-game as it is")
    ap.add_argument("--no-page", action="store_true", help="leave the manifests and the page alone")
    args = ap.parse_args()
    if not args.no_build:
        subprocess.run(["cargo", "build", "-p", "oag-game"], cwd=ROOT, check=True)
    pairs = [pj for pj in sorted(args.dir.rglob("pair.json")) if not args.only or args.only in str(pj.parent)]
    bad = 0
    for pj in pairs:
        result = rerender(pj, BINARY)
        bad += not result.startswith(("ok", "skipped: refused"))
        if not result.startswith(("ok", "skipped")):
            result = paint(result, RED)
        print("%-55s %s" % (pj.parent.relative_to(args.dir), result), flush=True)
    if not args.no_page:
        rewrite_manifests(args.dir)
        subprocess.run([sys.executable, str(HERE / "compare-page.py"), str(args.dir), "--out", str(ROOT / "data/compare")], check=True)
    print("%d pair(s), %d not re-rendered; by mean grey: %s, %s, %s" % (
        len(pairs), bad, paint("%d better" % TALLY["better"], GREEN), paint("%d worse" % TALLY["worse"], RED),
        paint("%d same" % TALLY["same"], DIM)))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
