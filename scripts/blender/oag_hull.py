"""A craft hull of this project's own, between Blender and `hull.glb` (docs/formats/own-hull.md).

In a live Blender session (Scripting tab, or the Python console):

    import sys; sys.path.insert(0, "<repo>/scripts/blender")
    import oag_hull
    oag_hull.export_hull(bpy.context.selected_objects, "<path>/hull.glb")
    oag_hull.import_hull("<path>/hull.glb")          # the inverse, into the current scene

Headless:

    blender -b --factory-startup --python-exit-code 1 --python scripts/blender/oag_hull.py -- export <scene.blend> <out.glb>
    blender -b --factory-startup --python-exit-code 1 --python scripts/blender/oag_hull.py -- reexport <in.glb> <out.glb>
    blender -b --factory-startup --python-exit-code 1 --python scripts/blender/oag_hull.py -- placeholder <out.glb>

The scene convention:
- One mesh object per part, named as the host mesh node (`shipShape`, `AirBrake_LeftShape`).
- Material slots named as the host texture's file name (`texture1.tga`).
- An object named `shadowShape` is the shadow occluder.
- Empties are locators (`engine_flare`, `ship_collision_fx1`); `ship_muzzle` and `cannon_flash_*`
  stay the host's, since the simulation reads them.
- Blender's `.NNN` suffixes are stripped.
- The object's origin is the part's pivot (an airbrake's hinge; the flap swings about its local X).
- Blender (x, y, z) is engine (x, z, -y), so the nose points Blender -Y.
- Engine uv is (u, 1 - v).
- Vertex colour comes from the active colour attribute, sRGB bytes, and is omitted when the
  mesh has none.
- Normals are the corner normals.

The glb is written here, byte by byte, with the track add-on's buffer writer, never through
Blender's glTF exporter (it re-splits vertices). The output is byte-stable for the same scene.
"""

import os
import re
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

from oag_track import trackfile  # noqa: E402
from oag_track.f32 import r as f32  # noqa: E402
from oag_track.f32 import text as f32_text  # noqa: E402

# The marker in the asset's extras; the reader refuses another kind or major.
KIND = "hull"
VERSION = "1.0.0"
SHADOW = "shadowshape"
SHADOW_NAME = "shadowShape"
TRIANGLES = 4
# Engine from Blender: (x, y, z) -> (x, z, -y).
AXES = Matrix(((1, 0, 0, 0), (0, 0, 1, 0), (0, -1, 0, 0), (0, 0, 0, 1)))
AXES_INV = AXES.transposed()
# The corner attribute an imported hull keeps its file's normals in, engine axes. Blender stores
# custom normals lossily, so a corner read back is a hair off what was set; a corner whose
# normal still is the file's writes the file's digits again.
KEPT_NORMALS = "oag_normal"
# The same for the file's texture coordinates: `1 - v` and back is not exact in `f32`.
KEPT_UVS = "oag_uv"
SAME_UV = 1e-6
# Two normals this close (radians) are one: the same corner read through two triangles.
SAME_NORMAL = 1e-3


def clean(name):
    """A Blender name without its `.NNN` duplicate suffix."""
    return re.sub(r"\.\d{3}$", "", name)


class HullFileError(Exception):
    """A hull the file cannot hold, or a file this reader refuses."""


def _column_major(m):
    return [f32(m[row][col]) for col in range(4) for row in range(4)]


def _engine_matrix(obj):
    return AXES @ obj.matrix_world @ AXES_INV


