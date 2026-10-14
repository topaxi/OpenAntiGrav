#!/usr/bin/env python3
"""Whole-race rendering benchmark: the race table, the runs, the comparison.

    scripts/bench-render.py run [--out DIR] [--tag NAME] [--profile release|debug]
                                [--repeat] [--views 1,2h,2v] [--size WxH] [title ...]
    scripts/bench-render.py compare A B        (two result files, or two run directories)
    scripts/bench-render.py selftest

`run` drives `oag-game --race` with `OAG_RENDER_BENCH_RACE` set (see
`crates/game/src/race_capture/race_bench.rs`), one race per title, and leaves
`<title>.json` (summary and reproduction facts) and `<title>.csv` (every frame)
in the output directory. `--views` picks the cases: `1` one view (the file is
`<title>.json`), `2h` two views cut horizontally and `2v` two side by side (the
files are `<title>-2h.json`, `<title>-2v.json`), so `compare` lines the same case
up across two runs. Guide: docs/tools/render-bench.md.

**It takes the gate lock** (`$HOME/.cache/oag/gate.lock`, the one `just
gate-affected` and `just test-data-affected` hold) for the runs only, so a bench
runs on a quiet machine and never overlaps a test suite. `OAG_BENCH_NO_LOCK=1`
skips it, for a human on an idle machine. Never run this from inside a gate, or
wrap a gate in it: the same lock held twice is a deadlock.
"""

import argparse
import datetime
import fcntl
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
LOCK = pathlib.Path(os.environ.get("HOME", "/tmp")) / ".cache/oag/gate.lock"

# The fixed race of each title. **Chosen, not measured**: nothing here is read
# off an original, it is a repeatable workload. Every title races its own
# default circuit (the loader's `racing on ...` line is recorded in the result),
# in a single race against the full AI field, the player's craft flown by the
# autopilot from the start to its own last crossing, the speed class the loader
# defaults to, seed 1, a 1280x720 offscreen target, and every render setting at
# its default (the settings file is a throwaway, so the defaults are all there
# is). Change a row and the results stop being comparable with the old ones.
COMMON = [
    "--race",
    "--mode", "single_race",
    "--seed", "1",
    "--autopilot",
    "--no-audio",
    "--log-every", "0",
]
# Safety cap in ticks: three laps are about 7,500 ticks on Pulse; a race that is
# still going at the cap reports `finished: false` instead of hanging.
TICK_CAP = 40000
TITLES = {
    "pulse": {"source": "data/images/pulse-psp-eu.chd", "extra": []},
    "pure": {"source": "data/images/pure-psp-eu.chd", "extra": []},
    "hd": {"source": "data/images/hdfury-ps3-eu-dec.iso", "extra": []},
    "2048": {"source": "data/extracted/vita/PCSF00007", "extra": []},
    "omega": {"source": "data/extracted/ps4", "extra": []},
}

METRICS = ["tick_us", "record_us", "submit_us", "cpu_us", "frame_us", "gpu_us", "draws", "triangles"]
DEFAULT_SIZE = "1280x720"
# Case name -> the flags that make it. `1` adds none, so a one-view result is
# exactly what it was before the cases existed.
CASES = {
    "1": [],
    "2h": ["--players", "2", "--split-cut", "horizontal"],
    "2v": ["--players", "2", "--split-cut", "vertical"],
}
STATS = ["p50", "p95", "p99", "max"]


def load_average():
    try:
        return " ".join(pathlib.Path("/proc/loadavg").read_text().split()[:3])
    except OSError:
        return "unavailable"


