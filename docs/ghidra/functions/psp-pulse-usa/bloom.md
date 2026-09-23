# The bloom: a four-pass framebuffer post-process

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP, UCUS-98712) |
| **Subsystem** | full-screen post-processing |
| **Related** | [`exhaust.md`](exhaust.md) (what writes the mask), [`mesh-draw.md`](mesh-draw.md) (the GE state helpers), [`main-loop.md`](main-loop.md) (`Game_MainLoop`) |

Recovered 2026-08-10, prompted by the exhaust comparison on
[`exhaust.md`](exhaust.md) measuring the original's plume at **2.2x** our
light output and [`roadmap.md`](../../../overview/roadmap.md)'s M6 item
"bloom and the bright-pass on the exhaust and lights" carrying no evidence
either way.

**The original has a real bloom, and it is fully specified below.** That
settles the question the roadmap item left open: implementing one is a
**port**, not an invention.

## Why two obvious searches find nothing

Recorded first, because both are the natural first move and both fail:

- **There is no framebuffer copy.** The binary's only `TRANSFERSTART`
  (`0xea`) emitter is `Gu_CopyImage` (`0x08810fe0`) and its single caller is
  the texture-to-VRAM uploader (`FUN_08928174`). A block-transfer search
  therefore concludes "no post-process", which is wrong: this bloom **binds
  the framebuffer directly as a texture** out of EDRAM and never copies it.
- **A `lui 0xa000` scan for `TEXADDR` finds nothing**, because `Gu_TexImage`
  emits the command as a computed `(level + 0xa0) << 24` rather than as a
  constant. Only the commands whose byte is a literal (`0xc6`, `0xea`, `0x9c`)
  are findable that way. The scan that *does* work here is on
  `sceGuDrawBuffer`'s `FRAMEBUFPTR` (`0x9c`): two emitters, six call sites,
  and **three of the six are in one function** - which is the shape of
  render-to-texture and is what led here.

## The object

`Bloom_Construct` (`0x089072d4`, confidence 82) builds a **singleton**
- it stores itself in `g_bloom` (`DAT_08b32c60`) - and is reached from
`Game_MainLoop` (`0x08807418` -> `FUN_0888adcc` -> here), so this is live code
on the main path rather than a dead class.

| Field | Contents |
| --- | --- |
| `+0x38` | the method table, `DAT_08ad148c` |
| `+0x48` | `0x3c23d70a` = **`0.01f`**, a first-order lag rate - the same constant `Exhaust_UpdateEngineSound` uses |
| `+0x50` / `+0x54` | `sceGeEdramGetAddr() + 0x16f080` and `+ 0x174340`, the two blur vertex buffers |
| `+0x60`..`+0x6c` | the four prebuilt display lists, built once here |
| `+0x70`..`+0x7c` | their cached uncached-mirror copies |
| `+0x80` | `0x294` bytes, the bright-pass sprite buffer |
| `+0x84` | `0xb4` bytes, the composite sprite buffer |

