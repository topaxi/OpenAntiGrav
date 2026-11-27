# 2048's GXT loader: every loaded texture is trilinear with a -1.5 LOD bias

**Binary:** `vita-2048-eu-v104` `eboot.elf`, image base `0x81000000`, Ghidra
`/2048/eboot-vita-2048-eu-v104.elf`. Read 2026-11-27 (lane `farfield-lod-titles`). The GXM imports
are named by NID from vitasdk's `vita-headers` database (`db/360/SceGxm.yml`). Per-title summary:
[texture-level-per-title.md](../../../rendering/texture-level-per-title.md).

## The function

`GxtLoader_LoadTexture` (`0x812f0ec4`, confidence **85**) loads one `.gxt` into a
`Pal::Render::Texture`. Its own messages name it:

- the allocation tag `"PSP2/Psp2.GxtLoader.cpp"`;
- `"The GXT file is abnormally to small. 64 bytes are required at least for the header section."`;
- `"Not enough Vram and MainUncache memory available to allocate GXT texture %s"`.

It reads the 64-byte header and allocates the texture object (`0x14` bytes) and the texel memory. It
then initialises the `SceGxmTexture` by the header's type word:

| Type word | Init call |
| --- | --- |
| `0x00000000` | `sceGxmTextureInitSwizzled` |
| `0x60000000` | `sceGxmTextureInitLinear` |
| `0x40000000` | `sceGxmTextureInitCube` |
| `0xa0000000` | `sceGxmTextureInitSwizzledArbitrary` |
| `0xe0000000` | `sceGxmTextureInitCubeArbitrary` |

Then, for **every** texture it loads, unconditionally (`0x812f1172..0x812f11a8`):

| NID | Call | Argument | Meaning |
| --- | --- | --- | --- |
| `0xFA695FD7` | `sceGxmTextureSetMagFilter` | 1 | linear |
| `0x416764E3` | `sceGxmTextureSetMinFilter` | 1 | linear |
| `0x8699ECF4` | `sceGxmTextureSetUAddrModeSafe` | 0 | repeat |
| `0xFA22F6CC` | `sceGxmTextureSetVAddrModeSafe` | 0 | repeat |
| `0x1CA9FE0B` | `sceGxmTextureSetMipFilter` | `0x200` | mipmaps enabled (linear between levels) |
| `0xB65EE6F7` | `sceGxmTextureSetLodBias` | `0x13` | raw 19 |

`sceGxmTextureSetLodBias` has **one call site in the binary**: this one. Raw 19 reads as
`(19 - 31) / 8` = **-1.5 levels**, the decode Vita3K applies (read for behaviour). No call sets an
anisotropic filter.

Two independent sources agree on the value:

1. the static call above;
2. the live `SceGxmTexture` of the front-end canvas atlas, which carries `lod_bias` raw 19 in all 606
   copies read from Vita3K's RAM ([canvas3d-lattice.md](canvas3d-lattice.md), "The atlas
   descriptor decoded").

`GxtLoader_LoadTexture` has no direct caller: its address sits in a function-pointer table at
`0x81517624`, so the resource system reaches it. `SetMinFilter` and `SetMipFilter` have about twenty
other callers, which never call `SetLodBias`. Two were read:

- **`FUN_810242e8`**: render targets, mip filter off.
- **`Environment_LoadHDFuryContent`**: the same, plus the **Zone and Detonator track skins**. It loads
  fifteen `data/Tex/zoneModeTrackN.gxt` (or `DetonatorModeTrackN.gxt` when the mode word `DAT_8153fd24`
  is `0xe`) into `DAT_815d2d0c` (allocated under `Game/FunkLayer.LIVE/FunkLayer.zone.cpp`). For each it
  sets min and mag filter linear and **mip filter 0 (off)**, then copies the descriptor into
  `DAT_815d2d48` and sets the copy to **point** min and mag, mip filter off. Nothing sets the bias, so
  both keep raw 19. The v1.04 patch (`data2.psarc`) ships these skins with **one level** (256x256 RGBA8),
  where the base `data.psarc` has seven, so no bias or mip filter has anything to select.

`FUN_81043b1c` (nine call pairs) and the rest were not read.

## In a race: the bound descriptors (2026-11-27, `v2048-texlevel-measure`, `v2048-texlevel-more`)

The claim under test: every disc texture bound in a 2048 race is sampled with the loader's state. It is
false for a texture whose live `SceGxmTexture` has a bias other than raw 19, mip filter off, a min or mag
filter other than linear, or a minimum level other than 0. **As worded it is false: the Zone skins differ.
Every other disc texture read carries the loader's state, in five race runs and two Zone runs.**

**Method.** Vita3K 0.2.1 (`885f9168d`), the EU v1.04 game. Guest memory was dumped under `SIGSTOP`
([vita3k-capture.md](../../../reverse-engineering/vita3k-capture.md), "Which textures a frame binds").
**Bound state** is Vita3K's queued `SetState`/`Texture` commands, which carry the 16-byte descriptor each
bind used. Each descriptor's texel address was matched to a `.gxt` on the disc by the texels themselves.
Every dump also shows non-loader states (probes, render targets), so the read can see a difference.

