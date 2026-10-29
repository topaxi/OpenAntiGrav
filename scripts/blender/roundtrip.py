"""Headless driver for the add-on (docs/tools/blender.md, `just blender-fixtures`).

    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- loop <out-dir>
    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- motion <out-dir>
    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- reexport <in.track.toml> <out-dir>
    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- line <out-dir>

`loop` builds a track from scratch (8 control points, floor, walls, start, one pad
of each kind, and art: a canopy over the line textured with a generated checker,
a sun, a fill light, an ambient, its own sky (gradient, checker overhead), a
lens flare and a plate on each pad) and exports
`blender_loop.track.toml` with `blender_checker.png` beside it. `motion` is the
same loop with keyframed motion (ADR-0076 decision 10): an airship box that flies
and spins on a loop, and a sun that turns and holds, as `blender_motion.track.toml`.
The checker is
written here, byte for byte: never disc pixels. `reexport` imports a track
file with the add-on's importer and exports it again as `<stem>.reexport.track.toml`.
`line` writes no fixture: it asserts the racing-line paths (computed by default, off by
switch, the operator's editable curve, a nudged point, the import round trip).
The add-on is registered from this directory, never installed: nothing is written
to the Blender user directory.
"""

import math
import os
import struct
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import bpy  # noqa: E402
from mathutils import Quaternion, Vector  # noqa: E402

import oag_track  # noqa: E402
from oag_track import racing_line, scene, trackfile  # noqa: E402

# Eight control points of a loop, in Blender's X/Y plane (Z up), world units.
CONTROL = [
    (-900, -300),
    (-300, -500),
    (300, -500),
    (900, -300),
    (900, 300),
    (300, 500),
    (-300, 500),
    (-900, 300),
]


