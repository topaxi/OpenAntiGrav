#!/usr/bin/env python3
"""Measure the original's lateral, forward and yaw response while cornering.

The companion to `scripts/trace-force-balance.py`. That script probes the
*forward* axis on straight-line captures; this one probes a capture that
corners, which lets it separate four things a straight cannot:

- `speed` (`body+0x398`) and `speed_cached` (`craft+0x2ec`) from `|velocity|`.
  All three coincide on a straight; at a slip angle they are three different
  numbers, and each is checked against its documented reading.
- the **lateral** force law, by projecting the measured acceleration onto the
  ship's right axis - where `Ship_ApplyLateralGrip` is the dominant term.
- the **airbrake** terms, which are identically zero in any capture that holds
  the airbrakes level.
- the **yaw** law, by comparing the recorded angular-momentum column against
  the rotation the recorded basis actually performs.

Method notes that make the numbers trustworthy:

- `<Physical mass>` is 1, so measured acceleration *is* net force.
- Forces are accumulated against the basis as it stands at the start of a
  frame and the integrator applies them over that same frame, so projecting
  `(v[i+1] - v[i]) / dt[i]` onto the axes recorded at tick `i` is exact rather
  than a small-angle approximation. No rotating-frame correction is needed or
  wanted: the projection measures world force, not body-frame velocity change.
- Gravity is removed analytically on both axes (`g * axis.y`, mass 1). It is
  not negligible on either.
- The hover spring and the inline vertical damping act along the ship's own
  **up** axis, so they cancel exactly in both projections by orthonormality.
  The hover *downforce* acts along the averaged surface normal instead, which
  is only approximately the ship's up - that is the one term this method
  cannot cancel, and it is why residuals are reported rather than hidden.
- `speed / |velocity| == 1.0000` is an exact per-tick test for a tick the
  contact response never touched; every fit below runs on those ticks only.

No shipped design data is embedded here: every handling value the analysis
needs is a command-line argument, read from the user's own disc at analysis
time (see `docs/formats/handling-stats.md` and ADR-0006 for why a tuning table
is content rather than documentation).

Usage:

    python3 scripts/trace-cornering.py data/traces/talons-junction-time-trial-lap.csv \
        --normal-gravity <Physical normal_gravity> \
        --accelcap <Engine accelcap> --engine-amount <Engine amount> \
        --grip-ground <Antigrav grip_ground> --grip-air <Antigrav grip_air> \
        --slidegrip <Airbrake slidegrip> --airbrake-amount <Airbrake amount> \
        --airbrake-drag <Airbrake drag> --airbrake-turn <Airbrake turn> \
        --turning-amount <Turning amount> --turning-gain <Turning gain> \
        --turning-falloff <Turning falloff>
"""

from __future__ import annotations

import argparse
import csv
import math
import statistics

# Read from the instruction stream, identical in both binaries; see
# `docs/ghidra/functions/psp-pulse/engine.md`, "The passive terms".
DRAG_GROUND = 0.005
ROLLING_RESISTANCE = 2.0
# `Body_Integrate`'s linear velocity damping, `body+0x384`.
LINEAR_DAMPING = 0.01
# `Body_SetBoxInertia` (0x0884e1ac), called once with the code-literal box
# (12, 8, 12) at mass 0.9, on (right, up, forward); see rigid-body.md.
INERTIA = (15.6, 21.6, 15.6)
# `Ship_ApplyAngularDamping` (0x08848ed0): the yaw entry is -5.
YAW_DAMPING = -5.0
# Load-time scalings, `HandlingXml_ParseAirbrake` / `ParseEngine`.
AIRBRAKE_AMOUNT_SCALE = 1e-4
SLIDEGRIP_SCALE = 1e-4
ENGINE_AMOUNT_SCALE = 1e-3
# The two literals on the airbrake drag term, `0x0884ccf4` and `0x0884cd78`.
AIRBRAKE_DRAG_SCALE = 1e-5
# The exact test for a tick untouched by the contact response.
CLEAN_TOLERANCE = 5e-5


def vec(row, prefix):
    return tuple(float(row[f"{prefix}_{axis}"]) for axis in "xyz")


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def norm(a):
    return math.sqrt(dot(a, a))