`Bloom_Draw` (`0x089075c0`, confidence 85) sits at `0x08ad14d0`, and
`0x08ad14d0 - 0x08ad148c = 0x44` - **exactly the `draw` slot** in the
method-table layout [`exhaust.md`](exhaust.md#the-method-table-layout)
recovered independently for `Engine Flare` and `Trail`. That offset agreeing
is what identifies this as a draw method rather than a guess from position.

## The four passes

`Bloom_Draw`'s whole body, with the helpers resolved:

```c
Gu_PixelMask(0);                                   // write every channel
Gu_TexImage(0, 512, 512, 512, edram + framebuffer_offset);   // the framebuffer, as a texture
Gu_DrawBuffer(psm, BLOOM_SCRATCH_A, 256);  Gu_CallList(list0);  // bright pass  -> A
Gu_DrawBuffer(psm, BLOOM_SCRATCH_B, 256);  Gu_CallList(list1);  // blur X   A   -> B
Gu_DrawBuffer(psm, BLOOM_SCRATCH_A, 256);  Gu_CallList(list2);  // blur Y   B   -> A
FUN_0891ec04(display);                             // restore the real draw buffer
Gu_CallList(list3);                                // composite A over the scene
```

The two scratch buffers are `g_bloom_scratch_a` (`DAT_08a84cf8` =
`0x110000`) and `g_bloom_scratch_b` (`DAT_08a84cfc` = `0x132000`), both
EDRAM offsets, both at stride 256.

### Pass 0 - the bright pass, and its mask is *destination alpha*

`Bloom_BuildBrightPassList` (`0x089078c0`, confidence 85):

```text
disable STENCIL_TEST, DEPTH_TEST, ALPHA_TEST, CULL_FACE, LIGHTING
enable  TEXTURE_2D, BLEND
Gu_TexWrap(CLAMP, CLAMP)
Gu_BlendFunc(GU_ADD, src = GU_SRC_ALPHA, dst = GU_FIX 0x000000)
Gu_Color(0xffffffff)
Gu_DrawTiledSprites(src 480 x 272, dst 240 x 136, at 0, 0)
```

`dst = GU_FIX 0` means the destination contributes nothing, so the pass is a
**replace**, and the value written is

```text
scratch_a = framebuffer.rgb * framebuffer.a
```

**The bright pass is not a luminance threshold. It is a mask carried in the
framebuffer's own alpha channel**, which the scene writes as it draws.
Confidence **85** on that reading, and the corroboration is the composite
pass below setting `Gu_PixelMask(0xff000000)` - it deliberately
**write-protects the alpha channel** while compositing, which is only worth
doing if that channel carries something.

The downsample is `g_bloom_buffer_width` x `g_bloom_buffer_height`
(`DAT_08a84cf0`/`DAT_08a84cf4`) = **240 x 136**, exactly half of 480 x 272 on
each axis.

### Passes 1 and 2 - a separable 11-tap blur

`Bloom_BuildBlurHorizontalList` (`0x089079d0`) binds scratch A and calls
`Bloom_EmitBlurTaps(self, 0)`; `Bloom_BuildBlurVerticalList` (`0x08907a70`)
binds scratch B and calls `Bloom_EmitBlurTaps(self, 1)`. Both set

```text
Gu_BlendFunc(GU_ADD, GU_FIX 0xffffff, GU_FIX 0xffffff)     // pure additive
```

`Bloom_EmitBlurTaps` (`0x08907c40`, confidence 85) first clears the target -
one untextured `GU_SPRITES` draw of colour `0` over `(0,0)`-`(240,136)`,
with `TEXTURE_2D` and `BLEND` disabled for it - then emits the taps.

The loops are `v` in steps of `0x22` (34) to `0x88` (136), `u` in steps of
`0x10` (16) to `0xf0` (240), and **11 taps** per tile. The tap offset is
`i - 5`, so **-5..+5 pixels**, applied to `x` when `param_2 == 0` and to `y`
when `param_2 == 1` - a **separable blur, horizontal then vertical**.

**The kernel is 11 bytes at `g_bloom_blur_weights` (`DAT_08ab2348`)**, read
directly out of `.data`:

```text
20, 30, 40, 50, 64, 64, 64, 50, 40, 30, 20        (decimal)
14 1e 28 32 40 40 40 32 28 1e 14                  (hex)
```

Each byte is splatted into RGB with alpha `0xff` and sent as the sprite's
vertex colour, so under `MODULATE` plus the additive blend the pass computes
`sum(tap_i * w_i / 255)`. The weights sum to **472**, i.e. a gain of
`472/255 = 1.85` per axis and **3.43x over both** - the blur deliberately
brightens rather than preserving energy, which is a large part of why the
original's glow is as strong as the exhaust measurement found.

The outer tiling into 16 x 34 chunks is a GE texture-cache concession, not
part of the effect.

### Pass 3 - the composite

`Bloom_BuildCompositeList` (`0x08907af4`, confidence 85):

```text
Gu_DepthMask(off)
Gu_TexScale(1.0, 1.0);  Gu_TexOffset(0, 0)
Gu_PixelMask(0xff000000)                    // protect the alpha/mask channel
disable ALPHA_TEST, LIGHTING
Gu_Color(0xffffffff)
Gu_BlendFunc(GU_ADD, src = GU_FIX 0xafafaf, dst = GU_FIX 0xffffff)
enable BLEND
Gu_TexImage(0, 256, 256, 256, edram + BLOOM_SCRATCH_A)
Gu_DrawTiledSprites(src 240 x 136, dst 480 x 272, at 0, 0)
re-enable DEPTH_TEST, LIGHTING
```

Both factors are `GU_FIX`, so the composite is

```text
framebuffer = framebuffer + (0xaf / 255) * upscale2x(scratch_a)
           = framebuffer + 0.686 * bloom
```

`g_bloom_composite_strength` (`DAT_08ab2344`) is the byte **`0xaf` = 175**,
replicated across RGB by the caller. Depth write is off and the source is the
240 x 136 buffer stretched back to 480 x 272 - the upscale is the only
"blur" the wide radius gets beyond the 11 taps.

## `Gu_DrawTiledSprites` (`0x0890710c`)

Confidence 82. Both the bright pass and the composite go through it, and its
argument roles are what make the two rectangles above readable:

```c
int Gu_DrawTiledSprites(float src_w, float src_h,
                        float dst_w, float dst_h,
                        float dst_x, float dst_y, void *buffer);
```

It walks the source rectangle in tiles of at most
`DAT_08ab233c` x `DAT_08ab2340` = **64 x 272**, and per tile writes a
two-vertex sprite:

```text
vertex.u, vertex.v = the source position, in texels
vertex.x = u * (dst_w / src_w) + dst_x
vertex.y = v * (dst_h / src_h) + dst_y
Gu_DrawArray(GU_SPRITES, 0x800183, count * 2, 0, buffer)
```

`0x800183` decodes as `GU_TEXTURE_32BITF | GU_VERTEX_32BITF |
GU_TRANSFORM_2D` = 8 + 12 = **20 bytes**, and the function's own return value
is `tiles * 0x28` - two vertices of 20 bytes - which is the same
stride-agrees-with-decode cross-check the flare quad and the ribbon each
passed. **`GU_TRANSFORM_2D` is why the texture coordinates are in texels
rather than normalised**, and therefore why `src_w`/`src_h` are the *source*
extent and `dst_w`/`dst_h` the *destination*: reading them the other way
round inverts every scale factor on this page.

Half-texel nudges (`0.0 -> 0.5`, `edge -> edge - 0.5`) are applied to the
source coordinates, the standard PSP texel-centre correction.

## What this corrects elsewhere

**[`exhaust.md`](exhaust.md) calls the ribbon's stencil ramp "destination-alpha
bookkeeping, visually inert for the ribbon itself". It is not inert - it is
the bloom mask.** `Trail_BuildStateList` sets `sceGuStencilOp(KEEP, KEEP,
REPLACE)` with `Gu_StencilFunc(GU_ALWAYS, ...)`, so the ribbon writes a
per-segment stepped value into exactly the channel pass 0 multiplies by, and
`Exhaust_Update` drives that value's base every frame with
`intensity * 0.5 * 0.9`. So the exhaust's glow strength is authored, it
ramps with engine intensity, and it reaches the picture through this
subsystem rather than through the ribbon's own draw. That page is corrected.

## The glow mask: who writes it, and how it is gated

Both of this page's first two open items are now settled, and the second one
opened something larger.

**The pixel format is `GU_PSM_8888`.** `DAT_08abf5c4` and `DAT_08abf5c8` both
read **`3`**, and `3` is `GU_PSM_8888` in the `sceGuDrawBuffer` /
`sceGuTexMode` enum. So the framebuffer and both scratch buffers are 32-bit,
and **the glow mask has a full 8 bits of alpha**, not the 1 bit a `5551`
target would have given. Confidence 88 - the constants are read directly and
the enum is `sceGu`'s own.

**`Bloom_SetPixelMask` (`0x08907828`) is the glow-mask API**, and it is the
reason `g_bloom` appears to have 22 readers. Every one of them is the same
three instructions - load the singleton, null-check it, call this - and
nothing else uses the object:

```c
void Bloom_SetPixelMask(Bloom *self, bool writable) {
    Gu_PixelMask(writable ? 0x00000000 : 0xff000000);
}
```

`0xff000000` **protects** the alpha channel (a set mask bit is a masked bit),
`0` opens every channel. So its 16 call sites are exactly the list of
subsystems that bracket their own writes to the glow buffer -
`Gfx_FlushRenderManager` (both directions), `Mesh_SetBatchDrawState`, the
particle draw path (`FUN_08915fd0`), and ten others not yet read.

**The per-batch gate is `pass_mask & 0x40`.** The tail of
`Mesh_SetBatchDrawState` (`0x0890d9c0`) is:

```c
if ((batch->pass_mask & 0x40) == 0) {
    if (g_bloom != 0) Bloom_SetPixelMask(g_bloom, 0);   // protect alpha
} else {
    Gu_PixelMask(0);                                     // this batch writes the glow mask
}
```

That is the same `pass_mask` u16 [`mesh-draw.md`](mesh-draw.md) already
decodes for the blend classes, so **the flag is readable from shipped data
today**. Confidence 85.

> **Corrected 2026-09-23.** The census below counted `0x40` alone. The
> writers are `pass_mask & 0xc0`, and `0x80` is set on every `_GLOW`
> texture's batches - 214 of `16_Track`'s 2,079. They stamp the texture's
> own glow byte through the stencil, and every other opaque batch stamps a
> base value of `4`. Read live out of EDRAM on
> [`glow-mask.md`](../../../rendering/glow-mask.md). The paragraphs below are
> kept as the record.

**And a census says no shipped mesh uses it at all.** Measured through
`oag_vex::vex::mesh_batches` on the European disc, over the four tracks
whose `Data\Environments\<n>_Track\track.vex` resolves plus a ship and its
plume:

| File | Batches carrying `0x40` |
| --- | --- |
| `10_Track`, `13_Track`, `14_Track`, `16_Track` | 0 of 1609, 1823, 1394, 2079 |
| `Assegai\Ship.vex` | 0 of 14 |
| `Assegai\shipboost.vex` | 0 of 4 |
| **total** | **0 of 6,923** |

So **no authored surface anywhere opts into the glow mask through the mesh
path** - not a hull, not a plume, not a track material, not a trackside
light. That makes the design coherent rather than surprising: **alpha is a
dedicated glow buffer and ordinary geometry is masked off it by default**,
and everything that glows does so from *code*, through one of
`Bloom_SetPixelMask`'s sixteen call sites. The first such writer this project
has identified is the exhaust ribbon's stencil ramp (see the correction
below).