def checker_png(path, size=64, cell=8):
    """A deterministic RGBA checker PNG (two greys, alpha 255 and 0 in the corner cell)."""
    rows = b""
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            light = ((x // cell) + (y // cell)) % 2 == 0
            v = 200 if light else 60
            a = 0 if (x < cell and y < cell) else 255
            row += bytes((v, v, v, a))
        rows += bytes(row)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(rows, 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as fh:
        fh.write(png)


def add_art(ctx, out_dir):
    """A canopy over the line near (1000, 0), textured with the checker, a cutout."""
    png = os.path.join(out_dir, "blender_checker.png")
    checker_png(png)
    image = bpy.data.images.load(png)
    mat = bpy.data.materials.new("checker")
    mat.use_nodes = True
    mat.use_backface_culling = False
    tex = mat.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = image
    bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    mat.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    clip = mat.node_tree.nodes.new("ShaderNodeMath")  # alpha clip, the Blender 4.2+ way
    clip.operation = "GREATER_THAN"
    clip.inputs[1].default_value = 0.5
    mat.node_tree.links.new(tex.outputs["Alpha"], clip.inputs[0])
    mat.node_tree.links.new(clip.outputs[0], bsdf.inputs["Alpha"])
    me = bpy.data.meshes.new("canopy")
    me.from_pydata(
        [(950.0, -60.0, 40.0), (1050.0, -60.0, 40.0), (1050.0, 60.0, 40.0), (950.0, 60.0, 40.0)],
        [],
        [(0, 1, 2, 3)],
    )
    uv = me.uv_layers.new(name="UVMap")
    for loop, (u, v) in zip(me.loops, ((0.0, 0.0), (1.0, 0.0), (1.0, 0.25), (0.0, 1.0))):
        uv.data[loop.index].uv = (u, v)
    col = me.color_attributes.new("Col", "FLOAT_COLOR", "CORNER")
    for i, c in enumerate(col.data):
        c.color = (0.25, 0.5, 1.0, 1.0)
    col.data[0].color_srgb = (0.5, 0.5, 0.5, 1.0)
    me.color_attributes.active_color = col
    me.materials.append(mat)
    obj = bpy.data.objects.new("canopy", me)
    ctx.scene.collection.objects.link(obj)
    obj.oag_shadow = "cast"
    obj.oag_layer = 2

    def sun_lamp(name, direction, colour, intensity):
        data = bpy.data.lights.new(name, "SUN")
        data.color = colour
        lamp = bpy.data.objects.new(name, data)
        ctx.scene.collection.objects.link(lamp)
        lamp.rotation_mode = "QUATERNION"
        lamp.rotation_quaternion = Vector(direction).normalized().to_track_quat("Z", "Y")
        lamp.oag_intensity = intensity
        return lamp

    sun = sun_lamp("sun", (0.0, 0.0, 1.0), (1.0, 0.9, 0.7), 1.0)
    sun.data.angle = math.radians(2.0)
    sun_lamp("fill", (0.0, 1.0, 0.0), (0.2, 0.25, 0.4), 0.5)
    ctx.scene.oag_sun = sun
    world = bpy.data.worlds.new("World")
    ctx.scene.world = world
    world.use_nodes = True
    bg = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs[0].default_value = (0.1, 0.12, 0.15, 1.0)
    bg.inputs[1].default_value = 1.0
    # Its own sky, so the fixture borrows nothing from a disc and runs on every
    # title: the gradient, with the checker overhead (ADR-0076 decision 14).
    ctx.scene.oag_skybox = True
    ctx.scene.oag_sky_gradient = True
    ctx.scene.oag_sky_zenith = (0.1, 0.25, 0.6)
    ctx.scene.oag_sky_horizon = (0.7, 0.75, 0.8)
    ctx.scene.oag_sky_face_2 = image
    ctx.scene.oag_lensflare = True
    ctx.scene.oag_lensflare_atlas = image


def fresh():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    if hasattr(bpy.types, "OAG_OT_tag"):  # a second scene in one run: start from unregistered
        oag_track.unregister()
    oag_track.register()


def make_loop(ctx):
    """The eight-point loop's tagged spline Curve, active, in the current scene."""
    curve = bpy.data.curves.new("centre line", "CURVE")
    curve.dimensions = "3D"
    sp = curve.splines.new("BEZIER")
    sp.bezier_points.add(len(CONTROL) - 1)
    for p, (x, y) in zip(sp.bezier_points, CONTROL):
        p.co = (x, -y, 0.0)
        p.handle_left_type = p.handle_right_type = "AUTO"
    sp.use_cyclic_u = True
    obj = bpy.data.objects.new("centre line", curve)
    ctx.scene.collection.objects.link(obj)
    obj["oag_kind"] = "spline"
    obj.oag_points = 64
    ctx.view_layer.objects.active = obj
    return obj


def add_motion(ctx):
    """An airship (a box, two material slots) flying a loop over the line, and the sun turning.

    Keys at frames 1, 49 and 97 of a 30 fps scene: the airship, 60 above the first
    straight (Blender y = 500, where the start's camera looks), goes 400 units along +X
    and comes back, spinning a full turn about Z (Blender's default Bezier easing,
    sampled per frame on export), and loops; the sun tilts 30 degrees by frame 61 and
    holds (clamp).
    """
    scene = ctx.scene
    scene.render.fps, scene.render.fps_base = 30, 1.0
    scene.frame_start, scene.frame_end = 1, 97
    mats = []
    for name, colour in (("hull", (0.8, 0.2, 0.1, 1.0)), ("gondola", (0.2, 0.2, 0.25, 1.0))):
        mat = bpy.data.materials.new(name)
        mat.use_nodes = True
        bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
        bsdf.inputs["Base Color"].default_value = colour
        mats.append(mat)
    s = 20.0
    verts = [(x * 2 * s, y * s, z * s) for x in (-1, 1) for y in (-1, 1) for z in (-1, 1)]
    faces = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
    me = bpy.data.meshes.new("airship")
    me.from_pydata(verts, [], faces)
    for m in mats:
        me.materials.append(m)
    for poly in me.polygons:
        poly.material_index = 1 if poly.index == 0 else 0
    # Baked light, dark against the sky: a red hull and a near-black gondola.
    col = me.color_attributes.new("Col", "FLOAT_COLOR", "CORNER")
    for poly in me.polygons:
        for loop in poly.loop_indices:
            col.data[loop].color = (0.05, 0.05, 0.08, 1.0) if poly.index == 0 else (0.45, 0.04, 0.02, 1.0)
    me.color_attributes.active_color = col
    obj = bpy.data.objects.new("airship", me)
    scene.collection.objects.link(obj)
    obj.oag_loop = "loop"
    for frame, x, turn in ((1, -200.0, 0.0), (49, 200.0, math.pi), (97, -200.0, 2 * math.pi)):
        obj.location = (x, 500.0, 60.0)
        obj.rotation_euler = (0.0, 0.0, turn)
        obj.keyframe_insert("location", frame=frame)
        obj.keyframe_insert("rotation_euler", frame=frame)
    sun = scene.oag_sun
    rest = sun.rotation_quaternion.copy()
    sun.keyframe_insert("rotation_quaternion", frame=1)
    sun.rotation_quaternion = Quaternion((1.0, 0.0, 0.0), math.radians(30.0)) @ rest
    sun.keyframe_insert("rotation_quaternion", frame=61)
    sun.oag_loop = "clamp"
    scene.frame_set(1)


def build_loop(out_dir, motion=False):
    fresh()
    ctx = bpy.context
    obj = make_loop(ctx)
    ctx.scene.oag_track_id = "blender-motion" if motion else "blender-loop"
    ctx.scene.oag_track_title = "Blender Motion" if motion else "Blender Loop"
    ctx.scene.oag_track_notes = "authored headless in Blender with the OAG add-on: 8 control points (chosen, not measured)"
    assert bpy.ops.oag.build_ribbon() == {"FINISHED"}
    assert bpy.ops.oag.add_start() == {"FINISHED"}
    ctx.scene.cursor.location = (900.0, 0.0, 0.0)
    assert bpy.ops.oag.add_pad(kind="speedup") == {"FINISHED"}
    ctx.scene.cursor.location = (-900.0, 0.0, 0.0)
    assert bpy.ops.oag.add_pad(kind="weapon") == {"FINISHED"}
    add_art(ctx, out_dir)
    assert bpy.ops.oag.add_pad_plates() == {"FINISHED"}
    obj.oag_cover_light = 96
    if motion:
        add_motion(ctx)
    export(os.path.join(out_dir, "blender_motion.track.toml" if motion else "blender_loop.track.toml"))


def export(path):
    track = scene.write_track(bpy.context, path)
    art = len(track.art.meshes) if track.art else 0
    print(f"wrote {path}: {len(track.paths[0].points)} points, {len(track.nodes)} nodes, {art} art meshes")


def reexport(src, out_dir):
    fresh()
    track = trackfile.read(src)
    scene.populate(bpy.context, track, os.path.dirname(src))
    stem = os.path.basename(src).removesuffix(".track.toml")
    export(os.path.join(out_dir, f"{stem}.reexport.track.toml"))


def read_lines(path):
    return [p.racing_line for p in trackfile.read(path).paths[0].points]


def line_checks(out_dir):
    """The racing line's paths, each asserted (docs/tools/blender.md, "The racing line")."""
    fresh()
    ctx = bpy.context
    obj = make_loop(ctx)
    assert bpy.ops.oag.build_ribbon() == {"FINISHED"}
    tagged = lambda kind: [o for o in ctx.scene.objects if o.get("oag_kind") == kind]

    # 1. Default: computed, inside the corridor and off the bounds by the margin.
    auto = [p.racing_line for p in scene.collect(ctx).paths[0].points]
    path = scene.collect(ctx).paths[0].points
    assert any(abs(v) > 1.0 for v in auto), "the default line stayed on the centre"
    for p, (lo, hi) in zip(path, racing_line._box(path, racing_line.MARGIN)):
        assert lo - 1e-5 <= p.racing_line <= hi + 1e-5

    # 2. The switch: off is the centre, exactly as before.
    obj.oag_auto_racing_line = False
    assert all(p.racing_line == 0.0 for p in scene.collect(ctx).paths[0].points)
    obj.oag_auto_racing_line = True

    # 3. The operator makes one editable curve, again replaces it, and the export of
    # an untouched curve is the computed line bit for bit.
    assert bpy.ops.oag.compute_racing_line() == {"FINISHED"}
    assert bpy.ops.oag.compute_racing_line() == {"FINISHED"}
    (line,) = tagged("racing_line")
    assert [p.racing_line for p in scene.collect(ctx).paths[0].points] == auto

    # 4. A nudged point moves its offset by about the nudge, nothing else moves; a
    # nudge past the corridor stops at the margin.
    spline = line.data.splines[0]
    k, push = 20, 3.0
    lateral = scene.to_blender(path[k].lateral)
    base = tuple(spline.points[k].co)[:3]
    spline.points[k].co = (*(base[i] + push * lateral[i] for i in range(3)), 1.0)
    nudged = [p.racing_line for p in scene.collect(ctx).paths[0].points]
    top = racing_line._box(path, racing_line.MARGIN)[k][1]
    assert abs((nudged[k] - auto[k]) - push) < 0.05 or abs(nudged[k] - top) < 0.01, nudged[k]
    assert max(abs(nudged[i] - auto[i]) for i in range(len(auto)) if i != k) < 0.05
    spline.points[k].co = (*(base[i] + 500.0 * lateral[i] for i in range(3)), 1.0)
    far = [p.racing_line for p in scene.collect(ctx).paths[0].points]
    assert abs(far[k] - top) < 1e-4, far[k]
    spline.points[k].co = (*base, 1.0)

    # 5. Export, import (the importer rebuilds the curve), export again: the same bytes.
    first = os.path.join(out_dir, "line_check.track.toml")
    scene.write_track(ctx, first)
    fresh()
    track = trackfile.read(first)
    scene.populate(ctx, track, out_dir)
    assert len(tagged("racing_line")) == 1, "the importer did not rebuild the racing-line curve"
    again = os.path.join(out_dir, "line_check.reexport.track.toml")
    scene.write_track(ctx, again)
    stem = lambda path, ext: path.removesuffix(".track.toml") + ext
    assert open(stem(first, ".glb"), "rb").read() == open(stem(again, ".glb"), "rb").read(), "glb changed"
    names = lambda path: open(path).read().replace(os.path.basename(stem(path, ".glb")), "<glb>")
    assert names(first) == names(again), "the manifest changed in the round trip"

    # 6. Editing the imported curve changes the imported (baked) spline's line.
    (line,) = tagged("racing_line")
    spline = line.data.splines[0]
    before = read_lines(again)
    base = tuple(spline.points[10].co)[:3]
    lateral = scene.to_blender(track.paths[0].points[10].lateral)
    push = -2.0 if before[10] > 0 else 2.0  # towards the centre: room inside the box
    spline.points[10].co = (*(base[i] + push * lateral[i] for i in range(3)), 1.0)
    edited = os.path.join(out_dir, "line_check.edited.track.toml")
    scene.write_track(ctx, edited)
    after = read_lines(edited)
    assert abs((after[10] - before[10]) - push) < 0.05, (before[10], after[10])
    # 7. An imported (baked) spline has no computed default, but the operator works on it.
    fresh()
    oval = os.path.join(HERE, "..", "..", "crates", "trackfile", "tests", "fixtures", "oval.track.toml")
    track = trackfile.read(oval)
    assert all(p.racing_line == 0.0 for p in track.paths[0].points)
    scene.populate(ctx, track, os.path.dirname(oval))
    assert not tagged("racing_line"), "a file with the centre line everywhere gets no curve"
    (spline_obj,) = tagged("spline")
    ctx.view_layer.objects.active = spline_obj
    assert bpy.ops.oag.compute_racing_line() == {"FINISHED"}
    out = os.path.join(out_dir, "line_check.oval.track.toml")
    scene.write_track(ctx, out)
    made = trackfile.read(out).paths[0].points
    assert any(abs(p.racing_line) > 1.0 for p in made), "the operator left the imported oval on the centre"
    for p, (lo, hi) in zip(made, racing_line._box(made, racing_line.MARGIN)):
        assert lo - 1e-5 <= p.racing_line <= hi + 1e-5
    print("racing line: default, switch, operator, nudge, clamp, import round trip, imported edit, operator on an import: ok")


def main():
    args = sys.argv[sys.argv.index("--") + 1 :]
    if args[:1] == ["loop"]:
        build_loop(args[1])
    elif args[:1] == ["motion"]:
        build_loop(args[1], motion=True)
    elif args[:1] == ["line"]:
        line_checks(args[1])
    elif args[:1] == ["reexport"]:
        reexport(args[1], args[2])
    else:
        raise SystemExit(__doc__)


main()
