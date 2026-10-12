"""OpenAntiGrav track add-on: author a track in Blender, export the `.track.toml` + `.glb`.

The format is docs/formats/track-file.md; the page for modders is docs/tools/blender.md.
"""

import os

import bpy
from bpy.props import (
    EnumProperty,
    FloatProperty,
    FloatVectorProperty,
    IntProperty,
    StringProperty,
)
from bpy.types import Operator, Panel
from bpy_extras.io_utils import ExportHelper, ImportHelper

from . import derive, scene, trackfile

PANEL_CATEGORY = "OAG"
KIND_ITEMS = [(k, k, "") for k in scene.KINDS]


class OAG_OT_tag(Operator):
    bl_idname = "oag.tag"
    bl_label = "Tag Selected"
    bl_description = "Set oag_kind on the selected objects"
    bl_options = {"REGISTER", "UNDO"}

    kind: EnumProperty(name="Kind", items=KIND_ITEMS)

    def execute(self, context):
        for obj in context.selected_objects:
            obj["oag_kind"] = self.kind
        return {"FINISHED"}


class OAG_OT_untag(Operator):
    bl_idname = "oag.untag"
    bl_label = "Untag Selected"
    bl_description = "Remove oag_kind from the selected objects (they are not exported)"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        for obj in context.selected_objects:
            obj.pop("oag_kind", None)
        return {"FINISHED"}


def _active_spline(self, context):
    obj = context.active_object
    if obj is None or scene._kind(obj) != "spline":
        self.report({"ERROR"}, "Make a spline-tagged object active")
        return None
    return obj


class OAG_OT_build_ribbon(Operator):
    bl_idname = "oag.build_ribbon"
    bl_label = "Build Floor and Walls"
    bl_description = "Make tagged floor and wall meshes along the active spline"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        obj = _active_spline(self, context)
        if obj is None:
            return {"CANCELLED"}
        try:
            scene.build_ribbon(context, obj)
        except (trackfile.TrackFileError, derive.AuthoringError) as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        return {"FINISHED"}


class OAG_OT_add_start(Operator):
    bl_idname = "oag.add_start"
    bl_label = "Add Start"
    bl_description = "Place the start slot 154 units behind spline point 0 (the lap line)"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        obj = _active_spline(self, context)
        if obj is None:
            return {"CANCELLED"}
        try:
            frame = scene.start_from(context, obj)
        except (trackfile.TrackFileError, derive.AuthoringError) as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        scene.make_start_object(context, frame)
        return {"FINISHED"}


class OAG_OT_add_pad(Operator):
    bl_idname = "oag.add_pad"
    bl_label = "Add Pad"
    bl_description = "Place a pad on the active spline, at the point nearest the 3D cursor"
    bl_options = {"REGISTER", "UNDO"}

    kind: EnumProperty(name="Kind", items=[("speedup", "speedup", ""), ("weapon", "weapon", "")])

    def execute(self, context):
        obj = _active_spline(self, context)
        if obj is None:
            return {"CANCELLED"}
        try:
            scene.add_pad_at(context, obj, self.kind, context.scene.cursor.location)
        except (trackfile.TrackFileError, derive.AuthoringError) as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        return {"FINISHED"}


class OAG_OT_export(Operator, ExportHelper):
    bl_idname = "oag.export_track"
    bl_label = "Export OAG Track"
    bl_description = "Write <name>.track.toml and <name>.glb"

    filename_ext = ".track.toml"
    filter_glob: StringProperty(default="*.track.toml", options={"HIDDEN"})

    def execute(self, context):
        path = self.filepath
        if not path.endswith(".track.toml"):
            path = path.removesuffix(".toml").removesuffix(".track") + ".track.toml"
        try:
            track = scene.collect(context)
            trackfile.write(track, path)
        except (trackfile.TrackFileError, derive.AuthoringError) as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        self.report(
            {"INFO"},
            f"{os.path.basename(path)}: {sum(len(p.points) for p in track.paths)} spline points, "
            f"{len(track.nodes)} collision nodes, "
            f"{len(track.speedup_pads)} speedup + {len(track.weapon_pads)} weapon pads",
        )
        return {"FINISHED"}