**Two honest limits on that.** The branch is read at instruction level but
**no shipped batch exercises it**, so "`0x40` means this batch writes the
glow mask" is an inference about the *code path*, not a reading corroborated
by any data example - it is what the branch does, not what the artists used
it for. And 20 of the 24 track names did not resolve, so the census covers
four circuits rather than the whole disc.

## Who writes the glow mask, all sixteen sites read

2026-08-10, second pass. `a1` is `Bloom_SetPixelMask`'s `writable`, so **`1`
opens the alpha channel and `0` protects it**. Read off every call site:

| `writable` | Sites |
| --- | --- |
| **`1` - opens alpha** | `Gfx_FlushRenderManager` `0x0891e42c`, `FUN_088731c4` `0x08873234`, `FUN_08900a9c` `0x08900b00` |
| `0` - protects alpha | the other thirteen, including `Gfx_Init`, `Mesh_SetBatchDrawState`, `Bloom_BuildCompositeList`, `FUN_08915fd0` (the particle-draw neighbour) and nine more |

**Only three of sixteen let anything into the glow buffer**, which is the
same conclusion the `pass_mask & 0x40` census reached from the data side:
the channel is defended by default and opened deliberately.

`Bloom_SetPixelMask` is not the only route, though - `Gu_PixelMask`
(`0x08811898`) has **29 direct callers**, and three of them matter here:

