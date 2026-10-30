"""Blender keyframes <-> the art's motion (ADR-0076 decision 10; docs/tools/blender.md, "Motion").

**Export samples, it does not copy keys.** An art object or the sun lamp whose
own action (or an ancestor's) moves it is evaluated at every whole frame from the
scene's `frame_start` to the end of the latest action in its chain, and its world
matrix is decomposed into translation, rotation and scale. That is what the
player sees whatever drove it (Bezier easing, constraints, a parent, drivers),
and the race clock's zero is `frame_start`: a key at frame `f` is at
`(f - frame_start) / fps` seconds, `fps` being `render.fps / render.fps_base`.
Keys are `LINEAR`. A translation or scale key that lies on the line between its
neighbours is dropped, and so is a rotation key inside a run of equal ones; a
property that never changes is left to the rest pose. A turning rotation keeps
every frame: Blender blends quaternion curves component-wise, so a dropped key
would not come back on a re-import.

One glTF animation per `oag_anim` name (the object's name when unset), so the
art meshes of one object's material slots, and objects that share a name, share
an animation and its `oag_loop` (`loop` or `clamp`). A loop wraps on the
animation's last key: make the last frame the first pose again for a seamless
cycle.

The moving object's vertices are written in its own space and its pose at
`frame_start` is the node's rest pose. A sun lamp only turns (its location and
scale do not travel); `[art.sun] direction` stays its direction at `frame_start`.
"""

import math

import bpy
from bpy_extras import anim_utils
from mathutils import Quaternion, Vector

from . import f32
from .model import Animation, Channel

TOLERANCE = 1e-5


def _actions(obj):
    out = []
    while obj is not None:
        ad = obj.animation_data
        if ad is not None and ad.action is not None:
            out.append(ad.action)
        obj = obj.parent
    return out


def candidates(objs):
    """The objects whose chain carries an action, with the last frame it reaches."""
    out = {}
    for o in objs:
        acts = _actions(o)
        if acts:
            out[o] = max(a.frame_range[1] for a in acts)
    return out


def to_format_trs(m):
    """A Blender world matrix as (translation, rotation xyzw, scale) in the format's Y-up axes.

    The axis change (x, y, z) -> (x, z, -y) is a rotation, so a node matrix M becomes
    C M C^-1: the translation turns, the quaternion's vector part turns with it and
    the per-axis scale is permuted.
    """
    loc, q, sc = m.decompose()
    t = tuple(f32.r(c) for c in (loc.x, loc.z, -loc.y))
    q.normalize()
    r = (q.x, q.z, -q.y, q.w)
    s = tuple(f32.r(c) for c in (sc.x, sc.z, sc.y))
    return t, r, s


def to_blender_trs(trs):
    t, r, s = trs
    return (
        Vector((t[0], -t[2], t[1])),
        Quaternion((r[3], r[0], -r[2], r[1])),
        Vector((s[0], s[2], s[1])),
    )


def _r4(q):
    return tuple(f32.r(c) for c in q)


def _close(a, b):
    return all(abs(x - y) <= TOLERANCE for x, y in zip(a, b))


def _reduce(times, values):
    """Drops the keys inside a run of equal keys."""
    keep_t, keep_v = [times[0]], [values[0]]
    for i in range(1, len(times) - 1):
        if not values[i] == keep_v[-1] == values[i + 1]:
            keep_t.append(times[i])
            keep_v.append(values[i])
    if len(times) > 1:
        keep_t.append(times[-1])
        keep_v.append(values[-1])
    return keep_t, keep_v


def sample(scene, objs):
    """Each moving object's per-frame poses (format axes): {obj: (times, [trs])}.

    Leaves the scene at `frame_start`; the caller restores the frame it had.
    """
    ends = candidates(objs)
    poses = {o: ([], []) for o in ends}
    if ends:
        fps = scene.render.fps / scene.render.fps_base
        start = scene.frame_start
        last = int(math.ceil(max(ends.values())))
        for f in range(start, last + 1):
            scene.frame_set(f)
            t = f32.r((f - start) / fps)
            for o, end in ends.items():
                if f <= math.ceil(end):
                    poses[o][0].append(t)
                    poses[o][1].append(to_format_trs(o.matrix_world))
    scene.frame_set(scene.frame_start)
    out = {}
    for o, (times, trs) in poses.items():
        # One rotation, two quaternions: keep each on the side of its predecessor.
        rots = []
        for _, r, _ in trs:
            if rots and sum(a * b for a, b in zip(rots[-1], r)) < 0:
                r = tuple(-c for c in r)
            rots.append(r)
        trs = [(t, _r4(r), s) for (t, _, s), r in zip(trs, rots)]
        if len(times) > 1 and any(not _close([*p[0], *p[1], *p[2]], [*trs[0][0], *trs[0][1], *trs[0][2]]) for p in trs):
            out[o] = (times, trs)
    return out


