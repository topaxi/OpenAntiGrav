"""The art's motion in a track file (ADR-0076 decision 10), with no Blender.

The reader of record is `oag_trackfile` (`crates/trackfile/src/glb/anim.rs` and
`art/motion.rs`): this writes what it reads. A moving art mesh node carries its
rest pose as glTF `translation`, `rotation` and `scale`; the sun is an `oag/sun`
node with only a rest rotation; glTF `animations` hold float samplers (`LINEAR` or
`STEP`) and an `oag_loop` extra (`loop` or `clamp`).
"""

import struct

from . import f32
from .model import PATHS, REPEATS, Animation, Channel

SUN = "oag/sun"
FLOAT = 5126
UNIT_TOLERANCE = 1e-3
WIDTH = {"translation": 3, "rotation": 4, "scale": 3}
IDENTITY = ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0, 1.0), (1.0, 1.0, 1.0))


class MotionError(Exception):
    """Motion the format cannot hold."""


def _quat(what, q):
    n = sum(c * c for c in q) ** 0.5
    if abs(n - 1.0) > UNIT_TOLERANCE:
        raise MotionError(f"{what}: a rotation of length {n}, not 1")


def validate(art):
    """The checks `art::motion::validate` makes."""
    driven = set()
    for a, anim in enumerate(art.animations):
        if anim.repeat not in REPEATS:
            raise MotionError(f"art animation {a}: unknown repeat {anim.repeat!r}")
        for c, ch in enumerate(anim.channels):
            what = f"art animation {a} ({anim.name}) channel {c}"
            if ch.path not in PATHS or ch.interpolation not in ("LINEAR", "STEP"):
                raise MotionError(f"{what}: unknown path or interpolation")
            if not ch.times or len(ch.values) != len(ch.times):
                raise MotionError(f"{what}: {len(ch.values)} keys for {len(ch.times)} times")
            if ch.times[0] < 0 or any(b <= a for a, b in zip(ch.times, ch.times[1:])):
                raise MotionError(f"{what}: key times must start at or after 0 and strictly increase")
            if any(len(v) != WIDTH[ch.path] for v in ch.values):
                raise MotionError(f"{what}: a key of the wrong width")
            if ch.path == "rotation":
                for q in ch.values:
                    _quat(what, q)
            if ch.target == "sun":
                if art.sun is None or ch.path != "rotation":
                    raise MotionError(f"{what}: the sun takes rotation only, and needs a sun")
            elif not 0 <= ch.target < len(art.meshes):
                raise MotionError(f"{what}: mesh {ch.target}, the file has {len(art.meshes)}")
            key = (ch.target, ch.path)
            if key in driven:
                raise MotionError(f"{what}: a second channel drives {key}")
            driven.add(key)
    for n, m in enumerate(art.meshes):
        moved = any(t == n for t, _ in driven)
        if m.transform is not None and not moved:
            raise MotionError(f"art mesh {n} ({m.name}): a transform on a mesh no animation moves")
        if m.transform is not None:
            _quat(f"art mesh {n}", m.transform[1])
    turned = any(t == "sun" for t, _ in driven)
    if turned != (art.sun_rotation is not None):
        raise MotionError("art sun node: a rest rotation exactly when an animation turns the sun")


def set_trs(node, trs):
    """The rest pose on a glTF node dict; identity parts left out, as the Rust writer does."""
    for key, value, rest in zip(("translation", "rotation", "scale"), trs, IDENTITY):
        if tuple(value) != rest:
            node[key] = list(value)


def write(b, art, nodes, mesh_nodes, push):
    """Rest poses, the sun node and the `animations` list (empty when nothing moves)."""
    validate(art)
    for m, at in zip(art.meshes, mesh_nodes):
        if m.transform is not None:
            set_trs(nodes[at], m.transform)
    sun_node = None
    if art.sun_rotation is not None:
        sun_node = push(SUN, [], None)
        nodes[sun_node]["rotation"] = list(art.sun_rotation)
    out = []
    for anim in art.animations:
        samplers, channels = [], []
        for ch in anim.channels:
            node = sun_node if ch.target == "sun" else mesh_nodes[ch.target]
            times = list(ch.times)
            inp = b.add(struct.pack(f"<{len(times)}f", *times), FLOAT, "SCALAR", len(times), 34962)
            b.accessors[inp]["min"], b.accessors[inp]["max"] = [times[0]], [times[-1]]
            out_acc = b.floats(ch.values, "VEC4" if ch.path == "rotation" else "VEC3")
            samplers.append({"input": inp, "output": out_acc, "interpolation": ch.interpolation})
            channels.append({"sampler": len(samplers) - 1, "target": {"node": node, "path": ch.path}})
        out.append(
            {"name": anim.name, "channels": channels, "samplers": samplers, "extras": {"oag_loop": anim.repeat}}
        )
    return out


def trs_of(node):
    """A node's rest pose, or None for the identity; a matrix is refused."""
    if "matrix" in node:
        raise MotionError(f"{node.get('name', '')}: a node matrix (write translation, rotation, scale)")
    trs = tuple(
        tuple(f32.r(float(c)) for c in node.get(key, rest))
        for key, rest in zip(("translation", "rotation", "scale"), IDENTITY)
    )
    return None if trs == IDENTITY else trs


def read(g, target_of):
    """The document's animations; `target_of(node_index)` is a mesh index, "sun" or None."""
    out = []
    for a, anim in enumerate(g.doc.get("animations", [])):
        repeat = (anim.get("extras") or {}).get("oag_loop", "loop")
        if repeat not in REPEATS:
            raise MotionError(f"animation {a}: oag_loop {repeat!r} is not loop or clamp")
        channels = []
        for c, ch in enumerate(anim["channels"]):
            what = f"animation {a} channel {c}"
            target = target_of(ch["target"]["node"])
            if target is None:
                raise MotionError(f"{what}: only art meshes and the sun move")
            path = ch["target"]["path"]
            if path not in PATHS:
                raise MotionError(f"{what}: path {path!r} is not read")
            s = anim["samplers"][ch["sampler"]]
            interp = s.get("interpolation", "LINEAR")
            if interp not in ("LINEAR", "STEP"):
                raise MotionError(f"{what}: {interp} is not read; sample it to LINEAR keys")
            times = g.accessor(s["input"], FLOAT, "SCALAR")
            values = g.accessor(s["output"], FLOAT, "VEC4" if path == "rotation" else "VEC3")
            channels.append(Channel(target, path, list(times), list(values), interp))
        out.append(Animation(anim.get("name", ""), channels, repeat))
    return out
