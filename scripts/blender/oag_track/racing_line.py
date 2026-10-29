"""The AI racing line: one lateral offset per spline point, inside the AI corridor.

**Chosen, not measured.** The engine only reads the number (`racing_line`, a signed
distance along the point's own `lateral` axis, positive to the driver's right, from
the spline point; `oag_vex::track::Sample`, `oag_raceplay::spline::racing_line`). How
a track's author arrives at it is open, and this is the default the add-on offers
when the author draws none.

Method: **minimum curvature**, in the usual linearised form (Braghin et al.,
"Race driver model", 2008; Heilmeier et al., "Minimum curvature trajectory planning",
2019). Moving point `i` by `o[i]` along its lateral axis gives the driven line
`q[i] = p[i] + o[i] * lateral[i]`. Its curvature at `k` is the sideways part of the
second difference over the square of the gap, `kappa[k] = lateral[k] . (q[k-1] - 2
q[k] + q[k+1]) / h[k]^2` (`h[k]` is the gap at `k`), and the solver minimises the
integral of its square, `sum_k kappa[k]^2 h[k]`, plus a tie-break:

    E(o) = sum_k w[k]^2 * (lateral[k] . (q[k-1] - 2 q[k] + q[k+1]))^2
           + RIDGE * sum_k h[k] * o[k]^2,            w[k] = h[k]^(-3/2)

With `h` fixed `E` is a quadratic form in `o`: `E = o.A.o + 2 g.o + const`, `A`
a cyclic matrix of bandwidth 2. The gap `h` depends on `o` (the outside of a bend
is longer), so the quadratic is solved `ROUNDS` times, each with the gaps of the
line the last one found; a fixed count, so the result never depends on a
tolerance. Without it the minimum hugs the inside of every bend (the second
difference of a long outside line is larger at the same index spacing).

Each `o[k]` is boxed to the middle of its corridor, `INSET` of the corridor's width in
from each bound (never less than `MARGIN`). The
box-constrained minimum is found by an **active set**: solve the free points exactly
(sparse Gaussian elimination, linear in the ring size), pin the ones that left the
box, release the pinned ones whose gradient points back inside, repeat. Every step
is a fixed visiting order over doubles, so the result is the same on every machine;
only the final offset is rounded to `f32` (`f32.r`), which is what the file stores.
`RIDGE` (a weight per length to the fourth, so the choice does not move with the
number of points) only picks a position where curvature does not care, as on a
straight, and is far too small to pull a bend's line back to the middle.

What it does to a track: through a bend the line runs wide at the entry, to the inside
edge of its box at the apex and wide again at the exit, the way a driver cuts a
corner, and on a straight it relaxes towards the middle (the tiny `RIDGE` is what
picks a position where curvature does not care). Its freedom is the box, so on a track
whose bends are gentle against the corridor the gain is small; the box is what the
original driver tolerates (see `INSET`).

Nothing here imports `bpy`.
"""

from . import f32

# Least room kept between the line and either AI bound: Pulse's loader forces the
# corridor to straddle the line by this much (docs/formats/track.md).
MARGIN = 0.1
# The line also stays out of the outer `INSET` of the corridor's width each side, so
# the middle 40 percent. The original driver measures its lane across the room on
# each side of the line, and a line near a bound leaves one side with almost none:
# on `blender_loop` (corridor +-12) a line allowed to 0.1 from the bound, and one
# allowed to 0.2 of the width in, both leave the AI's speed plan unable to get round
# (the learned plan never finishes a lap, "1 unresolved"); 0.25 and above get round.
# 0.3 keeps a margin from that cliff. Chosen, not measured.
INSET = 0.3
RIDGE = 1e-12
ROUNDS = 6
KNOTS = 64
MAX_ROUNDS = 100
GRADIENT_TOLERANCE = 1e-9

def _dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def _box(points, margin):
    """Per point `(low, high)`: the corridor shrunk by `INSET` of its width (at least `margin`) each side."""
    out = []
    for p in points:
        room = max(margin, INSET * (p.ai_bound_right - p.ai_bound_left))
        lo, hi = p.ai_bound_left + room, p.ai_bound_right - room
        if lo > hi:
            lo = hi = (p.ai_bound_left + p.ai_bound_right) / 2.0
        out.append((lo, hi))
    return out


