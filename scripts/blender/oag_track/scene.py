"""Blender scene <-> track model.

Axis change: the format is right-handed Y-up, Blender right-handed Z-up.
`(x, y, z)_blender = (x, -z, y)_format`. It is a permutation and a negation, so a
number that is exact in `f32` stays exact, a winding is kept, and a cross product
maps to a cross product.

An object is part of the track when it carries the custom property `oag_kind`:

- `floor`, `wall`, `reset`, `mag_floor`, `cage`, `track_wall` (a mesh): collision;
- `spline` (a closed Curve to sample, or a Mesh of baked points);
- `start` (an Empty: +Y forward, +Z up);
- `speedup`, `weapon` (an Empty: X lateral, +Y forward, +Z up);
- `racing_line` (a closed Curve beside a spline: the AI line, hand-editable; see
  `line_offsets`).

Baked records (what the importer makes, and what a re-export passes through) are
written back exactly. Authored records are derived at export (`derive.py`).
"""

import array
import json
import os
import re

import bpy
from mathutils import Matrix, Vector

from . import derive, f32, racing_line, scene_art, trackfile
from .model import (
    COLLISION_KINDS,
    HEADER_WORD,
    HOSTS,
    SHORT_POINT_VERSION,
    CollisionMesh,
    CollisionNode,
    Junction,
    Pad,
    Path,
    SplinePoint,
    Start,
    Track,
)
from .trackfile import MAX_VERTICES, TrackFileError

LINE_KIND = "racing_line"
KINDS = (*COLLISION_KINDS, "spline", LINE_KIND, "start", "speedup", "weapon")
COLLECTION = "OAG Track"
CACHE_TOLERANCE = 1e-6
# How far a racing-line curve point may sit from where the operator put it and still
# count as unmoved (the curve stores `f32`; the format's units are about a metre).
LINE_TOLERANCE = 1e-3


def to_format(v):
    return (v[0], v[2], -v[1])


def to_blender(v):
    return (v[0], -v[2], v[1])


def _natural(name):
    return [int(t) if t.isdigit() else t for t in re.split(r"(\d+)", name)]


def _kind(obj):
    return obj.get("oag_kind")


def _is_identity(m):
    return all(m[r][c] == (1.0 if r == c else 0.0) for r in range(4) for c in range(4))


def _unit_col(m, i):
    v = Vector((m[0][i], m[1][i], m[2][i]))
    if v.length == 0.0:
        raise TrackFileError("an object's axis has no length (zero scale)")
    return v.normalized()


def _flat16(m):
    return [m[r][c] for r in range(4) for c in range(4)]


def _matrix_unmoved(obj, key):
    baseline = obj.get(key)
    if baseline is None or len(baseline) != 16:
        return False
    now = _flat16(obj.matrix_world)
    return all(abs(a - b) <= CACHE_TOLERANCE for a, b in zip(now, baseline))


def _floats(attr, field, width, count):
    buf = array.array("f", bytes(4 * width * count))
    attr.data.foreach_get(field, buf)
    return buf


def _ints(attr, count):
    buf = array.array("i", bytes(4 * count))
    attr.data.foreach_get("value", buf)
    return buf


def _rows(buf, width):
    return [tuple(buf[i : i + width]) for i in range(0, len(buf), width)]


def _transform_point(m, ident, p):
    if ident:
        return p
    v = m @ Vector(p)
    return (f32.r(v[0]), f32.r(v[1]), f32.r(v[2]))


def _transform_dir(m, ident, p):
    if ident:
        return p
    v = (m.to_3x3() @ Vector(p)).normalized()
    return (f32.r(v[0]), f32.r(v[1]), f32.r(v[2]))


