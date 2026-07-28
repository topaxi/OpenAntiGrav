#!/usr/bin/env python3
"""Split the attitude question into its two halves, using `omega_*` (`body+0x150`).

`docs/physics/cornering-ground-truth.md` measured, on a lap with only the
momentum column recorded, that the basis rotates *more* than `body+0x160`
accounts for on pitch and roll (`0.231x` and `0.647x` of the inertia tensor)
while yaw closes at `0.970x`. With `body+0x150` recorded that single composite
identity splits into two independently measurable ones, and they point at
different mechanisms:

    (A)  avel(+0x160)  =  I * omega(+0x150)          the integrator's own map
    (B)  omega(+0x150) = -omega(basis)               the basis advance

If (A) holds and (B) fails, something rotates the basis outside the integrator.
If (A) fails and (B) holds, a second source writes `+0x150` without going
through the momentum column. The composite (C) `avel = -I * omega(basis)` is
what the earlier lap could see, and it is the product of the two.

`omega(basis)` is recovered from `M[i]^T M[i+1]` exactly the way
`scripts/trace-cornering.py` does; the cleanliness rule (`speed / |velocity|`
exactly 1 at *both* ends of the interval) is the same one, so the numbers here
are directly comparable with that page's table.

    uv run scripts/trace-omega-identity.py data/traces/<lap>.csv

No handling stats are needed: every fit here is kinematic.
"""

import argparse
import csv
import math
import statistics

# `Body_SetBoxInertia` (0x0884e1ac, conf 92): the code-literal box (12, 8, 12)
# at mass 0.9 on (right, up, forward). See docs/physics/angular-velocity-column.md.
INERTIA = (15.6, 21.6, 15.6)
CLEAN_TOLERANCE = 5e-5


def vec(row, prefix):
    return tuple(float(row["%s_%s" % (prefix, axis)]) for axis in "xyz")


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def norm(a):
    return math.sqrt(dot(a, a))


def load(path):
    with open(path, newline="") as handle:
        rows = list(csv.DictReader(handle))
    if "omega_x" not in rows[0]:
        raise SystemExit("%s predates the omega_* columns - re-capture it" % path)
    for row in rows:
        row["_vel"] = vec(row, "vel")
        row["_fwd"] = vec(row, "fwd")
        row["_right"] = vec(row, "right")
        row["_up"] = vec(row, "up")
        row["_avel"] = vec(row, "avel")
        row["_omega"] = vec(row, "omega")
        row["_pos"] = vec(row, "pos")
        row["_v"] = norm(row["_vel"])
        row["_ratio"] = float(row["speed"]) / row["_v"] if row["_v"] > 1e-9 else math.nan
    return rows


def clean_mask(rows):
    ok = []
    for i in range(len(rows) - 1):
        here, nxt = rows[i]["_ratio"], rows[i + 1]["_ratio"]
        ok.append(
            not math.isnan(here)
            and not math.isnan(nxt)
            and abs(here - 1.0) < CLEAN_TOLERANCE
            and abs(nxt - 1.0) < CLEAN_TOLERANCE
        )
    ok.append(False)
    return ok


def basis_angular_velocity(rows, i):
    """Angular velocity over `[i, i+1]`, in the tick-`i` body frame."""
    a, b = rows[i], rows[i + 1]
    cols_a = (a["_right"], a["_up"], a["_fwd"])
    cols_b = (b["_right"], b["_up"], b["_fwd"])
    rel = [[dot(cols_a[r], cols_b[c]) for c in range(3)] for r in range(3)]
    trace = rel[0][0] + rel[1][1] + rel[2][2]
    skew = (rel[2][1] - rel[1][2], rel[0][2] - rel[2][0], rel[1][0] - rel[0][1])
    half = norm(skew) / 2.0
    angle = math.atan2(half, max(-1.0, min(1.0, (trace - 1.0) / 2.0)))
    scale = 0.0 if half < 1e-12 else angle / (2.0 * half)
    return tuple(s * scale / float(a["dt"]) for s in skew)