def _gaps(points, offsets):
    """`h[k]`: half the chord from the driven line's previous point to its next."""
    n = len(points)
    q = [
        tuple(p.pos[c] + o * p.lateral[c] for c in range(3))
        for p, o in zip(points, offsets)
    ]
    return [
        sum((q[(k + 1) % n][c] - q[(k - 1) % n][c]) ** 2 for c in range(3)) ** 0.5 / 2.0
        for k in range(n)
    ]


def _normal_equations(points, gaps):
    """`A` (rows of `{column: value}`) and `g` for `E = o.A.o + 2 g.o`, given the gaps."""
    n = len(points)
    pos = [p.pos for p in points]
    lat = [p.lateral for p in points]
    rows = [{} for _ in range(n)]
    g = [0.0] * n
    for k in range(n):
        prev, nxt = (k - 1) % n, (k + 1) % n
        sideways = _dot(
            lat[k], tuple(pos[prev][c] - 2.0 * pos[k][c] + pos[nxt][c] for c in range(3))
        )
        weight = gaps[k] ** -3.0
        terms = (
            (prev, _dot(lat[k], lat[prev])),
            (k, -2.0 * _dot(lat[k], lat[k])),
            (nxt, _dot(lat[k], lat[nxt])),
        )
        for i, ci in terms:
            g[i] += weight * ci * sideways
            row = rows[i]
            for j, cj in terms:
                row[j] = row.get(j, 0.0) + weight * ci * cj
    for k in range(n):
        rows[k][k] += RIDGE * gaps[k]
    return rows, g


def _eliminate(rows, rhs):
    """Solve the symmetric positive definite system `rows . x = rhs` (rows destroyed).

    Gaussian elimination in index order over sparse rows. A cyclic band only fills
    in the last rows, so the work stays linear in the size.
    """
    n = len(rows)
    for k in range(n):
        pivot_row = rows[k]
        pivot = pivot_row[k]
        later = [j for j in pivot_row if j > k]
        for i in later:
            row = rows[i]
            factor = row.pop(k) / pivot
            for j in later:
                row[j] = row.get(j, 0.0) - factor * pivot_row[j]
            rhs[i] -= factor * rhs[k]
    x = [0.0] * n
    for k in range(n - 1, -1, -1):
        row = rows[k]
        total = rhs[k]
        for j, v in row.items():
            if j > k:
                total -= v * x[j]
        x[k] = total / row[k]
    return x


def _solve_free(rows, g, free, pinned):
    """The minimum over the `free` indices with the `pinned` ones held at their values."""
    slot = {i: s for s, i in enumerate(free)}
    sub, rhs = [], []
    for i in free:
        row, r = {}, -g[i]
        for j, v in rows[i].items():
            if j in slot:
                row[slot[j]] = v
            else:
                r -= v * pinned[j]
        sub.append(row)
        rhs.append(r)
    return _eliminate(sub, rhs)


def _active_set(rows, g, box):
    """Minimum of `o.A.o + 2 g.o` with `o[i]` in `box[i]`."""
    n = len(rows)
    pinned = {}  # index -> (value, 'low' | 'high')
    offsets = [0.0] * n
    for _ in range(MAX_ROUNDS):
        free = [i for i in range(n) if i not in pinned]
        values = {i: v for i, (v, _) in pinned.items()}
        if free:
            solved = _solve_free(rows, g, free, values)
            for i, v in zip(free, solved):
                offsets[i] = v
        for i, v in values.items():
            offsets[i] = v
        outside = [i for i in free if not box[i][0] <= offsets[i] <= box[i][1]]
        if outside:
            for i in outside:
                if offsets[i] < box[i][0]:
                    pinned[i] = (box[i][0], "low")
                else:
                    pinned[i] = (box[i][1], "high")
                offsets[i] = pinned[i][0]
            continue
        released = []
        for i, (_, side) in pinned.items():
            slope = g[i] + sum(v * offsets[j] for j, v in rows[i].items())
            if (side == "low" and slope < -GRADIENT_TOLERANCE) or (
                side == "high" and slope > GRADIENT_TOLERANCE
            ):
                released.append(i)
        if not released:
            break
        for i in released:
            del pinned[i]
    return offsets


