#!/usr/bin/env python3
"""Replay every surface probe of a `psp-projectile-sweeps.py --box` capture against the firing craft's box.

`Collision_SweepSegment` (`0x0883198c`) answers `0x7f` (nothing) when the nearest thing its mesh
raycast meets is a craft's box collider (`hit.kind == 3`). `Collision_RaycastBox` (`0x08817b1c`)
reaches a box face only after a separating-axis test whose frame is built with `vtfm4.q C000, E100,
C200` over the rows loaded from `collider+0x80`; read live (`docs/ghidra/functions/psp-pulse-usa/
rocket-visuals.md`, "The own-craft box is a rotated frame"), that frame is `R^T * (p - centre)`, the
transpose of the one that would put a point in the box's own axes. The model checked here:

    hit = SAT(R^T-frame segment against the half dimensions)      # the original's gate
          and the segment crosses a face of the true oriented box  # its 12 triangles
          and that crossing is nearer than the mesh hit, if any

`--frame rows` replays the same capture with the correct frame; it mispredicts every probe whose
craft is yawed off the track axes by a non-multiple of 90 degrees.

    python3 scripts/psp-craft-box-replay.py data/traces/pulse-own-craft-box/*.json [-v] [--frame rows|cols]

(captures from `psp-projectile-sweeps.py --box ... --out FILE`; the 2026-11-12 set is 56 files.)

Exit status 1 when the chosen frame mispredicts any probe.
"""

import argparse
import json
import math
import struct
import sys

PROBE_LENGTH = {"rocket": 6.0}
DEFAULT_PROBE_LENGTH = 12.0


def floats(blob, offset, count):
    return struct.unpack_from("<%df" % count, blob, offset)


def sub(a, b):
    return [x - y for x, y in zip(a, b)]


def add(a, b):
    return [x + y for x, y in zip(a, b)]


def scale(a, s):
    return [x * s for x in a]


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def separating_axis_passes(start, end, extent):
    """The decompiled test: segment midpoint and half direction against the half dimensions."""
    mid = scale(add(start, end), 0.5)
    half = sub(mid, start)
    a = [abs(v) for v in half]
    if any(abs(mid[i]) > a[i] + extent[i] for i in range(3)):
        return False
    crosses = (
        (abs(mid[1] * half[2] - mid[2] * half[1]), a[2] * extent[1] + a[1] * extent[2]),
        (abs(mid[0] * half[2] - mid[2] * half[0]), a[2] * extent[0] + a[0] * extent[2]),
        (abs(mid[0] * half[1] - mid[1] * half[0]), a[1] * extent[0] + a[0] * extent[1]),
    )
    return all(lhs <= rhs for lhs, rhs in crosses)


def face_crossing(rows, centre, extent, start, end):
    """Smallest t in (0, 1] at which the segment crosses a face of the true oriented box."""
    direction = sub(end, start)
    offset = sub(start, centre)
    local_start = [dot(offset, r) for r in rows]
    local_dir = [dot(direction, r) for r in rows]
    found = []
    for axis in range(3):
        if abs(local_dir[axis]) < 1e-9:
            continue
        for sign in (-1.0, 1.0):
            t = (sign * extent[axis] - local_start[axis]) / local_dir[axis]
            if 0.0 < t <= 1.0:
                point = [local_start[j] + local_dir[j] * t for j in range(3)]
                if all(abs(point[j]) <= extent[j] + 1e-4 for j in range(3) if j != axis):
                    found.append(t)
    return min(found) if found else None


def predicts_box_hit(update, probe, length, frame):
    blob = bytes.fromhex(probe["box_snapshot"]["box"])
    rows = [floats(blob, 0x80, 3), floats(blob, 0x90, 3), floats(blob, 0xA0, 3)]
    centre = floats(blob, 0xB0, 3)
    extent = [v * 0.5 for v in floats(blob, 0x1D0, 3)]
    start = probe["next"]
    end = sub(start, scale(update["normal"], length))
    if frame == "rows":
        axes = rows
    else:
        axes = [[rows[j][i] for j in range(3)] for i in range(3)]
    local0 = [dot(a, sub(start, centre)) for a in axes]
    local1 = [dot(a, sub(end, centre)) for a in axes]
    if not separating_axis_passes(local0, local1, extent):
        return False, None
    t = face_crossing(rows, centre, extent, start, end)
    if t is None:
        return False, None
    if probe["mesh_kind"] == 1:
        mesh_t = math.dist(probe["mesh_point"], start) / length
        if mesh_t <= t:
            return False, t * length
    return True, t * length


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("captures", nargs="+")
    parser.add_argument("--frame", choices=("cols", "rows"), default="cols")
    parser.add_argument("-v", action="store_true", help="one line per probe")
    args = parser.parse_args()
    total = wrong = box_hits = 0
    for path in args.captures:
        log = json.load(open(path))
        length = PROBE_LENGTH.get(log["weapon"], DEFAULT_PROBE_LENGTH)
        seen = bad = 0
        for number, update in enumerate(log["updates"], 1):
            probe = update.get("probe")
            if not probe or "box_snapshot" not in probe:
                continue
            observed = probe["mesh_kind"] == 3
            hit, t = predicts_box_hit(update, probe, length, args.frame)
            seen += 1
            box_hits += observed
            bad += hit != observed
            if args.v:
                print("%s u%02d observed=%s predicted=%s face=%s%s" % (
                    path, number, observed, hit, t, "" if hit == observed else "  <-- MISS"))
        total += seen
        wrong += bad
        print("%s: %d probes, %d mispredicted" % (path, seen, bad))
    print("total %d probes (%d meet the box), %d mispredicted with the %s frame" % (
        total, box_hits, wrong, args.frame))
    sys.exit(1 if wrong else 0)


if __name__ == "__main__":
    main()
