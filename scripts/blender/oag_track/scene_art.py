"""Blender scene <-> the art section (ADR-0076; docs/tools/blender.md, "Art").

An **untagged** Mesh object that renders (`hide_render` off) is art: one draw per
material slot it uses, vertices split per face corner (a corner's normal, UV and
colour are its own). What becomes what:

- `COLOR_0`: the active colour attribute's `.color`, which Blender keeps linear
  (never `.color_srgb`); white when the mesh has none. This is the baked light.
- `TEXCOORD_0`: the active UV map, `v` flipped to the disc's v-down.
- Material: the first Image Texture node's image. An image with the custom
  properties `oag_host_entry` and `oag_host_name` is a **host** texture (a `Texture`
  node of a circuit on the host title's disc, by name: no pixels travel); any other
  image is the author's own PNG file, copied beside the manifest.
- Blend: the material's `oag_blend`, or from Blender's blend mode when it is
  `auto` (CLIP is `cutout` with `oag_alpha_ref` = alpha threshold x 255, BLEND is
  `alpha`, anything else `opaque`). `oag_culled` is the material's backface culling.
- Per object: `oag_shadow`, `oag_layer`, `oag_lit`, `oag_pad` (a pad plate: `speedup` or
  `weapon`, drawn with the host's pad models; *Place Pad Plates* makes them).
- The sun is a Sun lamp (the scene's `oag_sun`, else the only Sun lamp): its
  world +Z is the direction towards it, its colour, `oag_intensity` (not Blender's
  strength, which is irradiance for Cycles), its angle as the angular diameter in
  degrees, its shadow switch. Every other Sun lamp is an `[[art.light]]` row.
- `[art.ambient]` is the world's Background colour times its strength.

Imported records keep the file's own numbers (normals, colours, texcoords, a lamp's
direction) and write them back verbatim while they still match what Blender holds,
as pads do.
"""

import array
import math
import os
import re
import shutil

import bpy
from mathutils import Matrix, Vector

from . import f32, scene_motion
from .artfile import ArtError
from .model import Art, ArtMesh, DirectionalLight, LensFlare, Skybox, Sun, Texture

CACHE_TOLERANCE = 1e-5
HOST_ENTRY, HOST_NAME = "oag_host_entry", "oag_host_name"
COVER_LIFT = 5.0


def to_format(v):
    return (v[0], v[2], -v[1])


def to_blender(v):
    return (v[0], -v[2], v[1])


def _natural(name):
    return [int(t) if t.isdigit() else t for t in re.split(r"(\d+)", name)]


def _r3(v):
    return (f32.r(v[0]), f32.r(v[1]), f32.r(v[2]))


def _is_identity(m):
    return all(m[r][c] == (1.0 if r == c else 0.0) for r in range(4) for c in range(4))


def is_art(obj):
    return obj.type == "MESH" and obj.get("oag_kind") is None and not obj.hide_render


def is_caster(obj):
    return is_art(obj) and obj.oag_shadow in ("both", "cast")


def _image_node(mat):
    if mat is None or not mat.use_nodes or mat.node_tree is None:
        return None
    nodes = [n for n in mat.node_tree.nodes if n.type == "TEX_IMAGE" and n.image is not None]
    return nodes[0].image if nodes else None


def texture_of(image):
    """The `Texture` row an image stands for."""
    entry, name = image.get(HOST_ENTRY), image.get(HOST_NAME)
    if entry or name:
        if not (entry and name):
            raise ArtError(f"image {image.name}: a host texture needs {HOST_ENTRY} and {HOST_NAME}")
        return Texture(host=(str(entry), str(name)))
    path = bpy.path.abspath(image.filepath) if image.filepath else ""
    if not path or not os.path.isfile(path):
        raise ArtError(f"image {image.name}: save it as a PNG file first (no file at {path!r})")
    if not path.lower().endswith(".png"):
        raise ArtError(f"image {image.name}: {os.path.basename(path)} is not a PNG")
    return Texture(image=os.path.basename(path))


def alpha_clip(mat):
    """The alpha clip threshold in 0..1, or None. Blender 4.2 and later clip in the
    node tree (a Math node set to Greater Than feeding the shader's Alpha; setting
    `blend_method = 'CLIP'` is silently turned into HASHED on 5.2); earlier ones use
    the material's CLIP blend mode."""
    if mat.blend_method == "CLIP":
        return mat.alpha_threshold
    if not mat.use_nodes or mat.node_tree is None:
        return None
    for link in mat.node_tree.links:
        n = link.from_node
        if link.to_socket.name == "Alpha" and link.to_node.type == "BSDF_PRINCIPLED":
            if n.type == "MATH" and n.operation == "GREATER_THAN":
                return n.inputs[1].default_value
    return None


