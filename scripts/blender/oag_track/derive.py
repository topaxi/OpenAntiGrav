"""Authoring maths: a closed centre line in, the spline records the format needs out.

Everything here is **chosen, not measured**. It follows the generator behind the
committed fixture (`crates/trackfile/src/fixture.rs`) step for step, in `f32`,
so a flat loop authored in Blender matches what that generator would write. The
only generalisation is a third dimension: the lateral axis stays horizontal and
the up axis follows the slope. Banking is not authored (open item in
docs/tools/blender.md).

Inputs and outputs are in the format's axes (right-handed, Y up).
"""

import math

from . import f32
from .model import SHORT_POINT_VERSION, CollisionMesh, Pad, Path, SplinePoint, Start

WORLD_UP = (0.0, 1.0, 0.0)

# The host engine's own start-line advance (`oag_race::Course::START_LINE_ADVANCE`):
# the lap line sits this far ahead of the start slot along the spline.
START_BEHIND = 154.0

# Chosen defaults, shared with the fixture generator.
HALF_WIDTH = 20.0
WALL_HEIGHT = 15.0
AI_FRACTION = 0.6
PAD_BOX = ((-10.0, -2.0, -10.0), (10.0, 8.0, 10.0))


class AuthoringError(Exception):
    """The centre line cannot be turned into a track; the message says why."""


def resample_closed(points, count):
    """`count` points evenly spaced by arc length along the closed polyline."""
    n = len(points)
    if n < 3:
        raise AuthoringError("a closed centre line needs at least 3 points")
    if count < 3:
        raise AuthoringError("a spline needs at least 3 points")
    gaps = [math.dist(points[i], points[(i + 1) % n]) for i in range(n)]
    total = sum(gaps)
    if total <= 0.0:
        raise AuthoringError("the centre line has no length")
    out, seg, run = [], 0, 0.0
    for k in range(count):
        want = total * k / count
        while seg < n - 1 and run + gaps[seg] <= want:
            run += gaps[seg]
            seg += 1
        t = 0.0 if gaps[seg] == 0.0 else (want - run) / gaps[seg]
        a, b = points[seg], points[(seg + 1) % n]
        out.append(tuple(f32.r(a[i] + (b[i] - a[i]) * t) for i in range(3)))
    return out


def _lateral(forward):
    lateral = f32.cross(forward, WORLD_UP)
    if f32.length(lateral) < 1e-6:
        raise AuthoringError("the centre line runs vertically; no lateral axis")
    return f32.unit(lateral)


def build_path(positions, half_width=HALF_WIDTH, ai_fraction=AI_FRACTION):
    """The spline records for a closed ring of `positions`, as one `Path`."""
    n = len(positions)
    if n < 3:
        raise AuthoringError("a spline needs at least 3 points")
    pos = [tuple(f32.r(c) for c in p) for p in positions]
    gaps = [f32.dist(pos[i], pos[(i + 1) % n]) for i in range(n)]
    if min(gaps) <= 0.0:
        raise AuthoringError("two neighbouring spline points coincide")
    total = 0.0
    for g in gaps:
        total = f32.r(total + g)
    hw = f32.r(half_width)
    bound = f32.r(hw * f32.r(ai_fraction))
    points, run = [], 0.0
    for i in range(n):
        forward = f32.unit(f32.sub(pos[(i + 1) % n], pos[(i - 1) % n]))
        lateral = _lateral(forward)
        up = f32.unit(f32.cross(lateral, forward))
        down = (-up[0], -up[1], -up[2])
        points.append(
            SplinePoint(
                pos=pos[i],
                tangent=forward,
                down=down,
                lateral=lateral,
                progress=f32.r(run / total),
                half_width_left=hw,
                half_width_right=hw,
                ai_bound_left=-bound,
                ai_bound_right=bound,
                racing_line=0.0,
                section_id=0xFF,
                flags=0,
                light_scale=(255, 255, 255, 255),
            )
        )
        run = f32.r(run + gaps[i])
    return Path(points, max_spacing(pos), entry=0, exit=0)


def max_spacing(positions):
    n = len(positions)
    return max(f32.dist(positions[i], positions[(i + 1) % n]) for i in range(n))


def start_frame(points, behind=START_BEHIND):
    """The start slot: `behind` units back along the ring from point 0.

    The race lays its own grid from this and puts the lap line `behind` ahead of it
    (`Course::START_LINE_ADVANCE`), so point 0 is the lap line.
    """
    n = len(points)
    remaining = f32.r(behind)
    at, start = 0, points[0].pos
    for _ in range(n):
        prev = (at - 1) % n
        step = f32.dist(points[prev].pos, points[at].pos)
        if step >= remaining:
            t = f32.r(remaining / step)
            start = f32.add_scaled(
                points[at].pos, f32.sub(points[prev].pos, points[at].pos), t
            )
            break
        remaining = f32.r(remaining - step)
        at = prev
        start = points[at].pos
    fw = points[at].tangent
    forward = f32.unit((fw[0], 0.0, fw[2]))
    return Start(start, f32.cross(WORLD_UP, forward), WORLD_UP, forward)


def pad_at(point, box=PAD_BOX):
    """A pad on a spline point: the unit basis (lateral, up, forward) in the rows,
    the size in the box, as the fixture does."""
    up = f32.unit(f32.cross(point.lateral, point.tangent))
    rows = [point.lateral, up, point.tangent]
    to_world = []
    for row in rows:
        to_world += [*row, 0.0]
    to_world += [*point.pos, 1.0]
    return Pad(tuple(to_world), box[0], box[1], 0.0)


def _mesh(vertices, triangles):
    return CollisionMesh(list(vertices), list(triangles), [1.0] * len(vertices))


def ribbon(path, wall_height=WALL_HEIGHT):
    """Floor and wall meshes along `path`, wound as the fixture's are.

    Returns `(floor, walls)`: the floor between the two edges, then the inner
    wall (left edge) and outer wall (right edge) one wall height tall.
    """
    pts, n = path.points, len(path.points)
    h = f32.r(wall_height)
    left, right = [], []
    for q in pts:
        left.append(f32.add_scaled(q.pos, q.lateral, -q.half_width_left))
        right.append(f32.add_scaled(q.pos, q.lateral, q.half_width_right))
    up = [f32.unit(f32.cross(q.lateral, q.tangent)) for q in pts]
    # The fixture lifts wall tops straight up; a slope raises them along `up`.
    left_top = [f32.add_scaled(left[i], up[i], h) for i in range(n)]
    right_top = [f32.add_scaled(right[i], up[i], h) for i in range(n)]

    floor_v, floor_t = [], []
    for i in range(n):
        j = (i + 1) % n
        k = len(floor_v)
        floor_v += [left[i], right[i], left[j], right[j]]
        floor_t += [(k, k + 1, k + 2), (k + 1, k + 3, k + 2)]
    walls_v, walls_t = [], []
    for i in range(n):
        j = (i + 1) % n
        k = len(walls_v)
        walls_v += [left[i], left[j], left_top[i], left_top[j]]
        walls_t += [(k, k + 1, k + 2), (k + 1, k + 3, k + 2)]
    for i in range(n):
        j = (i + 1) % n
        k = len(walls_v)
        walls_v += [right[j], right[i], right_top[j], right_top[i]]
        walls_t += [(k, k + 1, k + 2), (k + 1, k + 3, k + 2)]
    return _mesh(floor_v, floor_t), _mesh(walls_v, walls_t)


POINT_VERSION = SHORT_POINT_VERSION
