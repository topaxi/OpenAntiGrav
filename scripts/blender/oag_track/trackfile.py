"""Reads and writes the track file (docs/formats/track-file.md) with no Blender.

The glb is built here, byte by byte, rather than through Blender's glTF
exporter: that exporter re-splits vertices, converts attribute types and cannot
emit the `u8` VEC4 or `MAT4` accessors the format needs. Every number goes out
as a little-endian binary accessor, so an `f32` held in the model reaches the
file bit for bit.

Coordinates here are the format's: right-handed, Y up. The Blender side
(`scene.py`) does the axis change.
"""

import json
import math
import os
import struct
import tomllib

from . import artfile, f32, motionfile
from .model import (
    COLLISION_KINDS,
    HEADER_WORD,
    CollisionMesh,
    CollisionNode,
    Junction,
    Pad,
    Path,
    SplinePoint,
    Start,
    Track,
    Trigger,
)

SCHEMA_LOCATION = "https://oag.topaxi.com/mods/schemas/v1/track.tosd"
SCHEMA_VERSION = "1.0.0"

FLOAT, UBYTE, USHORT = 5126, 5121, 5123
ARRAY_BUFFER, ELEMENT_ARRAY_BUFFER = 34962, 34963
POINTS, TRIANGLES = 0, 4
MAX_VERTICES = 65535

START = "oag/start"
SPEEDUP = "oag/pads/speedup"
WEAPON = "oag/pads/weapon"
TRIGGERS = "oag/triggers"
MAX_TRIGGERS = 64
MAX_PROPS = 1024
SPLINE = "oag/spline/"

_WIDTH = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
_COMPONENT = {FLOAT: ("f", 4), UBYTE: ("B", 1), USHORT: ("H", 2), artfile.UINT: ("I", 4)}


class TrackFileError(Exception):
    """A track the format cannot hold, or a file this reader refuses."""


def prop_key(name):
    """FNV-1a 32 of the name: the key a mod reads a property by (`oag_ext_abi::track`)."""
    h = 0x811C9DC5
    for b in name.encode("utf-8"):
        h = ((h ^ b) * 0x01000193) & 0xFFFFFFFF
    return h


def prop_value(what, key, v):
    """A property's value as the f32 the race hands a mod: numbers and bools only."""
    if isinstance(v, bool):
        return 1.0 if v else 0.0
    if isinstance(v, (int, float)):
        return f32.r(float(v))
    raise TrackFileError(f"{what}: property {key!r} is a {type(v).__name__}; properties are numbers and bools")


def check_props(what, props):
    seen = {}
    for key, v in props.items():
        if not key:
            raise TrackFileError(f"{what}: a property has an empty name")
        if not math.isfinite(v):
            raise TrackFileError(f"{what}: property {key!r} is not finite")
        h = prop_key(key)
        if h in seen:
            raise TrackFileError(f"{what}: properties {seen[h]!r} and {key!r} hash alike; rename one")
        seen[h] = key


def _check_all(track):
    check_props("props", track.props)
    for what, pads in (("speed pads", track.speedup_pads), ("weapon pads", track.weapon_pads)):
        for i, pad in enumerate(pads):
            check_props(f"{what} {i}", pad.props)
    if len(track.triggers) > MAX_TRIGGERS:
        raise TrackFileError(f"{len(track.triggers)} triggers, a mod reads at most {MAX_TRIGGERS}")
    for t in track.triggers:
        check_props(f"trigger {t.name}", t.props)
    total = len(track.props) + sum(
        len(p.props) for p in (*track.speedup_pads, *track.weapon_pads, *track.triggers)
    )
    if total > MAX_PROPS:
        raise TrackFileError(f"{total} properties, a mod reads at most {MAX_PROPS}")


def _props_json(props):
    return {k: props[k] for k in sorted(props)}


def _flat(rows):
    for row in rows:
        yield from (row if isinstance(row, (tuple, list)) else (row,))