- **`Trail_BuildStateList` (`0x08929740`)** - `Gu_PixelMask(0)`, then
  `sceGuStencilOp(KEEP, KEEP, REPLACE)` with `GU_ALWAYS`. The exhaust
  ribbon's stepped ramp, base `intensity * 0.5 * 0.9`, lands in alpha. The
  writer [`exhaust.md`](exhaust.md) used to call inert.
- **`ExhaustFlare_BuildDisplayList` (`0x08904c60`)** - also `Gu_PixelMask(0)`,
  and its list disables `STENCIL_TEST`, so what reaches alpha is the
  **fragment's own alpha**: the flare's flickered `rand_int(200, 255)`. So the
  nozzle sprite is a strong glow source in its own right, not only through the
  ribbon. **New**, and it means the boost's bloom has two exhaust
  contributors rather than one.
- **`FUN_088731c4`** - a scrolling additive ribbon, below.

### `FUN_088731c4`: a second glow-writing ribbon, and it is authored

The only site that both calls `Gu_PixelMask(0)` *and* `Bloom_SetPixelMask(g_bloom, 1)`:

```text
Fog_Disable;  disable CULL_FACE, LIGHTING, ALPHA_TEST
Gu_PixelMask(0);  Bloom_SetPixelMask(g_bloom, 1)     // open alpha, twice over
enable DEPTH_TEST;  Gu_DepthFunc(6);  Gu_DepthMask(on)
enable STENCIL_TEST
Gu_StencilOp(KEEP, KEEP, REPLACE)
Gu_StencilFunc(GU_ALWAYS, DAT_08ab1078, 0xff)        // a constant glow value
depth range 0xfdb2 / 0x3e9                            // the transparent layer
enable BLEND;  Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX 0xffffff)
Gu_TexWrap(REPEAT, REPEAT)
Gu_TexOffset(self+0x1180, self+0x1184)                // animated, per frame
Gu_DrawArray(GU_TRIANGLE_STRIP, 0x19f, count * 2, ..., self+0x3c0)
```

