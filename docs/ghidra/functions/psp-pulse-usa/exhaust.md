# Ship exhaust: the `Engine Flare` and `Trail` node classes

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP, UCUS-98712) |
| **Subsystem** | scene-graph effects nodes |
| **Related** | [`vex.md`](../../../formats/vex.md) (the class-ID table), [`main-loop.md`](main-loop.md) (`Gfx_FlushRenderManager`), [`camera.md`](camera.md) (`craft+0x794`, the ship scene node) |

One page for the subsystem, per this directory's convention: the evidence is
mostly shared between the functions and splitting it leaves every page asserting
what its neighbour proves.

## Summary

The ship's exhaust is **not** a particle system. It is two scene-graph node
classes with dedicated code and two dedicated textures:

| Class | ID | What it is |
| --- | --- | --- |
| `Engine Flare` | `0x3bf` | one additive camera-facing quad at the nozzle, plus the engine sound and the boost model |
| `Trail` | `0x3c8` | a position-history ribbon, N layers, alpha-faded to the tail |

The 25 `Data\Psys\WO_*.POB` particle systems in the image are all weapons,
impacts and ship death. **There is no engine or exhaust `.POB` on either
binary** - see [Cross-platform](#cross-platform).

## The class-ID table

`docs/formats/vex.md` already recorded the table at `0x08ab2370`, stride 12,
`{u32 id, char *name, ptr}`, names at `0x08a84d40`. This pass read the IDs out.

Two structural facts that earlier readings had wrong, both load-bearing:

- **The third field is a runtime slot, not a shared vtable.** It reads
  `0x08b62c08` in every shipped entry. `Vex_RegisterClass` writes each class's
  descriptor into it at boot, and `Vex_FindClassDescriptor` returns
  `&DAT_08b62c08` - a *fallback* descriptor - on a miss. All-`0x08b62c08` in
  `.data` means nothing is registered yet, not that every class shares a handler.
- **The table is terminated by `id == -1`**, per both walkers. It continues past
  the game classes into generic Maya classes with small sequential ids
  (`0 Invalid`, `1 Base`, `2 Name`, …). Read as far as `0x08ab26a0` here; the
  terminator was not reached, so **the extent is not stated**.

Confidence **95**, and the reason is that the decode is self-validating: ten IDs
already sit in `crates/formats/src/vex.rs:64-103`, put there by earlier passes
from unrelated evidence, and every one lands exactly where this read puts it -
including the two most easily confused, `Mag Floor Collision 0x3e6` and
`Cage Collision 0x3e7`.

`0x3e3` has no entry. Recorded rather than skipped, because the table does contain
out-of-order ids (`0x3d0`, `0x3e9`, `0x3eb`), so a gap is not proof of anything.

## `Vex_RegisterClass`

| | |
| --- | --- |
| **Address** | `0x08908eb8` |
| **Confidence** | 90 |

```c
ClassDescriptor *Vex_RegisterClass(ClassDescriptor *self, u32 class_id);
```

Stores `class_id` at `self+0x44`, then walks the table from `0x08ab2370` in
stride-12 steps until `id == -1` or `id == class_id`, and on a hit writes `self`
into the entry's third field.

**It has 46 callers, one per node class** - `Collision_RegisterNodeClasses`
(`0x08934d44`) among them, which is the template this follows. Each caller is a
static initialiser of the same shape: register the id, assign the base method
table `&DAT_08ad22f4` (the `Transform` class, id `0x6e`), then overwrite it with
the derived table.

The callers are in **alphabetical order by class name**, with monotonically
increasing method-table addresses - static-initialiser link order, one
translation unit per class. Method tables are `0x88` bytes apart.

**Class dispatch is by descriptor lookup, never by immediate compare.** There is
no `li 0x3bf` anywhere in 635,898 instructions, so searching for a class ID as a
constant will not find its handler; the registration call site is the only route.

### The method-table layout

Slots are 8 bytes (`{function, 0}`). Read off the two tables below, which agree
slot for slot:

| Offset | Role | `Engine Flare` (`0x08ad129c`) | `Trail` (`0x08ad226c`) |
| --- | --- | --- | --- |
| `+0x24` | update(dt) | `0x089058b0` | `0x0892a450` |
| `+0x34` | submit | `0x0890490c` | `0x0892a408` |
| `+0x44` | draw | `0x08904a30` | `0x0892a588` |
| `+0x7c` | init | `0x08905308` | `0x0892a2b0` |

Slots holding `0x08944xxx` are base-class defaults shared by both. That the two
independently-registered classes place their update, submit and draw at the same
three offsets is what makes the role assignment evidence rather than a guess.

## `ExhaustFlare_Submit`

| | |
| --- | --- |
| **Address** | `0x0890490c` |
| **Confidence** | 85 |

Returns early when `self+0x84 != 0`. Otherwise takes the node's world
translation (`*(self+0x30)` at `+0x30`), asks `Gfx_ViewDepth_q` for a depth, and
when that is negative builds a 20-bit back-to-front sort key:

```c
key = 0xfffff - (min((int)(-depth * 349.525), 0xfffff) & 0xfffff);
Gfx_Enqueue_q(g_display, self, key | 0x4d000000);
```

`0xfffff / 349.525 = 3000.0`, so the key spans **3,000 world units** of depth at
layer `0x4d`.

**This is why the transparent pass sorts instead of depth-writing.** The original
resolves its own transparency ordering with this key, which is a recovered reason
to disable depth write in a reimplementation rather than a stylistic choice.
`Gfx_FlushRenderManager` (`0x0891e3c0`, [main-loop.md](main-loop.md)) is what
sorts the queue - identified earlier as "sort plus deferred draw callbacks", and
this is what it sorts.

`self+0x84` is a tri-state, not a flag: submit skips on `!= 0` and
`Exhaust_Update` returns early on `== 2`. The constructor sets it to `0`. What
writes `1` or `2` was not found - see [Open](#open).

## `ExhaustFlare_Draw`

| | |
| --- | --- |
| **Address** | `0x08904a30` |
| **Confidence** | 90 |

Pushes identity onto GE matrices 1 and 2 - so the quad is submitted in the space
`Math_TransformVec4` produced, not in world space - binds
`g_engine_flare_texture`, writes four vertices at `self+0x100`, then
`Gu_CallList(self+0x180)`.

```
vertex stride 0x18 = 24 bytes:  f32 u, f32 v, u32 colour, f32 x, f32 y, f32 z
x = cx +/- half_size        (half_size = self+0xc4)
y = cy +/- half_size
colour = self+0xc8
```

Two independent confirmations that this is exactly one quad of exactly that
format, which is why the confidence is 90 rather than 75:

- `sceKernelDcacheWritebackRange(self+0x100, 0x60)` - `0x60` = 4 x 24 exactly.
- The display list's own `Gu_DrawArray(4, 0x19f, 4, 0, self+0x100)`: prim `4` is
  `GU_TRIANGLE_STRIP` with 4 vertices, and vtype `0x19f` decodes as
  `GU_TEXTURE_32BITF (0x3) | GU_COLOR_8888 (0x1c) | GU_VERTEX_32BITF (0x180)` =
  8 + 4 + 12 = **24 bytes**, matching the stride derived from the writes.

The quad is rebuilt from scratch every draw. **There is no history buffer here** -
the flare is a nozzle sprite, and the ribbon is the separate `Trail` class.

**The quad is a view-space billboard with world-sized extents, and it shrinks
with distance.** Settled 2026-08-07 by reading the matrix live at a breakpoint
on this function: the display's stack top at `+0x1410` - what
`Math_TransformVec4` transforms the nozzle by - read back as a pure
world-to-view rigid transform (orthonormal rows, fourth column `0,0,0,1`,
translation in row 3), `Math_TransformVec4` is a plain `vtfm4.q` with no
perspective divide, and the two `Gu_SetMatrix` calls below set GE matrices 1
and 2 to identity while **leaving the projection live** (only matrix 1 is
restored afterwards). So `half_size` is in view units - world-sized under a
rigid view - and the sprite projects like any other geometry. This supersedes
the "post-projection units, constant on-screen size" reading an earlier
version of this page carried, and with it the fitted `2.15` world conversion:
`oag_render::exhaust::HALF_SIZE_TO_WORLD` is now `1.0`, confirmed by a
matched-pose frame comparison (`--pose-from` a captured crossing row, same
recorded camera) where `1.0` reproduces the original's flare-to-hull ratio and
`2.15` read double.

Statically corroborated the same day (confidence up from the live read alone):
`Gu_SetMatrix` is a thunk onto `FUN_08811d64`, a `sceGuSetMatrix` clone whose
command bytes pin index `0` = projection (`0x3e`/`0x3f`, the only 16-word
upload), `1` = view (`0x3c`/`0x3d`), `2` = world (`0x3a`/`0x3b`) - so this
function's two calls really are view and world, and it emits no index-0
command at all. The scene projection it inherits is built by `FUN_08901dc4`
(fov in degrees at `cam+0x50`, near hardcoded `1.2`, far `2000.0` from
`g_display+0x1698`), installed into the display's parallel projection stack at
`+0x1190`; `+0x1410` is the **view** stack, written from the camera node's
matrix at `+0x50` by `FUN_08900884`/`FUN_08900a9c`. The `FUN_0891e9d0` called
after `Gu_CallList` is fog re-evaluation (`Fog_FindVolume`, then GE state 7
enable/disable), not matrix restoration. One caveat for future live reads: the
second camera type (`FUN_088b7f24`/`FUN_088b8548`, aspect hardcoded
`1.7647059`) installs **identity** into the view stack and carries everything
in the projection - a read taken inside its scope shows identity.

**The quad is square, and nothing stretches it. Settled 2026-08-08**, closing
the open item this paragraph used to carry. Three independent readings agree:

- the geometry, at instruction level. `0x08904b58`..`0x08904ba4` loads
  `half_size` into `f13` once and writes `cx +/- f13` into the four vertices'
  `x` and `cy +/- f13` into their `y`. **One register, both axes.**
- the art, measured in the space the quad samples. `ExhaustFlare_Init` writes
  the four UVs as `(0,1) (0,0) (1,1) (1,0)` - the whole texture - and
  `grabbedEngineFlare128x64x8.mip`'s glow has intensity-weighted
  `sigma_u / sigma_v = 1.007` **in UV space**, centred at `(0.495, 0.498)`.
  The artists authored it round *for a square quad*; it is 2:1 in pixels only
  because the image is 128x64.
- the projection, read live off the running game at a breakpoint on this
  function: `m00 = 0.955628`, `m11 = 1.686403`, `m11 / m00 = 1.764706`
  exactly - aspect-correct for 480x272, so a view-space square projects
  square. (The vertical fov that comes out, `60.04` degrees, is the authored
  `<ExternalCameraFar fov>`; see [camera.md](camera.md).)

The "visible glow measures about 1.7:1" observation from the 2026-08-07
stalled-craft capture is contradicted by all three and is not reproduced. Its
companion, "a square-drawn flare read visibly narrower against a live capture"
(2026-08-02), was taken while `HALF_SIZE_TO_WORLD` was still the fitted
`2.15`, so what it compared was *size*, not aspect - which is why it did not
survive the correction that retired `2.15` either. `oag_render::exhaust`'s
`FLARE_ASPECT` is gone; `Exhaust::vertices` carries the three readings above
so nobody reinstates it from a screenshot measurement.

All four vertices take the **single** colour
at `self+0xc8` - white, flickered alpha - so the flare has no per-layer
structure at all; the three staggered ramps belong to the `Trail` below, and
mapping them onto three concentric flare quads (an early reading this project
implemented) is an invention with no counterpart in the function.

### The blend state, from the display list

`0x08904c18` builds the 0x200-byte display list at `self+0x180`:

```
disable  CULL_FACE, LIGHTING, STENCIL_TEST, ALPHA_TEST
enable   DEPTH_TEST
enable   BLEND
sceGuBlendFunc(GU_ADD, src = 2, dst = 10, srcfix = 0, dstfix = 0xffffff)
Gu_DrawArray(GU_TRIANGLE_STRIP, 0x19f, 4, 0, self+0x100)
restore  CULL_FACE, LIGHTING
```

`src = 2` is `GU_SRC_ALPHA`, `dst = 10` is `GU_FIX` with `dstfix = 0xffffff`,
i.e. `(1,1,1)`. So:

```
result = src.rgb * src.a  +  dst.rgb * 1.0
```

**Additive, weighted by source alpha** - `BlendComponent { src_factor: SrcAlpha,
dst_factor: One, operation: Add }`. Depth test on, no cull, no lighting, no alpha
test. No `sceGuDepthMask` call appears, so depth write is inherited from whatever
the transparent layer sets; the sort key above is the evidence that it is off.

Confidence **80** on the blend reading. The GU enum values are standard and the
`0x19f` cross-check lands exactly, but `0x08810e10` / `0x08810db8` are identified
as `sceGuDisable` / `sceGuEnable` **by the argument pattern rather than by their
bodies** - the assignment that makes the sequence coherent (disable cull and
lighting for a sprite, enable depth test and blend) is the one adopted. The
opposite assignment would enable stencil and alpha test on a glow and disable
blending before setting a blend function, which is not a sequence that draws
anything.

## `Exhaust_Update`

| | |
| --- | --- |
| **Address** | `0x089058b0` |
| **Confidence** | 85 |

```c
bool Exhaust_Update(float dt, EngineFlare *self);
```

Returns `1` immediately if `self+0xc0` (the craft) is null, `0` if
`self+0x84 == 2`. Otherwise, with `i = self+0xbc` the intensity:

```
speed_kmh  = speed * 3.6                        -> self+0x8c
speed_ramp = clamp((speed_kmh - 100) / 500)      -> self+0x90    0 at 100, 1 at 600 km/h
boost_acc  = thrust > 0 ? += thrust / 3000 : -= 0.1 ; clamp[0,1]
             then floored at speed_ramp * 0.6    -> self+0x60
engine_on  = thrust > 0 || boost_timer > 0.2     -> self+0x94

three layer alphas, K = 0.7 (DAT_08a84c3c):
  L0 = clamp(i          * 1.00 * K)   -> child +0x0f8..0x104
  L1 = clamp((i - 0.25) * 1.33 * K)   -> child +0x128..0x134
  L2 = clamp((i - 0.50) * 2.00 * K)   -> child +0x158..0x164

child +0x7c = i * 0.35 + 0.2
child +0x80 = i * 0.5 * 0.9            (0.9 = DAT_08a84c38)
child +0xa0 = -(node basis row 2), scaled     <- backwards along the nozzle

half_size = ((i * 0.6 + 0.4) * 2.5 + boost_timer * 8.0) * rand(0.75, 1.25)   -> self+0xc4
colour    = (rand_int(200, 255) << 24) | 0x00ffffff                          -> self+0xc8
```

Three layers staggered at `i = 0`, `0.25`, `0.5`, each reaching full at `i = 1`
(`1/0.75 = 1.33`, `1/0.5 = 2.0`) - concentric plumes fading in as thrust rises.

**`half_size` and the colour's alpha are both re-randomised every frame.** The
flicker is in the original; it is not a stylistic addition.

`self+0x64` (the "child" above) is a 0x210-byte object the constructor builds via
`0x0892a050`, in the same code block as the `Engine_noise` texture loader.

Both lag filters in this class integrate as `n = (int)(dt / 0.0166666680)`
substeps of `v += (target - v) * rate`, with no transcendental. That is the same
first-order lag at the same 60 Hz that `crates/render/src/camera/chase.rs:35-41`
already uses and defends, and the rising-edge behaviour below is `Chase::snapped`.

## `Exhaust_UpdateEngineSound`

| | |
| --- | --- |
| **Address** | `0x08904cf4` |
| **Confidence** | 80 |

Called from `Exhaust_Update` with the same `dt`.

**This function is audio, not geometry, and it is worth saying so loudly because
it reads exactly like a flame length.** Its output is
`base + speed_kmh * 5.0`, lagged - which is the shape of a trail length, and was
read that way at first. The constructor settles it:

```c
Sound_Play(1.0, self+0x78, ..., "~ENGINE", /* out */ self+0x7c);
```

`self+0x7c` receives a sound instance, so `*(self+0x7c) + 4` and `+ 8` are
**pitch and volume**, not length and brightness. Two further facts agree:

- The constructor initialises `base` (`self+0x80`) to `rand(-127.0, 127.0) -
  1143.0`, i.e. about **-1143** - a negative number that cannot be a length, and
  a per-instance random spread that makes each craft's engine note slightly
  different.
- `self+0x78` is a 0x70-byte emitter with a world position at `+0x50` and `50.0`
  at `+0x38`, the shape of a positional sound with a radius.

Behaviour:

```
on rising edge of engine_on:  lag = pitch          (snap, no sweep-in)
ON :  target = base + speed_kmh * 5.0
      lag   += (target - lag) * rate               (substepped; rate = 0.01 (0x3c23d70a))
      pitch  = lag ;  i += dt * 0.25
OFF:  if base <= pitch { pitch -= 48.0 } ;  i -= dt * 0.5
clamp i to [0,1]
*(self+0x7c) + 4 = pitch                (x 0.3 in the four-corner-hover variant)
*(self+0x7c) + 8 = i * 0.6 + 0.4        (x 0.85 when craft+0x368 is set)
boost_timer = max(0, boost_timer - dt)
```

The intensity `i` it maintains **is** shared with the visual - `Exhaust_Update`
reads `self+0xbc` for the three layer alphas and for `half_size`. So this function
drives the sound directly and the picture indirectly.

## The `Engine Flare` constructor

| | |
| --- | --- |
| **Address** | `0x08905308` |
| **Confidence** | 82 |

Worth recording for what it shows the node owns:

| Field | Contents |
| --- | --- |
| `+0x64` | a 0x210-byte effect object (three colour blocks, direction, two scalars) - the flame |
| `+0x78` / `+0x7c` | the `~ENGINE` sound emitter and its instance |
| `+0xc0` | the craft, found by walking the parent chain |
| `+0x100`..`+0x160` | its own four flare-sprite vertices |
| `+0x160` | a **`<Team>boost.vex`** model, hidden until boosting |
| `+0x180` | the 0x200-byte display list |

**`<Team>boost.vex` gets an answer here.** `docs/formats/vex.md` lists the path
template `%s\%sboost.vex` (`0x08a84ccc`) with no note of what loads it. This
constructor formats that template and calls
`Vex_LoadModel(obj, path, 0x4d000000, 0xfdb2, 0x3e9, 0)` - the same layer byte
`0x4d` the flare's sort key uses. `Exhaust_Update` reveals it, via the `& 4` flag
and `0x08912890`, when `engine_on && boost_timer > 0.2`, and hides it again 1.5 s
later. So it is **the boost visual, parented to the engine-flare node**.

## `Trail`

| | |
| --- | --- |
| **Addresses** | update `0x0892a450`, draw `0x0892a588`, init `0x0892a2b0`, push `0x08929958`, ribbon draw `0x0892acc8`, preset table `0x0892a724`, vertex-colour bake `0x08929c8c`, state list recorder `0x089296f8` |
| **Confidence** | 90 |

Confidence was 78 while the width array, layer count and `+0x1c` scale were read
as shapes only. A 2026-08-02 pass decompiled the preset table, the vertex-colour
bake and the ribbon's own GE state display list, then confirmed the lot against
a live PPSSPP race (breakpoint on `Trail_DrawRibbon`, ring at `0x09a2ad40`,
craft at roughly half intensity) - every recovered value below that has a live
column was read back from the running game and matched.

A genuine position-history ribbon, and a general-purpose class rather than a
ship-specific one.

**`Trail_Update` (`0x0892a450`) takes no `dt`.** It copies the node's world
translation into `self+0x90..0x9c` and pushes **one point per call**, i.e. one per
frame. When `self+0x1f0` is set it pushes `self+0x64` points in a burst - a
prefill so a newly-spawned trail is not degenerate - and clears the flag.

**`Trail_PushPoint` (`0x08929958`)** is a ring buffer: `+0x04` capacity, `+0x0c`
count saturating at capacity, `+0x10` write index wrapping at capacity, points at
`+0x5c` with **stride 0x20 = two vec4s, `{position, direction}`**.

**`Trail_DrawRibbon` (`0x0892acc8`)** draws only when the ring is **full**
(`count >= capacity`), a camera exists (`DAT_08ab10b0 != 0`) and `count > 1` -
those three conditions are its *only* gates; an earlier note here about a
`& 0x20` flag was wrong (nothing in the function tests one). It walks the ring
backwards from the write index, newest to oldest, and for each sample offsets it
as `position + direction * w[i]` (`w` from the array at `+0x138`). Per sample it
emits, for each of `self+0x64` layers:

- 10 vertices, `Gu_DrawArray(GU_TRIANGLE_STRIP, 0x1fe, 10, ...)`, stride 0x20 -
  `0x1fe` decodes as `GU_TEXTURE_16BIT | GU_COLOR_8888 | GU_NORMAL_32BITF |
  GU_VERTEX_32BITF` = 4 + 4 + 12 + 12 = **32 bytes**, matching the `0x140` =
  10 x 0x20 per-segment stride the loop steps by. A second format cross-check of
  the same kind that validated the flare quad.
- **The 10 vertices per segment are a four-fin cross, not one flat quad.** The
  strip alternates the two sample centres offset by `+up`, `+right`, `-up`,
  `-right` and closes back on `+up`: two quads through the trail line,
  perpendicular to each other, both scaled by the same width. `up` and `right`
  are columns 0 and 1 of the camera matrix at `DAT_08ab10b0 + 0x40..0x64`, so
  the cross is camera-aligned. An earlier version of this page read the
  geometry as a single camera-facing quad per segment.
- a width of `layer_width * self+0x1c * taper(i)`. **`self+0x1c` is not the
  constant `1.0` `Trail_InitRing` leaves there**: `Exhaust_Update` overwrites it
  every frame with `intensity * 0.35 + 0.2` (the write this page previously
  described as "child +0x7c", which is the same address), so a racing ship's
  ribbon runs at `0.2`..`0.55` of the authored widths. Live: `0.3737` at
  intensity `0.497`.
- one of the **three** `Engine_noise` texture slots, indexed per layer:
  `Gfx_BindTexture(g_engine_noise_texture[*(iVar24+4)])`. This is why
  `Texture_LoadEngineNoise` writes `0x08b657c0`, `c4` and `c8` - all three hold
  the same handle.
- a scrolling UV offset, `u += du * K * 3.0` and `v += dv * K`, each wrapped to
  `[0,1]` and applied with `sceGuTexOffset`.
- **a flat per-layer colour, sent as GE state rather than as vertex data**: the
  layer's RGBA at block `+0x20` is packed to ABGR8888 and handed to
  `Gu_Ambient` (`0x0881125c`, GE commands `0x5c`/`0x5d` = `sceGuAmbient`). See
  the pipeline section below for why that colours anything at all.

### The ribbon's GE state, from its own display list

`Texture_LoadEngineNoise` records a 0xc0-byte display list into `DAT_08b65700`
at load time (via `Gu_Start`/`Trail_BuildStateList`/`sceGuFinish`), and
`Trail_DrawRibbon` plays it before drawing any layer. Decoded from
`Trail_BuildStateList` (`0x089296f8`), whose helpers each write one named GE
command - the command bytes are in the helpers' own bodies, so this is not an
argument-pattern guess:

```
sceGuSetMatrix(GU_MODEL, identity)        0x08a907a0 is an identity matrix:
                                          the ribbon is submitted in world space
disable FOG, CULL_FACE, LIGHTING, STENCIL_TEST, ALPHA_TEST
enable  TEXTURE_2D
sceGuPixelMask(0)
sceGuStencilOp(KEEP, KEEP, REPLACE)
enable  DEPTH_TEST;  sceGuDepthFunc(GU_GREATER);  sceGuDepthMask(off)
sceGuDepthRange(0xfdb2, 0x3e9)            the transparent layer's range, the
                                          same pair Vex_LoadModel passes
enable  BLEND
sceGuBlendFunc(GU_ADD, GU_FIX 0xffffff, GU_FIX 0xffffff)
sceGuTexWrap(GU_REPEAT, GU_REPEAT)
enable  LIGHTING;  disable LIGHT0..LIGHT3
```

Three consequences, each of which corrects an earlier claim on this page:

1. **The ribbon's blend is `dst + src` - pure additive, not weighted by source
   alpha.** Both factors are `GU_FIX` white. The flare's
   `src.rgb * src.a + dst` reading stands for the flare; it was wrong to assume
   the ribbon shared it. Alpha contributes nothing to the ribbon's colour.
2. **Lighting is *enabled* with all four lights disabled.** That routes the GE's
   lit-colour path: fragment colour = material emissive + ambient light colour x
   material ambient, with every per-light term zero. `Gu_Ambient`'s per-layer
   colour is the *ambient light*; the per-vertex colours are the *material*
   (the first `sceGuStart` of every frame emits `sceGuColorMaterial(7)` -
   `Gu_Start` at `0x0881018c` does it when `DAT_08adc368` is clear - so vertex
   colour feeds all material components). The ribbon's colour is therefore
   `ramp x vertex colour x texture`, **linear in the ramp**. The earlier
   "contribution goes as the ramp squared" reading assumed the flare's blend
   and no vertex colour; both premises were wrong.
3. **What this page previously called an alpha-test ramp is a stencil ramp.**
   The per-segment stepped value feeds `Gu_StencilFunc` (`0x08811914`, command
   `0xdc` = `sceGuStencilFunc`), with func `1` = `GU_ALWAYS` - so it tests
   nothing and, with `StencilOp REPLACE`, only writes the stepped value into
   the framebuffer's alpha/stencil bits. Destination-alpha bookkeeping,
   visually inert for the ribbon itself. Its base `ring+0x20` is not the zero
   `Trail_InitRing` leaves either: `Exhaust_Update` writes
   `intensity * 0.5 * 0.9` there every frame (live: `0.2233` at intensity
   `0.497`). Only the innermost layer (layer 0, drawn last, per-segment
   `Gu_DrawArray` calls) steps it; layers 1 and 2 replay prebuilt per-layer
   display lists (`Trail_ApplyPreset` records them via `0x08929c00`, nine
   `Gu_DrawArray` commands each).

What the list does **not** set is the texture function (`0xc9`): the ribbon
inherits whatever `TFX` the frame left. **Now pinned at confidence 90 rather
than presumed** (2026-08-02, second live pass): scanning a full frame's command
stream at the `Trail_DrawRibbon` breakpoint - main list *and* all 53-64 CALLed
sub-lists - found **zero** `0xc9` commands, so the texfunc is set once and
carried; the one-time set is `Gfx_Init`'s (`FUN_0891f320`)
`sceGuTexFunc(GU_TFX_MODULATE, GU_TCC_RGBA)` (`FUN_0881141c(0, 1)`), and the
driver's shadow state read live confirms it never moved:
`ctx+0xe8 = 0` (modulate), `ctx+0xec = 1` (RGBA), **`ctx+0xe4 = 0` - the
colour-double / `GU_FRAGMENT_2X` bit is off**, with enable-shadow bit 21 clear.
Three other registers the ribbon depends on were pinned the same way:
`MATERIALEMISSIVE` (`0x54`) has **no emitter anywhere in the binary** (the only
`lui 0x5400` candidates are absent), so it holds the GE reset value 0 forever;
the material ambient/diffuse/specular/alpha last written before the ribbon are
all white (`ffffff`/`ff`, read out of the live stream); and the state display
list itself read back from `0x08b65700` **byte-identical to the static
decode** - including `df0000aa`/`e0ffffff`/`e1ffffff` (the pure-additive
blend), `dd020000` (stencil op REPLACE), `e7000001` (depth write off) and the
`0x44/0x47` viewport-z words that resolve the `0xfdb2/0x3e9` pair as
`sceGuDepthRange`, confirming `FUN_0891e988`'s reading.

**One decode subtlety with a visible consequence:** the bake writes u16
texcoords as `fraction * 65535`, but the GE decodes 16-bit texcoords as
`value / 32768`, *unsigned* (PPSSPP `Step_TcU16ToFloat`). Every baked fraction
therefore lands at just under **twice** its written value: the noise advances
`0.2 * texscale` per sample and wraps twice around the four-fin tube. The
scroll offsets are exempt (`sceGuTexOffset` applies after the decode).
`oag_render::exhaust` carries this as `TEXCOORD_U16_GAIN`.

### Why the ribbon can saturate on screen without any hidden gain

Worth recording, because chasing a "missing brightness factor" here cost a
pass: with every register above pinned, the per-fragment contribution really is
`ramp x baked colour x noise`, and its theoretical per-quad maximum
(`~(0.94, 0.56, 1.19)` summed over three layers at the noise texture's own
maximum - `Engine_noise` measures **mean grey 0.325**, max 0.94) cannot reach
broad white saturation. The original's big saturated plumes come from
**additive stacking, not amplitude**: at low speed the ten samples sit almost
on top of each other on screen (at 23 units/s the whole ring spans ~4 world
units), so up to nine segment quads x two tube surfaces x three layers add
into the same pixels; a hard corner does the same by curling the ribbon across
itself; and a boost (pads sit on the racing line) both widens the flare by
`boost_timer * 8` and reveals the additive `<Team>boost.vex` plume. A
straight-line, mid-intensity frame with none of those - captured live at
94 km/h - shows the same compact glow and thin subtle trail the
reimplementation draws. Comparing trail prominence across *different* speeds
measures segment overlap, not fidelity.

### The baked vertex colours: the authored colours are alive, and they fade

`Trail_ApplyPreset` (`0x0892a724`) fills the per-layer vertex buffers once at
init via `Trail_BakeVertexColours` (`0x08929c8c`), and the bake writes each
sample's colour as

```
vertex_rgba(i) = authored_layer_colour * powf(1.0 - fade_step * i, 1.0 / 0.3)
```

with `fade_step` = `ring+0x08` = **0.1** for preset 2 (a field the earlier
reading skipped) and the exponent's `0.3` the global `DAT_08ac00a0`
(`1/0.3 = 3.333`, computed once into `DAT_08af28a4`; `0x0897f6cc` is libm
`powf`). Head to tail that curve runs

```
1.0, 0.70, 0.475, 0.30, 0.18, 0.099, 0.047, 0.018, 0.0047, 0.0005
```

so **the back half of the ribbon is essentially invisible** - this, more than
any other single value, is why the original's exhaust reads as a compact bloom
rather than a streamer. Live confirmation: the white layer's baked vertex
colours at segments 0/2/4/6/8 read `0xd2`, `0x63`, `0x26`, `0x09`, `0x00` -
`210/210, 99/210, 38/210, 9/210, 0` - exactly the curve.

The bake also writes the texcoords: `u` advances `fade_step` per sample (u16,
`0`, `6553`, ... = 0.1 steps, before `sceGuTexScale`'s per-layer 7.5/6.0/4.5),
and `v` runs `0, 0.25, 0.5, 0.75, 1.0` across the five vertex pairs - **once
around the four-fin cross**, not across a flat width.

**The ribbon tapers in width, and does not fade in alpha.** Both readings here were
wrong at first and the correction came from the instructions rather than the
decompiler, at `0x0892ae60`..`0x0892ae84`:

```text
f12  = layer_width * ring[0x1c]      ; ring+0x1c is 1.0
S020 = f12 * S700                    ; S700 varies per sample
vscl.q C730, C000, S020              ; scales the camera basis by it
vscl.q C720, C010, S020
```

`S700` is `point.q0.x + point.q1.x * table[i].x`, i.e.
`ring[0x24] + ring[0x2c] * (i * dt)` = **`1.0 - 0.075 * i`**, reaching `0.325` at
sample 9. So the width runs from full at the head to about a third at the tail -
that taper is the ribbon's entire silhouette. The decompiler renders `S700` as a
position component, which is why it read as nonsense: **the first lane of a trail
point is this scalar and the position occupies the other three**, which also explains
`Trail_PushPoint`'s apparently jumbled field order.

The value stepped down toward the tail from `ring+0x20` feeds `Gu_StencilFunc`
with func `GU_ALWAYS` - a stencil ramp, not the alpha test an earlier version of
this page called it, and visually inert either way (see the GE-state section
above). **The visible fade toward the tail is real, but it is the baked vertex
colour curve**, `powf(1 - 0.1 i, 3.333)` - see the bake section above. "There is
no per-segment alpha fade for a racing craft" was true of the *alpha channel*
and wrong as a statement about the picture.

`Trail_BuildOffsetTable`'s weights come out at `2.6e-6` per sample - but the
direction vector they multiply is **not unit length**. `Exhaust_Update` writes
`child+0xa0` as `-(craft matrix row 2) * 200000.0` (`DAT_08a84c40`), and the
craft's row 2 carries the global `0.75` model scale, so the stored vector's
magnitude is about **150,000** (live: `150,080`). The displacement at sample `i`
is `150000 * (1 - exp(-i * dt / 80)) / 80` - `0` at the head rising to about
**3.5 world units** at the tail. A modest backwards stretch, weighted toward
the tail; "numerically nil" was an error from evaluating the weights without
the vector's magnitude.

## Textures

| Path | String | Handle | Loader |
| --- | --- | --- | --- |
| `Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip` | `0x08a84c80` | `g_engine_flare_texture` `0x08b62908` | `0x08905210` |
| `Data\Tex\engineFlare\Engine_noise.mip` | `0x08a889e4` | `0x08b657c0`/`c4`/`c8` | `0x0892a5a4` |

Both are `.mip`, decoded by `crates/formats/src/texture.rs`. Both paths are
**literal strings in the executable**, so the WAD lookup is an exact
`wad::hash_name` CRC-32 hit rather than a mined candidate.

## Cross-platform

| Platform | Notes |
| --- | --- |
| PSP (Pulse) | this page |
| PS2 (Pulse) | **asset and class-vocabulary parity is exact**; draw path not compared |

`SCES_547.48` carries the same `grabbedEngineFlare128x64x8.mip` (`0x002b7f90`)
and `Engine_noise.mip` (`0x002be7d0`) paths, the same `Engine Flare` /
`exitglow` / `engine_fire` / `ParticleSystem` class names, the same 25 `.POB`
set, and the same absence of any engine `.POB`.

**That is corroboration of the structure, not of the draw path.** The PS2 has no
GE; `ExhaustFlare_Draw`'s counterpart is VU1/GS code that was not compared, and
no PS2 rendering function is named anywhere in this project yet.

PS2 additionally carries three strings the PSP lacks -
`Data\Psys\Tex\orange_glow2.tga` (`0x002bcd58`),
`Z:\WipeoutPSP\Art_Resources\Psys\Tex\psysed_default_glow.tga` (`0x002b79b8`),
and the diagnostic `PSYS WARNING: NOT RENDERING SYSTEM WITH CORRUPTED SPRITE PAGE
INFO` (`0x002bcd78`). The last is a named anchor into the PS2 particle renderer
if that side is ever wanted.

Per [`goals.md`](../../../overview/goals.md#scope), the PSP side is what gets
implemented.

## History

- **2026-08-08** - the three opens from 2026-08-07 worked through. **The
  flare stretch is closed: there isn't one** (square quad at instruction
  level, round glow in UV space, aspect-exact projection read live);
  `FLARE_ASPECT` retired from `oag_render::exhaust`. **The fov chain is
  recovered end to end** and written up on [camera.md](camera.md) - five
  links, each pinned by a PPSSPP memory write breakpoint - leaving only which
  of its two paths a pad uses. **The plume's alpha question is sharpened
  rather than closed**: the authored alpha is a two-value rim/core split, the
  texture's alpha is constant, and the blend is confirmed unoverridable.
- **2026-08-07** - the flare's coordinate space corrected by a live matrix
  read at `ExhaustFlare_Draw` (view-space billboard, world-sized extents,
  projection live; `HALF_SIZE_TO_WORLD` retired to `1.0` and the fitted
  `2.15` superseded). First tick-addressable capture of a real pad crossing:
  the boost fov widening measured (~90-95 degrees against the authored 60,
  settling by age ~1.3 s), the plume's vertex alpha shown to reach the
  original's picture despite the fixed-factor blend, and the entry white-out
  explained by quad size at close camera distance. Three matching entries
  added under Open. Earlier history lived in the sections above and in git.

## Applied names

Per [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md): below 70
gains a `_q` suffix, below 50 is not renamed at all. Mirrored in
[`names.tsv`](names.tsv).

| Address | Kind | Name | Conf |
| --- | --- | --- | --- |
| `0x08908eb8` | function | `Vex_RegisterClass` | 90 |
| `0x08985ff8` | function | `Math_TransformVec4` | 90 |
| `0x08904a30` | function | `ExhaustFlare_Draw` | 90 |
| `0x08904c18` | function | `ExhaustFlare_BuildDisplayList` | 80 |
| `0x0890490c` | function | `ExhaustFlare_Submit` | 85 |
| `0x089058b0` | function | `Exhaust_Update` | 85 |
| `0x08904cf4` | function | `Exhaust_UpdateEngineSound` | 80 |
| `0x08905308` | function | `ExhaustFlare_Init` | 82 |
| `0x08904540` | function | `ExhaustFlare_Destroy` | 75 |
| `0x0892a450` | function | `Trail_Update` | 80 |
| `0x0892a588` | function | `Trail_Draw` | 80 |
| `0x0892a2b0` | function | `Trail_Init` | 75 |
| `0x08929958` | function | `Trail_PushPoint` | 85 |
| `0x0892acc8` | function | `Trail_DrawRibbon` | 90 |
| `0x08905210` | function | `Texture_LoadEngineFlare` | 85 |
| `0x0892a5a4` | function | `Texture_LoadEngineNoise` | 85 |
| `0x08928460` | function | `Gfx_BindTexture` | 75 |
| `0x0892a724` | function | `Trail_ApplyPreset` | 88 |
| `0x08929c8c` | function | `Trail_BakeVertexColours` | 88 |
| `0x089296f8` | function | `Trail_BuildStateList` | 85 |
| `0x08929a74` | function | `Trail_BuildOffsetTable` | 80 |
| `0x0892a050` | function | `Trail_InitPreset` | 80 |
| `0x0881018c` | function | `Gu_Start` | 88 |
| `0x0881125c` | function | `Gu_Ambient` | 90 |
| `0x08811914` | function | `Gu_StencilFunc` | 88 |
| `0x08811948` | function | `Gu_StencilOp` | 88 |
| `0x0881197c` | function | `Gu_BlendFunc` | 90 |
| `0x08811484` | function | `Gu_TexWrap` | 88 |
| `0x08811874` | function | `Gu_DepthMask` | 85 |
| `0x08811850` | function | `Gu_DepthFunc` | 85 |
| `0x08810db8` | function | `Gu_Enable` | 90 |
| `0x08810e10` | function | `Gu_Disable` | 90 |
| `0x08810a60` | function | `Gu_ColorMaterial` | 85 |
| `0x08810e6c` | function | `Gu_SetMatrix` | 88 |
| `0x08b62908` | data | `g_engine_flare_texture` | 88 |
| `0x08b657c0` | data | `g_engine_noise_textures` | 80 |
| `0x08b65700` | data | `g_trail_state_list` | 85 |
| `0x08ab2370` | data | `g_vex_class_table` | 92 |

The `Gu_*` helpers carry 85-90 because their bodies write the GE command byte
directly (`cmd << 24 | args` into the current list) - the command numbers are
the PSP GE's own, so the mapping is read, not inferred from call sites. The
2026-07-31 caveat about `0x08810db8`/`0x08810e10` being identified "by the
argument pattern rather than by their bodies" is retired: both bodies are now
read (they maintain a shadow enable-bit word at `DAT_08adc3a8` and emit through
`0x08811b58`), and the state indices are `sceGu`'s own enum.
| `0x0891e35c` | function | `Gfx_Enqueue` | 65 |
| `0x0890486c` | function | `Gfx_ViewDepth` | 60 |
| `0x0883d850` | function | `Ship_ThrustInput` | 55 |

`Vex_FindClassDescriptor` (`0x08908b68`) and `Vex_LoadModel` (`0x08912b80`) were
already named by earlier passes; `Gu_DrawArray` (`0x08810e98`) and `Gu_CallList`
(`0x08810598`) likewise, via [`vex.md`](../../../formats/vex.md).

Not renamed, deliberately: `0x08929c00` (the per-layer draw-list recorder; nine
`Gu_DrawArray` commands, too thin a body to earn a name over its caller's), and
`0x0897f6cc` (libm `powf` by structure - errno handling, subnormal paths - but
library identification is a different bar; the bake section cites it by
address). The earlier entries here for `0x0892a050` and the enable/disable pair
are superseded: all three are now read and named above.

## What the shipped data actually contains

Measured with `oag-view --nodes`, added in this pass because nothing could print a
node the parser does not decode. Against the PSP disc's `Data.wad`:

| File | `Engine Flare` `0x3bf` | `Trail` `0x3c8` | `exitglow` `0x3e4` |
| --- | ---: | ---: | ---: |
| `Ship.vex`, 8 teams | **1 each** | 0 | 0 |
| `shipwreck.vex` | 1 | **2** | 0 |
| `shipboost.vex` | 0 | 0 | 0 |
| `16_Track\track.vex` | 0 | 0 | **13** |

- **A ship has exactly one `Engine Flare`**, named `engine_flare`, a direct child
  of `world` with a 64-byte payload (a 4x4, like the other locator classes). So
  there is one nozzle, centred - not one per visible engine. Identical on all
  eight teams whose `Ship.vex` resolves by name in `Data.wad` (`AG_Systems`,
  `Assegai`, `EGX`, `Feisar`, `Goteki`, `Piranha`, `Qirex`, `Triakis`). The other
  five (`Auricom`, `Harimau`, `Icaras`, `Mirage`, `Van_Uber`) have no entry
  hashing to `Data\Ships\<Team>\Ship.vex` - a name-mining gap, not a decode
  failure.

- **`Trail` reaches a racing ship in code, and an earlier version of this page said
  it could not. That was wrong.** The correction is recorded rather than quietly
  replaced, because the reasoning that produced it is a trap worth naming.

  `ExhaustFlare_Init` allocates a 0x210-byte child at `flare+0x64` and calls
  `Trail_InitPreset` (`0x0892a050`), which sets `*(obj+0x38) = &DAT_08ad226c` -
  **the `Trail` method table** - initialises the ring at `obj+0x60` and fetches the
  same type id `Trail`'s own registration does (`0x08a6bd00`). So **every racing
  craft has a `Trail`**, constructed programmatically, and the three staggered
  intensity ramps in `Exhaust_Update` are *its* per-layer colours.

  The refuted claim was "a `Trail` can only come from an authored node, because its
  class descriptor (`0x08b654f0`) is referenced by nothing but its own registration
  function". The premise is true and the conclusion does not follow: **the
  descriptor is only used by the class-table lookup path**, which is how the `.vex`
  loader instantiates a node by ID. Code that builds one directly calls the
  constructor and assigns the vtable itself, touching the descriptor never. An
  xref count on a descriptor therefore bounds *authored* instances only, and says
  nothing about programmatic ones - which `<Team>boost.vex`, loaded in code by the
  same constructor, should already have suggested.

  What actually caught it: **the emulator shows a trail behind the player's craft**,
  and no screenshot comparison against PPSSPP had been attempted. A single side by
  side would have refuted the claim immediately.

  `Trail` is *also* authored twice on `shipwreck.vex`, named
  **`trail_con_left_wing`** and **`trail_con_right_wing`**. Both readings are true;
  the wreck one is not the whole story.

### The ship's trail parameters, from preset 2

`Trail_InitPreset` is a table of three presets and `ExhaustFlare_Init` passes **2**:

| Field | Value | Reading |
| --- | --- | --- |
| `ring+0x04` | 10 | ring capacity, in samples |
| `ring+0x08` | 0.1 | **per-sample colour fade step** for the vertex bake (a field the first reading skipped; presets 0/1 use 1/12 and 0.125) |
| `ring+0x1c` | 1.0 at init | width scale - **overwritten every frame** by `Exhaust_Update` with `intensity * 0.35 + 0.2` (live: `0.3737` at intensity `0.497`), so the shipped `1.0` never survives a race tick |
| `ring+0x20` | 0 at init | stencil ramp base - overwritten every frame with `intensity * 0.5 * 0.9` (live: `0.2233`); visually inert |
| `ring+0x2c` | -4.5 | **taper rate**, derived as `-(ring[0x24] / (capacity * dt)) * 0.75` |
| `ring+0x24` | 1.0 | **width taper at the head** |
| `ring+0x64` | 3 | layer count |
| `ring+0x60` | 90 | `capacity * 10 - 10`, vertices per layer |

Per-layer blocks are **stride `0x30` from `ring+0x78`**, and laying them out is what
makes the fields readable:

| In block | Layer 0 | Layer 1 | Layer 2 | Reading |
| --- | ---: | ---: | ---: | --- |
| `+0x00` | 1.0 | 0.7 | 0.5 | half-width, in world units |
| `+0x08` | 0 | 0 | 0 | `u` scroll offset, animated |
| `+0x0c` | 0 | 0 | 0 | `v` scroll offset, animated |
| `+0x10` | -1.5 | -3.0 | -6.0 | **`u` scroll rate** |
| `+0x14` | 1.0 | -1.0 | 1.0 | **`v` scroll rate** |
| `+0x18` | 7.5 | 6.0 | 4.5 | **`u` texture scale** (`sceGuTexScale`) |
| `+0x1c` | 1.0 | 1.0 | 1.0 | `v` texture scale |
| `+0x20`..`+0x2c` | see below | | | RGBA |

**`+0x10` is a scroll rate, not a position offset**, and an earlier version of this
page had it as the latter - which put the three layers up to 6 units apart along the
exhaust and stretched the plume badly. `Trail_DrawRibbon` advances

```
u += rate_u * dt * 3.0      (only u carries the 3.0)
v += rate_v * dt
```

wraps both to `[0,1]` and passes them to `sceGuTexOffset`. `dt` is
`_DAT_08a889d0` = `0x3c888889` = **0.016666668** - the same 1/60 the lag substeps
use, so at this project's fixed rate the two agree exactly. The middle layer scrolls
`v` the other way, which is what stops three copies of one texture reading as one
texture at three brightnesses.

What settles the layout is that the *same* stride-`0x30` block holds the colour
fields `Exhaust_Update` demonstrably writes, so the block boundaries are not a
guess.

The three layer colours land at `ring+0x98`, `+0xc8`, `+0xf8` - the `+0x20` field of
each block. In child coordinates those are `+0xf8`, `+0x128`, `+0x158`, **exactly the
three addresses `Exhaust_Update` writes**. That match is what ties the staggered
ramps to the ribbon rather than to three concentric quads, which is how they were
first read.

**The preset's own colours are alive, and an earlier version of this page
declared them dead.** Layer 0 is a deep blue `(0, 0.031, 0.502)`, layer 1 a
magenta `(0.608, 0, 0.486)`, layer 2 a near-white `(0.824, 0.824, 0.824)` (all
three authored alphas are `0`, which the pure-additive blend ignores).
`Exhaust_Update` does overwrite all four channels of all three ring colour
fields every frame with the staggered grey ramp - that part of the old reading
stands, and the tint at `flare+0x68`..`+0x74` it multiplies in is confirmed
written `(1, 1, 1, 1)` in the same function, every frame. What the old reading
missed is that **`Trail_BakeVertexColours` copied the authored colours into the
per-vertex data at init, before `Exhaust_Update` ever ran**, and the GE
multiplies the two: the per-frame ring value goes out as `sceGuAmbient` (the
grey intensity ramp), the baked authored colour rides in the vertices (the
material, via `sceGuColorMaterial(7)`), and the fragment is their product. The
ribbon is a blue outer / magenta middle / white core, each scaled by its grey
ramp - which is exactly the violet-white bloom the 2026-07-30 reference frame
shows. Live: ring colours read `(0.347, 0.347, 0.347, 0.347)` /
`(0.229, ...)` / `(0.0, ...)` at intensity `0.497` - the ramp formula to four
digits - while the baked vertex colours still held the authored values times
the fade curve.

The "contribution goes as the ramp squared" paragraph that used to stand here
is retracted: it assumed the flare's `src.a`-weighted blend applied to the
ribbon, and the ribbon's own display list sets `GU_FIX`/`GU_FIX` white instead.
The ramp contributes **linearly**, once, through the ambient register.

`Trail_BuildOffsetTable` (`0x08929a74`), called with `80.0`, fills `ring+0x138`
with the per-sample offset weights `Trail_DrawRibbon` indexes: `+0x28 = -1/80` and
weight `w(t) = (-1/80) * (exp(-t/80) - 1)`, an exponential ease rather than a
linear trail-off. Confidence 70 - `0x0897f16c` is read as `expf` from its use.

**Because capacity is 10 and `Trail_DrawRibbon` refuses to draw until the ring is
full, the trail needs 10 ticks of history before it appears at all.**

  Worth recording how a stronger claim was avoided: an intermediate reading here
  said "nothing in the shipped data authors a `Trail`", from 8 ships plus one
  track. That was wrong, and the reason it looked right is that `Trail` lives in a
  *sibling* file reached through a path template rather than in `Ship.vex`. The
  check that would have caught it earlier is the one that eventually did:
  **resolve the path templates** (`%s\%swreck.vex`, `%s\%sboost.vex`,
  `%s\%sshield.vex` at `0x08a7babc` / `0x08a84ccc` / `0x08a7c178`) rather than
  guessing filenames. The first `%s` is the team directory and the second is the
  FE team-model name, whose default is the literal string `"ship"` at
  `0x08a84cb4` - hence `shipwreck.vex`, not `Assegaiwreck.vex`. Three previously
  undocumented archive entries fall out of that.

  A `Trail` can **only** come from an authored node: its class descriptor
  (`0x08b654f0`) is referenced by nothing but its own registration function, so
  there is no code path that constructs one. That rules out a `Trail` being
  attached at runtime to remote craft, which is otherwise a reasonable guess given
  that `ExhaustFlare_Init` does attach `<Team>boost.vex` in code.

- **`exitglow` is a track class, not a ship one** - 13 on `16_Track`, 0 on every
  ship. So it is scenery and tunnel glow, and the earlier guess that it might be
  part of the ship exhaust is wrong.

### What a boost actually changes

Read out of `Exhaust_Update` (`0x089058b0`) in full, prompted by a report that
our flare looks too strong while our trail does not react at all. Both halves of
that report are explained, and only one of them is a bug.

**`boost_timer` (`self+0xb8`) reaches exactly three things.** Nothing else in
the function reads it:

1. the flare's half-size, `self+0xc4`,
2. the reveal of the `<Team>boost.vex` plume, and
3. the `engine_on` flag (`self+0x94`) - with thrust off, `boost_timer > 0.2`
   alone keeps the engine counted as on, which feeds the engine sound and the
   intensity ramp. `oag_render::exhaust::Exhaust::advance` already implements
   this third use (the `engine_on` line); only the "exactly two" phrasing here
   and in `oag_render::exhaust::BOOST_SECONDS`'s doc comment was off by one,
   corrected 2026-08-04.

**The `Trail` gets nothing from a boost.** Its two per-frame parameters and all
three layer colours come from `intensity` (`self+0xbc`) alone:

```c
trail->0x7c = intensity * 0.35 + 0.2;          // ring+0x1c, the width scale
trail->0x80 = intensity * 0.5  * 0.9;          // ring+0x20, the stencil ramp base
a0 = clamp(intensity          * 0.7, 0, 1);    // layer 0 colour multiplier
a1 = clamp((intensity - 0.25) * 1.33 * 0.7, 0, 1);
a2 = clamp((intensity - 0.5)  * 2.0  * 0.7, 0, 1);
```

So **the ribbon not reacting to a pad is faithful**, and at racing speed
`intensity` is already saturated, so it cannot react. Confidence **90** - an
exhaustive read of the one function that writes these fields.

**The size law is confirmed exactly**, which retires any doubt about the four
constants `oag_render::exhaust` carries:

```c
self->0xc4 = (intensity * 0.6 + 0.4) * 2.5 + boost_timer * 8.0;
```

At `intensity = 1` that is `2.5` at rest and `8.9` at the pad's armed `0.8`, so
the original's flare really is **3.56x** wider during a boost. The ratio is the
original's; only `oag_render::exhaust::HALF_SIZE_TO_WORLD` is ours, and it was
fitted on a *resting* frame.

**The conclusion for the renderer, and its resolution.** The original spends a
boost across two visuals - a much bigger flare *and* an additive plume - and
until 2026-08-04 this crate drew only the first. So the flare carried the whole
effect, which was exactly the reported symptom. **Detuning `HALF_SIZE_TO_WORLD`
would have been the wrong fix**: it would have broken the resting case to
compensate for a mesh that was not being drawn. The actual fix, now shipped,
is drawing `<Team>boost.vex`: `oag_render::exhaust::Exhaust` tracks the
plume's own 1.5 s reveal timer (`plume_timer`/`plume_visible`,
`PLUME_SECONDS`) beside the flare's, and `oag_game::race::Loaded::boost_model`
loads `Data\Ships\<Team>\shipboost.vex` for `Race::Scene` to draw additively
alongside the ship, model-matrix and all, whenever `plume_visible()` is true.

### Measured on a live crossing: the fov widens, and the plume needs its alpha

Two findings from the first tick-addressable capture of a real pad crossing
(2026-08-07: Talon's Junction pad 0, `psp-drive.py place` +
`psp-trace.py --camera --shot-every 2`, then our frames re-rendered from the
captured rows with `--pose-from`/`--pose-boost` for matched-pose comparison):

- **The original widens its field of view during the boost.** Trackside
  geometry (the B2 tower) reads ~1.4-1.5x smaller at boost age 0.3-0.7 s than
  at 1.3 s+, from a recorded camera pose that barely moves, and an fov sweep
  of our posed render lands the boosted frame at roughly 90-95 degrees
  against the authored 60. The widening outlives the 0.8 s flare timer
  slightly and settles back by about age 1.3 s. **This is a live measurement,
  not a binary read** - no code doing it has been located - and it means
  `oag_game`'s `BoostFovKick`, shipped as an invented effect, has a real
  counterpart whose magnitude sits nearest the strongest offered tier.
  Confidence **75**: one scenario, one crossing, tower-width pixel
  measurement.

  **The last sentence of that bullet is withdrawn, 2026-08-08.**
  `BoostFovKick` is a deliberate invention of this project and is not being
  fitted to the original; "has a real counterpart whose magnitude sits nearest
  the strongest offered tier" read as a calibration instruction and was never
  meant to be one. The measurement itself stands as an observation about the
  original, and [camera.md](camera.md) now records what actually moves the
  original's fov - but the two are not being reconciled, and the `90-95`
  figure must not be used to retune ours. See the [Open](#open) list.
- **The authored vertex alpha of `shipboost.vex` visibly reaches the
  original's picture**, even though the recovered blend
  (`GU_FIX`/`GU_FIX` additive, modulate texfunc) has no path for it: a build
  that ignored alpha entirely drew the plume's streak fins as hard-edged
  solid sheets, while the original's plume feathers away exactly where the
  authored alpha fades. Where the GE applies it is **open** - candidates
  include a texfunc or colour path not yet read for this batch type - and
  until that is settled `race.rs` premultiplies the alpha into the vertex RGB,
  which reproduces the observed picture under the fixed-factor blend.

The capture also shows the entry flash plainly: at crawl speed the enlarged
flare quad (half-size `~8.9` at a camera distance of a few units) covers the
frame and the first ~10 ticks after entry read as a near-total white-out,
tapering as `boost_timer` decays.

### The plume's alpha: what the authored data actually holds

2026-08-08. The question "how does the authored vertex alpha reach the GE" was
asked of the code and answered by the data instead, and the answer changes the
question.

**There is no alpha in the blend, and nothing downstream can add one.** The
literal call in `Mesh_SetBatchDrawState` is
`Gu_BlendFunc(0, 10, 10, 0xffffff, 0xffffff)` - `GU_ADD`, `GU_FIX` white on
both sides - and reading its caller `FUN_089307b4` settles the ordering that
was the last hope for an override: in the transparent (`& 2`) group the
material's own display list (`Gu_CallList(material + 0xc0)`) is replayed
**before** `Mesh_SetBatchDrawState`, so the blend function it programs is the
one that stands. The one remaining candidate on this page,
`FUN_0891e988`'s fourth argument, is not an alpha reference either: its whole
body is `sceGuDepthRange(ref * 6 + lo, ref * 6 + hi)`, a per-batch depth bias.
[mesh-draw.md](mesh-draw.md) is corrected.

**The authored alpha is not a fade.** Read out of `shipboost.vex` through
`oag_formats::vex::mesh_batches`, on every batch of all eight PSP teams, the
vertex colours take exactly **two** values:

| Colour | Alpha | Count in a 51-vertex batch |
| --- | ---: | ---: |
| `(255, 98, 5)` orange | `0` | 29 |
| `(255, 255, 255)` white | `255` | 22 |

Nothing in between, on any batch. So the thing an earlier pass described as
"the authored alpha fading" is a **rim/core colour split**: the fin's outer
vertices are orange with alpha 0 and its inner ones white with alpha 255, and
the gradient across the fin is the interpolation between them. The texture
does not supply a fade either - `pulse_boost2_ADD` is 64x16 with a **constant
alpha of 238** across every texel, so the whole "maybe the texture's alpha
feathers it" branch is dead without a further experiment. Its RGB *is* a
bright-to-dark streak gradient, mean 108, which is the only falloff in the
material.

That one split explains both of the failures this project has seen:

- premultiplying alpha into the RGB, which `race.rs` does today, maps the
  orange rim to **black** - so the plume reads white and un-orange, which is
  exactly the reported symptom that opened this task;
- leaving the RGB alone draws the rim at **full orange**, and the fins come
  out as hard-edged solid orange wedges. Rebuilt and screenshotted 2026-08-08
  (`data/shots/boost-visuals/`, `--pose-boost 0.5`), which reproduces the
  2026-08-07 result exactly rather than contradicting it.

The original sits between the two, so something scales the rim's contribution
down without discarding its hue, and it is not the blend. The premultiply
stays, labelled in `race.rs` as a compensation with no recovered mechanism.

**One suspect was raised and refuted the same day, and the refutation is the
more useful half.** The plume's authored `u` never leaves the first texel -
`[0.000, 0.008]` against a `v` of `[0.031, 0.953]`, one step of an 8-bit
texcoord on a 64x16 texture - so every fragment samples a single column of
`pulse_boost2_ADD` and the streak gradient never reaches the picture. There is
even a mechanism that would fix that in the original:
`Mesh_SetBatchDrawState`'s caller applies a per-material
`Gu_TexScale`/`Gu_TexOffset` pair through `FUN_089271cc` when the runtime
material's `& 0x10` bit is set, and the plume's material flags are `0x212`,
which has it. But rendering our plume with `u` scaled by 128, and again with
`u`/`v` swapped, changes the picture **not at all** - the fins stay solid
orange wedges. So where the plume samples its texture is not what is wrong
here, and the search has to look elsewhere.

**Nor is the texture dropped or mis-bound**, which was the next guess and is
now excluded by a direct test: replacing `pulse_boost2_ADD` with hard
horizontal stripes puts **visible bands on the plume's two trailing streaks**
while leaving the wedges flat. The sampler, the bind and the draw's texture
index are therefore all correct for at least half the model. What that test
did expose is the wedges' own UVs - the two short batches (9 and 10 vertices,
the all-white-alpha ones) decode to a **single point**, `u [0.008, 0.008]`
and `v [0.031, 0.031]`, so those draws sample one texel by construction and no
texture content can reach them whatever the scale. Whether that is our decode
or the authored data is open; the 51-vertex batches carry a real `v` sweep and
do band. A first stripe test striped `u` instead of `v` and showed nothing,
which is a false negative worth naming: **stripe the axis the geometry
actually varies in**, or the test proves only that the constant axis is
constant.

**That does upgrade `FUN_089271cc` on its own account**, independently of the
plume: [texture-animation.md](texture-animation.md) carries it at confidence
45 as "a two-line helper ... the most likely home of a general per-material
texture transform". It is that, and the gate is now read - the runtime
material's `& 0x10` bit, tested in `FUN_089307b4` at three separate call
sites. The on-disc material's `+0x0c..0x14` is zero on the plume as it is on
every track surface, so where the runtime block's scale and offset come from
is still unread, and no rename is proposed here.

### The boost visual, and settling the mount question

`Ship.vex` carries two `Transform` locators named `boost_flare` and
`boost_flare1`, children of `world`, 64 bytes each. `shipboost.vex` carries
exactly two meshes, **`bflare1Shape`** and `bflare2Shape`, textured
`Data/Tex/pulse_boost2_ADD.tga` - and the `_ADD` suffix is the artists' own note
that it composites additively, agreeing with the blend function recovered from
`ExhaustFlare_BuildDisplayList`. The name pairing looked strong, but nothing
traced the locators to the meshes at instruction level, so the mounting sat at
**confidence 60**, then 75 once a player's independent description of the
original ("expanding to almost the width of the ship, flaring along the wings
at the back") confirmed two wing-spread flares was the right shape rather than
one.

**Settled at 80, from two more angles, neither of them instruction-level but
both converging on the same answer.** First: `BOOT.BIN`'s string table has no
`boost_flare` string anywhere in it - only `StartBoost`, `boostMul`,
`boostimg`, the HUD label and the `%s\%sboost.vex` path template - and this
engine finds scene nodes through class-name strings (`"Engine Flare"` is
present as one), so no code path looks a `boost_flare` locator up by name. If a
mount exists at all it would have to be data-driven inside the `.vex` itself,
not driven from `BOOT.BIN`. Second, and decisively: rendering
`shipboost.vex` (`oag-view --mesh 'Data\Ships\Assegai\shipboost.vex'`) and
walking its node tree (`oag-view --nodes`) shows `bflare1Shape` and
`bflare2Shape` as **direct children of `World`, with no `Transform` node
between them** - the file's own scene graph carries no positioning layer for
either mesh to mount on. The two meshes are already spread apart at wing
width in their own baked vertex data, visible in the render without any
locator applied. So the `boost_flare`/`boost_flare1` locators are very likely
unused for this purpose, or used by something this crate does not yet reach
(a different node, another file); either way, mounting the plume through them
would be inventing a transform chain the format does not carry. **This is why
the renderer draws `shipboost.vex` once, in ship space, with no per-locator
mounting.**

`ExhaustFlare_Init` (`0x08905308`) is where it is loaded, and the path really is
built per team: it formats `"%s\%sboost.vex"` (`0x08a84ccc`) and calls
`Vex_LoadModel(child, path, 0x4d000000, 0xfdb2, 0x3e9, 0)`, then immediately
**clears the visibility bit** (`+0x2c &= ~4`), so the plume is hidden from birth
and only `Exhaust_Update` ever shows it. `child+0x8 = *(flare+0xc0)` and
`Attach(*(flare+0xc0), child)` right after - `flare+0xc0` is the flare's
ancestor matching the ship class, the same pointer `Exhaust_Update` reads
thrust through - so **the plume is parented to the craft, not to the flare**,
sitting at the craft origin as a sibling of the hull rather than at the
nozzle. Had it been parented to the flare it would need the flare's own world
position; parented to the craft, the ship's own model matrix is the right one,
which is what `Race::Scene` uses to draw it.

**Two of the eight PSP teams carry extra junk beside the three `0x000` nodes
every team has** (below): Assegai has one extra `Airbrake` node
(`ship:cockpit_etc:Airbrake_Left`, class `0x3c5`, 64 bytes), and Goteki has
one `Transform` (`ship:cockpit_etc:screen`, class `0x6e`) plus *two*
`Airbrake` nodes. All are direct children of `World`, siblings of the two
`Mesh` nodes rather than ancestors of either, so they do not disturb the
"no `Transform` between `World` and a `Mesh`" reading above. Measured across
all eight teams with `oag-view --nodes`, unfiltered - an earlier pass here
generalised from Assegai alone and got the shape wrong for the other seven.
Left unexplained rather than invented an origin for: none of it affects the
decode, since `mesh::build_with_textures` only reads `Mesh` and `Texture`
nodes.

`shipboost.vex` and `shipshield.vex` also each carry three class-`0x000` nodes
named `ViewCompass`, `UniversalManip` and `UniversalManip1`, with zero-byte
payloads, on every team. Those are **Maya viewport furniture exported by
accident** - the manipulator gizmos and the view compass - not a decode
failure. Recorded so the next reader does not go looking for a class `0`
handler.

**Not every team ships one.** `Auricom`, `Harimau` and `Icaras` carry no
`Ship.vex` - and so no `shipboost.vex` - on the PSP disc at all, and are
confirmed PS2-only (they resolve there - see `ps2_source_ground_truth.rs`'s
own `TEAMS`). Two more candidate names, `Mirage` and `Van_Uber`, resolve on
**neither** disc under this project's current name-mining - an open gap,
not evidence either way about which platform ships them - which is why the
split above ("eight teams whose `Ship.vex` resolves by name") is phrased as
eight of *thirteen* candidates, not eight of eleven; `docs/formats/handling-stats.md`
counts the same eight independently, over `handlingstats.xml` rather than
`Engine Flare`. `Loaded::boost_model` is `None` for a team with no entry
rather than a load failure, and `crates/game/tests/boost_plume_ground_truth.rs`
pins what is actually established: the eight PSP teams' `shipboost.vex`
decoding with exactly two meshes, a resolved texture and all four batches
landing in the transparent-draw list `exhaust::BLEND` needs to reach, and the
three *confirmed* PS2-only teams having no PSP entry to find.

## The boost, measured against a real crossing end to end

2026-08-08. The first pad capture that carries the flare's **own state per
tick** rather than an inferred boost age, taken on Talon's Junction pad 0 with
`psp-drive.py place` onto the pad's own axis and
`psp-trace.py --camera --flare --shot-every 2`, 150 ticks across the approach,
the boost and the plume's whole life. `--flare` is new and walks
`craft+0x1c4 -> +0x78`, checked against the flare's own `+0xc0` owner field
(see `scripts/psp_trace_fields.py`).

**Four recovered values are now confirmed on live data, not just read:**

| Claim | Recovered | Measured |
| --- | --- | --- |
| the flare timer is a `0.8` code literal | `0.8` | max `boost_timer` `0.783316` = `0.8 - dt` exactly |
| it decays by `dt` per tick in `Exhaust_UpdateEngineSound` | `-dt` | `0.7666 -> 0.7166` over 3 ticks = `3 x 0.016667` |
| the plume runs a separate 1.5 s timer, zeroed at the reveal | `1.5`, zeroed | `0.017` on the reveal tick, hidden once past `1.502` |
| `half_size = (i * 0.6 + 0.4) * 2.5 + boost_timer * 8.0` | - | `7.351` against `7.333` x flicker at `i = 0.1334`, `boost_timer = 0.7666` |

The flare's colour word read `0xd7ffffff` at rest - white, alpha `0xd7` = 215,
inside the recovered `rand_int(200, 255)`. **The trail's non-reaction is
confirmed too**: nothing in the ribbon moves on the entry tick.

One thing a teleported capture gets wrong if it is not watched: **the intensity
ramp is nowhere near saturated.** It climbs at `0.25`/s from wherever the
craft's idle time left it, and this capture entered the pad at `0.1292`, not
`1.0`. That is not cosmetic - intensity sets the flare's resting half-size
(`1.2` here against a saturated `2.5`) and all three ribbon layer alphas - so
`oag-game` grew `--pose-intensity` and `--pose-speed` to pose a frame at a
captured row's *measured* exhaust state rather than a saturated one.

### The craft's `0.75` render scale, confirmed three ways - and a residual that is not it

**This is a large part of why the boost reads weak, and it is not an exhaust bug
at all.** [`CRAFT_ROW_SCALE`](../../../../crates/render/src/exhaust.rs) has
recorded since the trail-direction read that the original's craft world matrix
carries a global `0.75` (`200000.0` times row 2, live `150,080`). The rigid
body's own rows are orthonormal - the capture harness measures them unit-length
over 200 ticks - so the scale belongs to the **render** matrix, and
`oag_game::race::Race::ship_model_matrix` builds rotation and translation only.

**Applied 2026-08-08** after a third independent confirmation, and the full
argument is on that function's own doc comment. It was briefly reverted the same
day on the reasoning that a constant leaving a residual is a fix only in
appearance; what changed that is the confirmation below, plus the point that a
simulation scaled by `0.75` and a mesh drawn at `1.0` is the one combination
that is certainly wrong.

**The third confirmation is the drop shadow.** `FUN_089038c8` is a stencil
shadow-volume pass over the craft; it computes its ground projection, scales it
by `1.0 / g_craft_scale`, and then `Gu_SetMatrix(2, ...)` installs the node's own
4x4 as the GE world matrix. **Dividing the scale back out is only correct if the
matrix it draws under carries it.** With the live trail-direction read
(`200000.0 * row2` measured `150,080`) that is two readings of the render path,
and `oag_physics::hover::TARGET_GLOBAL_SCALE` is the same global already applied
in the simulation - `g_craft_scale` has seventeen xrefs including
`Ship_HoverFourCorner`, `Ship_InitCraft`, `Ship_UpdateCraft` and
`Ship_UpdateCameraRigs`.

Caught by putting a real emulator frame beside ours from the original's own
recorded camera, 11.6 units back: the wingtip lamps span **172 px** in the
original against **268 px** in ours while the track and the buildings behind
them aligned. A field of view cannot do that - it scales the background too.

**The residual is 19 %, not 9 %, and it is not explained.** A lamp *span* is
glow-sensitive, so it was re-measured on the two lamps' **centroids**, which
bloom far less: separation `108.8 px` in the original against `189.3 px` in ours
at scale `1.0` (ratio `1.740`) and `129.6 px` at `0.75` (ratio `1.191`). So the
`0.75` is a large, correctly-signed improvement and demonstrably not the whole
story - a pure `0.75` would land the ratio near `1.00`, and the factor the
centroid measurement actually wants is nearer `0.575`, which is not a constant
this project has recovered anywhere. **The residual is not folded into this constant**, and the reason is that **the
metric cannot tell one candidate factor from another.** Measured lamp-centroid ratios at three
scales - `1.0` -> `1.740`, `0.75` -> `1.191`, `0.5625` -> `1.067` - are **not
linear in the scale they are measuring**. A linear metric would put `0.75` at
`1.740 * 0.75 = 1.305` and `0.5625` at `0.979`; it reads `1.191` and `1.067`
instead. So a glowing lamp's centroid moves with its own brightness and with
which pixels clear the detection threshold, and the number it produces is good
enough to establish **"our craft is too big"** and nothing finer. Any factor
picked to make it read `1.000` would be fitted to an instrument, which is the
kind of constant this project retires rather than adds (see
`HALF_SIZE_TO_WORLD` and the deleted `FLARE_ASPECT`).

`0.75` closes `1.740` only to `1.191`, so roughly a further `1.3x` is
unexplained and stays visible rather than cancelled.

**A second `0.75` is the live hypothesis, and it is arithmetic rather than a
reading.** `1 / 0.75^2 = 1.7778` sits 2.2 % from the measured `1.740`, and
`g_craft_scale` (`0x08ab0e1c`, a code literal `0.75` written once in
`Craft_Construct_q`) is confirmed read by the hover probe corners, the `<Misc>`
hull dimensions into the collider, the initial body height and both camera rigs
- see [camera.md](camera.md). If the original also scales the drawn mesh by it,
the craft is scaled twice where this project scales it never. **The second site
was searched for and not found**; `Ship_LoadModel` and `FUN_0884dab4` are the
two unopened calls in `Craft_Construct_q` that could set a node scale. Our own per-batch position scale in `oag_formats::vex`
(`s16 / 32768 * scale`, the `f32` at batch `+0x10`) was the third candidate and
**is now excluded** - see below. More practically: the chase camera is separately known
not to frame the craft the way the original's does and is being worked on, and a
half-correct ship scale sitting in the tree while that happens is how a camera
distance gets fitted to cancel a model error. One wrong constant is easier to
find than two that cancel.
### The residual is the craft model alone: the track is the right size

Fitting the horizontal displacement of **136 matched world edges** - track rails,
barriers and trackside structures, in the columns either side of the craft,
across fourteen scanlines of the same matched-pose frame - against distance from
the screen centre:

```text
offset = +0.00405 * (x - 480) + 1.37      RMS residual 9.9 px
implied world scale ours/theirs = 1.0041
```

A wrong world scale shows as a slope growing with distance from the centre. The
slope is `0.4 %` and the residual swamps it. **So the track and the buildings are
the right size, and the recorded camera and our projection are right with them**
- a wrong eye position or field of view would have bent that fit too. Only the
craft is oversized, and it is oversized *relative to a track that matches*.

That excludes the most attractive candidate: **our per-batch position scale in
`oag_formats::vex` is shared by every mesh including the track**, so a misread
there would make the track wrong as well. It does not. Every remaining candidate
is specific to the craft path - a second scale on the craft's own node (the two
unopened calls in `Craft_Construct_q`, `Ship_LoadModel` and `FUN_0884dab4`), or
the craft's authored vertex scale being consumed differently from a track's.

One caveat on the *size* of the remainder, not on its existence: the `1.19`
figure comes from lamp centroids, a metric shown above to be non-linear in the
scale it measures, so it bounds the craft as "too big" without pinning the
factor. The world fit is the trustworthy number here - 136 points, a slope and a
residual.

Superseded by the paragraph above, kept for one revision because two passes
cited it: candidates not separated by the older reading were a
second scale on the render node, a camera the recorded pose does not fully
describe, and this measurement's own perspective sensitivity (the two frames put
the craft at slightly different distances).

**Two consequences to be aware of whenever this is picked up again.** First, the
scale would reach **every frame of every race**, not just a boost -
`ship_model_matrix` also composes the `engine_flare` locator through
`Race::nozzle`, so the flare's centre and the trail's origin move with the hull.
Second, and this is the trap: the flare's *position* would scale while its *size*
must not. That is not an
oversight to tidy up - `half_size` is in view units under a rigid
world-to-view matrix with GE matrices 1 and 2 set to identity
(`HALF_SIZE_TO_WORLD` is `1.0`, from the live read at `ExhaustFlare_Draw`), so
the craft's model scale has no path to it. A future reader who "fixes" that
asymmetry will reintroduce a fitted constant.

Applying it would make the craft 25 % smaller in ordinary play, and whether that
reads *better* depends on the chase camera. Measured, from the same capture, as
the figure that camera work should be checked against:
the original's eye sits a near-constant **11.6 units** from the craft (11.65,
11.64, 11.64, 11.60 at ticks 34/54/78/120, over speeds from 94 to 153 units/s),
with 9.5-10.9 of that along its own forward axis.

The consequence for this page: the flare and the plume are sized in world units
and were right, but the hull they sit behind was `1.33x` too big, so the
flare-to-hull ratio came out a quarter small and from that camera the craft's
tail and most of its exhaust fell off the bottom of the frame. **The fit behind
`HALF_SIZE_TO_WORLD` was contaminated by this** - it was settled on
"reproduces the original's flare-to-hull ratio" while the hull was oversized.
The constant itself does not move, because it is recovered rather than fitted
(`1.0`, from the live matrix read at `ExhaustFlare_Draw` above); but that
particular corroboration for it must not be cited again.

### The plume's vertex alpha: the question was wrong, and the bug was ours

The three-pass-old question was "how does the authored vertex alpha reach the
picture, given a blend that cannot read it". **The premise held and the
arithmetic on our side was simply wrong.**

`race.rs` premultiplied the authored alpha into the vertex RGB **at the
vertices**, then let the rasteriser interpolate. That is not the same operation
as interpolating and then multiplying, and for this model the difference is the
entire visual: the rim is `(255, 98, 5)` at alpha `0`, so premultiplying maps it
to `(0, 0, 0)` and what crosses each fin is **black to white**. The authored
orange exists nowhere on it. Weighted per *fragment* instead, colour
interpolates orange-to-white while alpha interpolates `0`-to-`1` and the two
meet at the fragment, which leaves a dimmed orange fringe fading out - the
original's own feathered edge.

Measured on the capture above at boost age `0.367` s, matched camera and matched
entry intensity, over the magenta signature `r > 110, b > 110, r - g > 25,
b - g > 20` in one crop around the craft:

**The whole lower half of the frame**, which is the figure that describes what
actually ships, because it depends on no crop chosen per build:

| build | pixels | mean colour | `b - r` |
| --- | ---: | --- | ---: |
| **the original** | **9,217** | **`(223, 164, 224)`** | **+1** |
| premultiplied per vertex, `TRAIL_BLEND` (the old arithmetic) | 493 | `(180, 141, 228)` | +48 |
| raw alpha, `BLEND` (shipped) | **1,993** | **`(159, 118, 175)`** | **+16** |

Extent **4x** better and the red/blue balance three times closer to neutral -
the original's magenta has `r` and `b` within one unit of each other, ours went
from 48 apart to 16. Still only 22 % of the original's extent, and dimmer
overall; the section below on what does not match says why.

A second set of numbers, from a crop centred on the craft **and with the
craft-scale experiment applied**, is worth keeping only because it isolates the
arithmetic from everything else:

| build (cropped, craft scale `0.75`) | pixels | mean colour |
| --- | ---: | --- |
| the original | 55,731 | `(224, 166, 227)` |
| premultiplied per vertex, `TRAIL_BLEND` | 2,034 | `(151, 93, 212)` |
| raw alpha, `TRAIL_BLEND` | 7,582 | `(222, 168, 230)` |
| raw alpha, `BLEND` | 39,549 | `(203, 162, 219)` |

With the hull at the right size and the crop on the plume, raw-plus-`BLEND`
reaches 71 % of the original's extent at a hue within a few units. **That is
not the shipped configuration** - the craft scale was reverted, see below - so
it measures the blend change with the scale confound removed rather than what a
player sees. Both tables agree on the ordering, which is the point: the
premultiplied build is blue-dominant and far short on extent, raw alpha alone
recovers the hue but draws hard-edged solid wedges (the 2026-08-07 and
2026-08-08 results reproduced exactly), and the per-fragment weight is what
supplies the falloff those wedges were missing.

**But per-fragment alpha weighting is almost certainly not what the hardware
does, and the real mechanism was found the same day - by reading the GE state,
not the blend.** See [mesh-draw.md](mesh-draw.md) for the full decode; the three
results that bear on this page:

- **`GU_ALPHA_TEST` is disabled** for the transparent mesh pass, with
  `sceGuAlphaFunc` programmed `ALWAYS` / ref `0` / mask `0xff`
  (`Mesh_BeginTransparentPass`, `0x0890d904`). The inherited-alpha-test
  hypothesis for the rim is dead. Confidence 90.
- **`GU_BLEND` *is* enabled** for that pass, which closes `mesh-draw.md`'s own
  standing open question: the additive branch really composites `dst + src`.
  Confidence 90.
- **Nothing in the pass scales fragment RGB** except vertex colour x texture
  (`MODULATE`, colour-double off) x the blend. No fog, no colour test, no
  texenv colour, no emissive; lighting is *disabled* for transparent batches,
  so the ribbon's lit-colour path does not apply here. Confidence 85. That is a
  hard negative for "something scales the rim down without discarding its hue"
  being GE state at all.

**What supplies the falloff instead is the texture, reached through generated
coordinates.** The transparent pass sets `TEXMAPMODE` uvgen `2` -
**environment (shade) mapping** from the vertex normal and lights 0/1 - and
nothing inside the batch loop re-emits `0xc0` to undo it (the whole binary has
only two `0xc0` emitters, neither reachable there). So a transparent batch's
texture coordinates are *generated*, and `pulse_boost2_ADD`'s own
bright-to-dark streak gradient is what varies across the fin. Confidence 82;
the uvgen enum values come from pspgu/PPSSPP rather than from `BOOT.BIN`, which
is what holds it below 90.

**This also retires two of this page's own refutations, and explains the
authored data.** The plume's degenerate `u` is real and it is *unused*: the
batches declare 8-bit texcoords, which the GE decodes as `value / 128`, and the
authored bytes are `u = 1`, `v = 4` - exactly the `0.008` and `0.031` that were
recorded here as a suspected decode bug. **The decode was correct all along.**
And the two experiments this page cited as refutations - `u` scaled by 128, and
`u`/`v` swapped, both "changing the picture not at all" - were manipulating
coordinates the original never reads, so their conclusion stands but their
reasoning does not.

The decisive check: every one of the **32** `shipboost.vex` batches across all
eight PSP teams reads `vtype = 0x013d` - `tex=u8, col=8888, nrm=s8, pos=s16,
through=0`. Normals are present on every batch and none is in through mode, so
environment mapping has real normals to vary with.

`batch+0x0a` is the GE vertex-type word at confidence 88, from two independent
uses in the binary: `FUN_0892e8f0` emits it as GE command `0x12` and
`FUN_0890d3ac` tests its `& 0x60` (normal format) and `& 0x1c` (colour format).
**And the reading is now pinned disc-wide rather than on this one model**, by
`crates/formats/tests/vex_batch_layout_ground_truth.rs`: every batch's declared
payload size is reproduced by `count * stride` with the stride derived from the
vertex type alone, across eleven different strides, and `GU_TEXTURE_16BIT` never
appears anywhere on the disc - which is what rules out the competing reading of
a `0x013d` vertex as carrying two `u16` texcoords instead of one `u8` pair. Run
it with

```sh
OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-formats --run-ignored all \
    -E 'binary(vex_batch_layout_ground_truth)'
```

That is also what settles the `u8` decode behind the `0.008`/`0.031` above:
`u = 1` and `v = 4` over the GE's `/ 128`.

**So what ships is an empirical approximation, labelled as one.** `SrcAlpha`
reproduces a measured observable with arithmetic that is at least correct for
*some* per-fragment weighting, where the per-vertex premultiply was correct for
none. Environment-mapped UV generation is **not** implemented, deliberately: it
would change every transparent batch on every track and ship, and this repo
already carries a rejected project-wide blend flip
([mesh-draw.md](mesh-draw.md)) as the precedent for not doing that off one
pass's finding. It is the highest-value next step for this subsystem.

### What still does not match

Recorded rather than fixed, so the next pass starts from the measurement:

- **The plume is 71 % of the original's extent, not 100 %**, and its two fins
  stay closer to the wing roots where the original's reach further back and
  further outboard.
- **Ours blows out whiter in the core.** Near-white pixel counts are already
  comparable at 96,152 (theirs) against 102,829 (ours, premultiplied build) in
  the same crop, so ours is not short of brightness - it is short of *shape*.

  **And the shape it is short of is a bloom, measured.** Mean luminance in
  20-pixel annuli around each frame's own brightest exhaust pixel, same
  matched-pose frame:

  | radius (px) | 0-19 | 20-39 | 40-59 | 60-79 | 80-99 | 100-119 | 120-139 | 140-159 |
  | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
  | the original | 158 | 144 | 154 | 156 | 151 | 130 | 118 | 118 |
  | ours | **201** | 165 | 153 | 148 | 143 | 130 | 109 | **92** |

  The original holds a **flat shoulder near 150 out to about 100 px** and only
  then falls away; ours is sharply peaked - 28 % brighter at the core and 28 %
  darker at the outer edge - and falls monotonically. That is the bright-pass
  signature: the same energy spread wide against the same energy concentrated.
  It is [`roadmap.md`](../../../overview/roadmap.md)'s unchecked M6 item "bloom
  and the bright-pass on the exhaust and lights", and this is the first
  measurement that shows it is load-bearing for the boost specifically rather
  than for scene brightness in general. Part of the original's shoulder is the
  pad's own blown-out track surface, so treat the numbers as the shape of the
  difference rather than as a bloom radius to fit. Note the PSP has **no
  programmable shaders at all** - whatever the original does here is
  fixed-function GE work or a framebuffer pass, so "find the shader" is the
  wrong search; the roadmap's sibling item, motion blur / speed streaking, is
  visible in these same frames and is the other half of the look.
- **The original's field of view is wider during a boost**, reproduced
  independently here. Per the Open list below this is **not** a porting
  target and no constant in `display.rs` moves for it.
- **A 19 % ship-scale residual survives the `0.75`**, measured above, and until
  it is explained every absolute size on this page is a comparison of ratios
  rather than of pixels.
- **The original offers several cycleable race perspectives and this project has
  one.** Worth having in its own right, but note it does **not** explain the size
  gap above - that was the model scale, and a camera preset would have moved the
  background with the craft, which it did not.

## Open

- **`engine_fire` (`0x3e5`) and `exitglow` (`0x3e4`) have no registration call
  site.** The 46 callers are alphabetical and neither appears between
  `Engine Flare` and `fogCube` where it would sort. Either the `Engine Flare`
  class handles them, or they are never registered as runtime classes - which has
  precedent: `vex.md:110` records `gate` as exactly that. Not resolved. Note
  `exitglow` *is* authored 13 times on `16_Track`, so an unregistered class can
  still have instances.
- **The two authored `Trail` nodes on `shipwreck.vex`** still carry unread
  payload parameters. The racing ship's preset-2 values are now fully recovered
  (above); the wreck's authored ones are a separate, still-open read, out of
  scope while the wreck itself is.
- **Whether `Trail` appears outside `shipwreck.vex`.** Four files checked. A sweep
  over every `.vex` entry by index - names are unknown for most of `Data.wad`'s
  1,142 entries - would settle it.
- **The five teams whose `Ship.vex` does not resolve by name.** A `mine-names`
  job, not a format one.
- **What writes `self+0x84`** the `1` and `2` values that gate submit and update.
- **What arms `boost_timer` (`flare+0xb8`) in the *original binary*.**
  Everything downstream of it is recovered - `Exhaust_UpdateEngineSound` decays
  it, `engine_on` reads it against `0.2`, the half-size adds
  `boost_timer * 8.0`, and `Exhaust_Update` reveals the `<Team>boost.vex`
  model while it exceeds `0.2` (hiding it again once `flare+0x88` passes
  `1.5` s). The *writer* - a boost-start, pad contact or pickup - was searched
  for by store-offset and not found this pass; it is gameplay code, most
  likely reached at the same site as `ExhaustFlare_OnSpeedupPad`
  (`0x08904f10`, see `BOOST_SECONDS`'s doc comment). **This reimplementation's
  own trigger is settled and shipped**, independently of finding that
  original writer: `Race::test_speedup_pads` calls
  `self.exhaust.boost(exhaust::BOOST_SECONDS)` on the same edge
  `ExhaustFlare_OnSpeedupPad` fires on, and the plume now draws off that timer
  end to end. What remains open here is purely the RE question of where in
  `BOOT.BIN` the equivalent store happens, not whether the visual is
  triggered.
- ~~The ribbon's texture function~~ - **closed 2026-08-02** at 90: modulate /
  `TCC_RGBA` / colour-double off, set once by `Gfx_Init` and confirmed
  unchanged by a full-frame command-stream scan plus the driver's live shadow
  state. See the GE-state section.
- Two callees identified from use rather than from their bodies, hence `_q`:
  `Ship_ThrustInput_q` (`0x0883d850`, read as thrust) and `Gfx_ViewDepth_q`
  (`0x0890486c`, read as a view depth).
- The class table's `id == -1` terminator, and therefore its extent.
- ~~What widens the fov during a boost~~ - **not an open question, and never
  was an RE one. Withdrawn 2026-08-08.** `crates/game/src/display.rs`'s
  `BoostFovKick` is a **deliberate invention of this project**, added because
  a boost reads better with it, and the 2026-08-07 entry that listed it here
  as unrecovered original behaviour mis-stated its status. It is ours by
  choice; it is not being fitted to the original and its `DEFAULT` is not a
  measurement of anything. **Do not re-open this as a porting target.**

  What the original's own fov does is now documented anyway, on
  [camera.md](camera.md), as evidence rather than as a specification: the
  projection's fov is `authored + ship->0x790`, `ship->0x790` being a 4 Hz
  decaying cosine shake clamped to +/-30 degrees written only by `Hud_Update`,
  with a second path through `Ship_UpdateCameraRigs`' `ship->0x860 & 4`
  override. Which of the two a speed pad uses was not established. That
  writeup is worth having - it settles the fov *unit* and the hardcoded
  `480.0/272.0` aspect at instruction level, which the projection does depend
  on - but nothing downstream of it is owed a port.
- ~~**How the plume's authored vertex alpha reaches the picture.**~~ -
  **closed 2026-08-08, and the answer is that it does not.** `GU_ALPHA_TEST` is
  disabled, `GU_BLEND` is enabled with `GU_FIX` white on both sides, and nothing
  in the transparent pass scales fragment RGB but vertex colour x texture x the
  blend. The falloff comes from the *texture*, sampled through
  environment-generated coordinates. See the section above and
  [mesh-draw.md](mesh-draw.md). What replaced it as the open item was narrower
  and was a **rendering** gap rather than an RE one - environment-mapped UV
  generation was not implemented - and **that is closed too, 2026-08-09, on
  both halves**. [mesh-draw.md](mesh-draw.md) carries the uvgen-2 equation, the
  space each term is in, and the plume's two recovered light vectors
  `(0.9553365, -0.2486722, -0.1596704)` and `(0.0, 0.5403023, -0.8414710)` -
  fixed, world-space, ship-locked rather than camera-locked;
  `oag_render::texgen` generates the coordinates per vertex from them, for the
  boost plume alone.

  **What this fixed is bigger than the falloff the question was asked about.**
  The authored coordinates pin `u` to texel column 0, and column 0 of
  `pulse_boost2_ADD` is a uniform `(250, 248, 250)` down all sixteen rows - so
  the texture contributed a constant white, the plume drew as flat vertex
  colour, and it read as two hard-edged solid orange wedges. The rest of that
  64x16 texture is a **streak lookup**: `u` ramps white through violet
  (`(143, 72, 218)`) to `(10, 4, 18)`, and at any column but 0 the rows
  alternate hard between a bright magenta `(241, 110, 253)` and a dark navy
  `(20, 5, 122)`. The `v` axis is what breaks a fin into discrete bright
  bands, and it is where the original's streaked look comes from. That also
  accounts for this page's own unexplained colour gap - the original's boost
  region reads magenta `(223, 164, 224)` against ours at `(159, 118, 175)` -
  because with column 0 as the only sample there was no magenta anywhere in
  the reimplementation's pipeline for the additive blend to reach.
  `boost_plume_ground_truth.rs` pins both halves across all eight PSP teams.

  **The `SrcAlpha` blend was re-tested against the recovered one and kept, on
  the measurement rather than on inertia.** `SrcAlpha` was introduced only as a
  substitute for this missing falloff, so implementing the falloff is a real
  argument for returning the plume to the recovered `GU_FIX`/`GU_FIX`
  (`One`/`One`) blend. Measured at the original's own pose
  (`data/traces/pad0-boost.csv` tick 62, age `0.5` s) against its own frame,
  over the capture's plume mask (`min(r, b) - g > 25`, `luma > 60`):

  | build | plume px | mean | `b - r` | orange px |
  | --- | ---: | --- | ---: | ---: |
  | the original | 9,435 | `(220, 165, 237)` | +17.6 | 230 |
  | ours, authored UVs | 4,480 | `(184, 143, 203)` | +19.0 | 2,679 |
  | **ours, texgen + `SrcAlpha`** | **5,996** | `(197, 150, 210)` | +13.3 | **2,041** |
  | ours, texgen + `One`/`One` | 1,958 | `(243, 181, 227)` | -15.6 | 7,030 |

  **Re-measured 2026-08-09 after the trace harness showed the first run's pose
  was wrong**, so the flags are part of the result: `--pose-tick 62
  --pose-boost 0.517752 --pose-intensity 0.1250567 --pose-speed 148.8853`. The
  first table used `--pose-boost 0.5`, seeded intensity from the entry tick
  rather than the one before it, and was taken while `force_boost_state`
  charged the boost accumulator 100x too slowly. Every number moved 2-15 %.

  `One`/`One` restores the authored `(255, 98, 5)` rim at full strength: **31x
  the original's orange pixel count**, red-dominant where the original is
  blue-dominant, at 21 % of its extent. The capture's own connected-component
  pass says the same independently - the original's plume and ribbon are one
  violet family and neither is near that orange. **So a term in the recovered
  blend chain is still missing**: the GE state has since been read
  exhaustively - all five setters to their command byte, `Gu_TexFunc` confirmed
  `MODULATE`/`TCC_RGBA`, negative at confidence 92 - and it does not reproduce
  the picture, while an unrecovered source-alpha weight does. Kept because it
  measures better, not because it is understood.

  **One conclusion reversed on re-measurement and is recorded rather than
  buried**: on `b - r` alone the authored-UV build is now closer to the
  original (+19.0 against +17.6) than the generated-UV one (+13.3), where the
  first table read +22.0 against +15.0 and pointed the other way. It is not
  acted on, for two reasons. `b - r` is a mean over a mask of a different size
  in every row, so it is not a like-for-like colour comparison, and that mask
  includes a saturated white core pulling every build toward neutral. The
  extent and orange columns, which do compare the same quantity across rows,
  both still favour generated coordinates - by +34 % and -24 % - as does the
  streak structure the whole change was made for.

  **The extent column is not a calibrated ratio.** Our craft renders about
  `1.4x` too large at this same camera pose, so every pixel count here carries
  an unresolved scale error; see `HANDOVER.md`, "The craft render scale
  invalidates matched-pose pixel comparison".
- **Superseded phrasing kept for one revision**, because two passes cited it:
  the old question assumed some GE path weighted a mesh batch by source alpha. None does. And the two prebuilt
  lists at `mesh+0x70` `+0x48`/`+0x70` are now read: they hold **light 0 and
  light 1 directions plus `LIGHTTYPE`**, nine words each, and `*batch & 0x8000`
  selects only the light computation field. They were the last candidate for a
  hidden alpha path and they are not one - they are what environment-mapped UV
  generation reads. **Followed through 2026-08-09**: both writers of those lists
  are now read to the instruction, the plume's are written once at load from a
  fixed `Rx(-1.0) * Ry(0.3)` basis rather than per frame from the view matrix,
  and all 32 of the eight teams' plume batches measure `pass_mask = 0x1232`, so
  `& 0x8000` is clear on every one and they all replay list A. See
  [mesh-draw.md](mesh-draw.md).
- ~~What stretches the flare wide~~ - **closed 2026-08-08: nothing does.**
  See `ExhaustFlare_Draw` above.