class _Bin:
    def __init__(self):
        self.data = bytearray()
        self.views = []
        self.accessors = []

    def add(self, payload, component, ty, count, target, minmax=None):
        while len(self.data) % 4:
            self.data.append(0)
        self.views.append(
            {
                "buffer": 0,
                "byteOffset": len(self.data),
                "byteLength": len(payload),
                "target": target,
            }
        )
        self.data += payload
        acc = {
            "bufferView": len(self.views) - 1,
            "componentType": component,
            "count": count,
            "type": ty,
        }
        if minmax:
            acc["min"], acc["max"] = minmax
        self.accessors.append(acc)
        return len(self.accessors) - 1

    def floats(self, rows, ty):
        flat = list(_flat(rows))
        return self.add(
            struct.pack(f"<{len(flat)}f", *flat), FLOAT, ty, len(rows), ARRAY_BUFFER
        )

    def positions(self, rows):
        lo = [min(r[i] for r in rows) for i in range(3)]
        hi = [max(r[i] for r in rows) for i in range(3)]
        flat = list(_flat(rows))
        return self.add(
            struct.pack(f"<{len(flat)}f", *flat),
            FLOAT,
            "VEC3",
            len(rows),
            ARRAY_BUFFER,
            (lo, hi),
        )

    def bytes4(self, rows):
        flat = list(_flat(rows))
        return self.add(
            struct.pack(f"<{len(flat)}B", *flat), UBYTE, "VEC4", len(rows), ARRAY_BUFFER
        )


def _finite(record, values):
    for v in values:
        if v != v or v in (float("inf"), float("-inf")):
            raise TrackFileError(f"{record}: a non-finite number")


def _byte(record, field, v):
    if not (isinstance(v, int) and 0 <= v <= 255):
        raise TrackFileError(f"{record}: {field} {v!r} is not a byte")


