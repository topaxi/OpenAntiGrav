#!/usr/bin/env python3
"""Which reading of `body+0x150` matches the rotation the basis actually performs.

`body+0x150` is the angular quantity the original's contact resolvers rotate
through the basis rows before crossing with the lever arm (`vtfm4.q
C000,E100,C200` at `0x0884ea58` in `Body_ResolveContact` and `0x0884f038` in
`Body_ResolveContactPair`). **What that rotation means depends entirely on what
the field holds**, and reading it as world-space turns the resolvers' point
velocity into a sign error on the dominant mode - see
`docs/ghidra/functions/psp-pulse-usa/contact-response.md`.

A `data/traces/*.csv` capture records the basis rows (`body+0x00`, `+0x10`,
`+0x20`) and the column itself every tick, so the question is answerable from
the original's own data with no emulator and no staging: the rotation rate the
recorded basis performs between two ticks is

    omega_true = 0.5 * sum_i cross(row_i, d(row_i)/dt)

and each candidate reading predicts a different column from it. Run it over any
capture that has the `right_*`/`up_*`/`fwd_*`/`omega_*` columns:

    python3 scripts/omega-column-reading-fit.py data/traces/talons-junction-*.csv

Median residual against the recorded column, in rad/s, on three whole Talon's
Junction laps (2026-09-10):

| trace | tick pairs | median |omega_true| | World | NegatedWorld | Local | NegatedLocal |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| time-trial-lap | 3145 | 1.0333 | 2.0533 | 0.1778 | 2.0634 | **0.0205** |
| clean-lap      | 2976 | 1.5435 | 2.8725 | 0.3165 | 3.0417 | **0.0330** |
| autopilot      | 3019 | 1.5968 | 2.9687 | 0.3118 | 3.1387 | **0.0332** |
| standing-start |  299 | 0.1075 | 0.1711 | 0.0992 | 0.2151 | **0.0028** |

`NegatedLocal` wins by two orders of magnitude - a 2 % residual against a
1.0-1.6 rad/s signal, where `World` is a 200 % one. So `+0x150 = -R *
omega_true`, the resolvers' `R^T` is the body-local -> world unrotation rather
than a spurious rotation, and `R^T(+0x150) == -omega_true` makes their point
velocity the plain textbook `v + omega_true x r`.

This reproduces independently what `crates/trace/src/trace/frame.rs`'s
`Frame::angular_rate` records as a per-axis fit of `-1.0011`, `-0.9999`,
`-0.9993`, and it is the same `w_game = -w_physics` contract
`crates/physics/src/integrate.rs` states as a whole-crate rule.
"""

import argparse
import csv
import math
from pathlib import Path

READINGS = ("World", "NegatedWorld", "Local", "NegatedLocal")


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def scale(s, a):
    return tuple(s * x for x in a)


def add(*vectors):
    return tuple(sum(c) for c in zip(*vectors))


def norm(a):
    return math.sqrt(dot(a, a))


def basis(row):
    return (
        tuple(float(row["right_%s" % axis]) for axis in "xyz"),
        tuple(float(row["up_%s" % axis]) for axis in "xyz"),
        tuple(float(row["fwd_%s" % axis]) for axis in "xyz"),
    )


def column(row):
    return tuple(float(row["omega_%s" % axis]) for axis in "xyz")


def predictions(omega_true, rows):
    local = tuple(dot(omega_true, r) for r in rows)
    return {
        "World": omega_true,
        "NegatedWorld": scale(-1.0, omega_true),
        "Local": local,
        "NegatedLocal": scale(-1.0, local),
    }


def fit(path):
    with open(path, newline="") as handle:
        ticks = list(csv.DictReader(handle))
    if not ticks or "omega_x" not in ticks[0] or "right_x" not in ticks[0]:
        return None
    residuals = {name: [] for name in READINGS}
    magnitudes = []
    for index in range(len(ticks) - 1):
        dt = float(ticks[index + 1]["dt"])
        if dt <= 0.0:
            continue
        before, after = basis(ticks[index]), basis(ticks[index + 1])
        # The rotation the recorded basis performs, from its own derivative.
        omega_true = scale(
            0.5,
            add(*[cross(before[i], scale(1.0 / dt, sub(after[i], before[i]))) for i in range(3)]),
        )
        magnitudes.append(norm(omega_true))
        recorded = column(ticks[index])
        for name, predicted in predictions(omega_true, before).items():
            residuals[name].append(norm(sub(recorded, predicted)))
    return magnitudes, residuals


def median(values):
    return sorted(values)[len(values) // 2] if values else float("nan")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("traces", nargs="+", type=Path)
    args = parser.parse_args()
    for path in args.traces:
        result = fit(path)
        if result is None:
            print("%-44s (no basis/omega columns)" % path.name)
            continue
        magnitudes, residuals = result
        print(
            "%s  %d tick pairs, median |omega_true| %.4f rad/s"
            % (path.name, len(magnitudes), median(magnitudes))
        )
        for name in READINGS:
            values = residuals[name]
            print(
                "   %-13s median residual %8.4f   RMS %8.4f rad/s"
                % (name, median(values), math.sqrt(sum(v * v for v in values) / len(values)))
            )


if __name__ == "__main__":
    main()