def _blend(mat):
    """The blend class and alpha-test reference. A material's custom property
    `oag_alpha_ref` (what the importer sets for a non-cutout draw) wins."""
    if mat is None:
        return "opaque", None
    chosen = mat.oag_blend
    clip = alpha_clip(mat)
    ref = None if clip is None else max(0, min(255, round(clip * 255)))
    if mat.get("oag_alpha_ref") is not None:
        ref = int(mat["oag_alpha_ref"])
    if chosen != "auto":
        return chosen, ref
    if clip is not None:
        return "cutout", ref
    if mat.blend_method == "BLEND":
        return "alpha", None
    return "opaque", None


def _colour_reader(me):
    attr = me.color_attributes.active_color
    if attr is None:
        return lambda loop, vert: (1.0, 1.0, 1.0, 1.0)
    n = len(attr.data)
    buf = array.array("f", bytes(16 * n))
    attr.data.foreach_get("color", buf)
    rows = [tuple(buf[i : i + 4]) for i in range(0, 4 * n, 4)]
    if attr.domain == "CORNER":
        return lambda loop, vert: rows[loop]
    return lambda loop, vert: rows[vert]


def _corner_attr(me, name, width):
    """A float vector attribute on face corners, as rows; None when absent."""
    a = me.attributes.get(name)
    if a is None or a.domain != "CORNER":
        return None
    n = len(a.data)
    buf = array.array("f", bytes(4 * width * n))
    a.data.foreach_get("vector", buf)
    return [tuple(buf[i : i + width]) for i in range(0, width * n, width)]


def _close(a, b):
    return all(abs(x - y) <= CACHE_TOLERANCE for x, y in zip(a, b))


def art_meshes(obj, depsgraph, textures, local=False):
    """One `ArtMesh` per material slot `obj` draws with. `textures` collects rows.

    `local`: the vertices in the object's own space (a moving object, whose pose is the
    node's), else in the world."""
    ev = obj.evaluated_get(depsgraph)
    me = ev.to_mesh()
    try:
        me.calc_loop_triangles()
        m = Matrix.Identity(4) if local else obj.matrix_world
        ident = _is_identity(m)
        normal_m = m.inverted_safe().transposed().to_3x3()
        n = len(me.vertices)
        co = array.array("f", bytes(12 * n))
        me.vertices.foreach_get("co", co)
        nl = len(me.loops)
        cn = array.array("f", bytes(12 * nl))
        me.corner_normals.foreach_get("vector", cn)
        uv_layer = me.uv_layers.active
        uv = None
        if uv_layer is not None:
            uv = array.array("f", bytes(8 * nl))
            uv_layer.uv.foreach_get("vector", uv)
        colour = _colour_reader(me)
        cached_n = _corner_attr(me, "oag_normal", 3) if ident else None
        cached_t = _corner_attr(me, "oag_texcoord", 3)

        def position(v):
            p = (co[3 * v], co[3 * v + 1], co[3 * v + 2])
            if not ident:
                w = m @ Vector(p)
                p = (w[0], w[1], w[2])
            return _r3(to_format(p))

        def normal(loop):
            d = (cn[3 * loop], cn[3 * loop + 1], cn[3 * loop + 2])
            if not ident:
                w = (normal_m @ Vector(d)).normalized()
                d = (w[0], w[1], w[2])
            d = _r3(to_format(d))
            if cached_n is not None:
                c = to_format(cached_n[loop])
                if _close(c, d):
                    return _r3(c)
            return d

        def texcoord(loop):
            if uv is None:
                return (0.0, 0.0)
            t = (f32.r(uv[2 * loop]), f32.r(1.0 - uv[2 * loop + 1]))
            if cached_t is not None and _close(cached_t[loop][:2], t):
                return _r3(cached_t[loop])[:2]
            return t

        draws = {}
        for tri in me.loop_triangles:
            draws.setdefault(tri.material_index, []).append(tri)
        out = []
        multi = len(draws) > 1
        for slot in sorted(draws):
            mat = obj.material_slots[slot].material if slot < len(obj.material_slots) else None
            image = _image_node(mat)
            texture = None
            if image is not None:
                row = texture_of(image)
                if row not in textures:
                    textures.append(row)
                texture = textures.index(row)
            blend, ref = _blend(mat)
            seen, pos, nor, tex, col, idx = {}, [], [], [], [], []
            for tri in draws[slot]:
                for loop, vert in zip(tri.loops, tri.vertices):
                    key = (position(vert), normal(loop), texcoord(loop), tuple(f32.r(c) for c in colour(loop, vert)))
                    at = seen.get(key)
                    if at is None:
                        at = seen[key] = len(pos)
                        pos.append(key[0])
                        nor.append(key[1])
                        tex.append(key[2])
                        col.append(key[3])
                    idx.append(at)
            name = obj.get("oag_name") or obj.name
            if multi:
                name = f"{name}/{mat.name if mat else slot}"
            out.append(
                ArtMesh(
                    name=name,
                    positions=pos,
                    normals=nor,
                    texcoords=tex,
                    colours=col,
                    indices=idx,
                    texture=texture,
                    blend=blend,
                    culled=bool(mat.use_backface_culling) if mat else True,
                    lit=bool(obj.oag_lit),
                    shadow=obj.oag_shadow,
                    alpha_ref=ref,
                    layer=int(obj.oag_layer),
                    pad=None if obj.oag_pad == "none" else obj.oag_pad,
                )
            )
        return out
    finally:
        ev.to_mesh_clear()