class GateLock:
    """The shared gate lock as `flock -o` takes it: exclusive, blocking."""

    def __init__(self, enabled):
        self.enabled = enabled
        self.waited = 0.0
        self.fd = None

    def __enter__(self):
        if not self.enabled:
            return self
        LOCK.parent.mkdir(parents=True, exist_ok=True)
        self.fd = os.open(LOCK, os.O_RDWR | os.O_CREAT, 0o644)
        start = time.monotonic()
        try:
            fcntl.flock(self.fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            print(f"bench: waiting for the gate lock {LOCK} (a gate or test run holds it; this is not a hang)", flush=True)
            fcntl.flock(self.fd, fcntl.LOCK_EX)
        self.waited = time.monotonic() - start
        return self

    def __exit__(self, *_):
        if self.fd is not None:
            os.close(self.fd)


def run_one(binary, title, out, profile, case="1", size=DEFAULT_SIZE, repeat_name=None):
    spec = TITLES[title]
    source = ROOT / spec["source"]
    if not source.exists():
        return None, f"{source} is missing"
    name = title if case == "1" else f"{title}-{case}"
    stem = out / (repeat_name or name)
    flags = [*COMMON, "--size", size, *CASES[case]]
    tmp = pathlib.Path(tempfile.mkdtemp(prefix=f"oag-bench-{title}-"))
    try:
        env = dict(os.environ)
        env["OAG_RENDER_BENCH_RACE"] = str(stem)
        # A finished race writes records and ghosts; never into the player's saves.
        for var, name in (("XDG_CONFIG_HOME", "config"), ("XDG_DATA_HOME", "data"), ("XDG_STATE_HOME", "state"), ("XDG_CACHE_HOME", "cache")):
            env[var] = str(tmp / name)
        cmd = [str(binary), str(source), *flags, *spec["extra"], "--ticks", str(TICK_CAP), "--screenshot", str(tmp / "unused.png")]
        start = time.monotonic()
        done = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True)
        wall = time.monotonic() - start
        log = done.stdout + done.stderr
        if done.returncode != 0 or not pathlib.Path(f"{stem}.json").exists():
            return None, f"exit {done.returncode}; last lines:\n" + "\n".join(log.splitlines()[-8:])
        racing = re.search(r"racing on (.*)", log)
        result = json.loads(pathlib.Path(f"{stem}.json").read_text())
        result["run"] = {
            "title_key": title,
            "source": spec["source"],
            "track": racing.group(1).strip() if racing else None,
            "case": case,
            "command": [str(pathlib.Path(cmd[0]).name), spec["source"], *flags, *spec["extra"], "--ticks", str(TICK_CAP)],
            "env_note": "OAG_RENDER_BENCH_RACE set; XDG dirs on a throwaway directory (defaults for every setting)",
            "profile": profile,
            "tick_cap_chosen": TICK_CAP,
            "process_seconds": round(wall, 2),
        }
        pathlib.Path(f"{stem}.json").write_text(json.dumps(result, indent=2) + "\n")
        return result, None
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def cmd_run(args):
    profile = args.profile
    binary = ROOT / "target" / profile / "oag-game"
    if not binary.exists():
        sys.exit(f"{binary} is missing: build it first (cargo build --{profile if profile == 'release' else 'profile dev'} -p oag-game), or use `just bench-render`")
    cases = [c.strip() for c in args.views.split(",") if c.strip()]
    for case in cases:
        if case not in CASES:
            sys.exit(f"unknown view case {case!r}; known: {', '.join(CASES)}")
    titles = args.titles or list(TITLES)
    for title in titles:
        if title not in TITLES:
            sys.exit(f"unknown title {title!r}; known: {', '.join(TITLES)}")
    tag = args.tag or datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    out = pathlib.Path(args.out) if args.out else ROOT / "data" / "bench" / tag
    out.mkdir(parents=True, exist_ok=True)
    enabled = os.environ.get("OAG_BENCH_NO_LOCK", "") in ("", "0")
    failures = 0
    with GateLock(enabled) as lock:
        print(f"bench: gate lock {'held' if enabled else 'SKIPPED (OAG_BENCH_NO_LOCK)'}; load average {load_average()}; results in {out}", flush=True)
        for title, case in [(t, c) for t in titles for c in cases]:
            name = title if case == "1" else f"{title}-{case}"
            first, why = run_one(binary, title, out, profile, case, args.size)
            if first is None:
                print(f"bench: {name}: FAILED: {why}")
                failures += 1
                continue
            first["run"]["lock"] = {"held": enabled, "waited_seconds": round(lock.waited, 1)}
            first["run"]["load_average_at_run_start"] = first["load_average_start"]
            pathlib.Path(out / f"{name}.json").write_text(json.dumps(first, indent=2) + "\n")
            print(summary_line(name, first), flush=True)
            if args.repeat:
                second, why = run_one(binary, title, out, profile, case, args.size, repeat_name=f"{name}-repeat")
                if second is None:
                    print(f"bench: {name}: repeat FAILED: {why}")
                    failures += 1
                    continue
                same = (first["final_tick"], first["final_state_hash"], first["frames"]) == (second["final_tick"], second["final_state_hash"], second["frames"])
                print(f"bench: {name}: determinism {'OK' if same else 'MISMATCH'}: tick {first['final_tick']} / {second['final_tick']}, hash {first['final_state_hash']} / {second['final_state_hash']}")
                failures += 0 if same else 1
    return 1 if failures else 0