| Circuit | Mode | Boot | Dumps | Distinct disc textures | Differing |
| --- | --- | --- | ---: | ---: | --- |
| Queens Mall (native) | Race | 1 | 2 (0:10, 0:25) | 269 | none |
| Queens Mall | Race | 3 | 3 | 311 | none |
| Vineta K (DLC1, HD Campaign) | Race | 4 | 3 | 228 | none |
| Vineta K | Race | 7 | 3 | 216 | none |
| Altima (native; see below) | Race | 6 | 3 | 184 | none |
| Vineta K (HD Campaign) | Zone, zones 4 and 6 | 2 | 2 | 29 | 3 Zone skins |
| Vineta K | Zone, zones 4 and 6 | 5 | 2 | 29 | 3 Zone skins |

- Each boot number is a separate emulator process. Boot 1 is the first lane's.
- Queens Mall's boot 3 shares 263 of boot 1's 269 textures, all with the same state: the bound state on a
  second boot.
- Altima is a substituted event: the HD Campaign's Race cell with its circuit name overwritten in memory
  (`vita3k-mem.py rename-track Vineta_K altima`). Mode, class and craft are the HD cell's. 109 of its 141
  environment textures are under `environments/altima/`.
- A Zone race draws the track from the skins and binds only seven track and scenery textures, so the Zone
  runs say little about track materials. The 26 textures of each Zone run that are not skins (craft, pads,
  effects, HUD and those seven) all read the loader's state.

**By family**, state as bias raw, min, mag, mip filter (minimum level 0 on all):

| Family | Examples bound | State |
| --- | --- | --- |
| track and scenery, lightmaps | `environments/*/textures`, `lmaps/*-lmap.gxt` | 19, 1, 1, 1 |
| craft | `ships/*2048`, `hdships/*` | 19, 1, 1, 1 |
| speed and weapon pads | `speedpad_diff/norm`, `weaponpad_diff/norm`, DLC `ds_speedup_cs/ne` | 19, 1, 1, 1 |
| weapon models | `hd_rocket1`, `hd_mine1`, `hd_bomb*`, `hd_leechbeam*` | 19, 1, 1, 1 |
| exhaust flame and shield absorb | `flame_01`, `flame_01_hd`, `zonebattle_hd_absorbinternal` | 19, 1, 1, 1 |
| particles, ribbon trails, flares | `particles/tex`, `particles2048/tex`, `ribboneffects`, `engineflare`, `lensflare`, `smoke_trails` | 19, 1, 1, 1 |
| billboards, HUD | `billboards/hd_adverts`, `2048_hud` | 19, 1, 1, 1 |
| **Zone skins** | `zoneModeTrack2..4.gxt` | **19, 1, 1, 0** (linear) and **19, 0, 0, 0** (point copy), one level |

- On every disc texture bound in a race dump of boots 3, 4, 6 and 7 the descriptor's level count is the
  file's, so each is read over its own full chain. The exception is the Zone skins, whose one level is the
  v1.04 file's. On boot 1's wedged repeat, 226 of its 269 textures were resident with the loader's state,
  and so were all 444 environment and ship textures with a descriptor.
- The effect textures named above have 2 to 8 levels, so the bias selects levels on them. Two weapon flash
  textures (`pulse_bombflash_glow`, `pulse_mineflash_glow`) have one level.
- No texture of a magstrip, light cone, hologram or rim effect was bound in any run, so those are unread.
- This is the descriptor's state. A fragment program can still choose a level itself; no program was read
  for that.
- Zone skins, both boots: at zone 4 the linear descriptor of skin 3 and the point copies of skins 2 and 3
  are bound; at zone 6 the linear descriptor of skin 4 and the point copies of skins 3 and 4. The live
  descriptors of all fifteen pairs match the static loop above.
- No anisotropy: apart from the point copies, a few point binds of run-time textures and one stray
  record, min and mag read plain linear, the filter values Vita3K declares include no anisotropic one,
  and the loader makes no anisotropy call.

**Bound textures that are not `.gxt` files** follow other states:

- the circuit's `track.probes` (64x32): bias raw 31 (0.0), linear/linear, mip filter off;
- 768x512 targets: raw 31, point, mip filter off;
- in Zone, 64x2 (raw 31, linear) and 256x32 (raw 31, point) run-time textures, mip filter off.

**Outside the race**, two front-end textures, `tex/photomode_dots` and `tex/sony_legal_text`, keep bias 19
with the mip filter off (boot 1).

**Instrument traps.** A search of all memory for descriptors also finds stale renderer command slots (partly
overwritten, so the filters read 0) and stray words that decode as descriptors. Neither is bound state; only
the command read counts. The Zone skins are too flat for a match on 64 distinct bytes: they were matched on
their whole level 0 and on the pointers at `DAT_815d2d0c`.

## What this says, and what it does not