def _lamp_direction(obj):
    cache = obj.get("oag_direction")
    at = obj.get("oag_matrix_at")
    now = [obj.matrix_world[r][c] for r in range(4) for c in range(4)]
    if cache is not None and at is not None and all(abs(a - b) <= 1e-6 for a, b in zip(now, at)):
        return tuple(float(v) for v in cache)
    z = obj.matrix_world.to_3x3() @ Vector((0.0, 0.0, 1.0))
    return f32.unit(_r3(to_format(z.normalized())))


def _lamp_colour(obj):
    cache = obj.get("oag_colour")
    c = _r3(obj.data.color)
    if cache is not None and _close(cache, c):
        return tuple(float(v) for v in cache)
    return c


def sun_lamps(scene):
    return sorted(
        (o for o in scene.objects if o.type == "LIGHT" and o.data.type == "SUN" and not o.hide_render),
        key=lambda o: _natural(o.name),
    )


def the_sun(scene):
    lamps = sun_lamps(scene)
    if scene.oag_sun is not None and scene.oag_sun in lamps:
        return scene.oag_sun
    if len(lamps) == 1:
        return lamps[0]
    if lamps:
        raise ArtError("several Sun lamps: set the scene's OAG sun to the one that is the sun")
    return None


def _sun_record(obj):
    angle = obj.get("oag_angular_diameter")
    deg = f32.r(math.degrees(obj.data.angle))
    if angle is not None and abs(float(angle) - deg) <= 1e-3:
        deg = float(angle)
    return Sun(
        _lamp_direction(obj),
        _lamp_colour(obj),
        f32.r(obj.oag_intensity),
        deg,
        bool(obj.data.use_shadow),
    )


def ambient(scene):
    """The world Background node's colour times strength, linear; None without a world."""
    w = scene.world
    if w is None or not w.use_nodes or w.node_tree is None:
        return None
    for n in w.node_tree.nodes:
        if n.type == "BACKGROUND":
            cache = w.get("oag_ambient")
            c = n.inputs[0].default_value
            s = n.inputs[1].default_value
            v = _r3((c[0] * s, c[1] * s, c[2] * s))
            if cache is not None and _close(cache, v):
                return tuple(float(x) for x in cache)
            return v
    return None


