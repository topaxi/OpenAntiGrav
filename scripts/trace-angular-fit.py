#!/usr/bin/env python3
"""Settle what the recorded angular-velocity column at `body+0x160` actually is.

`oag-trace show` scores the four [`AngularReading`]s of that column - sign x
local/world - against the rotation the recorded basis itself performs, and on a
real capture it comes back *undecided*: all four residuals land within a few
percent of the column's own rms, which is what "none of these explains the
signal" looks like. This tool says why, and answers the question anyway.

**The column is not in the same units as the rotation it produces.** It is
proportional to the body-local angular velocity, per body axis, with three
distinct negative constants near -15, -21 and -15. A one-to-one comparison
cannot see that, so it scores every reading as equally wrong; fitting a scale
first separates them cleanly.

What the fit reports, and what each number decides:

- **Frame.** Compare `local, single scale` against `world, single scale`. These
  are the same model with the same number of free parameters, so the residuals
  are directly comparable and the better one is the frame.
- **Sign.** Read the *sign of the fitted scale*, not a separate hypothesis: the
  negated and un-negated readings differ only by the sign a free scale absorbs,
  so they always tie. A negative `k` means the stored column is the negation of
  the physical angular velocity.
- **Mechanism.** `local, diagonal` fits one constant per body axis. If that
  beats the single scale materially, the constants are a diagonal tensor rather
  than a unit conversion - i.e. the column is an angular *momentum* `I * w` in
  body coordinates, and `1/|k_y|` is the yaw term
  `oag_physics::forces::YAW_INVERSE_INERTIA` stands in for.

The measurement needs no simulation and no disc: a capture's own basis rows
differentiate into the angular velocity the ship demonstrably had, and the
recorded column is compared against that.

    python3 scripts/trace-angular-fit.py data/traces/*.csv

Traces are derived game data and stay under `data/traces/`; this script is not.
"""

from __future__ import annotations

import argparse
import csv
import math
import os

# Which recorded basis row is which body axis. Rows 0/1/2 of the matrix at
# `body+0x00` are the ship's right, up and forward in world coordinates, so a
# local component `i` of a world vector `w` is `dot(w, row_i)`.
ROWS = ("right", "up", "fwd")
AXIS_NAMES = ("x / right", "y / up", "z / forward")


def _vec(row, prefix):
    return [float(row[f"{prefix}_{axis}"]) for axis in "xyz"]


def _dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def _cross(a, b):
    return [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]


def basis_rate(row, nxt, dt):
    """The world-space angular velocity the recorded basis itself performs.

    For an orthonormal basis whose rows are `e_i`, the rotation rate satisfies
    `de_i/dt = w x e_i`, so `w = 0.5 * sum_i cross(e_i, de_i/dt)`. Averaging all
    three rows rather than using one is what makes it robust to the seven
    significant digits a capture writes.
    """
    total = [0.0, 0.0, 0.0]
    for name in ROWS:
        axis, ahead = _vec(row, name), _vec(nxt, name)
        derivative = [(b - a) / dt for a, b in zip(axis, ahead)]
        total = [t + 0.5 * c for t, c in zip(total, _cross(axis, derivative))]
    return total


def samples(path):
    """Yield `(recorded, world_rate, local_rate)` per usable tick."""
    with open(path, newline="") as handle:
        rows = list(csv.DictReader(handle))
    if "avel_x" not in (rows[0] if rows else {}):
        return
    for row, nxt in zip(rows, rows[1:]):
        dt = float(row["dt"])
        if dt <= 0.0:
            continue
        world = basis_rate(row, nxt, dt)
        basis = [_vec(row, name) for name in ROWS]
        local = [_dot(world, axis) for axis in basis]
        yield _vec(row, "avel"), world, local


def scale(pairs):
    """Least-squares `k` for `recorded = k * predicted`, and its residual rms."""
    numerator = sum(_dot(a, b) for a, b in pairs)
    denominator = sum(_dot(b, b) for _, b in pairs)
    k = numerator / denominator if denominator else float("nan")
    residual = math.sqrt(
        sum(sum((x - k * y) ** 2 for x, y in zip(a, b)) for a, b in pairs) / len(pairs)
    )
    return k, residual


def report(name, data):
    signal = math.sqrt(sum(_dot(a, a) for a, _, _ in data) / len(data))
    print(f"{name}: {len(data)} tick(s), column rms {signal:.4f}")

    def line(label, k, residual):
        share = 100.0 * (1.0 - residual / signal) if signal else float("nan")
        print(f"  {label:<34} k = {k:9.3f}   residual {residual:7.4f}  ({share:5.1f}% explained)")

    world_k, world_r = scale([(a, w) for a, w, _ in data])
    local_k, local_r = scale([(a, l) for a, _, l in data])
    line("world, single scale", world_k, world_r)
    line("local, single scale", local_k, local_r)

    axes = []
    for index in range(3):
        numerator = sum(a[index] * l[index] for a, _, l in data)
        denominator = sum(l[index] ** 2 for _, _, l in data)
        axes.append(numerator / denominator if denominator else float("nan"))
    residual = math.sqrt(
        sum(
            sum((a[i] - axes[i] * l[i]) ** 2 for i in range(3))
            for a, _, l in data
        )
        / len(data)
    )
    line("local, one scale per body axis", float("nan"), residual)
    for index, axis in enumerate(AXIS_NAMES):
        column = [(a[index], l[index]) for a, _, l in data]
        own_signal = math.sqrt(sum(x * x for x, _ in column) / len(column))
        own_residual = math.sqrt(
            sum((x - axes[index] * y) ** 2 for x, y in column) / len(column)
        )
        share = 100.0 * (1.0 - own_residual / own_signal) if own_signal else float("nan")
        print(
            f"      {axis:<12} k = {axes[index]:9.3f}   1/|k| = {1.0 / abs(axes[index]):.5f}"
            f"   ({share:5.1f}% explained, signal {own_signal:.4f})"
        )

    reading = "negated-" if local_k < 0 else ""
    frame = "local" if local_r < world_r else "world"
    print(f"  => the column reads as {reading}{frame}, scaled by {abs(local_k):.2f}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("traces", nargs="+")
    parser.add_argument(
        "--joint",
        action="store_true",
        help="pool every trace into one fit as well, which is the number to quote: "
        "a constant that reproduces two captures with different dynamics is not a "
        "property of either one",
    )
    args = parser.parse_args()

    pooled = []
    for path in args.traces:
        data = list(samples(path))
        if not data:
            print(f"{os.path.basename(path)}: no angular-velocity column, skipped")
            continue
        report(os.path.basename(path), data)
        print()
        pooled += data

    if args.joint and pooled:
        report("all traces pooled", pooled)


if __name__ == "__main__":
    main()
