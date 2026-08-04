#!/usr/bin/env python3
"""Recover the original's along-track force law from a captured trace.

This is a *measurement* tool, not a fit-a-constant tool. It takes a per-tick
capture from the original (`scripts/psp-trace.py` output) and reconstructs the
net force the original actually applied along the ship's forward axis, without
assuming anything about the engine.

Why this works, and why the answer is trustworthy:

- `<Physical mass>` is **1** for every shipped speed class, so measured
  acceleration *is* net force. No mass estimate enters anywhere.
- Projecting onto the ship's forward axis cancels every force that acts along
  the surface normal - the hover spring, the disputed hover downforce, and the
  vertical damping - because the alignment torque holds the ship's `up` on the
  surface normal, and `up . forward == 0` by orthonormality. It also cancels
  lateral grip, which `Ship_ApplyLateralGrip` writes along body `X`.

  So the forward projection sees exactly four things: engine thrust, quadratic
  drag, rolling resistance, and gravity's along-slope component. That is what
  makes it a clean probe of the engine/resistance balance specifically.
- Gravity is removed analytically (`normal_gravity * forward.y`, mass 1), which
  matters: the reference capture climbs a sustained ~4.25 % grade, so the ship
  is *not* on flat ground and a naive "speed is steady, so thrust equals
  resistance" reading is biased.

The output is the measured curve `net(fs)`. Interpreting which side of the
balance is wrong is a separate, evidence-backed question - see
`docs/physics/force-balance-ground-truth.md`.

No shipped design data is embedded here: the two handling values this needs
(`normal_gravity`, and optionally `accelcap`/`amount` for the comparison
column) are passed on the command line and read from the user's own disc.

Usage:

    python3 scripts/trace-force-balance.py data/traces/foo.csv --normal-gravity 5
    python3 scripts/trace-force-balance.py data/traces/a.csv data/traces/b.csv \
        --normal-gravity 5 --skip a.csv:8 --range b.csv:118-199
"""

from __future__ import annotations

import argparse
import csv
import math
import os
import statistics

# Confirmed identical in both binaries, delay slots included; see
# `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The passive terms".
DRAG_GROUND = 0.005
ROLLING_RESISTANCE = 2.0


def _vec(row, prefix):
    return [float(row[f"{prefix}_{axis}"]) for axis in "xyz"]


def _dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def samples(path, normal_gravity, lo=0, hi=None, skip=()):
    """Yield `(forward_speed, net_force, implied_thrust)` per usable tick.

    A tick is usable only at full throttle, fully grounded, with no brake and
    no airbrake, so that the only unknown left in the forward projection is the
    engine itself.
    """
    with open(path, newline="") as handle:
        rows = list(csv.DictReader(handle))
    hi = len(rows) - 1 if hi is None else min(hi, len(rows) - 1)
    for i in range(lo, hi):
        if i in skip:
            continue
        row, nxt = rows[i], rows[i + 1]
        if float(row["throttle"]) != 100.0 or float(row["grounded"]) != 1.0:
            continue
        if float(row["brake"]) or float(row["airbrake_l"]) or float(row["airbrake_r"]):
            continue
        dt = float(row["dt"])
        velocity, forward = _vec(row, "vel"), _vec(row, "fwd")
        accel = [(b - a) / dt for a, b in zip(velocity, _vec(nxt, "vel"))]

        speed = math.dist([0.0, 0.0, 0.0], velocity)
        forward_speed = _dot(velocity, forward)

        # Rolling resistance normalises the velocity, so its direction - and
        # with it the term below - is undefined at a dead stop. A standing-start
        # capture really does contain such ticks (the launch is captured from
        # rest, which is the whole point of taking one), so the row is dropped
        # rather than divided by zero. One tick later the ship is moving and the
        # row is usable again, so this costs at most the first tick of a launch.
        if speed <= 1e-6:
            continue

        # Net force along forward, with the along-slope gravity component
        # removed. Mass is 1, so acceleration is force.
        net = _dot(accel, forward) + normal_gravity * forward[1]
        # Adding back the two confirmed resistances leaves the engine's own
        # contribution - the quantity the force law is supposed to predict.
        thrust = (
            net
            + DRAG_GROUND * forward_speed * forward_speed
            + ROLLING_RESISTANCE * forward_speed / speed
        )
        yield forward_speed, net, thrust


