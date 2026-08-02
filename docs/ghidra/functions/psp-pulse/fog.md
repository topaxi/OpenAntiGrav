# Fog: `Gu_Fog` and the per-frame fog state

Found while looking for where the race pass sets its fog, on the lead recorded in
[`exhaust.md`](exhaust.md): `Trail_BuildStateList`'s display list **disables**
`FOG`, and nothing disables a feature that was never on.

It reaches further than the wrapper. The chain from an authored `fogCube`
`0x3d3` node to the GE commands is closed end to end, and closing it answered
three questions [`skycube.md`](../../../formats/skycube.md) had left open from
the data side alone.

## `Gu_Fog` (`0x08811748`)

| | |
| --- | --- |
| **Address** | `0x08811748` |
| **Confidence** | **95** |

```c
void Gu_Fog(float near, float far, u32 colour);
```

```c
float distance = far - near;
float scale = 0.0f;
if (distance != 0.0f) scale = 1.0f / distance;   // DAT_08ab0528 is 0000803f
*p     = colour & 0xffffff | 0xcf000000;         // FCOL, fog colour
p[1]   = (u32)far   >> 8   | 0xcd000000;         // FFAR, fog end
p[2]   = (u32)scale >> 8   | 0xce000000;         // FDIST, 1/(far - near)
```

Three things make this 95 rather than a guess:

- **The GE command bytes are in the function's own body**, not inferred from an
  argument pattern - the same standard `exhaust.md` set for the ribbon's blend
  state. `0xcd`/`0xce`/`0xcf` are `FOG1`/`FOG2`/`FCOL`.
- **The arithmetic is `sceGuFog`'s exactly**, including the divide-by-zero guard
  and the reciprocal. `DAT_08ab0528` was read and is `1.0f`.
- **Four of its five callers pair it with `Gu_Enable(7)`** on the very next line,
  and `7` is `GU_FOG` (below).

It follows the `Gu_*` wrapper shape used throughout `0x0881xxxx`: take the
display-list write pointer from `DAT_08adc3ac + 8`, write commands, advance it.
`>> 8` is the float send (a GE command carries a 24-bit payload, so a float goes
in truncated), `& 0xffffff` is the colour send - both confirmed against
`Gu_Ambient` (`0x0881125c`) and the viewport wrapper at `0x08811124`.

### Colour packing

`Fog_Apply` builds the argument as `0xAABBGGRR` - red in the low byte:

```c
(r * 255) & 0xff | ((g * 255) & 0xff) << 8 | ((b * 255) & 0xff) << 16 | (a * 255) << 24
```

`Gu_Fog` then masks the alpha off, so the fog colour is RGB only.

## `GU_FOG` is state `7`

Not assumed from a header - **decoded from `Trail_BuildStateList`
(`0x089296f8`) and self-checking**, because that one function names fourteen
states and every one agrees with the pspsdk enum:

| Call | State | Reading |
| --- | --- | --- |
| `Gu_Disable(7)` | `GU_FOG` | the lead this page started from |
| `Gu_Disable(5)` | `GU_CULL_FACE` | |
| `Gu_Disable(10)` / `Gu_Enable(10)` | `GU_LIGHTING` | disabled early, re-enabled at the end |
| `Gu_Disable(3)` | `GU_STENCIL_TEST` | followed by `Gu_StencilOp` |
| `Gu_Disable(0)` | `GU_ALPHA_TEST` | |
| `Gu_Enable(9)` | `GU_TEXTURE_2D` | |
| `Gu_Enable(1)` | `GU_DEPTH_TEST` | followed by `Gu_DepthFunc`/`Gu_DepthMask` |
| `Gu_Enable(4)` | `GU_BLEND` | followed by `Gu_BlendFunc` |
| `Gu_Disable(0xb..0xe)` | `GU_LIGHT0..3` | immediately after enabling `GU_LIGHTING` |

The last row is the clincher: four consecutive states disabled right after
lighting is enabled can only be the four light slots, which pins the whole
numbering by position.

## `Fog_Apply` (`0x0891ea14`)

| | |
| --- | --- |
| **Address** | `0x0891ea14` |
| **Confidence** | **85** |

```c
render_manager->fog_a = 0.0f;
Gu_Fog(render_manager->fog_near, render_manager->fog_far, pack(r, g, b, a));
Gu_Enable(GU_FOG);
```

Reads six consecutive floats off the render manager and pushes them:

