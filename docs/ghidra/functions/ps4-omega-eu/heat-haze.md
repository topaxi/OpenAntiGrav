# The heat-haze particle program and the distortion buffer: arithmetic read, CPU side partly open

2026-10-06, `heat-haze`. `eboot.bin` (WipEout: Omega Collection, PS4, EU), Ghidra
program `/omega/eboot-ps4-omega-eu.bin` (the **base** package's eboot). **Static
evidence only**: no PS4 emulator exists in this project's toolchain, so every
score is capped at 84 by the [rubric](../../../reverse-engineering/confidence-rubric.md).
The shaders were read with the scratch GCN disassembler
[`lightmap-prelit.md`](lightmap-prelit.md) describes. Nothing here is drawn yet:
the strength channel and the target's format are unread (see "Unknown").

This answers the question [`pob.md`](../../../formats/pob.md) left open for blend
class 8 (`Blend::Distort`): what `psys_normal_heathaze_vp/fp` does. **It is not a
scene grab refracted by the sprite.** A heat-haze particle writes a *signed
screen-space offset* into a separate buffer, and the final composite resamples the
scene at the displaced coordinate.

## Where it lives

| Item | Address | What |
| --- | --- | --- |
| Registrar | `Particle_RegisterShaders` (`FUN_01710730`, `0x01710730`) | Static initialiser for the particle shader records. Each record is `{name string, blob pointer, size}`: `psys_normal_vp` (blob `0x0195d350`), `psys_normal_fp` (`0x0195dc90`), **`psys_normal_heathaze_vp`** (name `0x018372bb`, blob `0x0195d970`, `0x31c` bytes, record `DAT_020e3928`), **`psys_normal_heathaze_fp`** (name `0x018372d3`, blob `0x0195d670`, `0x2fc` bytes, record `DAT_020e3958`), `psys_simplegeom_vp/fp`. |
| Renderer constructor | `ParticleRenderer_Construct` (`FUN_016d3bf0`, `0x016d3bf0`) | Builds the per-program handles: `kWorldViewProj`, `kColourScale`, `kDiffuseTexture`, `kDepthTexture`, `kInvSoftness`. The heathaze program is **slot 1** of a `0x260`-byte-stride program table (slot 0 is `psys_normal`; heathaze's `kWorldViewProj` handle is at `+0x1923b0`, `kColourScale` at `+0x1923b4`). |
| Batch draw | `Particle_DrawBatchesForPass` (`FUN_01711310`, `0x01711310`) (`param_1`, pass `param_2`) | Walks the particle batch list (count at `+0x393260`, 0x20-byte records from `+0x19326c`). A batch's program slot is read from its state entry (`state_table + 0x24`, stride `0x2028`); **slot 1 is drawn in pass 9**, the others in 8 or 10 (render mode `0x20000` picks 10). |
| Composite | blob `0x01956d50`, bound by `FUN_016110f0` | The five `wo_composite` siblings (`0x953920`, `0x953e10`, `0x95443c`, `0x954edc`, `0x955540`, `0x955a40` file offsets in the base eboot) all carry the distortion head below. |

The **patch** eboot (the one a race runs) was byte-searched for these bodies: they
are **recompiled, not byte-identical**. The patch's heathaze fragment program (code
at file offset `0x9d116c`) and composite (`0x9ca00c`) were disassembled and carry the
same arithmetic; the vertex program was not re-read in the patch.

## The law as read (base eboot; patch fragment and composite agree)

**Vertex (`psys_normal_heathaze_vp`, 204 bytes of code).** Standard
`kWorldViewProj` transform. Interpolants: `colour = (kColourScale, kColourScale,
kColourScale, in_colour.a)`; `uv` as authored; and a four-wide param 2:
`fb_u = 0.5 * x/w + 0.5`, `fb_v = -0.5 * y/w + 0.5` (the frame-buffer
coordinate), `fade = 2 / |w|^0.75` (`v_log`, `* 0.75`, `v_exp`, `v_rcp * 2`), and
`w` itself (linear view depth).

**Fragment (`psys_normal_heathaze_fp`, 268 bytes).**

```text
depth = sample(kDepthTexture, fb_uv).r
if !(depth >= w):                 // v_cmp_nlt_f32: scene is nearer than the particle
    out = (0, 0, 0, 0)            // lane keeps zero, alpha 0
else:
    t  = sample(kDiffuseTexture, uv)               // rgba
    k  = colour.a * fade * t.a                     // colour.rgb = kColourScale
    out.rgb = colour.rgb * k * (t.rgb - 0.5)       // signed, centred on 0.5
    out.a   = 1.0
export as packed half floats (v_cvt_pkrtz_f16_f32)
```

The sprite's RGB is therefore a **displacement vector centred on 0.5**, not a
colour. (The patch's version computes `fb_uv` in the fragment stage from clip
`xyw` and reads the same two textures; the depth gate and the product are the same.)

**Composite (`wo_composite*`, every sibling's head).**

```text
d   = sample(DistortionTexture, uv).rg
uv' = uv + 0.0100021 * (Aspect * d.x, d.y)
out = (Frame(uv') + LowResAdditive(uv') + BloomScale * Bloom(uv') + ScreenTint) * vignette
```

`0.0100021` is the immediate `0x3c23e000`. `Aspect` is `ScreenTint_Aspect.w`
(buffer offset `0x10`, dword 3): `FUN_01625750` writes `0x3fe38e39` (**16/9**) there
whenever `(DAT_01fc7c54 & 0xff) != 2`; the other mode was not read. All three
scene inputs are sampled at `uv'`, so the distortion bends the frame, the
low-resolution additive layer and the bloom together. This supersedes the
law line in [`tonemap.md`](tonemap.md), which listed `DistortionTexture` among the
composite's textures but not its use.

Confidence: **80** for the fragment, vertex and composite arithmetic (read
instruction by instruction, patch fragment and composite cross-read); **65** for
the pass number and slot mapping (read from `FUN_01711310`, not run).

## Unknown, with the addresses to read next

- **`kColourScale` per batch.** `FUN_01711310` writes it from the batch record's
  float at record offset `+0x1c` (`pfVar32[-1]`), for slot 1; for slot 0 it writes
  `2.0` or `1.0`. The batch records are pushed by about ten emitter functions
  (`FUN_0132c250`, `FUN_0132cfb0`, `FUN_016e92e0`, `FUN_016f1310`, `FUN_01714000`,
  `FUN_01714a80`, `FUN_01714de0`); which one builds a class-8 emitter's batch, and
  what it stores there, is unread. It is the whole strength scalar.
- **The distortion target:** format, size, clear value and the pass-9 blend. The
  heathaze pipeline's blend state is `BlendState_Clear` at construction; the
  per-blend variants are indexed by the state entry's `+0x14`. The fp16 export alone
  does not prove a signed float target. The render target behind `DistortionTexture`
  was not located (`DAT_01fc6a80` and `DAT_01fc6b40` are two target objects the
  composite binds; their creator was not found, only their teardown in `FUN_01621150`).
- **What `kDepthTexture` holds.** It is compared with clip `w`, so it is linear view
  depth; the resolve that produces it is unread.
- `kInvSoftness` (`1 / max(softness + batch float, 0.001)`) is set for the program
  but neither read shader uses it.

## Cross-title

- **HD: checked, differs.** `/hdfury/EBOOT-ps3-hdfury-eu.elf` has no
  `psys_normal_heathaze` and no `DistortionTexture`; it names `distortion` and
  `refractProject` uniforms ([`renderer.md`](../ps3-hdfury-eu/renderer.md)) for its
  own material and track-refraction path, and none of its `.pob` carries class 8.
- **2048: checked, differs.** `grep -a` over both Vita eboots (base and v1.04) finds no
  `heathaze` and no `DistortionTexture`; no 2048 `.pob` carries class 8.
- Omega's `emissive_alpha_heathaze_*` Tech De Ra materials (also shipped by HD) are a
  separate, material-side path and are **not** this program.

## 2026-10-06, `heat-haze-2`: the CPU side read, three items still unread

Same program (`/omega/eboot-ps4-omega-eu.bin`, base eboot), static evidence only,
so every score stays at or under 84. The "Unknown" list above is superseded by
this section: one item closed, one half closed, one open, and the pass-9 target
attachment added to it.

### Closed: `kColourScale` is authored per emitter, at emitter `+0xc84`

| Item | Address | What |
| --- | --- | --- |
| `Particle_RegisterEmitterStates` | `FUN_016d7ec0` (`0x016d7ec0`) | Walks the emitter tree (next sibling `+0x94c`, child systems `+0x944`/`+0x948`, then the layer list at `+0x9a8`). Per emitter it calls `Particle_AcquireRenderState` with `blend class == 8` as the second argument and `+0xc0` (the blend class) as the fourth, and stores the returned state index at emitter `+0xcac`. |
| `Particle_AcquireRenderState` | `FUN_01711af0` (`0x01711af0`) | Finds or appends a render-state entry (stride `0x2028`, up to 200, count at `+0x191f50`) keyed by texture, blend class, flags. Writes `entry+0x24 = (blend class == 8)`: **the program slot**, so slot 1 (`psys_normal_heathaze`) is exactly blend class 8. It also builds the sprite path `Data/Particles/%s` or `Data/Particles2048/%s` off `DAT_01f99bc0`. |
| `Particle_PushQuadBatch` | `FUN_01714000` (`0x01714000`) | Walks the particle pool (`param_1+0xf0`), writes the vertices, and appends one batch record. Its last store is `*(u64*)(batch+0x18) = *(u64*)(emitter+0xc84)` where `emitter = *(param_1+0x90)`, and `*(u16*)(batch+0x10) = emitter+0xcac`. |
| Second pusher | `FUN_01714de0` (`0x01714de0`) | The same qword copy at three call sites (`0x01715d75`, `0x01716329`, `0x01716ac9`). |

The batch record (`+0x20` stride from `param_1+0x193260`) is therefore: `+0x00` vertex
pointer, `+0x08` vertex count, `+0x10` state index, `+0x12` kind byte, `+0x18` **kColourScale**,
`+0x1c` the `kInvSoftness` addend. The previous page called `+0x1c` the colour scale; the
decompiler's `pfVar32` starts at record `+0x1c`, so `pfVar32[-1]` is `+0x18`. `FUN_01711310`
writes `batch+0x18` into the program's handle `+0x192154` for slot 1 (it writes `2.0`/`1.0`
from `state+0x28` for slot 0) and `batch+0x1c` through `kInvSoftness`.

**The value is a file field.** The `.pob` emitter record is the loaded image, so emitter
`+0xc84` is file offset `+0xc84` of the record. `crates/fx/examples/emitter_words.rs` dumps it
for every class 8 emitter in an archive (31 of 20 effects per archive; both archives, both
directories):

| Emitter | base eboot's `data00.psarc` | patch's `data05.psarc` (the one that runs) |
| --- | --- | --- |
| `shockdistort` in `WO_ROCKET_EXPLO`, `_TRACK`, `WO_MISSILE_EXPLO`, `WO_BOMB_SMOKERING`, `WO_PLASMA_LIGHTNING_EXPAND`, `WO_MINE_EXPLO`, `WO_SHIP_EXPLOSION` | `10.0` (`0x41200000`) | `2.0`, except `WO_MISSILE_EXPLO` `1.0` |
| `WO_RB_HEATHAZE` / `INNER`, `WO_RB_HAZETRAIL`, `WO_RB_HEATBOOST` | `0.8` / `1.5`, `1.3`, `1.0` | unchanged |
| `WO_ENV_SOL48_JET`, `WO_ENV_SOL2_MASSIVETURBINE` | `5.0`, `5.0` | `2.0`, `5.0` |
| others | `1.0` to `10.0` | `1.0` to `5.0` |

`+0xc88` is `0` on all of them, `+0xc80` is `1.0`, `+0xc8c` is `0.5` or `1.0` and `+0xc90`
`1.0` to `1.5`; the last two are unread. The patch retunes the explosions' distortion from
`10` to `2` (a fifth), which is a plausible fix for a shock ring that was too violent, and is
why the patch's file, not the base's, is the one to read. Confidence **85** for the
source (four push sites agree, and the values are sane floats on every one of 31 emitters);
**80** for the interpretation as the vertex program's `kColourScale` multiplier.

`slot 0` for reference: `kColourScale` is `2.0` when `state+0x28` is set, `1.0` otherwise;
`state+0x28` is the last argument of `Particle_AcquireRenderState`, which
`Particle_RegisterEmitterStates` passes as `emitter+0xbc == 3`.

### Half closed: the distortion target is `R8G8_SNORM` and its blend is pure additive

- **Format.** The composite's textures are created in `FUN_016134d0` by `FUN_017968a0(obj, 5,
  format, w, h, ...)` with the format a Gnm `DataFormat` word (surface format bits 0-7,
  channel type 8-11, swizzle 12-23). `DAT_01fc6b08` (view `DAT_01fc6b10`, bound as a
  texture by `FUN_01625750` at `0x01629210`) is created with `0x22c103`: surface format 3
  (`8_8`), channel type 1 (`SNorm`), swizzle `X Y 0 1`, at the output size. `DAT_01fc6a48`
  beside it (view `DAT_01fc6a50`) is `0x3ac706`, a float colour surface like the scene targets.
  The composite reads `DistortionTexture.rg`, which fits `R8G8_SNORM` and is the only signed
  target in the set. Confidence **75**: format decoded, the *which pass writes into it* link
  is inferred from the type, not read (below).
- **Blend.** `Particle_BuildBlendVariants` (`FUN_01710ec0`, `0x01710ec0`) fills each program's
  per-blend-class pipeline table (`program+0x30 + 0x20*class`) with `CB_BLEND0_CONTROL` words
  (`FUN_01209180`/`091a0`/`091c0`/`091f0` pack enable bit 30, separate alpha bit 29, colour
  src/comb/dst bits 0-12, alpha src/comb/dst bits 16-28). Gnm multipliers: 0 Zero, 1 One,
  4 SrcAlpha, 5 OneMinusSrcAlpha; combiners: 0 Add, 4 ReverseSubtract. The classes `1-7` get
  alpha-over, additive and reverse-subtract variants; **class 8, the heathaze program's only
  pipeline (`param_3+0x130`), gets colour `(One, Add, One)` and alpha `(One, Add, One)`**:
  `target += (out.rgb, 1)`, so offsets of overlapping particles sum and the alpha channel
  counts coverage. Confidence **80**.
- **Not read: the clear value** (zero is the only value under which an untouched pixel
  does not distort, but nothing here reads it), **and which render pass attaches this
  target** when pass 9 begins. `FUN_016304d0(9, ...)` only opens a draw list segment; the
  render-target list for a pass lives in the objects `FUN_0178a160` builds
  (`FUN_016134d0` lines for `DAT_01fc6bd8` and `DAT_01fc6b48`, neither names `DAT_01fc6b10`)
  and the pass table was not found.

### Half closed: `kDepthTexture` is `R16F`, 960 by 540, and its writer is unread

`DAT_01fc6530` (the view the batch draw binds as `program+0x19215c`) is a view of
`DAT_01fc61e0`, created in `FUN_016134d0` as `FUN_017968a0(.., 5, 0x204702, 0x3c0, 0x21c, ..)`:
surface format 2 (`16`), channel type 7 (`Float`), swizzle `X 0 0 1`: **`R16_FLOAT` at a fixed
960 by 540**, not the output size. The fragment compares it with clip `w`, so a linear
view-space depth is the only reading that makes `depth >= w` meaningful, and a 16-bit float
holds it. **Which pass writes it, and whether it stores `w` or a distance, is unread**: no
function other than the creator, the resizer (`FUN_01621650`) and the teardown names
`DAT_01fc61e0`, so the writer reaches it through a render-target object. Confidence **75** for
the format, **50** for "linear view depth" (the only consistent reading, not a read).

### What is still missing, and where to read it

1. The clear value and the attachment of the distortion target in pass 9.
2. The pass that fills the `R16F` depth, and its encoding.
3. The patch eboot's recompiled vertex shader (not read in either lane).

Nothing is drawn, for those three reasons: each is a number or a binding the drawn
strength or the depth gate depends on.

## 2026-10-06, `heat-haze-3`: the pass, the clear, the depth's writer, and the patch vertex program read

Same program (`/omega/eboot-ps4-omega-eu.bin`, base eboot) plus the patch eboot
(`data/extracted/ps4/omega-eu-patch/uroot/eboot.bin`, file offsets), static evidence
only, so every score stays at or under 84. All three items the previous section left
open are read, and so is the patch vertex program.

### Closed: pass 9 is the distortion target's pass, and it clears to `(0,0,0,0)`

| Item | Address | What |
| --- | --- | --- |
| `Renderer_OpenDrawSegment` | `FUN_016304d0` (`0x016304d0`) | Appends a 0x60-byte draw-list segment and stores its `param_1` (the pass number) at `segment+0x30de8` (`0x01630580` onward). This is the call `FUN_01711310` makes with `9` for blend class 8. |
| `Renderer_RunPassSegments` | `FUN_01629ce0` (`0x01629ce0`) | The frame's pass dispatcher. `switch((int)*(segment + 0x30de8))` (`FUN_01629ce0` line 963 of the decompile): case 5/6 = the post pass (target `DAT_01fc6bd8`), **8** `FUN_0161c960`, **9** `FUN_0161d040`, **10** `FUN_0161cf50`. The same function ends with `if ((DAT_0202f011 & 1) == 0) FUN_0161d040(...)`: **pass 9 runs even when no particle opened it**, so the target is cleared every frame whether or not a haze emitter exists. |
| `Renderer_BeginDistortionPass` | `FUN_0161d040` (`0x0161d040`) | `FUN_01788a40(param_1, &DAT_01fc6c68, 0, &DAT_01fc6ac8, 0, 0)`: render-target object `0x01fc6c68`, **no depth** (third argument 0), **one colour attachment: the surface descriptor `DAT_01fc6ac8`, the copy of `DAT_01fc6b08`'s descriptor, i.e. the `R8G8_SNORM` `DistortionTexture`**. Then `FUN_01793990(0, 0, 0, 0, pass, attachment)` for every bound attachment. |
| `Gnm_ClearSurface` | `FUN_01793990` (`0x01793990`) | `(r,g,b,a, pass, surface)`: a cache flush then `FUN_0122cac0(ctx, surface, &{r,g,b,a})`, which packs the four floats with `FUN_012289a0` into the surface's own format (the same call `FUN_01621650` uses to pack the depth surface's fast-clear colour). A **clear**, with the colour in the first four arguments. |
| `Renderer_BeginScenePass` | `FUN_0161c960` (`0x0161c960`) | Pass 8, the scene: `FUN_01788a40(pass, &DAT_01fc6b48, 0, colour0, &DAT_01fc61a0, colour2)`, so **`DAT_01fc61a0` (the descriptor of the `R16F` `DAT_01fc61e0`) is the scene pass's colour attachment 1**, written by the scene's own fragment shaders. |
| `Renderer_BeginPass10` | `FUN_0161cf50` (`0x0161cf50`) | Pass 10: `FUN_01788a40(pass, &DAT_01fc6c20, 0, &DAT_01fc6a08, 0, 0)`, one colour attachment, the `0x3ac706` float surface (`DAT_01fc6a48`, the `LowResAdditive` input of the composite); the same `(0,0,0,0)` clear. |
| `Renderer_ResizeTargets` | `FUN_01621650` (`0x01621650`) | Creates the targets at `(param_6, param_7)`: the `R16F` depth (`0x204702`), the `R8G8_SNORM` distortion target (`0x22c103`) and both pass objects `0x01fc6c20`/`0x01fc6c68` (their size words `+0x4`/`+0x6` are `param_6`/`param_7`). **Distortion target and depth share one size**, and the scene pass's own size is the other pair `(param_4, param_5)`. |

The distortion target's clear value is therefore `(0,0,0,0)`, and in `R8G8_SNORM`
that is zero displacement: an untouched pixel does not distort. Confidence **80** (the
pass number, the dispatcher's switch, the attachment list and the clear call are each
read; the call that opens pass 9 stores the same field the switch reads).

### Closed: the depth's writer is the scene pass, and its clear is `1000.0`

`FUN_0161c960` clears its attachments from a table on its stack
(`[RBP-0x60]` = `0x01fc6d60`, `[RBP-0x50]` = the constant at `0x017f92c0`,
`[RBP-0x40]` = `0x01fc6d70`, `0x0161caa5`..`0x0161cabf`, then four floats per
bound attachment at `0x0161cae8`..`0x0161cb07`). The constant at `0x017f92c0`
reads (`read_memory`) `00 00 7a 44 | 00 00 80 3f | 0 | 0` = **`(1000.0, 1.0, 0, 0)`**,
attachment 1's clear colour, so the `R16F` depth starts every frame at `1000.0`.
`FUN_01621650` independently packs `0x447a0000` (1000.0) as that surface's fast-clear colour
(`local_68 = 0x447a0000`, `FUN_012289a0`, written to the descriptor's `+0x2c`/`+0x30`).
Two reads, one value: **far is 1000.0**, a distance in the same units as the heathaze
program's clip `w`, which is what its fragment compares it with. Confidence **75**.

**Not read: what the scene's fragment shaders write there.** 13,326 of the 27,657
distinct `.rcsmaterial` shaders in the base `data00.psarc` export to `tgt=1` (a packed
half export), and several carry the literal `0x447a0000`; the scratch GCN disassembler
mis-decodes the middle of them (`.word 0xcc..` opcodes), so no export's source value was
traced to a `w`, a view `z` or a radial distance. The gate in the heathaze fragment is
`depth >= w` either way; planar versus radial stays **open** and any implementation that
uses a linearised depth buffer is **chosen, not measured** for that point.

### Closed: the patch vertex program agrees

The patch's heathaze vertex program is the blob at file offset `0x9d147c` (176 bytes;
the other two blobs carrying the `0.75` literal, `0x9d177c` and `0x9cfb2c`, multiply the
colour by the input colour and are the `psys_normal` variants). Instruction by
instruction: position `exp tgt=12` from `kWorldViewProj`; `v_log_f32 |w|`, `* 0.75`,
`v_exp_f32`, `v_rcp_f32 mul:2` = **`fade = 2 / |w|^0.75`**; params:
`param0 = (kColourScale, kColourScale, kColourScale, in.alpha)` where `kColourScale` is
`s_buffer_load_dword [cb + 0x10]`, `param1 = uv`, `param2 = (clip.x, clip.y, fade, w)`.
The only difference from the base program is that the patch exports **clip `xy` and the
fragment forms `fb_uv`** (`attr2.xy * rcp(attr2.w) * (0.5, -0.5) + 0.5`); the base
program forms it in the vertex stage. The patch fragment (`0x9d116c`, re-read in full
this lane) matches the law above exactly: gate `v_cmp_nlt_f32 depth, w`, `k = colour.a *
fade * t.a`, `out = colour.rgb * k * (t.rgb - 0.5)`, alpha 1.0, `(0,0,0,0)` where the gate
fails. Confidence **80**.

### Still not read

- What the scene shaders store in the `R16F` (planar `w` or radial distance): see above.
- The pixel size of the distortion target and depth (`FUN_01621650`'s `param_6`/`param_7`;
  `FUN_016134d0` creates the depth at a literal `0x3c0` by `0x21c` first). The composite
  samples both with a normalised coordinate, so the law does not depend on it.
- Nothing in the three items above needed a live run: there is no PS4 emulator in the
  toolchain, so no score exceeds 84.

Cross-title: HD and 2048 unchanged (no `psys_normal_heathaze`, no `DistortionTexture`; see
the first section).