def load(path):
    with open(path, newline="") as handle:
        rows = list(csv.DictReader(handle))
    for row in rows:
        row["_vel"] = vec(row, "vel")
        row["_fwd"] = vec(row, "fwd")
        row["_right"] = vec(row, "right")
        row["_up"] = vec(row, "up")
        row["_avel"] = vec(row, "avel")
        row["_v"] = norm(row["_vel"])
        row["_fs"] = dot(row["_vel"], row["_fwd"])
        row["_lat"] = dot(row["_vel"], row["_right"])
        row["_ratio"] = float(row["speed"]) / row["_v"] if row["_v"] > 1e-9 else math.nan
    return rows


def clean_mask(rows):
    """Per-tick cleanliness of the *interval* `[i, i+1]`.

    `speed` is sampled at the top of `Ship_UpdateCraft`, so it reports the
    velocity as `Body_Integrate` left it at the end of the *previous* frame. A
    contact impulse applied during frame `i` therefore shows up in tick
    `i + 1`'s ratio, which is why both ends are required.
    """
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


def steer_targets(rows, gain, falloff):
    """Recover the raw `steerX` input from the ramped `craft+0x2c0` column.

    The trace records the ramp's output, but the airbrake drag term multiplies
    by the **raw** input - engine.md's `Ship_UpdateAirbrakes` reading is
    explicit that the two are different values, and `airbrake.rs` pins the
    distinction with a test. The ramp only ever steps by `gain * dt` (moving
    away from centre) or `falloff * dt` (moving back), and the two rates are
    far enough apart to invert unambiguously.

    Returns `(targets, unexplained)` where `targets[i]` is `-100`, `0` or
    `+100`, and `unexplained` counts steps matching neither rate.
    """
    targets, unexplained = [], 0
    for i in range(len(rows)):
        if i + 1 >= len(rows):
            targets.append(targets[-1] if targets else 0.0)
            continue
        dt = float(rows[i]["dt"])
        here = float(rows[i]["steer"])
        step = float(rows[i + 1]["steer"]) - here
        toward, back = gain * dt, falloff * dt
        if abs(abs(step) - toward) < 0.15 * toward:
            targets.append(100.0 if step > 0 else -100.0)
        elif abs(abs(step) - back) < 0.15 * back:
            # Moving back toward centre: either the stick is released, or it is
            # held at a target the ramp has overshot, which only happens past
            # full deflection.
            targets.append((100.0 if here > 0 else -100.0) if abs(here) > 100.0 else 0.0)
        elif here == 0.0 and step == 0.0:
            targets.append(0.0)
        elif abs(step) < back and abs(float(rows[i + 1]["steer"])) < 1e-9:
            targets.append(0.0)  # the clamp-to-exactly-zero branch
        else:
            unexplained += 1
            targets.append(0.0)
    return targets, unexplained


def solve(matrix, rhs):
    """Gauss-Jordan on a small dense system."""
    n = len(matrix)
    aug = [row[:] + [rhs[i]] for i, row in enumerate(matrix)]
    for col in range(n):
        pivot = max(range(col, n), key=lambda r: abs(aug[r][col]))
        aug[col], aug[pivot] = aug[pivot], aug[col]
        for row in range(n):
            if row == col:
                continue
            factor = aug[row][col] / aug[col][col]
            for k in range(col, n + 1):
                aug[row][k] -= factor * aug[col][k]
    return [aug[i][n] / aug[i][i] for i in range(n)]


def multifit(xs, ys):
    """Least squares `y = sum(c_i * x_i)`; returns `(coeffs, rms, explained, se)`.

    `explained` is against the sum of squares of `y` itself, not against its
    variance about the mean - these models are all through the origin by
    construction, so that is the meaningful denominator. `se` is the standard
    error of each coefficient, which is what separates "the data pins this
    term" from "the data tolerates it".
    """
    n = len(xs[0])
    normal = [[sum(x[i] * x[j] for x in xs) for j in range(n)] for i in range(n)]
    rhs = [sum(x[i] * y for x, y in zip(xs, ys)) for i in range(n)]
    coeffs = solve(normal, rhs)
    resid = sum((y - sum(c * v for c, v in zip(coeffs, x))) ** 2 for x, y in zip(xs, ys))
    total = sum(y * y for y in ys)
    variance = resid / max(1, len(ys) - n)
    inverse = [solve(normal, [1.0 if k == i else 0.0 for k in range(n)])[i] for i in range(n)]
    se = [math.sqrt(max(0.0, variance * d)) for d in inverse]
    return coeffs, math.sqrt(resid / len(ys)), 1.0 - resid / total, se