def _surfaces(obj, depsgraph):
    """{material name: (positions, normals, uvs, colours or None, triangles)} in engine axes,
    part-local space."""
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh()
    try:
        mesh.calc_loop_triangles()
        uv_layer = mesh.uv_layers.active
        colour = mesh.color_attributes.active_color
        corner_normals = mesh.corner_normals
        kept = mesh.attributes.get(KEPT_NORMALS)
        if kept is not None and (kept.domain != "CORNER" or kept.data_type != "FLOAT_VECTOR"):
            kept = None
        kept_uvs = mesh.attributes.get(KEPT_UVS)
        if kept_uvs is not None and (kept_uvs.domain != "CORNER" or kept_uvs.data_type != "FLOAT2"):
            kept_uvs = None
        names = [clean(slot.material.name) if slot.material else "" for slot in obj.material_slots]
        out = {}
        for tri in mesh.loop_triangles:
            name = names[tri.material_index] if tri.material_index < len(names) else ""
            surface = out.setdefault(name, ([], [], [], [] if colour else None, [], {}))
            positions, normals, uvs, colours, triangles, seen = surface
            corner = []
            for loop in tri.loops:
                vi = mesh.loops[loop].vertex_index
                p = AXES @ mesh.vertices[vi].co
                n = AXES @ Vector(corner_normals[loop].vector)
                if kept is not None:
                    was = Vector(kept.data[loop].vector)
                    if was.length_squared > 0.25 and was.angle(n, 9.0) < SAME_NORMAL:
                        n = was
                uv = uv_layer.data[loop].uv if uv_layer else (0.0, 0.0)
                uv = (uv[0], 1.0 - uv[1])
                if kept_uvs is not None:
                    was = kept_uvs.data[loop].vector
                    if abs(was[0] - uv[0]) < SAME_UV and abs(was[1] - uv[1]) < SAME_UV:
                        uv = (was[0], was[1])
                rgba = None
                if colour is not None:
                    index = loop if colour.domain == "CORNER" else vi
                    rgba = tuple(max(0, min(255, round(c * 255))) for c in colour.data[index].color_srgb)
                key = (tuple(f32_text(x) for x in p), tuple(f32_text(x) for x in uv), rgba)
                here = seen.setdefault(key, [])
                at = next((i for m, i in here if m.angle(n, 9.0) < SAME_NORMAL), None)
                if at is None:
                    at = len(positions)
                    here.append((n.copy(), at))
                    positions.append(tuple(f32(x) for x in p))
                    normals.append(tuple(f32(x) for x in n))
                    uvs.append(tuple(f32(x) for x in uv))
                    if colours is not None:
                        colours.append(rgba)
                corner.append(at)
            triangles.append(corner)
        return {k: v[:5] for k, v in out.items()}
    finally:
        evaluated.to_mesh_clear()


def _shadow(obj, depsgraph):
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh()
    try:
        mesh.calc_loop_triangles()
        world = AXES @ obj.matrix_world
        positions = [tuple(f32(x) for x in world @ v.co) for v in mesh.vertices]
        triangles = [list(t.vertices) for t in mesh.loop_triangles]
        return positions, triangles
    finally:
        evaluated.to_mesh_clear()


def _indices(b, triangles, vertices, what):
    flat = [i for t in triangles for i in t]
    if not flat:
        raise HullFileError(f"{what}: no triangle")
    if vertices < 0xFFFF:
        payload, component = struct.pack(f"<{len(flat)}H", *flat), trackfile.USHORT
    else:
        payload, component = struct.pack(f"<{len(flat)}I", *flat), trackfile.artfile.UINT
    return b.add(payload, component, "SCALAR", len(flat), trackfile.ELEMENT_ARRAY_BUFFER)


