# How each title picks a texture's level at distance

The Pulse PSP frame audit found the GE's slope rule and a per-texture bias
([frame-audit.md](frame-audit.md), section 1). This page asks the same question of every other title:
what rule picks a texture's mip level at distance, the evidence for it, and what ours does today.
Lane `farfield-lod-titles`, 2026-11-27. Read alongside the Pulse PSP law, which lane
`pulse-farfield-lod` measured:

- in slope mode, `level = TexLog2(2 * w * 1/256) + bias`;
- a slope-mode texture's bias is `trunc16(clamp(0.5 * log2(x18) + K + 8.2, -8, 7.5))`, where `x18` is
  the f32 at the texture header's `+0x18`;
- K is `-log2(480 * m00)` of the camera current when the texture's bind list was built. The texture
  stores it and rebuilds when the current K moves more than 0.2 away;
- textures whose header bits `+0x06 & 0x18` read 1 use automatic mode with bias 0.

## Per title

| Title | The law | Evidence | Conf | Ours today | Outcome |
| --- | --- | --- | ---: | --- | --- |
| **Pure (PSP)** | Pulse's, call for call: slope `1/256` set once per frame (`Gu_TexLevelMode(2, 1.0)`, `Gu_TexLodSlope(1/powf(2,8))` in `FUN_08866bec`). Per-texture `TEXLEVEL` from `Texture_BuildBindList` `0x0887f058` with the bias of `Texture_LodBiasFromDensity` `0x0887d610`. K stored per texture at `+0x2a0`, rebuilt past 0.2 (`Texture_Bind` `0x0887e900`) | Static: every function is Pulse's body. Live, Modesto GE lists: one `TEXLODSLOPE` `1/256`; 56 `TEXLEVEL` values, mode 2 bias -0.06 to +6.25, plus mode-0 words. One stored K in `[-9.0507, -9.0486]` reproduces the bias byte exactly for 37 of 40 matched track textures (`x18` 0.007 to 1,584) | 85 | Track textures are disc chains (`9213011dd`) on the shared slope path with Pulse's constant bias 1.0: no per-texture bias, no mode-0 textures | **checked, applies, not wired**: Pure takes whatever `pulse-farfield-lod` wires, with its own header floats and K ([psp-pure-usa/texture-level.md](../ghidra/functions/psp-pure-usa/texture-level.md)) |
| **Pulse (PS2)** | One level. The GS samples level 0 at every distance (`TEX1` `MXL` 0) | Disc: none of the 7,020 PS2 texture blobs in `WADS2.WAD` carries a mip level (the upload packet's size closes on the base level and palette alone, [ps2-texture.md](../formats/ps2-texture.md)). Their eight GS-state quadwords hold no `TEX1` (it is set at draw time). GS dumps: every textured prim of seven front-end dumps (8,240 to 9,584 prims each) has `MXL` 0 (`TEX1` `0x61`, `0x260`, `0x200`), plus the in-race reads of the gantry board ([start-gantry.md](start-gantry.md), `TEX1` `0x61`) and the shield (`ps2-pulse-eu/shield-pickup.md`, `0x260`, eight dumps). No race dump was taken: the race save states are from PCSX2 v2.9.148 and do not load in v2.9.168 | 75 | `ps2_textures.rs` uploads each PS2 texture as RGBA8 with `mip_count None`, a box chain to 1x1 sampled by the hardware's derivative level. Distant PS2 surfaces are softer than the original's base-level reads | **checked, differs** (handover: ready to wire) |
| **HD/Fury (PS3)** | RSX hardware selection (derivative level) over the `.gtf`'s authored chain. Texture filter word `0x02063e80`: `LINEAR_MIPMAP_LINEAR`, bias `-1.5` (`0x1e80` as signed 5.8). Control word `0x80078000`: anisotropy 0. Environment probes `0x02062000`, bias 0 | Command rings of 2,619 + 3,935 binds on Anulpha Pass and The Amphiseum ([hd-track-tone.md](hd-track-tone.md), "The LOD bias, wired and held"); the authored chain ([gtf.md](../formats/gtf.md), "The mip chain is the disc's") | 85 | Authored chain, derivative level, `material_lod_bias` -1.5 on track and glass materials (`oag_hd::effects`, `Origin::Measured`). Anisotropy is the player's (16x default; the pairs run it off) | **ported** |
| **2048 (Vita)** | GXM hardware selection over the `.gxt`'s chain. Every texture the GXT loader makes is linear/linear, mip filter enabled, repeat, `sceGxmTextureSetLodBias(0x13)` = -1.5. No anisotropy | `GxtLoader_LoadTexture` `0x812f0ec4`, the only `SetLodBias` call site. The live canvas atlas descriptor carries raw 19 in 606 copies ([gxt-loader.md](../ghidra/functions/vita-2048-eu-v104/gxt-loader.md)) | 80 | `material_lod_bias` is `Rule::UNREAD`: bias 0, anisotropy 16x by default | **checked, differs** (handover: ready to wire, one `Rule` line) |
| **Omega (PS4)** | GNM hardware selection. Mipmapped colour samplers are `AnisoLinear` 16x with mip `Linear`, LOD bias 0.0 (+1.0 on 1 or 2 per boot). Bilinear 1x elsewhere | The native port's log of the game's own S#, four boots on three circuits ([omega-status.md](../formats/omega-status.md), "The port's anisotropy") | 90 | Bias 0 (no `material_lod_bias` rule), anisotropy 16x by default and in the `omega` pair profile. The +1.0 samplers are not mapped to materials | **ported** for bias and filter; the +1.0 samplers are open |

**2048 against Omega (CLAUDE.md's pairing): checked, differs.** Same lineage, different sampler
state:

- 2048's loader asks -1.5 with no anisotropy;
- Omega's S# ask bias 0 with 16x anisotropy.

Each is wired from its own read, never inherited from the other.

## What TEXTURE DETAIL reaches

The GRAPHICS row TEXTURE DETAIL (`original`, `high`, `maximum`) is `TextureDetail::level_shift`,
carried as `scene.fog.texlod_shift`. `mesh.wesl` reads it in **one place**: the slope path
(`shade.wesl`, the `texlod_slope` branch). That branch runs only for a model with a `Texels::Chain`:

| Title | Path | Does `high`/`maximum` move the level today? |
| --- | --- | --- |
| Pulse PSP | slope path | yes |
| Pure PSP | slope path (chained since `9213011dd`) | yes |
| Pulse PS2 | hardware derivative level over a box chain | **no** |
| HD/Fury | hardware derivative level + `material_lod_bias` (`textureSampleBias`/`textureSampleGrad`) | **no** |
| 2048 | hardware derivative level | **no** |
| Omega | hardware derivative level | **no** |

The row is shown on every title (`assets/ui/menu.toml`, no title gate), so on four of six it does
nothing. Ready to wire: add `scene.fog.texlod_shift` to the bias on the hardware path. That means
`material_sample`'s `textureSampleBias`, the `exp2` scale in `material_sample_grad`, and the plain
`textureSample` reads of material textures. A `maximum` shift of -64 clamps to the base level, as it
does on the slope path. Both apply to the shader as it stands on `main` at `80ef69d91`; lane
`pulse-farfield-lod` is changing the slope path.

## Reproducing

- Pure: [psp-pure-usa/texture-level.md](../ghidra/functions/psp-pure-usa/texture-level.md),
  "Reproducing".
- PS2: `scripts/pcsx2-gsdump.py` over a dump; read `TEX1_1` bits 2-4 (`MXL`) per textured prim.
- 2048: [vita-2048-eu-v104/gxt-loader.md](../ghidra/functions/vita-2048-eu-v104/gxt-loader.md),
  "Reproducing".