| Offset | Field |
| --- | --- |
| `+0x5dd4` | fog red |
| `+0x5dd8` | fog green |
| `+0x5ddc` | fog blue |
| `+0x5de0` | fog alpha - **zeroed by this function before it is packed** |
| `+0x5de4` | fog near |
| `+0x5de8` | fog far |

**Those six floats, in that order, with a zero in the fourth slot, are the same
shape as the `fogCube` payload's parameter set** - see
[`skycube.md`](../../../formats/skycube.md), which decoded `{rgb, 0, near, far}`
from shipped track data with no reference to the executable. Two derivations from
opposite directions agreeing on a six-float block with a hole in slot four is the
strongest single piece of corroboration either has.

The render manager's own defaults are written in its constructor
(`FUN_0891f320`): near `0x42700000` = **60.0**, far `0x43b90000` = **370.0**,
colour `(0, 0, 0.125)` - a dark blue. Those are the fallback, not any track's
authored values.

## `Fog_Disable` (`0x0891e9b4`)

| | |
| --- | --- |
| **Address** | `0x0891e9b4` |
| **Confidence** | **90** |

One line: `Gu_Disable(GU_FOG)`. Its only interest is that it is one of three
branches in the material state setter below.

## How a draw chooses its fog

`FUN_0891f890` is the per-material GE state setter, and it picks one of three fog
paths off a material flag word:

```c
if ((flags & 8) == 0) {
    if ((flags & 1) == 0) FUN_0891e9d0();   // conditional: on only inside a volume
    else                  Fog_Disable();
} else {
    Fog_Apply(render_manager);              // on, with the render manager's parameters
}
if (DAT_08abf5dc == 0) Gu_Disable(GU_FOG);  // global master switch, last word
```

Two consequences worth recording:

- **Fog is per-material, not per-scene.** A surface opts in through its own
  flags, which is why the exhaust ribbon can disable fog for itself without
  touching anything else.
- **`DAT_08abf5dc` is a global fog kill switch** applied after the per-material
  decision, so it overrides everything.

`FUN_0891e9d0` is the conditional branch:

```c
if (FUN_08887764(DAT_08b32c88, &out) == 0) Gu_Disable(GU_FOG);
else                                       Gu_Enable(GU_FOG);
```

It enables fog **without** setting parameters, relying on whatever was last
pushed.

## `fogCube` `0x3d3`: the class, end to end

The chain from the authored node to `Gu_Fog` is closed. No step in it is
inferred.

### It *is* registered - `FogCube_RegisterClass` (`0x08906b60`), confidence 95

```c
Vex_RegisterClass(&DAT_08b62948, 0x3d3);
DAT_08b62980 = &DAT_08ad1394;      // method table
DAT_08b62970 = &DAT_08a84ce0;      // class name
DAT_08b6294c = FUN_08a6bb6c();     // type token
```

This retires a standing worry. [`vex.md`](../../../formats/vex.md) records that
`engine_fire`, `exitglow` and `gate` are authored with **no** registration site,
and [`skycube.md`](../../../formats/skycube.md) carried that forward as the risk
that `fogCube` might have none either. It has one, found by its class id sitting
in the call as a literal - which works here precisely because
`Vex_RegisterClass`'s *second argument* is the id, the one place a class id does
appear as an immediate.

`FUN_08a6bb6c` is an identity token: a function that returns its own address, so
the address is the type id. That token is the thread that ties the rest together.

`FogCube_Construct` (`0x08906720`, confidence 85) is the instance constructor the
registration's allocator runs: it writes the same method table `&DAT_08ad1394`
into the instance at `+0x38` and the same type token into `+0x04`, and zeroes
`+0x40` and `+0x48` - the head of the six-float sample buffer that
`FogCube_Sample` later fills.

### `FogCube_Init` (`0x089068f8`), confidence 92

Method-table slot `+0x7c`, the init slot per [`exhaust.md`](exhaust.md).

```c
payload = bind_cursor->at;
self->payload = payload;              // +0x50
bind_cursor->at = payload + 0x80;     // <- 128 bytes
self->half_extent = payload[0x74] * 0.5f;   // +0x54
self->z_scale     = 1.0f / payload[0x74];   // +0x58
```

**`0x80` is 128, which is the `fogCube` payload length measured from shipped data
on all 36 nodes before this function was read.** Two independent derivations of
the same number.