def fit(xs, ys):
    """Least squares `y = c * x` through the origin, with the explained share."""
    sxx = sum(x * x for x in xs)
    if sxx <= 0.0:
        return math.nan, math.nan, math.nan
    c = sum(x * y for x, y in zip(xs, ys)) / sxx
    resid = sum((y - c * x) ** 2 for x, y in zip(xs, ys))
    total = sum(y * y for y in ys)
    return c, math.sqrt(resid / len(ys)), 1.0 - resid / total if total > 0 else math.nan


NAMES = ("pitch (right)", "yaw (up)", "roll (forward)")


def table(title, header, xs_by_axis, ys_by_axis, expected):
    print("\n%s" % title)
    print("  axis             %s   expected   ratio   %% explained     rms" % header)
    ratios = []
    for axis, name in enumerate(NAMES):
        c, rms, explained = fit(xs_by_axis[axis], ys_by_axis[axis])
        ratios.append(c / expected[axis])
        print(
            "  %-14s %+11.4f %+10.4f %+8.4f %11.2f %9.4f"
            % (name, c, expected[axis], c / expected[axis], 100.0 * explained, rms)
        )
    return ratios


def report_by_bank(basis, omega, avel, ups):
    """Both identities partitioned by how far the craft is from level.

    The partition is on `up.y` - a column, independent of every angular quantity
    being fitted - so this cannot be the circular "sort by residual, find the
    residual is large" reading. A pooled least-squares fit is dominated by the
    largest-amplitude samples, which is why the whole-lap ratios above look like
    one broken identity rather than the two different failures this shows.
    """
    print("\nboth identities by bank angle (up.y = 1 is level), as ratios to expected:")
    print("  up.y            n     (A) avel/(I*omega)          (B) -omega/basis")
    print("                        pitch    yaw   roll        pitch    yaw   roll")
    for lo, hi in ((0.99, 1.01), (0.97, 0.99), (0.93, 0.97), (0.85, 0.93), (0.0, 0.85)):
        picks = [i for i, u in enumerate(ups) if lo <= u < hi]
        if len(picks) < 30:
            continue
        a = [
            fit([omega[i][k] for i in picks], [avel[i][k] for i in picks])[0] / INERTIA[k]
            for k in range(3)
        ]
        b = [
            fit([basis[i][k] for i in picks], [omega[i][k] for i in picks])[0] / -1.0
            for k in range(3)
        ]
        print("  %.2f-%.2f %7d      %s        %s"
              % (lo, hi, len(picks),
                 " ".join("%6.3f" % v for v in a), " ".join("%6.3f" % v for v in b)))


def cross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def load_spline(path):
    """The track's surface point and normal, as `oag-trace track` dumps them.

    Only the normal is taken. The dumped tangent is per-sample and jitters
    enough to swamp the tilt components of a full triad-to-triad rotation - a
    first version of this test built the whole `(right, normal, tangent)` frame
    and measured a correlation of 0.5 % where the normal alone measures 72 %.
    The normal is the only part of the track frame the attitude question is
    about anyway.
    """
    points = []
    with open(path, newline="") as handle:
        for row in csv.DictReader(handle):
            points.append(
                (
                    tuple(float(row["pos_%s" % axis]) for axis in "xyz"),
                    tuple(-float(row["down_%s" % axis]) for axis in "xyz"),
                )
            )
    return points


