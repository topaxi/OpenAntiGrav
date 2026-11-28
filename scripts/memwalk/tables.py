#!/usr/bin/env python3
# docs/architecture/memory.md, "A finished race held under its replay (ADR-0099 step 5)".
"""Tabulate the runs of one cell (one title on one GPU or phone), in MiB.

    python3 scripts/memwalk/tables.py RUN_DIR...

Each figure is the median over a point's samples, then over the runs, with the
runs' range. `b-d` is the held-race estimate, taken run by run.
"""

import json
import statistics
import sys
from pathlib import Path

DESKTOP = ("VmRSS", "live", "vram", "gtt", "VmHWM")
PHONE = ("resident_swap", "VmRSS", "native_heap_alloc", "gpumem", "total_pss", "total_rss",
         "VmHWM")
ORDER = ("pre", "a", "b", "c", "e", "d", "d_trim")


def read(run):
    """One run as `{point: {field: MiB}}`."""
    by = {}
    for line in (run / "samples.jsonl").read_text().splitlines():
        row = json.loads(line)
        if row["window"]:
            row["resident_swap"] = row.get("VmRSS", 0) + row.get("VmSwap", 0)
            by.setdefault(row["window"], []).append(row)
    desktop = (run / "mallinfo.jsonl").exists()
    cell = {}
    for point, rows in by.items():
        cell[point] = {}
        for field in DESKTOP if desktop else PHONE:
            values = [r[field] for r in rows if field in r]
            if values:
                pick = max if field == "VmHWM" else statistics.median
                cell[point][field] = pick(values) / 1024
    if desktop:
        # `mallinfo2` readings: each follows the line naming its point.
        point = None
        for line in (run / "mallinfo.jsonl").read_text().splitlines():
            row = json.loads(line)
            if "window" in row:
                point = row["window"]
            else:
                cell.setdefault(point, {})["live"] = (row["uordblks"] + row["hblkhd"]) / 2**20
    return cell


def show(label, values):
    print(f"  {label:28} {statistics.median(values):7.0f}  ({min(values):.0f} to {max(values):.0f})")


def main(directories):
    runs = [read(Path(d)) for d in directories]
    if not runs:
        raise SystemExit(__doc__)
    print(f"{len(runs)} run(s)")
    fields = [f for f in dict.fromkeys(DESKTOP + PHONE)
              if any(f in cell for run in runs for cell in run.values())]
    for point in ORDER:
        for field in fields:
            values = [run[point][field] for run in runs if field in run.get(point, {})]
            if values:
                show(f"{point} {field}", values)
    for high, low in (("b", "d"), ("b", "pre"), ("b", "d_trim")):
        for field in fields:
            values = [run[high][field] - run[low][field] for run in runs
                      if field in run.get(high, {}) and field in run.get(low, {})]
            if values and field != "VmHWM":
                show(f"{high}-{low} {field}", values)


if __name__ == "__main__":
    main(sys.argv[1:])