def collect(context, depsgraph):
    """The `Art` the scene authors, or None when it authors none (a 1.0 file)."""
    scene = context.scene
    objs = sorted((o for o in scene.objects if is_art(o)), key=lambda o: (o.get("oag_art_index", 1 << 30), _natural(o.name)))
    sun_obj = the_sun(scene)
    if not objs and sun_obj is None and not scene.oag_sky and not scene.oag_skybox:
        return None
    keep = scene.frame_current
    try:
        moving = scene_motion.sample(scene, objs + ([sun_obj] if sun_obj is not None else []))
        textures, meshes, groups = [], [], []
        for o in objs:
            pose = moving.get(o)
            first = len(meshes)
            meshes += art_meshes(o, depsgraph, textures, local=pose is not None)
            if pose is None:
                continue
            times, trs = pose
            chans = []
            for n in range(first, len(meshes)):
                meshes[n].transform = trs[0]
                chans += scene_motion.channels(n, times, trs)
            groups.append((scene_motion.anim_name(o), o.oag_loop, chans))
        sun_rotation = None
        sun_pose = moving.get(sun_obj) if sun_obj is not None else None
        if sun_pose is not None:
            chans = scene_motion.channels("sun", *sun_pose, paths=("rotation",))
            if chans:
                sun_rotation = sun_pose[1][0][1]
                groups.append((scene_motion.anim_name(sun_obj), sun_obj.oag_loop, chans))
        try:
            animations = scene_motion.build(groups)
        except ValueError as e:
            raise ArtError(str(e)) from e
        sun = None if sun_obj is None else _sun_record(sun_obj)
    finally:
        scene.frame_set(keep)
    flare = None
    if scene.oag_lensflare:
        atlas = None
        if scene.oag_lensflare_atlas is not None:
            row = texture_of(scene.oag_lensflare_atlas)
            if row not in textures:
                textures.append(row)
            atlas = textures.index(row)
        over = {k: scene.get(f"oag_lensflare_{k}") for k in ("colour", "direction")}
        flare = LensFlare(atlas, *(None if v is None else tuple(float(c) for c in v) for v in over.values()))
    skybox = None
    if scene.oag_skybox:
        faces = []
        for i in range(6):
            image = getattr(scene, f"oag_sky_face_{i}")
            if image is None:
                faces.append(None)
                continue
            row = texture_of(image)
            if row not in textures:
                textures.append(row)
            faces.append(textures.index(row))
        gradient = scene.oag_sky_gradient
        skybox = Skybox(
            tuple(faces),
            _r3(tuple(scene.oag_sky_zenith)) if gradient else None,
            _r3(tuple(scene.oag_sky_horizon)) if gradient else None,
        )
    lights = []
    for o in sun_lamps(scene):
        if o is sun_obj:
            continue
        lights.append(DirectionalLight(_lamp_direction(o), _lamp_colour(o), f32.r(o.oag_intensity)))
    return Art(
        meshes=meshes,
        textures=textures,
        sky=scene.oag_sky or None,
        skybox=skybox,
        sun=sun,
        ambient=ambient(scene),
        lights=lights,
        lens_flare=flare,
        animations=animations,
        sun_rotation=sun_rotation,
        draw_collision=bool(scene.oag_draw_collision),
    )


def copy_images(art, scene, manifest_dir):
    """Copies each own-image row's file beside the manifest (the reader looks there)."""
    wanted = {t.image for t in art.textures if t.image is not None}
    for img in bpy.data.images:
        if img.get(HOST_ENTRY) or not img.filepath:
            continue
        src = bpy.path.abspath(img.filepath)
        base = os.path.basename(src)
        if base not in wanted:
            continue
        dst = os.path.join(manifest_dir, base)
        if os.path.abspath(src) != os.path.abspath(dst):
            shutil.copyfile(src, dst)
        wanted.discard(base)


def cover_light_scale(context, depsgraph, points, cover, sun_dir):
    """Per spline point light scale bytes, darkened where the line passes under art.

    **Chosen, not measured.** From a point lifted `COVER_LIFT` above the line, a ray
    straight up against art that casts: blocked, the point is covered and its
    ambient byte is `cover`, and so is its directional byte when a ray towards the
    sun is blocked too. An uncovered point stays 255 whatever shades it (a wall's
    shadow on a low sun is the bake's business, not the ship's). The two
    point-light bytes stay 255 (format 1.x carries no point light).
    """
    scene = context.scene

    def blocked(origin, direction):
        o = Vector(origin)
        d = Vector(direction).normalized()
        for _ in range(16):
            hit, loc, _n, _i, obj, _m = scene.ray_cast(depsgraph, o, d)
            if not hit:
                return False
            original = obj.original if hasattr(obj, "original") else obj
            if is_caster(original):
                return True
            o = loc + d * 0.01
        return False

    out = []
    for p in points:
        up = to_blender(tuple(-c for c in p.down))
        origin = Vector(to_blender(p.pos)) + Vector(up) * COVER_LIFT
        if not blocked(origin, up):
            out.append((255, 255, 255, 255))
            continue
        sun = cover if sun_dir is not None and blocked(origin, to_blender(sun_dir)) else 255
        out.append((cover, sun, 255, 255))
    return out