def report_alignment(rows, usable, residual, spline_path, window):
    """Is the unaccounted rotation the one that keeps `up` on the surface?

    Two different models, and the lap separates them. A *spring* would make the
    extra rotation proportional to the misalignment `cross(up, normal)` and
    vanish when the two already agree. *Slaving* would make it match the rate at
    which the surface normal itself turns as the craft drives along the track -
    `cross(n(t), n(t+dt)) / dt` - whether or not there is any misalignment to
    correct. Both are tested here against the same residual.

    `window` averages over N intervals before fitting, which trades sample count
    for the jitter of matching a craft up to 27 units off the centre line to a
    sampled spline. The unsmoothed row is the one to quote.
    """
    points = load_spline(spline_path)
    n = len(points)
    print("\nagainst the track's own surface normal (%s, %d points):" % (spline_path, n))

    def nearest(around, position):
        best, best_d = around, float("inf")
        for step in range(-20, 120):
            at = (around + step) % n
            d = norm(tuple(p - q for p, q in zip(points[at][0], position)))
            if d < best_d:
                best, best_d = at, d
        return best, best_d

    # Matched over *every* tick in order, not only the kept ones: the search
    # window is relative to the last match, so skipping a dropped stretch would
    # walk the index off the track (it did, by 800 units, the first time).
    index = 0
    per_tick = []
    for row in rows:
        index, best_d = nearest(index, row["_pos"])
        per_tick.append((index, best_d))

    matched, offs, misalign = [], [], []
    for i in usable:
        index, best_d = per_tick[i]
        matched.append(index)
        offs.append(best_d)
        world = cross(rows[i]["_up"], points[index][1])
        misalign.append(
            (
                dot(world, rows[i]["_right"]),
                dot(world, rows[i]["_up"]),
                dot(world, rows[i]["_fwd"]),
            )
        )
    print("  matched to the spline within %.1f units (median %.1f)"
          % (max(offs), statistics.median(offs)))
    print("  sin(angle between the ship's up and the surface normal): median %.4f, max %.4f"
          % (statistics.median(norm(a) for a in misalign), max(norm(a) for a in misalign)))

    flat = lambda vs: [v for x in vs for v in (x[0], x[2])]
    c, rms, explained = fit(flat(misalign), flat(residual))
    print("  a spring:  residual tilt = %+.4f * cross(up, normal)"
          "        %5.1f %% explained, rms %.4f rad/s" % (c, 100.0 * explained, rms))

    print("\n  slaving:   residual tilt against the rate the surface normal itself turns")
    print("    window   n      ship's own tilt / normal      residual / normal      rms")
    for w in (1, window):
        turn, own, resid, count = [], [], [], 0
        for s in range(len(usable) - w):
            i0, i1 = usable[s], usable[s + w]
            if i1 - i0 != w:  # contiguous ticks only - a gap is a contact interval
                continue
            dt = sum(float(rows[t]["dt"]) for t in range(i0, i1))
            row = rows[i0]
            body = lambda v: (dot(v, row["_right"]), dot(v, row["_up"]), dot(v, row["_fwd"]))
            turn.append(body(tuple(c / dt for c in cross(points[matched[s]][1],
                                                         points[matched[s + w]][1]))))
            own.append(body(tuple(c / dt for c in cross(rows[i0]["_up"], rows[i1]["_up"]))))
            resid.append(tuple(statistics.fmean(residual[t][k] for t in range(s, s + w))
                               for k in range(3)))
            count += 1
        c_own, _, e_own = fit(flat(turn), flat(own))
        c_res, rms_res, e_res = fit(flat(turn), flat(resid))
        print("    %4d  %6d      %+.4f (%5.1f %%)          %+.4f (%5.1f %%)     %.4f"
              % (w, count, c_own, 100.0 * e_own, c_res, 100.0 * e_res, rms_res))
        if w == window and window == 1:
            break


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace")
    parser.add_argument(
        "--speed-bands",
        action="store_true",
        help="repeat both identities in 20 units/s bands, which is what separates "
        "the low-speed pitch captures from the lap's curving geometry",
    )
    parser.add_argument(
        "--min-up-y",
        type=float,
        metavar="Y",
        help="drop intervals where the craft is banked past this, `up.y` being "
        "the column. Talon's Junction has an inverted section the craft is held "
        "through, and it is a different regime from the rest of the lap - "
        "`--min-up-y 0.85` is how to look at either on its own.",
    )
    parser.add_argument(
        "--spline",
        help="`oag-trace track` output. Turns the attitude-alignment attribution "
        "into a measurement: the residual of (B) is fitted against the rotation "
        "that would take the ship's `up` onto the track's own surface normal.",
    )
    parser.add_argument(
        "--smooth",
        type=int,
        default=21,
        metavar="N",
        help="second window, in intervals, for the slaving fit under --spline. "
        "The unsmoothed row is always printed too and is the one to quote.",
    )
    args = parser.parse_args()

    rows = load(args.trace)
    ok = clean_mask(rows)
    usable = [i for i in range(len(rows) - 1) if ok[i]]
    if args.min_up_y is not None:
        kept = [i for i in usable if rows[i]["_up"][1] >= args.min_up_y]
        print("keeping %d of %d clean intervals with up.y >= %.2f"
              % (len(kept), len(usable), args.min_up_y))
        usable = kept
    print("%s: %d ticks, %d clean intervals (%.1f %%)"
          % (args.trace, len(rows), len(usable), 100.0 * len(usable) / max(1, len(rows) - 1)))
    speeds = [rows[i]["_v"] for i in usable]
    print("speed over those intervals: %.1f .. %.1f units/s (median %.1f)"
          % (min(speeds), max(speeds), statistics.median(speeds)))

    basis = [basis_angular_velocity(rows, i) for i in usable]
    omega = [rows[i]["_omega"] for i in usable]
    avel = [rows[i]["_avel"] for i in usable]

    table(
        "(A) avel(+0x160) = I * omega(+0x150) - the integrator's own map:",
        "avel/omega",
        [[o[k] for o in omega] for k in range(3)],
        [[a[k] for a in avel] for k in range(3)],
        INERTIA,
    )
    table(
        "(B) omega(+0x150) = -omega(basis) - what the basis actually did:",
        "col/basis ",
        [[b[k] for b in basis] for k in range(3)],
        [[o[k] for o in omega] for k in range(3)],
        (-1.0, -1.0, -1.0),
    )
    table(
        "(C) avel(+0x160) = -I * omega(basis) - the composite the old lap saw:",
        "avel/basis",
        [[b[k] for b in basis] for k in range(3)],
        [[a[k] for a in avel] for k in range(3)],
        tuple(-i for i in INERTIA),
    )

    print(
        "\nresidual of (B), i.e. the rotation the recorded angular velocity does not\n"
        "account for: r = omega(basis) + omega(+0x150), in rad/s"
    )
    residual = [tuple(b[k] + o[k] for k in range(3)) for b, o in zip(basis, omega)]
    power = statistics.fmean(dot(r, r) for r in residual)
    tilt = statistics.fmean(r[0] * r[0] + r[2] * r[2] for r in residual)
    print("  measured rotation rms   yaw %.4f   tilt %.4f"
          % (math.sqrt(statistics.fmean(b[1] ** 2 for b in basis)),
             math.sqrt(statistics.fmean(b[0] ** 2 + b[2] ** 2 for b in basis))))
    print("  residual rotation rms   yaw %.4f   tilt %.4f"
          % (math.sqrt(statistics.fmean(r[1] ** 2 for r in residual)), math.sqrt(tilt)))
    print("  fraction of the residual's power in the tilt plane   %.3f" % (tilt / power))

    print(
        "\nand the residual of (A), r = avel(+0x160) - I * omega(+0x150), which is\n"
        "what a second writer of the momentum column would show up as:"
    )
    resid_a = [
        tuple(a[k] - INERTIA[k] * o[k] for k in range(3)) for a, o in zip(avel, omega)
    ]
    for axis, name in enumerate(NAMES):
        measured = math.sqrt(statistics.fmean(a[axis] ** 2 for a in avel))
        r = math.sqrt(statistics.fmean(x[axis] ** 2 for x in resid_a))
        print("  %-14s rms %8.5f against a measured %8.4f   (%.4f %%)"
              % (name, r, measured, 100.0 * r / measured))

    report_by_bank(basis, omega, avel, [rows[i]["_up"][1] for i in usable])

    if args.spline:
        report_alignment(rows, usable, residual, args.spline, args.smooth)

    if args.speed_bands:
        print("\nby speed band - (A) then (B), as ratios to their expected value:")
        print("  |v| band     n     (A) pitch    yaw   roll      (B) pitch    yaw   roll")
        bands = {}
        for index, speed in enumerate(speeds):
            bands.setdefault(min(int(speed // 20) * 20, 160), []).append(index)
        for lo in sorted(bands):
            picks = bands[lo]
            if len(picks) < 30:
                continue
            a_ratio = [
                fit([omega[i][k] for i in picks], [avel[i][k] for i in picks])[0] / INERTIA[k]
                for k in range(3)
            ]
            b_ratio = [
                fit([basis[i][k] for i in picks], [omega[i][k] for i in picks])[0] / -1.0
                for k in range(3)
            ]
            print("  %3d-%-3d %7d      %s      %s"
                  % (lo, lo + 20, len(picks),
                     " ".join("%6.3f" % v for v in a_ratio),
                     " ".join("%6.3f" % v for v in b_ratio)))


if __name__ == "__main__":
    main()
