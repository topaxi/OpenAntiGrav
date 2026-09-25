# `Material_ApplyRenderState`: the state word becomes RSX register writes

Read 2026-08-31, cold - no material/RSX-state function was named in this
database before this pass. Resolves what separates `Transparency::Mode2`
from `Transparency::Blended`, the open question in
[rcsmodel.md](../../../formats/rcsmodel.md#the-low-two-bits-of-the-state-word-gate-the-blend-factors).
`oag_rcs::rcsmodel::material` implements what this page measures.

## Finding the lead

The route in was not a name search - nothing here had one yet. It was the
register-offset search this thread's own next-step described: RSX's NV4097
method offsets for `SET_ALPHA_TEST_ENABLE` (`0x304`), `SET_ALPHA_FUNC`
(`0x308`), `SET_ALPHA_REF` (`0x30c`), `SET_BLEND_ENABLE` (`0x310`),
`SET_BLEND_FUNC_SFACTOR` (`0x314`) and `SET_BLEND_FUNC_DFACTOR` (`0x318`) -
taken from RPCS3's own `Emu/RSX/gcm_enums.h`, the same "read an open-source
emulator's own source" move already used elsewhere on this project.
`search_instructions` for the operand `0x314` found four `ori rX, rX, 0x314`
hits - a `lis`/`ori` immediate-construction idiom - at `0x005bd8e0`,
`0x005bd8fc`, `0x005c1fe0` and `0x005c207c`, the last two inside the same
0x005c0000-0x00650000 span `Rsx_UploadVertexConstants` (`0x005c176c`,
[renderer.md](renderer.md#the-vertex-light-constants-are-read-and-where-they-live-2026-08-20))
already put this render module in.

## The two blend-func helpers, confirmed structurally

`Rsx_SetBlendFunc` (`0x005c1fe0`) and `Rsx_SetBlendFuncSeparate` (`0x005c207c`)
both push a `0x80314` header word - `(2 << 18) | 0x314`, RSX's own
count-then-method packing - followed by two data words, i.e. a batched write
to the two consecutive registers `SET_BLEND_FUNC_SFACTOR`/`DFACTOR`. The
2-argument form packs `(rgb << 16 | rgb)` and `(dst << 16 | dst)`, one factor
for both channels; the 4-argument form takes RGB and alpha factors
separately. This is `cellGcmSetBlendFunc`/`cellGcmSetBlendFuncSeparate`'s
inline body, structurally - the header's method field is the raw register
byte offset, matching the convention `Rsx_UploadVertexConstants` already
established (`0x1ea0`, `0x1efc`, not their word-shifted forms). **Confidence
92**: the header arithmetic alone fixes which two registers are written and
in what order, independent of anything about materials.

`Rsx_SetAlphaFunc` (`0x005c1ec0`) is the same shape one register pair over:
header `0x80308` = `(2 << 18) | 0x308`, i.e. `SET_ALPHA_FUNC` then
`SET_ALPHA_REF` in one batch, taking `(func, ref)` as its two arguments
verbatim. **Confidence 92**, same reasoning.

`Rsx_SetMethod` (`0x005c1d0c`) is a generic single-register poke: header
`param_2 | 0x40000` = `(1 << 18) | method`, value `param_3`. It is called
elsewhere in this module with `0x310` (`SET_BLEND_ENABLE`), `0x304`
(`SET_ALPHA_TEST_ENABLE`), `0x183c` and `0xa74` - the last two unrelated to
this thread's question and not chased further. **Confidence 88**: the shape
is generic and the four call sites confirm it is used as one, but the name
records what the header math shows rather than a single fixed purpose.

## `Material_ApplyRenderState`, and the material struct it reads

`Material_ApplyRenderState` (`0x005d8f68`) is the caller that ties the above
to a material record. Its single caller, `0x005d72b8`, passes it a pointer
read out of `*(int*)(this+0xcc)`-indexed array - the per-chunk material
lookup [rcsmodel.md](../../../formats/rcsmodel.md#where-it-is) already
documents structurally ("a chunk's `+0x20` is the material index"). Reading
`param_2` (the material pointer) at the offsets `oag_rcs::rcsmodel`
already decodes:

```text
uVar2 = *(uint*)(param_2 + 0x10)   // Material::state
Rsx_SetMethod(ctx, 0x310, uVar2 & 1)          // SET_BLEND_ENABLE      = state bit 0
Rsx_SetMethod(ctx, 0x304, uVar2 >> 1 & 1)     // SET_ALPHA_TEST_ENABLE = state bit 1
Rsx_SetBlendFunc(ctx, *(u16*)(param_2+0x14), *(u16*)(param_2+0x16))   // src/dst_factor
Rsx_SetAlphaFunc(ctx, *(u32*)(param_2+0x18), *(f32*)(param_2+0x1c) * 255.0)
```

**This is the whole answer the thread was after.** `Transparency::Opaque`
(state bits `00`) enables neither; `Transparency::Blended` (`01`) enables
blend and leaves alpha test off; `Transparency::Mode2` (`10`) enables alpha
test and leaves blend off - two different fixed-function GPU features
selected by the same two bits, which is exactly why the blend equation alone
(211 of 212 mode-2 materials share mode 1's `0302`/`0303` pair) never
separated them. **Confidence 90** for the whole chain: every offset the
decompile reads is one `oag_rcs::rcsmodel::Material` already parses
independently and had already been corroborated against real files, so this
is two independent readings of the same struct agreeing, not one reading
alone.

The scaling constant multiplying the alpha reference before the RSX write is
`0x437f0000` at its TOC slot - read directly with `inspect_memory_content`,
exactly `255.0` as an IEEE-754 float. It converts the file's `[0, 1]`-authored
reference into the register's 8-bit range.

## `alpha_func`/`alpha_ref` are disc-wide constants, not per-material tuning

Extending `oag_rcs::rcsmodel::Material` with the two new fields and
sweeping all three `PSARC` archives and all 16 circuits
(`crates/render/examples/hd_state_census.rs`'s census pattern, run ad hoc):
**`alpha_ref` is exactly `0.5` on every material measured, and `alpha_func`
takes exactly two values, `0x0201` (`GL_LESS`) and `0x0204` (`GL_GREATER`) -
every one of the six `Transparency::Mode2` material names reads
`GL_GREATER`.** Neither field varies with the material's name or its
transparency mode otherwise; this looks like a shared default the state
block is built with; the two-values-total finding is why `Material::alpha_func`
stays an unmapped `u32` rather than a small named enum the way blend
[`Factor`] is - two of RSX's eight comparison functions is thin ground for
that.

**This overturns the standing "refuted by the picture" reading**, not just
adds to it. `docs/formats/rcsmodel.md` recorded that routing `Transparency::Mode2`
through a plain `0.5` alpha-test cutout "erases the crowd entirely", reasoning
from `crowd_avatars_22x4.gtf`'s alpha channel running `0..255` at a mean of
`120`. Histogramming the same texture
(`oag_texture::gtf`, ad hoc) shows the mean is not the shape: **52.9% of
texels sit at alpha `0` and 47.1% at alpha `255`, nothing between.**
`GL_GREATER` against `0.5` keeps the 47.1% - the crowd figures - and drops
the transparent background between them, which is what a cutout does
correctly. The mean-based reading assumed a distribution a hand-authored
cutout texture is specifically built not to have. `nr_crowd_bustle`'s
fragment microcode (`crates/rcs/src/rcsmodel/material.rs`'s sibling
reading, `scripts/ps3-microcode.py fp-file`) confirms the texture in
question really is what reaches output alpha: block `#3`'s `TEX H4, R2.zwzz
unit1` samples the sampler hash `0x11cb4f74`, and `Material::texture_sampler`
for `nr_crowd_bustle` is that same hash bound to `crowd_avatars_22x4.gtf` -
the alpha test really does run on the texture the earlier reading measured,
with no combination or inversion in between.

## `Rsx_SetBlendEquation`, and the front face (2026-09-25)

`0x005c2130` writes method header `0x40320` - `NV4097_SET_BLEND_EQUATION`,
one register - with `param_2 << 16 | param_2` as its value, the colour and
alpha equations packed together, after the same `GcmContext_Callback` room
check `Rsx_SetAlphaFunc` makes. **Every caller passes `0x8006`,
`GL_FUNC_ADD`**: thirteen direct call sites (`FUN_003b22a0`, `FUN_003b41d0`,
`FUN_003b4690`'s four, `FUN_003bbd40`, `FUN_003c8820`, `FUN_003df130`'s two,
`FUN_003e3268`'s two, `FUN_003f6770`, `FUN_0040e318`, `FUN_00639590`,
`Shadow_CompileAmbientShadowTrackRedraw`, `Shadow_CompileShadowedTrackRedraw`)
and the six callers of the thunk at `0x00678238`, each `li r4, 0` / `ori r4,
r4, 0x8006` immediately before the branch. `Material_ApplyRenderState` never
calls it. So no material on the disc blends with anything but `ADD`, whatever
its name says - `plasmasphere_subtractive_glow` included. **Confidence 90**
for the name: the header math and the call sites agree, the same evidence
class as the two blend-func helpers above. Four more `ori ..., 0x320` sites
(`FUN_005bd930`, `FUN_005bd958`, `FUN_0063e5a8`, `FUN_0064a1a8`) have no
callers the database resolves and were not read.

Two neighbours read the same way, for the cull question
[hd-unlit-programs.md](../../../rendering/hd-unlit-programs.md) leaves open:
`0x005c35b0` writes `0x41830`, `NV4097_SET_CULL_FACE` - `0x405` (`GL_BACK`)
from `Shadow_RenderShipSunOcclusionMaps`, `0x404` (`GL_FRONT`) from
`Shadow_RenderModelShadowMaps` and `FUN_0040ffe0` - and `0x005c362c` writes
`0x41834`, `NV4097_SET_FRONT_FACE`, `0x901` (`GL_CCW`) at all three of its
call sites, `Gcm_InitDevice` among them. The original therefore culls
clockwise-in-window-space triangles; whether its viewport flips `y` between
clip space and window space, which decides what that means for a triangle's
winding in clip space, was not read. The two setters are named
`Rsx_SetCullFace` and `Rsx_SetFrontFace` on the same evidence as
`Rsx_SetBlendEquation`, a one-register header each, confidence 88.

## What is still open

- `oag_render` does not implement fixed-function alpha test at all, and
  `Material::blend()` currently routes `Transparency::Mode2` through
  `Blend::Factors` - the same alpha-blend path as `Transparency::Blended` -
  because both satisfy `is_see_through()`. Given `crowd_avatars_22x4.gtf`'s
  alpha is a hard `0`/`255` split, per-pixel colour output is likely close
  either way (a blend factor of exactly `0` or `1` behaves like a cutout),
  but blending and testing differ in depth-buffer interaction: an alpha-test
  surface writes depth and sorts like opaque geometry, an alpha-blended one
  typically does not. This is a real, if probably subtle, rendering
  correctness gap - not yet reproduced against a screenshot.
- Wiring a real alpha test into `wgpu` needs a shader-side `discard`, since
  `wgpu` has no fixed-function alpha-test state to bind - `mesh.wgsl` would
  need the material's `alpha_func`/`alpha_ref` as a draw-time value and a
  comparison against sampled alpha before writing colour. Scoped as its own
  step: touches the pipeline, the WGSL, and how `mesh::rcs::surface` decides
  blend state, not just the format crate.
- `0x183c` and `0xa74`, the two other `Rsx_SetMethod` call sites in
  `Material_ApplyRenderState`, gated on state bits 4 and (`uVar1 >> 7 & 1 &
  uVar2 >> 3 & 1`) respectively - the second combines a caller-supplied flag
  with **state bit 3**, not bit 7. **Named 2026-09-23 from RPCS3's own
  `rpcs3/Emu/RSX/gcm_enums.h`: `0x183c` is `NV4097_SET_CULL_FACE_ENABLE`
  and `0xa74` is `NV4097_SET_DEPTH_TEST_ENABLE`** (confidence 90), so state
  bit 4 is back-face culling and bit 3 depth testing. `oag_render::mesh::rcs`
  honours neither yet; the Plasma explosion's three models are the first
  consumers, per model, in `oag_game::race::load::weapon_models::cull_as_authored`;
  see [plasma.md](plasma.md)'s 2026-09-23 section. Neither
  was chased; bit 7's own question (alpha-to-coverage, confidence 55 in
  rcsmodel.md) is untouched by this pass and still needs its own RSX-write
  reading.
