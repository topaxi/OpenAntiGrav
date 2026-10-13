#!/usr/bin/env python3
"""Read a `psp-ai-track-log.py` capture against a track's spline and report the passes of a zone.

For every craft it finds the nearest spline row at each sample (a windowed search, so a
lap wrap and a respawn are both visible as what they are), then reports each pass of the
zone `--zone LO HI` (spline rows): the approach speed and AI thrust a few rows before it,
whether the craft reached row `HI` afterwards, and every **reset** (a position that jumps
farther in one sample than a craft can travel). The spline is `oag-trace track`'s CSV:

    target/debug/oag-trace track --source data/images/pulse-psp-usa.chd \
        --track 'Data\\Environments\\13_Track\\track.vex' > 13_track.csv
    python3 scripts/psp-ai-track-analyze.py cap.jsonl 13_track.csv --zone 19 49

The rows are not the engine's racing-line indices one for one (the CSV resamples the spline
four to an interval), but on `13_Track` row 19-49 lands on the same stretch of road as
the engine's authored jump at line indices 19-49 (y 20 falling away), which is what the
default is for.
"""

import argparse
import csv
import json
import math
import sys


def load_spline(path):
    rows = list(csv.DictReader(open(path)))
    return [
        (float(r["line_x"]), float(r["line_y"]), float(r["line_z"])) for r in rows
    ], [(float(r["lat_x"]), float(r["lat_y"]), float(r["lat_z"])) for r in rows]


def nearest(spline, p, around=None, window=120):
    n = len(spline)
    if around is None:
        candidates = range(n)
    else:
        candidates = [(around + k) % n for k in range(-window, window + 1)]
    best = None
    for i in candidates:
        q = spline[i]
        d = (q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2 + (q[2] - p[2]) ** 2
        if best is None or d < best[0]:
            best = (d, i)
    return best[1], math.sqrt(best[0])


def passes_of(spline, rows, slot, lo, hi, reset_distance):
    """Walk one slot's samples: zone entries, resets, and which entry each reset belongs to."""
    n = len(spline)
    around = None
    prev = None
    entries = []  # each: dict(t, row, speed, thrust, place, y, fell_t)
    trail = []  # every sample, for the pad lookback
    resets = []
    inside = False
    for r in rows:
        s = r["slots"][slot]
        p = (s[0], s[1], s[2])
        if prev is not None and math.dist(p, prev["p"]) > reset_distance:
            resets.append({"t": r["t"], "row": prev["row"], "from": prev["p"], "to": p, "place": int(s[11])})
            around = None
            inside = False
        idx, dist = nearest(spline, p, around)
        around = idx
        row = {"p": p, "row": idx, "t": r["t"], "speed": s[6], "thrust": s[7], "place": int(s[11]), "y": s[1], "progress": s[10]}
        trail.append(row)
        if lo <= idx <= hi and not inside:
            inside = True
            entries.append({**row, "fell_t": None, "exit": None, "min_y": s[1], "trail_end": len(trail) - 1})
        elif inside and entries:
            if entries[-1]["exit"] is None:
                entries[-1]["min_y"] = min(entries[-1]["min_y"], s[1])
            if idx > hi and idx < hi + 200 and entries[-1]["exit"] is None:
                entries[-1]["exit"] = row
            elif idx < lo - 5:
                inside = False
        prev = row
    for reset in resets:
        before = [e for e in entries if 0 <= reset["t"] - e["t"] <= 3.0]
        if before:
            before[-1]["fell_t"] = reset["t"]
    return entries, resets, trail


def pad_crossing(spline, lateral, trail, end, pad_row, window=400):
    """(signed lateral offset at the pad's row, speed 25 rows before, speed 5 rows after), or None."""
    n = len(spline)
    best = None
    for k in range(end, max(end - window, 0), -1):
        d = (trail[k]["row"] - pad_row) % n
        d = d if d < n // 2 else d - n
        if d <= 0:
            best = k
            break
    if best is None:
        return None
    here = trail[best]
    row = here["row"]
    off = [here["p"][i] - spline[row][i] for i in range(3)]
    lat = sum(off[i] * lateral[row][i] for i in range(3))
    before = next((trail[k]["speed"] for k in range(best, max(best - 80, 0), -1) if (pad_row - trail[k]["row"]) % n >= 25), here["speed"])
    after = next((trail[k]["speed"] for k in range(best, min(best + 80, len(trail))) if (trail[k]["row"] - pad_row) % n >= 5 and (trail[k]["row"] - pad_row) % n < n // 2), here["speed"])
    return (lat, before, after)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("capture")
    parser.add_argument("spline")
    parser.add_argument("--zone", type=int, nargs=2, default=(19, 49), metavar=("LO", "HI"))
    parser.add_argument("--reset-distance", type=float, default=12.0, help="units between samples that count as a reset")
    parser.add_argument("--pad-row", type=int, help="spline row of a speed pad just before the zone: report each entry's signed lateral offset there and the speed gained across it")
    parser.add_argument("-v", "--verbose", action="store_true", help="list every pass")
    args = parser.parse_args()
    lo, hi = args.zone

    spline, lateral = load_spline(args.spline)
    rows = [r for r in map(json.loads, open(args.capture)) if "slots" in r]
    meta = rows[0]["meta"]
    player = meta["player_racer"]
    base, stride = meta["racer_base"], meta["racer_stride"]
    print(
        "capture %s: %d rows, race %.1f to %.1f s, skill level %d, laps %d, lap length %.0f, start offset %.0f"
        % (args.capture, len(rows), rows[0]["t"], rows[-1]["t"], meta["skill_level"], meta["laps"], meta["lap_length"], meta["start_offset"])
    )
    total_falls = total_passes = 0
    for slot in range(meta["racers"]):
        if base + slot * stride == player:
            continue
        entries, resets, trail = passes_of(spline, rows, slot, lo, hi, args.reset_distance)
        for e in entries:
            e["pad"] = pad_crossing(spline, lateral, trail, e["trail_end"], args.pad_row) if args.pad_row is not None else None
        falls = [e for e in entries if e["fell_t"] is not None]
        clears = [e for e in entries if e["fell_t"] is None]
        total_falls += len(falls)
        total_passes += len(entries)
        print("\nslot %d: %d zone entries, %d fell (reset within 3 s), %d cleared, %d resets in all" % (slot, len(entries), len(falls), len(clears), len(resets)))
        def line(e, tag):
            pad = ""
            if e.get("pad"):
                pad = "  | pad: lat %+5.1f speed %5.1f -> %5.1f" % e["pad"]
            return "  %-7s t=%6.1f place %d  speed %6.1f thrust %5.1f  min_y %6.1f%s" % (tag, e["t"], e["place"], e["speed"], e["thrust"], e["min_y"], pad)
        shown = entries if args.verbose else entries[:3] + [e for e in entries[3:] if e["fell_t"] is None]
        for e in shown:
            print(line(e, "FELL" if e["fell_t"] is not None else "cleared"))
        if falls:
            sp = [e["speed"] for e in falls]
            print("  fell at entry speed %.1f .. %.1f (thrust %s)" % (min(sp), max(sp), sorted({round(e["thrust"], 1) for e in falls})))
        if clears:
            sp = [e["speed"] for e in clears]
            print("  cleared at entry speed %.1f .. %.1f (thrust %s)" % (min(sp), max(sp), sorted({round(e["thrust"], 1) for e in clears})))
    print("\nfield: %d entries, %d fell" % (total_passes, total_falls))


if __name__ == "__main__":
    sys.exit(main())