def baked_spline(obj, index):
    """A `Path` out of a Mesh object holding the `oag_*` point attributes."""
    me = obj.data
    n = len(me.vertices)
    m = obj.matrix_world
    ident = _is_identity(m)
    need = ("oag_tangent", "oag_down", "oag_lateral", "oag_progress", "oag_half_width")
    for a in need:
        if a not in me.attributes:
            raise TrackFileError(f"{obj.name}: missing point attribute {a}")
    co = _rows(_floats_from_vertices(me, n), 3)
    pos = [to_format(_transform_point(m, ident, p)) for p in co]

    def vec(name):
        return [
            to_format(_transform_dir(m, ident, v))
            for v in _rows(_floats(me.attributes[name], "vector", 3, n), 3)
        ]

    tangent, down, lateral = vec("oag_tangent"), vec("oag_down"), vec("oag_lateral")
    progress = list(_floats(me.attributes["oag_progress"], "value", 1, n))
    half = _rows(_floats(me.attributes["oag_half_width"], "vector", 2, n), 2)
    bound = (
        _rows(_floats(me.attributes["oag_ai_bound"], "vector", 2, n), 2)
        if "oag_ai_bound" in me.attributes
        else [(-b[0] * 0.6, b[0] * 0.6) for b in half]
    )
    line = (
        list(_floats(me.attributes["oag_racing_line"], "value", 1, n))
        if "oag_racing_line" in me.attributes
        else [0.0] * n
    )

    def byte_attr(name, default):
        if name in me.attributes:
            return list(_ints(me.attributes[name], n))
        return [default] * n

    section, flags = byte_attr("oag_section", 0xFF), byte_attr("oag_flags", 0)
    light = [byte_attr(f"oag_light{k}", 255) for k in range(4)]
    ai = None
    if "oag_ai_has" in me.attributes:
        has = byte_attr("oag_ai_has", 0)
        lo, hi, floor = (byte_attr(k, 0) for k in ("oag_ai_lo", "oag_ai_hi", "oag_ai_floor"))
        ai = [(lo[i], hi[i], floor[i]) if has[i] == 1 else None for i in range(n)]
    points = []
    for i in range(n):
        points.append(
            SplinePoint(
                pos[i],
                tangent[i],
                down[i],
                lateral[i],
                progress[i],
                half[i][0],
                half[i][1],
                bound[i][0],
                bound[i][1],
                line[i],
                section[i],
                flags[i],
                tuple(light[k][i] for k in range(4)),
                ai[i] if ai else None,
            )
        )
    spacing = obj.get("oag_max_spacing")
    max_spacing = f32.r(spacing) if spacing is not None else derive.max_spacing(pos)
    return Path(points, max_spacing, _opt(obj.get("oag_entry")), _opt(obj.get("oag_exit")))


def _opt(v):
    return None if v is None or v < 0 else int(v)


def _floats_from_vertices(me, n):
    buf = array.array("f", bytes(12 * n))
    me.vertices.foreach_get("co", buf)
    return buf


def curve_ring(obj, depsgraph):
    """The closed centre line of a Curve object, in format axes."""
    if obj.type != "CURVE":
        raise TrackFileError(f"{obj.name}: a spline is a Curve or a baked point Mesh")
    if not obj.data.splines or not all(s.use_cyclic_u for s in obj.data.splines):
        raise TrackFileError(f"{obj.name}: the curve must be cyclic (a closed loop)")
    if len(obj.data.splines) != 1:
        raise TrackFileError(f"{obj.name}: one curve spline per track path")
    ev = obj.evaluated_get(depsgraph)
    me = ev.to_mesh()
    try:
        m = obj.matrix_world
        ring = [to_format(tuple(m @ v.co)) for v in me.vertices]
    finally:
        ev.to_mesh_clear()
    return ring


def authored_path(obj, depsgraph):
    ring = curve_ring(obj, depsgraph)
    points = derive.resample_closed(ring, obj.oag_points)
    return derive.build_path(points, obj.oag_half_width, obj.oag_ai_fraction)


def path_of(obj, depsgraph, index):
    if obj.type == "MESH":
        return baked_spline(obj, index)
    path = authored_path(obj, depsgraph)
    if obj.get("oag_entry") is not None:
        path.entry = _opt(obj["oag_entry"])
    if obj.get("oag_exit") is not None:
        path.exit = _opt(obj["oag_exit"])
    return path