def body_angular_velocity(rows, i):
    """Angular velocity over `[i, i+1]`, expressed in the tick-`i` body frame.

    The recorded axes are the body axes in world coordinates, i.e. the columns
    of the body-to-world matrix, so `M[i]^T M[i+1]` is the frame-to-frame
    rotation already in body coordinates and its skew part is the rotation
    vector.
    """
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


def accel(row, nxt):
    dt = float(row["dt"])
    return tuple((b - a) / dt for a, b in zip(row["_vel"], nxt["_vel"]))


def grip_coefficient(row, args):
    """`k = max(L, R) * (0.01 - slidegrip) - 1`, the recovered grip scale."""
    applied = max(float(row["airbrake_l"]), float(row["airbrake_r"]))
    return applied * (0.01 - args.slidegrip * SLIDEGRIP_SCALE) - 1.0


def ramped(rows, i):
    """The airbrake and steering ramp states frame `i`'s force terms actually saw.

    `Ship_UpdateAirbrakes` and `Ship_UpdateSteering` ramp `craft+0x2c0`-`0x2c8`
    and *then* use the ramped value in the same call, but a capture samples the
    craft at the top of `Ship_UpdateCraft` - so tick `i`'s columns are the
    previous frame's ramp output, and tick `i + 1`'s are frame `i`'s.

    This is measured, not assumed: pairing the yaw accumulator against tick `i`
    fits its drive coefficient at `1.68x` the disc's `Turning.amount` with an
    rms of 17.0, and against tick `i + 1` at `1.00x` with an rms of 3.05.
    """
    return rows[i + 1]


def engine_thrust(row, args):
    if float(row["throttle"]) != 100.0:
        return 0.0
    cap = 100.0 * args.engine_amount * ENGINE_AMOUNT_SCALE
    return 2.0 * min(cap, 0.5 * row["_fs"] + args.accelcap)


def airbrake_drag_term(row, state, target, args):
    """`fs * |L - R| * Airbrake.drag * |steerX| * 1e-5`, along +forward."""
    imbalance = abs(float(state["airbrake_l"]) - float(state["airbrake_r"]))
    return abs(row["_fs"]) * imbalance * args.airbrake_drag * abs(target) * AIRBRAKE_DRAG_SCALE


def airbrake_lateral_term(row, state, args):
    """`fs * Airbrake.amount * (R - L)`, along +right, before the load scale."""
    delta = float(state["airbrake_r"]) - float(state["airbrake_l"])
    return abs(row["_fs"]) * delta


def airbrake_yaw_term(row, state, args):
    """`fs * Airbrake.turn * (R - L) * 1e-3`, into the angular-local `.y` lane."""
    delta = float(state["airbrake_r"]) - float(state["airbrake_l"])
    return abs(row["_fs"]) * args.airbrake_turn * delta * 1e-3


def forward_residual(row, nxt, target, args):
    """Measured net forward force minus every term already confirmed."""
    net = dot(accel(row, nxt), row["_fwd"]) + args.normal_gravity * row["_fwd"][1]
    passive = (
        -DRAG_GROUND * abs(row["_fs"]) * row["_fs"]
        - ROLLING_RESISTANCE * row["_fs"] / row["_v"]
        - LINEAR_DAMPING * row["_fs"]
    )
    return net - engine_thrust(row, args) - passive


def lateral_residual(row, nxt, args):
    """Measured net lateral force minus every term but grip and the airbrake."""
    net = dot(accel(row, nxt), row["_right"]) + args.normal_gravity * row["_right"][1]
    passive = (
        -DRAG_GROUND * abs(row["_fs"]) * row["_lat"]
        - ROLLING_RESISTANCE * row["_lat"] / row["_v"]
        - LINEAR_DAMPING * row["_lat"]
    )
    return net - passive


def pad_mask(rows, ok, targets, args):
    """Ticks carrying the track-section force - speed pads.

    A pad is not a contact, so the `speed / |v|` test cannot see it: it is a
    *force*, and the ratio stays at exactly 1.0000 straight through one. It is
    found here as a sustained large positive excess over the force law, and
    reported as a measurement in its own right rather than quietly dropped.
    """
    flagged = [False] * len(rows)
    for i in range(len(rows) - 1):
        if ok[i] and forward_residual(rows[i], rows[i + 1], targets[i], args) > args.pad_threshold:
            flagged[i] = True
    return flagged