def fit_line(points):
    """Ordinary least squares `y = a + b*x`, returning `(a, b, rms, se_b)`."""
    n = len(points)
    sx = sum(x for x, _ in points)
    sy = sum(y for _, y in points)
    sxx = sum(x * x for x, _ in points)
    sxy = sum(x * y for x, y in points)
    denom = n * sxx - sx * sx
    b = (n * sxy - sx * sy) / denom
    a = (sy - b * sx) / n
    rms = math.sqrt(sum((y - (a + b * x)) ** 2 for x, y in points) / n)
    return a, b, rms, rms / math.sqrt(sxx - sx * sx / n)


def _spec(values, name):
    """Parse `file:spec` command-line pairs keyed by trace basename."""
    out = {}
    for item in values or ():
        key, _, rest = item.partition(":")
        if not rest:
            raise SystemExit(f"--{name} wants FILE:{name.upper()}, got {item!r}")
        out.setdefault(os.path.basename(key), []).append(rest)
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("traces", nargs="+")
    parser.add_argument(
        "--normal-gravity",
        type=float,
        required=True,
        help="<Physical normal_gravity> for the captured team and class, off your own disc",
    )
    parser.add_argument("--accelcap", type=float, help="<Engine accelcap>, for the comparison column")
    parser.add_argument("--amount", type=float, help="<Engine amount>, raw XML value")
    parser.add_argument("--skip", action="append", help="FILE:TICK, e.g. foo.csv:8")
    parser.add_argument("--range", action="append", help="FILE:LO-HI tick window")
    args = parser.parse_args()

    skips = _spec(args.skip, "skip")
    ranges = _spec(args.range, "range")

    points, thrusts = [], []
    for path in args.traces:
        base = os.path.basename(path)
        lo, hi = 0, None
        for window in ranges.get(base, []):
            lo, _, high = window.partition("-")
            lo, hi = int(lo), int(high)
        skip = {int(t) for t in skips.get(base, [])}
        rows = list(samples(path, args.normal_gravity, lo, hi, skip))
        print(f"{base}: {len(rows)} usable ticks, fs {min(r[0] for r in rows):.2f}-{max(r[0] for r in rows):.2f}")
        points += [(r[0], r[1]) for r in rows]
        thrusts += [(r[0], r[2]) for r in rows]

    a, b, rms, se = fit_line(points)
    print(f"\nnet forward force  = {a:+.3f} {b:+.4f} * fs   (rms {rms:.3f})")
    print(f"                     zero net force at fs = {-a / b:.2f}")
    ta, tb, trms, tse = fit_line(thrusts)
    print(f"implied engine     = {ta:+.3f} {tb:+.4f} * fs   (rms {trms:.3f}, se(slope) {tse:.4f})")
    print(f"                     slope is {abs(tb / tse):.0f} standard errors from zero")

    print("\n  fs   n   measured engine (median)", end="")
    if args.accelcap is not None and args.amount is not None:
        print("    engine.md's law", end="")
    print()
    binned = {}
    for fs, thrust in thrusts:
        binned.setdefault(round(fs), []).append(thrust)
    for speed in sorted(binned):
        values = binned[speed]
        if len(values) < 5:
            continue
        print(f"  {speed:3d} {len(values):4d} {statistics.median(values):20.3f}", end="")
        if args.accelcap is not None and args.amount is not None:
            documented = 2.0 * min(100.0 * args.amount * 0.001, 0.5 * speed + args.accelcap)
            print(f" {documented:18.2f}", end="")
        print()


if __name__ == "__main__":
    main()
