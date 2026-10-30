"""OpenAntiGrav track add-on: author a track in Blender, export the `.track.toml` + `.glb`.

The format is docs/formats/track-file.md; the page for modders is docs/tools/blender.md.
"""

import os

import bpy
from bpy.props import (
    BoolProperty,
    EnumProperty,
    FloatProperty,
    FloatVectorProperty,
    IntProperty,
    PointerProperty,
    StringProperty,
)
from bpy.types import Operator, Panel
from bpy_extras.io_utils import ExportHelper, ImportHelper

from . import artfile, derive, effectfile, model, scene, scene_art, trackfile

ERRORS = (trackfile.TrackFileError, derive.AuthoringError, artfile.ArtError)

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


class OAG_OT_compute_racing_line(Operator):
    bl_idname = "oag.compute_racing_line"
    bl_label = "Compute Racing Line"
    bl_description = (
        "Make (or replace) an editable curve of the AI racing line for the active spline: the "
        "minimum-curvature line inside the AI corridor. Edit its points; export reads them"
    )
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        obj = _active_spline(self, context)
        if obj is None:
            return {"CANCELLED"}
        try:
            line = scene.compute_racing_line(context, obj)
        except ERRORS as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        self.report({"INFO"}, f"{line.name}: {len(line.data.splines[0].points)} points")
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


class OAG_OT_add_pad_plates(Operator):
    bl_idname = "oag.add_pad_plates"
    bl_label = "Place Pad Plates"
    bl_description = (
        "A plate mesh on every pad: a quad slightly above the road over the pad's box, tagged as "
        "that pad's plate (a weapon plate is not drawn where weapons are off); replaces earlier plates"
    )
    bl_options = {"REGISTER", "UNDO"}

    material: StringProperty(name="Material", description="Material for every plate; empty: a chosen colour per kind")
    lift: FloatProperty(name="Lift", default=scene_art.PLATE_LIFT, min=0.0)

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self)

    def draw(self, context):
        self.layout.prop_search(self, "material", bpy.data, "materials")
        self.layout.prop(self, "lift")

    def execute(self, context):
        mat = bpy.data.materials.get(self.material) if self.material else None
        if self.material and mat is None:
            self.report({"ERROR"}, f"no material {self.material!r}")
            return {"CANCELLED"}
        plates = scene_art.place_plates(context, mat, self.lift)
        self.report({"INFO"}, f"{len(plates)} pad plate(s)")
        return {"FINISHED"}


