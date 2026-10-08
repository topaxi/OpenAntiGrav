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
  and its list disables `STENCIL_TEST`. ~~So what reaches alpha is the
  fragment's own alpha~~ **Corrected 2026-09-23: nothing reaches alpha.** The
  GE writes the framebuffer's alpha only through a stencil op, and with the
  test disabled it keeps the old value; the blend never touches it (PPSSPP's
  `DrawSinglePixel` and `ConvertBlendState`, and the live EDRAM read: `0`
  behind an idle nozzle, the ribbon's own ramp). The flare is not a glow
  source. See [`glow-mask.md`](../../../rendering/glow-mask.md).
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
   dedicated glow buffer** - `GU_PSM_8888`, so 8 bits. ~~Ordinary mesh
   batches must leave alpha alone~~ **Corrected 2026-09-23**: every opaque
   batch stamps `4`, a `pass_mask & 0xc0` batch its texture's glow byte, a
   transparent batch nothing, and the writers outside the mesh path are the
   exhaust ribbon's ramp and the scrolling ribbon class - not the flare, and
   not the plume. Measured out of EDRAM on
   [`glow-mask.md`](../../../rendering/glow-mask.md);
2. bright pass to a half-resolution target, `rgb * a`;
3. 11-tap horizontal then vertical blur with the recovered weights, additive,
   gain `1.85` per axis;
4. composite additively at `175/255` with depth write off.

`crates/post/src/` already carries the offscreen-target and
fullscreen-pass plumbing (FXAA, SMAA, FSR 1), so steps 2-4 fit its existing
shape. **Step 1 is the real work**, and it reaches every surface that should
glow - see the correction above, which is the first known consumer.

## The bloom draws over the HUD (2026-10-04, `pulse-bloom-roll`)

**Confidence 92.** `Bloom_Draw` is the last thing in the race's render queue,
after every HUD widget, so its composite adds the haze on top of the HUD.

- **The order is the queue's.** `Gfx_FlushRenderManager` sorts the queue with
  `Gfx_CompareQueueKeys` (`0x0891ddec`, ascending on the signed key, 95;
  [mesh-draw.md](mesh-draw.md)). A live dump of the queue (`display+0x16a0`,
  count `+0x5520`) on de Konstruct Black, two frames, 121 and 124 entries:
  the HUD's Image widgets (vtable `0x08acce34`, 10), two other widget classes
  at the same key (`0x08acf474`, 12 and `0x08acf884`, 2), and the 3-D widget
  views (`0x08acb708`) sit at keys `0x52` to `0x6d`. The bloom (`0x08ad148c`,
  this page's method table) is alone at `0x70` and last. The script is
  `data/scratch/pulse-bloom-roll/queue.py`, its output `queue.txt`.
- **The pixels agree.** The composite's display list (`*(g_bloom+0x6c)`)
  carries the source factor as the GE word `0xe0afafaf` at `+0x68`. Writing
  `0xe0000000` there turns the composite off and nothing else. With the craft
  at rest under a lit panel (`walls002_sb_GLOW`, mask 92), the 60 green
  pixels of the "Lap" and "/" glyphs read a mean of `(111, 206, 168)` with
  the composite off and `(160, 251, 235)` and `(162, 252, 236)` in the two
  frames with it on (`bloom_ab.py`, `ab1-*.png`). The glyphs are washed pale
  by the panel's glow.
- **What the HUD feeds into the bright pass.** The HUD does not stamp the
  mask ([glow-mask.md](../../../rendering/glow-mask.md); the energy bar's
  flash is the exception). Under a widget the bright pass therefore reads the
  HUD's colour times whatever the scene stamped there, and after per-tap
  truncation a mask of 4 contributes nothing. Over a glow surface the
  original blooms the widget's colour in place of the surface's. **Not
  reproduced**: our HUD is drawn at presentation resolution after the
  bright pass has read the scene.

**Ours since 2026-10-04.** `post::bloom::Bloom::prepare` runs the bright pass
and the two blurs after the scene pass. `Scene::composite_bloom` adds the
result onto the presentation target (or the capture view) after the HUD, in
`main/session/draw.rs` and `race/capture.rs`. A race parked behind the menus
composites before the menu rows; the original's order for that case was not
read. One consequence of our own: motion blur, which the original does not
have, no longer smears Pulse PSP's haze, since the composite now comes after
it. Matched against the original's rest pose on de Konstruct, ours moves
7,232 pixels by at most 15 against the old order, all where the HUD meets a
glow, and the rest of the frame is unchanged (mean `0.011`). At the same pose
at 480x272, the label glyphs of both titles sit over no glow: the original's
on/off frames differ there by under 1.1 per channel and ours by 0. The +50 on
the "Lap" glyphs above has no matched frame of ours yet.

The test `post::bloom::tests::the_composite_adds_over_a_hud_drawn_after_the_bright_pass`
pins `Bloom`'s contract, that the composite adds over whatever is drawn after
`prepare`. **It does not pin the game's call order**: putting
`Bloom::render` back in `Scene::render` passes every test. A capture-level
test needs a frame where a HUD glyph sits over a glow surface. In the old
order, such a glyph reads exactly its own opaque colour.

## Is ours stronger than the original? Yes, by the background term (2026-10-02)

The maintainer, playing: "I had bloom active and it was VERY strong on Pulse."
Measured against PPSSPP v1.20.4's **software** renderer, matched on circuit,
craft and pose, on Pulse PSP's own bloom.

**Method.** The original has no bloom switch, and `g_bloom_composite_strength`
(`0x08ab2344`) is a no-op to poke: the composite's blend factor is baked into a
display list. The list is in RAM: the one hit for the bytes `af af af e0`
(`GE_CMD_BLENDFIXEDA`, `0xafafaf`, followed by `ff ff ff e1`) over user RAM was
`0x08fb61f8` on all three boots. Writing `00 00 00` there through the debugger
removes the composite on the next frame (a no-op of the `jal` at `0x089077e4`
did nothing). `data/scratch/bloom-setting/pair.sh` captures 48 ticks of
Talon's Junction (Venom, Assegai), `--shot-every 1`, and pokes at tick 24.
Because the scene drifts (the mean of ticks 16-27 against 34-47 differs with
**no** poke), each poke run is paired with a control run that writes the *same*
value back; the figure is poke minus control. Ours is the same pose,
`--pose-from` that trace, `--ticks 1`, bloom on against off through
`settings.toml`, 480 x 272. **Ours needs the exhaust forced to the capture's
state** (`--pose-boost 10 --pose-intensity 1.0 --pose-speed 23.6`): a
`--pose-from` frame at `--ticks 1` renders the flare cold, which an earlier
version of this section did not do (see the correction below).

Mean luma added by the bloom, whole frame / around the craft (x 170-310,
y 140-272) / everywhere else:

| | racing straight, 82 km/h, flare `1.0` | at rest on the grid, flare `0` |
| --- | --- | --- |
| the original, boot 1 (3 poke, 2 control) | 1.32 / 7.3 / 0.33 | |
| the original, boot 2 (2 poke, 1 control) | 1.0 / 6.0 / 0.36 | |
| the original, boot 3 (2 poke, 2 control) | | 8.4 / 8.2 / 8.4 |
| ours before, float chain | **5.6-5.7** / 12.4-13.0 / **4.52** | 11.8 / 14.5 / 11.4 |
| ours after, truncated taps | 1.55-1.63 / 8.6-9.3 / 0.36-0.38 | **8.3** / 10.8 / 7.9 |

**Ours was 4.4-5.7 times the original over a racing frame and 1.4 times at
rest.** At rest the truncated chain matches the original to 0.4 % over the whole
frame (`8.32` against `8.35`), which is a second, independent test of the fix:
the grid's glow strips and the countdown gantry carry real mask bytes, and
only the stamp-4 floor differs. On the straight, outside the craft, the float
chain was 13 times the original (`4.52` against `0.33`), and that is where all
of the excess was.

**The cause: 8-bit arithmetic.** Every opaque batch stamps `4` into the mask
(`glow-mask.md`), so the bright pass writes `rgb * 4 / 255`, at most 4 (a white
texel). The GE blends each of the blur's eleven taps as its own additive draw
and keeps a whole byte: in the horizontal pass the three weight-64 taps keep 1
each, so at most 3 survives, and in the vertical pass `floor(3 * 64 / 255) = 0`.
**The stamp's whole contribution is zero in the original.** Our single-pass
float sum kept it and amplified it by the `3.43x` kernel gain. Truncating each
tap to a byte (`bloom.wgsl`, `fs_bright` and `fs_blur`) reproduces `0.33`
outside the craft as `0.37`; rounding to nearest gives `2.74`, so it is
truncation specifically. That the bright pass and the composite also truncate
is chosen, not measured (a unit test pins the stamp's zero, on white and on
grey).

**It is resolution independent** (the buffers are fixed at 240 x 136): the same
pose at 480 x 272, 960 x 544 at render scale 200 and 1920 x 1080 gives a delta
within 2 % (`23.1`, `23.1`, `22.7` for the float chain on the pad pose, and
`0.45` against `0.48` truncated, 480 x 272 against 1080, on the straight with
the cold flare). There was no scaling bug to fix.

**What remains.** On the racing straight ours is still `1.2-1.6x` the original
over the frame and about `1.3x` around the craft (`8.6-9.3` against `6.0-7.3`).
The flare and the plume are the candidates (their drawn size and mask ramp at
`flare_speed_kmh` 85 are the things this did not vary); unexplained, and not
chased here. **Retracted 2026-10-08**: read off the original's own scratch
buffers the excess is not there, and it was a posing and subtraction artefact;
see [Racing strength against the original's own scratch buffers](#racing-strength-against-the-originals-own-scratch-buffers-2026-10-08).

**Correction, same day.** The first version of this section measured ours with
the exhaust cold (`--pose-from` at `--ticks 1` starts the flare at intensity
`0`) and concluded the exhaust glow was "about a seventh of the original's".
That was a posing artefact, the same one this page retracted on 2026-08-10, and
the `0.95` around the craft in it is retracted. The glow mask over the craft box
with the flare forced reads `115` on 863 pixels (`0.45 * 255`, saturated
intensity), as the 2026-08-10 measurement did.

**Not achieved.** A boost-pad pair in the original: only 1 of 4 placed
approaches triggered the pad, so there is no matched on/off number for it, only
the side-by-side (the pad's glow was comparable in size) and a pad-pose delta of
`23.1` (float) to `19.4` (truncated) with a cold flare. Three boots, one circuit
and ship. Confidence **80** for "the original's background bloom is zero
because of per-tap truncation" (three boots, controls, two scenes, the
rounding alternative refuted); **60** for the racing whole-frame ratio, whose
control subtraction carries about `+-0.4` luma on a number near 1.

Scripts and frames: `data/scratch/bloom-setting/` (`pair.sh`, `nop.py`,
`ours.sh`, `ours2.sh`, `reg.py`, `cap/`).

## Is ours stronger than the original in Zone? No, about 0.9x racing (2026-10-02)

The maintainer, playing Pulse PSP: bloom feels "quite obvious" in Zone races.
Measured against PPSSPP v1.20.4's **software** renderer on a **native** Zone
race, matched on circuit (Talon's Junction White, `16_Track\zone_track.vex`),
craft (Venom class, the profile's default hull) and pose.

**Falsifier, written before capturing.** "Ours is too strong in Zone" is
refuted if ours-over-original, for the luma the bloom adds at a matched Zone
pose, stays at or below the non-Zone residual (1.2-1.6x on a racing straight).
It was refuted.

**How a native Zone race was reached (observed on two boots, recipe only).**
On a copy of `bloom-setting`'s PPSSPP profile (70 runs of use), the manual walk
`Racebox -> Custom Race -> RACE TYPE: right x5 -> ZONE` selected Zone, Track
Select offered 16 circuits (1/16 is Talon's Junction White) and `g_game_mode`
read `6` once the race was up. It is a real Zone load (its own
`zone_track.vex`, its HUD, `Results: Zone session complete`), not a patched
Single Race. **The dev-unlock byte (`+0x45f`) was set on boot 1 and left `0` on
boot 2, and Zone was selectable both times**, so on this profile it is not the
byte. What opens Zone on this profile (its progress, rather than the byte) was
not determined, and a fresh profile was not tried here, so "Zone is greyed on a
fresh profile" is neither confirmed nor refuted. `psp-drive.py menu
--race-type 5` landed on mode 3 even with the byte set (its 0.4 s press waits);
the walk that worked pressed one key at a time with 1 s waits. Zone then flies itself: a race with no input reached
zone 7 and the results screen about 60 s after GO, so a later stage needs
nothing but waiting.

**Method.** The composite is poked off at a frame boundary through the same
display-list word as above (`0x08fb61f8`, `af af af e0` -> `00 00 00`, found by
a scan on every load; it was `0x08fb61f8` again in Zone). A race's frames
drift, so the original is measured two ways: (1) a pair of runs (poke, control)
at the **start grid**, where the craft is stationary and the scene identical
before the poke (pre-poke difference `-0.03` / `+0.02`), and (2) **alternating
the composite off and on every 6 ticks inside one run** while racing, the
figure being the mean of the two neighbouring on-blocks minus the off-block
(last three ticks of each block, `scripts/psp-trace.py` breakpoint-stepped so
frames are tick-aligned). Ours is rendered at the trace's own pose
(`--pose-from`, `--pose-tick`, flare forced `--pose-boost 10 --pose-intensity
1.0 --pose-speed <trace>`; the trace reads intensity `1` in Zone even on the
grid; cause not read) with the bloom pass skipped by an
uncommitted local patch, never a setting.

Mean luma (0-255) the bloom adds over the whole frame:

| Where | The original | Ours | Ours / original |
| --- | --- | --- | --- |
| boot 1, racing, zone 2, 150 km/h, four blocks | 11.8, 12.6, 11.6, 21.9 (mean 14.5) | 9.9, 10.6, 13.1, 17.8 (mean 12.9) | 0.89 |
| boot 1, racing, zone 3, 146 km/h (scraping a wall), four blocks | 6.5, 6.2, 5.5, 4.2 (mean 5.6) | 5.1, 4.8, 5.3, 5.0 (mean 5.1) | 0.90 |
| boot 2, racing, zone 2, 137 km/h, four blocks | 10.8, 23.7, 20.5, 9.6 (mean 16.1) | 13.0, 17.5, 16.9, 9.2 (mean 14.2) | 0.88 |
| boot 2, racing, zone 5 (`flash` class), 147-156 km/h (scraping), four blocks | 18.3, 6.7, 16.3, 18.4 (mean 14.9) | 12.7, 15.5, 12.0, 12.1 (mean 13.1) | 0.88 |
| the start grid, countdown, craft at rest: boot 1 (poke, control) x2 sharing one control; boot 2 (poke, control) x2 | 10.28, 10.32; 10.89, 10.64 | 12.5-12.8 (one pose, rendered once) | 1.2 |

**Ours is not stronger racing in Zone: 0.88-0.90 of the original** over four stages-by-boots (zone 2 twice, zone 3, zone 5; two boots, sixteen blocks). Around the
craft ours is weaker still (boxes `10.4` against `13.0` in the zone 3 run), the
original's exhaust glow being the larger there. The grid frame is 1.2x, and it
is localised: split into a 3 x 3 grid, the top two thirds of the frame agree to
`0.94-1.07`, and the whole excess is the bottom third (ours `19.4 / 22.2 / 9.1`
against `9.3 / 14.2 / 0.6`). With the HUD suppressed in ours the same bottom
third remains (`19.8 / 23.1 / 14.5`), so it is not the HUD. Read out of
EDRAM at the same tick, ours stamps `214` on a strip at rows 231-243 across the
whole width where the original reads `4`, although the original's colour there
is the same bright cyan. A GE dump of a grid frame (`ge/rest.ppdmp`) shows every
`214`-stamping prim (all one texture, `GEQUAL` depth, alpha test on) **after**
the shadow pass's wipe quad (prim 189 of 427), so it is not the wipe. The cause
is **not identified**: the candidate is a coplanar decal that fails the
original's 16-bit depth test over part of its area (`glow-mask.md` already
records the decals' tie with their wall) and passes ours. It is a countdown-grid
artefact: no racing frame shows it.

**Why Zone's bloom reads as obvious: it is the original's own look.** The same
racing pose rendered by ours on the ordinary Talon's Junction environment adds
`1.47` luma (whole-frame luma with the bloom off `61`); on the Zone environment
it adds `13.1` (`164`). The original agrees with the second: `11.6-14.5` at that
pose, whole-frame luma `150-173`, and **7-13 % of the frame is clipped white**
(min channel >= 250; ours `5.6-15 %`, the bloom adds about 2 points of it in both).
Zone's mask is wide: **the original's mask is >= 200 on 13-18 % of the pixels**
in a racing Zone frame (`17,501` and `23,393` of `130,560` at two ticks; the
distribution is `255` on `10,951-11,800`, `214` on `6,278-11,388`), against
about 3 % on the ordinary circuit's grid (`glow-mask.md`). Zone's own
materials stamp their own `_GLOW` bytes over large lit panels; nothing is
added by the port. Ours applies **no Zone grade** on Pulse (`RaceDefaults::
zone_stages` is `None`; the `.effectSettings` ladder is HD's and 2048's), so
there is nothing of ours to stack with the bloom; whether the original's Pulse
Zone brightens with the zone number was not measured (bloom-off luma `139-164`
at zones 2, 3 and 5 in both, which shows no steady rise, but the poses differ).
**The player's default path draws the same as these stripped captures**: the
zone-2 pose at tick 34 rendered with every default (no `--msaa`, motion-blur,
filter or anisotropy flag, the default 1440 x 816) reads luma `177.3` and
`15.4 %` white against the stripped `177.4` and `15.5 %`.

**No fix.** The racing numbers show no excess, and the one 1.2x (countdown
grid) has no identified cause, so the PSP bloom/Zone path is untouched. The
player's "obvious" is the original's look, seen at 0.9x. If the maintainer
wants it quieter, that is a design decision against the title's own strength,
which has no setting (ruling 2026-10-02).

**Not done.** One circuit (Talon's Junction White) and one craft; zones 2, 3
and 5 only (the craft was scraping a wall at every racing capture, so none is
a clean-speed pose, and zones 6-7 were not captured); the original's mask is
read at ticks 30 and 40 of one boot only; Zone is not
modelled by the original's wipe quad either (same open as `glow-mask.md`).
Confidence **78** for "ours is 0.8-1.0x the original racing in Zone" (two
boots, three stages, sixteen blocks, one circuit; individual blocks swing
`0.4-2.3x` on scene change), **85** for "Zone's strength is authored
environment, not a mode-dependent bloom" (the same pose is 9x stronger on the
Zone environment in ours and the original matches ours), **40** for the grid
band's cause.

Scripts and frames: `data/scratch/zone-bloom/` (`zalt.sh`, `alta.py`,
`zpair.sh`, `pp.py`, `oursk.sh`, `ours.sh`, `zed.sh`, `mk.py`, `ge.py`, `cap/`).

## Racing strength against the original's own scratch buffers (2026-10-08)

Lane `bloom-racing`, maintainer's brief: "bloom is in but still 1.2-1.6x the
original's strength at a matched racing pose". **Result: the 1.2-1.6x does not
survive a direct readout. No recovered term of the chain differs; two small
arithmetic details and one mask law were fixed.**

**Falsifiers, written before capturing.** (1) *Mask*: if the original's glow
mask over the craft box agrees with ours in value and count, the mask is not
the cause. (2) *Chain*: if a CPU model of our shader, run on the original's
own framebuffer, lands on the original's scratch buffer A, the chain is right
and any excess is our scene input. (3) *Composite*: if the original's final
frame minus its pre-composite frame is not `floor(up(A) * 175 / 255)`, the
composite arithmetic differs. Any of the three failing would have been the
cause.

**Method (PPSSPP v1.20.4 software renderer, own profile, Xvfb, port 45498).**
The composite is poked off for the whole run (the display-list word at
`0x08fb61f8`, `af af af e0` to `00 00 00`, found again at the same address on
this boot), so the finished frame's framebuffer **is** the pre-composite scene
and its alpha **is** the mask, and the bloom still runs, so A and B hold that
very frame's chain output. `scripts/psp-trace.py --edram-every N --edram-dir D`
reads both framebuffers and the scratch buffers (EDRAM `0x110000` A and
`0x132000` B, stride 256, 136 rows) at a tick breakpoint, which removes the
poke-and-control subtraction the 2026-10-02 numbers carried (`+-0.4` luma on a
figure near 1). **`fb0` (EDRAM `0x0`) is the finished frame**; `fb1` is the next
one in progress. `scripts/psp-bloom-chain.py` is the model. Ours: `oag-game
--race --pose-from <that trace> --pose-tick N --pose-boost 0 --pose-intensity
<the trace's own> --pose-speed <the trace's own> --size 480x272`, bloom on
against off (a local, uncommitted `OAG_BLOOM_OFF` in `Scene::composite_bloom`),
`OAG_DUMP_GLOW_MASK` for the mask. Same ship (Assegai, Venom), Time Trial,
chase camera (`cam-craft` 11.6 on every row; a restart in the middle of the
lane flipped the trace to a 3.0 close camera and those frames were discarded).

**1. The chain is exact (confidence 85).** On the original's own framebuffer
the model reproduces its A: the mean over the frame agrees to `0.2-5 %` on six
frames (`scripts/psp-bloom-chain.py`; the residual is the bright pass's
sampling phase at lit edges and the craft, and the HUD's top rows). The
vertical pass is exact: given the original's B, `V(B)` equals A on `96-100 %`
of pixels on four frames (the rows `0-8` at the top edge carry the rest). The
tap arithmetic is **`(v * (w + 1)) >> 8`**, not `floor(v * w / 255)`: `99.1 %`
of pixels exact against `86.4 %` on the one frame both were counted on. `bloom.wesl` now does
that (it rises the added light 1 %, which is the tap's own `65/256` against
`64/255`; the stamp's zero is unchanged, `3 * 65 >> 8 = 0`). The bright pass is
the mean of each 2 x 2 block's rgb times the mean of its alpha, as ours is.
Ours is the model: the model run on **our own** inputs lands on our own
on-minus-off difference to within 1 % on all 13 frames of both circuits
(central band, HUD corners and craft excluded).

**2. The composite truncates (confidence 75).** `pre + floor(bilinear(A) * 175
/ 255)` is exact on 75 % of pixels against 8 % for the unrounded value, on a
start-line pair whose scene was not quite static (so 75 % is a floor). Ours
rounded to nearest, which keeps every fraction at or above a half; `bloom.wesl`
now truncates. Measured size on the original's own A: round over truncate is
`1.00-1.02`, so this is not the excess either.

**3. The ribbon's mask is one constant per segment (confidence 85).** Over the
craft box the original's ribbon mask holds exactly two values, `114` on 2,049
pixels and `102` on 552 at intensity `1.0` (`trunc(0.45 * 255 * (1 - k / 9))` for
`k = 0, 1`), `75` and `67` at `0.663`, `18` at `0.162`. Ours ramped across each
quad (`83-108`, mean 95). `oag_fx::exhaust::trail_stencil` now stamps the
truncated per-segment byte on every vertex of the segment, which is what
`Trail_DrawRibbon`'s `trunc(f20)` per draw does. Ours then shows `114 / 102 / 89`
as flat plateaus; the plateau areas differ (ours has more of segment 2 on
screen, a ribbon-geometry question for `exhaust.md`, not read here).

**4. The 1.2-1.6x was a posing artefact (confidence 80).** `--pose-boost 10`
advances the exhaust ten seconds at `0.25`/s from the given intensity, so every
low-intensity pose ran **saturated**: ours' ribbon mask read `~95` at a trace
intensity of `0.16` where the original reads `18`. With `--pose-boost 0` and the
trace's own intensity ours reads `14-19`. The same pose, tick 40 on Talon's
Junction, moves the craft-box ratio from `1.54` to `0.99` and the whole frame from
`1.21` to `1.07`.

**Before and after, mean luma the bloom adds over the whole frame, ours over
the original (original = clipped, from its own A):**

| Circuit | pose (tick, km/h, intensity) | before | ribbon fix | ribbon + arithmetic |
| --- | --- | ---: | ---: | ---: |
| Talon's Junction | 20, 42, 0.07 | 1.02 | 1.02 | 1.03 |
| | 60, 176, 0.24 | 1.02 | 1.02 | 1.03 |
| | 100, 202, 0.41 | 1.02 | 1.02 | 1.03 |
| | 160, 107, 0.66 | 0.94 | 0.95 | 0.96 |
| | 200, 93, 0.83 | 1.07 | 1.07 | 1.08 |
| | 240, 88, 0.99 | 0.99 | 0.99 | 1.00 |
| | 280, 87, 1.00 | 0.97 | 0.97 | 0.98 |
| Moa Therma (`03_Track`) | 40, 101, 0.16 | 1.14 | 1.14 | 1.15 |
| | 80, 252, 0.32 | 1.17 | 1.17 | 1.19 |
| | 100, 312, 0.41 | 1.13 | 1.14 | 1.15 |
| | 120, 357, 0.49 | 1.34 | 1.36 | 1.37 |
| | 140, 390, 0.57 | 1.16 | 1.17 | 1.18 |
| | 240, 23, 0.99 | 1.01 | 1.02 | 1.03 |

"Before" for Talon's is the same posing with the corrected intensity but the old
ramp and arithmetic, so the artefact in (4) is not in this table; the old posing
is the `1.54` above. **Talon's Junction: `0.96-1.08`, mean `1.01`.** Moa Therma
reads `1.03-1.37`, and the input energy tells why: summing `rgb * mask` in the
central band, ours over the original is `0.82-1.33` frame to frame and the
chain's own ratio follows it (model on ours over model on the original
`0.77-1.48`), so the swing is the **scene**, not the chain. On Moa the glow
surfaces are animated chevrons, banners and a start gantry, and a posed frame
starts those at tick 1 while the original is at tick 40-240; at `100-390 km/h` a
one-frame pose lag moves the camera 1.5-6.5 m as well (the lag minimising the
sky-band error was 4-5 ticks at 313 km/h). Where the craft is slow the two agree
(`1.02-1.03`).

**What this does not show.** The scene input itself (the colour under the mask)
is `2-5 %` brighter in ours on Talon's and `8-50 %` on Moa's emissive surfaces:
the standing scene-exposure gap `bloom.md` already named, which the bloom
inherits and which is not a bloom term. The original's bright pass also reads
the HUD's pixels, which ours' does not ([the HUD section](#the-bloom-draws-over-the-hud-2026-10-04-pulse-bloom-roll));
in the dumps the strongest unexplained B energy sits in the top rows where the
HUD is. Two circuits, one ship, three boots' worth of poses on one boot each,
PPSSPP only. Confidence **70** that no bloom term differs materially at
racing poses; **55** for Moa's `1.15` mean being scene rather than a remaining
term, since a pose at tick 1 cannot be made to match an animated emissive.

**Colour grading (Pulse PSP): there is none (confidence 80).** A GE record of
one Moa Therma frame (`gpu.record.dump`, 680 prims) has eight through-mode
(screen-space) prims in all: the shadow pass's wipe, the bright pass, the two
blur clears and sets, and the composite, **which is the frame's last prim**
(prim 679, `GU_FIX 0xafafaf`). Nothing is drawn after it and no full-screen
quad precedes it except the early wipe. One frame, one circuit, at rest, so the
same census on a racing frame and a second circuit is open. Motion blur: the
original has none (`status.md`); not re-read.

**Checked against the other titles.** The chain is Pulse PSP's own 8-bit GE
chain. HD/Fury's chain is a float HDR chain with its constants read off the
disc, so the tap and composite arithmetic does not apply to it
(`checked, differs`). HD's racing strength was read off the stored matched
captures (`hd-frame-compare.py`, Talon's Junction poses `00/01/03`): ours is
**weaker**, not stronger, whole-frame clipped white `2.85 / 6.03 / 0.37 %` against
`5.32 / 12.43 / 4.77 %`, luma `0.52 / 0.50 / 0.48` against `0.62 / 0.60 / 0.62`,
halo ring `+2px` `0.82-0.84` against `0.85-0.91`. No new RPCS3 boot was taken.
Pulse PS2 has its own chain (`ps2-bloom.md`), not touched.

Scripts and frames: `data/scratch/bloom-racing/` (`ours.sh`, `cmp.py`, `tab.py`,
`chk.py`, `inp.py`, `gelist.py`, `e3/`, `em1/`, `ge/moa_rest.ppdmp`, `report.md`).