def _line_objects(objs, count):
    """The racing-line curve of each path index, by `oag_path` (default 0)."""
    out = {}
    for o in _order(objs):
        index = int(o.get("oag_path", 0))
        if not 0 <= index < count:
            raise TrackFileError(f"{o.name}: oag_path {index} names no spline (there are {count})")
        if index in out:
            raise TrackFileError(
                f"{out[index].name} and {o.name} are both the racing line of spline {index}"
            )
        out[index] = o
    return out


def line_offsets(obj, path, depsgraph):
    """The racing line a curve draws, as one offset per point of `path`.

    A curve the add-on made and nobody moved (each point where the stored offset
    puts it) gives the stored offsets back verbatim, so an import or a computed line
    exports with the bits it was made with. Anything else - a point nudged, a curve
    drawn by hand, a Bezier, a different point count - is projected onto each
    spline point's cross-section (`racing_line.project`) and boxed to the corridor.
    """
    ring = curve_ring(obj, depsgraph)
    stored = obj.get("oag_line_offsets")
    pts = path.points
    if stored is not None and len(stored) == len(ring) == len(pts):
        stored = [float(v) for v in stored]
        if all(
            all(
                abs(r[c] - (p.pos[c] + o * p.lateral[c])) <= LINE_TOLERANCE
                for c in range(3)
            )
            for r, p, o in zip(ring, pts, stored)
        ):
            return stored
    try:
        return racing_line.project(pts, ring)
    except ValueError as e:
        raise TrackFileError(f"{obj.name}: {e}") from None


def line_for(spline_obj, path, line_obj, depsgraph):
    """The offsets to write for one spline, or `None` to keep what `path` holds.

    A racing-line curve wins; without one an authored (Curve) spline gets the
    computed line unless its `oag_auto_racing_line` is off; a baked Mesh spline
    keeps the records it was imported with.
    """
    if line_obj is not None:
        return line_offsets(line_obj, path, depsgraph)
    if spline_obj.type == "CURVE" and spline_obj.oag_auto_racing_line:
        return racing_line.solve(path.points)
    return None


def _order(objects, key="oag_path"):
    return sorted(
        objects,
        key=lambda o: (0, o[key], _natural(o.name)) if key in o else (1, 0, _natural(o.name)),
    )


def _start(objs, path):
    if objs:
        obj = _order(objs, "oag_index")[0]
        if len(objs) > 1:
            raise TrackFileError("more than one object is tagged start")
        cache = obj.get("oag_frame")
        if cache is not None and len(cache) == 12 and _matrix_unmoved(obj, "oag_matrix_at"):
            c = [float(v) for v in cache]
            return Start(tuple(c[0:3]), tuple(c[3:6]), tuple(c[6:9]), tuple(c[9:12]))
        m = obj.matrix_world
        forward = f32.unit(tuple(f32.r(v) for v in to_format(_unit_col(m, 1))))
        up = f32.unit(tuple(f32.r(v) for v in to_format(_unit_col(m, 2))))
        pos = tuple(f32.r(v) for v in to_format((m[0][3], m[1][3], m[2][3])))
        return Start(pos, f32.cross(up, forward), up, forward)
    if path is None:
        raise TrackFileError("no spline to place the start on")
    return derive.start_frame(path.points)


def _pad(obj):
    cache = obj.get("oag_to_world")
    if cache is not None and len(cache) == 16 and _matrix_unmoved(obj, "oag_matrix_at"):
        to_world = tuple(float(v) for v in cache)
    else:
        m = obj.matrix_world
        rows = [to_format(_unit_col(m, 0)), to_format(_unit_col(m, 2)), to_format(_unit_col(m, 1))]
        to_world = []
        for row in rows:
            to_world += [f32.r(row[0]), f32.r(row[1]), f32.r(row[2]), 0.0]
        t = to_format((m[0][3], m[1][3], m[2][3]))
        to_world += [f32.r(t[0]), f32.r(t[1]), f32.r(t[2]), 1.0]
        to_world = tuple(to_world)
    return Pad(
        to_world,
        tuple(f32.r(v) for v in obj.oag_min),
        tuple(f32.r(v) for v in obj.oag_max),
        f32.r(obj.oag_disabled),
    )


