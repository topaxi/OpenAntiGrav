"""A mod's particle effect (`schemas/v1/effect.tosd`) in Blender: read, write, preview.

The effect's TOML text lives in a Blender Text datablock (`oag/effect/<name>`), so an
author edits it in the Text Editor; a preview particle system is built from it on the
selected object, and export parses the text, checks it and writes it back canonically.
The text is the source of truth: Blender's particle settings are `float32` and would
not give the file's numbers back.
"""

import math
import tomllib

import bpy

SCHEMA = (
    "https://oag.topaxi.com/mods/schemas/v1/effect.tosd",
    "1.0.0",
)
TEXT_PREFIX = "oag/effect/"
PROP = "oag_effect_text"


class EffectError(ValueError):
    pass


def parse(text):
    """The file's dict, checked the way the game's loader checks the shape."""
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError as e:
        raise EffectError(f"not TOML: {e}") from e
    header = data.get("toml-schema")
    if not isinstance(header, dict) or not str(header.get("location", "")).endswith("/v1/effect.tosd"):
        raise EffectError("no [toml-schema] naming effect.tosd v1")
    emitters = data.get("emitter")
    if not emitters:
        raise EffectError("no [[emitter]]")
    for i, e in enumerate(emitters):
        for need in ("duration", "lifetime", "size", "alpha", "colour"):
            if need not in e:
                raise EffectError(f"emitter {i}: no {need}")
        life = e["lifetime"]
        if len(life) != 2 or life[0] < 1:
            raise EffectError(f"emitter {i}: lifetime is [centre, spread] with a centre of at least 1")
    return data


def _scalar(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, float):
        if math.isnan(v) or math.isinf(v):
            raise EffectError("a number is not finite")
        return repr(v)
    if isinstance(v, str):
        return '"' + v.replace("\\", "\\\\").replace('"', '\\"') + '"'
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(_scalar(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{k} = {_scalar(x)}" for k, x in v.items()) + " }"
    raise EffectError(f"cannot write {v!r}")


def dumps(data):
    """The canonical text of `data`: the header, the file's keys, each `[[emitter]]`."""
    lines = []
    for key, value in data.items():
        if key != "emitter":
            lines.append(f"{key} = {_scalar(value)}")
    for emitter in data.get("emitter", []):
        lines += ["", "[[emitter]]"]
        lines += [f"{k} = {_scalar(v)}" for k, v in emitter.items()]
    return "\n".join(lines) + "\n"


def _text_for(name):
    return bpy.data.texts.get(TEXT_PREFIX + name) or bpy.data.texts.new(TEXT_PREFIX + name)


def attach(obj, name, text):
    """Keep `text` for `obj` as an editable Text datablock; returns it."""
    parse(text)
    block = _text_for(name)
    block.clear()
    block.write(text)
    obj[PROP] = block.name
    return block


def text_of(obj):
    name = obj.get(PROP)
    block = bpy.data.texts.get(name) if name else None
    if block is None:
        raise EffectError(f"{obj.name} has no effect text; run Preview mod effect first")
    return block.as_string()


def _mean(v):
    return (v[0] + v[1]) / 2 if isinstance(v, list) else float(v)


def _first_value(curve):
    return float(curve if not isinstance(curve, list) else curve[0][1])


def preview(context, obj, text):
    """Particle systems on `obj`, one per emitter, playing the effect in the viewport.

    Approximate (Blender's particles are not the game's): count, timing, speed, spread and
    size follow the file; colour and blend are left to the viewport.
    """
    data = parse(text)
    for mod in [m for m in obj.modifiers if m.name.startswith("oag_fx")]:
        obj.modifiers.remove(mod)
    fps = context.scene.render.fps
    for i, e in enumerate(data["emitter"]):
        mod = obj.modifiers.new(f"oag_fx {e.get('name', i)}", "PARTICLE_SYSTEM")
        settings = mod.particle_system.settings
        settings.name = f"oag_fx {e.get('name', i)}"
        duration = float(e["duration"])
        every = _mean(e.get("interval", 1))
        burst = _mean(e.get("per-emission", 1))
        settings.count = max(1, int(math.ceil(duration / every) * burst))
        settings.frame_start = 1
        settings.frame_end = 1 + duration
        settings.lifetime = e["lifetime"][0]
        settings.lifetime_random = e["lifetime"][1] / e["lifetime"][0]
        speed = e.get("speed", [0.0, 0.0])
        settings.normal_factor = speed[0] * fps
        settings.factor_random = speed[1] * fps
        settings.particle_size = max(0.001, _first_value(e["size"]))
        settings.display_method = "RENDER"
        settings.render_type = "HALO"
        settings.effector_weights.gravity = 1.0 if e.get("gravity", 0.0) else 0.0
    return data
