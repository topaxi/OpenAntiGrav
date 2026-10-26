# A sun lens flare on Pure, and which titles carry it

2026-10-26, `lens-flare-scout` (read-only census, no code changed). The maintainer
watched a clear sun lens flare on Wipeout Pure's Vineta K under PPSSPP. Nothing in
our code draws one on any title: `crates/` names the class only in
`crates/vex/src/vex/class_names.rs` (HD's table, `0x3de`), and the envsettings
parser reads `Lighting.Use Lens Flare` as a flag nobody reads. Not to be confused
with the engine or exhaust flare (`Engine Flare` locator, `WO_ROCKET_FLARE`) or
HD's craft sprite flare (`engineflare.vex`).

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

Nothing. No code reads class `0x390` (`crates/vex`, `crates/raceplay`, `crates/render`)
and `Lighting.Use Lens Flare` is parsed and dropped (`crates/tables/src/envsettings.rs`,
tests at `crates/tables/src/envsettings/tests.rs`). The viewer prints the class as
"unnamed - not in the read table". I did not find a loader report that counts
unread classes, so whether one exists was not checked.

## Open

- **The Pure flare's drawing law is unread.** Nothing in the data gives the sprite
  geometry, the ray shape, the size, the brightness, or the occlusion or fade rule.
  The data gives the anchor (`lensflare_` payload) and the texture name only.
- **The Pure class id is not in `V4`** (`crates/vex/src/vex/classes.rs`). Adding
  `lensflare` at `0x390` is ready to wire, and it needs no RE to name the node.
- **The shadows.md Pure zero is in doubt** (see "Pure's shadow census contradicts"
  above). `docs/rendering/shadows.md` (line 100, "Wipeout Pure authors none of the
  seven") and `crates/vex/tests/shadow_occluder_ground_truth.rs` (`CLASS_OCCLUDER`
  is Pulse's `0x3c3`, line 73) both use Pulse's ids on Pure. Not edited here. The
  wiring or census lane must walk Pure's `0x375` nodes on Vineta K and compare the
  names with Pulse's occluder payloads before it trusts the zero.
- **Pure's eight unwalked circuits and the DLC packs are not searched by path.** A
  node with no name would not show in the name search.
- **HD's class `0x3de` nodes are not walked** (HD's track files are PSARC entries;
  the name search found no `lensflare` in them). **2048's and Omega's node censuses
  are not run.** Omega's `data02.psarc` names `lensflare_sun` and
  `lensflare_sunshape_` (circuit not mapped). The extracted Vita trees hold the
  eboot and no flare-named data file; they may not hold every data file.
- **The Pulse absence is checked by name** (whole `Data.wad`, uncompressed, 1,142
  entries, zero `lensflare` hits) **and by the earlier class census**
  (`shadows.md`, `0x3de` on 382 v6 files). Both use Pulse's own ids, which is sound
  for Pulse.

## Next Steps

1. **RE-first, Pure's draw path.** Read the string xrefs of `FxLensflare.cpp`
   (Pure `BOOT.BIN` file offset `0x276cf4`) and `Lensflare_Importer.cpp`
   (`0x24a394`) to the function that consumes the `lensflare_` node. Name that
   function, then read what it draws. A new Ghidra page under
   `docs/ghidra/functions/psp-pure-usa/` is owed. Ghidra was not touched by this
   scout.
2. **RE-first, HD's law** (it is the readable one): start at `LensFlare.cpp`'s first
   function, `0x002aee58`, the address `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`
   (line 113) records for that file. Then the `LensFlare_vp`/`LensFlare_fp` shaders
   (HD EBOOT string xrefs at vaddr `0x7a01a8` / `0x7a01b8`, file offsets
   `0x7901a8` / `0x7901b8`) and the reader of `Lighting.Use Lens Flare` (string at
   vaddr `0x785f80`, file offset `0x775f80`). Use it as the reference for Pure's
   sprite shape only after the Pure read agrees with it. The string addresses are
   for xrefs, not for direct reads.
3. **Ready-to-wire, now:** add `lensflare` `0x390` to Pure's V4 class table, and a
   census test over Vineta K's two nodes (`crates/pure/tests/`), so the anchor is
   read and named. No draw yet.