def _chunks(vertices, triangles, scalars):
    """Whole triangles grouped so no chunk holds more than the u16 index range."""
    if len(vertices) <= MAX_VERTICES:
        return [CollisionMesh(vertices, triangles, scalars)]
    out, remap, verts, tris, sc = [], {}, [], [], []
    for t in triangles:
        new = [v for v in set(t) if v not in remap]
        if len(verts) + len(new) > MAX_VERTICES:
            out.append(CollisionMesh(verts, tris, sc))
            remap, verts, tris, sc = {}, [], [], []
        for v in t:
            if v not in remap:
                remap[v] = len(verts)
                verts.append(vertices[v])
                sc.append(scalars[v])
        tris.append(tuple(remap[v] for v in t))
    out.append(CollisionMesh(verts, tris, sc))
    return out


def collision_meshes(obj, depsgraph):
    ev = obj.evaluated_get(depsgraph)
    me = ev.to_mesh()
    try:
        n = len(me.vertices)
        if n == 0:
            raise TrackFileError(f"{obj.name}: the mesh has no vertices")
        me.calc_loop_triangles()
        m = obj.matrix_world
        ident = _is_identity(m)
        co = [
            to_format(_transform_point(m, ident, p))
            for p in _rows(_floats_from_vertices(me, n), 3)
        ]
        nt = len(me.loop_triangles)
        buf = array.array("i", bytes(12 * nt))
        me.loop_triangles.foreach_get("vertices", buf)
        tris = [tuple(buf[i : i + 3]) for i in range(0, 3 * nt, 3)]
        if "oag_scalar" in me.attributes and me.attributes["oag_scalar"].domain == "POINT":
            scalars = list(_floats(me.attributes["oag_scalar"], "value", 1, n))
        else:
            scalars = [1.0] * n
        if not tris:
            raise TrackFileError(f"{obj.name}: the mesh has no faces")
        return _chunks(co, tris, scalars)
    finally:
        ev.to_mesh_clear()


def _junctions(scene, paths):
    raw = scene.get("oag_junctions")
    if raw:
        return [
            Junction(tuple(_opt(v) for v in j["prev"]), tuple(_opt(v) for v in j["next"]))
            for j in json.loads(raw)
        ]
    if len(paths) == 1 and paths[0].entry == 0 and paths[0].exit == 0:
        return [Junction((0, None), (0, None))]
    raise TrackFileError(
        "more than one path (or a path without entry 0 / exit 0) needs the scene "
        "property oag_junctions"
    )