def summary_line(title, result):
    m = result["metrics"]
    cell = lambda name, stat: (f"{m[name][stat]:.0f}" if m.get(name) else "n/a")
    views = " ".join(
        f"v{n} {m[k]['p50']:.0f}/{m[k]['p99']:.0f}"
        for n in range(8)
        if m.get((k := f"gpu_view{n}_us"))
    )
    return (
        f"bench: {title}: {result['frames']} frames, finished {result['finished']}, "
        f"cpu p50/p99 {cell('cpu_us', 'p50')}/{cell('cpu_us', 'p99')} us, "
        f"gpu p50/p99 {cell('gpu_us', 'p50')}/{cell('gpu_us', 'p99')} us, "
        f"frame p50/p99 {cell('frame_us', 'p50')}/{cell('frame_us', 'p99')} us "
        f"{('gpu per view p50/p99 us: ' + views + ' ') if views else ''}"
        f"({result['adapter']['name']}, load {result['load_average_start']} -> {result['load_average_end']})"
    )


def pct(a, b):
    """The change from a to b as a percentage; None when a is zero or missing."""
    if a is None or b is None or a == 0:
        return None
    return (b - a) / a * 100.0


def compare_pair(a, b):
    lines = []
    warnings = []
    for key in ("title", "mode", "laps_target", "tick_cap", "settings"):
        if a.get(key) != b.get(key):
            warnings.append(f"{key} differs: {a.get(key)} vs {b.get(key)}")
    if a.get("run", {}).get("case", "1") != b.get("run", {}).get("case", "1"):
        warnings.append(f"view case differs: {a.get('run', {}).get('case', '1')} vs {b.get('run', {}).get('case', '1')}")
    if a.get("run", {}).get("track") != b.get("run", {}).get("track"):
        warnings.append(f"track differs: {a.get('run', {}).get('track')} vs {b.get('run', {}).get('track')}")
    if a["adapter"]["name"] != b["adapter"]["name"] or a["adapter"]["driver_info"] != b["adapter"]["driver_info"]:
        warnings.append(f"adapter differs: {a['adapter']['name']} {a['adapter']['driver_info']} vs {b['adapter']['name']} {b['adapter']['driver_info']}")
    if (a["final_tick"], a["final_state_hash"]) != (b["final_tick"], b["final_state_hash"]):
        warnings.append(f"the race itself differs (tick {a['final_tick']} vs {b['final_tick']}, hash {a['final_state_hash']} vs {b['final_state_hash']}): the sim changed, not only the renderer")
    lines.append(f"{a['title']}  A: {a['git_hash']} ({a['build']}), load {a['load_average_start']} -> {a['load_average_end']}")
    lines.append(f"{' ' * len(a['title'])}  B: {b['git_hash']} ({b['build']}), load {b['load_average_start']} -> {b['load_average_end']}")
    for w in warnings:
        lines.append(f"  WARNING: {w}")
    header = f"  {'metric':<14}{'stat':<6}{'A':>12}{'B':>12}{'delta':>12}{'%':>9}"
    lines.append(header)
    extra = sorted({k for r in (a, b) for k in r["metrics"] if k.startswith("gpu_view")})
    for metric in METRICS + extra:
        ma, mb = a["metrics"].get(metric), b["metrics"].get(metric)
        if not ma or not mb:
            lines.append(f"  {metric:<14}{'(not timed in one of the runs)':<6}")
            continue
        for stat in STATS:
            va, vb = ma[stat], mb[stat]
            change = pct(va, vb)
            lines.append(
                f"  {metric:<14}{stat:<6}{va:>12.1f}{vb:>12.1f}{vb - va:>+12.1f}"
                + (f"{change:>+8.1f}%" if change is not None else f"{'n/a':>9}")
            )
    return "\n".join(lines)


