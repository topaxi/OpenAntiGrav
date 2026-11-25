#!/usr/bin/env python3
"""Split an Android `--frame-log` run made with `OAG_SKIP_PASSES_TOGGLE` into its two arms.

    scripts/android-skip-windows.py LOGCAT PERIOD [--from S] [--to S]

LOGCAT is `adb logcat -v threadtime -s oag RustStdoutStderr` of one run whose launch file
set `OAG_SKIP_PASSES=<names>` and `OAG_SKIP_PASSES_TOGGLE=<PERIOD>` (`oag_gpu::skip`). The
windows count from the `OAG_SKIP_PASSES:` line; odd windows leave the passes out. A
`frame-log` line is the mean of the second before it, so a line whose second straddles a
window edge is dropped. --from/--to bound the race seconds kept, counted from the first
`stage=race` line (default 15 to the end: the flyby and countdown first).

Prints, per arm: rows, fps (median and mean), mean frame_ms, per-second p99, scene GPU ms,
present wait, and the means of the `frame-phases` lines. The difference between the arms is
what the named passes cost (docs/tools/android.md, "Where an HD frame goes on the S21").
"""

import argparse
import re
import statistics as st
import sys


def seconds(stamp):
    h, m, s = stamp.split(":")
    return int(h) * 3600 + int(m) * 60 + float(s)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("logcat")
    ap.add_argument("period", type=float)
    ap.add_argument("--from", dest="lo", type=float, default=15.0)
    ap.add_argument("--to", dest="hi", type=float, default=float("inf"))
    args = ap.parse_args()

    origin = race = None
    rows = {0: [], 1: []}
    phases = {0: [], 1: []}
    frame_log = re.compile(
        r"frame-log stage=race fps=([\d.]+) frame_ms=([\d.]+) p99_ms=([\d.]+) "
        r"cpu_ms=\S+ gpu_ms=(\S+) present_ms=(\S+)"
    )
    for line in open(args.logcat, errors="replace"):
        parts = line.split()
        if len(parts) < 3 or ":" not in parts[1]:
            continue
        t = seconds(parts[1])
        if "OAG_SKIP_PASSES:" in line and origin is None:
            origin = t
            continue
        if origin is None:
            continue
        log = frame_log.search(line)
        phase = re.search(r"frame-phases (.*)", line)
        if not (log or phase):
            continue
        if log and race is None:
            race = t
        if race is None or not (args.lo <= t - race < args.hi):
            continue
        window = int((t - origin) // args.period)
        if window != int((t - 1.0 - origin) // args.period):
            continue
        arm = window % 2
        if log:
            gpu = None if log[4] == "-" else float(log[4])
            present = None if log[5] == "-" else float(log[5])
            rows[arm].append((float(log[1]), float(log[2]), float(log[3]), gpu, present))
        else:
            phases[arm].append(
                {k: float(v) for k, v in (kv.split("=") for kv in phase[1].split())}
            )
    if origin is None:
        sys.exit("no OAG_SKIP_PASSES line: was the run made with OAG_SKIP_PASSES set?")

    def mean(values):
        values = [v for v in values if v is not None]
        return st.mean(values) if values else float("nan")

    for arm, name in ((0, "passes drawn"), (1, "passes left out")):
        r = rows[arm]
        if not r:
            print(f"{name}: no rows")
            continue
        print(
            f"{name:16s} n={len(r):3d} fps median {st.median(x[0] for x in r):5.1f} "
            f"mean {st.mean(x[0] for x in r):5.1f}  frame_ms {mean(x[1] for x in r):5.1f}  "
            f"p99 {mean(x[2] for x in r):5.1f}  scene_gpu {mean(x[3] for x in r):5.1f}  "
            f"present {mean(x[4] for x in r):5.1f}"
        )
        if phases[arm]:
            keys = list(phases[arm][0])
            print(
                "    phases "
                + " ".join(f"{k}={mean(d.get(k) for d in phases[arm]):.2f}" for k in keys)
            )


if __name__ == "__main__":
    main()
