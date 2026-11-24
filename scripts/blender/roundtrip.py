"""Headless driver for the add-on (docs/tools/blender.md, `just blender-fixtures`).

    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- loop <out-dir>
    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- motion <out-dir>
    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- reexport <in.track.toml> <out-dir>
    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- line <out-dir>
    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- channels <out-dir>
    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- textures <out-dir>

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
`channels` writes no committed fixture (its `channels.track.toml` has a gate over spline point 8, well
clear of the grid, and the airship starting on the gate's channel, for a headless
screenshot pair): it asserts a trigger's `oag_emit`, an animation's
`oag_on` and the track's published channels through export, import and a re-export.
The add-on is registered from this directory, never installed: nothing is written
to the Blender user directory.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import bpy  # noqa: E402
from mathutils import Quaternion, Vector  # noqa: E402

import oag_track  # noqa: E402
from oag_track import pngwrite, racing_line, scene, scene_art, trackfile  # noqa: E402

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
    px = bytearray()
    for y in range(size):
        for x in range(size):
            light = ((x // cell) + (y // cell)) % 2 == 0
            v = 200 if light else 60
            a = 0 if (x < cell and y < cell) else 255
            px += bytes((v, v, v, a))
    with open(path, "wb") as fh:
        fh.write(pngwrite.encode_rgba(size, size, px))


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


def author_loop(out_dir, motion=False):
    """The loop (with motion, if asked) authored in the open scene, not yet exported."""
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
    # Its art is a canopy and two plates, not the road: draw the collision under it.
    ctx.scene.oag_draw_collision = True
    if motion:
        add_motion(ctx)
    return ctx


def build_loop(out_dir, motion=False):
    author_loop(out_dir, motion)
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


def effect_checks(src, out_dir):
    """A mod effect TOML into a preview and back out (docs/tools/blender.md, "Mod effects")."""
    import tomllib

    from oag_track import effectfile

    fresh()
    bpy.ops.mesh.primitive_cube_add()
    cube = bpy.context.active_object
    assert bpy.ops.oag.preview_effect(filepath=src) == {"FINISHED"}
    systems = [m for m in cube.modifiers if m.type == "PARTICLE_SYSTEM"]
    with open(src, "rb") as f:
        want = tomllib.load(f)
    assert len(systems) == len(want["emitter"]), "one particle system per emitter"
    first = systems[0].particle_system.settings
    assert first.lifetime == want["emitter"][0]["lifetime"][0] and first.frame_end > first.frame_start
    out = os.path.join(out_dir, "effect.reexport.toml")
    assert bpy.ops.oag.export_effect(filepath=out) == {"FINISHED"}
    with open(out, "rb") as f:
        got = tomllib.load(f)
    assert got == want, "export gave a different effect than import took"
    # Edited in the Text Editor, then exported again.
    block = bpy.data.texts[cube[effectfile.PROP]]
    block.clear()
    old = want["emitter"][0]["duration"]
    new = old + 2
    text = effectfile.dumps(want)
    assert f"duration = {old}" in text, "the first emitter's duration is in the text"
    block.write(text.replace(f"duration = {old}", f"duration = {new}", 1))
    assert bpy.ops.oag.export_effect(filepath=out) == {"FINISHED"}
    with open(out, "rb") as f:
        edited = tomllib.load(f)
    assert edited["emitter"][0]["duration"] == new
    # A broken one is refused and writes nothing.
    block.clear()
    block.write("not toml [")
    try:
        bpy.ops.oag.export_effect(filepath=os.path.join(out_dir, "never.toml"))
        raise AssertionError("a broken effect was exported")
    except RuntimeError:
        pass
    assert not os.path.exists(os.path.join(out_dir, "never.toml"))
    print("mod effect: preview, export equal on import, text edit, broken refused: ok")


def props_checks(out_dir):
    """`mod_*` properties on pads, the spline and the scene, trigger Empties, make-trigger,
    export, import."""
    fresh()
    ctx = bpy.context
    make_loop(ctx)
    assert bpy.ops.oag.build_ribbon() == {"FINISHED"}
    assert bpy.ops.oag.add_start() == {"FINISHED"}
    ctx.scene.cursor.location = (900.0, 0.0, 0.0)
    assert bpy.ops.oag.add_pad(kind="speedup") == {"FINISHED"}
    pad = ctx.active_object if ctx.active_object and ctx.active_object.get("oag_kind") == "speedup" else next(
        o for o in ctx.scene.objects if o.get("oag_kind") == "speedup"
    )
    pad["mod_boost"] = 1.5
    pad["mod_armed"] = True
    ctx.scene["mod_laps_bonus"] = 3.0
    next(o for o in ctx.scene.objects if o.get("oag_kind") == "spline")["mod_pit"] = True
    bpy.ops.mesh.primitive_cube_add(size=2.0, location=(10.0, 20.0, 5.0))
    box = ctx.active_object
    box.name = "gate"
    box.scale = (3.0, 4.0, 1.0)
    for o in ctx.scene.objects:
        o.select_set(False)
    box.select_set(True)
    assert bpy.ops.oag.trigger_from_mesh() == {"FINISHED"}
    trigger = bpy.data.objects["oag/triggers/gate"]
    trigger["mod_points"] = 10.0
    path = os.path.join(out_dir, "props.track.toml")
    track = scene.write_track(ctx, path)
    assert [t.name for t in track.triggers] == ["gate"]
    assert track.triggers[0].props == {"points": 10.0}
    assert track.speedup_pads[0].props == {"boost": 1.5, "armed": 1.0}
    assert track.props == {"laps_bonus": 3.0}
    assert track.paths[0].props == {"pit": 1.0}
    t = track.triggers[0]
    assert t.min == (-3.0, -1.0, -4.0) and t.max == (3.0, 1.0, 4.0), (t.min, t.max)
    again = trackfile.read(path)
    assert again.triggers[0].props == t.props and again.props == track.props
    assert again.paths[0].props == track.paths[0].props
    fresh()
    scene.populate(bpy.context, again, out_dir)
    path2 = os.path.join(out_dir, "props.reexport.track.toml")
    track2 = scene.write_track(bpy.context, path2)
    assert track2.triggers[0].props == t.props and track2.speedup_pads[0].props == track.speedup_pads[0].props
    assert track2.props == track.props
    assert track2.paths[0].props == track.paths[0].props
    same = open(path).read() == open(path2).read().replace("props.reexport.glb", "props.glb")
    assert same, "the re-export differs from the export"
    print("track props: pad, spline and scene mod_*, trigger Empty, make-trigger-from-mesh, export, import, re-export: ok")


def channel_checks(out_dir):
    """Event channels: a trigger Empty's `oag_emit`, an animation's `oag_on` and the track's
    published channels survive export, import and a re-export; a trigger naming a channel the
    track does not publish, and a malformed name, are refused by name."""
    ctx = author_loop(out_dir, motion=True)
    channel = "test.gate/open"
    bpy.data.objects["airship"].oag_on = channel
    bpy.data.objects["airship"].oag_on_value = "2"
    # A gate over spline point 8 (about 500 units on, well clear of the grid): a box 120 wide
    # either way and 80 tall, so a craft driving the road passes through it.
    ahead = scene.to_blender(scene.collect(ctx).paths[0].points[8].pos)
    # The airship flies its loop above the gate (its keys fly 400 units about x = 0, y = 500, z = 60).
    airship = bpy.data.objects["airship"]
    airship.delta_location = (ahead[0], ahead[1] - 500.0, ahead[2] + 25.0 - 60.0)
    bpy.ops.mesh.primitive_cube_add(size=2.0, location=ahead)
    box = ctx.active_object
    box.name = "gate"
    box.scale = (60.0, 60.0, 40.0)
    for o in ctx.scene.objects:
        o.select_set(False)
    box.select_set(True)
    assert bpy.ops.oag.trigger_from_mesh() == {"FINISHED"}
    trigger = bpy.data.objects["oag/triggers/gate"]
    trigger.oag_emit = channel
    path = os.path.join(out_dir, "channels.track.toml")
    try:
        scene.write_track(ctx, path)
    except trackfile.TrackFileError as e:
        assert channel in str(e) and "publishes" in str(e), str(e)
    else:
        raise AssertionError("a trigger naming an unpublished channel was written")
    ctx.scene.oag_track_channels = channel
    track = scene.write_track(ctx, path)
    assert [t.emits for t in track.triggers] == [channel] and track.channels == [channel]
    assert [a.on for a in track.art.animations] == [channel, None], [a.on for a in track.art.animations]
    assert [a.on_value for a in track.art.animations] == [2.0, None], [a.on_value for a in track.art.animations]
    again = trackfile.read(path)
    assert [t.emits for t in again.triggers] == [channel] and again.channels == [channel]
    assert [a.on for a in again.art.animations] == [channel, None]
    assert [a.on_value for a in again.art.animations] == [2.0, None]
    fresh()
    scene.populate(bpy.context, again, out_dir)
    assert bpy.data.objects["oag/triggers/gate"].oag_emit == channel
    assert any(o.oag_on == channel for o in bpy.data.objects), "no imported object starts on the channel"
    assert any(o.oag_on_value == "2.0" for o in bpy.data.objects), "no imported object keeps its start value"
    assert bpy.context.scene.oag_track_channels == channel
    path2 = os.path.join(out_dir, "channels.reexport.track.toml")
    scene.write_track(bpy.context, path2)
    same = open(path).read() == open(path2).read().replace("channels.reexport.glb", "channels.glb")
    assert same, "the re-export differs from the export"
    bpy.data.objects["oag/triggers/gate"].oag_emit = "gate"
    try:
        scene.write_track(bpy.context, path2)
    except trackfile.TrackFileError as e:
        assert "'gate'" in str(e) and "<mod id>/<name>" in str(e), str(e)
    else:
        raise AssertionError("a malformed channel name was written")
    print("event channels: trigger oag_emit, animation oag_on, track publishes, export, import, re-export: ok")


def quad_with(ctx, name, image, x):
    """A one-quad art mesh at `x` along the track whose material samples `image`."""
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    tex = mat.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = image
    bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    mat.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    me = bpy.data.meshes.new(name)
    me.from_pydata([(x, -20.0, 30.0), (x + 40.0, -20.0, 30.0), (x + 40.0, 20.0, 30.0), (x, 20.0, 30.0)], [], [(0, 1, 2, 3)])
    me.uv_layers.new(name="UVMap")
    me.materials.append(mat)
    obj = bpy.data.objects.new(name, me)
    ctx.scene.collection.objects.link(obj)
    return obj


def solid_png(path, rgba, size=4):
    with open(path, "wb") as fh:
        fh.write(pngwrite.encode_rgba(size, size, bytes(rgba) * (size * size)))


def exported_bytes(path):
    """The first texel of an exported PNG, as the four bytes a game reads."""
    img = bpy.data.images.load(path, check_existing=False)
    px = img.pixels[:4]
    return tuple(int(v * 255 + 0.5) for v in px)


def texture_checks(out_dir):
    """Every own-image case an export has to survive (docs/tools/blender.md, "Textures"):
    each fails if the piece that handles it is dropped."""
    import shutil

    work = os.path.join(out_dir, "textures")
    shutil.rmtree(work, ignore_errors=True)
    os.makedirs(os.path.join(work, "a"))
    os.makedirs(os.path.join(work, "b"))
    ctx = author_loop(work)
    for o in [o for o in bpy.data.objects if o.type == "MESH" and o.get("oag_kind") is None]:
        bpy.data.objects.remove(o)
    ctx.scene.oag_lensflare = False
    ctx.scene.oag_skybox = False

    solid_png(os.path.join(work, "a", "rock.png"), (10, 20, 30, 255))
    solid_png(os.path.join(work, "b", "rock.png"), (200, 150, 100, 255))
    a = bpy.data.images.load(os.path.join(work, "a", "rock.png"))
    b = bpy.data.images.load(os.path.join(work, "b", "rock.png"))
    a.name, b.name = "rock_a", "rock_b"

    packed_src = os.path.join(work, "packed_src.png")
    solid_png(packed_src, (1, 2, 3, 255))
    packed_bytes = open(packed_src, "rb").read()
    packed = bpy.data.images.load(packed_src)
    packed.name = "packed_in_blend"
    packed.pack()
    os.remove(packed_src)  # nothing on disk: only the .blend holds it

    painted = bpy.data.images.new("painted", 4, 2)
    painted.pixels = [0.5, 0.25, 1.0, 1.0] * 8
    floaty = bpy.data.images.new("hdr", 2, 2, float_buffer=True)
    floaty.pixels = [0.214, 0.214, 0.214, 1.0] * 4
    data = bpy.data.images.new("normals", 2, 2, float_buffer=True)
    data.colorspace_settings.name = "Non-Color"
    data.pixels = [0.5, 0.5, 1.0, 1.0] * 4

    jpg_path = os.path.join(work, "photo.jpg")
    jpg = bpy.data.images.new("photo", 8, 8)
    jpg.pixels = [200 / 255, 100 / 255, 50 / 255, 1.0] * 64
    jpg.filepath_raw = jpg_path
    jpg.file_format = "JPEG"
    jpg.save()
    bpy.data.images.remove(jpg)
    jpg = bpy.data.images.load(jpg_path)
    jpg.name = "photo"
    tga_path = os.path.join(work, "skin.tga")
    tga = bpy.data.images.new("skin", 4, 4)
    tga.pixels = [0.5, 0.5, 0.5, 1.0] * 16
    tga.filepath_raw = tga_path
    tga.file_format = "TARGA"
    tga.save()
    bpy.data.images.remove(tga)
    tga = bpy.data.images.load(tga_path)
    tga.name = "skin"

    dirty_path = os.path.join(work, "dirty.png")
    solid_png(dirty_path, (255, 0, 0, 255))
    dirty = bpy.data.images.load(dirty_path)
    dirty.name = "dirty"
    dirty.pixels = [0.0, 1.0, 0.0, 1.0] * 16

    images = [a, b, packed, painted, floaty, data, jpg, tga, dirty, a]
    for i, image in enumerate(images):
        quad_with(ctx, f"q{i}", image, 100.0 + 60.0 * i)
    path = os.path.join(work, "tex.track.toml")
    track = scene.write_track(ctx, path)
    rows = [t.image for t in track.art.textures]
    by_object = {m.name: rows[m.texture] for m in track.art.meshes}
    name_of = {i.name: by_object[f"q{n}"] for n, i in enumerate(images)}
    print("texture rows:", name_of)

    assert all(t.host is None for t in track.art.textures)
    assert name_of["rock_a"] == "rock.png" and name_of["rock_b"] != "rock.png", "a name collision got one name"
    assert len(set(rows)) == len(rows) == len(images) - 1, f"the image used twice is one row: {rows}"
    assert by_object["q9"] == by_object["q0"], "one image, two meshes, two rows"
    f = lambda image: os.path.join(work, name_of[image])
    assert open(f("rock_a"), "rb").read() == open(os.path.join(work, "a", "rock.png"), "rb").read(), "a file PNG is copied as it is"
    assert exported_bytes(f("rock_a")) == (10, 20, 30, 255), exported_bytes(f("rock_a"))
    assert exported_bytes(f("rock_b")) == (200, 150, 100, 255), "the second rock.png lost to the first"
    assert open(f("packed_in_blend"), "rb").read() == packed_bytes, "a packed PNG is written out as it is"
    assert exported_bytes(f("painted")) == (128, 64, 255, 255), exported_bytes(f("painted"))
    assert exported_bytes(f("hdr")) in {(127, 127, 127, 255), (128, 128, 128, 255)}, f"float is linear, a PNG is sRGB: {exported_bytes(f('hdr'))}"
    assert exported_bytes(f("normals")) in {(127, 127, 255, 255), (128, 128, 255, 255)}, "data is written raw"
    jr, jg, jb, ja = exported_bytes(f("photo"))
    assert abs(jr - 200) <= 4 and abs(jg - 100) <= 4 and abs(jb - 50) <= 4, (jr, jg, jb)
    assert exported_bytes(f("skin")) in {(127, 127, 127, 255), (128, 128, 128, 255)}, "a TGA keeps its stored values"
    assert all(os.path.splitext(n)[1] == ".png" for n in rows)
    assert exported_bytes(f("dirty")) == (0, 255, 0, 255), "a PNG painted since it loaded was copied stale"
    for n in rows:
        assert pngwrite.is_png(open(os.path.join(work, n), "rb").read(8)), n
    again = trackfile.read(path)
    assert [t.image for t in again.art.textures] == rows

    gone = os.path.join(work, "gone.png")
    solid_png(gone, (9, 9, 9, 255))
    lost = bpy.data.images.load(gone)
    lost.name = "lost_image"
    os.remove(gone)
    quad_with(ctx, "lost_quad", lost, 900.0)
    stale = os.path.join(work, "refused.track.toml")
    try:
        scene.write_track(ctx, stale)
    except scene_art.ArtError as e:
        assert "lost_image" in str(e) and "gone.png" in str(e), str(e)
    else:
        raise AssertionError("an image whose file is gone was exported")
    assert not os.path.exists(stale), "a refused export wrote a manifest"
    print("own textures: file, packed, generated, float, data, JPG, TGA, painted, collision, reused, missing: ok")


def main():
    args = sys.argv[sys.argv.index("--") + 1 :]
    if args[:1] == ["loop"]:
        build_loop(args[1])
    elif args[:1] == ["motion"]:
        build_loop(args[1], motion=True)
    elif args[:1] == ["line"]:
        line_checks(args[1])
    elif args[:1] == ["effect"]:
        effect_checks(args[1], args[2])
    elif args[:1] == ["props"]:
        props_checks(args[1])
    elif args[:1] == ["channels"]:
        channel_checks(args[1])
    elif args[:1] == ["textures"]:
        texture_checks(args[1])
    elif args[:1] == ["reexport"]:
        reexport(args[1], args[2])
    else:
        raise SystemExit(__doc__)


main()