def cmd_compare(args):
    a, b = pathlib.Path(args.a), pathlib.Path(args.b)
    if a.is_dir() and b.is_dir():
        names = sorted({p.name for p in a.glob("*.json")} & {p.name for p in b.glob("*.json")})
        names = [n for n in names if "-repeat" not in n]
        if not names:
            sys.exit("no result file name is in both directories")
        pairs = [(a / n, b / n) for n in names]
    elif a.is_file() and b.is_file():
        pairs = [(a, b)]
    else:
        sys.exit("compare takes two result files or two run directories")
    for left, right in pairs:
        print(compare_pair(json.loads(left.read_text()), json.loads(right.read_text())))
        print()
    return 0


def cmd_selftest(_):
    import unittest

    class Tests(unittest.TestCase):
        def test_pct(self):
            self.assertEqual(pct(100.0, 150.0), 50.0)
            self.assertEqual(pct(200.0, 100.0), -50.0)
            self.assertIsNone(pct(0, 5))
            self.assertIsNone(pct(None, 5))

        def result(self, scale, tick=100):
            stat = lambda v: {"count": 10, "min": v, "mean": v, "p50": v, "p95": v * 2, "p99": v * 3, "max": v * 4}
            return {
                "title": "T", "git_hash": "abc", "build": "release", "mode": "single_race", "laps_target": 3,
                "tick_cap": 1, "settings": {}, "final_tick": tick, "final_state_hash": "ff",
                "load_average_start": "1 1 1", "load_average_end": "1 1 1",
                "adapter": {"name": "gpu", "driver_info": "x"},
                "metrics": {m: stat(scale) for m in METRICS},
            }

        def test_compare_reports_percent(self):
            text = compare_pair(self.result(100.0), self.result(110.0))
            self.assertIn("+10.0%", text)
            self.assertNotIn("WARNING", text)

        def test_a_split_case_compares_its_views(self):
            left, right = self.result(100.0), self.result(110.0)
            for r in (left, right):
                r["run"] = {"case": "2h"}
                r["metrics"]["gpu_view0_us"] = r["metrics"]["gpu_us"]
                r["metrics"]["gpu_view1_us"] = r["metrics"]["gpu_us"]
            text = compare_pair(left, right)
            self.assertIn("gpu_view1_us", text)
            right["run"] = {"case": "2v"}
            self.assertIn("view case differs", compare_pair(left, right))

        def test_a_different_race_is_flagged(self):
            text = compare_pair(self.result(100.0), self.result(100.0, tick=101))
            self.assertIn("the race itself differs", text)

    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Tests)
    return 0 if unittest.TextTestRunner(verbosity=1).run(suite).wasSuccessful() else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run", help="bench the named titles (default: all)")
    run.add_argument("titles", nargs="*", help=f"one or more of: {', '.join(TITLES)}")
    run.add_argument("--out", help="output directory (default data/bench/<tag>)")
    run.add_argument("--tag", help="run name, the default output directory's name (default: a timestamp)")
    run.add_argument("--profile", choices=["release", "debug"], default="release")
    run.add_argument("--views", default="1", help="cases to run, comma separated: 1 (one view), 2h (two views cut horizontally), 2v (side by side); default 1")
    run.add_argument("--size", default=DEFAULT_SIZE, help=f"offscreen target WxH (default {DEFAULT_SIZE}; the Steam Deck's panel is 1280x800)")
    run.add_argument("--repeat", action="store_true", help="run each title twice and check tick count and final state hash agree")
    run.set_defaults(fn=cmd_run)
    compare = sub.add_parser("compare", help="deltas between two result files or run directories")
    compare.add_argument("a")
    compare.add_argument("b")
    compare.set_defaults(fn=cmd_compare)
    sub.add_parser("selftest", help="unit tests for the comparison").set_defaults(fn=cmd_selftest)
    args = parser.parse_args()
    sys.exit(args.fn(args))


if __name__ == "__main__":
    main()