def runs_of(flags):
    windows, start = [], None
    for i, flag in enumerate(flags):
        if flag and start is None:
            start = i
        elif not flag and start is not None:
            windows.append((start, i - 1))
            start = None
    if start is not None:
        windows.append((start, len(flags) - 1))
    return windows


def usable(rows, ok, pads, grounded=None):
    for i in range(len(rows) - 1):
        if not ok[i] or pads[i]:
            continue
        if grounded is not None and float(rows[i]["grounded"]) != grounded:
            continue
        yield i, rows[i], rows[i + 1]


def section(title):
    print(f"\n{title}\n{'-' * len(title)}")


def report_speed_columns(rows, ok):
    section("Cleanliness, and the three speed columns at a real slip angle")
    ratios = [r["_ratio"] for r in rows if not math.isnan(r["_ratio"])]
    exact = [r for r in ratios if abs(r - 1.0) < CLEAN_TOLERANCE]
    runs = runs_of(ok)
    print(f"ticks                        {len(rows)}")
    print(f"speed/|v| == 1.0000          {len(exact)} ({100.0 * len(exact) / len(ratios):.1f} %)")
    print(f"speed/|v| range              {min(ratios):.6f} .. {max(ratios):.6f}")
    print(f"worst deviation when clean   {max(abs(r - 1.0) for r in exact):.2e}")
    print(f"longest clean interval run   {max(hi - lo + 1 for lo, hi in runs)} ticks")

    moving = [row for row in rows if row["_v"] > 1.0]
    fresh = [row for row in moving if abs(row["_ratio"] - 1.0) < CLEAN_TOLERANCE]

    def slip(rs):
        return max(math.degrees(math.acos(max(-1.0, min(1.0, r["_fs"] / r["_v"])))) for r in rs)

    def gap(rs):
        return 100.0 * max(1.0 - r["_fs"] / r["_v"] for r in rs)

    print(f"slip angle, every tick       0.0 .. {slip(moving):.1f} degrees")
    print(f"slip angle, clean ticks      0.0 .. {slip(fresh):.1f} degrees   (what the fits see)")
    print(f"max (|v| - fs) / |v|         {gap(moving):.2f} % / {gap(fresh):.2f} % clean")
    print("                             (the separation a straight cannot give)")

    print("\n  slip angle bin    n    mean speed/|v|   max |ratio - 1|")
    binned = {}
    for row in rows:
        if math.isnan(row["_ratio"]) or abs(row["_ratio"] - 1.0) >= CLEAN_TOLERANCE or row["_v"] < 1.0:
            continue
        slip = math.degrees(math.acos(max(-1.0, min(1.0, row["_fs"] / row["_v"]))))
        binned.setdefault(min(int(slip // 4) * 4, 16), []).append(row["_ratio"])
    for lo in sorted(binned):
        values = binned[lo]
        print(
            f"  {lo:2d}-{lo + 4:2d} deg     {len(values):5d}    {statistics.fmean(values):.8f}"
            f"     {max(abs(v - 1.0) for v in values):.2e}"
        )

    # `craft+0x2ec` is documented as |dot(velocity, forward)|, one tick stale.
    # On a straight that is indistinguishable from |velocity|; here it is not.
    same = [abs(float(rows[i]["speed_cached"]) - abs(rows[i]["_fs"])) for i in range(len(rows) - 1)]
    lagged = [abs(float(rows[i + 1]["speed_cached"]) - abs(rows[i]["_fs"])) for i in range(len(rows) - 1)]
    magnitude = [abs(float(rows[i + 1]["speed_cached"]) - rows[i]["_v"]) for i in range(len(rows) - 1)]
    print("\n  speed_cached against three candidate readings:")
    print(f"    |dot(v, fwd)| of the same tick    mean {statistics.fmean(same):.5f}  max {max(same):.5f}")
    print(f"    |dot(v, fwd)| of the tick before  mean {statistics.fmean(lagged):.3e}  max {max(lagged):.3e}")
    print(f"    |velocity| of the tick before     mean {statistics.fmean(magnitude):.5f}  max {max(magnitude):.5f}")


def report_pads(rows, ok, pads, targets, args):
    section("The track-section force: speed pads are a force, not an impulse")
    windows = runs_of(pads)
    print(f"{sum(pads)} clean ticks carry a forward excess above {args.pad_threshold:+.0f} units,")
    print(f"in {len(windows)} contiguous windows:")
    print("\n   ticks        length   peak excess   |v| in -> out   speed/|v| throughout")
    for lo, hi in windows:
        excess = [forward_residual(rows[i], rows[i + 1], targets[i], args) for i in range(lo, hi + 1)]
        ratios = [rows[i]["_ratio"] for i in range(lo, hi + 2)]
        print(
            f"  {lo:4d}-{hi:<4d}    {hi - lo + 1:5d}   {max(excess):+11.1f}"
            f"   {rows[lo]['_v']:6.1f} -> {rows[hi + 1]['_v']:<6.1f}"
            f"  {min(ratios):.6f}-{max(ratios):.6f}"
        )
    print(
        "\nA one-frame velocity impulse would show as a single tick and would\n"
        "leave `speed / |v|` alone only if it were applied before the integrator.\n"
        "These are multi-tick, they ramp, and the ratio holds at 1.0000 across\n"
        "them - so on this track the boost enters the force accumulators."
    )


def report_forward(rows, ok, pads, targets, args):
    section("The forward axis while cornering")
    print(
        "residual = dot(a, fwd) + g * fwd.y\n"
        "           - 2 * min(100 * amount * 1e-3, 0.5 * fs + accelcap)   engine\n"
        "           + 0.005 * |fs| * fs                                   quadratic drag\n"
        "           + 2 * fs / |v|                                        rolling resistance\n"
        "           + 0.01 * fs                                           integrator damping\n"
        "and is then regressed on a constant, `fs`, and the airbrake drag term\n"
        "`fs * |L-R| * Airbrake.drag * |steerX| * 1e-5`."
    )
    xs, ys, terms = [], [], []
    for i, row, nxt in usable(rows, ok, pads, grounded=1.0):
        term = airbrake_drag_term(row, ramped(rows, i), targets[i], args)
        xs.append((1.0, row["_fs"], term))
        ys.append(forward_residual(row, nxt, targets[i], args))
        terms.append(term)
    coeffs, rms, explained, se = multifit(xs, ys)
    active = sum(1 for t in terms if t > 1e-9)
    print(f"\n{len(ys)} usable intervals, {active} with the airbrake drag term active")
    print(f"  fs range              {min(x[1] for x in xs):.1f} .. {max(x[1] for x in xs):.1f}")
    print(f"  drag term as read     median {statistics.median(t for t in terms if t > 1e-9):.2f}, max {max(terms):.2f}")
    print(f"\n  constant              {coeffs[0]:+8.3f}")
    print(f"  per unit of fs        {coeffs[1]:+8.5f}")
    print(f"  x the read drag term  {coeffs[2]:+8.4f} +/- {se[2]:.4f}   (the reading predicts +1)")
    print(f"  rms {rms:.3f}, {100.0 * explained:.1f} % of the residual explained")

    # The same fit without the drag term, to show what it is worth.
    bare, bare_rms, _, _ = multifit([(x[0], x[1]) for x in xs], ys)
    print(f"  dropping the drag term: rms {bare_rms:.3f} (constant {bare[0]:+.3f}, per fs {bare[1]:+.5f})")

    # The lateral axis carries a force an order of magnitude larger than
    # anything unexplained here, so a small leak between the two projections
    # would show as a slip-angle dependence. Test for one rather than assume
    # its absence: it changes what the airbrake coefficient above means.
    slip_xs = [x + (abs(row["_lat"]),) for x, (_, row, _) in zip(xs, usable(rows, ok, pads, grounded=1.0))]
    slip, slip_rms, _, slip_se = multifit(slip_xs, ys)
    print(f"  adding a |dot(v, right)| term: rms {slip_rms:.3f}, coefficient"
          f" {slip[3]:+.4f} +/- {slip_se[3]:.4f}, and the drag term moves to {slip[2]:+.4f}")

    print("\n   predicted drag term    n    median residual   median fitted")
    binned = {}
    for x, y in zip(xs, ys):
        binned.setdefault(min(int(x[2] // 4) * 4, 28), []).append((y, sum(c * v for c, v in zip(coeffs, x))))
    for lo in sorted(binned):
        values = binned[lo]
        label = f"{lo:2d}-{lo + 4:<2d}" if lo < 28 else "28+  "
        print(
            f"  {label:>10}        {len(values):5d}   {statistics.median(v for v, _ in values):+13.2f}"
            f"   {statistics.median(f for _, f in values):+13.2f}"
        )


def report_lateral(rows, ok, pads, args):
    section("The lateral axis: what the grip term actually looks like")
    print(
        "residual = dot(a, right) + g * right.y\n"
        "           + 0.005 * |fs| * v_lat + 2 * v_lat / |v| + 0.01 * v_lat\n"
        "regressed on the two terms that write the right axis:\n"
        "  Ship_ApplyLateralGrip   grip * dot(v, right) * k,  k = max(L,R) * (0.01 - slidegrip) - 1\n"
        "  Ship_UpdateAirbrakes    fs * Airbrake.amount * (R - L)"
    )
    xs, ys, lats = [], [], []
    for i, row, nxt in usable(rows, ok, pads, grounded=1.0):
        state = ramped(rows, i)
        xs.append(
            (
                row["_lat"] * grip_coefficient(state, args),
                airbrake_lateral_term(row, state, args),
                1.0,
            )
        )
        ys.append(lateral_residual(row, nxt, args))
        lats.append(row["_lat"])
    coeffs, rms, explained, se = multifit(xs, ys)
    print(f"\n{len(ys)} usable intervals, |v_lat| up to {max(abs(v) for v in lats):.2f}")
    stored = args.airbrake_amount * AIRBRAKE_AMOUNT_SCALE
    print(f"  x (v_lat * k)         {coeffs[0]:+9.4f} +/- {se[0]:.4f}   the disc's grip_ground"
          f"  ->  {coeffs[0] / args.grip_ground:.4f}")
    print(f"  x (fs * (R - L))      {coeffs[1]:+9.6f} +/- {se[1]:.6f}   the disc's Airbrake.amount"
          f" as stored  ->  {coeffs[1] / stored:.4f}")
    print(f"  constant              {coeffs[2]:+9.4f}")
    print(f"  rms {rms:.3f}, {100.0 * explained:.2f} % explained")

    print("\n  alternative shapes for the grip term, each fitted alone:")
    for label, basis in (
        ("grip * v_lat * k          ", lambda i, row: row["_lat"] * grip_coefficient(ramped(rows, i), args)),
        ("grip * v_lat  (k ignored) ", lambda i, row: row["_lat"]),
        ("grip * v_lat * |v_lat|    ", lambda i, row: row["_lat"] * abs(row["_lat"])),
        ("grip * v_lat * |v|        ", lambda i, row: row["_lat"] * row["_v"]),
    ):
        single, single_rms, single_ex, _ = multifit(
            [(basis(i, row),) for i, row, _ in usable(rows, ok, pads, grounded=1.0)],
            [lateral_residual(row, nxt, args) for _, row, nxt in usable(rows, ok, pads, grounded=1.0)],
        )
        print(f"    {label} {single[0]:+11.4f}   rms {single_rms:7.3f}   {100.0 * single_ex:6.2f} % explained")

    print("\n   |v_lat|      n    median residual   median fitted   median |fitted - residual|")
    binned = {}
    for x, y, lateral in zip(xs, ys, lats):
        fitted = sum(c * v for c, v in zip(coeffs, x))
        binned.setdefault(min(int(abs(lateral) // 4) * 4, 16), []).append((y, fitted))
    for lo in sorted(binned):
        values = binned[lo]
        label = f"{lo:2d}-{lo + 4:<2d}" if lo < 16 else "16+  "
        print(
            f"  {label:>8}  {len(values):6d}   {statistics.median(v for v, _ in values):+13.2f}"
            f"   {statistics.median(f for _, f in values):+13.2f}"
            f"   {statistics.median(abs(f - v) for v, f in values):+10.2f}"
        )

    airborne = [i for i, row, _ in usable(rows, ok, pads, grounded=0.5)]
    print(f"\n  half-grounded ticks in the capture: {len(airborne)} - too few to separate grip_air")


def report_angular(rows, ok, pads, args):
    section("The angular columns against the rotation the basis actually performs")
    print(
        "avel_* is body+0x160, body-frame angular MOMENTUM, so with the box\n"
        f"tensor I = {INERTIA} on (right, up, forward) it should read\n"
        "exactly -I * omega, omega recovered from the recorded basis."
    )
    omegas, avels, handed = [], [], []
    for i, row, nxt in usable(rows, ok, pads):
        omegas.append(body_angular_velocity(rows, i))
        avels.append(row["_avel"])
        handed.append(dot(cross(row["_right"], row["_up"]), row["_fwd"]))
    print(f"\nusable intervals {len(omegas)}; basis handedness dot(right x up, fwd)"
          f" median {statistics.median(handed):+.6f}")
    print("\n  axis             avel/omega    -I     ratio   % explained   rms")
    names = ("pitch (right)", "yaw (up)", "roll (forward)")
    for axis, name in enumerate(names):
        coeffs, rms, explained, se = multifit(
            [(o[axis],) for o in omegas], [a[axis] for a in avels]
        )
        print(
            f"  {name:<14} {coeffs[0]:+11.3f} {-INERTIA[axis]:+7.1f}  {coeffs[0] / -INERTIA[axis]:+7.3f}"
            f"  {100.0 * explained:11.1f}   {rms:.3f}"
        )
    print("\n  the same fit allowing cross-axis coupling (rows: avel component):")
    for axis, name in enumerate(names):
        coeffs, rms, explained, se = multifit(omegas, [a[axis] for a in avels])
        print(
            f"  {name:<14} " + "  ".join(f"{c:+9.3f}" for c in coeffs)
            + f"   rms {rms:.3f}   {100.0 * explained:.1f} %"
        )

    report_rotation_residual(rows, ok, pads, omegas, avels)
    report_yaw_accumulator(rows, ok, pads, args)


def report_rotation_residual(rows, ok, pads, omegas, avels):
    """Where the rotation the momentum column does not account for lives."""
    print(
        "\n  what `omega - (-L / I)` looks like. A rotation that tilts `up` onto a\n"
        "  surface has no component about `up`; a rotation the momentum drives has\n"
        "  no reason to avoid one."
    )
    residual = [
        tuple(o[k] + a[k] / INERTIA[k] for k in range(3)) for o, a in zip(omegas, avels)
    ]
    power = statistics.fmean(dot(r, r) for r in residual)
    tilt = statistics.fmean(r[0] * r[0] + r[2] * r[2] for r in residual)
    print(f"    measured rotation rms   yaw {math.sqrt(statistics.fmean(o[1] ** 2 for o in omegas)):.4f}"
          f"   tilt {math.sqrt(statistics.fmean(o[0] ** 2 + o[2] ** 2 for o in omegas)):.4f} rad/s")
    print(f"    residual rotation rms   yaw {math.sqrt(statistics.fmean(r[1] ** 2 for r in residual)):.4f}"
          f"   tilt {math.sqrt(tilt):.4f} rad/s")
    print(f"    fraction of the residual's power in the tilt plane   {tilt / power:.3f}")
    print("\n     |v| bin      n    median residual tilt rate   / |v|")
    binned = {}
    speeds = [row["_v"] for _, row, _ in usable(rows, ok, pads)]
    for r, speed in zip(residual, speeds):
        binned.setdefault(min(int(speed // 20) * 20, 140), []).append(math.hypot(r[0], r[2]))
    for lo in sorted(binned):
        values = binned[lo]
        if len(values) < 10:
            continue
        median = statistics.median(values)
        print(f"    {lo:3d}-{lo + 20:<3d} {len(values):7d}          {median:.4f}            {median / (lo + 10):.6f}")


def report_yaw_accumulator(rows, ok, pads, args):
    print(
        "\n  The yaw accumulator, term by term. Ship_ApplyAngularDamping damps the\n"
        "  momentum, so every writer of the `.y` lane lands in one accumulator with\n"
        "  no inertia between them:\n"
        "    d(avel_y)/dt = steer * Turning.amount          Ship_UpdateSteering\n"
        "                 - 5 * avel_y                       Ship_ApplyAngularDamping\n"
        "                 + fs * Airbrake.turn * (R-L) * 1e-3  Ship_UpdateAirbrakes\n"
        "                 - 0.1 * dot(v, right)               Ship_ApplyWeathervaneTorque\n"
        "                 + 30 * right.y                      the hover epilogue\n"
        "  The weathervane's yaw component is `dot(cross(fwd, v), up)`, which is\n"
        "  `dot(v, right)` exactly, by orthonormality."
    )
    xs, ys = [], []
    for i, row, nxt in usable(rows, ok, pads):
        state = ramped(rows, i)
        xs.append(
            (
                float(state["steer"]),
                row["_avel"][1],
                airbrake_yaw_term(row, state, args),
                -0.1 * row["_lat"],
                30.0 * row["_right"][1],
            )
        )
        ys.append((nxt["_avel"][1] - row["_avel"][1]) / float(row["dt"]))
    labels = ("drive/amount", "damping", "x airbrake yaw", "x weathervane", "x bank-to-yaw")
    print(f"\n  over {len(ys)} intervals, |steer| up to {max(abs(x[0]) for x in xs):.1f}:")
    for keep, title in ((2, "steering and damping alone"), (3, "+ airbrake yaw"), (5, "the whole accumulator")):
        coeffs, rms, explained, se = multifit([x[:keep] for x in xs], ys)
        scaled = [coeffs[0] / args.turning_amount] + list(coeffs[1:])
        errors = [se[0] / args.turning_amount] + list(se[1:])
        parts = ", ".join(
            f"{name} {value:+.4f} +/- {error:.4f}"
            for name, value, error in zip(labels, scaled, errors)
        )
        print(f"    {title:<26} rms {rms:7.3f}  {100.0 * explained:5.2f} %")
        print(f"      {parts}")
    print(f"    (the reading predicts +1.0000 on every one but the damping, which is {YAW_DAMPING:+.1f})")
    print(
        "\n  the same fit against the ramp states recorded at tick i rather than the\n"
        "  post-ramp ones, i.e. the naive pairing - kept because the gap between the\n"
        "  two is what establishes the one-tick offset:"
    )
    naive_xs, naive_ys = [], []
    for i, row, nxt in usable(rows, ok, pads):
        naive_xs.append(
            (
                float(row["steer"]),
                row["_avel"][1],
                airbrake_yaw_term(row, row, args),
                -0.1 * row["_lat"],
                30.0 * row["_right"][1],
            )
        )
        naive_ys.append((nxt["_avel"][1] - row["_avel"][1]) / float(row["dt"]))
    coeffs, rms, explained, _ = multifit(naive_xs, naive_ys)
    print(
        f"    drive/amount {coeffs[0] / args.turning_amount:+.4f}, damping {coeffs[1]:+.4f},"
        f" x airbrake yaw {coeffs[2]:+.4f}   rms {rms:.3f}  {100.0 * explained:.2f} %"
    )


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("trace")
    parser.add_argument("--normal-gravity", type=float, required=True, help="<Physical normal_gravity>, off your own disc")
    parser.add_argument("--accelcap", type=float, required=True, help="<Engine accelcap>")
    parser.add_argument("--engine-amount", type=float, required=True, help="<Engine amount>, raw XML")
    parser.add_argument("--grip-ground", type=float, required=True, help="<Antigrav grip_ground>")
    parser.add_argument("--grip-air", type=float, required=True, help="<Antigrav grip_air>")
    parser.add_argument("--slidegrip", type=float, required=True, help="<Airbrake slidegrip>, raw XML")
    parser.add_argument("--airbrake-amount", type=float, required=True, help="<Airbrake amount>, raw XML")
    parser.add_argument("--airbrake-drag", type=float, required=True, help="<Airbrake drag>")
    parser.add_argument("--airbrake-turn", type=float, required=True, help="<Airbrake turn>")
    parser.add_argument("--turning-amount", type=float, required=True, help="<Turning amount>")
    parser.add_argument("--turning-gain", type=float, required=True, help="<Turning gain>")
    parser.add_argument("--turning-falloff", type=float, required=True, help="<Turning falloff>")
    parser.add_argument(
        "--pad-threshold",
        type=float,
        default=40.0,
        help="units/s^2 of unexplained forward acceleration that marks a track-section force",
    )
    args = parser.parse_args()

    rows = load(args.trace)
    ok = clean_mask(rows)
    targets, unexplained = steer_targets(rows, args.turning_gain, args.turning_falloff)
    pads = pad_mask(rows, ok, targets, args)

    report_speed_columns(rows, ok)
    print(f"\nsteer ramp inverted to a raw input on {len(rows) - unexplained} of {len(rows)} ticks")
    report_pads(rows, ok, pads, targets, args)
    report_forward(rows, ok, pads, targets, args)
    report_lateral(rows, ok, pads, args)
    report_angular(rows, ok, pads, args)


if __name__ == "__main__":
    main()
