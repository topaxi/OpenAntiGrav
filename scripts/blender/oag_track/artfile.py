"""The art section of a track file (ADR-0076, docs/formats/track-file.md), with no Blender.

The reader of record is `oag_trackfile` (`crates/trackfile/src/glb/art.rs` and
`art.rs`): this writes what it reads. Meshes are `oag/art/<n>` glb nodes, one
`TRIANGLES` primitive each, `u32` indices, `POSITION`, `NORMAL`, `TEXCOORD_0`
and a float VEC4 `COLOR_0` (linear RGB, coverage alpha), with the material as node
extras and no node transform or glTF material. Everything else is the manifest's
`[art]` table, its floats written as the shortest decimal that reads back as the
same `f32`.
"""

import json
import struct

from . import f32, motionfile
from .model import (
    BLENDS,
    SHADOW_ROLES,
    Art,
    ArtMesh,
    DirectionalLight,
    PAD_PLATES,
    LensFlare,
    Skybox,
    Sun,
    Texture,
)

PREFIX = "oag/art/"
UINT = 5125
UNIT_TOLERANCE = 1e-3


class ArtError(Exception):
    """Art the format cannot hold."""


def _finite(what, values):
    for v in values:
        if v != v or v in (float("inf"), float("-inf")):
            raise ArtError(f"{what}: a non-finite number")


def _unit(what, d):
    n = sum(c * c for c in d) ** 0.5
    if abs(n - 1.0) > UNIT_TOLERANCE:
        raise ArtError(f"{what}: direction has length {n}, not 1")


def validate(art):
    """The checks `Art::validate` makes, so a bad file is refused at export, by name."""
    for i, t in enumerate(art.textures):
        if (t.image is None) == (t.host is None):
            raise ArtError(f"art texture {i}: exactly one of image or host")
        if t.image is not None and not t.image:
            raise ArtError(f"art texture {i}: an empty image file name")
        if t.host is not None and not all(t.host):
            raise ArtError(f"art texture {i}: a host texture needs an entry and a name")
    for n, m in enumerate(art.meshes):
        what = f"art mesh {n} ({m.name})"
        v = len(m.positions)
        for name, rows in (("normals", m.normals), ("texcoords", m.texcoords), ("colours", m.colours)):
            if len(rows) != v:
                raise ArtError(f"{what}: {len(rows)} {name}, {v} positions")
        if not v or not m.indices or len(m.indices) % 3:
            raise ArtError(f"{what}: empty, or indices not whole triangles")
        if max(m.indices) >= v:
            raise ArtError(f"{what}: an index is past the vertices")
        for rows in (m.positions, m.normals, m.texcoords, m.colours):
            _finite(what, (c for r in rows for c in r))
        if m.texture is not None and not 0 <= m.texture < len(art.textures):
            raise ArtError(f"{what}: texture {m.texture}, the file lists {len(art.textures)}")
        if m.blend not in BLENDS:
            raise ArtError(f"{what}: unknown blend {m.blend!r}")
        if m.shadow not in SHADOW_ROLES:
            raise ArtError(f"{what}: unknown shadow role {m.shadow!r}")
        if m.alpha_ref is not None and not 0 <= m.alpha_ref <= 255:
            raise ArtError(f"{what}: alpha ref {m.alpha_ref} is not in 0..=255")
        if m.pad is not None and m.pad not in PAD_PLATES:
            raise ArtError(f"{what}: oag_pad {m.pad!r} is not speedup or weapon")
        if not 0 <= m.layer < 2**32:
            raise ArtError(f"{what}: layer {m.layer} is not an unsigned 32-bit integer")
    if art.skybox is not None:
        sb = art.skybox
        if art.sky is not None:
            raise ArtError("art.skybox: a track draws one sky, the host form (sky) or its own, not both")
        if (sb.zenith is None) != (sb.horizon is None):
            raise ArtError("art.skybox: the gradient needs both zenith and horizon")
        for i, f in enumerate(sb.faces):
            if f is None and sb.zenith is None:
                raise ArtError(f"art.skybox.faces[{i}]: a face with no texture needs the gradient")
            if f is not None and not 0 <= f < len(art.textures):
                raise ArtError(f"art.skybox.faces[{i}]: texture {f}, the file lists {len(art.textures)}")
        for c in (sb.zenith or ()) + (sb.horizon or ()):
            if c < 0:
                raise ArtError("art.skybox: a negative colour")
    lights = [("art sun", art.sun)] if art.sun else []
    lights += [(f"art light {i}", l) for i, l in enumerate(art.lights)]
    for what, l in lights:
        _finite(what, (*l.direction, *l.colour, l.intensity))
        _unit(what, l.direction)
        if min(l.colour) < 0 or l.intensity < 0:
            raise ArtError(f"{what}: a negative colour or intensity")
    if art.sun and not 0.0 <= art.sun.angular_diameter <= 180.0:
        raise ArtError(f"art sun: angular diameter {art.sun.angular_diameter} is not in 0..=180")
    if art.ambient is not None and min(art.ambient) < 0:
        raise ArtError("art ambient: a negative colour")
    f = art.lens_flare
    if f is not None:
        if f.direction is not None:
            _unit("art lensflare", f.direction)
        if f.atlas is not None and not 0 <= f.atlas < len(art.textures):
            raise ArtError(f"art lensflare: texture {f.atlas}, the file lists {len(art.textures)}")
        if (f.colour is None or f.direction is None) and art.sun is None:
            raise ArtError("art lensflare: a flare lies along the sun's direction and the file has no sun")