- **Read, confidence 92.** A race, on the three circuits above, samples every disc texture it binds
  trilinear over the file's own chain with a -1.5 bias and no anisotropy. A runtime read in an emulator,
  and Omega does not agree, so it stays under 95.
- **Read, confidence 92.** The Zone skins are sampled with the mip filter off, linear and point, from one
  level. Static loop and two live boots agree.
- **Static only.** The Detonator skins take the same loop (confidence 80 for their live state). No
  Detonator race was run.
- **Not measured.** Fury (DLC2) circuits, the other circuits, a native circuit's Zone, Eliminator, split
  screen.
- **Score history.** 2026-11-27: 90 on Queens Mall, bound state from one boot (`v2048-texlevel-measure`);
  92 on five race runs across three circuits, with Queens Mall bound on a second boot (263 of 269 shared)
  and the Zone skins seen live on two boots (`v2048-texlevel-more`).

## Ours

2048's `Looks` carries the loader's bias as data (`oag_title::Looks::material_lod_bias` and
`every_texture_lod_bias`, both `Rule` `Any`/`Measured`, `oag_title::MATERIAL_LOD_BIAS = -1.5`). **Ported
for the bias.** `Scene::new` takes it and puts it on every pipeline of a race:

| Family | How the bias reaches it |
| --- | --- |
| track, scenery, lightmaps, pads, craft, weapon models, shields, adverts | the pipeline cache scope's override constants, on the material fetches (`material_lod_bias`) |
| flame, absorb shell, weapon programs and the other effect programs' own fetches | `detail_sample` (`effect_lod_bias`, `texture_lod.wesl`) |
| particle sprites | `psys` and distortion pipelines, over a sprite atlas that carries each `.gxt`'s own levels |
| flare, ribbon pair, leach beam, Cannon quads, wingtip contrails | each `oag_fx` pipeline's `disc_lod_bias`, over the file's own levels |
| the time-trial ghost's hull | `oag_render::ghost`'s hull pass |
| Zone track skins | no bias reaches them (`zone.wesl` has its own samplers); bound from one level (`ZoneStageTextures::track_single_level`) |

Environment probes are read at an explicit level and stay at bias 0, as the original's are. Anisotropy
stays the player's setting; the 2048 compare profile runs it off.

Levels: every 2048 effect `.gxt` the sprites and ribbons name authors 3 to 9 levels
(`data/particles2048/tex` is PVRTC), the ribbon textures 6 or 7, `engineflare_rich` 6. The atlas holds
five, a chosen depth (`oag_fx::psys::sprite::ATLAS_LEVELS`): a sprite sits on a 16-texel grid with its
edge texels repeated around it, and a level a file does not author is a box filter of the one above,
also chosen. Every sprite of a 2048 race still fits the 4096 atlas.

Open or inherited:

- **Material textures sample a box-filtered chain, not the file's own levels.** The bias is the
  original's; the levels it selects are ours.
- **The Zone skins' point descriptor** is not drawn: ours binds them through `zone.wesl`'s samplers.
- **Detonator skins** keep a synthesised chain: their live state is unread (static only, confidence 80).
- **Magstrip, light cone, hologram and rim effects** take the bias on the static loader read alone: no
  run bound one.
- **The HUD and menus** are not wired. Their textures read -1.5 in the original; at one texel a pixel
  or fewer that bias selects the base anyway.

The ground truth is `scene_build.rs`'s `v2048_biases_every_race_pipeline_and_its_circuit` (every
distinct pipeline, the effect textures, the ghost) and `zone_grade_ground_truth.rs`'s
`a_2048_zone_race_binds_its_track_skins_from_one_level`; `psp_slope_lod.rs`, `chain_draw.rs` and the
ghost's tests read the selected level off a draw.

## Omega (the reverse check)

**Checked, differs.** Omega's native port logs the game's own S# per sampler
([omega-status.md](../../../formats/omega-status.md), "The port's anisotropy"):

- `AnisoLinear` 16x with mip `Linear` on 5 to 8 of each boot's 16 to 19 samplers;
- LOD bias 0.0, and +1.0 on 1 or 2 of them;
- no negative bias anywhere.

So Omega's PS4 build does not carry 2048's -1.5. Its sharpness comes from 16x anisotropy instead.
**Ours: checked, differs.** Omega's `Looks` leaves both rules unread, so its race draws at bias 0.

**The Zone skins: checked on the files, applies.** Omega's `Data/tex/zoneModeTrack0..14.gnf` and
`detonatorModeTrack0..14.gnf` (`data00.psarc` and `data08.psarc`) are 256x256 with one level
(`last_mip_level` 0), as the v1.04 skins are. Their sampler state on Omega is **not checkable**: the
port's S# log is not mapped to textures.

## Reproducing

```sh
ghidra: decompile 0x812f0ec4 in /2048/eboot-vita-2048-eu-v104.elf
ghidra: xrefs to 0x813f2630 (the SetLodBias import stub)   # one call, 0x812f11a4
```

The race read: [vita3k-capture.md](../../../reverse-engineering/vita3k-capture.md), "Which textures a frame
binds".