So: an **additive, scrolling, glow-writing triangle strip** that stamps a
*constant* stencil value rather than a ramp. Its caller `FUN_088739b0`
(`0x08873ccc`) sits at `0x08acb08c` in a method table whose base is
`0x08acb048` - and `0x08acb08c - 0x08acb048 = 0x44`, **the draw slot** again,
the same offset that identified `Bloom_Draw`. Its constructor and init are
`FUN_08872aa0` and `FUN_08872b68`.

**That makes it an authored `.vex` node class, not a code-built singleton** -
so unlike the bloom itself, instances of it come from track data. Its class
id is not recovered here, so *which* ribbon it is - a lap or section marker, a
racing-line strip, a tunnel band - is **not** established. What is
established is that a scrolling glowing ribbon exists as a scene-node class
and that it feeds the bloom.

## Open

- **Which class `FUN_088739b0` belongs to**, via its `Vex_RegisterClass`
  caller and then a `--nodes` count over a track. That name is what would say
  whether this is the ribbon a player recognises.
- ~~**Where `Bloom_Draw` is called from.**~~ **Whether it runs: settled
  2026-09-23** - once per race frame, 121 of 121 frames counted by a read
  watchpoint only it trips. See [`glow-mask.md`](../../../rendering/glow-mask.md).
  The dispatch itself is still unread: The constructor is on the
  `Game_MainLoop` path and the singleton is plainly live - sixteen call sites
  null-check it every frame - but the draw itself is dispatched **through the
  method table** and still has no direct xref. "It runs every race frame" is
  inferred, not read, and any per-mode or per-settings gate on it is unknown.
- **Ten of the sixteen `Bloom_SetPixelMask` call sites are unread**, and they
  are the remaining enumeration of what glows. `FUN_08915fd0` sits in the
  particle-draw neighbourhood and is the most interesting of them.
- ~~A track-wide `pass_mask & 0x40` census~~ - **done, and it is empty**: 0
  of 6,923 batches over four circuits and two ship files. The 20 track names
  that did not resolve are the remaining gap.
