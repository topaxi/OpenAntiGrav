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
| **Addresses** | update `0x0892a450`, draw `0x0892a588`, init `0x0892a2b0`, push `0x08929958`, ribbon draw `0x0892acc8` |
| **Confidence** | 78 |

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
(`count >= capacity`) and `count > 1`. It walks the ring backwards from the write
index, newest to oldest, and for each sample offsets it as `position + direction *
w[i]` (`w` from the array at `+0x138`). Per sample it emits, for each of
`self+0x64` layers:

- 10 vertices, `Gu_DrawArray(GU_TRIANGLE_STRIP, 0x1fe, 10, ...)`, stride 0x20 -
  `0x1fe` decodes as `GU_TEXTURE_16BIT | GU_COLOR_8888 | GU_NORMAL_32BITF |
  GU_VERTEX_32BITF` = 4 + 4 + 12 + 12 = **32 bytes**, matching the `0x140` =
  10 x 0x20 per-segment stride the loop steps by. A second format cross-check of
  the same kind that validated the flare quad.
- a width from `self[+0x78 + layer*0x30] * self+0x1c`, applied to the camera
  basis read from `DAT_08ab10b0 + 0x40/0x44/0x50/0x54/0x60/0x64` - so each
  segment is **camera-facing**.
- one of the **three** `Engine_noise` texture slots, indexed per layer:
  `Gfx_BindTexture(g_engine_noise_texture[*(iVar24+4)])`. This is why
  `Texture_LoadEngineNoise` writes `0x08b657c0`, `c4` and `c8`.
- a scrolling UV offset, `u += du * K * 3.0` and `v += dv * K`, each wrapped to
  `[0,1]` and applied with `sceGuTexOffset`.

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

The value stepped down toward the tail from `ring+0x20` feeds `sceGuAlphaFunc` - an
alpha *test* reference - and **preset 2 never sets `ring+0x20`**, so
`Trail_InitRing`'s zero stands and the test passes everything. There is no
per-segment alpha fade for a racing craft.

`Trail_BuildOffsetTable`'s weights, evaluated, come out at `2.6e-6` per sample over
ten, so the displacement along the exhaust direction is **numerically nil**. The
ribbon's shape is purely its position history.

Confidence is 78 rather than 85 because the width array, the layer count and the
`+0x1c` scale are all read as *shapes* - nothing pins their authored values, and
whether a ship node supplies them or the class defaults them is unread.

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
| `0x0892acc8` | function | `Trail_DrawRibbon` | 78 |
| `0x08905210` | function | `Texture_LoadEngineFlare` | 85 |
| `0x0892a5a4` | function | `Texture_LoadEngineNoise` | 85 |
| `0x08928460` | function | `Gfx_BindTexture` | 75 |
| `0x08b62908` | data | `g_engine_flare_texture` | 88 |
| `0x08b657c0` | data | `g_engine_noise_textures` | 80 |
| `0x08ab2370` | data | `g_vex_class_table` | 92 |
| `0x0891e35c` | function | `Gfx_Enqueue` | 65 |
| `0x0890486c` | function | `Gfx_ViewDepth` | 60 |
| `0x0883d850` | function | `Ship_ThrustInput` | 55 |

`Vex_FindClassDescriptor` (`0x08908b68`) and `Vex_LoadModel` (`0x08912b80`) were
already named by earlier passes; `Gu_DrawArray` (`0x08810e98`) and `Gu_CallList`
(`0x08810598`) likewise, via [`vex.md`](../../../formats/vex.md).

Not renamed, deliberately: `0x0892a050` (the flame child's constructor),
`0x08810e10` / `0x08810db8` (the GU enable/disable pair - see the blend-state
caveat above; the argument pattern is suggestive but the bodies are unread, and
ADR-0005 puts a guess dressed as a name below the bar).

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
| `ring+0x1c` | 1.0 | width scale, applied to every layer width |
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

**The preset's own colours are dead on arrival for a racing craft**, and this is
worth recording because they look meaningful: layer 0 is a deep blue
`(0, 0.031, 0.502)`, layer 1 a magenta `(0.608, 0, 0.486)`, layer 2 a near-white
`(0.824, 0.824, 0.824)`. `Exhaust_Update` overwrites **all four channels of all
three** every frame with that layer's staggered ramp, multiplied by a tint at
`flare+0x68`..`+0x74` that it sets to `(1, 1, 1, 1)` in the same breath. So the
ribbon a racing ship draws is greyscale, and the authored colours only matter to
whatever else uses this preset - or to a tint that is always white here and is a
hook for something unfound.

That `Exhaust_Update` writes the ramp into **rgb as well as alpha** matters to the
result: with the additive blend weighting rgb by alpha, the contribution goes as the
ramp *squared*. Writing `(1, 1, 1, ramp)` instead gives a linear falloff and a
visibly hotter, flatter plume.

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

### The boost visual, and two mount points

`Ship.vex` carries two `Transform` locators named `boost_flare` and
`boost_flare1`, children of `world`, 64 bytes each. `shipboost.vex` carries
exactly two meshes, **`bflare1Shape`** and `bflare2Shape`, textured
`Data/Tex/pulse_boost2_ADD.tga` - and the `_ADD` suffix is the artists' own note
that it composites additively, agreeing with the blend function recovered from
`ExhaustFlare_BuildDisplayList`. The name pairing is strong but nothing traced
the locators to the meshes at instruction level, so treat the mounting as
**confidence 60**.

`shipboost.vex` and `shipshield.vex` also each carry three class-`0x000` nodes
named `ViewCompass`, `UniversalManip` and `UniversalManip1`, with zero-byte
payloads. Those are **Maya viewport furniture exported by accident** - the
manipulator gizmos and the view compass - not a decode failure. Recorded so the
next reader does not go looking for a class `0` handler.

## Open

- **`engine_fire` (`0x3e5`) and `exitglow` (`0x3e4`) have no registration call
  site.** The 46 callers are alphabetical and neither appears between
  `Engine Flare` and `fogCube` where it would sort. Either the `Engine Flare`
  class handles them, or they are never registered as runtime classes - which has
  precedent: `vex.md:110` records `gate` as exactly that. Not resolved. Note
  `exitglow` *is* authored 13 times on `16_Track`, so an unregistered class can
  still have instances.
- **`Trail`'s authored parameters** - the per-layer width array, the layer count
  and the `+0x1c` scale. Now readable, since `shipwreck.vex` supplies two real
  instances; not read in this pass because the wreck is out of scope for it.
- **Whether `Trail` appears outside `shipwreck.vex`.** Four files checked. A sweep
  over every `.vex` entry by index - names are unknown for most of `Data.wad`'s
  1,142 entries - would settle it.
- **The five teams whose `Ship.vex` does not resolve by name.** A `mine-names`
  job, not a format one.
- **What writes `self+0x84`** the `1` and `2` values that gate submit and update.
- **`Trail`'s authored parameters** - width array, layer count, `+0x1c` scale.
- Two callees identified from use rather than from their bodies, hence `_q`:
  `Ship_ThrustInput_q` (`0x0883d850`, read as thrust) and `Gfx_ViewDepth_q`
  (`0x0890486c`, read as a view depth).
- The class table's `id == -1` terminator, and therefore its extent.