class OAG_OT_import(Operator, ImportHelper):
    bl_idname = "oag.import_track"
    bl_label = "Import OAG Track"
    bl_description = "Read a .track.toml and its .glb into tagged Blender objects"

    filename_ext = ".track.toml"
    filter_glob: StringProperty(default="*.track.toml", options={"HIDDEN"})

    def execute(self, context):
        try:
            track = trackfile.read(self.filepath)
            scene.populate(context, track)
        except (trackfile.TrackFileError, OSError, KeyError, ValueError) as e:
            self.report({"ERROR"}, f"{self.filepath}: {e}")
            return {"CANCELLED"}
        return {"FINISHED"}


class OAG_PT_track(Panel):
    bl_label = "OpenAntiGrav Track"
    bl_idname = "OAG_PT_track"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = PANEL_CATEGORY

    def draw(self, context):
        layout = self.layout
        sc = context.scene
        box = layout.box()
        box.prop(sc, "oag_track_id")
        box.prop(sc, "oag_track_title")
        box.prop(sc, "oag_track_notes")
        row = layout.row(align=True)
        row.operator("oag.import_track", text="Import", icon="IMPORT")
        row.operator("oag.export_track", text="Export", icon="EXPORT")

        layout.label(text="Tag selected objects:")
        col = layout.column(align=True)
        for kind in scene.KINDS:
            col.operator("oag.tag", text=kind).kind = kind
        layout.operator("oag.untag")

        obj = context.active_object
        if obj is None:
            return
        kind = scene._kind(obj)
        layout.separator()
        layout.label(text=f"{obj.name}: {kind or 'not tagged (not exported)'}")
        if kind == "spline":
            if obj.type == "CURVE":
                layout.prop(obj, "oag_points")
                layout.prop(obj, "oag_half_width")
                layout.prop(obj, "oag_ai_fraction")
            layout.prop(obj, "oag_wall_height")
            layout.operator("oag.build_ribbon")
            layout.operator("oag.add_start")
            layout.operator("oag.add_pad", text="Add speedup pad").kind = "speedup"
            layout.operator("oag.add_pad", text="Add weapon pad").kind = "weapon"
        elif kind in ("speedup", "weapon"):
            layout.prop(obj, "oag_min")
            layout.prop(obj, "oag_max")
            layout.prop(obj, "oag_disabled")


CLASSES = (
    OAG_OT_tag,
    OAG_OT_untag,
    OAG_OT_build_ribbon,
    OAG_OT_add_start,
    OAG_OT_add_pad,
    OAG_OT_export,
    OAG_OT_import,
    OAG_PT_track,
)


def _props():
    sc, ob = bpy.types.Scene, bpy.types.Object
    sc.oag_track_id = StringProperty(name="Track id", description="Stable id; keys records and ghosts")
    sc.oag_track_title = StringProperty(name="Title")
    sc.oag_track_notes = StringProperty(name="Notes", description="Free text printed in the loader report")
    ob.oag_points = IntProperty(name="Spline points", default=64, min=3, max=4096)
    ob.oag_half_width = FloatProperty(name="Half width", default=derive.HALF_WIDTH, min=0.01)
    ob.oag_ai_fraction = FloatProperty(name="AI line fraction", default=derive.AI_FRACTION, min=0.0, max=1.0)
    ob.oag_wall_height = FloatProperty(name="Wall height", default=derive.WALL_HEIGHT, min=0.01)
    ob.oag_min = FloatVectorProperty(name="Box min (lateral, up, forward)", size=3, default=derive.PAD_BOX[0])
    ob.oag_max = FloatVectorProperty(name="Box max (lateral, up, forward)", size=3, default=derive.PAD_BOX[1])
    ob.oag_disabled = FloatProperty(name="Disabled", default=0.0)


def _unprops():
    for name in ("oag_track_id", "oag_track_title", "oag_track_notes"):
        delattr(bpy.types.Scene, name)
    for name in (
        "oag_points",
        "oag_half_width",
        "oag_ai_fraction",
        "oag_wall_height",
        "oag_min",
        "oag_max",
        "oag_disabled",
    ):
        delattr(bpy.types.Object, name)


def register():
    _props()
    for cls in CLASSES:
        bpy.utils.register_class(cls)


def unregister():
    for cls in reversed(CLASSES):
        bpy.utils.unregister_class(cls)
    _unprops()