- The `0.01f` lag at `+0x48` and the flags at `+0x58`/`+0x59` are not traced.
  `+0x58` is a build-once latch for the tap buffers, read; `+0x48` is not
  read by anything on this page, so an adaptation or fade term exists that
  these four lists do not show.
- `FUN_08811898` is read as `sceGuPixelMask` from its two call sites and its
  argument shape, not from its body - hence `Gu_PixelMask` at 78.

## Applied names

Per [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md): below 70
gains a `_q` suffix, below 50 is not renamed at all. Mirrored in
[`names.tsv`](names.tsv).

| Address | Kind | Name | Conf |
| --- | --- | --- | --- |
| `0x089075c0` | function | `Bloom_Draw` | 85 |
| `0x089072d4` | function | `Bloom_Construct` | 82 |
| `0x089078c0` | function | `Bloom_BuildBrightPassList` | 85 |
| `0x089079d0` | function | `Bloom_BuildBlurHorizontalList` | 85 |
| `0x08907a70` | function | `Bloom_BuildBlurVerticalList` | 85 |
| `0x08907af4` | function | `Bloom_BuildCompositeList` | 85 |
| `0x08907c40` | function | `Bloom_EmitBlurTaps` | 85 |
| `0x0890710c` | function | `Gu_DrawTiledSprites` | 82 |
| `0x088115b0` | function | `Gu_TexImage` | 88 |
| `0x088107f0` | function | `Gu_DrawBuffer` | 85 |
| `0x08810fe0` | function | `Gu_CopyImage` | 85 |
| `0x08811898` | function | `Gu_PixelMask` | 78 |
| `0x08ab2348` | data | `g_bloom_blur_weights` | 88 |
| `0x08ab2344` | data | `g_bloom_composite_strength` | 85 |
| `0x08a84cf0` | data | `g_bloom_buffer_width` | 85 |
| `0x08a84cf4` | data | `g_bloom_buffer_height` | 85 |
| `0x08a84cf8` | data | `g_bloom_scratch_a` | 82 |
| `0x08a84cfc` | data | `g_bloom_scratch_b` | 82 |
| `0x08b32c60` | data | `g_bloom` | 80 |

`Gu_TexImage` and `Gu_CopyImage` score highest because their bodies write the
GE command bytes directly (`0xa0`/`0xa8`/`0xb8`/`0xcb`, and the
`0xb2`-`0xb5`/`0xeb`/`0xec`/`0xee`/`0xea` transfer block), the same evidence
class as the `Gu_*` helpers on [`mesh-draw.md`](mesh-draw.md). The four
`Bloom_Build*List` names are 85 rather than 90 because the *role* of each
list comes from the order `Bloom_Draw` replays them in, which is a reading of
the caller rather than of the callee - a strong reading, since the draw-buffer
switches between them pin which surface each one targets, but a reading.

## Measured after the port, and two earlier numbers retracted

2026-08-10, third measurement pass. The glow mask is now readable **without any
instrumentation**: `oag-game --screenshot` writes RGBA and the alpha channel of
a capture *is* the mask, so it can be isolated exactly rather than inferred.
Two earlier readings on this page's sibling were taken from clipped frames and
do not survive.

**The mask is exactly the recovered value.** Over the exhaust, a flat-mask
build reads `115` at every percentile - `0.45 * 255` to the count, which is
`intensity * 0.5 * 0.9` at saturation - and the ramped build spreads to
`91..108`. The ramp reaches the picture; the visible trail in frame is simply
all near-head, where the mask legitimately is near its maximum.

**The `2.2x` light deficit on [`exhaust.md`](exhaust.md) is retracted as a
measurement of the exhaust.** Taken at a matched pose - same track, same
recorded camera, same tick, fov-corrected - where **neither side clips**
(`0.006` and `0.020`):

| | background | contribution | clipped |
| --- | ---: | ---: | ---: |
| the original | 127.7 | 54.2 | 0.006 |
| ours | 133.2 | 36.8 | 0.020 |

