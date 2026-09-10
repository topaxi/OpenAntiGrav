#!/usr/bin/env python3
"""Which frames `body+0x150` and `body+0x160` are in, and which way the tensor turns.

`Body_Integrate` maps one to the other every sub-step through a matrix it builds
from `body+0x40` and the basis (`0x0884e380`-`0x0884e39c`), caching the result at
`body+0x80`. **Which direction that change of basis runs decides what both blocks
hold**, and it was read backwards until 2026-09-10: `docs/ghidra/functions/
psp-pulse-usa/rigid-body.md` called `+0x40` the body-space tensor and `+0x80` its
world copy, and the contact resolvers' use of `+0x40` on a world-space `r x n`
had to be written off as a literal-but-wrong transcription because of it.

A `data/traces/*.csv` capture records both columns *and* the basis rows every
tick, so the map is measurable with no derivative, no emulator and no staging.
Write `R` for the matrix whose rows are the recorded basis rows - the craft's
world-space axes, so `R` maps world to body - and fit the diagonal `I` under each
candidate pairing of frames, predicting the recorded `+0x160` from the recorded
`+0x150`:

    A   L = I (x) w             both in body coordinates, diagonal map
    B   L = R^T (I (x) R w)     momentum world, rate body
    F   L = R (I (x) R^T w)     omega_world = I^-1 L_world, both stored in body

Three whole Talon's Junction captures (2026-09-10), as "% explained", which is
`1 - |residual| / |column|`:

| trace | ticks | A | B | **F** |
| --- | ---: | ---: | ---: | ---: |
| time-trial-lap | 3146 | 95.17 | 94.87 | **100.00** |
| clean-lap      | 2977 | 91.34 | 92.39 | **100.00** |
| standing-start |  300 | 95.44 | 93.49 | **100.00** |

`F` is exact, and its free three-parameter fit returns `(15.600, 21.600, 15.600)`
on every capture - `Body_SetBoxInertia`'s own code-literal box tensor, recovered
from a column it was never compared against. So the engine's inverse inertia is a
**fixed diagonal in world axes**, `+0x80` is its body-space copy, and the `R` and
`R^T` in the integrator do nothing but enter and leave the frame the two fields
are stored in.

All three models have the same three free parameters, so the residuals are
directly comparable. `--code-literal` scores them against `(15.6, 21.6, 15.6)`
instead of fitting, and `--off-level` restricts to ticks where the craft is
banked or pitched past `up_y < 0.9` - the only ticks where `A` and `F` can
differ at all, because a `diag(a, b, a)` tensor is invariant under yaw. That
invariance is why the mislabel survived, and why the engine gets away with it.

    python3 scripts/trace-inertia-frame-fit.py data/traces/talons-junction-*.csv

Traces are derived game data and stay under `data/traces/`; this script is not.
"""

import argparse
import csv
import math
from pathlib import Path

# `Body_SetBoxInertia` (0x0884e1ac, conf 92): the code-literal box at mass 0.9,
# on (right, up, forward). See docs/physics/angular-velocity-column.md.
CODE_LITERAL = (15.6, 21.6, 15.6)
LEVEL_LIMIT = 0.9


def vec(row, prefix):
    return [float(row["%s_%s" % (prefix, axis)]) for axis in "xyz"]


def to_body(basis, world):
    """`R w`: the components of a world vector along the three basis rows."""
    return [sum(basis[i][k] * world[k] for k in range(3)) for i in range(3)]


def to_world(basis, body):
    """`R^T v`: a body-coordinate triple back out along the basis rows."""
    return [sum(body[i] * basis[i][k] for i in range(3)) for k in range(3)]


def model_a(w, basis, inertia):
    return [inertia[i] * w[i] for i in range(3)]


def model_b(w, basis, inertia):
    return to_world(basis, [inertia[i] * x for i, x in enumerate(to_body(basis, w))])


def model_f(w, basis, inertia):
    return to_body(basis, [inertia[i] * x for i, x in enumerate(to_world(basis, w))])


MODELS = (
    ("A  L = I (x) w         ", model_a),
    ("B  L = R^T (I (x) R w) ", model_b),
    ("F  L = R (I (x) R^T w) ", model_f),
)


def solve(matrix, rhs):
    rows = [matrix[i][:] + [rhs[i]] for i in range(3)]
    for i in range(3):
        pivot = max(range(i, 3), key=lambda r: abs(rows[r][i]))
        rows[i], rows[pivot] = rows[pivot], rows[i]
        if rows[i][i] == 0.0:
            continue
        for r in range(3):
            if r != i:
                factor = rows[r][i] / rows[i][i]
                for k in range(i, 4):
                    rows[r][k] -= factor * rows[i][k]
    return [rows[i][3] / rows[i][i] if rows[i][i] else 0.0 for i in range(3)]


def fit(samples, predict):
    """Least squares for the three diagonal entries: each is one design column."""
    normal = [[0.0] * 3 for _ in range(3)]
    rhs = [0.0] * 3
    for w, basis, momentum in samples:
        columns = [
            predict(w, basis, [1.0 if j == i else 0.0 for j in range(3)]) for i in range(3)
        ]
        for i in range(3):
            rhs[i] += sum(columns[i][k] * momentum[k] for k in range(3))
            for j in range(3):
                normal[i][j] += sum(columns[i][k] * columns[j][k] for k in range(3))
    return solve(normal, rhs)


def explained(samples, predict, inertia):
    residual = sum(
        sum((momentum[k] - predict(w, basis, inertia)[k]) ** 2 for k in range(3))
        for w, basis, momentum in samples
    )
    signal = sum(sum(x * x for x in momentum) for _, _, momentum in samples)
    if signal <= 0.0:
        return float("nan")
    return 100.0 * (1.0 - math.sqrt(residual / signal))


def load(path, off_level):
    with open(path, newline="") as handle:
        rows = list(csv.DictReader(handle))
    if not rows or "omega_x" not in rows[0] or "avel_x" not in rows[0]:
        return None
    samples = []
    for row in rows:
        basis = [vec(row, "right"), vec(row, "up"), vec(row, "fwd")]
        if off_level and basis[1][1] >= LEVEL_LIMIT:
            continue
        samples.append((vec(row, "omega"), basis, vec(row, "avel")))
    return samples


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("traces", nargs="+", type=Path)
    parser.add_argument(
        "--code-literal",
        action="store_true",
        help="score against Body_SetBoxInertia's own tensor instead of fitting",
    )
    parser.add_argument(
        "--off-level",
        action="store_true",
        help="only ticks with up_y below %s, where the models can differ" % LEVEL_LIMIT,
    )
    args = parser.parse_args()
    for path in args.traces:
        samples = load(path, args.off_level)
        if samples is None:
            print("%-44s (no basis/omega/avel columns)" % path.name)
            continue
        if len(samples) < 50:
            print("%-44s (only %d tick(s))" % (path.name, len(samples)))
            continue
        print("%s  %d tick(s)" % (path.name, len(samples)))
        for name, predict in MODELS:
            inertia = CODE_LITERAL if args.code_literal else fit(samples, predict)
            print(
                "   %s I = (%7.3f, %7.3f, %7.3f)   %6.2f %% explained"
                % (name, inertia[0], inertia[1], inertia[2], explained(samples, predict, inertia))
            )


if __name__ == "__main__":
    main()
