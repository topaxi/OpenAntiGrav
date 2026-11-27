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
| **Pure (PSP)** | Pulse's, call for call: slope `1/256` set once per frame (`Gu_TexLevelMode(2, 1.0)`, `Gu_TexLodSlope(1/powf(2,8))` in `FUN_08866bec`). Per-texture `TEXLEVEL` from `Texture_BuildBindList` `0x0887f058` with the bias of `Texture_LodBiasFromDensity` `0x0887d610`. K stored per texture at `+0x2a0`, rebuilt past 0.2 (`Texture_Bind` `0x0887e900`) | Static: every function is Pulse's body. Live, Modesto GE lists: one `TEXLODSLOPE` `1/256`; 56 `TEXLEVEL` values, mode 2 bias -0.06 to +6.25, plus mode-0 words. One K in `[-9.0507, -9.0486]` reproduces the bias byte exactly for 37 of 40 matched track textures (`x18` 0.007 to 1,584). That K is the pre-race flyby camera's (`m00` 1.10471), held as history: the world textures are bound in a race at K `-8.8799`, 0.17 away, inside the 0.2 rebuild threshold. The `-8.32` projection is another pass of the frame, not the one the textures bind under | 85 (bias form), 85 (K) | Track textures are disc chains (`9213011dd`) on the shared slope path with Pulse's constant bias 1.0: no per-texture bias, no mode-0 textures | **checked, applies, not wired**: Pure takes the same per-texture bias, with its own header floats and the same stored-K rule ([psp-pure-usa/texture-level.md](../ghidra/functions/psp-pure-usa/texture-level.md)) |
| **Pulse (PS2)** | One level. The GS samples `flags & 0x1f` levels beyond the base (the texture file header's own count), and every shipped texture authors 0, so a race reads the base at every distance: `TEX1` `MXL` 0, `MMIN` 1 (linear, no mipmapping) | Four in-race GS dumps, the disc census of 14,005 texture objects and the loader read: [below](#pulse-ps2-in-a-race) and [texture-level.md](../ghidra/functions/ps2-pulse-eu/texture-level.md) | 92 | `ps2_textures.rs` uploads each PS2 texture with the file's own level count (`Ps2Texture::level_count`, `flags & 0x1f` + 1), so one level; the player's anisotropy still takes its taps on it (the compare pairs run it off) | **ported** |
| **HD/Fury (PS3)** | RSX hardware selection (derivative level) over the `.gtf`'s authored chain. Texture filter word `0x02063e80`: `LINEAR_MIPMAP_LINEAR`, bias `-1.5` (`0x1e80` as signed 5.8). Control word `0x80078000`: anisotropy 0. Environment probes `0x02062000`, bias 0 | Command rings of 2,619 + 3,935 binds on Anulpha Pass and The Amphiseum ([hd-track-tone.md](hd-track-tone.md), "The LOD bias, wired and held"); the authored chain ([gtf.md](../formats/gtf.md), "The mip chain is the disc's") | 85 | Authored chain, derivative level, `material_lod_bias` -1.5 on track and glass materials (`oag_hd::effects`, `Origin::Measured`). Anisotropy is the player's (16x default; the pairs run it off) | **ported** |
| **2048 (Vita)** | GXM hardware selection over the `.gxt`'s chain. Every texture the GXT loader makes is linear/linear, mip filter enabled, repeat, `sceGxmTextureSetLodBias(0x13)` = -1.5. No anisotropy | `GxtLoader_LoadTexture` `0x812f0ec4`, the only `SetLodBias` call site. The live canvas atlas descriptor carries raw 19 in 606 copies ([gxt-loader.md](../ghidra/functions/vita-2048-eu-v104/gxt-loader.md)). **In a race**, 269 distinct disc textures bound over two frames all carry the loader's state (bias raw 19, linear, linear, mip filter on); `track.probes` bind at bias 0 with the mip filter off. One boot for the bound state, a second for the resident descriptors. The Zone and Detonator track skins have their mip filter switched off after load (static, not measured live) | 90 | `material_lod_bias` is `Rule::UNREAD`: bias 0, anisotropy 16x by default | **checked, differs** (handover: ready to wire) |
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
with 0 levels beyond the base**. The count includes copies (a nested set repeats textures the archive also
holds at top level); the distinct count was not taken. All 14,005 also carry exactly one level's block offsets and two packet
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

## Pulse PS2 wired: what moved in the pairs

2026-11-27, 22 PS2 pairs rendered before and after. 18 moved (17 race frames and the Team menu's hull
preview); a second render of the new build was pixel-identical on all 22. Every PSP, HD, 2048, Omega
and Pure pair is pixel-identical to the build before. Mean grey over the 22 went 75.38 to 75.35
(originals 76.02).

By eye at 3x on far-field crops the new frames match the originals and the old ones do not: the
box chain smeared the crowd stands and the trackside wall, and erased the tread texture on a girder
at Vertica, all of which the original and the new frame resolve.

**RMSE against the original went up all the same**, and is recorded rather than explained away:

| Over the 17 race frames | Before | After | Original |
| --- | ---: | ---: | ---: |
| RMSE, native 1280x720 | 0.1832 | 0.1878 (farther on 17) | |
| RMSE, ours drawn at 1820x1024 and Lanczos-resized as the original is | 0.1832 | 0.1855 (farther on 17) | |
| RMSE after a 16x box reduction of both, native | 0.1228 | 0.1230 | |
| Mean vertical neighbour step, 1820x1024 route | 6.58 | 7.73 | 7.15 |

The 16x figures are level, so colour and coverage did not move; the difference is in fine detail, and
the original's amount of it lies between the two builds. The likely reading is that a sharp picture
scores worse than a blurred one against a reference whose pixels do not line up with ours. Partly
tested: with the field of view 0.05 degrees wider (about half a pixel at the edge), each build's own
17 frames moved by a mean RMSE of 0.0234 before and 0.0327 after, so the new picture is the more
sensitive to a sub-pixel offset. Whether that accounts for the whole rise is **not established**.
The new frames are somewhat grainier than the originals on distant crowds: the original is drawn at
2x internal resolution and resized, ours takes one tap per output pixel. The level rule itself is the
GS dumps' measurement, not the pairs'.

## What TEXTURE DETAIL reaches

The GRAPHICS row TEXTURE DETAIL (`original`, `high`, `maximum`) is `TextureDetail::level_shift`
(0, -1, -64 levels), carried as `scene.fog.texlod_shift`. `high` and `maximum` are this project's own,
chosen rather than measured. A model takes it on one of two paths:

| Path | Which models | Where the shift goes |
| --- | --- | --- |
| Slope | a model with `Texels::Chain` (Pulse PSP, Pure) | added to the level of the GE's law (`shade.wesl`, the `texlod_slope` branch) |
| Hardware | every other: Pulse PS2, HD/Fury, 2048, Omega, and a PSP model without a chain | added to the sampler's own level (`texture_lod.wesl`, `detail_shift`) |

On the hardware path it reaches:

- every material fetch (`material_sample`), on top of the title's `material_lod_bias`, as a
  `textureSampleBias` of the sum;
- the explicit-gradient fetches (`material_sample_grad`, and HD's lightmapped glass sheen), as a
  scale of both gradients by `2^shift`;
- the effect programs' own fetches (flame, magstrip floor, light cone, absorb shell, weapon
  programs, rim, `holo.wesl`) through `detail_sample`, which adds the shift and no title bias.

At `original` the sum is the title's bias alone, so a pipeline with none takes a plain
`textureSample`. `textureSampleBias` takes -16 at the lowest, past the base of any chain, so `maximum`
reads level 0. A slope-path model's hardware fetches (its second binding and the like) take no shift,
as before.

Not reached:

- the Zone stage textures (`zone.wesl`), whose alpha is a band index;
- explicit-level reads (`textureSampleLevel`: the panel march's field, probes, shadow maps);
- sprites and particles (`oag-fx`).

**On Pulse PS2 the row has no choice**: every texture has one level, so there is no finer one to
reach. Whole frames are byte-identical across the three presets
(`texture_detail_ground_truth.rs`, `pulse_ps2_has_one_level_for_texture_detail_to_move`), so a PS2
boot drops the row (`menu_tree::trim_texture_detail_row`, `menu_web_rows.rs`). The setting and
`--texture-detail` are still read there.

Tests:

| Claim | Test |
| --- | --- |
| Level 2, 1, 0 per preset at no title bias (2048, Omega), at -1.5 (HD), and through an effect fetch | `psp_slope_lod.rs`, `texture_detail_shifts_the_samplers_own_level`, `..._adds_to_a_titles_own_material_bias`, `..._reaches_an_effect_programs_own_fetch` |
| The same through explicit gradients | `omega_panel_map_lit_path.rs`, `texture_detail_scales_the_explicit_gradients` |
| HD's whole frame differs pairwise across the presets | `texture_detail_ground_truth.rs`, `each_texture_detail_preset_draws_its_own_picture_on_hd` |
| A distant PS2 texel is a texel of the base | `psp_slope_lod.rs`, `a_distant_ps2_texel_reads_the_base_level`; `ps2_texture_binding_ground_truth.rs` over 32 track sets |

2048 against Omega (CLAUDE.md's pairing): **ported**, both take the same hardware path and the
no-title-bias test is theirs. Omega's panel-map road is the explicit-gradient test's fixture.

## Reproducing

- Pure: [psp-pure-usa/texture-level.md](../ghidra/functions/psp-pure-usa/texture-level.md),
  "Reproducing".
- PS2 dumps: `pcsx2-qt` binds `GSDumpSingleFrame` to F9 under `scripts/pcsx2-drive.py`; `zstd -d` the
  `.gs.zst`, then `scripts/pcsx2-gsdump-texlevel.py frame.gs --draws`. The four dumps above are kept on the
  capture machine in `data/traces/ps2-texlevel/` (gitignored: a dump carries VRAM).
- PS2 disc: `scripts/ps2-texture-level-census.py`, its docstring has the extraction commands.
- 2048: [vita-2048-eu-v104/gxt-loader.md](../ghidra/functions/vita-2048-eu-v104/gxt-loader.md),
  "Reproducing".