def write_nodes(b, art, push):
    """Appends the `oag/art/<n>` nodes to a glb under construction (`trackfile._Bin`)
    and answers their node indices."""
    validate(art)
    out = []
    for n, m in enumerate(art.meshes):
        attrs = {
            "POSITION": b.positions(m.positions),
            "NORMAL": b.floats(m.normals, "VEC3"),
            "TEXCOORD_0": b.floats(m.texcoords, "VEC2"),
            "COLOR_0": b.floats(m.colours, "VEC4"),
        }
        idx = list(m.indices)
        indices = b.add(struct.pack(f"<{len(idx)}I", *idx), UINT, "SCALAR", len(idx), 34963)
        extras = {
            "oag_name": m.name,
            "oag_blend": m.blend,
            "oag_culled": bool(m.culled),
            "oag_lit": bool(m.lit),
            "oag_shadow": m.shadow,
        }
        if m.texture is not None:
            extras["oag_texture"] = m.texture
        if m.alpha_ref is not None:
            extras["oag_alpha_ref"] = m.alpha_ref
        if m.layer:
            extras["oag_layer"] = m.layer
        if m.pad is not None:
            extras["oag_pad"] = m.pad
        out.append(push(f"{PREFIX}{n}", [{"attributes": attrs, "indices": indices, "mode": 4}], extras))
    return out


def _vec(v):
    return "[" + ", ".join(f32.text(c) for c in v) + "]"


def _q(s):
    return json.dumps(s, ensure_ascii=False)


def manifest_lines(art):
    """The manifest's `[art]` tables, after `[counts]`."""
    out = ["[art]"]
    if art.sky is not None:
        out.append(f"sky = {_q(art.sky)}")
    if art.ambient is not None:
        out.append(f"ambient = {_vec(art.ambient)}")
    if art.skybox is not None:
        sb = art.skybox
        out += ["", "[art.skybox]"]
        if any(f is not None for f in sb.faces):
            out.append("faces = [" + ", ".join(str(-1 if f is None else f) for f in sb.faces) + "]")
        if sb.zenith is not None:
            out.append(f"zenith = {_vec(sb.zenith)}")
        if sb.horizon is not None:
            out.append(f"horizon = {_vec(sb.horizon)}")
    if art.sun is not None:
        s = art.sun
        out += [
            "",
            "[art.sun]",
            f"direction = {_vec(s.direction)}",
            f"colour = {_vec(s.colour)}",
            f"intensity = {f32.text(s.intensity)}",
            f"angular_diameter = {f32.text(s.angular_diameter)}",
            f"shadows = {'true' if s.shadows else 'false'}",
        ]
    if art.lens_flare is not None:
        f = art.lens_flare
        out += ["", "[art.lensflare]"]
        if f.atlas is not None:
            out.append(f"atlas = {f.atlas}")
        if f.colour is not None:
            out.append(f"colour = {_vec(f.colour)}")
        if f.direction is not None:
            out.append(f"direction = {_vec(f.direction)}")
    for l in art.lights:
        out += [
            "",
            "[[art.light]]",
            'kind = "directional"',
            f"direction = {_vec(l.direction)}",
            f"colour = {_vec(l.colour)}",
            f"intensity = {f32.text(l.intensity)}",
        ]
    for t in art.textures:
        out += ["", "[[art.texture]]"]
        if t.image is not None:
            out.append(f"image = {_q(t.image)}")
        else:
            out.append(f"host = {{ entry = {_q(t.host[0])}, name = {_q(t.host[1])} }}")
    return out + [""]