def write_glb(parts, locators, shadow):
    """The glb bytes of a hull. `parts` is [(name, matrix, {texture: (positions, normals, uvs,
    colours or None, triangles)})], `locators` [(name, matrix)], `shadow` (positions, triangles)
    or None; engine axes, matrices column-major, every number already an `f32`."""
    b = trackfile._Bin()
    nodes, meshes, materials = [], [], []

    def material(name):
        if name not in materials:
            materials.append(name)
        return materials.index(name)

    for name, matrix, surfaces in parts:
        trackfile._finite(name, matrix)
        prims = []
        for texture, (positions, normals, uvs, colours, triangles) in sorted(surfaces.items()):
            what = f"{name} {texture}"
            trackfile._finite(what, [x for row in (*positions, *normals, *uvs) for x in row])
            attrs = {
                "POSITION": b.positions(positions),
                "NORMAL": b.floats(normals, "VEC3"),
                "TEXCOORD_0": b.floats(uvs, "VEC2"),
            }
            if colours is not None:
                attrs["COLOR_0"] = b.bytes4(colours)
                b.accessors[attrs["COLOR_0"]]["normalized"] = True
            indices = _indices(b, triangles, len(positions), what)
            prims.append({"attributes": attrs, "indices": indices, "material": material(texture), "mode": TRIANGLES})
        if not prims:
            raise HullFileError(f"{name}: no surface")
        meshes.append({"name": name, "primitives": prims})
        nodes.append({"name": name, "matrix": matrix, "mesh": len(meshes) - 1})
    for name, matrix in locators:
        trackfile._finite(name, matrix)
        nodes.append({"name": name, "matrix": matrix})
    if shadow is not None:
        positions, triangles = shadow
        trackfile._finite(SHADOW_NAME, [x for p in positions for x in p])
        attrs = {"POSITION": b.positions(positions)}
        indices = _indices(b, triangles, len(positions), SHADOW_NAME)
        meshes.append({"name": SHADOW_NAME, "primitives": [{"attributes": attrs, "indices": indices, "mode": TRIANGLES}]})
        nodes.append({"name": SHADOW_NAME, "mesh": len(meshes) - 1})
    while len(b.data) % 4:
        b.data.append(0)
    doc = {
        "asset": {
            "version": "2.0",
            "generator": "scripts/blender/oag_hull.py",
            "extras": {"oag_kind": KIND, "oag_version": VERSION},
        },
        "scene": 0,
        "scenes": [{"nodes": list(range(len(nodes)))}],
        "nodes": nodes,
        "meshes": meshes,
        "materials": [{"name": m} for m in materials],
        "buffers": [{"byteLength": len(b.data)}],
        "bufferViews": b.views,
        "accessors": b.accessors,
    }
    return trackfile.pack_glb(doc, b.data)


def export_hull(objects, path):
    """Writes `objects` (mesh parts, a `shadowShape`, locator empties) to `path`."""
    depsgraph = bpy.context.evaluated_depsgraph_get()
    meshes = sorted((o for o in objects if o.type == "MESH"), key=lambda o: clean(o.name))
    empties = sorted((o for o in objects if o.type == "EMPTY"), key=lambda o: clean(o.name))
    parts, shadow = [], None
    for obj in meshes:
        if clean(obj.name).lower() == SHADOW:
            shadow = _shadow(obj, depsgraph)
            continue
        parts.append((clean(obj.name), _column_major(_engine_matrix(obj)), _surfaces(obj, depsgraph)))
    locators = [(clean(obj.name), _column_major(_engine_matrix(obj))) for obj in empties]
    glb = write_glb(parts, locators, shadow)
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, "wb") as f:
        f.write(glb)


def read_glb(data):
    """The hull in glb bytes `data`, in `write_glb`'s shapes: (parts, locators, shadow), every
    surface (positions, normals, uvs, colours or None, triangles). A surface is keyed by its
    material's name; images the file may carry are not read."""
    g = trackfile._Glb(data)
    extras = g.doc.get("asset", {}).get("extras") or {}
    if extras.get("oag_kind") != KIND:
        raise HullFileError(f"not a hull glb (oag_kind {extras.get('oag_kind')!r})")
    if str(extras.get("oag_version", "")).split(".")[0] != VERSION.split(".")[0]:
        raise HullFileError(f"hull version {extras.get('oag_version')!r}, this reader takes {VERSION}")
    materials = [m.get("name", "") for m in g.doc.get("materials", [])]

    def indices(prim):
        a = g.doc["accessors"][prim["indices"]]
        return g.accessor(prim["indices"], a["componentType"], "SCALAR")

    parts, locators, shadow = [], [], None
    for node in g.doc.get("nodes", []):
        name = node.get("name", "")
        if "mesh" not in node:
            locators.append((name, list(node["matrix"])))
            continue
        prims = g.doc["meshes"][node["mesh"]]["primitives"]
        if clean(name).lower() == SHADOW:
            a = prims[0]["attributes"]
            shadow = (g.accessor(a["POSITION"], trackfile.FLOAT, "VEC3"), _chunks(indices(prims[0]), 3))
            continue
        surfaces = {}
        for prim in prims:
            a = prim["attributes"]
            colours = g.accessor(a["COLOR_0"], trackfile.UBYTE, "VEC4") if "COLOR_0" in a else None
            surfaces[materials[prim["material"]]] = (
                g.accessor(a["POSITION"], trackfile.FLOAT, "VEC3"),
                g.accessor(a["NORMAL"], trackfile.FLOAT, "VEC3"),
                g.accessor(a["TEXCOORD_0"], trackfile.FLOAT, "VEC2"),
                colours,
                _chunks(indices(prim), 3),
            )
        parts.append((name, list(node["matrix"]), surfaces))
    return parts, locators, shadow


