#!/usr/bin/env python3
"""Re-render only OUR side of every comparison pair, leaving each original untouched.

    just compare-rerender-ours [--only <substring>] [--no-build] [--jobs N] [DIR]   # DIR default data/reference/compare-page

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

Pairs render `--jobs` at a time (default 3, each its own `oag-game` process); results print in pair order, with one
status line on a terminal naming the pairs still running.
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
import threading
from concurrent.futures import ThreadPoolExecutor
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_menu as cm  # noqa: E402

BINARY = ROOT / "target/debug/oag-game"
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
    # Pairs recorded with their own `--screenshot <path>` would meet the one appended below twice (a clap error).
    while "--screenshot" in flags:
        at = flags.index("--screenshot")
        del flags[at:at + 2]
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
    # A race shot (`--field-from`, drawn at tick 0) runs ours' animation on the offset alone: the original's own mesh
    # clock, its offset over the HUD clock plus the HUD clock. Pairs recorded before 2026-10-26 carry
    # `offset - GO_TICK / 60`, 43 s short on `pulse-psp-race/talons-alongside` (the left wall's neon strips dark).
    orig = rec.get("original") or {}
    if "--field-from" in flags and "--world-clock-offset" in flags and orig.get("world_clock_offset_s") is not None \
            and orig.get("race_clock_s") is not None and rec["profile"] == "pulse-psp":
        flags[flags.index("--world-clock-offset") + 1] = "%.3f" % (orig["world_clock_offset_s"] + orig["race_clock_s"])
    # A race shot's pickup row (the running Autopilot's icon and countdown): pairs recorded before 2026-10-26 draw none.
    # The countdown is the timer the pair script read at the stop (`autopilot_timer_at_stop_s`); it runs when the
    # newer `held_pickup.autopilot_running` says so or, in an older record, the provenance's `autopilot.armed`.
    held = orig.get("held_pickup") or {}
    running = held.get("autopilot_running", ((orig.get("provenance") or {}).get("autopilot") or {}).get("armed"))
    timer = orig.get("autopilot_timer_at_stop_s")
    if "--hud-state" in flags and "--hud-autopilot" not in flags and rec["profile"] == "pulse-psp" and running \
            and timer is not None and 0.0 < timer <= 600.0:
        flags += ["--hud-autopilot", "%.4f" % timer]
    # An HD pair's gantry pin is the RAM time less the presented lag (`ce.gantry_pin`); pairs made before 2026-10-27 pin the raw
    # RAM time, which sits on the GO board's rising edge on Tech de Ra and Modesto Heights.
    lag = (orig.get("presented_lag") or {}).get("hud_clock_ticks")
    if rec["profile"] == "hd-fury" and "--gantry-seconds" in flags and orig.get("gantry_seconds") is not None and lag is not None:
        flags[flags.index("--gantry-seconds") + 1] = "%.6f" % ce.gantry_pin(orig["gantry_seconds"], lag)
    plat = ce.platform(rec["profile"])
    # Pairs recorded before the profile held the exposure (2048) drew an adapting one; hold it as the profile says.
    if "--hold-exposure" not in flags:
        flags += ce.exposure_hold_flags(plat["ours"])
    # The pair scripts record the flags after the disc image (`base[2:]`): the image is the pair's `--image`, else its
    # profile's, the same rule the scripts apply.
    argv = rec.get("argv") or []
    image = argv[argv.index("--image") + 1] if "--image" in argv else str(ce.MAIN / plat["image"])
    with tempfile.TemporaryDirectory(prefix="oag-ours-") as tmp:
        tmp = Path(tmp)
        # The Safe Area the pair's original ran at (`original.safe_area`) overrides the profile's pin; a pair with none
        # recorded keeps the pin and the record says so. A menu pair's own extras (front-end styling, the race
        # selection a picker highlights) are put back too: without them a picker draws the title's first team.
        safe_area = ce.recorded_safe_area(rec)
        ce.write_ours_settings(plat, tmp / "profile", safe_area)
        if rec.get("kind") == "menu":
            style = rec["ours"].get("front_end_style")
            # a pair records prose for a title with no styling choice ("the disc's own"): only HD's two values are a setting
            cm.extra_settings(tmp / "profile", style if style in ("HD", "FURY") else None, rec["ours"].get("race"),
                              locked=rec["ours"].get("unlock_everything") is False)
        if "hud_safe_area" in plat["ours"]:
            rec["ours"]["safe_area_basis"] = ("the pair's recorded original value, %d%%" % safe_area if safe_area is not None
                                             else "no value recorded for this pair: the profile's %d%%" % plat["ours"]["hud_safe_area"])
        shot = tmp / "ours.png"
        env = dict(os.environ, **ce.ours_env(tmp / "profile"))
        done = subprocess.run([str(binary), image] + flags + ["--screenshot", str(shot)], env=env, cwd=ROOT,
                              capture_output=True, text=True, timeout=600)
        if done.returncode != 0 or not shot.exists():
            tail = (done.stderr.strip().splitlines() or ["no output"])[-1]
            return "FAILED (rc %d): %s" % (done.returncode, tail)
        if ce.image_size(shot) != ce.image_size(pair_dir / "theirs.png"):
            return "FAILED: ours is %s, theirs %s" % (ce.image_size(shot), ce.image_size(pair_dir / "theirs.png"))
        if "hud_safe_area" in plat["ours"]:  # the override reached the run: the file the game rewrote still says it
            want = safe_area if safe_area is not None else plat["ours"]["hud_safe_area"]
            got = tomllib.loads(ce.settings_path(tmp / "profile").read_text())["graphics"].get("hud_safe_area")
            if got != want:
                return "FAILED: ours ran with hud_safe_area %r, the pair says %r" % (got, want)
        offset = rec["ours"].get("display_offset") or ce.display_offset_record(rec["profile"], rec.get("kind"))
        if offset:
            offset["pixels"] = ce.apply_display_offset(shot, offset)
            rec["ours"]["display_offset"] = offset
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
    return "ok: " + changes(old, rec["overlap"], "%s/%s" % (pair_dir.parent.name, pair_dir.name))


#: `(key, label, format, lower is better)`, in the order printed.
SCORES = (("mean_grey", "mean grey", "%.2f", True), ("rmse", "rmse", "%.3f", True), ("edge_ncc", "edge", "%.2f", False))
COLOUR = (sys.stdout.isatty() and "NO_COLOR" not in os.environ) or bool(os.environ.get("FORCE_COLOR"))
GREEN, RED, DIM, RESET = "\033[32m", "\033[31m", "\033[2m", "\033[0m"
TALLY = {"better": 0, "worse": 0, "same": 0}
TALLY_LOCK = threading.Lock()
#: `(pair, old mean grey, new mean grey)` of every pair that had a previous score, for the closing summary.
MOVES = []
#: A mean grey move smaller than this is reported as small: a frame that differs by a few pixels' worth.
SMALL = 0.5
#: How many of the largest gains and regressions the summary names.
NAMED = 5


def paint(text, code):
    return code + text + RESET if COLOUR else text


def changes(old, new, pair):
    """Each score old -> new, green where it improved, red where it regressed and dim where the printed value did not
    move; the pair is tallied by its mean grey difference and kept for the summary."""
    parts = []
    for key, label, fmt, lower in SCORES:
        if key not in old or key not in new:
            continue
        a, b = fmt % old[key], fmt % new[key]
        verdict = "same" if a == b else "better" if (float(b) < float(a)) == lower else "worse"
        if key == "mean_grey":
            with TALLY_LOCK:
                TALLY[verdict] += 1
                MOVES.append((pair, old[key], new[key]))
        parts.append(paint("%s %s -> %s" % (label, a, b), {"same": DIM, "better": GREEN, "worse": RED}[verdict]))
    return ", ".join(parts) if parts else "no previous scores"


def summary():
    """The closing lines: the tally split by size, the summed mean grey per title and overall, and the largest gains
    and regressions by name, so a run with many small regressions and a few large wins reads as what it is."""
    if not MOVES:
        return []
    deltas = [(new - old, pair) for pair, old, new in MOVES]
    big_better = sum(1 for d, _ in deltas if d <= -SMALL)
    big_worse = sum(1 for d, _ in deltas if d >= SMALL)
    small = sum(1 for d, _ in deltas if 0 < abs(d) < SMALL)
    lines = ["mean grey moves: %s, %s, %s, %s" % (
        paint("%d better by %.1f+" % (big_better, SMALL), GREEN), paint("%d worse by %.1f+" % (big_worse, SMALL), RED),
        "%d within %.1f" % (small, SMALL), paint("%d unchanged" % sum(1 for d, _ in deltas if d == 0), DIM))]
    titles = {}
    for pair, old, new in MOVES:
        t = titles.setdefault(pair.split("/")[0], [0, 0.0, 0.0])
        t[0] += 1
        t[1] += old
        t[2] += new
    net = sum(d for d, _ in deltas)
    lines.append("summed mean grey: %.1f -> %.1f (%s), per title:" % (
        sum(o for _, o, _ in MOVES), sum(n for _, _, n in MOVES), paint("%+.1f" % net, GREEN if net < 0 else RED if net > 0 else DIM)))
    for name, (n, old, new) in sorted(titles.items(), key=lambda kv: kv[1][2] - kv[1][1]):
        if new != old:
            colour = GREEN if new < old else RED
            lines.append("  %-18s %3d pair(s)  %8.2f -> %8.2f  %s" % (name, n, old, new, paint("%+.2f" % (new - old), colour)))
    gains = sorted(d for d in deltas if d[0] <= -SMALL)[:NAMED]
    losses = sorted((d for d in deltas if d[0] >= SMALL), reverse=True)[:NAMED]
    if gains:
        lines.append("largest gains:   " + ", ".join(paint("%s %+.2f" % (p, d), GREEN) for d, p in gains))
    if losses:
        lines.append("largest regressions: " + ", ".join(paint("%s %+.2f" % (p, d), RED) for d, p in losses))
    return lines


def rewrite_manifests(d):
    """Every page manifest in `d`, so no page row keeps a score from before the re-render (a fixed list once left the
    `menus*.toml` rows at their old numbers)."""
    for toml in sorted(d.glob("*.toml")):
        dirs = [str((toml.parent / e["original"]).parent) for e in tomllib.loads(toml.read_text()).get("entry", [])]
        if dirs:
            ce.manifest(toml, dirs)


def render_all(pairs, root, jobs):
    """Renders `pairs` `jobs` at a time and prints each result in pair order as soon as every earlier one is in; on a
    terminal one status line below the rows names what is still running. Returns how many were not re-rendered."""
    live = sys.stdout.isatty()
    names = [str(pj.parent.relative_to(root)) for pj in pairs]
    results, running, cond = {}, set(), threading.Condition()

    def work(i):
        with cond:
            running.add(i)
            cond.notify()
        try:
            result = rerender(pairs[i], BINARY)
        except Exception as e:  # one pair's crash is that pair's row, not the run's end
            result = "FAILED: %s" % e
        with cond:
            running.discard(i)
            results[i] = result
            cond.notify()

    bad, printed = 0, 0
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        for i in range(len(pairs)):
            pool.submit(work, i)
        with cond:
            while printed < len(pairs):
                cond.wait_for(lambda: printed in results or bool(running))
                if live:
                    sys.stdout.write("\r\033[K")
                while printed in results:
                    result = results[printed]
                    bad += not result.startswith(("ok", "skipped: refused"))
                    if not result.startswith(("ok", "skipped")):
                        result = paint(result, RED)
                    print("%-55s %s" % (names[printed], result))
                    printed += 1
                if live and printed < len(pairs):
                    now = ", ".join(names[i] for i in sorted(running)) or "starting"
                    status = "[%d/%d] %s" % (printed, len(pairs), now)
                    width = shutil.get_terminal_size().columns - 1
                    sys.stdout.write(paint(status[:width], DIM))
                sys.stdout.flush()
                if printed < len(pairs) and printed not in results:
                    cond.wait()
    return bad


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("dir", nargs="?", type=Path, default=ROOT / "data/reference/compare-page")
    ap.add_argument("--only", help="substring of the pair directory path")
    ap.add_argument("--no-build", action="store_true", help="use target/debug/oag-game as it is")
    ap.add_argument("--no-page", action="store_true", help="leave the manifests and the page alone")
    ap.add_argument("--jobs", type=int, default=3, choices=range(1, 5), metavar="N",
                    help="pairs rendered at once, 1-4 (default 3)")
    args = ap.parse_args()
    if not args.no_build:
        subprocess.run(["cargo", "build", "-p", "oag-game"], cwd=ROOT, check=True)
    pairs = [pj for pj in sorted(args.dir.rglob("pair.json")) if not args.only or args.only in str(pj.parent)]
    bad = render_all(pairs, args.dir, args.jobs)
    if not args.no_page:
        rewrite_manifests(args.dir)
        subprocess.run([sys.executable, str(HERE / "compare-page.py"), str(args.dir), "--out", str(ROOT / "data/compare")], check=True)
    print("%d pair(s), %d not re-rendered; by mean grey: %s, %s, %s" % (
        len(pairs), bad, paint("%d better" % TALLY["better"], GREEN), paint("%d worse" % TALLY["worse"], RED),
        paint("%d same" % TALLY["same"], DIM)))
    for line in summary():
        print(line)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
