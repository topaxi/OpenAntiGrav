---
status: ready-to-wire
titles: [pure]
updated: 2026-10-26
summary: "Pure's sun flare law is measured (docs/rendering/lens-flare.md); wire a 2D additive pass; shadows.md Pure zero in doubt; HD/2048/Omega unread"
---

# A sun lens flare on Pure, and which titles carry it

2026-10-26, `lens-flare-scout` (read-only census, no code changed). The maintainer
watched a clear sun lens flare on Wipeout Pure's Vineta K under PPSSPP. Nothing in
our code draws one on any title: `crates/` names the class only in
`crates/vex/src/vex/class_names.rs` (HD's table, `0x3de`), and the envsettings
parser reads `Lighting.Use Lens Flare` as a flag nobody reads. Not to be confused
with the engine or exhaust flare (`Engine Flare` locator, `WO_ROCKET_FLARE`) or
HD's craft sprite flare (`engineflare.vex`).

**What the maintainer saw (PPSSPP, 2026-10-26), the acceptance picture:** at
Vineta K's start a faint circle sits at screen centre; driving forward, a chain
of rings runs from the centre toward the bright point in the skybox (the sun).
A classic lens flare: sun anchor plus a ring set laid along the line through
screen centre. The compare pair `pure-psp/vineta` shows it only faintly.

Pointers for the RE lane, not yet checked:
- Sun position: Vineta K's `Skycube` (`oag-view --sky`) and the
  `DirectionalLight` nodes (`0x12a` on Pure,
  `docs/ghidra/functions/psp-pure-usa/scene-light.md`).
- Ring textures: `LensflareComponents.tga` (a `Texture` node on Vineta K, texel
  block not decoded); `glare3.tga`, `GFLARE.tga`, `absorb_glare.tga` under
  `\Art_Resources\Psys\Tex\` are candidates.
- Per-ring offsets: the `lensflare_shape_` child payloads, the ring count in the
  80-byte `lensflare_` payload, then any table near `FxLensflare.cpp` in BOOT.

## Answer per title

| Title | Sun or lens flare | Where it is authored | Evidence |
| --- | --- | --- | --- |
| **Pure (PSP, USA)** | **present** | `lensflare` class nodes (id `0x390`) in circuit `track.vex` files, one anchor per circuit | Vineta K has two nodes; seven of the sixteen circuits name one (below) |
| **Pulse (PSP, USA)** | **absent (authored)** | nowhere | `lensflare` `0x3de` is in the exporter's name table only; zero nodes in 382 v6 files (`docs/rendering/shadows.md`); zero flare names in all 1,142 entries of `Data.wad` |
| **HD/Fury (PS3, EU)** | **present, module-drawn** | `Lighting.Use Lens Flare` flag in the circuits' `.envsettings` (true on 29 of 32, `docs/formats/envsettings.md`, which scopes its count to HD circuits); module in the EBOOT; textures `LensflareComponents_256x256.gtf` and `_Rays.gtf` in `DATA02.PSARC` | No `lensflare`-named node in any of the sixteen HD `track.vex` files (sixteen entries pulled with `scripts/psarc.py cat` and grepped, all zero) |
| **2048 (Vita, both base eboots)** | **present in the executable** | same module and key as HD | Asset census not run (`.pkg`) |
| **Omega (PS4, EU)** | **present** | `Lighting.Use Lens Flare` plus the H-ray and sun-scale keys; `LensflareComponents` textures and `lensflare_*` nodes in the base PSARCs | Node census not run (PSARC); shadPS4 frames show it (`docs/tools/compare-page.md`, Omega row, "off: ... HUD and lens flare only") |

Labels: **present** = authored data or a shipped module with its key; **absent** =
checked, not authored, not shipped; **unknown** is not used here.

## Pure, measured

**Class id.** The class-name run is identical on Pulse and Pure through index 34
(`lensflare` is name 35 in both `BOOT.BIN`s, at the same relative offset). Pure's
version-4 table has no `lensflare` field yet, but the run's own gap rule
(`class_table_ground_truth.rs`, `the_derived_table_is_the_one_oag_formats_states`)
predicts `0x36b + 34 + 3 = 0x390`. Confirmed by name: every `lensflare_*` node
below reports as `0x390` in `oag-view --nodes`. Pure's `oag-view` census shows
`0x390` as "unnamed - not in the read table", so nothing decodes it.

**Vineta K** (`Data\Environments\01_Vineta_K\track.vex`, `Data.wad` entry hash
`94caadb7`, entry 827, 5,609,984 bytes; 1,315 nodes):

- `lensflare_` (`0x390`, 80-byte payload) with child `lensflare_shape_` (`0x000`).
  The payload is a row-major 4x4: rows 0-2 identity, row 3 =
  `(-51.65, 12.76, 23.4, 1.0)`, then one integer `187` (`0xbb`). So it is a world
  anchor, not a model. Its position is the flare's source in the sky.
- `Texture` `0x373` node `Z:/Data/defaults/LensflareComponents.tga`, 96 bytes. The
  texel data is in the file's texture block and is not decoded here.

**Other circuits with a named flare node** (same name search over all of `Data.wad`;
node census by `oag-view --nodes` on the seven track entries):

| Circuit | Entry | `0x390` nodes | Names |
| --- | --- | --- | --- |
| Sol 2 | 757 | 2 | `lensflare_`, `lensflare_shape_` |
| Sebenco Climb | 766 | 2 | `lensflare_sun`, `lensflare_sunshape_` |
| Citta Nuova V | 777 | 2 | `lensflare_citta_sky`, `..._citta_skyshape_` |
| Blue Ridge | 797 | 2 | `lensflare_BluRidge`, `..._BluRidgeshape_blueridge` |
| Chenghou Project | 807 | 2 | `lensflare_Cheng`, `..._Chengshape_` |
| Modesto Heights | 817 | 2 | `lensflare_sun`, `lensflare_sunshape_` |
| **Vineta K** | **827** | **2** | `lensflare_`, `lensflare_shape_` |
| Sinucit | 787 | 0 | none (1,288 nodes walked) |

`Data.wad` names eight circuit environments (`Data/Environments/`: 01 Vineta K, 03
Modesto Heights, 04 Chenghou Project, 07 Blue Ridge, 08 Sinucit, 09 Citta Nuova V,
10 Sebenco Climb, 12 Sol 2). `docs/formats/pure-status.md` counts sixteen circuits,
so the other eight have no environment-name string in this WAD and were not walked
by path. The whole-`Data.wad` name search still covers them: flare names occur in
exactly the seven entries above. Caveat: an unnamed `0x390` node would not show in
that search, and the eight unnamed circuits have no node census.

Pure's EU disc: the same Vineta K node is on `pure-psp-eu.chd`
(`oag-view --nodes 'Data\Environments\01_Vineta_K\track.vex' --class 0x390`, one
node, `lensflare_`). The DLC packs are not searched.

Still frame: in `data/reference/compare-page/pure-psp/vineta/theirs.png` (480x272)
I could not pick out a flare. The maintainer's live PPSSPP observation is the
evidence for the visible effect; the still does not show it.

**Pure's shadow census contradicts this thread's Pure class mapping.** Vineta K
has four `0x375` nodes named `Shadow_Shape`, `Shadow_Shape1`, `shadow_Shape` and
`shadow_1Shape`. By the same class-name run rule that gives `lensflare` `0x390`,
`0x375` is Pure's `Dynamic Shadow Occluder` (index 9, gap 1). Pulse's `0x3c3` is
what `docs/rendering/shadows.md` searched Pure for, and shadows.md says Pure authors
none of its seven classes, which is the wrong-id search it warns against
elsewhere. The census here is node names only; the class mapping is predicted,
not confirmed.

## Executables: strings and file offsets

File offsets are into the extracted files under `data/extracted/`. Ghidra addresses
are not given here: the PSP `BOOT.BIN` base is not measured, so resolve each
offset by xref, not by arithmetic.

- **Pure** `psp/pure-usa/PSP_GAME/SYSDIR/BOOT.BIN`: `lensflare` `0x246588`
  (class name), `Lensflare_ImportNode` `0x24a2dc`, `Lensflare_Importer.h` `0x24a35c`,
  `Lensflare_Importer.cpp` `0x24a394`, `FxLensflare` `0x276ce8`,
  `c:/Work/Wipeout/Code/System/Particles/FxLensflare.cpp` `0x276cf4`.
  So Pure has a Maya importer for the class and a particle-side module. Its shader
  and texture names are not in the BOOT.
- **Pulse** `psp/pulse-usa/PSP_GAME/SYSDIR/BOOT.BIN`: `lensflare` `0x280fe0` (class
  name only). No `Lensflare_*` importer, no `LensFlare.cpp`, no `Use Lens Flare`.
- **HD/Fury** `ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf` (the ELF; its first LOAD
  maps file offset `0` to `0x10000`, so vaddr = offset + `0x10000` for these
  read-only strings): `Lighting.Use Lens Flare` `0x775f80`, `lensflare` `0x78c8c8`,
  `LensFlare.cpp` `0x790120`, `Data/Tex/LensFlare/LensflareComponents_256x256.gtf`
  `0x790130`, `..._Rays.gtf` `0x790178`, `LensFlare_vp` `0x7901a8`, `LensFlare_fp`
  `0x7901b8`, `Lensflare_Importer.cpp` `0x790230`. Already cited in
  `docs/ghidra/functions/ps3-hdfury-eu/renderer.md` (its source-file table, and the
  `Lighting.Use Lens Flare` note near line 1915, which is unread).
- **2048** `vita/PCSA00015/base/eboot.elf` and `vita/PCSF00007/base/eboot.elf`
  (both base binaries, not patch): `lensflare` `0x4ba81c` / `0x4be364`,
  `>System/Render/LensFlare.cpp` `0x4bdb83` / `0x4c16cb`, the `.gxt` components
  `0x4bdba0` / `0x4c16e8` (and `_Rays`, `Ray2`), `LensFlare_vp`/`_fp`, and
  `Lighting.Use Lens Flare` `0x4f3cbc` / `0x4f7d80`.
- **Omega** `ps4/omega-eu/uroot/eboot.bin`: `Lighting.Use Lens Flare` `0x82ecd4`,
  `Lighting.Use Lens Flare H Ray` `0x82ed0c`, `Lighting.Lens Flare H Ray Min Width`
  `0x82ed2a`, `... Max Width` `0x82ed4e`, `... H Ray Height` `0x82ed72`,
  `... H Ray Max Brightness` `0x82ed93`, `Lighting.Lens Flare Scale` `0x82edbc`,
  `Lighting.Lens Flare Sun Scale` `0x82edd6`. The horizontal ray and the sun scale
  are keys HD's string set does not show in this grep.

Strings that matched the flare search but are not this effect: `WO_ROCKET_FLARE`,
`Engine Flare`, `engineflare.vex`, `EngineFlare_Importer`, `DeformFlare`, `Sunday`,
`sundanese`, `watersun`.

## Cross-title check (CLAUDE.md rule)

- **Omega against 2048 and HD:** the key set and the module strings match HD's and
  2048's, plus Omega's extra H-ray and sun-scale keys. Omega's data also ships
  `lensflare_sun` and `lensflare_sunshape_` node names in `data02.psarc` (circuit
  not mapped). **Checked, applies, not wired.**
- **2048 against Omega and HD:** the same module and key are in the Vita eboot. Its
  asset census is not run. **Checked, applies, not wired** (the executable only).
- **HD against Omega:** the key is the same name. HD's track files carry no
  `lensflare` node by name (sixteen HD track files grepped),
  but HD's `.envsettings` flag is set on 29 circuits. So HD's flare is probably
  drawn by the module from the flag, and Omega's from named nodes too. Which HD
  class id 0x3de nodes exist, if any, is not walked. Renderer.md's HD reading ("a
  sprite effect, not the bloom chain") is unverified against the module.
- **Pulse against the rest:** absent on the disc, so there is nothing to check.
- **Pure's module sits in the particle code** (`FxLensflare`, not a `LensFlare`
  render module like HD's), so it may be a particle-style effect. Unread. Pure and
  Omega both name flare nodes in circuit data; HD's and 2048's node census is not run.

## What our renderer does today

Nothing draws the flare. `oag_vex::lensflare` reads the node and `classes::V4.lensflare` is `0x390`
(2026-10-26); `Lighting.Use Lens Flare` is still parsed and dropped (`crates/tables/src/envsettings.rs`).

## Done 2026-10-26 (`pure-lens-flare-re`)

The draw path, payloads, law, texture and class id are recovered and written up:
[`docs/rendering/lens-flare.md`](../../docs/rendering/lens-flare.md) (the law, with the
numbers) and [`docs/ghidra/functions/psp-pure-usa/lensflare.md`](../../docs/ghidra/functions/psp-pure-usa/lensflare.md)
(the functions). `lensflare` is `0x390` in V4, `oag_vex::lensflare` reads the node, and the
census is in `class_table_ground_truth.rs`. Where this page's census above and those differ,
those win (the Vineta K anchor is `(-51.826, 12.774, 23.323)`; the sun is a direction, not a point).

## Open

- **Wire the draw** (ready): a 2D additive pass after the 3D scene, the law in `lens-flare.md`. It needs the
  camera rotation, a 4 x 4 depth sample of the sun's pixel, the atlas from the circuit's `LensflareComponents.tga`.
  Render-side; the sun direction is per circuit data, no simulation state.
- **Unread, own address each:** the queue ordering of key `0x4e000000` itself (the frame position is measured: after scenery, before the post passes and HUD); whether the depth
  sample is one frame old; the random source `FUN_0882f70c` and flag `DAT_002883fc`; the sort comparator `0x08936598`.
- **The shadows.md Pure zero is in doubt** (`0x375` `Shadow_Shape` nodes on Vineta K; `docs/rendering/shadows.md`
  and `shadow_occluder_ground_truth.rs` use Pulse's ids on Pure). Not touched by this lane.
- **Other titles:** HD's `LensFlare.cpp` (`0x002aee58`), 2048 and Omega are unread; they ship a rays texture and a flag
  Pure lacks. Pure's law is not assumed for them. Pure's DLC packs and the EU disc's other circuits are not searched.

## Next Steps

1. Wire the Pure flare (`oag-wire`): load the node and atlas, draw ten additive quads per the law, behind a settings
   value with the original's look as default. Judge a frame pair against `pure-psp/vineta` as the maintainer saw it.
2. Walk `0x375` on Vineta K against Pulse's occluder payloads before trusting the shadows.md zero.