def collect(context):
    """The `Track` the scene authors. Raises `TrackFileError`/`AuthoringError` by name."""
    scene = context.scene
    context.view_layer.update()
    depsgraph = context.evaluated_depsgraph_get()
    tagged = [o for o in scene.objects if _kind(o) is not None]
    for o in tagged:
        if _kind(o) not in KINDS:
            raise TrackFileError(f"{o.name}: unknown oag_kind {_kind(o)!r} (one of {', '.join(KINDS)})")
    by = lambda kinds: [o for o in tagged if _kind(o) in kinds]

    splines = _order(by(("spline",)))
    if not splines:
        raise TrackFileError("no object is tagged spline")
    paths = [path_of(o, depsgraph, i) for i, o in enumerate(splines)]
    lines = _line_objects(by((LINE_KIND,)), len(splines))
    for i, (o, path) in enumerate(zip(splines, paths)):
        offsets = line_for(o, path, lines.get(i), depsgraph)
        if offsets is not None:
            for p, offset in zip(path.points, offsets):
                p.racing_line = offset
    art = scene_art.collect(context, depsgraph)
    for o, path in zip(splines, paths):
        if o.type == "CURVE" and o.oag_cover_light < 255:
            sun = art.sun.direction if art is not None and art.sun is not None else None
            rows = scene_art.cover_light_scale(context, depsgraph, path.points, o.oag_cover_light, sun)
            for p, light in zip(path.points, rows):
                p.light_scale = light
    start = _start(by(("start",)), paths[0])
    pads = {
        k: [_pad(o) for o in _order(by((k,)), "oag_index")] for k in ("speedup", "weapon")
    }

    groups, solo = {}, []
    for o in by(COLLISION_KINDS):
        if o.type != "MESH":
            raise TrackFileError(f"{o.name}: a {_kind(o)} object must be a Mesh")
        if "oag_node" in o:
            groups.setdefault(int(o["oag_node"]), []).append(o)
        else:
            solo.append([o])
    ordered = [groups[k] for k in sorted(groups)] + sorted(solo, key=lambda g: _natural(g[0].name))
    nodes = []
    for objs in ordered:
        objs = sorted(objs, key=lambda o: (o.get("oag_mesh", 0), _natural(o.name)))
        kinds = {_kind(o) for o in objs}
        if len(kinds) != 1:
            raise TrackFileError(f"objects of one oag_node disagree on oag_kind: {sorted(kinds)}")
        meshes = []
        for o in objs:
            meshes += collision_meshes(o, depsgraph)
        head = objs[0]
        version = head.get("oag_version")
        nodes.append(
            CollisionNode(
                kinds.pop(),
                meshes,
                head.get("oag_name"),
                HEADER_WORD if version is None else int(version),
            )
        )
    if not nodes:
        raise TrackFileError("no collision mesh is tagged (floor, wall, ...)")

    notes = [n for n in scene.oag_track_notes.split("\n") if n]
    track_id = scene.oag_track_id or re.sub(r"[^a-z0-9]+", "-", scene.name.lower()).strip("-")
    return Track(
        track_id,
        scene.oag_track_title or track_id,
        "" if scene.oag_track_host == "none" else scene.oag_track_host,
        [n.strip() for n in scene.oag_track_titles.split(",") if n.strip()],
        int(scene.get("oag_point_version", SHORT_POINT_VERSION)),
        paths,
        _junctions(scene, paths),
        start,
        nodes,
        pads["speedup"],
        pads["weapon"],
        notes,
        art,
    )


def write_track(context, path):
    """Collects the scene and writes `path` and its glb, with the own images beside them."""
    track = collect(context)
    trackfile.write(track, path)
    if track.art is not None:
        scene_art.copy_images(track.art, context.scene, os.path.dirname(os.path.abspath(path)))
    return track


def _collection(scene):
    col = bpy.data.collections.get(COLLECTION)
    if col is None:
        col = bpy.data.collections.new(COLLECTION)
        scene.collection.children.link(col)
    elif col.name not in scene.collection.children:
        scene.collection.children.link(col)
    return col


def _link(scene, obj):
    _collection(scene).objects.link(obj)
    return obj


def _attr(me, name, ty, field, flat):
    a = me.attributes.new(name, ty, "POINT")
    a.data.foreach_set(field, flat)


def _vec_flat(rows):
    return [c for r in rows for c in to_blender(r)]


def make_spline_object(scene, path, index):
    pts = path.points
    n = len(pts)
    me = bpy.data.meshes.new(f"oag/spline/{index}")
    me.vertices.add(n)
    me.vertices.foreach_set("co", _vec_flat([p.pos for p in pts]))
    me.edges.add(n)
    me.edges.foreach_set("vertices", [v for i in range(n) for v in (i, (i + 1) % n)])
    me.update()
    _attr(me, "oag_tangent", "FLOAT_VECTOR", "vector", _vec_flat([p.tangent for p in pts]))
    _attr(me, "oag_down", "FLOAT_VECTOR", "vector", _vec_flat([p.down for p in pts]))
    _attr(me, "oag_lateral", "FLOAT_VECTOR", "vector", _vec_flat([p.lateral for p in pts]))
    _attr(me, "oag_progress", "FLOAT", "value", [p.progress for p in pts])
    _attr(me, "oag_racing_line", "FLOAT", "value", [p.racing_line for p in pts])
    _attr(me, "oag_half_width", "FLOAT2", "vector", [c for p in pts for c in (p.half_width_left, p.half_width_right)])
    _attr(me, "oag_ai_bound", "FLOAT2", "vector", [c for p in pts for c in (p.ai_bound_left, p.ai_bound_right)])
    _attr(me, "oag_section", "INT", "value", [p.section_id for p in pts])
    _attr(me, "oag_flags", "INT", "value", [p.flags for p in pts])
    for k in range(4):
        _attr(me, f"oag_light{k}", "INT", "value", [p.light_scale[k] for p in pts])
    if any(p.ai_bytes is not None for p in pts):
        z = (0, 0, 0)
        _attr(me, "oag_ai_has", "INT", "value", [0 if p.ai_bytes is None else 1 for p in pts])
        for k, name in enumerate(("oag_ai_lo", "oag_ai_hi", "oag_ai_floor")):
            _attr(me, name, "INT", "value", [(p.ai_bytes or z)[k] for p in pts])
    obj = bpy.data.objects.new(f"oag/spline/{index}", me)
    obj["oag_kind"] = "spline"
    obj["oag_path"] = index
    obj["oag_max_spacing"] = path.max_spacing
    obj["oag_entry"] = -1 if path.entry is None else path.entry
    obj["oag_exit"] = -1 if path.exit is None else path.exit
    return _link(scene, obj)


