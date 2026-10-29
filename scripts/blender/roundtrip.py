"""Headless driver for the add-on (docs/tools/blender.md, `just blender-fixtures`).

    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- loop <out-dir>
    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- reexport <in.track.toml> <out-dir>

`loop` builds a track from scratch (8 control points, floor, walls, start, one pad
of each kind, and art: a canopy over the line textured with a generated checker,
a sun, a fill light, an ambient, a sky and a lens flare) and exports
`blender_loop.track.toml` with `blender_checker.png` beside it. The checker is
written here, byte for byte: never disc pixels. `reexport` imports a track
file with the add-on's importer and exports it again as `<stem>.reexport.track.toml`.
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
from mathutils import Vector  # noqa: E402

import oag_track  # noqa: E402
from oag_track import scene, trackfile  # noqa: E402

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
    ctx.scene.oag_sky = "Data\\Environments\\03_Track\\track.vex"
    ctx.scene.oag_lensflare = True
    ctx.scene.oag_lensflare_atlas = image


def fresh():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    oag_track.register()


def build_loop(out_dir):
    fresh()
    ctx = bpy.context
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
    ctx.scene.oag_track_id = "blender-loop"
    ctx.scene.oag_track_title = "Blender Loop"
    ctx.scene.oag_track_host = "wipeout-pulse"
    ctx.scene.oag_track_notes = "authored headless in Blender with the OAG add-on: 8 control points (chosen, not measured)"
    assert bpy.ops.oag.build_ribbon() == {"FINISHED"}
    assert bpy.ops.oag.add_start() == {"FINISHED"}
    ctx.scene.cursor.location = (900.0, 0.0, 0.0)
    assert bpy.ops.oag.add_pad(kind="speedup") == {"FINISHED"}
    ctx.scene.cursor.location = (-900.0, 0.0, 0.0)
    assert bpy.ops.oag.add_pad(kind="weapon") == {"FINISHED"}
    add_art(ctx, out_dir)
    obj.oag_cover_light = 96
    export(os.path.join(out_dir, "blender_loop.track.toml"))


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


def main():
    args = sys.argv[sys.argv.index("--") + 1 :]
    if args[:1] == ["loop"]:
        build_loop(args[1])
    elif args[:1] == ["reexport"]:
        reexport(args[1], args[2])
    else:
        raise SystemExit(__doc__)


main()