class OAG_OT_host_texture(Operator):
    bl_idname = "oag.host_texture"
    bl_label = "Make Host Texture"
    bl_description = (
        "Turn the active material's image into a host reference: a Texture node of a circuit "
        "on the host title's disc, by entry and name (no pixels are exported)"
    )
    bl_options = {"REGISTER", "UNDO"}

    entry: StringProperty(name="Circuit entry", default="Data\\Environments\\03_Track\\track.vex")
    name: StringProperty(name="Texture name")

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self)

    def execute(self, context):
        image = scene_art._image_node(context.active_object.active_material)
        if image is None or not self.entry or not self.name:
            self.report({"ERROR"}, "an image texture, an entry and a name are all needed")
            return {"CANCELLED"}
        image[scene_art.HOST_ENTRY] = self.entry
        image[scene_art.HOST_NAME] = self.name
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
            track = scene.write_track(context, path)
        except ERRORS as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        self.report(
            {"INFO"},
            f"{os.path.basename(path)}: {sum(len(p.points) for p in track.paths)} spline points, "
            f"{len(track.nodes)} collision nodes, "
            f"{len(track.speedup_pads)} speedup + {len(track.weapon_pads)} weapon pads, "
            f"{len(track.art.meshes) if track.art else 0} art meshes",
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
            scene.populate(context, track, os.path.dirname(self.filepath))
        except (*ERRORS, OSError, KeyError, ValueError) as e:
            self.report({"ERROR"}, f"{self.filepath}: {e}")
            return {"CANCELLED"}
        return {"FINISHED"}


class OAG_OT_preview_effect(Operator, ImportHelper):
    bl_idname = "oag.preview_effect"
    bl_label = "Preview mod effect"
    bl_description = (
        "Read a mod effect TOML (schemas/v1/effect.tosd) onto the active object as an editable "
        "Text, and build particle systems from it that play in the viewport"
    )
    bl_options = {"REGISTER", "UNDO"}

    filename_ext = ".toml"
    filter_glob: StringProperty(default="*.toml", options={"HIDDEN"})

    def execute(self, context):
        obj = context.active_object
        if obj is None:
            self.report({"ERROR"}, "select the object the effect plays on")
            return {"CANCELLED"}
        try:
            with open(self.filepath, encoding="utf-8") as f:
                text = f.read()
            name = os.path.basename(self.filepath).removesuffix(".toml")
            effectfile.attach(obj, name, text)
            effectfile.preview(context, obj, text)
        except (effectfile.EffectError, OSError) as e:
            self.report({"ERROR"}, f"{self.filepath}: {e}")
            return {"CANCELLED"}
        return {"FINISHED"}


class OAG_OT_export_effect(Operator, ExportHelper):
    bl_idname = "oag.export_effect"
    bl_label = "Export mod effect"
    bl_description = "Write the active object's effect text back as a mod effect TOML"

    filename_ext = ".toml"
    filter_glob: StringProperty(default="*.toml", options={"HIDDEN"})

    def execute(self, context):
        obj = context.active_object
        try:
            data = effectfile.parse(effectfile.text_of(obj))
            with open(self.filepath, "w", encoding="utf-8", newline="\n") as f:
                f.write(effectfile.dumps(data))
        except (effectfile.EffectError, OSError, AttributeError) as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        return {"FINISHED"}


class OAG_OT_trigger_from_mesh(Operator):
    bl_idname = "oag.trigger_from_mesh"
    bl_label = "Make trigger from mesh"
    bl_description = "Turn each selected mesh's bounds into a trigger box (oag/triggers/<mesh name>)"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        meshes = [o for o in context.selected_objects if o.type == "MESH"]
        if not meshes:
            self.report({"ERROR"}, "select a mesh")
            return {"CANCELLED"}
        for obj in meshes:
            scene.trigger_from_bounds(context, obj)
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
        box.prop(sc, "oag_track_host")
        box.prop(sc, "oag_track_titles")
        box.prop(sc, "oag_track_notes")
        row = layout.row(align=True)
        row.operator("oag.import_track", text="Import", icon="IMPORT")
        row.operator("oag.export_track", text="Export", icon="EXPORT")

        row = layout.row(align=True)
        row.operator("oag.preview_effect", text="Preview effect", icon="PARTICLES")
        row.operator("oag.export_effect", text="Export effect")
        layout.operator("oag.trigger_from_mesh")

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
                layout.prop(obj, "oag_cover_light")
                layout.prop(obj, "oag_auto_racing_line")
            layout.prop(obj, "oag_wall_height")
            layout.operator("oag.build_ribbon")
            layout.operator("oag.compute_racing_line")
            layout.operator("oag.add_start")
            layout.operator("oag.add_pad", text="Add speedup pad").kind = "speedup"
            layout.operator("oag.add_pad", text="Add weapon pad").kind = "weapon"
            layout.operator("oag.add_pad_plates")
        elif kind == scene.LINE_KIND:
            layout.label(text="Racing line: edit the points sideways;")
            layout.label(text="export reads each across its spline point.")
        elif kind in ("speedup", "weapon"):
            layout.prop(obj, "oag_min")
            layout.prop(obj, "oag_max")
            layout.prop(obj, "oag_disabled")
            layout.label(text="mod_* custom properties go to simulation mods")
        elif kind == "trigger":
            layout.prop(obj, "oag_min")
            layout.prop(obj, "oag_max")
            layout.label(text="mod_* custom properties go to simulation mods")
        elif scene_art.is_art(obj):
            layout.label(text="Art mesh (untagged):")
            layout.prop(obj, "oag_shadow")
            layout.prop(obj, "oag_layer")
            layout.prop(obj, "oag_lit")
            layout.prop(obj, "oag_pad")
            if obj.animation_data is not None:
                layout.prop(obj, "oag_loop")
            mat = obj.active_material
            if mat is not None:
                layout.prop(mat, "oag_blend")
                layout.prop(mat, "use_backface_culling", text="Culled")
                image = scene_art._image_node(mat)
                if image is not None and scene_art.HOST_ENTRY in image:
                    layout.prop(image, f'["{scene_art.HOST_ENTRY}"]', text="Host entry")
                    layout.prop(image, f'["{scene_art.HOST_NAME}"]', text="Host name")
                elif image is not None:
                    layout.operator("oag.host_texture")
        elif obj.type == "LIGHT" and obj.data.type == "SUN":
            layout.prop(obj, "oag_intensity")
            layout.prop(obj.data, "angle", text="Angular diameter")
            layout.prop(obj.data, "use_shadow", text="Shadows")
            if obj.animation_data is not None:
                layout.prop(obj, "oag_loop")


class OAG_PT_art(Panel):
    bl_label = "Art"
    bl_idname = "OAG_PT_art"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = PANEL_CATEGORY
    bl_parent_id = "OAG_PT_track"

    def draw(self, context):
        layout = self.layout
        sc = context.scene
        layout.prop(sc, "oag_sun")
        layout.prop(sc, "oag_sky")
        layout.prop(sc, "oag_draw_collision")
        layout.prop(sc, "oag_skybox")
        if sc.oag_skybox:
            box = layout.box()
            box.prop(sc, "oag_sky_gradient")
            if sc.oag_sky_gradient:
                box.prop(sc, "oag_sky_zenith")
                box.prop(sc, "oag_sky_horizon")
            for i in range(6):
                box.prop(sc, f"oag_sky_face_{i}")
        layout.prop(sc, "oag_lensflare")
        if sc.oag_lensflare:
            layout.prop(sc, "oag_lensflare_atlas")
        layout.label(text="Ambient: the world's Background colour")


CLASSES = (
    OAG_OT_tag,
    OAG_OT_untag,
    OAG_OT_build_ribbon,
    OAG_OT_compute_racing_line,
    OAG_OT_add_start,
    OAG_OT_add_pad,
    OAG_OT_add_pad_plates,
    OAG_OT_host_texture,
    OAG_OT_export,
    OAG_OT_import,
    OAG_OT_preview_effect,
    OAG_OT_export_effect,
    OAG_OT_trigger_from_mesh,
    OAG_PT_track,
    OAG_PT_art,
)


def _props():
    sc, ob = bpy.types.Scene, bpy.types.Object
    sc.oag_track_id = StringProperty(name="Track id", description="Stable id; keys records and ghosts")
    sc.oag_track_title = StringProperty(name="Title")
    sc.oag_track_host = EnumProperty(
        name="Runs on",
        description="The title whose disc the art borrows host textures and the sky from (required when it does); "
        "another title races the track as a Race Remix with this disc as the track source",
        items=[("none", "none (borrows nothing)", "")] + [(h, h, "") for h in model.HOSTS],
        default="none",
    )
    sc.oag_track_titles = StringProperty(
        name="Offered on",
        description="Comma-separated title slugs the track is offered on (e.g. wipeout-pulse, wipeout-hd); empty is every title",
    )
    sc.oag_track_notes = StringProperty(name="Notes", description="Free text printed in the loader report")
    ob.oag_points = IntProperty(name="Spline points", default=64, min=3, max=4096)
    ob.oag_half_width = FloatProperty(name="Half width", default=derive.HALF_WIDTH, min=0.01)
    ob.oag_ai_fraction = FloatProperty(name="AI line fraction", default=derive.AI_FRACTION, min=0.0, max=1.0)
    ob.oag_wall_height = FloatProperty(name="Wall height", default=derive.WALL_HEIGHT, min=0.01)
    ob.oag_min = FloatVectorProperty(name="Box min (lateral, up, forward)", size=3, default=derive.PAD_BOX[0])
    ob.oag_max = FloatVectorProperty(name="Box max (lateral, up, forward)", size=3, default=derive.PAD_BOX[1])
    ob.oag_disabled = FloatProperty(name="Disabled", default=0.0)
    ob.oag_cover_light = IntProperty(
        name="Cover light",
        description="Light scale byte where art casts over the line (255: off, the spline stays fully lit)",
        default=255,
        min=0,
        max=255,
    )
    ob.oag_auto_racing_line = BoolProperty(
        name="Compute racing line",
        description="With no racing-line curve, export computes the AI line (minimum curvature "
        "inside the AI corridor). Off: the line stays on the centre",
        default=True,
    )
    ob.oag_shadow = EnumProperty(
        name="Shadow",
        items=[(k, k, "") for k in ("both", "cast", "receive", "none")],
        default="both",
    )
    ob.oag_layer = IntProperty(name="Layer", description="Render layer: draws go by layer, then file order", default=0, min=0)
    ob.oag_lit = BoolProperty(name="Lit", description="The light rig applies on top of the baked colour", default=False)
    ob.oag_pad = EnumProperty(
        name="Pad plate",
        description="Drawn with the host's pad models instead of the track (a weapon plate is not drawn "
        "where weapons are off); visual only, the pad itself is the pad Empty",
        items=[("none", "none", ""), ("speedup", "speedup", ""), ("weapon", "weapon", "")],
        default="none",
    )
    ob.oag_loop = EnumProperty(
        name="Motion",
        description="Past its last key the object's animation loops or holds (glTF oag_loop); objects "
        "sharing an oag_anim name share one animation",
        items=[("loop", "loop", ""), ("clamp", "clamp", "")],
        default="loop",
    )
    ob.oag_intensity = FloatProperty(
        name="Intensity",
        description="The file's intensity (a multiplier on the colour for the ships' light), not Blender's strength",
        default=1.0,
        min=0.0,
    )
    bpy.types.Material.oag_blend = EnumProperty(
        name="Blend",
        items=[("auto", "auto (from blend mode)", "")] + [(k, k, "") for k in ("opaque", "cutout", "alpha", "additive", "unblended")],
        default="auto",
    )
    sc.oag_sun = PointerProperty(
        name="Sun",
        type=bpy.types.Object,
        poll=lambda self, o: o.type == "LIGHT" and o.data.type == "SUN",
    )
    sc.oag_sky = StringProperty(name="Sky", description="A host circuit's .vex entry whose Skycube is drawn")
    sc.oag_draw_collision = BoolProperty(
        name="Draw collision",
        description="Draw the flat-shaded floor and walls under the art ([art] draw_collision), for art "
        "that does not cover the road",
        default=False,
    )
    sc.oag_skybox = BoolProperty(
        name="Own sky",
        description="The track's own sky: a cube of the author's images and/or a zenith-to-horizon gradient "
        "(never together with a host Sky)",
        default=False,
    )
    sc.oag_sky_gradient = BoolProperty(name="Gradient", default=True)
    sc.oag_sky_zenith = FloatVectorProperty(
        name="Zenith", subtype="COLOR", size=3, min=0.0, default=(0.18, 0.36, 0.75)
    )
    sc.oag_sky_horizon = FloatVectorProperty(
        name="Horizon", subtype="COLOR", size=3, min=0.0, default=(0.75, 0.82, 0.9)
    )
    for i, face in enumerate(("+X", "-X", "+Y (zenith)", "-Y (nadir)", "+Z", "-Z")):
        setattr(
            sc,
            f"oag_sky_face_{i}",
            PointerProperty(name=f"Face {face}", type=bpy.types.Image, description="Empty draws the gradient"),
        )
    sc.oag_lensflare = BoolProperty(name="Lens flare", default=False)
    sc.oag_lensflare_atlas = PointerProperty(name="Flare atlas", type=bpy.types.Image)


def _unprops():
    for name in (
        "oag_track_id",
        "oag_track_title",
        "oag_track_host",
        "oag_track_titles",
        "oag_track_notes",
        "oag_sun",
        "oag_sky",
        "oag_draw_collision",
        "oag_skybox",
        "oag_sky_gradient",
        "oag_sky_zenith",
        "oag_sky_horizon",
        *(f"oag_sky_face_{i}" for i in range(6)),
        "oag_lensflare",
        "oag_lensflare_atlas",
    ):
        delattr(bpy.types.Scene, name)
    del bpy.types.Material.oag_blend
    for name in (
        "oag_points",
        "oag_half_width",
        "oag_ai_fraction",
        "oag_wall_height",
        "oag_min",
        "oag_max",
        "oag_disabled",
        "oag_cover_light",
        "oag_auto_racing_line",
        "oag_shadow",
        "oag_layer",
        "oag_lit",
        "oag_loop",
        "oag_pad",
        "oag_intensity",
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