def make_line_object(scene, path, index, offsets):
    """A closed poly curve through each spline point moved by its offset, tagged `racing_line`.

    The offsets are kept on the object (`oag_line_offsets`): while no point moves,
    export writes them back exactly (`line_offsets`).
    """
    n = len(path.points)
    curve = bpy.data.curves.new(f"oag/racing_line/{index}", "CURVE")
    curve.dimensions = "3D"
    sp = curve.splines.new("POLY")
    sp.points.add(n - 1)
    flat = []
    for p, o in zip(path.points, offsets):
        q = to_blender(tuple(p.pos[c] + o * p.lateral[c] for c in range(3)))
        flat += [q[0], q[1], q[2], 1.0]
    sp.points.foreach_set("co", flat)
    sp.use_cyclic_u = True
    obj = bpy.data.objects.new(f"oag/racing_line/{index}", curve)
    obj["oag_kind"] = LINE_KIND
    obj["oag_path"] = index
    obj["oag_line_offsets"] = [float(o) for o in offsets]
    obj.hide_render = True
    obj.color = (1.0, 0.45, 0.0, 1.0)
    return _link(scene, obj)


def compute_racing_line(context, spline_obj):
    """(Re)make the editable racing-line curve of `spline_obj` from the solver."""
    scene = context.scene
    splines = _order([o for o in scene.objects if _kind(o) == "spline"])
    index = splines.index(spline_obj)
    path = path_of(spline_obj, context.evaluated_depsgraph_get(), index)
    offsets = racing_line.solve(path.points)
    for old in [o for o in scene.objects if _kind(o) == LINE_KIND and int(o.get("oag_path", 0)) == index]:
        data = old.data
        bpy.data.objects.remove(old, do_unlink=True)
        if data.users == 0:
            bpy.data.curves.remove(data)
    return make_line_object(scene, path, index, offsets)


def _matrix_from(columns):
    m = Matrix.Identity(4)
    for i, c in enumerate(columns):
        for r in range(3):
            m[r][i] = c[r]
    return m


def _place(context, obj, matrix, key, cache):
    obj.matrix_world = matrix
    context.view_layer.update()
    obj[key] = cache
    obj["oag_matrix_at"] = _flat16(obj.matrix_world)


def make_pad_object(context, pad, kind, index):
    r = [pad.to_world[i : i + 4] for i in range(0, 16, 4)]
    cols = [to_blender(r[0][:3]), to_blender(r[2][:3]), to_blender(r[1][:3]), to_blender(r[3][:3])]
    obj = bpy.data.objects.new(f"oag/pads/{kind}/{index}", None)
    obj.empty_display_type = "CUBE"
    obj.empty_display_size = max(1.0, *(abs(v) for v in (*pad.min, *pad.max))) / 2
    obj["oag_kind"] = kind
    obj["oag_index"] = index
    obj.oag_min, obj.oag_max, obj.oag_disabled = pad.min, pad.max, pad.disabled
    _link(context.scene, obj)
    _place(context, obj, _matrix_from(cols), "oag_to_world", list(pad.to_world))
    return obj


