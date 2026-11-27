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
| **2048 (Vita)** | GXM hardware selection over the `.gxt`'s chain. Every texture the GXT loader makes is linear/linear, mip filter enabled, repeat, `sceGxmTextureSetLodBias(0x13)` = -1.5. No anisotropy | `GxtLoader_LoadTexture` `0x812f0ec4`, the only `SetLodBias` call site. The live canvas atlas descriptor carries raw 19 in 606 copies ([gxt-loader.md](../ghidra/functions/vita-2048-eu-v104/gxt-loader.md)). **In a race**, three circuits (Queens Mall on two boots, Altima, Vineta K): every disc texture bound carries the loader's state, pads, weapon models and effect textures included; `track.probes` bind at bias 0 with the mip filter off. **Zone** (two boots): the Zone skins alone differ, mip filter off, a linear and a point descriptor, one level. The Detonator skins take the same loop (static only) | 92 | `material_lod_bias` is `Rule::UNREAD`: bias 0, anisotropy 16x by default | **checked, differs** (handover: ready to wire) |
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

**RMSE against the original went up all the same.** Over the 17 race frames mean RMSE went 0.1832 to 0.1878 (all
three channels), farther on every one, and 0.1228 to 0.1230 after a 16x box reduction of both, so colour and coverage
did not move. Why: [below](#why-the-error-rose-when-the-law-went-in).

## Why the error rose when the law went in

Lane `ps2-sharpness-check`, 2026-11-27. Three readings were stated before measuring: (A) the law is right and the rise
is misalignment only; (B) the original's picture is softer than ours for a reason outside the law; (C) the law is wrong
for these frames. **(B), confidence 80: the original is a 512x512 frame shown through PCSX2's 2x and stretch, ours a
1280x720 render of a different horizontal projection. Through the same chain, level 0 matches the original's detail and
the old box chain does not (C rejected).** The law's confidence stays 92. The verdict is 80 and no higher: one
original capture per pair, PCSX2's stretch filter assumed, and 18 % of the rise unexplained.

All figures below are over the 16 grid pairs. The Zone pair is left out: its state draws 640x448, not 512x512, and its
moving craft does not correlate block for block.

### The two differences between the pictures

1. **Projection, known and larger.** The original's race projection is `40/21` shown at `16:9`, ours a true `16:9`
   ([camera.md](../ghidra/functions/ps2-pulse-eu/camera.md)). Ours draws everything 7.1 % wider, so the pairs are
   misregistered by about +35 px at the left edge and -32 px at the right at 1280 wide (Metropia, Talon's Junction).
   With ours' projection aspect overridden to `40/21` (a scratch build, not committed), Metropia's block offsets outside
   the HUD fall under 2 px.
2. **The capture grid.** The 16 grid originals are GS screenshots of 1820x1024, twice 512 rows, where the Zone pair's
   is 1592x896, twice 448; each grid state's GS registers scissor 512x512, and the screenshot holds detail up to its
   vertical Nyquist, so its 1024 rows are real. (The [GS dumps](#pulse-ps2-in-a-race) above are from a boot that drew
   640x448.) At `upscale_multiplier` 2 PCSX2 therefore samples 1024 columns and 1024
   rows, stretches them to the 1820x1024 screenshot, and one Lanczos takes that to 1280x720. The original has 1024
   samples across the picture, ours 1280; down it, the original has 1024 rows into 720, ours 720. Its screenshot's
   spectrum shows it: power at the screenshot's Nyquist is 0.8 across and 93 down.

The chain was copied on our side: ours at 1024x1024 with the `40/21` projection, stretched to 1820x1024 (bilinear,
assumed: PCSX2's filter was not read), then the pair script's Lanczos to 1280x720.

### Detail

Fixed boxes on far stands and walls, one per pair, chosen on the original. Laplacian standard deviation of luma
([frame-audit.md](frame-audit.md)'s kernel, computed in numpy: that page's `fx:standard_deviation` command returns
about 3e10 on ImageMagick 7.1.2-36 HDRI), and the standard deviation of the second difference along x and y.

| Picture | Laplacian | x | y |
| --- | ---: | ---: | ---: |
| original | 10.04 | 17.19 | 21.02 |
| ours as paired, old box chain | 9.15 | 18.16 | 17.54 |
| ours as paired, level 0 | 15.70 | 32.98 | 31.54 |
| through the chain, old box chain | 6.94 | 10.91 | 14.62 |
| through the chain, level 0 | 10.60 | 17.73 | 22.45 |

As paired, the original sits between the two builds, which is what made the old picture look right. Through the chain,
level 0 lands within 7 % of the original on all three, and the old chain carries about half the energy (0.49 of the
original's high band against 1.07). Level 0 is the closer one on 11, 13 and 12 of the 16 boxes (Laplacian, x, y).

### Error

| Route | Summed mean grey, old to level 0 | Mean RMSE of the pair score |
| --- | --- | --- |
| as paired | 215.09 to 222.31 (+7.22) | 0.1502 to 0.1543 |
| projection matched | 122.62 to 126.71 (+4.09) | 0.0969 to 0.1000 |
| projection and capture grid matched | 145.68 to 147.01 (+1.33) | 0.1154 to 0.1165 |

Mean grey and RMSE here are the pair score's own (`compare_env.overlap_score`), so the RMSE does not match the
0.18 figures above. Compare within a row only: in the last route our HUD was laid out at 1024x1024 and stretched, which raises both sides'
error. Matching the projection removes 43 % of the rise and matching the grid as well another 38 %.

**The last 18 % is not closed.** It sits in the far crowd boxes: there the old chain still scores a lower RMSE after
each 32 px tile is registered on its own (20.80 old, 22.13 level 0, 8-bit, through the chain), while a band of the frame
clear of the HUD is level once registered (21.89 old, 21.74 level 0). In those boxes level 0's high band correlates only 0.68 with the original's (0.67 for the old
chain). At that correlation RMSE is lowest for a picture holding about 0.68² = 0.46 of the original's detail energy,
which is what the old chain holds, so RMSE there prefers it whether or not its level is right. Why the correlation is
not higher is not established.

**Do not read a PS2 pair's RMSE or mean grey as a verdict on texture sharpness** while ours is rendered at 1280x720
in a true `16:9`.

### Anisotropy is not the cause

Every PS2 pair already renders with `--anisotropy off`, as the compare profile records, so it took no part in the
change. Turning it to 16x on Metropia, Platinum Rush and Talon's Junction lowers mean grey by 0.05 to 0.3 on all three
routes, the direction any added smoothing goes. The GS takes one bilinear tap (`MMIN` 1); the player's setting is left
as it is and nothing is wired.

### Limits

- The original side is one capture per pair. A second grab was not possible: the grid states were saved by PCSX2
  v2.9.148 and v2.9.168 refuses them (two tried, Metropia Black and Talon's Junction White;
  [compare-page.md](../tools/compare-page.md), "Zone pairs: Wipeout Pulse PS2").
- No GS dump was taken at a pair's state, for the same reason. `MXL` 0 there rests on the dumps above and the disc census.
- Our renders repeat pixel for pixel on a second run.

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