def _catmull_rom(values, k, t):
    """The cyclic cubic through `values[k]` (at `t = 0`) and `values[k + 1]` (at `t = 1`)."""
    n = len(values)
    p0, p1, p2, p3 = (values[(k + c) % n] for c in (-1, 0, 1, 2))
    return 0.5 * (
        2.0 * p1
        + (p2 - p0) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t**3
    )


def solve(points, margin=MARGIN):
    """The minimum-curvature offsets for a closed ring of `SplinePoint`s, as `f32`.

    `pos`, `lateral`, `ai_bound_left` and `ai_bound_right` are read; nothing is
    written. Fewer than 5 points have no curvature to speak of and keep the centre.

    A ring of more than `KNOTS` points is solved on every `m`-th point and the
    offsets are joined by a cyclic cubic (Catmull-Rom) between them. The polyline a curve is sampled into
    has a kink at every vertex, and at a fine spacing a curvature minimiser spends
    its freedom fitting those instead of the bends; the knots keep it to the bends,
    and the cost linear in the knots whatever the ring.
    """
    n = len(points)
    if n < 5:
        return [0.0] * n
    step = max(1, -(-n // KNOTS))
    knots = points[::step]
    if len(knots) < 5:
        step, knots = 1, points
    box = _box(knots, margin)
    offsets = [0.0] * len(knots)
    for _ in range(ROUNDS):
        rows, g = _normal_equations(knots, _gaps(knots, offsets))
        offsets = _active_set(rows, g, box)
    count = len(knots)
    fine = []
    for i in range(n):
        k, t = divmod(i, step)
        span = step if k + 1 < count else n - k * step
        fine.append(_catmull_rom(offsets, k, t / span))
    return [f32.r(min(max(o, lo), hi)) for o, (lo, hi) in zip(fine, _box(points, margin))]


def curvature_energy(points, offsets):
    """`sum_k |q[k-1] - 2 q[k] + q[k+1]|^2 / h[k]^3` of the driven line: the integral
    of curvature squared, whole (tangential part included). Lower is straighter;
    the check that a line is better than the centre."""
    n = len(points)
    q = [
        tuple(p.pos[c] + o * p.lateral[c] for c in range(3))
        for p, o in zip(points, offsets)
    ]
    total = 0.0
    for k, h in enumerate(_gaps(points, offsets)):
        a, b, c = q[(k - 1) % n], q[k], q[(k + 1) % n]
        total += sum((a[i] - 2.0 * b[i] + c[i]) ** 2 for i in range(3)) / h**3
    return total


def project(points, polyline, margin=MARGIN):
    """Offsets read off a hand-drawn closed `polyline` (format axes), as `f32`.

    For each spline point the line is cut with the plane through the point whose
    normal is the point's tangent, going forward through it; of the cuts the one
    nearest the point wins, and its distance along the point's `lateral` is the
    offset. A nearest-point projection would be wrong in a bend, where the nearest
    point on a line to one side is not across from the point. The result is boxed
    to the corridor like the solver's. Raises `ValueError` naming a point no part of
    the line crosses.
    """
    m = len(polyline)
    if m < 2:
        raise ValueError("the racing line needs at least 2 points")
    box = _box(points, margin)
    out = []
    for index, (p, (lo, hi)) in enumerate(zip(points, box)):
        t = p.tangent
        side = [_dot((v[0] - p.pos[0], v[1] - p.pos[1], v[2] - p.pos[2]), t) for v in polyline]
        best, best_d = None, None
        for a in range(m):
            b = (a + 1) % m
            if side[a] <= 0.0 < side[b]:
                u = -side[a] / (side[b] - side[a])
                hit = tuple(polyline[a][c] + (polyline[b][c] - polyline[a][c]) * u for c in range(3))
                rel = (hit[0] - p.pos[0], hit[1] - p.pos[1], hit[2] - p.pos[2])
                d = _dot(rel, rel)
                if best_d is None or d < best_d:
                    best, best_d = rel, d
        if best is None:
            raise ValueError(f"the racing line never crosses spline point {index}'s cross-section")
        out.append(f32.r(min(max(_dot(best, p.lateral), lo), hi)))
    return out