def _chunks(values, n):
    return [tuple(values[i : i + n]) for i in range(0, len(values), n)]


def _matrix(flat):
    return Matrix([[flat[c * 4 + r] for c in range(4)] for r in range(4)])


def _material(name):
    return bpy.data.materials.get(name) or bpy.data.materials.new(name)


def import_hull(path, collection=None):
    """Builds the objects `path` describes in `collection` (the scene's by default); returns them."""
    with open(path, "rb") as f:
        parts, locators, shadow = read_glb(f.read())
    collection = collection or bpy.context.scene.collection
    made = []
    for name, transform, surfaces in parts:
        mesh = bpy.data.meshes.new(name)
        verts, faces, face_mats, normals, uvs, colours = [], [], [], [], [], []
        file_normals, file_uvs = [], []
        has_colour = any(s[3] is not None for s in surfaces.values())
        textures = []
        for texture, (ps, ns, ts, cs, tris) in surfaces.items():
            base = len(verts)
            textures.append(texture)
            cs = cs if cs is not None else [(255, 255, 255, 255)] * len(ps)
            verts += [AXES_INV @ Vector(p) for p in ps]
            for tri in tris:
                faces.append(tuple(base + i for i in tri))
                face_mats.append(len(textures) - 1)
                for i in tri:
                    normals.append(AXES_INV @ Vector(ns[i]))
                    file_normals.append(ns[i])
                    file_uvs.append(ts[i])
                    uvs.append((ts[i][0], 1.0 - ts[i][1]))
                    colours.append(cs[i])
        mesh.from_pydata(verts, [], faces)
        for texture in textures:
            mesh.materials.append(_material(texture))
        for poly, m in zip(mesh.polygons, face_mats):
            poly.material_index = m
        uv = mesh.uv_layers.new(name="UVMap")
        for i, value in enumerate(uvs):
            uv.data[i].uv = value
        if has_colour:
            attr = mesh.color_attributes.new("Col", "BYTE_COLOR", "CORNER")
            for i, c in enumerate(colours):
                attr.data[i].color_srgb = tuple(x / 255 for x in c)
            mesh.color_attributes.active_color = attr
        mesh.normals_split_custom_set([tuple(n) for n in normals])
        keep = mesh.attributes.new(KEPT_NORMALS, "FLOAT_VECTOR", "CORNER")
        for i, n in enumerate(file_normals):
            keep.data[i].vector = n
        keep = mesh.attributes.new(KEPT_UVS, "FLOAT2", "CORNER")
        for i, t in enumerate(file_uvs):
            keep.data[i].vector = t
        obj = bpy.data.objects.new(name, mesh)
        obj.matrix_world = AXES_INV @ _matrix(transform) @ AXES
        collection.objects.link(obj)
        made.append(obj)
    for name, transform in locators:
        obj = bpy.data.objects.new(name, None)
        obj.empty_display_type = "ARROWS"
        obj.matrix_world = AXES_INV @ _matrix(transform) @ AXES
        collection.objects.link(obj)
        made.append(obj)
    if shadow is not None:
        positions, triangles = shadow
        mesh = bpy.data.meshes.new(SHADOW_NAME)
        mesh.from_pydata([AXES_INV @ Vector(p) for p in positions], [], triangles)
        obj = bpy.data.objects.new(SHADOW_NAME, mesh)
        collection.objects.link(obj)
        made.append(obj)
    return made


