# Every HUD asset on every disc, and where each one stands

Census, 2026-10-16 (`hud-asset-census` lane). One table per title of what its
archives ship under a HUD name, how many layouts reach each family, whether
this engine draws it, and how a player gets to it in the original. A number
with no score is a count of files; a reachability claim carries a score from
the [rubric](../reverse-engineering/confidence-rubric.md); **chosen, not
measured** carries none.

Pinned by `crates/hud/tests/hud_asset_census_ground_truth.rs` (`#[ignore]`d,
needs the images; `OAG_SWEEP=1` prints the per-file tables this page is built
from). It walks each title's archives once and asserts every number below.
The older, narrower pages stay authoritative for what they cover:
[hud.md](hud.md) (Pulse's layout schema), [hud-skins.md](hud-skins.md) (the
skin investigation and the legibility fixes), [hd-hud.md](../formats/hd-hud.md),
[2048-hud.md](../formats/2048-hud.md).

## How it was counted

- **Counting rule.** A path is lower-cased, `\` read as `/`, a leading `/` and
  a leading `data/` dropped, and counted **once however many archives carry it**
  (base, patch, DLC). Omega ships `Data/xml/2048_hud/` in the patch and
  `data/xml/2048_hud/` in the base; they are the same files. Per-archive
  counts are in the test's `OAG_SWEEP` output.
- **Where the numbers come from.** Each PSARC's own manifest, not a truncated
  listing: HD Fury 7 archives (3,157 + 5,486 + 53 + 1,163 + 54 + 151 + 1,600
  entries, 11,139 unique paths), HD PSN 4 archives (6,504 unique), Omega 9
  archives (base `data00-04`, patch `data05/07/08/09`, 27,077 unique; there is
  no `data06`), 2048 EU `data.psarc` + patch `data1/data2` + `dlc1/dlc2`
  (22,147 unique) and USA the same without `dlc2` (20,467).
- **Roots and fragments are measured, not named.** Every XML in a HUD
  directory is composed with `oag_hud::compose`. A file some other file
  `<LoadXML>`s is a **fragment**; one nothing includes is a **root** (this
  includes files that are simply never included, so it overcounts "layouts a
  mode loads"). Every composed file's quoted `.gtf/.gnf/.gxt/.mip/.fnt/.vex/
  .rcsmodel/.rcsmaterial` value is a **reference**; all resolve on every
  PSARC title except five on Omega (below).
- **Wiring states** of a non-XML asset: `Wired` (a root `Title::hud` names has
  it in its closure), `UnwiredRoot` (only roots nothing in the engine names),
  `Declared` (only a `plugins/**.xml` names it, the font slots in
  `definition.xml`), `ByModel` (a `hud/` model or material names it, a byte
  match and not a parse), `NamedByCode` (a crate spells its file name),
  `Unreferenced` (nothing found). **`Unreferenced` is "no layout, plugin XML or
  crate names it", not "unused"**: the executables can still reach a texture
  by a name built at run time.
- **The WADs store no names.** Pulse and Pure are walked from the names the
  layouts and title crates spell plus 70 probes (see below); a count is "found
  by name", never "ships none".

## Skins per title

| Title | HUD sets shipped | Played today | Offered by a setting in the original |
| --- | --- | --- | --- |
| Pulse PSP EU/USA, PS2 | 1 (4 mode roots + `MPTag_HUD`, self-contained) | 1 | none found |
| Pure PSP EU/USA | 1 (`Arcade`, `TimeTrial`, `Zone`, `MPTag`) | 1 | none found |
| HD Fury disc | 3 (bare root, `2097_hud`, `wo3_hud`) | 1 | `HUD Style`: `HD`, `2097`, `WIP3OUT` |
| HD PSN (no Fury) | 3, the same | 1 | the same list |
| Omega | 4 (bare root, `2097_hud`, `wo3_hud`, `2048_hud`) | 1 | `HUD Style` for the first three; `2048_hud` by the 2048-lineage managers |
| 2048 EU/USA | 4 (bare root, `2097_hud`, `wo3_hud`, `2048_hud`) | 1 (`2048_hud`) | none found |

## What each title ships

Counts of files per family. `xml/<dir>/font`, `/texture` and
`/splitscreen_hud` are subfolders of a skin.

| Family | HD Fury | HD PSN | Omega | 2048 EU | 2048 USA |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/roots` (`*_hud*.xml` in `xml/`, not `hud_*`) | 8 | 7 | 22 | 8 | 8 |
| `xml/fragments` (`hud_*.xml` in `xml/`) | 32 | 20 | 83 | 32 | 32 |
| `xml/2097_hud` (+ font, texture, splitscreen) | 27 + 4 + 2 + 23 | 23 + 6 + 2 + 18 | 27 + 11 + 3 + 23 | 27 + 6 + 2 + 23 | same as EU |
| `xml/wo3_hud` (same subfolders) | 27 + 4 + 2 + 23 | 23 + 6 + 2 + 18 | 27 + 8 + 3 + 23 | 27 + 6 + 2 + 23 | same as EU |
| `xml/2048_hud` (+ font, texture) | - | - | 119 + 7 + 8 | 56 + 4 + 5 | same as EU |
| `xml/splitscreen_hud` | 31 | 18 | 31 | 31 | 31 |
| `xml/splitscreenzone_hud` | 10 | 16 | 16 | 16 | 16 |
| `xml/duel_hud` (+ `duel_splitscreen_hud`) | 5 + 5 | - | 15 + 5 | 5 + 5 | 5 + 5 |
| `hud/textures` | 44 | 32 | 45 | 42 | 42 |
| `hud/materials` | 4 | - | 31 | 6 | 6 |
| `hud/models` (`.vex`, `.rcsmodel`, `.ma.xml`) | 20 | 8 | 24 | 44 | 44 |
| `hud/debug` | - | - | - | 3 | 3 |
| `fe/fonts` named `*hud*` or `small*` (`.fnt` + atlas) | 12 | 12 | 16 | 6 | 6 |

Notes that change a reading:

- **HD PSN ships more split-screen Zone files than the Fury disc** (16 against
  10: six `vert` twins, `zone_hud{,1,_2097,_2097_1,_wo3,_wo3_1}vert`)
  and fewer split-screen Eliminator ones (no `splitscreen_elimination_hud_*`:
  Eliminator is Fury's mode). Omega's 16 match the PSN's.
- **Omega's `2048_hud` set is the 2048 set plus** `_hd`, `_vr`, `_vr_ext` and
  `_splitscreen_h/_v` layouts for every mode (119 files against 2048's 56) and
  `.2x.gnf` atlases. 2048 ships only `_hd` variants.
- **Omega's 24 `hud/models` are the sights, `emp_hud`, `pulse_ready_go` and 13
  VR files** (`vr_hud*`, `vr_2048`: six `.vex`, six `.rcsmodel`, one `.ma.xml`);
  it ships none of the 20 nitro-bar and zone-battle pieces 2048's 44 models
  carry (30 of which no layout, plugin or crate names, 4 named only by code),
  though its 31 `hud/materials` are there.
- **Five dangling references on Omega**: HD-lineage layouts name
  `hud/zone_battle_hud_{bars_full,bars_full_rays,boost,boost_rays,hex_full}.vex`,
  which Omega does not ship (HD, 2048 do). Every other reference on every
  PSARC title resolves.

## Pulse and Pure: found by name

The census asks each WAD whether an entry hashes to a name: the layouts' own
references, the title's front-end composition, and 70 probes (the three HUD
fonts, 33 `Data\HUD\Textures\*.mip` stems and 25 `Data\HUD\*.vex` stems HD, Omega
and 2048 ship, plus nine layout names such as `Detonator_HUD.xml`, `2097_hud\`,
`2048_hud\` and `SplitScreen_hud\`).

| Source | Entries (data + FE/SP) | Layouts composed | References resolved | Probe hits of 70 | Front-end files read, none naming a HUD selector |
| --- | --- | ---: | ---: | ---: | ---: |
| Pulse PSP EU | 1,138 + 27 | 5 | 6 of 6 | 25 | 19 |
| Pulse PSP USA | 1,142 + 27 | 5 | 6 of 6 | 25 | 19 |
| Pulse PS2 EU | 7,200 + 193 | 5 | 6 of 6 | 44 | 14 |
| Pure PSP EU | 841 + 158 | 4 | 20 of 20 | 12 | 11 |
| Pure PSP USA | 832 + 157 | 4 | 20 of 20 | 12 | 11 |

- **Entries the census did not name stay unnamed** (the probes name 25 of
  Pulse PSP's 1,165). That a probe hit nothing says only that the name is not
  there.
- **Pulse PSP's 25 hits**: the three fonts, `PulseHUD.mip`, and 21 `Data\HUD\*.vex`
  models (the 26 polygonal models [hud.md](hud.md) lists as unreferenced; four
  of the 25 probed `.vex` stems (`emp_hud`, three `zone_battle_hud_*`) are HD names Pulse does not have). The PS2
  adds 19 hit `.pct` textures (`1st`-`8th`, `speed`, `grid`, `ghostship`,
  `barcode`, `memstick`, `hud_bars`, `gunsight_add`, `gunsightdot_add`,
  `ready_go_nomip`, `pulse_ready_go_nomip`, `zone_outline_01`).
- **Pure's 12**: `small.fnt` and 11 `Data\HUD\*.vex` (sights, `Bar_*`, `Zone_*`,
  `grid`, `Ready_GO`); no Pure HUD font by Pulse's name.
- **The 11 PSP DLC zips** unpack to 24 WAD archives (Pulse 4 packs x 4, Pure 8
  packs, the Omega pack in two directories); **none answers any of the 70
  probes**, so no PSP DLC pack ships a HUD file under these names. The three
  2048 DLC `.pkg` are the extracted `dlc1`/`dlc2` mounted above; they add no
  HUD family beyond the base.

## Wiring and how a player reaches it

"Wired" is what `Title::hud` and the title's `hud` module name and `oag-game`
draws. A **reach** score is confidence that the original lets a player get
there, from the cited evidence.

### Pulse (PSP, PS2) and Pure

| Asset | Wiring | How a player reaches it |
| --- | --- | --- |
| `Arcade`, `Elimination` (Pulse), `TimeTrial`, `Zone` roots, `PulseHUD.mip`, `PulseHud.fnt`, `small.fnt` | wired | default, every mode (Time Trial's file also serves Speed Lap: this disc ships no `SpeedLap_HUD.xml`, [hud.md](hud.md)) |
| `MPTag_HUD.xml` | named in `oag_pulse::hud::layouts::MP_TAG`, not drawn | multiplayer tag; this engine has no tag mode |
| `Data\HUD\Bar_1`/`Bar_3`/`Bar_outline_1` and `Zone_Bar_1`/`Zone_Bar_3`/`Zone_outline_1` (Pure's speed and shield gauge) | wired on Pure (2026-10-22): the layouts name them as `Speed3D`/`Shield3D`/`Overlay3D`, [pure-status.md](../formats/pure-status.md#the-speed-gauge-is-three-models-2026-10-22); Pulse's layouts do not | every Pure race |
| the other 20 `Data\HUD\*.vex` polygonal models (`Speed`, `Shield`, `Lap`, ...) | unwired, no layout names them ([hud.md](hud.md)) | not determined; no `BOOT.BIN` string (confidence 70 that nothing reads them by name) |
| PS2-only HUD `.pct` textures (19) | unwired | not determined |
| Pilot Assist indicator | own layer: `assets/fallback/Data/XML/HUD_assist_indicator.xml` + `AssistIndicator.pix` (our art, chosen, not measured) | with the Pilot Assist option on |
| Split-screen layout | own layer: `SplitHud_Pulse.toml` / `SplitHud_Pure.toml` (these discs author none) | two-view race |
| Alternate skin | none found: no HUD selector in 19/14/11 composed front-end files, and no skin directory among the probes | never (confidence 60: the front end was walked through its composed files, not every plugin) |

### HD / Fury (disc and PSN)

| Asset | Wiring | How a player reaches it |
| --- | --- | --- |
| Default root skin: `arcade`, `elimination`, `timetrial`, `speedlap`, `zone` (+ 32 fragments) | wired (`oag_hd::hud::LAYOUTS`) | default, `HUD Style` = `HD` (list default). `fury_hud` is not a second skin: it is the atlas the Eliminator, Detonator and Duel layouts name (31 root closures, 5 of them wired), `hdhud` the rest |
| `splitscreen_hud/splitscreen_hud_{0,1}`, `splitscreen_elimination_hud_{0,1}` and the four `vert_` twins | wired (`oag_hd::hud::SPLIT`) | two-player split, Single Race and Eliminator; PSN lacks the Eliminator pair |
| `2097_hud` (27 files + 4 font + 2 texture + 23 split), `wo3_hud` (same) | **wired** (2026-10-16): `oag_hd::hud::SKINS`, picked by `display.hud_style` / `--hud-style` ([hud-skins.md](hud-skins.md), part E); the Eliminator keeps the default root (the executable names no skinned one) | `HUD Style` list in `plugins/frontend/gui/additional_definition.xml` (entries `HD`, `2097`, `WIP3OUT`; read today on Fury **and** PSN); `PI_HUD` rows in `definition.xml` with `Location="Data\XML\2097_hud"` / `wo3_hud`, `<Unlock locked="true" team="any" loyalty=6000>` and `10000`; reach **85**, unlock threshold **85** (read off the data); the executable's race managers branch on the strings, [race-hud.md](../ghidra/functions/ps3-hdfury-eu/race-hud.md) |
| `HUD Style P2` (split-screen player 2's own skin) | unwired (the skin is chosen for both views) | named by the executable ([hud-skins.md](hud-skins.md)); confidence 70 |
| `splitscreenzone_hud` (10 on the disc, 16 on PSN): `zone_hud`, `zone_hud1`, `zone_hud_2097[_1]`, `zone_hud_wo3[_1]`, `vert` twins | unwired: Zone is not offered in our split | two-player Zone; the `_2097`/`_wo3` files say the Zone split follows `HUD Style` (confidence 60, from names) |
| `detonator_hud` | named in `oag_hd::hud::layouts::DETONATOR`, not drawn | Detonator mode (Fury); this engine has no such mode |
| `duel_hud/*` (5 + 5 split) | named, not drawn | Duel mode; no such mode here |
| `mptag_hud`, `arcade_hud_old`, `hud_timers`, `splitscreen_sights` | unwired, no includer; `arcade_hud_old`/`hud_timers` are Pulse's layout at 4x | never found referenced (confidence 70 data, 40 that an engine could draw them) |
| Sights `missile_sight_{inner,outer}`, `leachbeam_sight`, `pulse_ready_go` `.vex` | wired via `HudArt` | default |
| `emp_hud`, `zone_battle_hud_*` models (6 `.vex`) | referenced by unwired roots only | via the unwired roots above |
| 25 `hud/textures` and 4 `hud/materials` nothing names (`1st`-`8th`, `bar_outline_01_nomip`, `barcode`, `ghostship`, ...); 8 more textures only a `hud/` model names, 2 only a crate | not determined | not determined |

### Omega

| Asset | Wiring | How a player reaches it |
| --- | --- | --- |
| Bare root skin (HD's five roots at HD's paths, 83 fragments incl. `_vr`/`_vr_ext`) | wired (`oag_omega::hud::LAYOUTS`, `.gnf`) | default: `HUD Style` default is `HD` (read today in Omega's `additional_definition.xml`) and the arcade manager takes the bare root for anything but `WIP3OUT`/`2097` ([race-hud-selection.md](../ghidra/functions/ps4-omega-eu/race-hud-selection.md)); reach 80 |
| `splitscreen_hud` (4 two-view + 4 `vert_` roots) | wired (`oag_omega::hud::SPLIT`) | two-player split |
| `2097_hud`, `wo3_hud` | **wired** (2026-10-16, `oag_omega::hud::SKINS`, all five modes and every split pair) | `HUD Style` `2097`/`WIP3OUT`, same `PI_HUD` rows and loyalty 6000 / 10000 as HD (`definition.xml` in base, patch `data08`/`data09`); reach 85 |
| `2048_hud` (119 files: 43 with no includer; `_hd`, `_splitscreen_h/_v`, `_vr`, `_vr_ext`) | **unwired** | by 2048-lineage race managers reached through virtual dispatch (`FUN_015ae4d0`, `FUN_015ac880`, `FUN_015bc700`); which race picks it is **not determined**, address above |
| `2048_hud/*_splitscreen_h/_v` | unwired | the 2048-lineage managers' split layouts, same open question |
| `*_vr`, `*_vr_ext`, `hud/vr_hud*` models, `duel_hud_*_vr*` | unwired | PSVR only; VR is not planned, so **never reachable** here |
| `HUD Effects` setting (list `OPT_HUD_EFFECTS`, default `FE_ON`) | unwired | in Omega's options (not HD's); what it switches is **not determined** |
| `detonator_hud`, `duel_hud`, `mptag_hud`, `splitscreenzone_hud` (16) | named in no code, not drawn | modes this engine lacks / Zone split not offered |
| `arcade_hud_old`, `hud_timers` | unreferenced | as on HD |

### 2048 (Vita)

| Asset | Wiring | How a player reaches it |
| --- | --- | --- |
| `2048_hud` (5 roots wired: `arcade`, `elimination`, `speedlap`, `speedlap_timetrial`, `zone`; + `zombie`, `mptag`, `checkpoint`, `detonator` ...) | five wired (`oag_2048::hud::LAYOUTS`), the rest named in `ROOTS` | default: the race managers hard-code `2048_hud\` ([2048-hud.md](../formats/2048-hud.md)) |
| Bare root (8 files) | wired only for the attract demo (`skins::demo`) | the demo manager |
| `2097_hud`, `wo3_hud` (58 files each across subfolders) | unwired | **no way in found**: no `HUD Style` list, no `PI_HUD` rows (`PI_HUD` appears in 0 plugin files, against 1 on HD/Omega), the executable holds no `wo3_hud`/`2097_hud` string; stats strings `STATS_WON_*_HUD` say a choice once existed. Treat as unreachable (confidence 70) |
| `splitscreen_hud` (31), `splitscreenzone_hud` (16), `duel_hud` (5 + 5) | unwired: `split: None`, own `SplitHud_2048.toml` is drawn instead | two-player split / Duel in the original; confidence 60 |
| `hud/models` (44), `hud/materials` (6) | unwired | `nitro_bars_*`, `zone_battle_hud_*`, VR panels; not determined |
| `hud/debug/test_hud{1,2,3}.gxt` | never | debug textures, nothing names them |

## Does the other title ship it? (2048 / Omega rule)

| Family | Finding |
| --- | --- |
| Default / bare root skin | **ported**: Omega carries HD's roots at HD's paths and draws them; 2048 ships the bare root and plays it only as the demo. |
| `2097_hud`, `wo3_hud` | **ported** on Omega (`oag_omega::hud::SKINS`, own ground-truth test `hud_style_ground_truth`); **checked, differs** on 2048 (files present, no selector or unlock rows). |
| `2048_hud` | 2048 plays it; **checked, applies, not wired** on Omega (119 files, `_vr`/`_split_h/_v` extras, selector in the executable not read). |
| `splitscreen_hud` | HD **wired**, Omega **ported**; **checked, applies, not wired** on 2048 (31 authored files, 2048 draws our own TOML). |
| `splitscreenzone_hud`, `duel_hud` | **checked, applies, not wired** on HD, Omega and 2048 (no Zone-split or Duel mode here). |
| `HUD Effects` | Omega only (HD, 2048: **checked, differs**: no such list). |
| `HUD Style` list | HD and Omega ship it (default `HD`); 2048 **checked, differs**. |
| VR layouts | Omega only; not checkable on the others (they ship none) and not planned. |
| Pulse's 4x atlas | HD `pulsehud.gtf`, Omega `.gnf` and 2048 `PulseHUD.gxt` are named only by `arcade_hud_old`/`hud_timers`, the two never-included roots on all three. |

## Wiring gaps, as work

Reachable in the original, unwired here, in the order a player would notice:

1. ~~`HUD Style`: `2097` and `WIP3OUT` on HD (disc and PSN) and Omega~~: landed
   2026-10-16 ([hud-skins.md](hud-skins.md), part E). Still open: the `HUD Style P2`
   slot and the loyalty unlock gate (both skins are offered unlocked).
2. **Omega's `2048_hud` set**: the selector (which races) is unread.
3. **Split-screen Zone** (`splitscreenzone_hud`) and Omega's `2048_hud`
   split-h/-v layouts: waiting on a Zone split and on item 2.
4. **2048's authored `splitscreen_hud`** instead of the own TOML.
5. **Omega's `HUD Effects`**: read what it switches.

Not reachable as a player feature: VR layouts, `hud/debug`, `arcade_hud_old`
and `hud_timers`, and the Pulse/Pure/2048 polygonal HUD models until something
names them.

## Corrections to older pages

- [hud-skins.md](hud-skins.md) says the `HUD Style` list and `PI_HUD` rows ship
  "only in the Fury layer, not the original `DATA02`". The list is in
  `additional_definition.xml` of `data03` and the rows in `definition.xml`, and
  **the plain PSN download (no Fury) carries both** (census 2026-10-16, read
  across every copy of each file). Its skin counts (56/72/58 files) were
  archive entries; the table above counts unique paths per family.