def populate(context, art, manifest_dir):
    """Objects, materials, lamps and scene settings for `art` (the importer)."""
    scene = context.scene
    from .scene import _link  # the track collection

    images = []
    for i, t in enumerate(art.textures):
        if t.host is not None:
            img = bpy.data.images.new(t.host[1].rsplit("\\", 1)[-1], 8, 8)
            img[HOST_ENTRY], img[HOST_NAME] = t.host
        else:
            path = os.path.join(manifest_dir, t.image)
            img = bpy.data.images.load(path, check_existing=True) if os.path.isfile(path) else bpy.data.images.new(t.image, 8, 8)
        images.append(img)
    objs = []
    for n, a in enumerate(art.meshes):
        me = bpy.data.meshes.new(f"oag/art/{n}")
        tris = [tuple(a.indices[i : i + 3]) for i in range(0, len(a.indices), 3)]
        me.from_pydata([to_blender(p) for p in a.positions], [], tris)
        loops = [v for t in tris for v in t]
        normals = me.attributes.new("oag_normal", "FLOAT_VECTOR", "CORNER")
        normals.data.foreach_set("vector", [c for v in loops for c in to_blender(a.normals[v])])
        me.normals_split_custom_set([Vector(to_blender(a.normals[v])) for v in loops])
        # A 3-vector, not FLOAT2: Blender takes any FLOAT2 corner attribute for a UV map.
        texc = me.attributes.new("oag_texcoord", "FLOAT_VECTOR", "CORNER")
        texc.data.foreach_set("vector", [c for v in loops for c in (*a.texcoords[v], 0.0)])
        uvs = me.uv_layers.new(name="UVMap")
        uvs.uv.foreach_set("vector", [c for v in loops for c in (a.texcoords[v][0], 1.0 - a.texcoords[v][1])])
        col = me.color_attributes.new("Col", "FLOAT_COLOR", "CORNER")
        col.data.foreach_set("color", [c for v in loops for c in a.colours[v]])
        me.color_attributes.active_color = col
        mat = bpy.data.materials.new(f"oag/art/{n}")
        mat.use_nodes = True
        mat.oag_blend = a.blend
        mat.use_backface_culling = a.culled
        if a.blend in ("alpha", "additive", "unblended"):
            mat.blend_method = "BLEND"
        bsdf = next(nd for nd in mat.node_tree.nodes if nd.type == "BSDF_PRINCIPLED")
        attr = mat.node_tree.nodes.new("ShaderNodeVertexColor")
        attr.layer_name = "Col"
        if a.texture is not None:
            tex = mat.node_tree.nodes.new("ShaderNodeTexImage")
            tex.image = images[a.texture]
            mix = mat.node_tree.nodes.new("ShaderNodeMix")
            mix.data_type = "RGBA"
            mix.blend_type = "MULTIPLY"
            mix.inputs["Factor"].default_value = 1.0
            mat.node_tree.links.new(tex.outputs["Color"], mix.inputs[6])
            mat.node_tree.links.new(attr.outputs["Color"], mix.inputs[7])
            mat.node_tree.links.new(mix.outputs[2], bsdf.inputs["Base Color"])
            if a.blend == "cutout" and a.alpha_ref is not None:
                clip = mat.node_tree.nodes.new("ShaderNodeMath")
                clip.operation = "GREATER_THAN"
                clip.inputs[1].default_value = a.alpha_ref / 255
                mat.node_tree.links.new(tex.outputs["Alpha"], clip.inputs[0])
                mat.node_tree.links.new(clip.outputs[0], bsdf.inputs["Alpha"])
            else:
                mat.node_tree.links.new(tex.outputs["Alpha"], bsdf.inputs["Alpha"])
        else:
            mat.node_tree.links.new(attr.outputs["Color"], bsdf.inputs["Base Color"])
        if a.alpha_ref is not None and (a.blend != "cutout" or a.texture is None):
            mat["oag_alpha_ref"] = a.alpha_ref
        me.materials.append(mat)
        obj = bpy.data.objects.new(f"oag/art/{n}", me)
        obj["oag_name"] = a.name
        obj["oag_art_index"] = n
        obj.oag_shadow = a.shadow
        obj.oag_layer = a.layer
        obj.oag_lit = a.lit
        obj.oag_pad = a.pad or "none"
        _link(scene, obj)
        objs.append(obj)
    context.view_layer.update()

    def lamp(name, direction, colour, intensity):
        data = bpy.data.lights.new(name, "SUN")
        data.color = colour
        obj = bpy.data.objects.new(name, data)
        _link(scene, obj)
        obj.rotation_mode = "QUATERNION"
        obj.rotation_quaternion = Vector(to_blender(direction)).to_track_quat("Z", "Y")
        obj.location = Vector(to_blender(direction)) * 1000.0
        obj.oag_intensity = intensity
        context.view_layer.update()
        obj["oag_direction"] = list(direction)
        obj["oag_colour"] = list(colour)
        obj["oag_matrix_at"] = [obj.matrix_world[r][c] for r in range(4) for c in range(4)]
        return obj

    sun_obj = None
    if art.sun is not None:
        s = art.sun
        obj = sun_obj = lamp("oag/sun", s.direction, s.colour, s.intensity)
        obj.data.angle = math.radians(s.angular_diameter)
        obj.data.use_shadow = s.shadows
        obj["oag_angular_diameter"] = s.angular_diameter
        scene.oag_sun = obj
    for i, l in enumerate(art.lights):
        lamp(f"oag/light/{i}", l.direction, l.colour, l.intensity)
    if art.ambient is not None:
        w = scene.world or bpy.data.worlds.new("World")
        scene.world = w
        w.use_nodes = True
        bg = next(nd for nd in w.node_tree.nodes if nd.type == "BACKGROUND")
        bg.inputs[0].default_value = (*art.ambient, 1.0)
        bg.inputs[1].default_value = 1.0
        w["oag_ambient"] = list(art.ambient)
    scene.oag_sky = art.sky or ""
    scene.oag_draw_collision = art.draw_collision
    scene.oag_skybox = art.skybox is not None
    if art.skybox is not None:
        sb = art.skybox
        for i, f in enumerate(sb.faces):
            setattr(scene, f"oag_sky_face_{i}", None if f is None else images[f])
        scene.oag_sky_gradient = sb.zenith is not None
        if sb.zenith is not None:
            scene.oag_sky_zenith = sb.zenith
            scene.oag_sky_horizon = sb.horizon
    if art.lens_flare is not None:
        scene.oag_lensflare = True
        if art.lens_flare.atlas is not None:
            scene.oag_lensflare_atlas = images[art.lens_flare.atlas]
        for key, v in (("colour", art.lens_flare.colour), ("direction", art.lens_flare.direction)):
            if v is not None:
                scene[f"oag_lensflare_{key}"] = list(v)
    scene_motion.apply(context, art, objs, sun_obj)


