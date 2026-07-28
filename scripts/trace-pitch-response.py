#!/usr/bin/env python3
"""Identify the pitch axis as a second-order system, from a capture or a run.

The pitch scenarios (`verification/scenarios/pitch-both-ways.inputs`, and
`pitch-hold-thrust.inputs`) hold the pitch axis for a fixed number of ticks and
then release it. The craft answers with a step response: the rate rises, the
attitude comes back, and how it comes back is what identifies the restoring
path. `docs/physics/angular-velocity-column.md` fitted the first four extrema by
hand; this does the same identification as a least-squares fit over a whole
window, so the original and a simulated run can be measured the same way.

The model, on the recorded body-local pitch rate `omega_x`:

    omega_x(t) = A * exp(-sigma * t) * cos(omega_d * t + phi)

fitted by scanning `(sigma, omega_d)` and solving the remaining linear problem
in closed form at each node - no optimiser, no dependency beyond numpy, and the
residual surface is printed as a sanity check rather than trusted blind. From
the fit,

    omega_n     = sqrt(sigma^2 + omega_d^2)      the natural frequency
    zeta        = sigma / omega_n                the damping ratio
    2*zeta*omega_n = 2 * sigma                   what the crate has to supply

`2*sigma` is the quantity to compare: it is the coefficient of the rate-feedback
term in `theta'' + 2*zeta*omega_n*theta' + omega_n^2*theta = 0`, so it is
additive across damping sources and independent of the drive's magnitude.

Usage:

    python3 scripts/trace-pitch-response.py data/traces/talons-junction-pitch-both-ways.csv
    python3 scripts/trace-pitch-response.py A.csv B.csv --window 40 160 --label original ours

Captures are gitignored derived data; see `docs/reverse-engineering/ppsspp-debugger.md`.
"""

from __future__ import annotations

import argparse
import csv
import math
from pathlib import Path

import numpy as np


def read_column(path: Path, column: str) -> tuple[np.ndarray, np.ndarray]:
    """Returns (time in seconds, the column), from a trace's own `dt`."""
    with path.open(newline="") as handle:
        rows = list(csv.DictReader(handle))
    if not rows:
        raise SystemExit(f"{path}: no rows")
    if column not in rows[0]:
        raise SystemExit(f"{path}: no {column!r} column (has {', '.join(rows[0])})")
    dt = np.array([float(row["dt"]) for row in rows])
    values = np.array([float(row[column]) for row in rows])
    time = np.concatenate([[0.0], np.cumsum(dt)[:-1]])
    return time, values


def fit_damped_sinusoid(
    time: np.ndarray, signal: np.ndarray
) -> tuple[float, float, float, float]:
    """Least-squares `A exp(-sigma t) cos(omega_d t + phi)`.

    Returns `(sigma, omega_d, amplitude, relative residual)`. The scan bounds are
    wide enough for both sides of every pitch capture taken so far and are
    deliberately not narrowed to what the answer turned out to be.
    """
    t = time - time[0]
    best = None
    for sigma in np.linspace(0.0, 12.0, 481):
        envelope = np.exp(-sigma * t)
        for omega_d in np.linspace(1.0, 30.0, 581):
            basis = np.stack([envelope * np.cos(omega_d * t), envelope * np.sin(omega_d * t)], axis=1)
            coefficients, *_ = np.linalg.lstsq(basis, signal, rcond=None)
            residual = float(np.sum((basis @ coefficients - signal) ** 2))
            if best is None or residual < best[0]:
                best = (residual, sigma, omega_d, coefficients)
    residual, sigma, omega_d, coefficients = best
    amplitude = float(math.hypot(*coefficients))
    total = float(np.sum(signal**2))
    return sigma, omega_d, amplitude, math.sqrt(residual / total) if total > 0 else math.nan


def extrema(signal: np.ndarray, count: int) -> list[float]:
    """The first `count` local extrema, the hand method the docs used."""
    found: list[float] = []
    for i in range(1, len(signal) - 1):
        rising = signal[i] > signal[i - 1] and signal[i] >= signal[i + 1]
        falling = signal[i] < signal[i - 1] and signal[i] <= signal[i + 1]
        if rising or falling:
            found.append(float(signal[i]))
            if len(found) == count:
                break
    return found


def report(path: Path, label: str, column: str, window: tuple[int, int]) -> None:
    time, values = read_column(path, column)
    start, end = window
    end = min(end, len(values))
    if start >= end:
        raise SystemExit(f"{path}: empty window {window}")
    t = time[start:end]
    signal = values[start:end]

    sigma, omega_d, amplitude, residual = fit_damped_sinusoid(t, signal)
    omega_n = math.hypot(sigma, omega_d)
    zeta = sigma / omega_n if omega_n > 0 else math.nan
    peaks = ", ".join(f"{value:+.3f}" for value in extrema(signal, 4))

    print(f"{label}  ({path.name}, ticks {start}..{end}, column {column})")
    print(f"  first extrema        {peaks}")
    print(f"  amplitude            {amplitude:.4f} rad/s")
    print(f"  omega_d              {omega_d:.3f} rad/s")
    print(f"  omega_n              {omega_n:.3f} rad/s")
    print(f"  zeta                 {zeta:.4f}")
    print(f"  2*zeta*omega_n       {2 * sigma:.3f}")
    print(f"  fit residual         {residual:.1%} of signal")
    print()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("traces", nargs="+", type=Path)
    parser.add_argument(
        "--window",
        nargs=2,
        type=int,
        default=(40, 160),
        metavar=("START", "END"),
        help="tick range to fit; the default is pitch-both-ways' first hold",
    )
    parser.add_argument("--column", default="omega_x", help="default omega_x, the body-local pitch rate")
    parser.add_argument("--label", nargs="*", default=None, help="one label per trace")
    args = parser.parse_args()

    labels = args.label or [path.stem for path in args.traces]
    if len(labels) != len(args.traces):
        raise SystemExit("--label needs one label per trace")
    for path, label in zip(args.traces, labels):
        report(path, label, args.column, tuple(args.window))


if __name__ == "__main__":
    main()
