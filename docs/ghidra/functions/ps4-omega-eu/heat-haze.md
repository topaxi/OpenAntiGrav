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
