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
| **Pure (PSP)** | Pulse's, call for call: slope `1/256` set once per frame (`Gu_TexLevelMode(2, 1.0)`, `Gu_TexLodSlope(1/powf(2,8))` in `FUN_08866bec`). Per-texture `TEXLEVEL` from `Texture_BuildBindList` `0x0887f058` with the bias of `Texture_LodBiasFromDensity` `0x0887d610`. K stored per texture at `+0x2a0`, rebuilt past 0.2 (`Texture_Bind` `0x0887e900`) | Static: every function is Pulse's body. Live, Modesto GE lists: one `TEXLODSLOPE` `1/256`; 56 `TEXLEVEL` values, mode 2 bias -0.06 to +6.25, plus mode-0 words. One K in `[-9.0507, -9.0486]` reproduces the bias byte exactly for 37 of 40 matched track textures (`x18` 0.007 to 1,584). That K is not `-log2(480 * m00)` of the drawn projection (-8.32), and with the 0.2 rebuild threshold it cannot be a stale build either; its source is unread | 85 (bias form), 50 (K) | Track textures are disc chains (`9213011dd`) on the shared slope path with Pulse's constant bias 1.0: no per-texture bias, no mode-0 textures | **checked, applies, not wired**: Pure takes whatever `pulse-farfield-lod` wires, with its own header floats and K ([psp-pure-usa/texture-level.md](../ghidra/functions/psp-pure-usa/texture-level.md)) |
| **Pulse (PS2)** | One level. The GS samples `flags & 0x1f` levels beyond the base (the texture file header's own count), and every shipped texture authors 0, so a race reads the base at every distance: `TEX1` `MXL` 0, `MMIN` 1 (linear, no mipmapping) | Four in-race GS dumps, the disc census of 14,005 texture objects and the loader read: [below](#pulse-ps2-in-a-race) and [texture-level.md](../ghidra/functions/ps2-pulse-eu/texture-level.md) | 92 | `ps2_textures.rs` uploads each PS2 texture as RGBA8 with `mip_count None`, a box chain to 1x1 sampled by the hardware's derivative level. Distant PS2 surfaces are softer than the original's base-level reads | **checked, differs** (handover: ready to wire) |
| **HD/Fury (PS3)** | RSX hardware selection (derivative level) over the `.gtf`'s authored chain. Texture filter word `0x02063e80`: `LINEAR_MIPMAP_LINEAR`, bias `-1.5` (`0x1e80` as signed 5.8). Control word `0x80078000`: anisotropy 0. Environment probes `0x02062000`, bias 0 | Command rings of 2,619 + 3,935 binds on Anulpha Pass and The Amphiseum ([hd-track-tone.md](hd-track-tone.md), "The LOD bias, wired and held"); the authored chain ([gtf.md](../formats/gtf.md), "The mip chain is the disc's") | 85 | Authored chain, derivative level, `material_lod_bias` -1.5 on track and glass materials (`oag_hd::effects`, `Origin::Measured`). Anisotropy is the player's (16x default; the pairs run it off) | **ported** |
| **2048 (Vita)** | GXM hardware selection over the `.gxt`'s chain. Every texture the GXT loader makes is linear/linear, mip filter enabled, repeat, `sceGxmTextureSetLodBias(0x13)` = -1.5. No anisotropy | `GxtLoader_LoadTexture` `0x812f0ec4`, the only `SetLodBias` call site. The live canvas atlas descriptor carries raw 19 in 606 copies ([gxt-loader.md](../ghidra/functions/vita-2048-eu-v104/gxt-loader.md)). That race materials keep the loader's bits is inferred, not read (60). The Zone and Detonator track skins have their mip filter switched off after load | 85 for the loader, 60 for race materials | `material_lod_bias` is `Rule::UNREAD`: bias 0, anisotropy 16x by default | **checked, differs** (handover: measure one live race-material descriptor, then one `Rule` line) |
| **Omega (PS4)** | GNM hardware selection. Mipmapped colour samplers are `AnisoLinear` 16x with mip `Linear`, LOD bias 0.0 (+1.0 on 1 or 2 per boot). Bilinear 1x elsewhere | The native port's log of the game's own S#, four boots on three circuits ([omega-status.md](../formats/omega-status.md), "The port's anisotropy") | 90 | Bias 0 (no `material_lod_bias` rule), anisotropy 16x by default and in the `omega` pair profile. The +1.0 samplers are not mapped to materials | **ported** for bias and filter; the +1.0 samplers are open |

**2048 against Omega (CLAUDE.md's pairing): checked, differs.** Same lineage, different sampler
state:

- 2048's loader asks -1.5 with no anisotropy;
- Omega's S# ask bias 0 with 16x anisotropy.

Each is wired from its own read, never inherited from the other.

## Pulse PS2 in a race

Lane `ps2-texlevel-measure`, 2026-11-27. The claim under test was "a race samples only level 0 at every
distance". It would have been false on any in-race textured draw with `MXL` > 0, on a level image uploaded
for a texture, or on code or data that builds one. **It holds, at confidence 92.**

### GS dumps

PCSX2 v2.9.168, `SCES_547.48`, Racebox Single Race, Venom, Assegai, entered from boot with the dev-unlock
word set. Two boots, two circuits with long sight lines, two frames each. A dump holds both interlaced
fields, so every count below covers two. A *draw* is a run of textured primitives under one context,
`TEX0`, `TEX1`, `FRAME`, `ALPHA` and primitive type. A primitive is *far* when it is inside the scissor and
reads 2 or more texels per pixel, where a mipmapping sampler would take level 1 or higher.

| Frame | Textured primitives | Far | Draws | Draws with a far primitive | `MXL` > 0 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Talon's Junction White, grid at GO | 84,708 | 19,888 | 1,026 | 711 | 0 |
| Talon's Junction White, lap clock 7 s, open straight | 43,102 | 9,393 | 565 | 383 | 0 |
| Platinum Rush White, start straight | 76,516 | 17,255 | 1,128 | 681 | 0 |
| Platinum Rush White, against a wall, stands beyond | 38,107 | 6,225 | 556 | 295 | 0 |

Far primitives spread over levels 1 to 6 and over 69 to 252 distinct textures per frame, so the zero is not
for lack of anything to select a level on.

- Every textured primitive uses context 1. `TEX1_2` is never written.
- Only two `TEX1_1` values appear: `0x260` (`LCM` 0, `MXL` 0, `MMAG` 1, `MMIN` 1, `MTBA` 1, `L` 0, `K` 0;
  824 to 1,361 writes a frame) and `0x61` (`LCM` 1, `MXL` 0, linear; 10 to 16 writes). No textured
  primitive inherits a `TEX1` from before the dump.
- `MMIN` 1 is bilinear with no mipmapping, so the filter would ignore a chain even if `MXL` allowed one.
- `MIPTBP1_1`/`MIPTBP2_1` **are** written, once with every `0x260` bind. They name no level image: level 1
  is the bound texture's own CLUT address and level 3 its own base, on every primitive under `0x260`
  ([why](../ghidra/functions/ps2-pulse-eu/texture-level.md#texture_buildgsstate-0x00220e30)). `MXL` 0 means
  the GS never reads them.
- Uploads are bases and CLUTs only. Every upload no primitive reads (0 to 29 a frame) is named as `TBP0` or
  CBP by a bind whose batch drew nothing.
- The tool was checked against a failure: rewriting the grid dump's `0x260` words to `MXL` 2 makes it report
  `MXL` > 0 on 81,048 primitives.

### The loader and the disc

The upload path builds no level: `Texture_ReadFromStream` sends the file's own packets to the GS, and the two
constructors that take no file clear the level count
([texture-level.md](../ghidra/functions/ps2-pulse-eu/texture-level.md)). The engine can take a chain, but
only from the file header's `flags & 0x1f`.

So the census is of that byte. It finds a texture object by its upload packet's register words, at any
offset, so one with levels would be found. (The earlier count of 7,020 blobs could not have: it admitted
only blobs whose length closed on one level and a palette.) Over `WADS2.WAD` and `WADSP.WAD` with 443 nested
WADs expanded: **14,005 texture objects, 12,152 at the start of a file and 1,853 inside a larger one, all
with 0 levels beyond the base**. All 14,005 also carry exactly one level's block offsets and two packet
sizes (level and CLUT), as their dimensions imply. `PRERACE.WAD` (voice clips) and `PS2MUSIC.WAD` are not
texture archives and are not scanned.

### Why 92 and no higher

- A runtime trace caps at 94 without a second binary, and none exists here: PSP's law is another GPU's.
- Two of 32 circuits and one race mode were dumped. Zone, Eliminator and split screen were not, though their
  textures are inside the disc census.
- The dumps are PCSX2's record of the game's packets, not a console's.
- Not every GIF emitter that writes `TEX1` was read.
- Entries `0xb077d3bd` and `0x8d17fa0d` hold 61 and 122 texture headers, all with 0 levels, in a packing
  that is not decoded ([ps2-texture.md](../formats/ps2-texture.md#not-determined)).

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
- PS2 dumps: `pcsx2-qt` binds `GSDumpSingleFrame` to F9 under `scripts/pcsx2-drive.py`; `zstd -d` the
  `.gs.zst`, then `scripts/pcsx2-gsdump-texlevel.py frame.gs --draws`. The four dumps above are kept on the
  capture machine in `data/traces/ps2-texlevel/` (gitignored: a dump carries VRAM).
- PS2 disc: `scripts/ps2-texture-level-census.py`, its docstring has the extraction commands.
- 2048: [vita-2048-eu-v104/gxt-loader.md](../ghidra/functions/vita-2048-eu-v104/gxt-loader.md),
  "Reproducing".