It also settles the float at payload `+0x74`, recorded in `skycube.md` as "500.0
on every track, unknown": it is the **cube's edge length**. Half of it is the
box's half-extent and its reciprocal is the axis normaliser below.

### `FogCube_Sample` (`0x089069b0`), confidence 92

```c
v = matrix_at(payload + 0x00) * vec4(world_pos, 1);   // world -> volume space
if (any(abs(v.xyz) > self->half_extent)) return 0;    // outside
float t = v.z * self->z_scale + 0.5f;                 // -> 0..1 across the box
out[0..5] = lerp(payload[0x40..0x54], payload[0x58..0x6c], t);
return 1;
```

**Every offset it touches is one this project decoded from shipped data
independently**: the 64-byte matrix at `+0x00`, parameter set A at
`+0x40..0x54`, set B at `+0x58..0x6c`. Six offsets agreeing leaves no room for
coincidence, and it is what promotes the payload decode from a reading to a fact.

**It also answers what the second parameter set is for, which `skycube.md` left
open.** The two sets are the two ends of a **linear interpolation across the
volume's local Z axis** - colour, near *and* far all slide between them as the
camera moves through the box. That is why 28 of the 36 shipped `fogCube` nodes
have byte-identical sets (uniform fog) and 8 do not (graded fog).

### `Fog_FindVolume` (`0x08887764`), confidence 88

Walks the world's `fogCube` list - `world + 0x81c`, count at `world + 0xb3c` -
calling `FogCube_Sample` on each with the camera position, and keeps the
**smallest** by `payload[0x70]` when more than one contains the camera. Caches
the result against the camera position (`world + 0xc50`, valid flag `+0xc78`,
cached six floats `+0xc60..0xc74`), so a stationary camera costs one compare.

That list is populated by `World_CollectNodeLists` (`0x088879d4`, confidence 80),
which gathers three typed node arrays into the world; the third filters on
**`FUN_08a6bb6c`, the `fogCube` type token**, with a capacity of 200. That token
identity is the last link: the array `Fog_FindVolume` iterates provably holds
`fogCube` instances.

`DAT_08b32c88` is the world singleton - written by the world constructor
(`FUN_08886af4`), which is also what calls `World_LoadTrack`.

## The fog "curve", resolved

[`roadmap.md`](../../../overview/roadmap.md) records the original's fog as "a
specific curve rather than a linear ramp", and that sat awkwardly against
`Gu_Fog` being flatly linear.

Both are true and there is no contradiction. **The GE ramp is linear within a
frame; its parameters are re-interpolated every frame from where the camera sits
in the fog volume.** `FogCube_Sample` lerps all six - colour, near and far -
across the box's local Z. Fog that thickens and shifts hue as the craft descends
through a volume is not a curve in the ramp function, and reproducing it needs
the volume and the lerp, not a shaping function on the ramp.

**So a linear GE ramp is correct and must not be "fixed" into a curve.**

## Open

- **`payload[0x70]`, the overlap tiebreak.** `Fog_FindVolume` picks the smallest
  and the shipped values are 1.6e9, 2.5e9, 7.8e9 - large enough to be a volume or
  a squared extent, but the units are not established and nothing else reads it.
  Recorded as "smallest wins", not as a name.
- **`FUN_08906188` builds the fog GE command lists and has no callers.** It
  writes two four-word lists at `DAT_08b62918` and `DAT_08b62930` - the second
  identical except `FCOL` forced to **black** - from a six-float fog buffer, with
  defaults of far `5001.0`, `1/dist 1.0`, colour `0x808080` when no volume
  contains the camera. It is *not* in `fogCube`'s method table (which takes the
  base-class update, submit and draw), so what dispatches it is unknown and it is
  deliberately unnamed. The two-list split - one fogged to the fog colour, one
  fogged to black - is the most interesting unexplained thing here, and is very
  likely the additive-versus-alpha distinction the exhaust already needs.
- **`FUN_0891f890`'s material flag bits `1` and `8`**, which choose between
  `Fog_Apply`, `Fog_Disable` and the conditional path. Their effect is recovered;
  their authored source is not.
- **`Skycube` `0x3c6` has had none of this done to it.** Its draw handler is
  still unrecovered, so the sky's depth state in this engine remains chosen
  rather than measured. `FogCube_RegisterClass` is now the worked example for
  finding it: search for the class id as the second argument of
  `Vex_RegisterClass`.