def write_glb(track):
    """The glb bytes for `track`, in the node order `oag_trackfile` writes."""
    b = _Bin()
    nodes, meshes = [], []

    def push(name, prims, extras=None):
        node = {"name": name}
        if prims:
            meshes.append({"name": name, "primitives": prims})
            node["mesh"] = len(meshes) - 1
        if extras is not None:
            node["extras"] = extras
        nodes.append(node)
        return len(nodes) - 1

    s = track.start
    _finite(START, [*s.position, *s.left, *s.up, *s.forward])
    push(
        START,
        [
            {
                "attributes": {
                    "POSITION": b.positions([s.position]),
                    "_OAG_LEFT": b.floats([s.left], "VEC3"),
                    "_OAG_UP": b.floats([s.up], "VEC3"),
                    "_OAG_FORWARD": b.floats([s.forward], "VEC3"),
                },
                "mode": POINTS,
            }
        ],
    )

    for i, path in enumerate(track.paths):
        if not path.points:
            continue
        name = f"{SPLINE}{i}"
        p = path.points
        for q in p:
            _finite(
                name,
                [
                    *q.pos,
                    *q.tangent,
                    *q.down,
                    *q.lateral,
                    q.progress,
                    q.half_width_left,
                    q.half_width_right,
                    q.ai_bound_left,
                    q.ai_bound_right,
                    q.racing_line,
                ],
            )
            _byte(name, "section", q.section_id)
            _byte(name, "flags", q.flags)
            for v in q.light_scale:
                _byte(name, "light scale", v)
        attrs = {
            "POSITION": b.positions([q.pos for q in p]),
            "_OAG_TANGENT": b.floats([q.tangent for q in p], "VEC3"),
            "_OAG_DOWN": b.floats([q.down for q in p], "VEC3"),
            "_OAG_LATERAL": b.floats([q.lateral for q in p], "VEC3"),
            "_OAG_PROGRESS": b.floats([(q.progress,) for q in p], "SCALAR"),
            "_OAG_HALF_WIDTH": b.floats(
                [(q.half_width_left, q.half_width_right) for q in p], "VEC2"
            ),
            "_OAG_AI_BOUND": b.floats(
                [(q.ai_bound_left, q.ai_bound_right) for q in p], "VEC2"
            ),
            "_OAG_RACING_LINE": b.floats([(q.racing_line,) for q in p], "SCALAR"),
            "_OAG_SECTION_FLAGS": b.bytes4([(q.section_id, q.flags, 0, 0) for q in p]),
            "_OAG_LIGHT_SCALE": b.bytes4([q.light_scale for q in p]),
        }
        if any(q.ai_bytes is not None for q in p):
            rows = []
            for q in p:
                if q.ai_bytes is None:
                    rows.append((0, 0, 0, 0))
                else:
                    for v in q.ai_bytes:
                        _byte(name, "AI byte", v)
                    rows.append((*q.ai_bytes, 1))
            attrs["_OAG_AI_BYTES"] = b.bytes4(rows)
        if any(q.cage is not None and any(q.cage) for q in p):
            raw = [tuple(v & 0xFF for v in (q.cage or (0,) * 8)) for q in p]
            attrs["_OAG_CAGE_A"] = b.bytes4([r[:4] for r in raw])
            attrs["_OAG_CAGE_B"] = b.bytes4([r[4:] for r in raw])
        push(name, [{"attributes": attrs, "mode": POINTS}])

    for name, pads in ((SPEEDUP, track.speedup_pads), (WEAPON, track.weapon_pads)):
        if not pads:
            continue
        for pad in pads:
            _finite(name, [*pad.to_world, *pad.min, *pad.max, pad.disabled])
        centre = [(q.to_world[12], q.to_world[13], q.to_world[14]) for q in pads]
        attrs = {
            "POSITION": b.positions(centre),
            "_OAG_TO_WORLD": b.floats([q.to_world for q in pads], "MAT4"),
            "_OAG_MIN": b.floats([q.min for q in pads], "VEC3"),
            "_OAG_MAX": b.floats([q.max for q in pads], "VEC3"),
            "_OAG_DISABLED": b.floats([(q.disabled,) for q in pads], "SCALAR"),
        }
        extras = None
        if any(q.props for q in pads):
            extras = {"oag_props": [_props_json(q.props) for q in pads]}
        push(name, [{"attributes": attrs, "mode": POINTS}], extras)

    if track.triggers:
        t = track.triggers
        for q in t:
            _finite(f"{TRIGGERS} {q.name}", [*q.to_world, *q.min, *q.max])
        attrs = {
            "POSITION": b.positions([(q.to_world[12], q.to_world[13], q.to_world[14]) for q in t]),
            "_OAG_TO_WORLD": b.floats([q.to_world for q in t], "MAT4"),
            "_OAG_MIN": b.floats([q.min for q in t], "VEC3"),
            "_OAG_MAX": b.floats([q.max for q in t], "VEC3"),
        }
        extras = {"oag_names": [q.name for q in t]}
        if any(q.props for q in t):
            extras["oag_props"] = [_props_json(q.props) for q in t]
        push(TRIGGERS, [{"attributes": attrs, "mode": POINTS}], extras)

    for n, node in enumerate(track.nodes):
        name = f"oag/collision/{n}"
        if node.kind not in COLLISION_KINDS:
            raise TrackFileError(f"{name}: unknown oag_kind {node.kind!r}")
        prims = []
        for m, mesh in enumerate(node.meshes):
            what = f"{name} mesh {m}"
            if not mesh.triangles or not mesh.vertices:
                raise TrackFileError(f"{what} is empty")
            if len(mesh.vertices) > MAX_VERTICES:
                raise TrackFileError(f"{what}: more than {MAX_VERTICES} vertices")
            if len(mesh.scalars) != len(mesh.vertices):
                raise TrackFileError(f"{what}: one scalar per vertex is required")
            if any(i >= len(mesh.vertices) for t in mesh.triangles for i in t):
                raise TrackFileError(f"{what}: a triangle index is out of range")
            _finite(what, [*_flat(mesh.vertices), *mesh.scalars])
            idx = list(_flat(mesh.triangles))
            attrs = {
                "POSITION": b.positions(mesh.vertices),
                "_OAG_SCALAR": b.floats([(v,) for v in mesh.scalars], "SCALAR"),
            }
            indices = b.add(
                struct.pack(f"<{len(idx)}H", *idx),
                USHORT,
                "SCALAR",
                len(idx),
                ELEMENT_ARRAY_BUFFER,
            )
            prims.append({"attributes": attrs, "indices": indices, "mode": TRIANGLES})
        extras = {"oag_kind": node.kind, "oag_version": node.version}
        if node.name is not None:
            extras["oag_name"] = node.name
        push(name, prims, extras)

    animations = []
    if track.art is not None:
        try:
            mesh_nodes = artfile.write_nodes(b, track.art, push)
            animations = motionfile.write(b, track.art, nodes, mesh_nodes, push)
        except (artfile.ArtError, motionfile.MotionError) as e:
            raise TrackFileError(str(e)) from e

    while len(b.data) % 4:
        b.data.append(0)
    doc = {
        "asset": {"version": "2.0", "generator": "oag-track blender add-on 1.0"},
        "scene": 0,
        "scenes": [{"nodes": list(range(len(nodes)))}],
        "nodes": nodes,
        "meshes": meshes,
        "buffers": [{"byteLength": len(b.data)}],
        "bufferViews": b.views,
        "accessors": b.accessors,
    }
    if animations:
        doc["animations"] = animations
    text = json.dumps(doc, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    total = 12 + 8 + len(text) + 8 + len(b.data)
    return (
        b"glTF"
        + struct.pack("<II", 2, total)
        + struct.pack("<I", len(text))
        + b"JSON"
        + text
        + struct.pack("<I", len(b.data))
        + b"BIN\0"
        + bytes(b.data)
    )


def _code(v):
    return -1 if v is None else v


def _quote(s):
    return json.dumps(s, ensure_ascii=False)


def _key(s):
    return s if s and all(c.isascii() and (c.isalnum() or c in "_-") for c in s) else _quote(s)


def write_manifest(track, glb_name):
    """The `.track.toml` text. Integers and strings only, but for `[props]`: an f32
    written as the shortest decimal of its exact double, which reads back the same."""
    if not track.id:
        raise TrackFileError("the track has no id")
    if not track.host and track.art is not None and track.art.borrows_from_a_disc():
        raise TrackFileError(
            "the art borrows from a disc (a host texture or a sky): set Runs on to the title it borrows from"
        )
    lines = [
        "[toml-schema]",
        f"location = {_quote(SCHEMA_LOCATION)}",
        f"version = {_quote(SCHEMA_VERSION)}",
        "",
        "[track]",
        f"id = {_quote(track.id)}",
        f"title = {_quote(track.title)}",
    ]
    if track.host:
        lines.append(f"host = {_quote(track.host)}")
    if track.titles:
        lines.append("titles = [" + ", ".join(_quote(n) for n in track.titles) + "]")
    lines += [
        f"glb = {_quote(glb_name)}",
    ]
    if track.notes:
        lines.append("notes = [" + ", ".join(_quote(n) for n in track.notes) + "]")
    lines += ["", "[spline]", f"point_version = {track.point_version}"]
    for p in track.paths:
        lines += [
            "",
            "[[spline.path]]",
            f"points = {len(p.points)}",
            f"entry = {_code(p.entry)}",
            f"exit = {_code(p.exit)}",
            f"max_spacing_bits = {f32.bits(p.max_spacing)}",
        ]
    for j in track.junctions:
        lines += [
            "",
            "[[spline.junction]]",
            f"prev = [{_code(j.prev[0])}, {_code(j.prev[1])}]",
            f"next = [{_code(j.next[0])}, {_code(j.next[1])}]",
        ]
    lines += [
        "",
        "[counts]",
        f"collision_nodes = {len(track.nodes)}",
        f"speedup_pads = {len(track.speedup_pads)}",
        f"weapon_pads = {len(track.weapon_pads)}",
    ]
    if track.art is not None:
        lines.append(f"art_meshes = {len(track.art.meshes)}")
    if track.triggers:
        lines.append(f"triggers = {len(track.triggers)}")
    if track.props:
        lines += ["", "[props]"]
        lines += [f"{_key(k)} = {float(track.props[k])!r}" for k in sorted(track.props)]
    if track.art is not None:
        lines += ["", *artfile.manifest_lines(track.art)]
    else:
        lines.append("")
    return "\n".join(lines)


def write(track, manifest_path):
    """Writes `<stem>.track.toml` and `<stem>.glb` beside it; returns the glb path."""
    manifest_path = os.fspath(manifest_path)
    if not manifest_path.endswith(".track.toml"):
        raise TrackFileError("the manifest name must end in .track.toml")
    glb_path = manifest_path[: -len(".track.toml")] + ".glb"
    _check_all(track)
    glb = write_glb(track)
    text = write_manifest(track, os.path.basename(glb_path))
    with open(glb_path, "wb") as fh:
        fh.write(glb)
    with open(manifest_path, "w", encoding="utf-8") as fh:
        fh.write(text)
    return glb_path


class _Glb:
    def __init__(self, data):
        if data[:4] != b"glTF":
            raise TrackFileError("not a glb")
        pos, self.doc, self.blob = 12, None, b""
        while pos + 8 <= len(data):
            size, kind = struct.unpack_from("<I4s", data, pos)
            body = data[pos + 8 : pos + 8 + size]
            if kind == b"JSON":
                self.doc = json.loads(body)
            elif kind == b"BIN\0":
                self.blob = body
            pos += 8 + size
        if self.doc is None:
            raise TrackFileError("no JSON chunk")

    def accessor(self, index, expect_component, expect_type):
        a = self.doc["accessors"][index]
        if a["componentType"] != expect_component or a["type"] != expect_type:
            raise TrackFileError(f"accessor {index} is not {expect_type}")
        view = self.doc["bufferViews"][a["bufferView"]]
        if view.get("byteStride"):
            raise TrackFileError(f"accessor {index} is strided")
        fmt, _ = _COMPONENT[a["componentType"]]
        width = _WIDTH[a["type"]]
        count = a["count"]
        start = view.get("byteOffset", 0) + a.get("byteOffset", 0)
        flat = struct.unpack_from(f"<{count * width}{fmt}", self.blob, start)
        if width == 1:
            return list(flat)
        return [tuple(flat[i * width : (i + 1) * width]) for i in range(count)]

    def prim(self, node, record):
        prims = self.doc["meshes"][node["mesh"]]["primitives"]
        if len(prims) != 1:
            raise TrackFileError(f"{record}: expected exactly one primitive")
        return prims[0]


def _read_start(g, node):
    a = g.prim(node, START)["attributes"]

    def one(key):
        return g.accessor(a[key], FLOAT, "VEC3")[0]

    return Start(
        one("POSITION"), one("_OAG_LEFT"), one("_OAG_UP"), one("_OAG_FORWARD")
    )


def _read_spline(g, node, name):
    a = g.prim(node, name)["attributes"]

    def v3(k):
        return g.accessor(a[k], FLOAT, "VEC3")

    def v2(k):
        return g.accessor(a[k], FLOAT, "VEC2")

    def s1(k):
        return g.accessor(a[k], FLOAT, "SCALAR")

    def b4(k):
        return g.accessor(a[k], UBYTE, "VEC4")

    pos, tangent, down, lateral = (
        v3("POSITION"),
        v3("_OAG_TANGENT"),
        v3("_OAG_DOWN"),
        v3("_OAG_LATERAL"),
    )
    progress, line = s1("_OAG_PROGRESS"), s1("_OAG_RACING_LINE")
    half, bound = v2("_OAG_HALF_WIDTH"), v2("_OAG_AI_BOUND")
    sf, light = b4("_OAG_SECTION_FLAGS"), b4("_OAG_LIGHT_SCALE")
    ai = b4("_OAG_AI_BYTES") if "_OAG_AI_BYTES" in a else None
    cage_a = b4("_OAG_CAGE_A") if "_OAG_CAGE_A" in a else None
    cage_b = b4("_OAG_CAGE_B") if "_OAG_CAGE_B" in a else None
    out = []
    for i in range(len(pos)):
        ai_bytes = tuple(ai[i][:3]) if ai is not None and ai[i][3] == 1 else None
        cage = None
        if cage_a is not None and cage_b is not None:
            cage = tuple(v - 256 if v > 127 else v for v in (*cage_a[i], *cage_b[i]))
        out.append(
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
                sf[i][0],
                sf[i][1],
                tuple(light[i]),
                ai_bytes,
                cage,
            )
        )
    return out