PLATE_LIFT = 0.3
# Chosen, not measured: what an untextured plate's baked colour is (blue speed, amber weapon).
PLATE_COLOURS = {"speedup": (0.1, 0.45, 1.0, 1.0), "weapon": (1.0, 0.45, 0.05, 1.0)}


def place_plates(context, material=None, lift=PLATE_LIFT):
    """A plate mesh on every pad Empty: a quad `lift` above the pad's origin over its box's
    lateral and forward extent, tagged `oag_pad`, receiving shadows, with `material` (a
    Material, or none: the kind's chosen colour as its baked light). Replaces the plates a
    previous run placed. Returns the plates."""
    from .scene import _kind, _link

    scene = context.scene
    for o in [o for o in scene.objects if o.get("oag_plate_of")]:
        bpy.data.objects.remove(o, do_unlink=True)
    plates = []
    pads = sorted((o for o in scene.objects if _kind(o) in ("speedup", "weapon")), key=lambda o: _natural(o.name))
    for pad in pads:
        kind = _kind(pad)
        (x0, _, y0), (x1, _, y1) = pad.oag_min, pad.oag_max
        me = bpy.data.meshes.new(f"plate {pad.name}")
        me.from_pydata([(x0, y0, lift), (x1, y0, lift), (x1, y1, lift), (x0, y1, lift)], [], [(0, 1, 2, 3)])
        uv = me.uv_layers.new(name="UVMap")
        for loop, co in zip(me.loops, ((0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0))):
            uv.data[loop.index].uv = co
        col = me.color_attributes.new("Col", "FLOAT_COLOR", "CORNER")
        for c in col.data:
            c.color = (1.0, 1.0, 1.0, 1.0) if material is not None else PLATE_COLOURS[kind]
        me.color_attributes.active_color = col
        if material is not None:
            me.materials.append(material)
        obj = bpy.data.objects.new(f"oag/plate/{pad.name}", me)
        _link(scene, obj)
        # The pad Empty's basis is unit (scale is ignored, the box is the size), so its
        # world matrix places the quad: local X lateral, Y forward, Z up.
        obj.matrix_world = pad.matrix_world.normalized()
        obj["oag_plate_of"] = pad.name
        obj.oag_pad = kind
        obj.oag_shadow = "receive"
        plates.append(obj)
    context.view_layer.update()
    return plates