def _box(name, size, at, textures):
    """A cuboid of `size` centred on its origin at Blender `at`, one material per entry of
    `textures` (faces dealt in turn), flat shaded."""
    import bmesh

    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    for v in bm.verts:
        v.co = Vector((v.co.x * size[0], v.co.y * size[1], v.co.z * size[2]))
    bmesh.ops.triangulate(bm, faces=bm.faces[:])
    uv = bm.loops.layers.uv.new("UVMap")
    for i, face in enumerate(bm.faces):
        face.material_index = i % len(textures)
        face.smooth = False
        for loop in face.loops:
            loop[uv].uv = (0.5 + loop.vert.co.x * 0.1, 0.5 + loop.vert.co.y * 0.05)
    bm.to_mesh(mesh)
    bm.free()
    for texture in textures:
        mesh.materials.append(_material(texture))
    obj = bpy.data.objects.new(name, mesh)
    obj.location = at
    bpy.context.scene.collection.objects.link(obj)
    return obj


def build_placeholder():
    """A plain wedge with an airbrake pair: wiring stand-in until the real hull is exported."""
    import bmesh

    made = []
    mesh = bpy.data.meshes.new("shipShape")
    bm = bmesh.new()
    # Nose at Blender -Y, tail at +Y, a flat underside.
    pts = [(0, -7, -0.2), (-2.6, 5, -0.6), (2.6, 5, -0.6), (0, 5, 1.2), (0, 5, -0.6)]
    vs = [bm.verts.new(p) for p in pts]
    for face in ((0, 2, 3), (0, 3, 1), (0, 1, 4), (0, 4, 2), (1, 3, 4), (2, 4, 3)):
        bm.faces.new([vs[i] for i in face])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    uv = bm.loops.layers.uv.new("UVMap")
    for i, face in enumerate(bm.faces):
        face.material_index = 0 if i < 4 else 1
        face.smooth = False
        for loop in face.loops:
            loop[uv].uv = (0.5 + loop.vert.co.x * 0.18, 0.5 + loop.vert.co.y * 0.07)
    bm.to_mesh(mesh)
    bm.free()
    for texture in ("texture1.tga", "engine_general.tga"):
        mesh.materials.append(_material(texture))
    ship = bpy.data.objects.new("shipShape", mesh)
    bpy.context.scene.collection.objects.link(ship)
    made.append(ship)
    for name, x, texture in (("AirBrake_LeftShape", 2.0, "texture2.tga"), ("AirBrake_RightShape", -2.0, "texture3.tga")):
        made.append(_box(name, (0.9, 0.5, 0.06), (x, 4.6, 0.0), [texture, "engine_general.tga"]))
    made.append(_box("shadowShape", (5.2, 12, 1.6), (0, -1, 0.3), ["engine_general.tga"]))
    flare = bpy.data.objects.new("engine_flare", None)
    flare.location = (0, 5.2, 0.2)
    bpy.context.scene.collection.objects.link(flare)
    made.append(flare)
    return made


def _main(argv):
    if not argv:
        raise SystemExit("usage: oag_hull.py -- export <scene.blend> <out> | reexport <in> <out> | placeholder <out>")
    mode = argv[0]
    if mode == "export":
        bpy.ops.wm.open_mainfile(filepath=argv[1])
        export_hull(list(bpy.context.scene.objects), argv[2])
    elif mode == "reexport":
        bpy.ops.wm.read_factory_settings(use_empty=True)
        export_hull(import_hull(argv[1]), argv[2])
    elif mode == "placeholder":
        bpy.ops.wm.read_factory_settings(use_empty=True)
        export_hull(build_placeholder(), argv[1])
    else:
        raise SystemExit(f"unknown mode {mode!r}")


if __name__ == "__main__":
    _main(sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else [])