def _floats3(v):
    return tuple(f32.r(float(c)) for c in v)


def from_manifest(table, meshes):
    """An `Art` from the manifest's `[art]` table and the meshes read out of the glb."""
    table = table or {}
    sun = table.get("sun")
    flare = table.get("lensflare")
    textures = []
    for t in table.get("texture", []):
        if "image" in t:
            textures.append(Texture(image=t["image"]))
        else:
            textures.append(Texture(host=(t["host"]["entry"], t["host"]["name"])))
    lights = []
    for l in table.get("light", []):
        if l.get("kind") != "directional":
            raise ArtError(f"art light: unknown kind {l.get('kind')!r}")
        lights.append(
            DirectionalLight(_floats3(l["direction"]), _floats3(l["colour"]), f32.r(float(l["intensity"])))
        )
    return Art(
        meshes=meshes,
        textures=textures,
        sky=table.get("sky"),
        skybox=None
        if "skybox" not in table
        else Skybox(
            tuple(None if f == -1 else f for f in table["skybox"].get("faces", [-1] * 6)),
            None if "zenith" not in table["skybox"] else _floats3(table["skybox"]["zenith"]),
            None if "horizon" not in table["skybox"] else _floats3(table["skybox"]["horizon"]),
        ),
        sun=None
        if sun is None
        else Sun(
            _floats3(sun["direction"]),
            _floats3(sun["colour"]),
            f32.r(float(sun["intensity"])),
            f32.r(float(sun["angular_diameter"])),
            bool(sun.get("shadows", True)),
        ),
        ambient=None if "ambient" not in table else _floats3(table["ambient"]),
        lights=lights,
        lens_flare=None
        if flare is None
        else LensFlare(
            flare.get("atlas"),
            None if "colour" not in flare else _floats3(flare["colour"]),
            None if "direction" not in flare else _floats3(flare["direction"]),
        ),
    )


def read_node(g, node, float_kind):
    """An `ArtMesh` out of an `oag/art/<n>` node of a parsed glb (`trackfile._Glb`)."""
    name = node["name"]
    try:
        transform = motionfile.trs_of(node)
    except motionfile.MotionError as e:
        raise ArtError(str(e)) from e
    prims = g.doc["meshes"][node["mesh"]]["primitives"]
    if len(prims) != 1:
        raise ArtError(f"{name}: expected exactly one primitive")
    a = prims[0]["attributes"]
    e = node.get("extras") or {}
    return ArtMesh(
        name=e.get("oag_name", ""),
        positions=g.accessor(a["POSITION"], float_kind, "VEC3"),
        normals=g.accessor(a["NORMAL"], float_kind, "VEC3"),
        texcoords=g.accessor(a["TEXCOORD_0"], float_kind, "VEC2"),
        colours=g.accessor(a["COLOR_0"], float_kind, "VEC4"),
        indices=g.accessor(prims[0]["indices"], UINT, "SCALAR"),
        texture=e.get("oag_texture"),
        blend=e.get("oag_blend", "opaque"),
        culled=e.get("oag_culled", True),
        lit=e.get("oag_lit", False),
        shadow=e.get("oag_shadow", "both"),
        alpha_ref=e.get("oag_alpha_ref"),
        layer=e.get("oag_layer", 0),
        transform=transform,
        pad=_pad_tag(name, e.get("oag_pad")),
    )


def _pad_tag(name, tag):
    if tag is not None and tag not in PAD_PLATES:
        raise ArtError(f"{name}: oag_pad {tag!r} is not speedup or weapon")
    return tag