def make_start_object(context, start):
    cols = [
        tuple(-c for c in to_blender(start.left)),
        to_blender(start.forward),
        to_blender(start.up),
        to_blender(start.position),
    ]
    obj = bpy.data.objects.new("oag/start", None)
    obj.empty_display_type = "SINGLE_ARROW"
    obj.empty_display_size = 40.0
    obj["oag_kind"] = "start"
    _link(context.scene, obj)
    cache = [*start.position, *start.left, *start.up, *start.forward]
    _place(context, obj, _matrix_from(cols), "oag_frame", cache)
    return obj


def make_collision_object(scene, kind, mesh, node, n, m, multi):
    name = f"oag/collision/{n}" + (f".{m}" if multi else "")
    me = bpy.data.meshes.new(name)
    me.from_pydata([to_blender(v) for v in mesh.vertices], [], [tuple(t) for t in mesh.triangles])
    _attr(me, "oag_scalar", "FLOAT", "value", list(mesh.scalars))
    obj = bpy.data.objects.new(name, me)
    obj["oag_kind"] = kind
    obj["oag_node"] = n
    obj["oag_mesh"] = m
    if node.name is not None:
        obj["oag_name"] = node.name
    if node.version != HEADER_WORD:
        obj["oag_version"] = node.version if node.version < 2**31 else float(node.version)
    return _link(scene, obj)


def populate(context, track, manifest_dir="."):
    """Creates the objects and scene properties for `track` (the importer)."""
    scene = context.scene
    scene.oag_track_id = track.id
    scene.oag_track_title = track.title
    for name in [track.host] * bool(track.host) + list(track.titles):
        if name not in HOSTS:
            raise TrackFileError(f"{name!r} is not a title this add-on knows: {', '.join(HOSTS)}")
    scene.oag_track_host = track.host or "none"
    scene.oag_track_titles = ", ".join(track.titles)
    scene.oag_track_notes = "\n".join(track.notes)
    scene["oag_point_version"] = track.point_version
    scene["oag_junctions"] = json.dumps(
        [
            {"prev": [-1 if v is None else v for v in j.prev], "next": [-1 if v is None else v for v in j.next]}
            for j in track.junctions
        ]
    )
    for i, path in enumerate(track.paths):
        make_spline_object(scene, path, i)
        if any(p.racing_line != 0.0 for p in path.points):
            make_line_object(scene, path, i, [p.racing_line for p in path.points])
    make_start_object(context, track.start)
    for kind, pads in (("speedup", track.speedup_pads), ("weapon", track.weapon_pads)):
        for i, pad in enumerate(pads):
            make_pad_object(context, pad, kind, i)
    for n, node in enumerate(track.nodes):
        for m, mesh in enumerate(node.meshes):
            make_collision_object(scene, node.kind, mesh, node, n, m, len(node.meshes) > 1)
    if track.art is not None:
        scene_art.populate(context, track.art, manifest_dir)
    context.view_layer.update()


def build_ribbon(context, spline_obj):
    """Floor and wall meshes along a spline object, as tagged collision objects."""
    depsgraph = context.evaluated_depsgraph_get()
    path = path_of(spline_obj, depsgraph, 0)
    floor, walls = derive.ribbon(path, spline_obj.oag_wall_height)
    made = []
    node = 0
    for kind, mesh in (("floor", floor), ("wall", walls)):
        cn = CollisionNode(kind, [mesh], kind)
        obj = make_collision_object(context.scene, kind, mesh, cn, node, 0, False)
        obj.name = kind
        obj["oag_node"] = node
        made.append(obj)
        node += 1
    return made


def start_from(context, spline_obj):
    path = path_of(spline_obj, context.evaluated_depsgraph_get(), 0)
    return derive.start_frame(path.points)


def add_pad_at(context, spline_obj, kind, near):
    """A pad on the spline point nearest `near` (a Blender-space Vector)."""
    path = path_of(spline_obj, context.evaluated_depsgraph_get(), 0)
    target = to_format(tuple(near))
    best = min(path.points, key=lambda p: sum((p.pos[i] - target[i]) ** 2 for i in range(3)))
    pad = derive.pad_at(best)
    index = sum(1 for o in context.scene.objects if _kind(o) == kind)
    obj = make_pad_object(context, pad, kind, index)
    return obj