def _read_pads(g, node, name):
    a = g.prim(node, name)["attributes"]
    matrix = g.accessor(a["_OAG_TO_WORLD"], FLOAT, "MAT4")
    lo = g.accessor(a["_OAG_MIN"], FLOAT, "VEC3")
    hi = g.accessor(a["_OAG_MAX"], FLOAT, "VEC3")
    off = g.accessor(a["_OAG_DISABLED"], FLOAT, "SCALAR")
    props = _read_props(node, name, len(matrix))
    return [Pad(matrix[i], lo[i], hi[i], off[i], props[i]) for i in range(len(matrix))]


def _read_props(node, name, n):
    rows = (node.get("extras") or {}).get("oag_props")
    if rows is None:
        return [{} for _ in range(n)]
    if len(rows) != n:
        raise TrackFileError(f"{name}: {len(rows)} oag_props entries for {n} records")
    return [{k: prop_value(f"{name} {i}", k, v) for k, v in row.items()} for i, row in enumerate(rows)]


def _read_triggers(g, node):
    a = g.prim(node, TRIGGERS)["attributes"]
    matrix = g.accessor(a["_OAG_TO_WORLD"], FLOAT, "MAT4")
    lo = g.accessor(a["_OAG_MIN"], FLOAT, "VEC3")
    hi = g.accessor(a["_OAG_MAX"], FLOAT, "VEC3")
    names = (node.get("extras") or {}).get("oag_names", [])
    if len(names) != len(matrix):
        raise TrackFileError(f"{TRIGGERS}: {len(names)} names for {len(matrix)} triggers")
    props = _read_props(node, TRIGGERS, len(matrix))
    return [Trigger(names[i], matrix[i], lo[i], hi[i], props[i]) for i in range(len(matrix))]


