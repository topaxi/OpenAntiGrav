#!/usr/bin/env python3
"""This engine's track samples for the original driver, mimicked against a capture.

`oag_raceplay` hands the original driver two samples: the craft's own sample, a
projection onto the segment from its located line index to the next, and the
lookahead sample, a walk of `lookahead_distance` along the sample polyline
(`original_ai.rs`, `original_own` and `original_ahead`). The original evaluates its
spline at a progress instead. This mimics ours over one path of an
`oag-trace track` CSV at every frame of a `scripts/psp-autopilot-air.py --at
controls` capture and prints how far ours sits from the original's own sample and
target, along and across the track, plus the target's across error as steer
units (`950 * SteerMul`, PHANTOM's `0.7`).

It ignores routes, so a frame on a fork the capture took off the given path shows
up as a large miss; those frames are dropped by an own-sample gate of 10 units.

    cargo run -q -p oag-trace -- track --source data/images/pulse-psp-usa.chd \\
        --track 'Data\\Environments\\14_Track\\track.vex' > data/scratch/<lane>/track14.csv
    python3 scripts/psp-autopilot-samples.py data/scratch/<lane>/air.json \\
        data/scratch/<lane>/track14.csv --path 2
"""

import argparse
import csv
import json
import math


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("capture")
    parser.add_argument("track_csv")
    parser.add_argument("--path", default="0", help="the CSV path column to walk")
    parser.add_argument("--lookahead-secs", type=float, default=0.3)
    parser.add_argument("--steer-mul", type=float, default=0.7)
    args = parser.parse_args()

    rows = [r for r in csv.DictReader(open(args.track_csv)) if r["path"] == args.path]
    n = len(rows)

    def vec(r, p):
        return [float(r[p + "_x"]), float(r[p + "_y"]), float(r[p + "_z"])]

    def sub(a, b):
        return [a[i] - b[i] for i in range(3)]

    def add(a, b):
        return [a[i] + b[i] for i in range(3)]

    def mul(a, s):
        return [x * s for x in a]

    def dot(a, b):
        return sum(a[i] * b[i] for i in range(3))

    def length(a):
        return math.sqrt(dot(a, a))

    def unit(a):
        return mul(a, 1 / length(a)) if length(a) > 0 else a

    pos = [vec(r, "pos") for r in rows]

    def point(i, t):
        a, b = rows[i % n], rows[(i + 1) % n]
        down = unit(add(vec(a, "down"), mul(sub(vec(b, "down"), vec(a, "down")), t)))
        p = add(vec(a, "pos"), mul(sub(vec(b, "pos"), vec(a, "pos")), t))
        lateral = unit(add(vec(a, "lat"), mul(sub(vec(b, "lat"), vec(a, "lat")), t)))
        line = float(a["racing_line"]) + (float(b["racing_line"]) - float(a["racing_line"])) * t
        return {"pos": sub(p, mul(down, 3.0)), "lat": lateral, "rl": line}

    def locate(p):
        best = None
        for i in range(n):
            a, b = pos[i], pos[(i + 1) % n]
            s = sub(b, a)
            t = max(0.0, min(1.0, dot(sub(p, a), s) / dot(s, s)))
            d = length(sub(p, add(a, mul(s, t))))
            if best is None or d < best[0]:
                best = (d, i, t)
        return best[1], best[2]

    def ahead(i, t, distance):
        here = add(pos[i % n], mul(sub(pos[(i + 1) % n], pos[i % n]), t))
        left = distance
        for k in range(i, i + n):
            nxt = pos[(k + 1) % n]
            step = length(sub(nxt, here))
            if step >= left and step > 0:
                base = pos[k % n]
                reached = add(here, mul(sub(nxt, here), left / step))
                span = length(sub(nxt, base))
                return point(k, max(0.0, min(1.0, length(sub(reached, base)) / span)))
            left -= step
            here = nxt
        return None

    frames = json.load(open(args.capture))["frames"]
    out = []
    for k, r in enumerate(frames):
        i, t = locate(r["pos"])
        own = point(i, t)
        o = r["own"]
        if length(sub(own["pos"], o["pos"])) > 10:
            continue
        h = dot(sub(r["pos"], own["pos"]), r["up"])
        air = max(0.0, min(1.0, (abs(h) - 15) * 0.025))
        distance = max(r["smoothed_speed"] * (args.lookahead_secs + air), 15) * 1.5
        a = ahead(i, t, distance)
        target = add(a["pos"], mul(a["lat"], a["rl"]))
        e = sub(target, r["target"])
        eo = sub(own["pos"], o["pos"])
        reach = length(sub(r["target"], r["pos"]))
        across = dot(e, r["ahead"]["lateral"])
        out.append((k, dot(e, r["ahead"]["tangent"]), across, dot(eo, o["tangent"]),
                    dot(eo, o["lateral"]), across / reach * 950 * args.steer_mul))

    def stats(col):
        v = sorted(abs(x[col]) for x in out)
        return "p50 %.3f p90 %.3f p99 %.3f" % (v[len(v) // 2], v[9 * len(v) // 10],
                                               v[99 * len(v) // 100])

    print("frames on the path: %d of %d" % (len(out), len(frames)))
    for name, col in (("target along", 1), ("target across", 2), ("own along", 3),
                      ("own across", 4), ("steer units of the target's across error", 5)):
        print("%s: %s" % (name, stats(col)))
    print("worst frames by steer units:")
    for x in sorted(out, key=lambda x: -abs(x[5]))[:5]:
        print("  frame %d pos %s steer %.1f" % (x[0], [round(c) for c in frames[x[0]]["pos"]],
                                               x[5]))


if __name__ == "__main__":
    main()