def channels(target, times, trs, paths=("translation", "rotation", "scale")):
    """The channels of one target's poses; a property that never changes has none."""
    out = []
    for k, path in enumerate(("translation", "rotation", "scale")):
        if path not in paths:
            continue
        values = [p[k] for p in trs]
        # Decomposing a turning matrix leaves last-bit noise in a scale that never changed.
        if all(_close(v, values[0]) for v in values):
            continue
        t, v = _reduce(times, values)
        out.append(Channel(target, path, t, v))
    return out


def anim_name(obj):
    return obj.get("oag_anim") or obj.name


def build(groups):
    """Animations from [(name, repeat, [Channel], on)] in first-seen order, merged by name."""
    out, by_name = [], {}
    for name, repeat, chans, on in groups:
        if not chans:
            continue
        anim = by_name.get(name)
        if anim is None:
            anim = by_name[name] = Animation(name, [], repeat, on)
            out.append(anim)
        elif anim.repeat != repeat:
            raise ValueError(f"animation {name!r}: objects disagree on oag_loop")
        elif anim.on != on:
            raise ValueError(f"animation {name!r}: objects disagree on oag_on")
        anim.channels += chans
    return out


RATES = (24, 25, 30, 48, 50, 60, 120)


def fit_fps(art):
    """The first common frame rate every key time of `art` lands on a whole frame at, else None."""
    times = [t for a in art.animations for c in a.channels for t in c.times]
    for fps in RATES:
        if all(abs(t * fps - round(t * fps)) < 1e-3 for t in times):
            return fps
    return None


def apply(context, art, objs, sun_obj):
    """Keyframes on the imported objects (`objs[n]` is art mesh n's) for `art`'s motion.

    The scene takes the frame rate the keys land on whole frames at, so an export samples
    them where they are; a file whose keys fit no common rate keeps the scene's."""
    scene = context.scene
    if not art.animations:
        return
    fps = fit_fps(art)
    if fps is not None:
        scene.render.fps, scene.render.fps_base = fps, 1.0
    fps = scene.render.fps / scene.render.fps_base
    last = max(t for a in art.animations for c in a.channels for t in c.times)
    scene.frame_end = max(scene.frame_end, scene.frame_start + int(math.ceil(last * fps)))
    interp = {}
    for n, m in enumerate(art.meshes):
        if m.transform is not None:
            loc, rot, sc = to_blender_trs(m.transform)
            o = objs[n]
            o.rotation_mode = "QUATERNION"
            o.location, o.rotation_quaternion, o.scale = loc, rot, sc
    if art.sun_rotation is not None and sun_obj is not None:
        sun_obj.rotation_mode = "QUATERNION"
        sun_obj.rotation_quaternion = to_blender_trs(((0, 0, 0), art.sun_rotation, (1, 1, 1)))[1]
        context.view_layer.update()
        sun_obj["oag_matrix_at"] = [sun_obj.matrix_world[r][c] for r in range(4) for c in range(4)]
    for anim in art.animations:
        for ch in anim.channels:
            o = sun_obj if ch.target == "sun" else objs[ch.target]
            if o is None:
                continue
            o["oag_anim"] = anim.name
            o.oag_loop = anim.repeat
            o.oag_on = anim.on or ""
            o.rotation_mode = "QUATERNION"
            prop = {"translation": "location", "rotation": "rotation_quaternion", "scale": "scale"}[ch.path]
            for t, v in zip(ch.times, ch.values):
                if ch.path == "translation":
                    value = to_blender_trs((v, (0, 0, 0, 1), (1, 1, 1)))[0]
                elif ch.path == "rotation":
                    value = to_blender_trs(((0, 0, 0), v, (1, 1, 1)))[1]
                else:
                    value = to_blender_trs(((0, 0, 0), (0, 0, 0, 1), v))[2]
                setattr(o, prop, value)
                frame = scene.frame_start + t * fps
                # An f32 time a hair off a whole frame lands on it, so a re-export samples the key.
                if abs(frame - round(frame)) < 1e-4:
                    frame = round(frame)
                o.keyframe_insert(prop, frame=frame)
            interp[(o.name, prop)] = "CONSTANT" if ch.interpolation == "STEP" else "LINEAR"
    # Keys come in as Bezier whatever the preference says: set each curve's blend as the file's.
    for o in {o for o in [*objs, sun_obj] if o is not None and o.animation_data is not None}:
        ad = o.animation_data
        for fc in anim_utils.action_get_channelbag_for_slot(ad.action, ad.action_slot).fcurves:
            kind = interp.get((o.name, fc.data_path))
            if kind is not None:
                for k in fc.keyframe_points:
                    k.interpolation = kind
    scene.frame_set(scene.frame_start)
