#!/usr/bin/env python3
"""Aggregate the `ROW,` lines two pose-walk sweeps print, as the physics pages quote them.

    gate   CAPTURE.csv=SWEEP.txt ...   `crest_pitch_pose_walk`'s output against its
                                       capture: the pitch residual over the full input
                                       torque (`1.923` rad/s^2), signed by the stick, on
                                       the airborne rows, split by the recorded
                                       `dot(up, -spline_down)` against `0.9`.
    frames [CAPTURE.csv=]SWEEP.txt ... `autopilot_ramp_pose_walk`'s output: rms and median
                                       |residual| per phase (both rows grounded / both
                                       airborne) and axis (yaw, pitch, roll).

Rows are dropped where the next position is off the one-step prediction by more than
`--rise` (a cage or escape moved the body; gate) or the velocity residual exceeds `--dv`
(a wall hit; frames), below `--min-speed` units/s where the capture is given (a craft
held against a wall gives a small one-step `dv`; frames), and before `--from` (the placement
transient). Prints only.
"""

import argparse
import csv
import math
import statistics


def gate(pairs, rise, first):
    groups = {}
    dropped = 0
    for pair in pairs:
        capture, sweep = pair.split("=", 1)
        rows = {}
        for r in csv.DictReader(open(capture)):
            f = {k: float(v) for k, v in r.items()}
            align = -(f["up_x"] * f["spline_down_x"] + f["up_y"] * f["spline_down_y"]
                      + f["up_z"] * f["spline_down_z"])
            rows[int(f["tick"])] = (align, f["stick_pitch"])
        for line in open(sweep):
            if not line.startswith("ROW,"):
                continue
            p = line.strip().split(",")
            tick = int(p[1])
            if tick < first or float(p[2]) != 0 or float(p[3]) != 0:
                continue
            align, stick = rows[tick]
            if stick == 0:
                continue
            if len(p) > 17 and float(p[17]) > rise:
                dropped += 1
                continue
            key = "open" if align > 0.9 else "closed"
            groups.setdefault(key, []).append(float(p[5]) * math.copysign(1.0, stick) / 1.923)
    print("dropped %d" % dropped)
    for key, v in sorted(groups.items()):
        print("%-6s n %4d median %+.3f mean %+.3f" % (key, len(v), statistics.median(v),
                                                   statistics.mean(v)))


def frames(sweeps, dv_max, first, min_speed):
    cells = {}
    for item in sweeps:
        capture, _, sweep = item.rpartition("=")
        speed = {}
        if capture:
            for r in csv.DictReader(open(capture)):
                speed[int(float(r["tick"]))] = math.sqrt(
                    sum(float(r["vel_" + c]) ** 2 for c in "xyz"))
        for line in open(sweep):
            if not line.startswith("ROW,"):
                continue
            t, g, gn, yaw, pitch, roll, dv, stun = (float(x) for x in line.strip().split(",")[1:])
            if t < first or stun > 0 or dv > dv_max:
                continue
            if speed and speed[int(t)] < min_speed:
                continue
            phase = "ground" if g == 1 and gn == 1 else "air" if g == 0 and gn == 0 else None
            if phase is None:
                continue
            for axis, value in (("yaw", yaw), ("pitch", pitch), ("roll", roll)):
                cells.setdefault((phase, axis), []).append(value)
    for (phase, axis), v in sorted(cells.items()):
        rms = math.sqrt(sum(x * x for x in v) / len(v))
        print("%-6s %-5s n %4d rms %.3f median|r| %.3f" % (
            phase, axis, len(v), rms, statistics.median(abs(x) for x in v)))


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("mode", choices=["gate", "frames"])
    parser.add_argument("inputs", nargs="+")
    parser.add_argument("--rise", type=float, default=0.05)
    parser.add_argument("--dv", type=float, default=1.0)
    parser.add_argument("--from", dest="first", type=int, default=0)
    parser.add_argument("--min-speed", type=float, default=0.0)
    args = parser.parse_args()
    if args.mode == "gate":
        gate(args.inputs, args.rise, args.first)
    else:
        frames(args.inputs, args.dv, args.first, args.min_speed)


if __name__ == "__main__":
    main()
