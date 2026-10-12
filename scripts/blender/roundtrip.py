"""Headless driver for the add-on (docs/tools/blender.md, `just blender-fixtures`).

    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- loop <out-dir>
    blender -b --factory-startup --python-exit-code 1 --python roundtrip.py -- reexport <in.track.toml> <out-dir>

`loop` builds a track from scratch (8 control points, floor, walls, start, one pad
of each kind) and exports `blender_loop.track.toml`. `reexport` imports a track
file with the add-on's importer and exports it again as `<stem>.reexport.track.toml`.
The add-on is registered from this directory, never installed: nothing is written
to the Blender user directory.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import bpy  # noqa: E402

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
    ctx.scene.oag_track_notes = "authored headless in Blender with the OAG add-on: 8 control points (chosen, not measured)"
    assert bpy.ops.oag.build_ribbon() == {"FINISHED"}
    assert bpy.ops.oag.add_start() == {"FINISHED"}
    ctx.scene.cursor.location = (900.0, 0.0, 0.0)
    assert bpy.ops.oag.add_pad(kind="speedup") == {"FINISHED"}
    ctx.scene.cursor.location = (-900.0, 0.0, 0.0)
    assert bpy.ops.oag.add_pad(kind="weapon") == {"FINISHED"}
    export(os.path.join(out_dir, "blender_loop.track.toml"))


def export(path):
    track = scene.collect(bpy.context)
    trackfile.write(track, path)
    print(f"wrote {path}: {len(track.paths[0].points)} points, {len(track.nodes)} nodes")


def reexport(src, out_dir):
    fresh()
    track = trackfile.read(src)
    scene.populate(bpy.context, track)
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