Backgrounds agree to 4 %, so this is the control every earlier comparison
lacked. **Ours is 68 % of the original, not 45 %** - the `2.2x` figure came
from bands where both frames were half clipped, and clipping truncates the
brighter one more.

**The paragraph that stood here was wrong and is retracted the same day.** It
claimed a posed capture cannot measure the bloom, on the reasoning that
`Trail_DrawRibbon` refuses to draw until its ring is full and a
`--pose-from --ticks 0` frame never runs a tick. The premise about the ring is
true; the conclusion is not, because `Race::pose_boost` **already lays down a
synthetic ten-sample ring** for exactly this reason, and says so in its own
comment. The evidence offered - "identical to the digit with the bloom on and
off" - was an artefact of a measurement band that did not cover where the
bloom actually lands.

Measured properly, by rendering the same matched pose with the composite
disabled and differencing:

| pose intensity | pixels changed by more than 4/255 | mean abs diff | max |
| --- | ---: | ---: | ---: |
| `0.125` (the capture's own) | 53,532 | 1.52 | 51 |
| `1.0` | 93,447 | 8.45 | 175 |

So the bloom runs in posed frames and its strength tracks the intensity ramp,
which is what `TRAIL_GLOW_GAIN` predicts: the mask is `intensity * 0.45`, so a
capture taken at `0.125` carries barely a tenth of the glow a racing craft
does. **That, not a missing ribbon, is why tick 62 shows so little of it.**

The lesson worth keeping: *a null result from a hand-placed measurement band
is not a null result.* Two claims on these pages have now been retracted for
the same reason - a band or a background patch chosen by eye rather than
derived - and both times the differencing control settled it in one run.

**At ordinary racing speed the bloom lands in the right range.** Same scene,
three builds, per-row local background:

| build | contribution | clipped |
| --- | ---: | ---: |
| no bloom | 166.4 | 0.371 |
| bloom, flat mask | 250.2 | 0.599 |
| **bloom, ramped mask** | **243.6** | **0.580** |
| the original, tunnel | 213.4 - 344.4 | 0.476 - 0.538 |

So the bloom moves clipping from below the original's range to just above it.
**An earlier note that it "blows the exhaust out to a solid white cone" was
drawn from a zoomed crop of a *forced-boost* frame** - the most extreme case
available - and overstated the ordinary one.

**What has not been achieved is a matched comparison at a *racing* intensity.**
The one pose that matches camera and track is a teleported capture whose
intensity is `0.125`, so both sides' bloom is near its floor there; the frames
that run at a real intensity sit on a scene whose background is `131.6`
against the original tunnel's `55.7`. Until a capture matches exposure *and*
runs at a saturated ramp, the residual is not attributable between the bloom,
the exhaust's own amplitude and the standing scene-exposure gap.

**The port is wired and confirmed live on both render paths**, which is a
separate question from whether it is calibrated: `--presented`, the
window-equivalent path through the render scale, upscaler and grade, moves
26,780 pixels with the composite enabled.

## Porting notes

Everything a reimplementation needs is above, and none of it requires
hardware:

1. render the scene to an offscreen **`Rgba8` colour target whose alpha is a
   dedicated glow buffer** - `GU_PSM_8888`, so 8 bits. Ordinary mesh batches
   must leave alpha alone (measured: 0 of 6,923 authored batches open it).
   The writers are the three found above - **the exhaust ribbon's stepped
   ramp, the flare quad's own flickered alpha, and the scrolling ribbon
   class** - so a port that wires only the first will under-glow the nozzle;
2. bright pass to a half-resolution target, `rgb * a`;
3. 11-tap horizontal then vertical blur with the recovered weights, additive,
   gain `1.85` per axis;
4. composite additively at `175/255` with depth write off.

`crates/render/src/post/` already carries the offscreen-target and
fullscreen-pass plumbing (FXAA, SMAA, FSR 1), so steps 2-4 fit its existing
shape. **Step 1 is the real work**, and it reaches every surface that should
glow - see the correction above, which is the first known consumer.