def _read_collision(g, node, kind, extras):
    meshes = []
    prims = g.doc["meshes"][node["mesh"]]["primitives"] if "mesh" in node else []
    for prim in prims:
        a = prim["attributes"]
        verts = g.accessor(a["POSITION"], FLOAT, "VEC3")
        scalars = (
            g.accessor(a["_OAG_SCALAR"], FLOAT, "SCALAR")
            if "_OAG_SCALAR" in a
            else [1.0] * len(verts)
        )
        idx = g.accessor(prim["indices"], USHORT, "SCALAR")
        tris = [tuple(idx[i : i + 3]) for i in range(0, len(idx), 3)]
        meshes.append(CollisionMesh(verts, tris, scalars))
    return CollisionNode(
        kind, meshes, extras.get("oag_name"), extras.get("oag_version", HEADER_WORD)
    )


def _index(v):
    return None if v < 0 else v


def read(manifest_path):
    """Reads `<manifest>` and the glb it names into a `Track`."""
    manifest_path = os.fspath(manifest_path)
    with open(manifest_path, "rb") as fh:
        m = tomllib.load(fh)
    loc = (m.get("toml-schema") or {}).get("location", "")
    if not loc.endswith("/v1/track.tosd"):
        raise TrackFileError(f"not a v1 track manifest: location {loc!r}")
    glb_path = os.path.join(os.path.dirname(manifest_path), m["track"]["glb"])
    with open(glb_path, "rb") as fh:
        g = _Glb(fh.read())
    rows = m.get("spline", {}).get("path", [])
    splines = [None] * len(rows)
    start, speedup, weapon, nodes, art, triggers = None, [], [], [], [], []
    art_nodes, sun = {}, None
    for at, node in enumerate(g.doc["nodes"]):
        name = node.get("name", "")
        extras = node.get("extras") or {}
        if "oag_kind" in extras:
            kind = extras["oag_kind"]
            if kind not in COLLISION_KINDS:
                raise TrackFileError(f"unknown oag_kind {kind!r}")
            nodes.append(_read_collision(g, node, kind, extras))
        elif name == START:
            start = _read_start(g, node)
        elif name.startswith(SPLINE):
            splines[int(name[len(SPLINE) :])] = _read_spline(g, node, name)
        elif name == SPEEDUP:
            speedup = _read_pads(g, node, name)
        elif name == WEAPON:
            weapon = _read_pads(g, node, name)
        elif name == TRIGGERS:
            triggers = _read_triggers(g, node)
        elif name.startswith(artfile.PREFIX):
            i = int(name[len(artfile.PREFIX) :])
            art.append((i, artfile.read_node(g, node, FLOAT)))
            art_nodes[at] = i
        elif name == motionfile.SUN:
            sun = (at, tuple(f32.r(float(c)) for c in node.get("rotation", (0.0, 0.0, 0.0, 1.0))))
    if start is None:
        raise TrackFileError(f"no {START} node")
    paths = []
    for row, points in zip(rows, splines):
        points = points or []
        if len(points) != row["points"]:
            raise TrackFileError("a spline's point count disagrees with the manifest")
        paths.append(
            Path(
                points,
                f32.from_bits(row["max_spacing_bits"]),
                _index(row["entry"]),
                _index(row["exit"]),
            )
        )
    junctions = [
        Junction(
            (_index(j["prev"][0]), _index(j["prev"][1])),
            (_index(j["next"][0]), _index(j["next"][1])),
        )
        for j in m.get("spline", {}).get("junction", [])
    ]
    t = m["track"]
    art.sort(key=lambda a: a[0])
    if [i for i, _ in art] != list(range(len(art))):
        raise TrackFileError("the art meshes are not numbered 0..n")
    track_art = None
    if "art" in m or art:
        track_art = artfile.from_manifest(m.get("art"), [a for _, a in art])
        try:
            track_art.animations = motionfile.read(
                g, lambda at: "sun" if sun is not None and at == sun[0] else art_nodes.get(at)
            )
            track_art.sun_rotation = None if sun is None else sun[1]
            motionfile.validate(track_art)
        except motionfile.MotionError as e:
            raise TrackFileError(str(e)) from e
    host = t.get("host", "")
    if not host and track_art is not None and track_art.borrows_from_a_disc():
        raise TrackFileError("the art borrows from a disc but track.host names no title")
    return Track(
        t["id"],
        t["title"],
        host,
        list(t.get("titles", [])),
        m["spline"]["point_version"],
        paths,
        junctions,
        start,
        nodes,
        speedup,
        weapon,
        list(t.get("notes", [])),
        track_art,
        {k: prop_value("props", k, v) for k, v in (m.get("props") or {}).items()},
        triggers,
    )
