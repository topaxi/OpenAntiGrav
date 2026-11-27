# Wipeout: Omega Collection status

What `oag-omega` and the rest of this project's tooling read off the PS4
package pair unchanged, what changed, and what stays unread. Measured against
`data/extracted/ps4/omega-eu` (base) and `omega-eu-patch` (mandatory day-one
patch) - see [`source-images.md`](../reverse-engineering/source-images.md#omega-ps4-eupkg--omega-ps4-eu-patchpkg---wipeout-omega-collection-ps4).

**The short version.** Omega's front end is [HD's own `PI001` plugin carried
forward](omega-frontend.md), not a rewrite, and everything that follows from
that reads with **no parser change at all**: `skin.xml`'s `FEGlobals`, the
boot chain, `mainmenu_definition.xml`'s `<HorizMenu>`, `CellMode_Definition.xml`'s
`Grid Selection`/`Cell Selection` schema. What is genuinely new is the
*plumbing* - `oag-omega` as a title package, a nine-archive candidate list
where the patch's own front-end-complete archive has to win a name collision
with the base package's older, incomplete copies - and one thing that is
**not** carried forward unchanged: the circuit files ship every one of HD's
files under new spellings (`track.final.rcsmodel`, a 19-byte collision node, a
64-bit `.rcsmodel`) that this project's 2048 readers turn out to read after
three small changes - see "Racing" below.
Front-end images are Sony's PS4 `.gnf`
container, which [`gnf.md`](gnf.md) now decodes for most of the sprite sheet
(219 of 289 front-end/campaign `.gnf` files draw). **A race starts** - see
"Racing" below - and [`omega-frontend.md`](omega-frontend.md) is the frontend
census this page builds on.

## What reads unchanged

| Layer | Reads? | Notes |
| --- | --- | --- |
| [PSARC](psarc.md) container | **yes** | Nine archives across the base package and the patch; version and flags not diffed against HD's own PS3 copies this lane. |
| Front-end XML (`Screens::from_xml`) | **yes** | `skin.xml`, `mainmenu_definition.xml`, `cellmode_definition.xml`, `additional_definition.xml` all parse with the same `oag_ui::screen::Screens` reader HD and Pulse/Pure use - confirmed both by a docs-only sweep and by this lane's own boot (`Data\Plugins\Frontend\Gui\Skin.xml: 27 screens, 74 globals, 24 LoadXML includes`). |
| `FEGlobals` menu layout | **yes, bit-identical to HD's** | Ten of ten authored globals match HD's own `skin.xml` to the digit, re-derived directly against `data09.psarc` in `crates/omega/src/frontend.rs` rather than copied - see that module and its ground-truth test. |
| Boot chain shape | **declared, not measured** | Same eight redirects in the same order as HD's `DATA00`/`DATA05`/`DATA06` family, no dead `LogoFMV`. `Provenance::Declared`: no capture of Omega's boot exists to upgrade it the way HD's RPCS3 capture did (shadPS4 runs the game since 2026-10-22, `scripts/shadps4-omega-pair.py`, but it walks a fresh profile's menus to the start grid and does not record the boot chain). |
| Campaign schema (`CellMode_Definition.xml`) | **yes, HD's own schema** | `Grid Selection`/`Cell Selection` as `FlyerSelection`/`CellSelection` screens, plain UTF-8 (not Pulse's dictionary-shortened copy). Nineteen grids where HD has sixteen (`oag_omega::campaign::GRID_COUNT`), confirmed by direct listing. Sixteen of nineteen parse (2026-09-30; twelve when this row was written) - grids 16-18 carry no `RequiredPoints` attribute and are skipped with a logged warning; the rest fail per-row the same tolerant way a bad HD grid already does (`docs/formats/hd-frontend.md`'s note on `grid_04.xml`'s own broken tag). |
| Plugin definitions | **yes, but split three ways, like 2048's** | `Data\Plugins\teams\`, `tracks\` and `music\` each ship their own `Definition.xml`, not HD's single `Data\Plugins\Frontend\Definition.xml`. New finding, not in `omega-frontend.md`, which only opened the GUI files. |

## What changed, and what state each is in

**The archive layout is new.** Omega ships as a base `.pkg` (five archives,
`data00`-`data04`) plus a mandatory day-one patch (four more, `data05`,
`data07`, `data08`, `data09`) - and the patch **replaces** the front end
wholesale rather than supplementing it: `skin.xml`, `mainmenu_definition.xml`,
`racebox_definition.xml` and `cellmode_definition.xml` exist in three nested
layers, the base `data00.psarc` (an older build; `skin.xml` 30,235 bytes) and the
patch's `data08.psarc` (35,828) and `data09.psarc` (41,150, the newest), not only in
`data09` as this page said until 2026-10-13 (re-read with the digest-matching
reader, [`omega-frontend.md`](omega-frontend.md#the-census-front-end-content-is-in-four-of-nine-archives)).
`oag_omega::TITLE` makes `data09` the bulk (`data`)
candidate specifically so it is searched before the other eight - chosen to
win the collision, not measured to be what a PS4 actually loads first. See
`crates/omega/src/lib.rs`'s own `DATA_CANDIDATES` doc and
[`omega-frontend.md`](omega-frontend.md#load-order-is-not-settled).
Omega's `racebox_definition.xml` carries HD's two local-split lists (`NumberOfPlayers`,
`SplitScreen`). They were read from 2026-10-13 as the RACE page's `NUMBER OF PLAYERS` and `SPLIT
SCREEN` rows; **since 2026-10-14 (`mp-menu-flow`) nothing reads them**: split screen is the
MULTIPLAYER row's LOCAL MULTIPLAYER flow on every title, and the head count is the join screen's
(**ported** with HD: the same extras row, and the join line `FE_GUEST_JOIN`, which on Omega says
"PLAYER 2 PRESS OPTIONS TO JOIN"). Its `Single Player` page is not walked.

**The ship directory moved to 2048's scheme, not HD's.** `Data\art\published\hdships\<team>\`,
lowercase ids (`ag_systems` confirmed present), not HD's `Data\Ships\<Team>\`.
Confirmed by direct listing on `data00.psarc`; `crates/omega/src/race.rs`'s
own `SHIP_DIR` constant records it. Sound banks moved too:
`Data\audio\sound\<name>.bnk`, not HD's `Data\Sound\<name>.bnk` - `shipHD.bnk`,
`weapons.bnk` and `speech.bnk` are confirmed present under the new path.

**Front-end images are `.gnf`, a PS4 texture container `gnf.md` now decodes
for most of the sprite sheet.** The XML still names `.gtf` (HD's spelling,
never re-authored), and the shipped file is `.gnf` under the same stem -
`saveIcons.gnf`, `line.gnf`, `Title_Arrow_HD.gnf` all confirmed present this
way. `crates/game/src/boot/sprites.rs::gnf_sibling` checks the `.gnf`
sibling directly: when it decodes, its bytes are pushed into the sheet under
the widget's own `.gtf`-spelled key (`crates/hud/src/sprite.rs::Image::decode_gnf`
is what actually rasterises them); when it does not, the boot report names
the exact reason rather than a bare "not found" (`image
Data\FE\Images\saveIcons.gtf: found as Data\FE\Images\saveIcons.gnf (158976
bytes) - PS4 GNF container, 342 block(s) in the base level have no valid BC7
mode bit - refusing rather than decoding around missing bytes, see
docs/formats/gnf.md, drawing nothing`). On this lane's own two-archive
extraction, the front end's own **boot-time** image set is small (8 names)
and mostly either genuinely absent from these archives or corrupt -
`Title_Arrow_HD.gtf` (32x32) is the one that decodes and draws, visible as
the nav-legend arrow in `just play omega --menu-page main`'s own screenshot.
The front end's full sprite sheet fares far better: `gnf_frontend_census`
(`crates/texture/examples/gnf_frontend_census.rs`) measures 219 of 289
front-end/campaign `.gnf` files decoding clean, 53 refused for a corrupt
base level, 17 in a format this module does not decode at all (`Bc4`, font
surfaces) - see `gnf.md`'s own "Tiling" section for the evidence that closed
the tiling question and the `CorruptBlocks` refusal's own reasoning. What
still draws with no images at boot specifically is a data-availability fact
about this two-archive extraction (most boot-time names are absent or
corrupt here), not a decoder gap.

**Fonts are unread too, on the same terms.** `Data\FE\Fonts\helv.fnt`/
`helvb.fnt` (HD's own spellings) do not resolve on Omega under either
name checked this lane; whether Omega ships a font under a different name or
a different container was not searched. **Superseded 2026-10-13**: on the corrected
extraction both ship in `data00.psarc` (`Data/fe/fonts/helv.fnt` 530,304 bytes,
`helvb.fnt` 1,054,592, with `.gnf` atlases) and in `data08.psarc`, and the font
sections below read them; the "do not resolve" came from the short-read copy.

**Circuits: every one ships a `track.vex` with a readable `WO Track`, and its
baked outputs under a `.final` infix.** Across the whole package pair, **39 of
39** `track.vex`/`track_reversed.vex` files carry a `WO Track` node that
`oag_vex::track::parse` reads unmodified with `encoded_len()` equal to the
payload's length to the byte (`tech_de_ra`: 2 paths, 2 junctions, 835 points;
`environments2048\altima`: 6 paths, 4 junctions, 2,029 points, 195,104 bytes,
the same numbers 2048's own file has, at the same 96-byte point
[`track.md`](track.md) measured there). Beside each sit `track.final.rcsmodel`,
`track.final.rcsskeleton`, `track.final.rcsanimclip`, `track.final.pvs`,
`track.final.audio`, `track.final.checkpoint` and `track_col.col`; the four
`zone_N` circuits use the plain `track.rcsmodel`-style spelling instead, and
**no directory ships both**. This page's earlier revision said the opposite
(`talons_junction` "ships `.final.*` in place of a `track.vex`", `tech_de_ra`'s
`track.vex` "confirmed to have no readable `WO Track` node - a clean negative
result") and **both claims were the extraction bug**, below: on the short-read
copy `tech_de_ra\track.vex` has 278 nodes and no `WO Track`, on the corrected
one 458 nodes and one (60,151 of its 133,888 bytes differ, 110,298 of them
zero against 50,172).

**The in-race HUD is HD's, and it draws (`omega-hud`, 2026-10-06).** An earlier
revision of this page said Omega ships no per-mode composition. **That was a
truncated census, not a finding**: `scripts/psarc.py` split a PS4 manifest on
`\n`, and a PS4 `.psarc` separates paths with NUL, so the whole listing came back
as one entry and a search for `arcade_hud.xml` found nothing. Re-counted through
the engine's own reader: 46,005 entries over the nine archives (`data00` 10,926,
`data01` 4,641, `data02` 5,611, `data03` 661, `data04` 4,569, `data05` 8,205,
`data07` 6, `data08` 11,266, `data09` 120).

- **Five roots at HD's own paths** in `data00`: `Data\XML\Arcade_HUD.xml`,
  `Elimination_HUD.xml`, `TimeTrial_HUD.xml`, `SpeedLap_HUD.xml`, `Zone_HUD.xml`,
  composing to 17/15/14/15/5 files (HD's counts) with nothing missing and nothing
  skipped, and 148/132/52/51/36 sprites (HD's Arcade is 138: Omega's XML is its
  own). `crates/omega/src/hud.rs` names them; the executable names them too, from
  the single-player race managers (`docs/ghidra/functions/ps4-omega-eu/race-hud-selection.md`,
  measured), the bare root being the default of its "HUD Style" check.
- **Three more skins and a fourth set**: `wo3_hud\`, `2097_hud\` (the "HUD Style"
  values), split-screen families, `duel_hud\`, and a **`2048_hud\` set** (the
  Vita's 26 file names plus `_HD` variants and `_VR` ones in `data08`) reached only
  through virtual dispatch. It composes whole on Omega and every atlas it names
  resolves (`the_2048_hud_set_omega_also_ships_composes_whole`). **A race on a
  `Data\Environments2048\` circuit draws it** (2026-11-12, `omega-2048-hud-laps`,
  `RaceDefaults::circuit_lineages`, [below](#2048-mode-hud-and-laps-2026-11-12-omega-2048-hud-laps)); the
  executable's selector (the Race Box's GAME MODE) is not read, the circuit's directory stands in.
- **Nine atlases, all `.gnf`, all BC7**, under `Data\HUD\Textures\` (and
  `Data\FE\Images\voiceCom.gnf`). The XML names them `.gtf`/`.mip` (HD's spelling),
  so the PS4 spelling (`oag_assets::texture_name`) is `.gnf` - none resolves under its authored
  name. `fury_hud` is 2048 square on Omega against HD's 1024 and its layout UVs
  reach 2010 against HD's 1005: the XML is authored for the atlas it ships.
  The rest are HD's sizes.
- **Row order: top-down, no reversal.** Unlike the eight front-end `.gnf` of
  `FrontEnd::bottom_up_gnf`, a HUD atlas decoded straight out of
  `gnf::Texture::decode` equals HD's `.gtf` as `oag_hud::sprite` draws it
  (reversed from the file): mean absolute difference 0.33 (`HUD_Components`),
  0.70 (`_01`), 0.19 (`missile_reticule`), 0.33 (`nitro_hud`) against 9-91 as
  filed. `hdHUD` is 17.06 against 80.59: the same order, art partly redrawn. So
  no `HudArt` field for row order was needed.
- **Base versus patch**: `data00` and `data08` both ship every atlas (the patch
  copy with a mip chain); the base level is identical for seven of eight and
  `HUD_Components_01` was repainted by the patch. `Archives` serves the patch
  copy (`the_patch_and_base_copies_of_each_atlas_agree_at_the_base_level`).
- **Text is laid out at half its texels.** Omega's `PulseHud.fnt` and `small.fnt`
  are HD's at exactly twice the size (line height 184/68 against 92/34, advance
  median 2.0) under layouts authoring HD's numbers; drawn unscaled the lap time
  was twice HD's size. `hud_font` now applies `Space::font_texel_scale`, the
  scale the front end already uses (confidence 60, an inference, see that
  function). 
- **Chosen, not measured**: `always_on`, the lock-on sight names, `RUNTIME`
  (shield bar colours, arcs), `shield_percent`, `message_slots` are HD's
  readings; every name they carry is authored by Omega's layouts
  (`the_widget_names_title_data_inherits_are_authored_by_omegas_layouts`), which
  proves the names match, not the values. `zone_speed_classes` is Omega's own table off its
  executable ([zone-speed-class-ladder.md](../ghidra/functions/ps4-omega-eu/zone-speed-class-ladder.md)): an
  HD-lineage Zone race draws `FLASH` beside zone 5 and `SUB-RAPIER` on the next-class row, as the original's frame does.

Frames, 1920x1080, `--race --ticks 700/900/1500` on `data/extracted/ps4`:
time trial, two moments: the timers move; Arcade; zone;
compared with HD's `hud-hd.png` and 2048's `hud-2048.png` in the same directory.
Omega's time trial is HD's frame element for element; 2048 draws its own
skin.

**Cross-check against 2048: checked, differs.** 2048's race reads its
`2048_hud` skin off `.gxt` atlases; Omega's played skin is the bare root HD
set off `.gnf`. Omega also ships the `2048_hud` XML and it composes; wired per circuit
lineage since 2026-11-12 (below).

## 2048 mode: HUD and laps (2026-11-12, `omega-2048-hud-laps`)

**A race on a `Data\Environments2048\` circuit draws 2048's HUD and runs the original's lap counts.**
`oag_omega::race::LINEAGES` (`oag_title::race::CircuitLineage`, read through
`RaceDefaults::circuit_lineage`, the same key `effect_dir_by_circuit` uses) carries, per circuit family: the
layouts (`oag_omega::hud::LAYOUTS_2048`, `Data\XML\2048_hud\*_HUD.xml`), the art (`ART_2048`: 2048's
dialect over Omega's own readings), the layout grid and the laps. `oag_raceplay::hud::hud_roots` takes the
lineage ahead of any `HUD Style` skin and the add-on campaign path; `oag_raceplay::catalogue::lineage_laps`
resolves the count into `Setup::laps_override` at load, so a replay (which records the options) rebuilds
with the same count.

- **Grid.** Omega's copy of the set is authored for the Vita's 960x544, not for 1920x1080 (the reticle
  placeholder is still `(-480, 272)`, atlases `.gxt` spelling); the original draws it scaled to the screen
  (`LAP` at about x=30 at 1080p, 2x the Vita's `(15, 10)`). The lineage names the grid as
  `Platform::Vita` and `load_hud` takes `Space::of` it. Omega's `2048HUD` font role resolves and draws at
  the right size with no extra scale (the glyph run `LAP` is 90 px wide in the original's frame and ours).
- **Laps.** The Race Box authors none (`racebox.xml`: `m_numOfLaps` is `0` on all nine `* 2048` entries, so
  the count is the executable's). Measured on the original (the `wipeout_pc` port): `LAP 1/4` at C CLASS on
  seven circuits, `LAP 1/5` at A CLASS on Sol (confidence 70, `gallery-omega-2048`), `LAP 1/4` at B CLASS on
  Altima (one boot, confidence 55: the pair was refused on its name-crop check, which fired during the walk
  and not on the HUD; the Race Box frame shows B CLASS selected). **A+ CLASS is unmeasured**: the one boot
  stalled before the race, its Race Box frame reads C CLASS, so the walk's class presses had been lost.
  The table is `[4, 4, 5, 5]`, A+ **chosen** equal to A. Single race only; the Time Trial and Speed Lap
  counts of 2048 mode are not measured. The class mapping C, B, A, A+ to the ladder's venom, flash, rapier,
  phantom is the ladder's order, assumed.
- **Sway.** `ART_2048` inherits Omega's `sway` (`x_per_lean: -15`, itself HD's literal), which now acts on the 960-wide grid, so the slide on screen is twice HD's; 2048's own art records the same caveat. Unread.
- **Sight.** The lock-on grid is simulation state: a 2048-era circuit locks in the Vita's 960x544 (the HUD's authored grid), in the view build and in `Sighting::without_view` (the `oag-server` build) alike, asserted in the ground-truth test.
- **Not carried over from 2048's art**: `pickup_icon_uv` (the rectangles are in the Vita atlas's pixels and
  Omega's `.gnf` is repainted) and the Pilot Assist icon, so a held weapon's icon and the assist plate draw
  nothing in 2048 mode yet. The Zone and Time Trial roots are wired by name, not looked at.
- **Frame** (`--race --track Data\Environments2048\altima\track.vex --ticks 100/400`, 1920x1080, Venom): `LAP 1/4`,
  `TOTAL`, `CURRENT`, `POS 8/8`, the `100%` shield silhouette and the thrust bar are all drawn where the
  original has them; ours sits about 8 px right of the original's labels (not explained; a candidate is the
  shared safe-area slider's 1:1 inset).
- **Tests**: `omega_hud_ground_truth::a_2048_era_circuit_draws_2048s_hud_and_an_hd_era_one_keeps_hds`,
  `a_2048_era_circuit_runs_the_original_s_laps_per_class`,
  `omega_2048_mode_hud_names_are_authored_by_its_layouts` (the first two fail with `circuit_lineages: &[]`).
- **2048 cross-check: checked, differs.** 2048 (Vita) has no Race Box (`GAME MODE` is Omega's), its laps are
  the event's `m_numOfLaps` (`SP.xml`), and its HUD is already this set; nothing to wire there.

## The menu backdrop (2026-09-30, `omega-menu-backdrop`)

Omega's menus now sit on the HD-style `<BackgroundAnim>` scene, which is the only backdrop
Omega can have: no `.points2` clouds ship in any of the nine archives. The scene is
`FrontEndScene_HD_ATG.vex` with its PS4 `.rcsmodel` (byte-identical in `data00` and `data08`);
the `*_VR` scenes are not drawn. The `.vex` carries the motion and the camera (60 s loop, 2,274
translation keys on `camera1`), the `.rcsmodel` the geometry, and the two bind by shape name -
`oag_mesh::mesh::rcs::psp2::build_with_vex`. The picture is the scene drawn on white and filtered by
`FEBackgroundAnim_fp` into a grey line drawing; everything is read in
[`menu-backdrop-scene.md`](../ghidra/functions/ps3-hdfury-eu/menu-backdrop-scene.md).
`--menu-page main` (settled `Main Menu` row: edge 0.6, fill 0.2, width 0.5, no blur) and any other
page (the `default` row: 0.3, 0.1, 1.5, blur 3) draw it, and `--anim-seconds` picks the moment. **The
boot screens do not** - they are outside `Top FE Screen` in the skin, so the original has none there
either. **Not validated against the original**: no capture of Omega's menu backdrop exists (shadPS4 pairs only the race's start grid, `scripts/shadps4-omega-pair.py`) and HD's one capture of its HD
style shows a flat white page. Chosen, not measured: the blur kernel, the linear sampling, and which
page takes which row. The `--menu-page grid-select`/`cell-select` stills draw it too (they take the
page's picture). **The live campaign stage draws it too, and did before this was written**
(`menu_stage.rs` hands the stage's `shown` picture, the movie or else the style's
`Live::picture`, to every campaign list builder, and `Live::tick` runs per frame): walked windowed under
Xvfb + lavapipe on `main` `4def1665` plus the texture-limit change, language -> `Main Menu` -> `RACE CAMPAIGN`
-> `Grid Selection` -> `Cell Selection` by mouse, 2026-10-02, and the scene draws behind all three. The end-of-race screens sit under the
same widget and do not yet. Omega authors both selection screens
(`Track_Selection_Definition.xml` and `Team_Selection_Definition.xml`, `data08`/`data09`, the
files `skin.xml` includes by `SrcRel`); Track Select is read and drawn
([below](#the-circuit-model-on-track-select-2026-10-09-omega-track-model)), and so is Team Select
([Ship Select](#ship-select-2026-10-14-omega-ship-select)).

## The Track Select fly-by (2026-11-19, `hd-track-flyby`)

**Checked, differs.** Omega's `Track_Selection_Definition.xml` authors HD's `FlyByMovie` (X 500, Y 190,
260x170, `Loading_Icon.bik`), but no environment of the merged `data00`..`data09` ships a `preview` file in
any container, so `oag_title::FrontEnd::circuit_fly_by` is `None` and the panel draws nothing. The box's
picture on Omega is the `TrackPreview` image (`fe/track0N.gnf`), unwired. Detail:
`docs/ui/campaign-screens.md`, "The fly-by, drawn".

## The circuit model on Track Select (2026-10-09, `omega-track-model`)

**Omega draws each circuit's own front-end model on `Track Select`, as HD does.** The screen is
HD's file (`Track_Selection_Definition.xml`, nine lines of diff in colours and `.4x` texture
names), read by the same reader (`oag_ui_screens::picker::hd::track`), reached from RACEBOX ->
START (`track_select` is `Some`; at the time Omega's ship picker was `None`, so a confirmed circuit
launched the race - Ship Select follows it now, see [below](#ship-select-2026-10-14-omega-ship-select)). Walked live under Xvfb 2026-10-09: RACEBOX, START, `Track Select`, Right steps the
circuit and the model reloads, a click on a hexagon selects it, Return launches.

**The table** is `oag_omega::frontend`'s `CIRCUIT_MODELS`, 26 rows, each a `fe/` scene on
`data00.psarc` (`<folder>\FE\<scene>.vex` and its `.rcsmodel`): HD's twelve under Omega's own
folders (`01_vineta_k` .. `15_anulpha_pass`, `02_metropia`, `03_moa_therma`, the four pack
circuits `talons_junction`, `amphiseum`, `modesto_heights`, `tech_de_ra`), the four Zone circuits
(`zone_1..4`, `track01`; the screen lists them as `25_Track`..`28_Track`) and 2048's ten under
`environments2048` (`Track01.vex`). Every row builds (22 to 156 thousand vertices); every circuit
the screen lists has one. The HD folder names (`03_Track`) are not Omega folders.

**Geometry** is 2048's container: `oag_mesh::mesh::rcs::psp2::build_with_vex`, the builder the
menu backdrop already joins a PS4 scene with, now shared through `oag_game::preview::psp2_scene`.
The Omega pairs hold HD's own geometry (Moa Therma's bounds match HD's `03_Track` to four
places). Not every Omega `.vex` says so in its header, so `psp2_scene::circuit_model` asks the
sibling `.rcsmodel` which container it is - scoped to the circuit screen, so `preview::model`
(the flyer cards, the end-of-race placements) is unchanged.

**The material is not HD's `cf_fetracks`.** All 26 models name
`FrontEndConstantFranelBlend.rcsmaterial` (`_1`, `_2` and `cf_FEtracks*` files sit in the same
folders and no model, `.vex` or material names them), which carries no texture: there is no
`fe_grad`. Its pixel shader reads, per pixel, **confidence 80** (a hand-built GCN decoder over
one shader, member order from the file's own reflection table, no emulator run; since 2026-10-27 the
lines below are reproduced by `oag_rcs::gcn`, [gcn.md](gcn.md), and held by `gcn_ground_truth`):

```text
refl  = ReflectivityMin + ReflectivityScale * (1 - N.V)^ReflectivityPower
rgb   = colourDiffAlpha.rgb + Constant1 * constantAmbientColour * refl
out   = fog(rgb)           ; fogColour mixed in by the vertex's own factor
```

Evidence: the pixel shader is the `Shdr` at byte `0x323c` of
`Data/environments/15_anulpha_pass/fe/Materials/FrontEndConstantFranelBlend.rcsmaterial`
(`data00.psarc`; type `0` by the footer's `(byte 8 >> 2) & 0xf`, 436 bytes). Its constant buffer
members, in reflection order, are `colourDiffAlpha`@dw0 (4), `ReflectivityMin`@4,
`ReflectivityScale`@5, `ReflectivityPower`@6, `Constant1`@8, `constantAmbientColour`@12,
`fogColour`@16, `fadeResCoeffs`@20, and the lines that carry the law are:

```text
v_mad_f32  v7, -v6, v7, 1.0 clamp   ; 1 - N.V, from two interpolants normalised by v_rsq
s_buffer_load_dword s[22], s[0:3], 0x6        ; ReflectivityPower
v_log_f32  v7, |v7|
v_mul_f32  v7, s22, v7              ; * power
v_exp_f32  v7, v7                   ; (1 - N.V)^power
v_mov_b32  v12, s20                 ; s20 = ReflectivityMin (dw4)
v_mac_f32  v12, s21, v7             ; + ReflectivityScale (dw5) * that      = refl
v_mul_f32  v10, s4, v10             ; s4.. = Constant1 (dw8..10) * constantAmbientColour (dw12..14)
v_mad_f32  v6, v10, v12, s0         ; * refl + colourDiffAlpha (dw0..2)
v_mac_f32  v3, -s12, v0 / v_mad_f32 v3, v0, v6, s12   ; fog: fogColour (dw16..18) mixed by attr0.w
```

The model's material instance authors every uniform but one: `Constant1` `1 1 1` (the Zone
circuits `0.98 0.97 0.94`), `colourDiffAlpha` `0.060 0.048 0.029` (Zone `0.002 0.001 0.0006`),
`ReflectivityMin` `0.05`, `ReflectivityPower` `3`, and `ReflectivityScale` `1` or `2` (Talons
Junction and Tech De Ra `2`). A circuit is dark face-on and bright at the rim.
`oag_game::preview::track_model::Ramp::fresnel` evaluates it once per vertex per frame, the same
way HD's texture ramp is (`Ramp::of` picks by the model's container, never by title).

**Chosen, not measured**: `constantAmbientColour` (no instance authors it; its source was not
located; taken as 1, as HD's multiplier is), no fog, and **unlit**: the same file carries pixel
shaders of the same material that also declare `liveLighting0diffuse`/`direction` (`0x3dbc`,
`0x49dc`), and which variant the screen selects is unread. At `ReflectivityScale` 2 the rim
reaches 2.11 and clamps in the 8-bit target; whether Omega's front end tonemaps is unread. HD's
same-named material (unreferenced there) shares the idea, a fresnel term over a constant
(`ps3-microcode.py fp-file`, block #2: `Constant1` mixed toward a second constant by
`(1 - N.V)^5`), but its form is not Omega's and the Omega shader is the one that was read.

**The widget** is authored differently: `OriginX/Y` `960/540`, `x` `0.525`, `y` `0.125`, `z`
`-2.05`, `orthoScaleX/Y/Z` `0.012`, `nearZ` `0.1`, `FoV` `100`. `orthoScale` scales the
placement (the campaign flyer's reading, `oag_game::flyer::campaign_pose`), which stands the
circuit 43.8 right, 10.4 up and 170.8 in front of the axis, close to HD's 180. **Chosen, not
measured, inherited from HD**: the field of view (1.0 radian; `FoV` 100 is unread, it is not
the flyer's units), pitch, the 24 s turntable and its start yaw. 2048's circuits are larger than
HD's and overrun the frame a little at this pose.

**Open**: the emblem of a 2048 circuit; the page's white
backdrop, against which the rim-lit circuit loses its edge (the material is authored for a dark
one); whether Omega's original ever shows this screen outside RACEBOX. The circuit names are
closed: see "Omega's Track Select names all 38 circuits" below (`--menu-page track-select` reads
`ANULPHA PASS`).

**2048, checked, applies, not wired.** 2048's own archives carry `fe/Track0N.vex` only for its
DLC1 circuits (`Vineta_K`, `Moa_Therma`, `Chenghou_Project`, `Anulpha_Pass`, `zone_1..4`, under
`art/published/DLC1/environments`), none for its ten own circuits (which Omega adds), and its
front end authors no circuit screen (`race_box`/`track_select` are `None`), so nothing draws
them; the reader and the material shape are the same.

## Ship Select (2026-10-14, `omega-ship-select`)

**Omega's `Team Selection` (and its `Player 1`/`Player 2` copies) is HD's file in HD's dialect and now draws
what HD's does: open model cells, each model's ratings, and the craft.** Reached from RACE -> START -> Track
Select, and per seat in a split. Before this change every cell was a padlock, the statistics bars empty and
the `SHIP MODEL` frame blank (`92aba1858` had wired only the screen).

**The roster, off `Data\plugins\teams\Definition.xml` (`data09`, 70,000 bytes; measured).** 19 `PI_Team`:
five 2048-era (`Feisar2048`, `Qirex2048`, `Piranha2048`, `AG_Systems2048`, `Auricom2048`, four craft each plus four
`IsVR` copies under `VR_2048`), twelve HD-era (`AG_Systems`, `Assegai`, `Auricom`, `EGX`, `Feisar`, `Goteki`, `Harimau`,
`Icaras`, `Mirage`, `Piranha`, `Qirex`, `Triakis`) and `Tigron_x` and `Tigron` (named `Tigron` and `VanUber`). Each HD-era team
authors thirteen `PI_TeamModel`: nine with an `<FE speed thrust handling shield length firepower>` (`chrome_c1`, `nitro`,
`concept1`, `normal`, `SKIN1`, `SKIN2`, `chrome`, `vr_hd`, `vr_fury`; `zone`, `detonator` and the two `vr_` ones have none), which is
the **nine rows** the honeycomb authors. The screen offers **19 teams** since 2026-10-14 (`omega-2048-era-teams`, below): the twelve, the two Tigrons and
the five 2048-era teams. Before that it offered 14: `load_teams` asked for `hdships\<id>\Ship.vex` and
`handlingstats.xml` by the bare id, and the 2048-era teams' sit under `Ships\<id>\<n>\` (the load report said "5 declared team(s)
have no ship or no handling stats").

**What opens.** `oag_omega::race::TEAM_VARIANTS`: the twelve HD-era teams, `""`, `_c1`, `_n1` joined as a suffix, labelled with the
models' own names (`Normal`, `Concept`, `Nitro`; **measured**: `normal` is at `hdships\<Team>`, `concept1` at `<Team>_c1` and
`nitro` at `<Team>_n1`). Since 2026-11-14 the rows open by each model's own `<Unlock>` (HD's rule and rows, ported:
[race-setup.md](race-setup.md#hd-era-model-unlocks)): a fresh profile opens `concept1` and `normal`, loyalty and
campaign golds open `nitro`, `SKIN1`, `SKIN2`, `chrome` and `chrome_c1`, each racing its own livery folder; the
two PSVR models stay padlocks (no directory of the variant table), by `Team::hex_cells`. `crates/raceplay/tests/omega_2048_era_craft_ground_truth.rs`
now resolves every combined id's hull, `.rcsmodel` and tuning (24 more than before) and holds Feisar's
ratings. `Tigron_x` and `Tigron` carry no `_c1`/`_n1` directory (their models all name one folder), so they sit outside the table
and `Team::own_directory_variant` gives their screen the one row their own directory is: of the models sharing it
(none is `<Unlock>`-gated) the one named `normal` opens, and since 2026-11-14 their three skins
(`SKIN1`, `SKIN2`, `chrome`, in their own livery folders) open beside it (`Team::model_for_directory`; the first in file order was `chrome_c1`
with the wrong ratings), `Tigron_x` rated 10/7.5/7/7 and `Tigron` (the disc's "VanUber") 8.5/9/10/6.5 (pinned in the ground
truth; walked 2026-10-14, one open cell each). **Chosen, not
measured**: `opponents_match_player_variant` is `false` (HD's `true` was read off HD's executable, Omega's `Ship_Construct` is unread)
and `fresh_variant` is `None` (a fresh profile opens on `normal`). **Not measured**: whether the original shows the two
PSVR rows on a flat screen; this build draws them as padlocks.

**The statistics.** The four bars take the selected model's own `<FE>` (tenths, so Feisar `normal` reads `070 080 100 080`
and `concept1`/`nitro` `080 085 100 080`) through `Team::variant_stats`, as HD does, with the down-nobble and the three-digit
value. **The bar's two halves.** HD draws left of the split in `HD_Grey` and right of it in the
block's `AlwaysSolidColor` (`0xff646464`). Until 2026-10-14 (`omega-colour-template`) this build resolved Omega's `FEGlobals->HD_Grey`
to `0xFF646464`, the value in the skin's first colour block (`HD_Colours`), whose own comment says it is *"IGNORED AND OVERWRITTEN by
the WOHD_Colours and Fury_Colours templates"*, so the two halves were one grey. Omega's boot copies `WOHD_Colours` over that block
(always: the selector compares `"HD"` with itself, see [omega-frontend.md](omega-frontend.md#the-colour-skin-switch-is-restructured-not-just-copied)),
`HD_Grey` is `0xFF010001`, and the left half is near black over the grey remainder, a visible bar (seen in a headless still and in a windowed boot-to-Ship-Select run on Xvfb:
the rating, the nobble and the three digits now sit on a two-tone bar). The same change moved every Omega screen's `HD_Grey`-tinted chrome from mid grey to near black. `LOYALTY` stays empty in our own build, but Omega's executable does keep loyalty (its award is HD's law, `docs/ghidra/functions/ps4-omega-eu/endrace-loyalty.md`); the race now banks it. `firepower` and
`length` are authored per model and no block of this screen shows them: drawn nowhere.

**The craft.** `ship_preview_hull` is `"ship.vex"` and the hull is the team's race hull, a PS4 `.rcsmodel` (8-byte pointers, `.gnf`
textures) that `preview::model` does not read: the preview goes through `psp2_scene::model` (the loader the circuit model already used, renamed
from `circuit_model` and now used by `variant::model_or_default` too, so the flyer and end-of-race callers still do not draw a PS4 scene).
Each of the fourteen builds with its own textures (Assegai: 19,934 triangles, 16 submeshes) and `_c1`/`_n1` are hulls of their own (Feisar: three shapes).
**Pose, read off the update (2026-11-19, `hd-ship-craft`)**: the screen's update (`FUN_015a3b80`) writes the `ShipModel` item's rest rotation
(`+0.4307`, `-0.4438`), position `(0.4, 0.1, -2.05)`, scale `0.1` and lens `100` degrees every frame, over the widget's own origin `(960, 540)`
and `nearZ 0.1` (`oag_title::FrontEnd::ship_preview_pose`, `Some` on Omega only). Its depth fit runs and its result is never read, so
there is no fit and no settle on a step. Those values draw the craft 2.5 times smaller than a shadPS4 frame shows, and the one factor that
closes it (`ShipPose::zoom`) is **fitted to that frame, its cause open**. Feisar's concept hull then lands within 1 pixel across and 7 down of
the frame's, and the pair `omega/menu-ship-select` (re-made on the original's own selection) reads 26.13 to 20.03 mean grey
(`docs/ghidra/functions/ps4-omega-eu/team-selection-pose.md`). The load report says "ship preview: rest pose from the screen's update".
Omega's `<FE length>` (Feisar `3.8`) is read and unused.
**Not drawn**: `ShipDesc` (the model's description string, authored per model with `teletype`), the `L1/R1` livery hint,
`PadlockText1/2` (they hold unlock conditions this build keeps no state for).

**Walked 2026-10-14**, windowed on an own Xvfb with two virtual pads: RACE, two players, join, Track Select, `SHIP SELECT - PLAYER 1`
(Assegai, then Mirage) and `PLAYER 2` (Icaras), each with its own hull and ratings, then a two-view race flying Mirage and Icaras. A one-player
still (`--menu-page ship-select`) and a `--race --team Feisar --variant _c1`/`_n1` race load.

**2048 cross-check.**
- *Team definition and ratings*: **checked, applies, not wired.** 2048's own `Data\plugins\teams\Definition.xml` is the same shape: the catalogue reader finds
  17 teams and 104 `PI_TeamModel` with an `<FE>`, all 104 rated (run on `PCSF00007/base`). 2048 authors no
  `Team Selection` (and so no per-player pair), so nothing draws them.
- *PS4/Vita `.rcsmodel` hull as a preview*: **checked, applies, not wired.** `psp2_scene::model` reads the Vita container too (`mesh::rcs::psp2::is_psp2`); 2048 has no ship screen to draw it.
- *Numbered craft (`Subdirectory` join) in the honeycomb*: **done on Omega 2026-10-14** (`omega-2048-era-teams`, next section); 2048's own result is there.

## 2048-era teams on Ship Select (2026-10-14, `omega-2048-era-teams`)

**The five 2048-era teams (`Feisar2048`, `Qirex2048`, `Piranha2048`, `AG_Systems2048`, `Auricom2048`) are on Omega's Ship Select, for one
person and for each split seat, and picking one starts a race on that craft.** Walked on an own Xvfb with a virtual pad: RACE, START, Track Select,
`SHIP SELECT`, right to `Feisar2048` (the team column is the definition's file order, which puts them first; the wrap from `Tigron` is **chosen, not
measured**), the `speed` craft, confirm, and the race loads and starts on the yellow `Feisar2048\3` hull. The split seats were walked too (two virtual pads: RACE, 2 players, join, Track Select): `SHIP SELECT - PLAYER 1` on `Feisar2048`, `PLAYER 2` opening on the next team, `Qirex2048`, with its own hull and ratings, then a two-view race flying a `Feisar2048` and a `Qirex2048` craft against HD-era opponents.

- **What a team is.** `Feisar2048` is offered under its bare id, as the definition declares it; the bare id names no craft (`Ships\Feisar2048\ship.vex`
  does not exist), only `Feisar2048\1`..`\4` do. `oag_title::GuestRoster::offered` (data, `true` on Omega's roster, `false` on 2048's HD-derived twelve,
  which stay reachable through RACE REMIX only) lets `boot::roster::load_teams` offer a guest roster's team when **every** craft its table names resolves
  its hull and tuning in the roster's own directories (`ships_for`/`handling_dir_for`). The load report's "declared team(s) have no ship" line is gone on
  Omega. Compared with a build of `28d164268`: HD's Ship Select frames are byte-identical and 2048's team list is the same five; 2048 has no
  ship-select page to compare.
- **The join is data.** `Team::hex_cells` and `variant_stats` take the `VariantJoin` (`RaceDefaults::join_for`, the variant table's own join, a suffix
  for a team with none): `Data\...\Feisar2048` + `3` is `...\Feisar2048\3`, where the old suffix append gave `Feisar20483`. `Team::model_for_directory`
  now compares the whole path, not its leaf: `Feisar2048\3` and the `IsVR` model at `VR_2048\3` share the leaf `3`.
- **Rows, in the definition's file order** (so the honeycomb places them, nothing chosen): `speed` (`\3`), `combat` (`\1`), `agility` (`\2`),
  `prototype` (`\4`), then the four `IsVR` models (`vr_speed`...), which this build draws as padlocks like HD's two PSVR rows. Four open cells and four
  padlocks per team, and a ninth padlock under them: Omega's grid is nine rows and no model fills the ninth, so `hex::draw` gives it the grid's padlock
  (**chosen, not measured**; HD's own seven rows are all filled, and HD-era teams author nine models). The variant table's labels are the models' own `name` (`combat`, `agility`, `speed`, `prototype`; Omega's table said `fighter` for `\1`
  after the string id, 2048's own table still does); the ground truth holds each label to its model. **The change is visible outside Ship Select**:
  RACE REMIX's VARIANT row for "Wipeout 2048 (Omega)" reads this table (`variant_choices`), so its first entry now says `combat`. The HD-era table's labels are
  title-cased (`Normal`/`Concept`/`Nitro`), these are the lowercase authored names: a convention difference, not changed here.
- **Ratings** off each model's `<FE>` as for HD (Feisar2048: combat 5/8.5/6/8, agility 5/8/9/6, speed 8/8/7/4, prototype 8/8/7/4, in tenths 50/85/60/80 ...).
  Thumbnails are `<model>\FE\thumb0.gnf` per craft (drawn), the logo `Ships\<Team>\FE\Logo.gnf` per team: `picker::hd::logo_src` now takes the team's
  `location` and the entry carries it (`Details::Ship::logo`), where it joined `ship_dir` and the id and so only reached `hdships`.
- **The preview** is `Ships\<Team>\<n>\ship.vex`, through the same `PreviewSource` join the HD-era variants use; the pose is the update's rest pose (above).
- **Team-level unlock rules are not read, as on the HD-era teams.** `Piranha2048` and `AG_Systems2048` author `<Unlock purchase="1">` on the team (as HD-era
  `Qirex`, `Piranha`, `AG_Systems`, `Triakis`, `Goteki`, `EGX`, `Assegai` do, and `Mirage`..`Icaras` a `grid`); this build keeps no unlock state, offers all
  nineteen and says nothing is locked. **Not measured**: what the original draws for an unpurchased team.
- **Not drawn, said so**: `ShipDesc` (the `description` string id), the hexagonal window's stills (`Ships\<Team>\screen.xml` has no slideshow chain, the load report
  says so), the four `IsVR` models.
- **Opponents unchanged (chosen, not measured).** The grid's pool is drawn from the bare ids of the offered teams except those that name no craft
  (`RaceDefaults::names_no_craft`): nothing chose which craft an AI flies, and a bare `Feisar2048` is not a directory. A race's opponents are the HD-era teams
  as before, whoever the player is; whether the original mixes eras is unmeasured. A player's pick reaches the race through the usual
  `apply_race_team`/`combine_variant` (`Feisar2048` + `3` = `Feisar2048\3`), pinned by a test.
- **Frames.** HD's Ship Select frames (Feisar `_c1`, Mirage `_n1`) are byte-identical to a build of `28d164268`. Omega's HD-era frames differ in the two
  neighbour columns (the wrap now shows 2048-era neighbours) and by at most 2/255 on a few frame lines, the sprite sheet having grown by the five logos and
  forty thumbnails.

**2048 cross-check.**
- *The teams definition and `<FE>` of the five teams*: **checked, applies, not wired.** 2048's own `data/plugins/teams/Definition.xml` authors the same twenty
  models (name, directory and `<FE>` identical to Omega's, all twenty compared, `IsVR` copies aside); the reader finds all of them. 2048 has no Ship Select, so
  nothing draws them.
- *Logo and thumbnails under each team's own directory*: **checked, differs.** 2048's `data.psarc` has no `Ships\<Team>\fe\` entry at all (no `Logo`, no `thumb0`), only `FE\NewImages\Team_Logos\<Team>2048.gxt` icons (Omega ships those as `.gnf` too, and the directory ones besides); it has no screen to need either.
- *`Subdirectory` join in the honeycomb*: **ported**, to Omega, with a disc-backed ground truth (`omega_2048_era_craft_ground_truth`); 2048 has no honeycomb.
- *Labels*: **checked, differs.** 2048's `oag_2048::race::TEAM_VARIANTS` calls craft `1` `fighter` (string id `..._Fighter_...`), its definition `combat`.

## What a menu-page capture does today

**Invocation (watched working, 2026-09-30).** The source is a positional path,
the extraction root holding both packages:

```sh
cargo run -p oag-game -- data/extracted/ps4 --no-audio --menu-page main --screenshot out.png
# or, after a build: ./target/debug/oag-game data/extracted/ps4 --no-audio --menu-page main --screenshot out.png
# the Language Selection boot screen:
./target/debug/oag-game data/extracted/ps4 --no-audio --until "Language Selection" --hold start --screenshot out.png
```

`--menu-page` takes `main`, `grid-select` or `cell-select`. `just play omega ...`
maps the keyword `omega` onto `data/extracted/ps4` and forwards the rest; it
builds `--release`, and was not re-run here. A bare `oag-game --menu-page main`
with no path, or with `omega` passed as if it were one, reads no source and
fails with the missing `<SOURCE>` usage error.

**Scale (2026-09-30, `omega-fe-retouch`).** Omega's source used to fall through
`Space::of` to the PSP's 480x272 grid ("front-end grid 480x272" in the boot
report), so HD-authored coordinates were drawn on a grid four times too small
and the disc's own fonts came out huge. Now `Space::OMEGA` (1920x1080, grid
confidence 85: the legal line at `x=960 y=972` and two `line.gtf` rules at
`x=160 width=1600`, asserted by `omega_font_scale_ground_truth`). Omega's faces
are also HD's at twice the pixel size (`helv` line height 64 against 33, `helvb`
89 against 44, `PS_BUTTONS` 109 against 54; median glyph width, height and
advance ratio 2.0 on every shared glyph), so its atlases are read at
`Space::font_texel_scale` 0.5 - **an inference, confidence 60**: the ratio is
measured, that the PS4 build draws at half size is not (no PS4 text path has
been read). The main menu now lays out as HD's does.

**Screens (2026-09-30, `omega-fe-screens`).** Four defects closed, three of them
shared with HD:

- *Language Selection rows overlapped* (HD too). The picker's `<Menu>` authors
  x, y, scale, colour and alignment and **no pitch**, so the step was always an
  inference, and the table it inferred from (`font_line_height`) is Pulse's: 13
  units against HD's 33 and Omega's 32 (`helv`, 64 at 0.5). `oag_title::BootProfile::picker_from_loaded_faces`
  (true for HD and Omega only) now steps by one line of the loaded `Default`
  face; the face height is the disc's, the rule "one line of the widget's font"
  is the inference. The selected row's band is drawn the full row and 300 wide,
  **chosen, not measured**. No reference exists: HD's picker was never caught on
  RPCS3 (`hd-frontend.md`, the redirect fires in milliseconds) and Omega's
  picker was never captured on shadPS4. Pulse, Pure and 2048 still use the table, byte-identical.
- *The confirm glyph was a box* (HD too). `ControlTextConfirmButton` authors
  `font="menu"`, HD's language plugins declare no `menu` role, and `FE_CONFIRM_BUTTON`
  is a single Buttons-face codepoint. The picker now draws it (and the back
  prompt) through the `Buttons` slot, the mechanism the campaign footer already
  uses. Which face the original picks is **chosen, not measured**.
- *`cell-select`/`grid-select` raw ids ("Event line", "rc laps line"), `%d`, the
  footer overprint and "more points needed" over "010"* were one cause: the
  campaign still and live stage dispatched `title == HD` to `oag_ui_screens::campaign::hd`
  and sent Omega to Pulse's draw list, which has no arm for HD's widget names.
  `oag_game::campaign::draws_hd_campaign` covers both titles. Omega's `Cell
  Selection` also authors a `RecordsButton` (`FE_RECORDS`) at exactly
  `DifficultyButton`'s `x=944 y=994`; it is suppressed (no leaderboard behind it,
  as HD's endrace drops `RecordsCycle`) - **chosen, not measured**.
- The HUD draws on an Omega race now (`omega-hud`, 2026-10-06, above).

Still visible on Omega: the three medal icons under the target row
drew bronze, silver, gold (fixed, see "The medal atlas"); no flyer and a white backdrop on `grid-select` (`flyers:
None`); `Event 01/08` is an English literal on HD too; the Endrace screens still
dispatch `title == HD` alone. The picker's `Svenska` x4 and `Portugus` are **the
disc's own**: `definition.xml` really says `ID="Japanese" String="Svenska"` (and
Korean, TraditionalChinese), and `ID="Portuguese" String="Portugus"`. `P??????`
(Russian) is **not** Cyrillic missing from an atlas: the file holds seven bytes
`50 3f 3f 3f 3f 3f 3f`. The plugin's own table authors `Русский` (`OPT_RUSSIAN`),
which the loader reads instead, and the picker's `helv.fnt` has no Cyrillic, so
Russian is not offered there (2026-10-09, checked against HD: same bytes).
Omega's `Portugus` is the same class of loss (the `ê` is absent from the file)
and is left as the disc wrote it.

`just play omega` boots the front end, reads `data09.psarc`'s ten `FEGlobals`,
the boot chain and the string table, and stops on **Language Selection** - the
disc's own real first screen - because German (this build's compiled-in
default) is not offered: every `english`/`german`/... `Definition.xml` this
lane found is one of the all-zero copies `omega-frontend.md`'s own census
already flagged for `english`, so the picker shows exactly as it would for a
real player with an unsupported language.

`--menu-page main`/`grid-select`/`cell-select` draw this build's own
reimplemented menus - **not** the disc's own `Main Menu`/`Grid Selection`/
`Cell Selection` screens, which are never parsed for an HD-idiom front end at
all: `crates/game/src/boot/includes.rs`'s `LoadXML`-follow mechanism only runs
for a title whose `FrontEnd::touch` is `Some` (2048 alone), by design, since
following every include on HD would pull in art this build never samples.
`main` draws the RACE CAMPAIGN row on white, with `Title_Arrow_HD.gtf`'s
decoded `.gnf` sibling as the nav-legend arrow (no block art - see "What did
not draw" below); `grid-select` shows `EVENT 01/08` and lays out the twelve
grids that parsed, with `Subtitle_Arrow_HD.gtf`/`Padlock.gtf` also decoding
through their own `.gnf` siblings; `cell-select` draws the header with no
cell content (needs a grid selected first, not exercised this lane).

### What did not draw, by name

- **Most of the front end's own *boot-time* sprite set** (8 names reached
  before `Language Selection`) - `saveIcons.gnf` is present but its base
  level carries 342 corrupt blocks (`Error::CorruptBlocks`, refused by
  name); `line.gnf` is present but fails the `GNF ` magic check outright
  (an all-zero PSARC entry, the same population `psarc.md`'s "Block data
  location" section documents); `vr_headset.gtf` and the four
  `NewImages/Demo/*.gtf` names have no entry at all on this base+patch
  extraction, `.gnf` sibling included. Only `Title_Arrow_HD.gtf` (32x32)
  decodes and draws. The front end's *full* sprite sheet fares much better
  off this same boot's screens - see `gnf.md`'s own census (219 of 289
  decode) - this boot-time set is just small and unlucky.
- **`Data/FE/Images/StudioLiverpool.bik`**, `Studio Logo`'s own reel - not
  found under that name in any of the nine archives, unlike every other
  front-end asset this lane looked for. Whether it is elsewhere under a
  different name, or genuinely absent from this build of the front end, was
  not chased.
- ~~**The menu block art** (`MenuSkin::blocks`) - deliberately left `None`~~ **drawn, 2026-10-06
  (`omega-frontend`).** The 2026-09-30 reading "the fill swatch reads alpha 0.004 on Omega's copy,
  cause unread" had a cause: **Omega's `file2.gnf`, `cursor.gnf` (and `file`, `corner`, `corner2`,
  `square`, `line`, `unlocked_corner`) decode to HD's `.gtf` pictures in HD's own file row order,
  bottom-up**, where every image a screen names was re-authored top-down. A `.gnf` carries no
  flag, and the sheet leaves a `.gnf` unreversed (right for the 276 re-authored ones), so the
  nine-patch came out mirrored and the swatch texel `(4.5, 58.5)` of the file was read at the
  wrong row. Census (Omega `.gnf` against HD
  `.gtf` of the same stem and size, deduplicated across the base and patch archives): the
  re-authored ones match HD's rows reversed (135), the eight above match HD's rows as stored,
  22 are symmetric (no verdict), 41 differ in size, and 971 have no HD file of the stem to
  compare (so they are unverified, drawn unreversed as before). The census compares each Omega `.gnf`'s decoded rows with HD's `.gtf` of the same stem,
  as stored and reversed (a throwaway probe, not committed). Fix: `oag_title::FrontEnd::bottom_up_gnf`
  (the eight stems, empty on every other title) makes `boot::sprites::load` reverse them as it
  does a `.gtf`; that includes `line` (the two rules `skin.xml` stretches across the frame) and
  `unlocked_corner` (the selection and manual screens), which now match HD's sheet; `oag_omega::frontend::MENU_BLOCKS` is
  HD's table, **chosen, not measured** for Omega - every number is read from HD's `EBOOT.elf`
  and nobody has read Omega's `eboot.bin`; what supports it is that the nine-patch decodes to
  HD's picture to within BC7 error (`omega_menu_blocks_ground_truth.rs`, texel for texel
  within 8/255 of HD's sheet on this engine, and a focus move eases the strip) and that the boot report reads
  `menu blocks: HD style, fill swatch alpha 0.427 then 1.000, underline decoded, arrows decoded`.
  **Reference: HD's front end on this engine, and Omega's own shipped data. No capture of
  Omega's front end exists (shadPS4 pairs only the race's start grid), so nothing here is compared with the original console.** The strip is now bordered
  blocks with a chamfered landing, the selected one eased wider (`Menu::focus_of`, one sixth a
  tick) with the underline mark sliding and blinking; before, the blocks-less strip took the
  measured-tab path whose tabs are fixed to their text - the "do not animate" in the report.
  Settings rows take the List blocks (520/280 to 340) and step arrows the same way. Not yet
  verified live: the pointer hit regions on the new block geometry are HD's own code path
  (`regions`), covered by `campaign_pointer_ground_truth` only for the campaign.
  **Removed with it:** the 2026-09-30 white-row-on-white-page boxes
  (`rows::text_is_lost_on_page`, `row_box`); Omega no longer reaches the bare-text row path,
  and no other title had a light page under light rows, so it was dead.
  **2048 cross-check:** `checked, applies, not wired` - 2048 declares no `MenuBlocks` and
  draws a touch front end, so there is no block art to reverse; whether its package ships
  `file2.gxt` in HD's row order was not opened.

- **Most of the campaign hex textures** (`Hexagon_HD_OUTLINE.gtf`,
  `Hexagon_HD.gtf`, `Hexlock_HD.gtf`, `Hexagon_HD_THICK_OUT.gtf`,
  `NonSelectable_Arrow_HD.gtf`) - `crates/game/src/campaign.rs::load_omega`
  now falls back to `crate::boot::sprites::gnf_sibling` the same way the
  front end's own sheet does, but none of these five has a `.gnf` sibling
  either on this base+patch extraction - genuinely absent content, not an
  unread name. `Hexmedal_HD.gnf` **is** present (1,402,112 bytes) but fails
  the `GNF ` magic check, the same all-zero population as `line.gnf` above.
  `Subtitle_Arrow_HD.gtf` and `Padlock.gtf` (`OTHER_TEXTURES`) both decode
  through their `.gnf` siblings and draw - see the `grid-select`/
  `cell-select` screenshots.

### Why so much of this is missing: an extraction-tool bug, not this build's data

`lane/omega-psarc`, 2026-09-27. Every "not found"/"all-zero"/"corrupt
blocks" name above (`saveIcons.gnf`, `line.gnf`, `Hexmedal_HD.gnf`, the five
campaign hex textures, most of the boot-time sprite set) was checked against
one open question this project's own extraction of
`omega-ps4-eu{,-patch}.pkg` left unresolved: is the content genuinely absent
from the package, or did this project's own copy of it come out wrong? It is
the latter, confidence 90 - see
[`gnf.md`'s "Root cause"](gnf.md#root-cause-confidence-90-a-short-streamread-in-the-extraction-tool-not-this-projects-reader)
for the full mechanism. In short: `PkgTool.Core`'s (`LibOrbisPkg`'s) PFSC
sector decompressor calls `DeflateStream.Read` once per 64 KiB sector and
never checks whether that call actually filled the buffer - most of the time
it does not, and the untouched remainder reads back as zero bytes, which is
what this page's "all-zero"/"corrupt" checks are seeing. **Not this
project's PSARC/GNF readers** - both are confirmed correct against the raw
archive bytes and against a from-source repro of the extraction tool. A
one-line loop fix in that tool's `PFSCReader.ReadSector`, verified by
rebuilding the tool and re-reading `Harimau_c1_Livery.gnf` directly out of
the real `.pkg`, turns a previously-refused (`Error::CorruptBlocks { count:
49899 }`) livery texture into a fully legible PNG through this project's own
unmodified decoder.

**Checked against a corrected re-extraction
(`data/extracted/ps4/{omega-eu,omega-eu-patch}`), 2026-09-27: every named
"missing"/"corrupt" texture above recovers, and none of it was genuinely
absent.** `saveIcons.gnf`, `line.gnf`, `Hexmedal_HD.gnf` and all five
campaign hex texture names (`Hexagon_HD_OUTLINE`, `Hexagon_HD`, `Hexlock_HD`,
`Hexagon_HD_THICK_OUT`, `NonSelectable_Arrow_HD`) all have real `.gnf`
entries in the corrected `data00.psarc` (duplicated in the patch's
`data08.psarc`) and decode clean through this project's own unmodified
`oag_texture::gnf::Texture::decode` - the old, corrupted manifest had not
just corrupted their pixel payloads but hidden most of them from the
directory listing entirely, which is why "no `.gnf` sibling either" read as
absence rather than corruption. `vr_headset.gnf` (447x370, `data08.psarc`)
recovers the same way, previously "no entry at all" for the same manifest
reason. `StudioLiverpool.bik` remains genuinely absent under that name even
in the corrected extraction, across all nine archives - not a reading
artefact. The exact decode results and the full census were a scratch file, not kept.

## The medal atlas (2026-11-05, `omega-medals`)

The player's report: gold and bronze change places on `Cell Selection`'s `Target0/1/2 Medal`,
silver is right. Neither of the two readings the thread opened with holds exactly: it is not an
indexed field whose order differs in the XML, and it is not a vertical flip between decoders.

- **The XML is HD's, to the digit** (confidence 95, read off `data09`'s `CellMode_Definition.xml`
  by `omega_medal_crops_read_gold_silver_bronze_in_every_difficulty_block`): `Target0/1/2 Medal`
  author `u=0`, `v=0/61/122`, 60x60, `TxtrWidth/TxtrHeight=60`, source `Hexmedal_HD.gtf`.
  Census of every `.xml` in the nine Omega archives (base `data00`-`04`, patch `data05`, `07`-`09`) for `Hexmedal`: only `cellmode_definition.xml`
  (the three `Target0/1/2 Medal`, 32 `Medal_x_y` with no crop, `MedalImageFury`/`MedalImageHD`)
  and `endrace_definition.xml` with its `_demo` twin (`Target0/1/2 Image` at `v=0/61/122`, one
  `Single Target Image`) name it.
- **The atlas keeps HD's grid** (confidence 90, measured): `Hexmedal_HD.gnf` is 976x549 against
  HD's 1024x768, which is 16 columns by 9 rows of HD's 61 px pitch (a 60 px frame and one clear
  row), three difficulty blocks of 183 px. Through every tier and block the icon's opaque rows
  stay inside rows 1..58 of its frame. `Hexmedal_HD.2x.gnf` is the same picture at 1952x1098;
  both files exist twice (base `data00`, patch `data08`); all four decode to the same
  per-tier mean colour in every block (to within a unit or two of 255).
- **The stored order within a block is bronze, silver, gold from the top** (confidence 95,
  mean colour of the opaque texels, first column frame, plain block: bronze rgb 196 174 135,
  green over red 0.887; silver 204 209 207, red minus blue -3; gold 214 212 168, 0.989; the hook
  and swirl blocks read the same way, see the test's output). **The blocks run plain, hook,
  swirl from the top**, HD's authored `v` order. HD's `.gtf` sheet is reversed on load, which
  puts gold at `v=0` there; Omega's `.gnf` is stored the other way round within a block and the
  sheet leaves a `.gnf` as stored, so `v=0` landed on bronze and `v=122` on gold.
- **A whole-atlas flip is refuted**: it would put the swirl block at `v=0`, and the shadPS4 frame
  (`omega/menu-cell-select`, `AI DIFFICULTY (NOVICE)`) draws plain hexes in the target row, gold
  at `1ST`, silver at `2ND`, bronze at `3RD`. So `Hexmedal_HD` is not an entry for
  `FrontEnd::bottom_up_gnf` either: that reverses the blocks and the tiers together, putting the
  swirl at Novice.
- **The fix is title data** (`oag_title::Campaign::medal_tier_rows`, Omega `[2, 1, 0]`, HD and the
  rest `None` = `[0, 1, 2]`), carried through `CellArt::medal_tier_rows` into `hd_medal_frame`,
  the one crop both draws of the atlas use: `Target0/1/2 Medal` and the hex `Medal_x_y` badges.
  `MedalImageFury`/`MedalImageHD` are not drawn (they author no crop), and `EndRace Results`'
  `Target0/1/2 Image` never loads the atlas on either title (`oag_hd::endrace::EXTRA_TEXTURES`
  omits it; Omega's end of race goes through the same `load_hd`), so the target row draws nothing
  there and there is no third draw to correct.
- **Measured against the capture**: over the target row's region of `omega/menu-cell-select`
  (x 750-1690, y 855-925), mean absolute grey difference against the original 4.51 before and
  2.19 after; per icon (1ST, 2ND, 3RD) 21.7, 1.7, 20.5 before and 1.4, 1.7, 1.1 after. Both
  frames opened and read by eye: gold, silver, bronze left to right, at 4x on the crop. HD's
  `menu-cell-select` still frame is byte-identical (same SHA-256) before and after.
- **What stays unmeasured**: Skilled and Elite on Omega. The atlas keeps HD's block order and
  Novice is plain on the capture, so hook is Skilled and swirl Elite by HD's capture
  (`docs/ui/campaign-screens.md`): confidence 70, no Omega frame at those rungs. Whether the
  original samples the 1x or the 2x file is unread (the `eboot.bin` is in Ghidra, the bridge was
  down for this lane); both carry the same tier order, so it changes sharpness only.
- **2048 against this** (census of every `psarc` of both Vita packages, base, patch and DLC:
  `PCSF00007` and `PCSA00015`): **checked, differs**. 2048 ships no `Hexmedal` file and no hex
  campaign screen; its medals are separate icons (`NewImages/medals/Icon_*_medal.gxt`,
  `Post_Race_Medals.gxt`). Nothing of this carries across, and nothing of 2048's is wrong by it.

## Campaign: confirming a cell starts its race, 2026-09-30

**Correction, 2026-09-30 (`omega-frontend-fixes`): the mouse walk below proved less than it
said, and a player could not start the second unlocked cell with a mouse.** It clicked
`grid0_3_1`, the first target in the list, which wins every overlap; `grid0_3_2` (also
`Locked="false"`, Speed Lap on `08_Track`) was never hovered. HD's and Omega's hex
sprites are 128x64 power-of-two textures holding a 72x62 hexagon in the top-left corner
(alpha box measured on `Hexagon_HD.mip`: x 1..72, y 1..62), and the pointer target was the
whole texture, so every hit region was 128 wide, 111 tall and 28 units right of the art.
Hovering `grid0_3_2` selected its padlocked neighbour `grid0_2_2` (the panel read
`Zeitrennen / 08_Track`) and a confirm did nothing; the log said only `grid0_2_2 is
locked`. The keyboard and pad path was never broken (`Return` and `Down`, `Return` both
reach a race, walked from the language picker). Fixed: the target is the box of the
sprite's opaque pixels (`Sheet::opaque_extent`, `oag_game::campaign::hit`), held by
`crates/game/tests/campaign_pointer_ground_truth.rs` on every Omega and HD grid (it fails
on the old targets). **HD had the same bug.** Two more things a mouse-only player met:
`Grid Selection` has no flyer card on Omega, so its click target was the invisible
`Flyer Pad Lock` rect - with no card drawn a click on the page's content band now confirms
the tier (chosen, not measured); and a refused cell (then 25 of 167: `NitroBattle`, `Detonator`; now 13, `NitroBattle`)
only logged, it now shows `OAG_CAMPAIGN_MODE_UNSUPPORTED` above the footer. The walk below
otherwise stands.

`omega-campaign-launch`, walked windowed under Xvfb by mouse: `RACE CAMPAIGN`,
`Grid Selection` (a click on the flyer's authored `Flyer Pad Lock` rect),
`Cell Selection` (a hover, then a second click on the hex), then a running race.
Two cells, two modes, two circuits: grid 0's `Race` on `01_Track` (Vineta K,
Venom, 3 laps, 8 craft) and its `Speed Lap` on `08_Track` (7 laps, solo).
`Session::launch_campaign_cell` needed **no Omega arm**: the cell's circuit id
resolves through the teams/track plugin catalogue (`Data\Plugins\teams\Definition.xml`,
38 circuits, the archive-filtered list every title's shell builds), and the
mode, class and laps mapping is HD's. What was added is a test: the mapping is
now `oag_game::campaign::launch::plan_cell`, a pure function the session applies,
and `crates/game/tests/omega_campaign_launch_ground_truth.rs` holds it to the
disc: of the 167 cells in the 16 grids that parse, **154 launch onto a circuit
the catalogue resolves (12 of them `Detonator` cells, launched as `Mode::Detonator` and
medalled on the cannon's score) and 13 (`NitroBattle`) are refused for their mode** - never a
circuit id that resolves to nothing. Grids 16-18 lack
`RequiredPoints` and are skipped by the grid reader (logged).

- **The cell's class reaches Omega's handling.** `--race --class venom` and
  `--class flash` on Omega log `GravityMul 0.85 for the venom class` against
  `1 for the flash class` on the same team, so the class is not cosmetic here
  even though `DEFAULTS.speed_classes` is `None`. Both walked cells are Venom;
  a Flash or Rapier cell was not walked.
- **AI count is not a launch option.** `AICount` is 7 on `Race`/`Elimination`/
  `Tournament` and 1 on `Head2Head`; the test asserts every Omega cell's value
  equals `Mode::opponent_count()`, so the field the mode already races with is
  the authored one. Nothing reads `AICount` for any title.
- ~~**Team Selection is skipped, not wired.**~~ Wired 2026-10-13 (`FrontEnd::team_select`), the honeycomb,
  ratings and hull preview 2026-10-14: see [Ship Select](#ship-select-2026-10-14-omega-ship-select).
  What stands from the 2026-09-30 walk: a campaign cell races the RACE page's team (`settings.race.team`;
  `Session::apply_race_team`), which is a campaign path and not Ship Select. **Chosen, not measured.**
- **Omega also ships 2048's end-race file** (checked 2026-10-06, `checked, applies, not wired`): `data09.psarc`'s `data/plugins/frontend/gui/vita/vita_EndRace_Definition.xml` is byte-identical (22,098 bytes) to 2048 v1.04's `NEWGUI/EndRace_Definition.xml`, beside Omega's own HD-lineage `endrace_definition.xml`; nothing in Omega's front-end XML names the `vita/` file. See [endrace-2048.md](../ui/endrace-2048.md#omega). The same folder's `vita_InGame_Definition.xml` (3,193 bytes, `data00`/`data08`/`data09`, loaded by `skin.xml`) is a stub: the in-race `InGame2048` screen alone, none of 2048's pause screens (**checked, differs**; [the pause menu](../ui/pause-menu.md), "2048").
- **The EndRace screens are skipped, not wired.** (Census 2026-10-06: the three copies of `EndRace_Definition.xml` author Results, Menu and Podium and no Rewards - `hd-endrace-screens.md`.) `EndRace_Definition.xml` is at
  HD's path too, and dispatching Omega through HD's loader draws `EndRace
  Results`, but `--menu-page endrace-menu` then draws no option blocks where
  HD's draws three, so the way back (`RETURN TO GRID`) is unreachable. Without
  the screens a finished race (autopilot, 3 laps of `01_Track`, about 7,000
  ticks) shows the built-in results table and the next confirm returns to
  `Main Menu`, **not** `Cell Selection` as on HD.
- **Omega draws Fury's flyer cards since 2026-10-27 and its three-way `Campaign Selection` since 2026-11-05** (`omega-chooser`): `load_omega` reads the chooser (2048, HD, Fury, each target screen's grids) with the same `Chooser::read` HD uses; `RACE CAMPAIGN` opens it on 2048, HD opens `grid0` and Fury `grid8`, and 2048 stays on the chooser with a notice (its three grids are cell-less stubs the grid reader skips, its events are `Campaign2048_Definition.xml`'s map, unread). See `docs/ui/campaign-screens.md`, "Omega: `Campaign Selection`".
- **Omega's Track Select names all 38 circuits since 2026-11-05**: 28 numbered circuits are keyed `NN_TRACK` in every copy of `entries.xml`, the ten of 2048 (`altima`, `arena`, ...) author `StringTableId="Track_Altima_0"` (`Track_Rockway_Stadium_0` for `arena`, `Track_Downtown_0` for `cathedral`, `Track_Empire_Climb_0` for `tower`) and no copy names them under their bare id, so `CircuitNames::choose` found no copy covering the list and every row read its id. `choose` now takes `(id, key)` pairs and `oag_raceplay::catalogue::Track::name_key` is the authored key; all four copies (`data00`, `data05`, `data08`, `data09`) name all 38, `data09`'s is chosen. Census, same reader: HD 28 of 28 named, no `StringTableId` anywhere (**checked, unchanged**); 2048 Vita 38 tracks, no `StringTableId`, no copy chosen by `choose` (what its own definitions and campaign file name a circuit by was read 2026-11-12, `vita-2048-circuit-names`: see "2048 Vita's circuit names" below). `omega_track_model_ground_truth::every_circuit_a_menu_reaches_has_a_name_not_an_id`.

  Re-checked 2026-11-12, the lane `omega-circuit-names` (brief: "no copy names all 38, add a per-circuit fallback"): that was already fixed, so no fallback was written. The ground-truth test above passes, and `--menu-page track-select` on `data/extracted/ps4` reads `ANULPHA PASS`. **2048, checked, applies, not wired**: on 2048 Vita (`PCSA00015`) the same `choose` finds no copy of `languages/american/entries.xml` naming all 26 raceable circuits (`debug` loader line: "none of the 3 copy/copies names all 26 circuit(s); each one shows its id"), and the `circuits:` line beside it lists all 26 as `id (id)` (`square (square)` ... `Sol_2_R (Sol_2_R)`), so none of them is named. That open question was closed the same day, below.

- **2048 Vita's circuit names (2026-11-12, `vita-2048-circuit-names`): ten of 26 named from the campaign file, sixteen keep their ids.** Census, USA package `PCSA00015` (base, patch 1.04's `data1`/`data2`, `dlc1`), `languages/american/entries.xml` in all three copies (241,385 / 241,573 / 242,632 bytes), the 26 being `data/plugins/tracks/Definition.xml`'s ten and the base package's `art/published/DLC1/plugins/tracks/Definition.xml`'s sixteen (the eight HD circuits and their `_R`):
  - **No copy names any of the 26 by its bare id** (no `square`, no `Vineta_K`; only `MP_SQUARE_RACE_FLASH`-style sentences), and no `PI_Track` authors a `StringTableId` or any key (`grep` over every `.xml` of the four archives; the Vita executable carries no `StringTableId`). So a per-circuit fallback across copies names **zero**: the three copies are identical on every circuit key, and `CircuitNames::choose` is unchanged.
  - **The ten of 2048's own** have their names in the table too (`Track_Unity_Square_0` `UNITY SQUARE`, `Track_Queens_Mall_0`, `Track_Metro_park_0`, `Track_Capital_Reach_0`, `Track_Empire_Climb_0`, `Track_Rockway_Stadium_0`, `Track_Subway_0`, `Track_Downtown_0`, `Track_Sol_0`, `Track_Altima_0`), but **no file links an id to such a key** (no `.xml` outside `entries.xml` mentions one; the keys are irregular: `Metro_park` against `Capital_Reach`). What the disc does author is `Data\xml\SP.xml`'s ten `TrackDefinition`s, whose only two fields are `M_TRACKNAME` (the `PI_Track` id, exact match 10 of 10) and `M_DISPLAYNAME` (the name, [`2048-campaign.md`](2048-campaign.md)). `oag_game::boot::roster::choose_circuit_names` now names, after the copy choice and only for a circuit it left unnamed, each track by that (`boot::campaign2048::name_circuits`) (`CircuitNames::name_if_absent`); **10 of 26 named, and each spelling equals the `Track_*_0` string** (a cross-check, not the link). The name is English-only on the disc; the `Track_*_0` strings are not read for other languages because no authored key reaches them.
  - **The sixteen HD-lineage circuits** (`Vineta_K`, `Anulpha_Pass`, `Moa_Therma`, `Chenghou_Project`, `Metropia`, `Sebenco_Climb`, `Ubermall`, `Sol_2` and their `_R`) are named by the numbered keys (`01_TRACK` `VINETA K` ... `08_TRACK` `ANULPHA PASS`), but nothing authored links them: their `PI_Track` carries `soundregister` 1 to 8 and `FEModelLocation` `...\fe\track01` to `track08`, and both numbers agree with the name of `0N_TRACK` 8 of 8, while the other `FEModelLocation`s are not numbers (`talons_junction`, `modesto`) or are wrong (the four Zones all say `track01`), and the executable holds the format `%02d_Track`. That is an inferred link, so it is **not wired** and the sixteen keep their ids; the `debug` loader line lists them ("16 of 26 circuit(s) are named by no table or campaign file and show their id"). The 2048 v1.00 front end authored four literal `TouchButton String=` names (`Vineta K`, `Anulpha Pass`, `Moa Therma`, `Chenghou Project`) that patch 1.04 dropped; not used.
  - **Read in Ghidra 2026-11-12 (`vita-2048-track-key`): the executable never makes that link**
    ([`circuit-name-key.md`](../ghidra/functions/vita-2048-eu-v104/circuit-name-key.md); v1.04 and v1.00 base agree).
    - `%02d_Track` is not a display key. It builds HD track *ids*: CRC32s of 24 HD numbers in `FUN_8100077c`, and the `-track <n>` debug option in `Game_Main`'s parser.
    - The executable names a circuit by `StringTable_LookupString` (`0x8106b86e`) of the case-folded `PI_Track` `name` attribute. On a miss it returns the name as authored. Eleven functions that hold a track agree (confidence 80).
    - `soundregister` (`+0x1c0`) and `FEModelLocation` (`+0x1e0`) are parsed (`PITrack_ParseValues`, `0x810c1f94`), but no name lookup reads them.
    - A `<Location>\stringtable.xml` or `<plugin>\stringtable.xml` would join the lookup chain (`0x810663ae`). None ships in either region. No `<StringTable>` sits outside the language plugins, and no `entries.xml` has a `VINETA_K`-style key.
    - So those sites would print `Vineta_K`, and `square` for 2048's own ten as well. What a player actually *sees* for the sixteen was open (about 50) and is answered in the next bullet: the original shows `VINETA K`: the ten's visible names come from `TrackDefinition` `m_displayName` (consumer unread), and the DLC grid draws photos, not text. A Vita3K still of a DLC-circuit label settles it.
    - **Keying the rule:** our `CircuitNames` lookup matches keys case-insensitively and falls back to the id, which is the `PI_Track` rule read in the executable. The still below shows it is not the whole of what the original draws: ours is **not** done (next bullet).
    - **Keying a nicer name:** `01_TRACK` ... `08_TRACK` are HD's own keys (HD's ids are `01_Track` ...), shipped unused. Keying the sixteen to them through `soundregister` or `trackNN` would be a **chosen stand-in, not measured**, and is a maintainer decision.
  - **2048 <-> Omega: checked, differs.** Omega's `PI_Track` authors the key (`StringTableId="Track_Altima_0"`) and its HD-lineage circuits are `NN_Track` ids, so one copy names all 38 through `choose`. The executables agree: Omega's parser `FUN_014b1330` stores `StringTableId` at `+0x278`, and its display sites (`FUN_015c6e60`, `FUN_015c9860`) look up `StringTableId` if it is authored, else the name. That is Vita's rule plus one override that Vita's parser lacks ([`circuit-name-key.md`](../ghidra/functions/vita-2048-eu-v104/circuit-name-key.md), "Omega: checked, differs"). Vita authors neither; the shared part is that both end up on the same `Track_*_0` strings for the ten (Omega via the key, Vita via the campaign file's identical spelling). Omega's 38 and HD's 28 resolve as before (`omega_track_model_ground_truth::every_circuit_a_menu_reaches_has_a_name_not_an_id`, `hd_boot_ground_truth`: both pass). Test: `vita_2048_dlc_menus_ground_truth::the_ten_circuits_of_2048_show_their_names_and_the_rest_keep_their_ids` (red on main: `square` read `square`).
  - **What a player sees, measured 2026-11-12 (`vita-2048-trackname-still`): the authored name as text, `VINETA K`, not the id.** Original: Vita3K, EU `PCSF00007` patch 1.04 with DLC1 (`data/extracted/vita/PCSF00007`), the maintainer's save copied into a private pref tree, muted, on its own weston and Xwayland. Stills under `data/reference/2048-circuit-names/` (960x544, the Vita's own grid), text read off the pixels by eye:
    - **HD Campaign, Uplift group, Race event on Vineta K** (cell `grid0_3_1`, `track="Vineta_K"`): the **event card** title reads `VINETA K`, with the class glyph and the `TARGET (NOVICE)` page (`vineta-k-event-card-race.png`). The same name sits on the **mode card drawn over the race's loading backdrop** (`vineta-k-mode-card-over-load.png`) and on the **pause screen's card** (`vineta-k-pause.png`). The Speed Lap cell of the same circuit reads `VINETA K` too.
    - **HD Campaign, Anulpha Pass** (`grid0_5_1`, `track="Anulpha_Pass"`): the event card reads `ANULPHA PASS` (`anulpha-pass-event-card.png`).
    - **2048's own**, Single Player Campaign, Time Trial event on Metro Park: the event card reads `METRO PARK` over `TIME TRIAL` (`metro-park-event-card.png`). The AdHoc game lobby's circuit panel reads `UNITY SQUARE` over `COMBAT` (`unity-square-adhoc-lobby.png`), so a second screen names Unity Square, a circuit that `PI_Track` ids as `square`.
    - **Fury Campaign, Blitzed group, a DLC2 circuit** (`grid8`, `track="Amphiseum"`): the Nitro Battle and the Speed Lap cards read `THE AMPHISEUM` (`the-amphiseum-nitro-battle-card.png`, `the-amphiseum-speed-lap-card.png`); the Elimination-or-Race card on `Modesto_Heights` reads `MODESTO HEIGHTS` (`modesto-heights-card.png`). **`THE AMPHISEUM` cannot come from the id `Amphiseum` by any transform** (underscore to space, capitals): it is the authored string, `entries.xml` `19_TRACK` / `20_TRACK` (`THE AMPHISEUM`; `17_TRACK` is `TALON'S JUNCTION` against the id `Talons_Junction`). So the id fallback is ruled out, and so is a pure re-spelling of the id. The names are looked up, by a key this lane did not read.
    - The race HUD draws no circuit name (lap, position, clock only), and the load completed in under 6 s on a warm shader cache, so no loading-screen name was photographed. Nothing anywhere showed `Vineta_K` or `vineta_k` as text.
    - Repeated on a second cold boot: `VINETA K` and `ANULPHA PASS` read the same (`*-boot2.png`). Text is deterministic, so two boots are the whole repeat. **The text read: confidence 95. That it is an authored `NN_TRACK`-family string and not the id re-spelled: 85 (Amphiseum). The key that selects it: unread.**
    - **So the answer is the first branch of the lane's falsifier: the sixteen (and DLC2's four) are named through a path that `circuit-name-key.md` did not read.** The eleven `PI_Track`-name sites there would print `Vineta_K`; what is drawn is an authored string. Candidates for the key, none tested: the event card's title setter (a `PI_Cell` card, not one of the `PI_Track` sites) keys `NN_TRACK` by an index (`soundregister`, `FEModelLocation`'s `trackNN`, or the circuit's position in a list); or the lookup folds the id through a table this lane did not find. The forward and reverse numbering is the discriminator: `01_TRACK`..`08_TRACK` and `09_TRACK`..`16_TRACK` carry the same eight names, `13_TRACK_OLD` reads `VINETA K REVERSE`, and `17_TRACK`..`24_TRACK` are the four DLC2 circuits twice each. A card for a `_R` cell (`Talons_Junction_R`, `Amphiseum_R`, `Modesto_Heights_R` and `Tech_De_Ra_R` cells are in Fury's `grid_09` and later) would read `THE AMPHISEUM` again if the key is a circuit number, or something else if it is per-direction. That is the still that decides it. The AdHoc lobby and the pause card name circuits the same way as the event card, so the consumer is shared.
    - **The fallback to the id is not what the original shows**: the thread bullet "ours already matches" is **false**. Ours (`oag-game data/extracted/vita/PCSF00007 --until card:0:grid0_3_1 --press start,cross,cross,cross`, the main checkout's debug build of 2026-11-12, not rebuilt in the lane's worktree) drew `Vineta_K` and `Anulpha_Pass` as the card titles, with a `RACE` sub-line and a Personal/Friends/Global tab strip that the original's HD cards do not carry (`ours-vineta-k-card.png`, `ours-anulpha-pass-card.png`). The ten match by construction (`SP.xml` `M_DISPLAYNAME`; ours reads `EMPIRE CLIMB` on Event 1, `ours-empire-climb-card.png`); the original's Empire Climb card was not photographed (the save is past it), and ours' Metro Park event could not be reached on a fresh profile, so no same-circuit pair exists for the ten, only the spelling rule.
    - **Not wired by this lane.** Turning `Vineta_K` into `VINETA K` in `CircuitNames` is the cheapest fix that would match the still, but without the path it is a guess at the rule (`Amphiseum` reads `THE AMPHISEUM`, so a re-spelling of the id is wrong for at least one circuit; only four circuits were photographed). Next read: the card title setter, from the `grid0_3_1` event card back to its `Label` string, and a `_R` cell's card on Vita3K.
    - **2048 <-> Omega: checked, differs.** Omega reaches `ANULPHA PASS` through authored keys it carries (`NN_TRACK`, `StringTableId`; `FUN_014b1330`), Vita reaches the same visible text by a path that is unread; no Omega original run of an HD-lineage card was made. The visible text agrees across the two titles.
  - **The path, read 2026-11-12 (`vita-2048-card-title-key`, confidence 85): a static alias table.** The cards title a circuit by `StringTable_LookupString(TrackAlias_KeyForHash(crc32(track)))`, where `g_track_alias_table` (`0x814b1578`, 102 rows `{name, reversed, key}`) pairs each 2048 track name with an HD id per direction (`Vineta_K` `01_Track`, `Vineta_K_R` `09_Track`, `Amphiseum` `19_Track`, ...; no row gives `99_Track`). Sites: `CampaignEventCard_DrawCellPanel` (`0x8105021c`, called by the HD and Fury grid cell screens, `LoadingScreen_Draw` state 4 and `Callout_Item`), `CampaignEventCard_DrawPanel`'s no-event branch and the online game list's row (`FUN_810a27f8`). Not `soundregister`: DLC2's `Amphiseum` is `10` but `19_Track`. Full table, sites and the Russian per-direction difference: [`circuit-name-key.md`](../ghidra/functions/vita-2048-eu-v104/circuit-name-key.md), "The card's key: the track alias table". Omega has the same table at `0x01990120` (its card site not read); for its 28 `NN_Track` ids the alias is the identity, and its ten 2048 circuits stay on `StringTableId`, so the **checked, differs** below still holds for that path. **Wired in ours 2026-11-12 (`vita-2048-card-names-wire`)** on the event card of an HD or Fury cell and the online game list: `VINETA K`, `ANULPHA PASS`, `TALON'S JUNCTION` (a reverse, no `FE_REVERSE` word), 28 rows in `oag_2048::frontend::CARD_TRACK_ALIASES`; see `circuit-name-key.md`, "Ours".

- **Not Omega's to fix here, seen on the walk:** (the HUD absence seen here was
  fixed 2026-10-06, `omega-hud`), (the 793 unresolved materials of `08_Track` quoted here
  were not re-measured; Anulpha's 81 were fixed 2026-10-27, below), and the
  menu backdrop is white.

### Anulpha Pass's grid floor and walls drew white: `Diffuse_Specular` is a colour sampler (2026-10-27, `omega-track-lit`)

The road and side walls of Anulpha's start grid read near-white with no panel detail (pair
`omega/anulpha-pass`, mean grey difference 61.50). Cause, by probe (the lit terms replaced one at
a time in `shade.wesl`): those draws were **unresolved**, so they bound the white albedo and the
black lightmap placeholder, and the sun alone saturated them. The material is
`cf_chevron_pulse_displacement2out` (floor, wall and lead-in variants, 81 draws): its sampler
list opens on `*_displacement_df2.gnf`, which decodes to nothing, and its colour sampler is named
`Diffuse_Specular` (hash `0x28e981a4`, read from the file's own `Diffuse_SpecularSampler`
reflection string; the others are `Emiss_Mask`, `Normal`, `Wave`, `lightmap`). The diffuse pick
([`oag_rcs::rcsmodel::psp2::material::DIFFUSE_SPECULAR_SAMPLER_NAMES`]) now names it. Result:
mean grey difference **61.50 to 38.92**, RMSE 0.329 to 0.242, edge NCC 0.23 to 0.24. The lightmap
(UVs 0..1, atlases decode) and Omega's Nova prelit were never the fault; the PS4 red-channel
coverage lead is **not** the cause here (checked: a lightmap's channels sample correctly).

### Anulpha Pass's chevron floor: HD's law by matching hashes drew it worse, the original draws it flat (2026-10-27, `omega-chevron-pulse`, negative result)

`cf_chevron_pulse_displacement2out` (33 materials in Omega's two Anulpha models, `data01.psarc`) declares
HD's chevron program's names exactly: emask sampler `0x1202d8df` (`ds_floorchevron_emask.gnf`), wave
`0x85c9fd48` (`ds_wave_c.gnf`), `Colour` `0x02ab9f07` (0, 1.49, 0), `k` `0x220cf0e6` (0.2), `offset`
`0xa6f5352f` (0.0139 forward, 1.0 reversed), and no `time`. The GCN is unread, so the trial
**inherited the program from HD by matching hashes** (a `psp2::chevron` module, **tried and not merged**, sharing `mesh::rcs::chevron_pulse`'s entry,
`slots::LIT_ALPHA` and the fifth and sixth bindings). The offset is held at the authored value (the PS4
material curves are unread), so nothing sweeps; HD's own offset runs on the scenery clock, chosen not measured.

The textures, read as pictures: `ds_floorchevron_sc` RGB holds the bright cyan circuit chevrons (the
"solid cyan bars" the generic reading drew), its alpha is brushed grey metal and **black under the chevrons**;
the emask RGB is a dim blue circuit, its alpha a white hex outline of the chevrons.

Result on pair `omega/anulpha-pass` (whole-frame mean grey difference, tool's metric): **41.04 before, 48.68 after.**
The road tiles' colour now matches (box centre (151, 204, 212) against theirs (142, 192, 197); it was (66, 26, 98)),
but the walls and road read too bright and green outline glows show on every chevron. **The original is flat:**
in `frame00`, `frame03` and `frame06` the whole road is one grey-teal and the walls are one dark teal (22, 39, 42),
with no chevron, no circuit, no hex panel, so it is not a phase of the pulse. HD's original of the same circuit
does show the chevrons and the wall panels. HD's law cannot give a flat road at any phase (`sc.a` is 0 inside the
chevrons), so Omega's program does something else with these samplers, or the emulator capture does not sample them:
**unknown**. Next: read the GCN of this program (it is not a file in the archives; the model's GPU section or `eboot.bin`),
or find another `*_displacement2out` surface in a shadPS4 frame.

Checked: `omega/vineta-k` 48.19, unchanged (no material of the family); HD pairs `hd/anulpha-pass`, `hd/talons`,
`hd-race/talons-pack` and the 2048 Vita pairs byte-identical to a build of `93e01ff43`.
**2048: checked, applies, not wired visually.** Vita's base `data.psarc` ships `cf_chevron_pulse` (16 materials in 2
models) and `dlc1` 30, with the same names; the take-condition admits them and no 2048 pair shows them, so the draw is
unseen. 2048's Vita materials carry a readable GXP program, which is the cheaper place to read the law than Omega's GCN.

**Revised 2026-11-14 (`anulpha-tone`): the original is not flat, and the albedo law is wired.** The
"flat road" was read off shadPS4 frames; the native-port original (the compare page's reference since
2026-11-01) shows brushed grey plates with dark hex outlines and cyan-glowing chevron lanes. The GCN is now
read: `cf_chevron_pulse_displacement2out` and the walls' and plates' `track_surface_displaced2out` light the
albedo's alpha and tint the specular with its colour, as HD does, and the chevron adds an unlit
`max(E.rgb, Colour * mix(E.rgb, W.rgb, E.a))` that is not HD's emissive (programs, addresses and before/after
regions: [`hd-track-tone.md`](../rendering/hd-track-tone.md#2048-and-omega-2026-11-14-anulpha-tone)).
**Ported** with 2048: `slots::LIT_ALPHA` is set on both titles; the cyan bars and purple stripes are gone
(road plate dB/G +0.180 to -0.047). The chevron's mask is drawn with `Colour` held at zero (the authored
value paints green the original never shows), and the cyan centre lanes, `cf_centre_plasma_pulse`, draw
their `E.a * wave` and their own `Uvset1` coordinate, which the PS4 reader had refused (the lane sampled one
texel): whole-frame mean grey 30.28 to **25.03**.

### The texel decode is the sRGB curve (2026-11-14, `anulpha-tone`)

**Ported from HD.** 3,115 of the 3,234 circuit pictures of `data01` and `data02` are `ChannelType::Srgb` GNF and
the lightmap atlases `Unorm`, so the texture unit decodes the pictures with the sRGB curve; ours used the `2.2`
power. `Looks::texels_srgb` selects `SceneStore::SrgbTexels` on Omega: every Omega race pair's mean grey fell or
held (Metropia 35.36 to 34.63, the largest), census and table in
[`hd-track-tone.md`](../rendering/hd-track-tone.md#omegas-texels-decode-with-the-srgb-curve-2026-11-14-anulpha-tone).
**2048: checked, differs** (its programs square the texel, no sRGB unit).

### Vineta K's shadows: the lightmap is right, the display encode is the lead (2026-10-27, `omega-vineta-shadow`, negative result)

Pair `omega/vineta-k` draws the shadowed road and wall about 65 and 52 grey too bright (boxes x 700-1300,
y 420-560 and x 1100-1900, y 340-520 of the 1920x1080 frame; original 31.3 and 38.1, ours 96.3 and 90.5).
Measured, by false-colour probes written straight to the target:

- **The channel hypothesis is false.** Road and wall draws bind a real `-lmap.gnf` (`nova_mesh_combine_TSF1`
  and kin, BC7, 2048x2048). Its rgb is a flat grey (about 0.5, channels equal) and its alpha is a binary
  sun-shadow mask with the roof's outline; decoded alpha on screen follows the arches. No PS4-wide channel swap
  is needed; the sun mask is `lightmap.a` as on HD. Vineta K's road materials carry no `VertexColour1`.
- **Ours holds a flat ambient floor the original does not show.** Shadowed half of the road box: ours rgb
  (45, 48, 56), original (6, 5, 3). Prelit alone (`2.0 * 0.5^2.5`, bias 0) already reads 68 in the road box.
  An sRGB decode of the lightmap, or gating prelit by the alpha mask, does not close it (road 54, 54).
- **The display encode does.** Replacing `oag_post::omega_tonemap`'s final `pow(1/2.2)` (recorded above as
  "chosen, not measured") with the identity: road diff 65.0 to 15.3, wall 52.4 to 0.2, shadowed halves
  (7, 8, 11) and (8, 8, 17) against (6, 5, 3) and (17, 15, 11). **But it is not landed**: whole frame 46.69 to
  59.04 and `omega/anulpha-pass` 38.92 to 59.68 (its original is pale and flat, ours goes darker still), so the
  gamma is not globally wrong. Scaling the exposure by 0.35 instead gives road 27.1 and wall 16.6, which no
  adaptive exposure can reach (`(a + b L) / L` is at least `b` = 0.498).
- Lit patches differ too: original road top decile (169, 160, 157), ours (242-248, 165-207, 83-149) - the
  12 / 6.8 / 3.2 sun saturates per channel in ours and the original's lit road is near neutral.

**2026-10-27, `omega-display-encode`:** the encode is read. The scanout is `A2R10G10B10` UNORM, filled by a
plain copy, and the video out encodes it (on shadPS4, which captured these frames, with the piecewise sRGB
curve). That curve is now wired, and it is **not** the separating term: Vineta 46.69 to 48.19, Anulpha
38.92 to 41.04. The evidence and the unmeasured leads the frames point at (Vineta's blown-out sky and the bloom after
the curve, the canopy's shadow coverage, Anulpha's chevrons) are in
[tonemap.md](../ghidra/functions/ps4-omega-eu/tonemap.md), "The output encode".

**Next (as written before the encode was read):** find why the original's low end is that dark on Vineta K and not on Anulpha: the per-circuit
`Tonemap.*` curve (`Start angle` -6.93, `End angle` -12.06, source end 1.506 here), the scanout encode of the
resolve (`FUN_017864a0` curve resolve versus the compute resolve), and the unwired shadow-light factor. Read
`tonemap.md`'s "Resolve" row against a captured Anulpha frame before touching the encode.
2048: not checked (`Material::samplers` is PS4-only; the Vita lightmaps were not probed).

**Still open:** the chevron's own program (emask, `Wave`, `Colour`, swept by `offset`) is not
wired for PS4 materials as it is for HD's `cf_chevron_pulse` (`mesh::rcs::chevron_pulse`), so
the chevrons draw as a bright cyan albedo where the original's frame shows the road plain; the
road reads purple-blue against the original's grey-teal. 2048: the Vita sampler table is not
read (`Material::samplers` is PS4-only), so whether 2048's displacement circuits carry
`Diffuse_Specular` is **not checked** here.

## Racing: a race starts, on this title's own data

**Try:** `oag-game data/extracted/ps4 --race
--hold cross --no-audio --screenshot out.png` (any directory holding both
packages' `uroot/dataNN.psarc`; they are found by name). `just play omega`
reads the same directory. `data/extracted/ps4` has held the corrected
extraction since 2026-09-29; the short-read copy was `data/extracted/ps4.bak` (deleted 2026-10-12),
and a race did not get this far on it. Census figures on this page that name
a count were measured on whichever copy the paragraph says.

`omega-race`, 2026-09-29. On the corrected extraction a race loads the real
spline, the real collision, the real craft and the real circuit geometry with
its textures, the craft rides the circuit (`grounded 1.0`, 27 units/s after 300
ticks with accelerate held, 58 units along the spline by tick 900, where it
stops against the first barrier - nothing steers it), and the frame is legible: an AG Systems hull on Tech De Ra's start gantry with the
circuit's road, barriers and banners behind it. Nothing here is a stand-in for an asset
the disc authors except what "What does not draw right" lists.

### What loads, from what, through which reader

| Layer | Omega file | Reader | Measured |
| --- | --- | --- | --- |
| Spline | `track.vex`, `WO Track` | `oag_vex::track`, **unchanged** | 39 of 39 files, `encoded_len` = payload length |
| Start position, pads | `track.vex` nodes | unchanged | `tech_de_ra`: 15 speedup and 11 weapon pad volumes |
| Collision | `track_col.col` | `oag_vex::kdcol`, **19-byte node** | 38 of 38 decode; `tech_de_ra` 12,894 vertices, 20,777 triangles |
| Circuit geometry | `track.final.rcsmodel` | `oag_rcs::rcsmodel::psp2`, **third pointer gap** | `tech_de_ra` 3,186 submeshes, 2,379,040 triangles, 0 unpaired |
| Node table | same file | `psp2::nodes`, **8-byte pointers** ([`2048-animation.md`](2048-animation.md#the-models-node-table)) | `tech_de_ra`: 1,702 nodes, all with a written matrix; 2,659 mesh objects; 1,684 node-bound submeshes placed by their bind matrix |
| Skeleton, clip | `track.final.rcsskeleton`, `.rcsanimclip` | `oag_rcs::rcsskeleton`, `rcsanimclip`, **8-byte offsets** | `tech_de_ra`: 168 nodes, 64 animated tracks, 250 s loop, 150 submeshes moving |
| Materials | same file | `psp2::material::read_ps4`, **by the header's own table** | 461 materials; 453 resolve a texture, 3,155 of 3,186 draws textured |
| Textures | `.gnf` | `oag_texture::gnf`, unchanged | as [`gnf.md`](gnf.md) |
| Craft hull | `hdships\<team>\Ship.vex` + `ship.rcsmodel` | `psp2` | `ag_systems` 12 submeshes, 22,666 triangles, 4 of 4 materials textured |
| Handling | `hdships\<team>\handlingstats.xml` | `oag_tables::handling`, unchanged | `ag_systems`: team "AG Systems", class `venom` |

The three changes, each with what it rests on:

1. **A collision node is 19 bytes, not 24 - confidence 85.** All 38
   `track_col.col` files state stride 19, and in every one the next `"----"`
   sits at exactly `0x14 + 19 * N`. `environments2048\altima` is 2048's own file
   with the node array re-encoded: past it, 415,286 bytes (leaf indices, bounds,
   triangle soup, final tag) are **byte-identical**, and the file is smaller by
   exactly `5 * 27,199`. The packed node is the wide one with the constant
   `0x000b` half-word dropped and the axis narrowed to a byte; against 2048's
   nodes on altima the children, axis, count and leaf start agree on all
   27,199, the split on 22,582 exactly and on 4,617 to one ulp. All 38 files'
   leaf runs tile their leaf array. `oag_vex::kdcol::NodeLayout::Packed`;
   [`2048-collision.md`](2048-collision.md).
2. **A circuit's model is `track.final.rcsmodel` - confidence 95, a counted
   fact.** 22 circuits use the `.final` spelling for the model, skeleton, clip,
   `.pvs`, audio and checkpoint files, four (`zone_1`..`zone_4`) use the plain
   one, and none ships both. The game tries the plain sibling first and the
   `.final` one only when it is absent (`oag_mesh::mesh::rcs::siblings::Siblings::probe`, [rcsmodel-siblings.md](rcsmodel-siblings.md)),
   so no other title's lookup changes.
3. **A PS4 `.rcsmodel` is 2048's container with 64-bit pointers.**
   - *Same container, told apart by header word `+0x04` - confidence 95:* 0 on
     all 953 of 2048's base package, `0x100` on all 1,272 of Omega's (entries,
     a few paths repeat across archives)
     (`crates/rcs/examples/omega_rcsmodel_census.rs`).
   - *Submesh record: vertex pointer 32 bytes past the index pointer, not 28 -
     confidence 90.* Twelve records on `ag_systems\ship.rcsmodel`: three in the
     Vita's 184-byte shape (a mirrored twin, 710 triangles, all a reader
     without the new gap found) and nine in the new one (the body alone is
     10,486 triangles). Every pair clears the reader's arithmetic and the
     position and 3-signed-byte normal decode at the Vita's offsets to unit
     normals (mean 0.994). Cross-title: Omega's Feisar and Qirex hulls decode to
     17 / 22,527 triangles / 5 materials and 15 / 29,643 / 4, **identical** to
     2048's ports of them (vertex counts differ - Omega re-welded them). All
     the 1,272 Omega `.rcsmodel` entries all decode, with 90 unpaired pointers in total
     (82 in the crowd rigs and `start.rcsmodel`, **closed 2026-11-20** by a fourth PS4 record shape: 8 remain, 2 on `sol`, 6 in the front-end scenes). The Vita
     corpus is unaffected by construction: a gap is only consulted for a site
     no known gap paired, and no Vita site goes unpaired.
   - *Materials, by the header's own table - confidence 95 (was 75, by name
     pointers alone, until 2026-09-30).* The `.rcsmaterial` paths sit in the
     clear and a material is a 64-bit word pointing at one; the file states its
     count at CPU `+0x70` and its table of header pointers at `+0x78`
     ([below](#a-ps4-submesh-points-at-its-own-vertex-declaration-and-its-materials-are-the-headers-own-table)).
     A submesh names its material 0x28 bytes before its record, and all
     239,108 submeshes in the package name one inside the table. On
     `ag_systems` `GlassShape` gets `glass_texture` and `FlashyFlashyShape` gets
     `emissive_bloom`.
   - *A sampler entry is 0x28 bytes, the name hash first and the texture
     pointer 0x18 in - confidence 85.* On `tech_de_ra` the hashes resolve to
     `lightmap` (291 entries, every one an `-lmap.gnf`), `Texture1`,
     `DiffuseTexture`, `Diffuse`, `Normal` and so on. **This is what picks the
     diffuse:** the road, `track_surface_displacement2out`, lists
     `track_de_ra_displacement_df2.gnf` first and read in address order the
     first frame drew the road as a white sheet; ranked by sampler role its
     `Diffuse` (`track_de_ra.gnf`) comes first.

### A PS4 submesh points at its own vertex declaration, and its materials are the header's own table

Two readings that looked settled were wrong for part of the corpus, both found
by asking why a lightmap control group was not exact.

- **A submesh record holds a 64-bit pointer to its own declaration's header at
  `+0x28` from the record start - confidence 95, a counted fact.** It resolves,
  with the stride the declaration states equal to the buffer-derived one, on all
  226,381 submeshes of the five base archives
  (`crates/rcs/tests/omega_declaration_ground_truth.rs`). The Vita's `u32` at the
  same offset resolves on every Vita submesh too (retracted 2026-10-27: the 14.2 % is not
  reproduced, see [2048-rcsmodel.md](2048-rcsmodel.md#the-flat-submesh-census-and-the-record-pointer)). Keyed by stride, one
  declaration stood for every chunk of that stride, and `tech_de_ra` has 43
  layouts over 8 strides: 285 of its 3,186 submeshes read the diffuse coordinate
  out of a tangent or colour set, and the reversed circuit decoded 218,672
  vertices to non-finite floats. Both are gone. A PS4 file no other reading of
  which is measured falls back to the stride's declaration.
- **The declaration's `lightmapUV` and the material's `lightmap` sampler agree
  on every submesh** - 50,042 have both, the rest neither, **0 in either
  off-diagonal** - and all 17.97 M decoded lightmap coordinates lie in `0..=1`.
  Two independent reads agreeing everywhere is what says the pointer offset is
  right; it is also what found the second bug.
- **A PS4 material table is the header's, not the address order of the name
  pointers - confidence 95.** The count is a 64-bit word at CPU `+0x70` and the
  address of `count` header pointers at `+0x78`; a header's `.rcsmaterial` name
  pointer is 8 bytes in. Every file's read materials are exactly that count.
  Taking every name site in address order gave the same list where none lies
  outside the table, and shifted every later material by one where one does
  (`05_ubermall`'s `diffuse_normal_specular.rcsmaterial` at `0x1a248`: 594
  sites, 593 entries; thirteen circuit models) - a submesh drew with its neighbour's
  material. `tech_de_ra`, `talons_junction` and the craft were not affected.
- The atlas decodes as BC7 at 2048x2048 with an alpha channel whose mean is 32 to
  77 of 255. What the alpha is for is not read.

**The mount order is chosen, not measured.** 10,921 of the 27,077 distinct
paths are named by more than one archive (the patch repacks most of the base)
and where the copies differ the patch's is the larger, newer one - 29 of 40
sampled race-relevant collisions differ, and every `.EnvSettings` is ~4.8 KB in
the base and ~5.6 KB in the patch. `oag_omega::EXTRA_CANDIDATES` therefore
searches `data08`, `data07`, `data05` before `data00`-`data04`. Nobody watched
a PS4 mount them.

### The material uniform and sampler table (2026-10-05, `omega-uv-scroll`)

A PS4 material header carries the same instance table as the Vita's
([2048-material-params.md](2048-material-params.md)), with 64-bit pointers and
one improvement: **a uniform states its own component count**. Read by
`oag_rcs::rcsmodel::psp2::material::ps4_params`; confidence 92 on the layout.

```text
header  +0x3c u32  count of 32-bit floats   +0x40 u64  offset of the float pool
        +0x48 u32  entry count              +0x50 u64  offset of the entries
        +0x34 +0x38 +0x44 +0x4c +0x54       zero on every material
entry (0x28 bytes)
        +0x00 u32  name hash, ~crc32(name)  +0x04 u32  kind: 1 uniform, 0x12 sampler
uniform +0x10 u64  offset of the value      +0x18 u32  components << 16 (1 to 4)
        +0x1c u32  pool, 0x1000 the float pool (no half pool on PS4)
sampler +0x18 u64  offset of the .gnf path, 0 for a sampler nothing is bound to
        +0x20 u32  sampler state (mostly 0x522800; meaning not read)
```

**What the numbers say**, `crates/rcs/tests/omega_material_param_ground_truth.rs`
over all nine archives (five base, `data05`, `data07`, `data08`; `data09` ships no model),
every `.rcsmodel` that has materials:

- **0 entries of a third kind, 0 dropped, 0 stray header words.** Every entry
  is a uniform or a sampler and the reader returns every one; a sampler is a
  `.gnf` path or a null pointer (unbound: 18,746 of them).
- **The float pool tiles exactly on all 34,423 materials**: the uniforms'
  stated component counts add up to the pool's count with none missing and none
  overlapping. A wrong stride or offset would not do this; it is the invariant
  that replaces the Vita pass's name check, because PS4 ships no shader that
  names its inputs.
- **The names are 2048's, not HD's.** Of the `uv_anim`/`uvanim`/`scroll`
  material families (35 names over `data00` to `data02`), the glow layer's
  `Emissive_UV_Offset`, `Emissive_UV_Scale`, `GlowTint` and `time`/`TimeScaler`
  resolve through the 2048 table (`mt_uvanim_diffuse_emissive*`,
  `uvanim_diffuse_emissive*`, `fc09`/`fc10_effects_vscroll_lambertalpha_emissive`)
  and the glow uniforms read the widths their names say (1, 1 and 3) on every
  one. HD's names (`V_Offset`, `VSpeed`, `speed`) appear only on
  `scrollingalpha`, `basic_uv_scroll` and the `uvanim_diffuse_emissive*_bloom`
  extras, which no consumer reads.
- **317 materials are glow layers** (both `Emissive_UV_*` uniforms plus bound
  emissive and diffuse samplers: 16 in `data00`, 128 in `data01`, 153 in
  `data02`, 20 in `data04`) and **19 are plain `speed_multipliaer` scrolls**
  (2 and 17).

**Consumed by the existing 2048 glow plan, unchanged**:
`oag_mesh::mesh::rcs::psp2::glow::plan` reads `Material::params` and
`samplers` by name hash, so filling them on PS4 is the whole wiring. A Vineta K
race reports `28 glow layer(s) and 0 plain scroll(s) off the materials' own
uniforms (rates chosen, not measured)`. Rates are the 2048 rule: `TimeScaler`
where authored (**chosen, not measured**), else `1.0`, the engine clock HD's own
records play, since 2026-10-06 (`psp2-scroll`; the authored `time` is no longer
read, see [2048-material-params.md](2048-material-params.md#what-scrolls-and-what-is-acted-on)).
**Ported 2026-10-06:** the HD-named vertex scrolls (`basic_uv_scroll`
`USpeed`/`VSpeed`, `cf_uvanim_emssive*`, `emissive_bloom`/`emissive_lights`) are
admitted by name and authored rate hashes, labelled inherited from HD in the load
report (Omega's GCN is unread, so no program confirms `time` there); 11 materials
on Anulpha Pass, up to 15 on Amphiseum, 0 on any `environments2048` circuit
(`crates/mesh/tests/psp2_glow_ground_truth.rs`).
Frames at ticks 120, 180, 240 and 300, against a tick-300 frame from before the change (not kept). Before, the panels the glow layer lights are
dull grey; after, they glow. The only pixels that differ from main's build at
tick 300 are those surfaces (438 pixels over a threshold of 24, all inside one
band of the frame). On that panel the texture is near-uniform, so the scroll is
subtle at this size.

### What one race costs in memory

2026-09-30, `lane/omega-memory`. Tech De Ra forward, `--race --hold cross
--ticks 300`, a **debug** build, Linux, peak RSS from `getrusage` (the
`--screenshot` run; the same binary that took 375 s and 9.6 GiB).

| | RGBA8 decode (before) | BC7 blocks (after) |
| --- | --- | --- |
| **Peak RSS** | **9,605 MiB** | **3,360 MiB** |
| Wall clock to the still | 375 s | 7 s |
| Track albedo, CPU (461 textures) | 6,129 MiB | 2,043 MiB |
| Track lightmaps, CPU (51 atlases) | 709 MiB | 236 MiB |
| Sky (6), CPU | 384 MiB | 128 MiB |
| Craft (3 teams), CPU, decoded once per team | 513 MiB each | 171 MiB each |
| Track GPU, summed uploads incl. mips | 8,172 + 945 MiB | 2,043 + 236 MiB |
| Craft GPU, 8 craft | 684 MiB x 8 | 171 MiB x 3 distinct (20 of 551 uploads reused) |
| Track vertices + indices, CPU and GPU each | 188 + 27 MiB | unchanged |
| Archive buffers held | none (`RssFile` 19 MiB) | none |

Every texture in the circuit is BC7 (`Thin_1DThin`) and 4 bytes a texel decoded
against 1 as the disc ships it, so the decode was the whole cost: the textures
**are** 2,279 MiB of blocks and 6,838 MiB decoded, and the source-format size
is what the upload now is. The biggest are 18 x 8192-square and 21 x
4096-square files - the craft liveries are 8192-square (85 MiB with the chain)
despite the `_1024` in their names.

Those figures count one decode per material slot. Since 2026-10-07 a model
decodes each texture file once and its materials share it, so Tech De Ra's
load holds 200 distinct textures (1.4 GiB without a sink) and uploads each file
once; see [Load time](../architecture/load-time.md).

What was tested against each lead suspect: **(a) confirmed and fixed** - BC7
passes through with the disc's own chain (`gnf.md`, "Mip chains"). **(b)
confirmed and fixed** - the lightmaps are BC7 too, 709 to 236 MiB. **(c)
confirmed** - a `Drawable` kept its `Model`, so every decoded texture lived for
the race; `Model::release_texels` now drops them once uploaded, which frees
2.3 GiB of blocks after the upload. **(d) refuted** - `RssFile` is 19 MiB; the
PSARC readers seek. **(e) refuted as a large item** - the whole circuit's
geometry is 215 MiB per copy, against 2 GiB of textures; a 2.38M-triangle
circuit's parse costs about 380 MiB transient.

**What the peak was, and the change that removed it** (`omega-texture-stream`,
2026-10-02). Every model's textures were decoded before the scene built any of
them, so the peak (3,360 MiB) was all of them at once; after the uploads RSS
fell to 2,585 MiB and stayed there because glibc keeps freed pages. A load
given a device now uploads each texture as it is decoded
(`race::TextureSink`, `oag_mesh::mesh_render::TextureSinkScope`,
`Texels::Uploaded`; the mechanism is in
[`race-load-transition.md`](../architecture/race-load-transition.md)), so a
`Model` never holds the blocks and glibc never has them to keep.

Same method, same circuit (Tech De Ra forward, `--race --hold cross --ticks
300 --screenshot`, debug build, peak from `getrusage`, steady RSS read at the
last log line; the two builds were run back to back on one machine):

| | before (main `503eb268`) | after |
| --- | ---: | ---: |
| **Peak RSS** | **3,417 MiB** | **792 MiB** |
| RSS at the still, after every upload | 2,650 MiB | 560 MiB |
| `CPU kept` in the loader's texture line (track, sky, craft) | 2,043 + 236 / 128 / 171 each | 0 |
| `GPU uploaded` in the same line | unchanged | unchanged (2,043 + 236, 128, 171 each) |
| Still, `cmp` against the before still | | byte-identical |

The shared RCS path moved too, and draws the same pixels:

| `--race --hold cross --ticks 300 --screenshot`, default circuit | peak before | peak after | still |
| --- | ---: | ---: | --- |
| Wipeout HD Fury (`hdfury-ps3-eu-dec.iso`, Dion) | 769 MiB | 667 MiB | byte-identical |
| Wipeout 2048 (Vita `PCSF00007`, Altima) | 701 MiB | 349 MiB | byte-identical |

What this does not cover, so the numbers are not read wider than they are:

- **Only a caller whose scene is built from the device it loaded through opens
  the sink**: the menus' `LAUNCH RACE` (the frame loop's own device) and the
  headless `--race --screenshot` (which now opens its device before the load).
  The windowed `--race` loads before its window exists, and `--trace-out`,
  `--dry-run` and the 2048 front-end capture build no scene from that device;
  those keep every texture on the CPU as before.
- **The GPU side is not in RSS on this machine** (an AMD Radeon RX 7800 XT, RADV, discrete). On a
  software or integrated adapter the uploaded bytes live in the process, and the
  total there is the GPU column, which this does not shrink.
- **HD's `sky.gtf` is still decoded on the CPU** (24 MiB kept in the loader
  line): the sky cube path builds its textures without going through the
  `.rcsmodel` decode sites the sink is wired into.
- The flush rule (a submit and a non-blocking poll every 64 MiB uploaded) is
  **chosen, not measured**; the peak above is with it, and no run without it
  was taken.

**Textures wider than the device allows.** 2026-10-02, `omega-talon-crash`. Two
circuits ship a 16,384-wide texture, against wgpu's default
`max_texture_dimension_2d` of 8,192: Talon's Junction's
`texturesps4/ds_floor_cs.gnf` and Modesto Heights'
`texturesps4/track_floor_diffuse_new.gnf` (both 16,384 by 4,096). On `main` a race
on either panicked in `create_texture` ("Dimension X value 16384 exceeds the
limit of 8192", reproduced on both). Every other circuit's largest side is 8,192
(the craft liveries and the sky among them), including the 10 `environments2048/*`.
Two changes, one commit:

1. **The device asks for the adapter's own texture limits**
   (`oag_mesh::mesh_render::required_limits`: wgpu's defaults with only the
   1D/2D/3D dimension limits raised, `Limits::using_resolution`). RADV and
   lavapipe both report 16,384, so on this machine both circuits go up whole.
   It also raises the clamp `oag_present::upscale::target_size` reads from the same
   limit; `Scale::RANGE` tops out at 200 %, so only a display wider than 4,096
   pixels at that scale could ever have hit 8,192.
2. **A texture still over the limit uploads from the first mip level that fits**
   (`texture::first_fitting_level`, one rule shared by the `Chain`, `Blocks`,
   decoded-blocks and synthesised-RGBA8 arms, so a 16,384-by-4,096 chain on an
   8,192 device goes up from level 1, 8,192 by 2,048). **Chosen, not measured**:
   nothing is known of how the original treated an oversize texture. A texture
   with no fitting level (a single-level one over the limit) is not uploaded and
   its slot is bound as a texture that never decoded is: the same path as a `.gnf`
   the decoder refuses (`a_gnf_the_decoder_refuses_draws_nothing`: the slot is
   `None` and the build binds its 1x1 "undecoded" white, which on a surface that
   carries real art is a plain white surface, see the Talon's Junction check
   below). That is the codebase's existing convention for a texture it cannot
   hold, not a new mechanism, and no Omega texture reaches it. Both cases are
   named in the loader line: `N texture(s) wider than this device's
   max_texture_dimension_2d of L uploaded from the first mip level that fits -
   chosen, not measured: <labels>` and, for the refusal, a warning. The census's
   `GPU uploaded` figure counts what went up, not what the disc holds
   (`texture::plan` mirrors `texture::upload`).

Forcing an 8,192 device (a sink on `DeviceDescriptor::default()`) over every
circuit that loads - 15 `environments/*` forwards, 12 reversed (`zone_*` ship no
reversed entry, `zone_3` loads to no triangles on `main` too: 27 loads, the same
27 as the census below) and the 10 `environments2048/*` forwards: Talon's Junction
and Modesto Heights, forwards and reversed, are the only ones that trim, one
texture each, from level 1; no texture is left undrawn anywhere. Checked on a
real race, `--race --hold cross --ticks 120 --screenshot`, on the RX 7800 XT
(16,384): all 27 loads reach the still without a panic (and the 10
`environments2048` ones). **What `ds_floor_cs.gnf` is**, found by making `upload`
refuse it locally (not committed) and differencing the stills: the circuit's
**road surface**. With it refused, the road in the `--ticks 300` frame is plain
white where it is textured otherwise (3.3 % of the frame differs, all of it road;
the tick-300 frame against a no-floor frame and a difference frame, not kept), so Talon's Junction draws its floor with the
whole 16,384 texture on this machine, and the white road is also what a refusal
would look like. Also viewed: `talon_t700.png`, `talon_lvp_t30.png` (lavapipe).
Tech De Ra's still, HD Dion's and 2048 Altima's are byte-identical before and
after. Tests: `texture::tests` (the rule, the descriptor's limit, an over-limit
chain / RGBA8 / BC7 on a 1,024 device, a refusal) and
`texture_stream_ground_truth`'s two Talon's Junction tests (an 8,192 device, the
renderer's own).

**Single-level `.gnf` textures per circuit.** The `.rcsmodel` loader report now
carries one clause, `; N .gnf texture(s) kept as BC7 blocks, M decoded to RGBA8
with a synthesised chain (S single-level, O off the block grid, R refused as
blocks)`, from `ModelTexture::from_gnf_form`. Over the 27 `environments/*` loads
that succeed (15 circuits, forwards and reversed where one ships; `zone_3`
loads to no triangles) and the 10 `environments2048/*` forwards, the textures the
track model names are all BC7 chains: **0 single-level and 0 off the block grid
on every circuit**, and 1 refused as blocks that still decodes on `amphiseum`
and on `modesto_heights`, forward and reversed (the report counts it but does not
name the file).
The archives do hold 846 single-level `.gnf` entries (a scratch census over
the base and patch archives **without deduplicating a path present in both**, so
an upper bound on distinct files; it also could not parse a large share of the
`.gnf` names it was handed, not looked into here), all of which decode: 99 under
`environments2048/mall`, 71 `tower`, 59 `shared`, 135 `art/published`, and the
rest front-end and particle art - but
no circuit's track model names one. Whether the 200 to 690 unresolved draws each
`environments2048` circuit reports (identical on `main`) are materials that
name them was not checked.

**A picture difference, and it is the mips.** The still differs from the RGBA8
one by a mean 0.88 of 255, 0.7% of pixels over 16, at high-contrast edges:
the renderer now samples the disc's authored chain instead of a box-filtered
one, and the GPU's BC7 decoder does the base level. The authored chain is what
the PS4 sampled.

Not Omega, but found here: **Pulse and HD peak at 8.4 and 8.2 GiB, and it is
not their assets** - it is 128 full `Drawable`s per weapon model
(`MAX_PROJECTILES`), 1,152 on Pulse and 896 on HD, each about 7 MiB. Omega
built no weapon pools when this was measured; since 2026-10-26 it builds eight bodies' worth, and a 30-tick race load peaks at 1,399 MiB resident with them against 1,392 MiB with the table empty (HD, same run: 817 MiB). See the handover thread.

### What does not draw right, or at all

- **Node-bound scenery is placed by the model's bind matrices, and nothing
  else places it** (`omega-nodes`, 2026-09-29). The PS4 node table reads
  (Vita layout, 8-byte pointers; one invariant per header pointer over all
  124,709 mesh objects, [`2048-animation.md`](2048-animation.md#the-models-node-table)),
  and a race now draws `tech_de_ra`'s 1,684 node-bound submeshes at their
  nodes. Before, all of them drew in node space, a clump inside the box
  `(-87, -25, -46)..(25, 25, 23)` around the world origin - visible from the
  start looking toward -z as a grey blob of legs and pods beside the AG
  banner (17,446 pixels differ, all inside that blob); now the camera droid
  `CamBot_New2` hovers over the stands at `(-73, -1, 89)`, the crowd entities
  stand at their nodes' translations (placed box `(-1275, -79, -1097)..(1402,
  566, 508)`), and the craft's airbrake flaps are back on their hinges at
  `(±2.16, -0.05, -4.47)` instead of under the cockpit. **The box at the bottom of
  `final_300_close.png` is not node-bound and did not move**: it is static
  circuit geometry (`node None`, road level under the start position; the 56-
  and 228-vertex `wohdtrack_0021Shape`/`SFLine_003Shape` boxes contain the
  point), drawn identically with and without the table - the earlier note that
  a node-bound prop sat under the camera was an inference that the render does
  not bear out. Nothing within 25 units of the craft at tick 300 is node-bound
  except the droid. A node with no written matrix and no skeleton entry is not drawn
  and is counted (`Report::unplaced`): 3,220 submeshes on `data00` and 15,035
  on `data04` while the skeleton did not decode, all in 2048's `trackZone`
  (Zone mode) models and a few props on `cathedral`/`mall`/`tower`. With the
  skeleton read the census finds none: the skeleton names every one (below).
- **Skeleton and clip decode, and the scenery moves** (`omega-catchup`,
  2026-09-30). The PS4's are the Vita's files with 8-byte offsets
  ([`2048-animation.md`](2048-animation.md#rcsskeleton)); `tech_de_ra` is 168
  skeleton nodes and 64 animated tracks over a 250 s loop, 150 submeshes move,
  and the 1,534 model nodes the skeleton does not name stand at their own bind
  matrix. The camera droid `CamBot_New2` behind the start line is upright and
  facing the grid at tick 0 and has dropped and turned away by tick 1800
  (from the tick-0 and tick-1800 frames, the same camera pose). The 12,310 mesh objects on nodes with no matrix of their own across Omega's base
  archives (2048's Zone models and a few props) are all on nodes its skeleton
  names, so the census leaves none unplaced; **`Report::unplaced` itself was not
  re-measured through the render path on such a model** (no Omega race loads a
  Zone model). What is
  *not* checked: the rotors' spin and the crowd against a reference (nothing
  to compare with), and the shader's node-table ceiling on the Zone models,
  which no race here loads.
- **Lightmaps are bound, and their combination is chosen, not measured**
  (`omega-catchup`, 2026-09-30; **the combination is now read, see the next
  bullet**). The atlas is each material's `lightmap`
  sampler (a 2048x2048 BC7 `-lmap.gnf`, 49 distinct on `tech_de_ra` reversed)
  and its coordinate is `lightmapUV` in the submesh's own vertex declaration
  ([the declaration pointer](#a-ps4-submesh-points-at-its-own-vertex-declaration-and-its-materials-are-the-headers-own-table)).
  The shader path is Wipeout HD's, unchanged: the prelit curve on the atlas
  colour and the atlas alpha as the sun mask. Nothing measured says that is this
  title's combination. **Two authored things it does not read:** the patch's
  `.EnvSettings` carries `"Lighting.Nova prelit scale bias power"=2.5 0.2 2`
  (2048's Vita files carry `1 0 1 0`; the formula behind three numbers is not
  recovered, so the scene keeps its 1/1 defaults), and a `Tonemap.*` block
  (`Exposure minimum/maximum/response/time`, luminance and source-colour
  coefficients) where the reader looks for HD's `HDR and Bloom.*`, so the earlier
  "no HDR/bloom block" line means "none under HD's names". The atlas alpha is 39
  to 55 % zero on the three atlases sampled, so the sun mask is not the constant
  it is on HD's circuits, and what it means here is a question for the shader.
  Frames (forward circuit, tick 600, hold accelerate, debug build):
  `fwd-t600-pre.png` (no lightmap, 11.98 % clipped
  white, mean luminance 0.619) against `fwd-t600-lm1.png` (8.32 %, 0.563), with
  HD's Tech De Ra at tick 300 (`hd-tdr-t300.png`, 4.78 %, 0.321) as the only
  comparison there is. The reversed frame at tick 300 goes the other way
  (0.99 % to 7.04 %), so the aggregate is not a verdict; the walls now carry
  baked shading where they were flat. **The purple crystalline kerbs are not
  a defect of this title's reader**: HD's own render of the same circuit has
  them. Load time is about 19 s longer in a debug build.
- **The lightmap's combination is read off the pixel shaders, and wired for
  Omega alone** (`omega-lightmap`, 2026-10-02). `Lighting.Nova prelit scale bias
  power` is three scalars, `(scale, bias, power)`, and the circuit pixel shaders
  compute **`scale * pow(lightmap.rgb, power) + bias`** on the *raw* atlas
  (`v_log_f32`, `v_mul_f32` by the power, `v_exp_f32`, `v_mad_f32` by scale and
  bias; no clamp). All 1,028 `*-lmap.gnf` atlases are BC7 **UNORM** (the same
  descriptor parse reads number format 9 on albedo and 0 on normal maps), so the
  HD renderer's `pow(lightmap, 2.2)` was a second decode. **No constant ambient
  joins a lightmapped surface**: of 4,664 unique pixel shaders that declare the
  triple, all are lightmapped and none declares `constantAmbientColour`. The atlas
  alpha is the direct sun's mask (`liveLighting0diffuse * lightmap.a * N.L`), the
  same role as on HD, so the 39 to 55 % zero is a baked sun shadow. Evidence,
  addresses and census:
  [`ps4-omega-eu/lightmap-prelit.md`](../ghidra/functions/ps4-omega-eu/lightmap-prelit.md).
  A file that omits the key keeps the executable's static default
  `(1.4, 0.2, 1.5)`; 88 of the title's 97 files author it, 20 distinct triples,
  one with a negative bias.

**Airbrake flaps swing** (2026-10-05, `airbrake-flaps`). Same builder as 2048 (`psp2::build`); the twelve `hdships` hulls in `data01.psarc` all carry both flaps.
The swing is the same `Flap::deflect` Pulse uses (`hinge * Rx(angle) * hinge^-1`,
about the hinge frame's local X), scaled by the title's own `<AirbrakeGraphics>`
`amount` and rates through `RaceView::airbrake_flaps`. **Which way it turns is
checked, not read:** the title's own `Airbrake` handler is unread, so
`psp2_airbrake_flaps_ground_truth` asserts the physical claim Pulse's recovered axis makes (a positive
deflection raises the flap and flares it outward, both sides) on every Omega hull in `data01.psarc`. Only
the player's craft swings, as on Pulse; a rival's flaps stay stowed.

  **Wired** in `oag-render` as `Light::with_nova_prelit` and `mesh.wgsl`'s
  `nova` branch, **only for a PS4 container** (`GeometryKind::Ps4`, by
  `psp2::is_ps4`): 2048 authors the key as `1 0 1 0` and its Vita shader is
  unread, so 2048 does not take it. The branch is gated on the lightmap bit, so
  the bias never lights an unlit draw. Tests: `omega_nova_prelit` (the quad
  reads `2.5 t^2 + 0.2`, no ambient leaks in, an unlit draw is unchanged, HD's
  curve is unchanged with the flag off; it fails if the bias is dropped) and, on
  the real disc, `omega_lightmap_ground_truth`.

  Frames (`oag-game --no-audio --race --hold cross`, debug, 1440x816,
  before from `main` `679bd898`):

  | frame | clipped white before -> after | mean luminance |
  | --- | --- | --- |
  | Tech De Ra forward, tick 300 | 8.13 % -> 6.72 % | 0.566 -> 0.529 |
  | Tech De Ra forward, tick 600 | 12.24 % -> 10.47 % | 0.585 -> 0.598 |
  | Tech De Ra reversed, tick 300 | 5.47 % -> 3.88 % | 0.522 -> 0.491 |
  | Anulpha Pass forward, tick 300 | 2.75 % -> 2.86 % | 0.293 -> 0.315 |
  | Anulpha Pass reversed, tick 300 | 4.27 % -> 4.46 % | 0.343 -> 0.364 |
  | HD Tech De Ra, tick 300 | 5.16 % -> 5.16 % | **byte-identical** |
  | 2048 default circuit, tick 300 | 4.97 % -> 4.97 % | **byte-identical** |

  **Sweep of all twelve forward circuits** (tick 30): every one binds its
  lightmaps with **0 unresolved** and reports its own triple
  (`omega-lightmap` load logs), so no lightmapped draw falls back to the bias
  without an atlas; the outlier is Sebenco Climb's `16 * lightmap^12 + 0.96`,
  which renders brighter (0.375 -> 0.461 mean luminance, 0.43 % -> 1.65 %
  clipped) and plausible. The Zone rig rebuild (`ZoneGrade::light`) now carries
  the Omega flag through, though Omega ships no Zone palette today
  (`zone_palette: None`), so no Omega Zone race reaches it. (2026-11-19: an HD-lineage circuit does take
  HD's title-wide table through `ZONE_FALLBACK`; Omega's own ladder was read and is HD's, so zone 5 is stage 4,
  see [zone-speed-class-ladder.md](../ghidra/functions/ps4-omega-eu/zone-speed-class-ladder.md).)

  Read by eye (`*-before-after.png`): Tech De Ra's blown-white upper walls now
  carry panel shading, and Anulpha Pass's walls and floor take a deeper baked
  shadow, which is what dropping the 1.0 constant ambient and the second sRGB
  decode do. **The clipped share does not rise, against the expectation that a
  2.5x scale would clip more**; it is not a verdict either, because the
  original writes fp16 and tonemaps afterwards.

  **Not wired, recovered:** the shadow-light factor on the prelit term
  (`0.75 + 0.25 * sat(N.L_shadow) + ...`, general in presence across the 4,664
  shaders and fixed in form in only 1,780), left at 1 (**chosen, not
  measured**); and the vertex-colour variant of the same curve (`NOVAColor`),
  which needs role bits the PS4 material container does not yet provide.

  **The `Tonemap.*` block is applied** (`omega-tonemap`, 2026-10-05). Its consumer
  is `ToneMap_ApplyEnvSettings` (`0x01620980`), which reads the block through the
  environment object (the earlier absolute-address search could not see that), and
  the law is the executable's own, read off GCN microcode and checked by emulation:
  a Rec.601 mean luma of the frame, averaged over `round(time * 60)` frames,
  `LAvg` stepped toward it by at most `response / 60` a frame, an exposure
  `clamp((a + b LAvg) / LAvg, min, max)`, and a **cubic Hermite curve** from `(0, 0)`
  to `(t1, 1)` with end slopes set by the two angles, applied **per colour channel**
  inside the 4x MSAA resolve. Bloom is added after it, from a four-level chain over the
  curved frame, built from the circuit's `Bloom.*` keys and drawn since 2026-11-04 (its per-material mask is not). Evidence, scores and what is
  not read: [`ps4-omega-eu/tonemap.md`](../ghidra/functions/ps4-omega-eu/tonemap.md).
  The answer to "does Omega render differently by console" is: only in where the
  curve runs (the 4x MSAA resolve on a base PS4; a compute resolve with the same
  coefficients under the Pro's checkerboard mode, whose sample setup is not read); HDR video out switches to the
  `TonemapHDR.*` twins and a curve ending at 40.0 instead of 1.0.

  **Wired** as `oag_post::omega_tonemap`: an Omega race (a PS4 circuit
  whose file has the block) draws into the linear `Rgba16Float` scene target and
  the chain runs the law, then the `pow(1/2.2)` display encode every linear target
  here ends on (**chosen, not measured**: the scanout format is unread; the HDR
  twin's 0..40 range is why the curve's output is taken as linear light). Also
  chosen: per pixel rather than per MSAA sample, the first frame starts settled,
  no readback latency, brightness at the middle setting. Tests:
  `the_law_reproduces_the_original_coefficient_shader` (numbers produced by the
  original shader's own instructions on an emulated lane),
  `the_chain_draws_the_law_on_a_real_device`, and on the disc
  `omega_lightmap_ground_truth`. Frames (`oag-game --no-audio --race --hold cross`,
  debug, 1440x816, before from `main`):

  | frame | clipped white before -> after | mean luminance |
  | --- | --- | --- |
  | Tech De Ra forward, tick 300 | 5.97 % -> 0.01 % | 0.529 -> 0.412 |
  | Tech De Ra forward, tick 600 | 9.31 % -> 0.14 % | 0.598 -> 0.453 |
  | Altima (2048 heritage), tick 300 | 7.58 % -> 0.09 % | 0.713 -> 0.566 |
  | Altima (2048 heritage), tick 600 | 13.51 % -> 3.08 % | 0.585 -> 0.501 |
  | HD Tech De Ra, tick 300 | 5.26 % -> 5.26 % | **byte-identical** |
  | Pulse PSP, tick 300 | 4.35 % -> 4.35 % | **byte-identical** |

  Read by eye: Tech De Ra's blown ship hull, banner and upper walls carry their
  detail. **Altima's road is a flat colour in both frames**: before, it clipped
  to white; after, it is a flat pink with no texture. That was a defect the clip
  was hiding, not the curve: **closed 2026-10-19 (`omega-pink-road-sun`), see
  "Altima's road" below**.

- **`track.final.pvs` is read and culls** (`omega-pvs-sound`, 2026-09-30). It is
  [HD's `track.pvs`](hd-pvs.md) in the 2048 lineage's dialect: little-endian,
  header word 2 is `1`, the bitmap is `ceil(chunks / 8)` bytes, and a chunk is a
  **`.rcsmodel` mesh object** (`scene.meshes`, 2,659 on `tech_de_ra`) and not a
  submesh (3,186). All **44** shipped `.pvs` files (34 `track.final.pvs` and 10
  zone-mode `trackzone.pvs`, over `data00`, `data01`, `data02`, `data04`) parse, declare exactly their model's mesh-object count, and fill
  their table to the byte (`psp2_pvs_ground_truth`); the four `zone_N` circuits
  ship a `.pvsxml` and no `.pvs`, so they draw every chunk and the load report
  says so. A cell sees on average 30 to 78 percent of a `track.final.pvs`
  circuit's chunks (16 to 51 percent for `trackzone`; HD's is about 33), so
  the reduction is smaller than HD's on the open circuits.
  **The name follows the model that was found**: the loader used to ask for
  `track.pvs` / `track_reversed.pvs` (derived from the `.vex`), which the archive
  does not have, so *neither* direction of any circuit was ever read.
  Measured at the same tick with `--pvs false` and `--pvs true`
  (debug build, 1440x816, `tech_de_ra`, hold
  accelerate; `false` is frustum culling alone, which is on by default - the whole
  scene is 3,198 draws and 2,379,040 triangles):

  | frame | draws `false` / `true` | triangles `false` / `true` | pixels that differ |
  | --- | --- | --- | --- |
  | forward, tick 300 | 1,271 / 757 | 939,301 / 619,822 | 2,384 (0.20%) |
  | forward, tick 600 | 1,283 / 759 | 952,672 / 620,986 | 2,107 (0.18%) |
  | reversed, tick 300 | 943 / 708 | 718,371 / 618,388 | **0** |
  | reversed, tick 600 | 944 / 707 | 725,503 / 625,349 | **0** |
  | 2048 (Vita) Altima, tick 300 | 616 / 447 | 145,779 / 98,032 | **0** |

  The forward frames differ in one 57 x 77 pixel block on the right-hand wall
  (`fwd-t300-crop.png`): with the PVS off a dark slab lies across the pipe wall,
  with it on the wall is continuous. **It is one object**, found by logging the
  draws the PVS rejects and the frustum passes and projecting them onto that
  block: mesh object 2412, `tracksurface:wohdtrack_0022Shape`, two triangles of
  `track_surface_displacement2out` in world space, **set in none of the
  circuit's 882 cells** (56 of its 2,659 mesh objects are set in none). That
  the original never draws it is an inference (confidence 75): no Omega
  executable has been read, and HD's PS3 loader was read to use one cell's
  bitmap as the frame's visible set, so a chunk no cell sets is one that engine
  never draws. Nothing else the PVS-off frame draws differs
  (`omega_pvs_placement_ground_truth`).
  **The cell padding is HD's** (`CHUNK_PAD`, `CHUNK_TRUST_RADIUS`, chosen for
  HD's 12-unit cell spacing; Omega's are about 5.6 apart) - **chosen, not
  measured** for this title. A debug build still spends minutes decoding the
  453 textures; culling does not shorten that.
- **Sky turn and fog are ported from 2048** (2026-10-06): the six-face sky
  cube is turned by `Sky rotation` and the circuit's `Lighting.Fog colour`
  draws, the curve inherited from HD ([envsettings.md](envsettings.md)).
  Omega's altima road drew flat pink in a start frame (pre-existing, not part
  of that change; closed 2026-10-19, below).
- **Altima's road drew flat pink because it bound the wrong sampler**
  (`omega-pink-road-sun`, 2026-10-19). `road_alphaspec_normal_emissive_panelmap1`
  (79 sampler entries over Omega's circuit models, all of Altima's road) lists
  `track_a_floor_panel_df` ahead of its `DiffuseMap`; that file is a 1024x256
  single-channel `R16F` panel map, which `ModelTexture::from_gnf_form` does not
  decode (a signed distance field, decoded on
  [panel-displacement.md](../ghidra/functions/ps4-omega-eu/panel-displacement.md)), so every road draw was unresolved and drew the lit colour alone. **It
  was not a normal map** (the lead from `hd-finish-line-purple`): the road is not
  in the `track_albedo` census at all, and the census counts did not move.
  `Material::diffuse_texture` now takes the sampler named `DiffuseTexture`,
  `DiffuseAlphaMap` or `DiffuseMap` (names read off the hashes, `0x38c2e35a` is
  `DiffuseMap`) and only then the first texture in file order; `glow.rs` shares
  the list. Altima unresolved draws 690 to 449
  (`omega_altimas_road_binds_its_diffuse_map`, ratcheted); the 449 left are other
  causes, not taken apart. Altima, Mall and Tower join the census at 0 faults.
  2048: **checked, differs** - the Vita road is `fc16_phong_alphaspec_normal_emissive`
  with `track_a_floor_ds` as its `DiffuseMap` first, so its binding is unchanged.
- **The `tarmac_*` road families draw the wrong coordinate set: Rockway Stadium's black road patch**
  (`omega-arena-square`, 2026-11-12, confidence 85 for arena, 55 for the other seven circuits; fixed
  below).
  The patch seen around the craft on `arena` is the road, not a light or a shadow. Method: the
  same pose on 2048's own arena (Vita) has a normal grey road; `OAG_SKIP_MATERIAL=road_a_ds`
  removes the black and shows the floor under it; an unlit render (`OAG_ALBEDO_ONLY`, wired into
  the psp2 build for the test) stays black, so the road's *picture* is dark, not its light; the
  texture itself decodes to grey asphalt with a yellow line and lane dashes (`road_a_ds.gnf`,
  4096 x 4096), so the coordinates are wrong. The road material is
  `tarmac_markings_dirt_markingsnormal_emis` (and its siblings `tarmac_markings_dirt_markingsnormal`,
  `tarmac_dirt`, `tarmac_dirt_emissive`): their vertex programs name `uvMarkings` and
  `uv1_uvDirt` (`oag-rcs` `gcn_dump --summary`), and the vertex declaration of an arena road
  submesh lists `uvMarkings` (half pair, offset 20), `lightmapUV` (24), `uv1_uvDirt` (four halves,
  28) and `uv1` / `uvDirt` (both at 32), **no `Uv1`**. `own_diffuse_texcoord` therefore takes its
  confidence-70 fallback, the first half pair that is not the lightmap's: `uvMarkings`, a unique
  unwrap (u 0.79 to 0.99, v 0.01 to 0.68 over the whole 250-unit road, so 33 by 94 texels under
  the craft: the texture's dark top-right strip). The real tiling set is `uv1_uvDirt`'s first pair
  (u -44 to 41, v 0 to 1 on arena). **Probe** (a one-line local change, not committed: read
  `uv1_uvDirt` at offset 28 as the diffuse coordinate): the road draws grey asphalt with the yellow
  kerb line, lane dashes and the cyan edge line, as the original's frame does (whole-frame RMSE
  to the original 0.272 to 0.229 on the compare pair, the HUDs excluded from neither).
  **The declaration's own `uv1` entry is not the answer:** it sits at offset 32 with `uvDirt`, the
  `.zw` pair of `uv1_uvDirt`; reading it as the diffuse coordinate draws a smeared, dash-less
  surface (RMSE 0.077 from the offset-28 frame, read by eye against the original), so the
  coordinate is `uv1_uvDirt`'s first pair at offset 28, not "the file's `uv1`" by name.
  **Reach:** 661 submeshes over 8 of the 10 `environments2048` circuits (mall, tower, square,
  park, cathedral, subway, arena, bridge; not altima, not sol) name `uv1_uvDirt` and no `Uv1`
  (`crates/rcs/examples/psp2_uv_census.rs`); Omega's HD-era circuits do not. Subway's frame moves
  with the probe (RMSE 0.073 between the two renders) toward the original's reddish rails;
  Mall's barely (0.013), Square's not at all (its road is another family, below). The pixel
  program's combine is read in "The `tarmac_*` combine" below.
  2048: **checked, differs** - none of 2048's arena, square, altima or mall materials is a
  `tarmac_*` family; its roads name `Uv1`, which is why 2048's own arena draws a normal road.
  **Wired** (2026-11-12, `omega-tarmac-uv`): `VertexDecl::own_diffuse_texcoord` takes
  `uv1_uvDirt` (`0x273a_b13e`, four halves) after a named `Uv1` and before the confidence-70
  file-order rule, so the first pair at offset 28 is the diffuse coordinate; the declaration's own
  `uv1` entry (offset 32) is not read. `diffuse_texcoord_is_fallback` no longer counts these.
  Unit tests on an arena-shaped declaration (offsets 20, 24, 28, 32; red on `uvMarkings` before the
  change). Ground truth, `crates/rcs/tests/psp2_flat_census_ground_truth.rs`: 661 submeshes name
  `uv1_uvDirt` and no `Uv1`, **all of them `tarmac_*`** (122 in `data00`: mall, tower; 539 in `data04`:
  arena, bridge, cathedral, park, square, subway), and for every one the reader's coordinates equal
  a fresh decode of `uv1_uvDirt`'s first two halves off the vertex buffer. The range is not the
  thread's "u -44 to 41, v 0 to 1" (that was one reading near the craft): the whole of arena's
  `track.final.rcsmodel` road material `tarmac_markings_dirt_markingsnormal_emis` spans u -53.9 to
  56.9, v -4.0 to 3.5, its sibling `tarmac_markings_dirt_markingsnormal` u -2.8 to 3.7, v -4.2 to 3.6;
  the test asserts the road's u span exceeds 80 (`uvMarkings` spans 0.2). One road submesh (787
  triangles) that drew flat on `uvMarkings` now reads a real coordinate (`omega_data04`'s flat pin
  36,178 to 36,177). Compare pair `arena-2048` re-rendered on the branch (a private copy of the
  pair, its recorded flags): whole-frame mean grey 49.25 to 35.30, RMSE to the original 0.261 to
  0.216; mean RGB of three road boxes (1920 x 1080 pixels; left of the craft 280,500 340x180,
  right 1350,560 400x200, ahead 720,330 400x160): original (86,122,114) (131,138,128)
  (124,132,123), before (29,44,39) (163,172,166) (66,85,77), after (74,115,108) (139,147,137)
  (95,129,120). The road is grey asphalt with the kerb line and dashes. The markings and dirt
  layers are wired by `omega-road-markings` (next entry).
  2048: **checked, differs, and the ground truth is unchanged**: the same census finds 0
  `uv1_uvDirt` submeshes on every Vita archive (base, dlc1, dlc2, both patch archives) and every
  Vita pin, `empire_climb_is_pinned`'s `own_fallback` 4,761 included, passes unchanged.
- **The `tarmac_*` pixel programs combine four pictures: diffuse, then dirt, then markings, plus an
  emissive** (`omega-road-markings`, 2026-11-12, confidence 85). Read from decoded GCN
  (`gcn_dump <data00.psarc | data05.psarc> data/environments2048/shared/objectlibrary/materials/tarmac_markings_dirt_markingsnormal_emis.rcsmaterial --at 0x2e44`,
  which now also prints `sample @<offset> reads API slot <n>` from `Program::sampled_slots`, the
  usage-record placement `signed_textures` uses). Vertex program `@0x29c4` (60 instructions) writes
  `param2 = uvMarkings`, `param3 = uv1_uvDirt.xy`, `param4 = uv1_uvDirt.zw`; the pixel program's
  `attr1`, `attr2`, `attr3` are those three. The unlit program `@0x2e44` (86 instructions in `data00`,
  94 in the patch `data05`; the combine is identical, the patch adds a `max(w, 0.5)` clamp on the
  fade divide) samples API slot 0 (`DiffuseAlphaMap`) on `attr2.xy`, slot 2 (`Dirt`) on `attr3.xy`,
  slot 1 (`Markings`) on `attr1.xy`, slot 3 (`EmissiveTexture`) on `attr2.xy`, and computes
  `c = diffuse.rgb; c += dirt.a * (dirt.rgb - c); c += markings.a * (markings.rgb - c);
  out = fog(emissive.rgb + constantAmbientColour * c)`. The lit programs (`@0x3f44`, 200
  instructions in `data00`) take the same three coordinate sets for their sample (`attr2` normal and
  diffuse and emissive, `attr1` markings and `MarkingsNormal`, `attr3` dirt), run the same two lerps
  over the diffuse rgb in the same order (dirt first, markings on top, each weighted by its own
  alpha), and use the diffuse alpha and the layers' alphas only to weight specular
  (`MarkingsSpec`), which this reader does not draw. Sibling `tarmac_markings_dirt_markingsnormal`
  (unlit 83 instructions) samples the same three slots without the emissive; `tarmac_dirt` samples
  slots 0 and 1 (`Dirt`) only; `tarmac_dirt_emissive` adds the emissive. Their combines were read
  by sample order and slot only, not instruction by instruction.
  **Wired** (`omega-road-markings`, 2026-11-12): `oag_mesh::mesh::rcs::psp2::tarmac` admits
  `tarmac_markings_dirt_markingsnormal` and `_emis` by name and by both samplers (`Markings`
  `0x4a5bd928`, `Dirt` `0xc7aab177`); `SubMesh::markings_texcoords` / `dirt_texcoords` read
  `uvMarkings` (offset 20) and `uv1_uvDirt.zw` (offset 32 on arena) from the record's own
  declaration (census: all 661 submeshes agree with a fresh decode). The pictures ride the fifth
  (markings) and sixth (dirt) bindings, the `EmissiveTexture` the fourth (added after the light at
  the tiling pair, weighted by the glow entry's tint `x`), a glow-table entry of mode
  `PROGRAM_TARMAC` (14) selects the combine in `shade.wesl`; the markings pair rides `texcoord2`
  and the dirt pair `colour.xy` (`colour.zw` = 1, tint forced white on the mode) because the
  vertex has no fourth pair. **Not wired:** `tarmac_dirt` and `tarmac_dirt_emissive`, the specular
  weighting by `MarkingsSpec`, the roads' normal-map blend with `MarkingsNormal`. (The shadow-light factor is wired since
  `omega-2048-road`; the lobe is read and waits for the track's shadow term: "The tarmac roads' lit programs" below.)
  **Result on `arena-2048`** (private copy of the pair, recorded flags, 1920 x 1080; mean RGB of
  the three boxes): original (86,122,114) (131,138,128) (124,132,123); before this change
  (74,115,108) (139,147,137) (95,129,120); after (72,111,105) (117,125,118) (83,114,107). The
  dirt splat adds the crack and stain structure the original shows, but the road is **darker than
  the original** in the right and ahead boxes (right was 8 over, is now 13 under; ahead was 29 under, is 41
  under) and whole-frame RMSE moves 0.216 to 0.218: either the dirt's weight is too strong in our
  domain (the splat is grey 33 to 132 with alpha mean 96/255, mixed here on gamma values) or the
  original's diffuse is brighter than ours before the layers. **Isolated** (a temporary per-layer
  weight and a linear-domain switch, not kept): dirt alone gives the whole move (boxes (72,111,105)
  (117,125,118) (83,114,107)); markings alone change nothing in these three boxes (no painted symbol
  there); dirt mixed in the linear domain gives (73,113,106) (122,130,121) (85,116,109), still under
  the original's (86,122,114) (131,138,128) (124,132,123). **The programs do not square or root the
  taps**: the unlit combine is plain `v_sub`/`v_mac` lerps on the sampled rgb (read in full, see
  above), so a gamma-domain blend is not what the program asks; whether the hardware decodes the
  `.gnf` as sRGB before the lerp is not read. The box 720,330 was already 29 red under before the
  layers, so part of the gap is not the layers.
  **Shipped state:** the markings layer and the emissive are on (neutral where there is no decal);
  the **dirt layer is off by default**, switched on with `OAG_TARMAC_DIRT=1`
  (`psp2::tarmac::DIRT_SWITCH`, the entry's tint `y` read by `shade.wesl`). **Open, precisely:** with
  dirt on, arena's right and ahead boxes read darker than the original even with a linear-domain
  mix. Next read: whether the hardware decodes the `.gnf` dirt and diffuse textures as sRGB before
  the lerp (the T# format's number type, the sampler state), then the ahead box's pre-existing
  29-red gap (box 720,330 reads (95,129,120) before any layer against the original's
  (124,132,123)). The painted symbols (`KEEP CLEAR`,
  arrows, dashes, in the atlas) are not seen in this frame. Confidence in the combine 85; in the
  rendered match, none: not isolated further.
- **Unity Square's road is not black; what it lacks is the panel emissive and the displacement**
  (`omega-arena-square`, 2026-11-12, confidence 55). Square's road is
  `road_alphaspec_normal_emissive_panelmap1` (and `_mask`), the family of Altima's road above; its
  pixel program (428 instructions) reads `Cg_PanelDisplacement_1In1Depth`, `...SlopeWidth`,
  `...Aspect`, the panel map, `DiffuseMap` and `EmissiveMap`. Ours draws the diffuse panels and the
  baked sun shadow wedge (the original has the same wedge); the original also draws long cyan
  glowing lines along the panel edges on Square and on Queens Mall, which ours does not. The
  family's emissive texture (`track_a_floor_e.gnf`, 512 x 128) is one thin cyan-grey line, which
  the panel displacement would stretch along the grooves: the lines are probably that layer. Ours
  draws no such lines; which code path would draw this family's `EmissiveMap` (the
  `glow.rs` sampler list names `EmissiveMap`, so the family may be admitted and drawn
  without the displacement) was not read. Not isolated (no run draws the family's emissive alone).
  **The program was read on 2026-11-16**
  ([panel-displacement.md](../ghidra/functions/ps4-omega-eu/panel-displacement.md)): the emissive
  map is sampled at the ray-marched coordinate and added unscaled, so the lines are that layer
  seen through the displacement (confidence 85 for the program; the picture not rendered).
- **Omega's diffuse sun is its colour times `Lighting.Sun diffuse intensity`**
  ([sun-intensity.md](../ghidra/functions/ps4-omega-eu/sun-intensity.md),
  2026-10-19, confidence 80): authored 0.3 to 20, ignored until then. The
  earlier note here, "its sun (`[4.00, 2.33, 0.82]`...)", was Tech De Ra's colour
  alone; the sun is `[10.1, 5.9, 2.1]`. 2048 carries no intensity key: checked,
  differs. **Omega's `original` shadow tier does not draw on the ground**: see
  [shadows.md](../rendering/shadows.md) "Omega's `original` was tried". Omega's
  ship-shadow keys against 2048's, 2026-10-19: **checked, differs** (no `Ship
  shadow direction`, no debug gates; `shadowCoeffs` and `CastShipShadows` are
  Omega's own), `original` stays `Unbuilt` - [ship-shadows.md](../ghidra/functions/vita-2048-eu-v104/ship-shadows.md).
- **`.EnvSettings` is read through 2048's reader** and its sun colour was read
  as the diffuse colour alone (above). The patch's `data08` copy does carry the
  `Bloom.*`, `BloomHDR.*` and `Tonemap*.*` blocks (read 2026-11-20, `omega-frame-offset`; the earlier "no HDR/bloom block"
  meant none under HD's key names), and the chain is drawn since 2026-11-04.
- **Music plays** (`omega-music`, 2026-10-05): a race plays 17 of the 29
  `PI_Music` songs, the front end plays its own loop. A song is a Wwise
  `Music_Track` state: `Set_Music_Track_<location>__frontend` sets it, the
  music switch walks to one segment, and the song is that segment's 7 to 11
  stereo stems summed (unity, scaled down when it would clip - chosen, not
  measured). **Not played, by name:** location 0 (no event in the bank). The
  eleven songs that are one eight-channel file play folded to stereo by a
  standard 7.1 downmix (**chosen, not measured**: how the original folds is
  unread, [`wwise.md`](wwise.md#music-a-music_track-state-per-song)). A
  headless `--race` WAV is the first song, `Shake It (WipEout Omega
  Instrumental Edit)`, at the music bus's 0.44 gain (correlation 0.9999999 with
  the offline mix). The front end's loop is `Game_FLOW` `Menus`, **not
  watched in a running original**. See [`wwise.md`](wwise.md#music-a-music_track-state-per-song).
- **Sound effects are read, not played** (`omega-pvs-sound`, 2026-09-30). The banks are
  **Audiokinetic Wwise**, bank generator version 118, not the PSP's `SBlk`, so
  `sfx: <name> not loaded: unsupported bank version 1145588546` is still what a
  race prints: nothing wires Wwise events to this project's cues.
  [`wwise.md`](wwise.md) has the container, the event-to-media chain (0
  unresolved media over `data00` and `data08`) and the `.wem`: **every one is
  Sony's ATRAC9**, and every one decodes in process (four-, six- and
  eight-channel included since 2026-11-19, [`atrac9.md`](atrac9.md)) and agrees
  with FFmpeg to one LSB up to polarity, the LFE of some 7.1 media aside.
  The ten `.bnk` in `data02`, which an extension-based census had counted
  as Wwise, are the old `SBlk` container. One Tech De Ra cue plays through the mixer to a WAV.
  **How the executable names a cue is read** (`omega-wwise-sfx-re`, 2026-11-02): it posts
  `<cue>[_PC]__<bank>`, so the start is `_321_NGP__Weapons_NGP` in `English(US)/Weapons_NGP.bnk`; the
  sixteen 32-byte banks are legacy names it never loads
  ([wwise-sound.md](../ghidra/functions/ps4-omega-eu/wwise-sound.md)). **The start plays** those two events
  as their timelines (17 and 6 voices); no other effect does yet.
  **Particles play** (`omega-particles`, 2026-10-05): Omega keeps its effects in
  `Data\particles` and `Data\particles2048` (not `Data\Psys`), with `.gnf`
  sprites; a race reads `particles2048` on an `environments2048` circuit and
  `particles` elsewhere (chosen, not measured - `docs/ghidra/functions/ps4-omega-eu/particle-paths.md`).
  The Rocket craft blast plays its file's sprite templates (2026-11-26, ported with HD's trigger
  and 2.0 x depth cap inherited): `glow_longer` and `Glow` from `particles2048`, none from
  `particles` ([rocket-craft-blast.md](../ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md#the-other-titles)).
  The blend-class-8 distortion emitter (`shockdistort`) is **drawn** (2026-10-06, `heat-haze-3`): an offset target the Omega tone-map composite resamples the scene by, strength the emitter's own `+0xc84` float (`ghidra/functions/ps4-omega-eu/heat-haze.md`, `pob.md`); the heat-haze set has no trigger yet
  (2026-10-06, `pob.md`): the explosions play their other emitters, and the effects
  Omega never authors are left out of its table. The two HUD-atlas WARN lines are gone (2026-10-06, `omega-hud`: the atlases
  are `.gnf` and now resolve). Still silent: the
  blob shadow is the disc's own `Ambient_Shadow.gnf` in a Zone race (2026-10-06, `zone-shadow`, `shadows.md`) and this project's generated falloff for every other craft, which ship none.
- **Reversed circuits get their collision** (`omega-catchup`, 2026-09-30).
  `kdcol::sibling_name` used to pair only `track.vex`, so a reversed race
  loaded no collision at all. The reversed name is `track_col_reversed.col`
  (`_col` before `_reversed`), beside `track_reversed.vex`, on all twelve of
  `data01`/`data02`'s reversed circuits
  (`every_reversed_circuit_pairs_with_its_reversed_collision`). `tech_de_ra`
  reversed: 12,880 vertices, 20,757 triangles, 29,104 k-d nodes, four colliders,
  and the craft rides it (`grounded 1.0`, 27.1 units/s after 300 ticks). **This also changes 2048, on
  purpose.** Its packages ship `track_col_reversed.col` for 12 of their
  reversed circuits (3 in the base package, 5 in `dlc1`, 4 in `dlc2`; an earlier
  version of this note said none, from a listing cut short). Of the 12
  `track_reversed.vex` a race can name, five author their collision in the
  `.vex` (Anulpha Pass, Metropia, Talon's Junction, Ubermall, Vineta K) and the
  sibling is never asked, so those are unchanged; **five had no collision at
  all and now load the authored one** (Amphiseum, Modesto Heights, Sebenco
  Climb, Sol 2, Tech De Ra - the last with the same 12,880 vertices as Omega's,
  and a 2048 race on it grounds, `grounded 1`, 23.1 units/s at tick 300); two
  (Chenghou Project, Moa Therma) do not load for an older reason (`decoded to no
  triangles`). Pulse, Pure and HD author collision in the `.vex` and are
  unaffected. The "before" for those five is read from the old `sibling_name`,
  not observed: this was not re-run on the old tree. If 2048 must stay
  byte-identical, the pairing is one branch in `kdcol::sibling_name` to gate on
  the title; it was left on because a reversed race with no ground is a defect
  and the file is the disc's own.
- **Defaults are chosen, not measured:** `tech_de_ra` and `ag_systems`. Zone,
  boost, speed classes and every per-team variant table stay `None`.

Reproducers: `omega_rcsmodel_census.rs`; and the ground-truth suite
`crates/raceplay/tests/omega_race_ground_truth.rs`, which skips (loudly, even under
`OAG_REQUIRE_GAME_DATA`) on a short-read extraction.

## Zone flies one shared hull, `hdships\Zone` (2026-10-06, `zone-craft`)

The maintainer's report ("Zone does not pick the appropriate craft") was a
placeholder: `zone_craft` was `PlayerShip`, a "not a finding" value. Omega's
ship-model loader (`Ship_LoadModelSet`, `0x01301ba0`, confidence 75) names
`Data\Art\Published\HDShips\Zone\Ship.vex` for game mode 6 without reading the
craft, now `oag_title::ZoneCraft::OwnShipAt`. A Zone race on `tech_de_ra` with
`ag_systems` loads and draws it.
Livery key differs from 2048's (`zoneship_team` against `zoneship_zone`).
**Wired (2026-10-06, `omega-2048-craft`)**: an HD-era craft's `PI_TeamModel
name="zone"` `texturelocation` (`zoneship_quirex`; all twelve match a
`Zoneship_<Team>/Team.gnf` on disc, spelt as the disc spells them) replaces the
key on the hull's team texture request; the pixels change in the hull's patches. The five
2048-era teams author no zone model, keep the default `Zoneship_Team` skin and
the loader report says so (`Tigron` and `VanUber` name `livery1`, which has no Zone directory: the swap finds nothing and says so). 2048: **ported**, see
[2048-status.md](2048-status.md). Evidence:
[zone-craft.md](../ghidra/functions/ps4-omega-eu/zone-craft.md).

## 2048-era craft start a race (2026-10-06, `omega-2048-craft`)

Omega's `Data\plugins\teams\Definition.xml` lists nineteen raceable teams in
two eras: five `*2048` teams whose `PI_Team location` is
`Data\art\published\Ships\<Team>2048` (four craft each, `\1`..`\4`, with
`handlingstatslocation="Data\HandlingStats\<team>\<n>"` authored per craft) and
fourteen HD-era ones under `hdships`. The loader looked every craft up under
`hdships\`, so `--team Feisar2048\3` died on `hdships\feisar2048\3\handlingstats.xml`
(the old repro used a lowercase id, which no table recognises either way). Now
`oag_title::GuestRoster` carries a second tuning directory (`handling_dir`,
`None` for 2048's HD-derived twelve) and `oag_omega::race::ERA_2048_ROSTER`
names Omega's: Subdirectory join, the five teams, `Measured` (the definition
authors both directories). `omega_2048_era_craft_ground_truth` reads the
roster off the disc and resolves Ship.vex, ship.rcsmodel and a parsing
`handlingstats.xml` for all 34 craft ids, asserts each team's table directory
equals its declared `location` and that no 2048-era craft exists under
`hdships\`. A race on `Feisar2048\3` loads and draws. Side effect, accepted:
Omega now offers CRAFT TITLE "Wipeout 2048" in Race Remix when no 2048 disc is
present, and its own list drops the 2048-era teams to that row.
**2048 vs Omega: checked, differs** - 2048 keeps these craft as its own roster
(`team_variants`), Omega as a guest of its HD-era one.

### Vita 2048 against Omega's 2048-era craft: census (2026-10-06, `remix-labels`)

`crates/source/tests/two_title_2048_sources_ground_truth.rs` reads all twenty craft
(five teams, four each) off `data/extracted/vita/PCSF00007` and
`data/extracted/ps4`:

- **Roster**: identical ids on both, `AG_Systems2048`, `Auricom2048`,
  `Feisar2048`, `Piranha2048`, `Qirex2048`, `\1`..`\4` each.
- **`handlingstats.xml`**: all twenty parse to equal `Stats` (every field).
  Omega kept 2048's tuning unchanged, so a race cannot tell the sources apart
  by handling.
- **Models**: `ship.rcsmodel` differs in size for every craft (for example
  `Feisar2048\3` is 476,012 bytes on the Vita disc, 489,720 on Omega; Qirex
  craft are smaller on Omega, the rest larger). Textures are `.gxt` against
  `.gnf`. Not decoded to a per-vertex or LOD comparison here.

**Race Remix offers both** (maintainer, 2026-10-06): with only Omega mounted,
CRAFT TITLE offers the plain "Wipeout 2048", backed by Omega; with only the
Vita disc, unchanged; with both, "Wipeout 2048" is the Vita disc's and
"Wipeout 2048 (Omega)" Omega's, each loading its own source.
`oag_title::GuestRoster::alongside_label` carries the second name (`Some` for
Omega's roster, `None` for 2048's HD twelve, where a real HD disc still wins).
Title names are plain values in this menu, so the label needs no `string_id`.
2048 vs Omega: **checked, differs** (models and textures), **checked, same**
(tuning).

## The campaign flies the original opponent (2026-10-12, `orig-ai-omega`)

Omega's executable carries 2048's opponent code with both lineages behind
`driver+0xa0 = (mode id < 0x17)`, and every cell of the HD-schema campaign this
project launches maps its `Mode` to a named id below `0x17`. So the campaign's
opponents fly HD's laws plus one of Omega's own, the lane search's coin
(`oag_omega::race::ORIGINAL_DRIVER`, `RaceDefaults::original_opponent` on).
Static only (no capture of the original's AI; shadPS4 pairs only the start grid): [ps4-omega-eu/ai-controls.md](../ghidra/functions/ps4-omega-eu/ai-controls.md).
Both `AIRaceStats`/`AIControlStats` sets ship and read, every `PI_Track` has its
`stats.xml`, and the field finishes on Vineta K, Talon's Junction, Ubermall
(Phantom) and Amphiseum reversed with no respawn
(`crates/raceplay/tests/omega_original_ai_ground_truth.rs`). The 2048-lineage laws are
not wired: the 2048 campaign map opens from the chooser ([omega-frontend.md](omega-frontend.md), "The 2048 map opens from the chooser") but the lineage laws are 2048's and Omega's races on it are the incomplete ones this page lists.

## Engine trail and flare (2026-10-16, `omega-exhaust`)

`oag_omega::TITLE` carries `exhaust: Authored` and `flare: PerTeam` on 2048's
shape (they were `Unread`). An Omega race draws each craft's own `EngineFlare`
model and the shared `bluered` ribbon; `MagstripWake` is the track's electric
arc and is not the exhaust.

**Census** (`crates/rcs/tests/psp2_engine_trail_ground_truth.rs::omega_census`,
one walk over the nine `.psarc`s): 69 `EngineFlare` `.vex` + `.rcsmodel` pairs,
68 of which decode with geometry through `oag_rcs::rcsmodel::psp2` (one has no
geometry the reader finds, and draws nothing), and 9 ribbon templates.

| Archive | Flare pairs | Ribbon templates |
| --- | --- | --- |
| `data00` | 20 (`ships/<team>2048/1-4`, five teams) | 8 (`data/ribboneffects/*_triangle`) |
| `data01` | 27 (`hdships`) | 0 |
| `data03` | 13 | 0 |
| patch `data05` | 0 | 1 |
| patch `data08` | 9 | 0 |
| `data02`, `data04`, patch `data07`, `data09` | 0 | 0 |

**What the executable names** (`strings` on the base and the patch `eboot.bin`
alike): `data/RibbonEffects/enginetrail_bluered_triangle.vex`,
`%s\engineflare.vex` and `data/Tex/EngineFlare/Engine_Flare_Rich.gnf` (the patch
also names `Engine_Flare_Rich_red.gnf` and an `engineflare2048` program pair,
unread). These are 2048's three literals with `.gnf` for `.gxt`, so the axes are
2048's with the PS4 texture extension.

**Against 2048** (checked 2026-10-16): the `.vex` files are byte-identical
(`EngineTrail_BlueRed_triangle.vex`, `feisar2048/1`, `piranha2048/3`); the
`.rcsmodel` files differ (`enginetrail_bluered_triangle` and
`feisar2048/1/engineflare`, `qirex2048/1/engineflare`) - the PS4's 8-byte
pointers - and decode through the same `psp2` reader. Verdict: **ported**, with
`crates/omega/tests/omega_title_ground_truth.rs::exhaust_and_flare_axes_name_files_omega_ships`.

**Measured and not**: as on 2048 ([2048-status.md](2048-status.md#engine-trail-and-flare---2026-10-05)),
asset names, node groups and material names are read off the data; the ribbon
geometry, sprite flare law and boost reveal are HD's, **inherited, not measured
on Omega**, and the load report says so. The flare's `2048_engine_additive`
blend is chosen, not measured. The engine light is wired (EngineLightLaw, see
[engine-light.md](../ghidra/functions/ps4-omega-eu/engine-light.md), "Wired"). Checked in a headless race (Feisar, `--autopilot`, ticks 420 and
900): the nozzle glows and every rival's trail draws.

## Re-cooked, measured (2026-10-19, `asset-lineage-census`)

CLAUDE.md says Omega re-cooks 2048's and HD's content rather than shipping it
identical. This section measures how far. It is a census, not a reader change.
Every number comes from one command over the archives, and the tables are
counts and dimensions only:

```sh
cargo run -p oag-tools --example recook_census -- vita environment <PCSA00015 base data.psarc> <dlc1.psarc> <patch data1.psarc> <patch data2.psarc>
cargo run -p oag-tools --example recook_census -- ps4 environment <omega base data00..04.psarc> <patch data05,07,08,09.psarc>
cargo run -p oag-tools --example recook_census -- ps3 environment <hdfury DATA00..06.PSARC>
cargo run -p oag-tools --example recook_census -- {vita,ps4,ps3} hdships/ (or /ships/ on HD) <same archives>
```

Vita is EU `PCSA00015` (base, DLC1, patch v1.04); later archives override
earlier ones at the same path. Omega is EU `omega-eu` with patch `1.07`. Every
path read parsed, none refused. Circuit and hull names are matched by directory
name, with a leading `NN_` stripped on the HD-numbered circuits.

**Summary.** Textures are **re-encoded**, not just re-sized: 2048's `.gxt` is
PVRTC 4bpp, Omega's `.gnf` is BC7 (5,256 of 5,291 paired track textures), and
HD's are DXT. Track models are **re-cooked to a different triangle count**:
Omega's track has 3.0-8.7x the triangles of Vita's on the ten base circuits,
3.5-14.2x on the DLC circuits, and 1.3-2.9x HD's. Zone
models keep their triangles to 0.2% and re-weld their vertices. Hulls keep their
triangles, but not their vertices.

### Track: 2048-era circuits, Vita against Omega `Data/environments2048`

Vertices / triangles / materials (submeshes in brackets). Omega's main model is
`track.final.rcsmodel`; 2048 ships `track.rcsmodel`.

**`arena` and `square` are the two circuits whose vertex data (293.7 and 295.9 MiB) passes
wgpu's default 256 MiB buffer ceiling**, so ours cuts their vertex buffer in two on a device
with that ceiling; 2048's own copies are 51 and 73 MiB. Census of every circuit, and the
mechanism: [`vertex-buffers.md`](../rendering/vertex-buffers.md). 2048 cross-check: ported
(2048's circuits all fit one buffer and draw as before).

| circuit | Vita `track` | Omega `track.final` | tri Omega/Vita |
| --- | --- | --- | --- |
| altima | 531,735 / 419,023 / 527 [2,817] | 1,928,152 / 1,912,623 / 575 [3,379] | 4.56 |
| arena | 537,492 / 383,816 / 1,142 [3,668] | 3,080,162 / 3,331,753 / 1,140 [4,111] | 8.68 |
| bridge | 857,841 / 580,679 / 1,339 [5,753] | 1,840,112 / 1,740,321 / 1,344 [5,775] | 3.00 |
| cathedral | 630,193 / 434,299 / 1,194 [4,836] | 2,237,715 / 2,285,771 / 1,260 [4,864] | 5.26 |
| mall | 688,771 / 491,228 / 1,413 [7,357] | 2,164,322 / 2,240,855 / 1,436 [6,486] | 4.56 |
| park | 520,134 / 365,249 / 675 [4,829] | 2,507,290 / 2,475,557 / 689 [4,751] | 6.78 |
| sol | 386,724 / 257,477 / 490 [2,690] | 826,788 / 781,421 / 376 [2,822] | 3.03 |
| square | 760,538 / 495,222 / 1,832 [5,700] | 3,102,957 / 3,027,697 / 1,616 [5,999] | 6.11 |
| subway | 954,520 / 647,569 / 1,737 [7,221] | 2,476,429 / 2,413,478 / 1,690 [7,212] | 3.73 |
| tower | 688,628 / 470,840 / 1,311 [4,957] | 2,391,247 / 2,462,729 / 1,032 [4,936] | 5.23 |

The eight Vita DLC1 circuits are in the HD table below. Vita's DLC copies also
differ from the base package's own copies (Anulpha 352,012 against 351,727
vertices). The triangle counts are the census; why Omega's `track.final`
carries 3-14x Vita's is answered below, in "Why Omega's track has more
triangles".

### Track: HD-numbered circuits, PS3 against Omega `Data/environments`

Triangles only. Omega is `track.final.rcsmodel`, HD is `track.rcsmodel`.

| circuit | HD tri | Omega tri | Omega/HD | Vita DLC1 tri / HD |
| --- | --- | --- | --- | --- |
| 01 Vineta K | 953,467 | 2,169,965 | 2.28 | 0.16 |
| 04 Chenghou Project | 433,557 | 1,048,618 | 2.42 | 0.36 |
| 05 Ubermall | 812,523 | 2,128,580 | 2.62 | 0.32 |
| 10 Sebenco Climb | 451,174 | 1,006,192 | 2.23 | 0.23 |
| 12 Sol 2 | 750,855 | 979,714 | 1.30 | 0.26 |
| 15 Anulpha Pass | 599,607 | 779,064 | 1.30 | 0.37 |
| amphiseum | 773,890 | 1,696,658 | 2.19 | - |
| modesto_heights | 904,463 | 1,966,253 | 2.17 | - |
| talons_junction | 809,247 | 1,067,844 | 1.32 | - |
| tech_de_ra | 820,441 | 2,379,040 | 2.90 | - |

HD's `02_track` and `03_track` are not matched to Omega's `02_metropia` and
`03_moa_therma`; the names are unresolved and Omega's two have no HD row. The
Vita DLC1 copy of each circuit carries a sixth to a third of HD's triangles.

**Zone models** (`zone_1..4`, `Data/environments/zone_N/track.rcsmodel`): triangle
counts match HD to 0.2% (zone_1 455,747 on both; zone_2 602,817 against 602,953),
and vertices differ by under 2.5%.

### TrackZone: 2048 Vita against Omega `environments2048`

| circuit | sub | tri Vita / Omega | verts Omega/Vita |
| --- | --- | --- | --- |
| altima | 6,349 | 857,701 / 857,701 | 1.0032 |
| arena | 6,797 | 617,816 / 617,816 | 1.0049 |
| bridge | 11,795 | 1,130,566 / 1,130,566 | 1.0035 |
| cathedral | 6,067 | 499,266 / 499,266 | 1.0046 |
| mall | 11,687 | 854,189 / 854,189 | 1.0027 |
| park | 7,644 | 757,580 / 757,580 | 1.0047 |
| sol | 9,530 | 573,323 / 573,323 | 1.0041 |
| square | 8,460 | 807,816 / 807,816 | 1.0201 |
| subway | 11,269 | 1,130,759 / 1,130,759 | 1.0030 |
| tower | 10,386 | 981,368 / 981,368 | 1.0041 |

Every submesh count and material count is equal. Triangles are identical on
all ten: **re-welded, not re-cut**, which is the same finding the hulls give.

### Textures: stem-matched pairs, same circuit

Pairs are the textures whose stem appears on both titles inside the same circuit.
`same dims` is equal width and height; `second larger` is the second title's
area bigger. Ratio is the median width ratio, second title over first.

| circuit | pairs | Vita -> Omega same / Omega larger | median ratio | mips equal |
| --- | --- | --- | --- | --- |
| altima | 131 | 118 / 13 | 1.00 | 1 |
| arena | 320 | 285 / 35 | 1.00 | 1 |
| bridge | 317 | 295 / 21 | 1.00 | 5 |
| cathedral | 390 | 372 / 18 | 1.00 | 5 |
| mall | 353 | 296 / 57 | 1.00 | 1 |
| park | 315 | 261 / 53 | 1.00 | 0 |
| square | 557 | 483 / 74 | 1.00 | 9 |
| subway | 484 | 433 / 50 | 1.00 | 6 |
| sol / tower / shared | 105 / 351 / 128 | 76 / 302 / 95 | 1.00 | 1 / 1 / 3 |
| Vita DLC1 circuits (vineta_k, chenghou, metropia, moa_therma, sebenco, sol_2, ubermall, anulpha) | 139 to 287 each | 0 to 10 same | **2.00** (moa_therma 4.00) | 0 to 6 |

So the 2048 base circuits keep their dimensions on Omega for 83-95% of pairs
(a larger Omega texture is 10-16% of pairs on altima, mall and park). The DLC circuits
are half the width on Vita and the same as HD on Omega, for every pair. Omega
against HD on the same HD circuits: 1,828 of 2,699 pairs equal, 26 HD larger,
1,722 mip counts equal.

### Craft: hulls against Omega and HD

Main parts only (`ship`, `ship_lod`, `ship_lod1..3`, `shipwreck`, `shipshield`),
skin variants `_c1`/`_n1` excluded. Tallies over the parts both titles ship:

| pair | parts | triangles equal | vertices equal | submeshes equal | materials equal |
| --- | --- | --- | --- | --- | --- |
| Vita 2048 against Omega | 94 | 86 | 23 | 90 | 91 |
| Omega against HD | 70 | 27 | 5 | - | 44 |

Feisar's `ship.rcsmodel` on all three: 22,527 triangles and 5 materials each;
vertices 25,976 (Vita), 26,017 (Omega), 25,968 (HD). Detonator's `ship` is the
one part whose triangle count moved: Vita 15,666, Omega 13,867 (64 submeshes
against 62). Assegai `ship_lod` moved too: 7,918 to 8,038. Tigron and
Detonator VR exist only on Omega.

Hull textures are stem-matched on Vita and Omega: 1,025 pairs. 775 keep their
dimensions and 250 are larger on Omega; the `_c1` livery set (egx, harimau,
mirage) is twice as wide on Omega in the rows it shares with Vita. Every paired
hull texture is PVRTC 4bpp on Vita and BC7 on Omega.

### What this says, and what it does not

- **Textures**: re-encoded on every title (PVRTC on Vita, BC7 on Omega, DXT on
  HD). Dimensions are kept for the 2048 base circuits and the hulls. The DLC
  circuits are re-sized up to HD's on Omega. Mip counts differ on most pairs.
- **Track models**: Omega's `track.final` is a different triangle count on every
  circuit it shares with 2048 or HD, not a re-weld. Cause: new material groups,
  see "Why Omega's track has more triangles" below.
- **TrackZone and Zone**: re-welded, triangles kept. Zone is also HD's model.
- **Hulls**: triangles kept on most parts, vertices re-welded on all of them.
- **Not checked**: Vita USA (`PCSF00007`), and the HD names `02_track` and
  `03_track` (unresolved against Omega's `02_metropia` and `03_moa_therma`).
  Mip counts are read as `mip_levels` on `.gxt` and `last_mip_level + 1` on
  `.gnf`, assuming base level 0.

Check per CLAUDE.md, "A 2048 finding is checked against Omega": track and
trackZone counts are **checked, differs** (track) and **checked, applies**
(trackZone triangles); hull triangles are **checked, applies** with vertices
**checked, differs**.

### Why Omega's track has more triangles (2026-10-19, `recook-triangles`)

Answer: **remodelled geometry under new material groups** (confidence 65,
Plausible: the counts are measured, the reason for the new groups is inferred).
Not packing of Vita's sibling files, and subdivision is a minority. Checked on
three circuits: altima, arena and Anulpha Pass (DLC1 against Omega's
`15_anulpha_pass`).

The same example as the census, with the per-submesh and per-50 m cell rows
(`--submeshes` and `--cells` are the 2026-10-19 additions):

```sh
cargo run -p oag-tools --example recook_census -- --submeshes --cells 50 vita altima/ <PCSA00015 base data.psarc>
cargo run -p oag-tools --example recook_census -- --submeshes --cells 50 ps4 environments2048/altima/ <omega data04.psarc> <omega data00.psarc>
```

Node-bound submeshes are carried to world space by their node's bind matrix
(`psp2::nodes`); the pivot caveat in that module applies to those rows only.
Every number below is from the rows, not from the census table above.

**Where the triangles are (altima, `track.rcsmodel` against `track.final`).**

| part | Vita | Omega |
| --- | --- | --- |
| world-space submeshes: triangles / area m2 | 2,593 subs, 390,096 / 27.6 M | 2,591 subs, 1,379,536 / 34.8 M |
| node-bound submeshes: triangles | 224 subs, 28,927 | 788 subs, 533,087 |
| nodes carrying submeshes | 131 | 695 |

Omega's extra 1.49 M triangles are 66% world-space and 34% node-bound. Area
grows 26% in world space, while triangles grow 3.5x: the triangles are smaller
(mean 71 m2 down to 25 m2).

**Material groups (the main finding).** Joined on the material's basename,
since the folder differs by title. Omega triangles on basenames Vita never
uses:

| circuit | Vita tris | Omega tris | Omega on basenames absent from Vita |
| --- | --- | --- | --- |
| altima | 419,023 | 1,912,623 | 1,601,826 (84%), 24 basenames |
| arena | 383,816 | 3,331,753 | 3,082,785 (93%), 22 basenames |
| Anulpha Pass (DLC1) | 222,448 | 779,064 | 580,122 (74%), 27 basenames |

The largest of them on altima: `fc07_blend_vertred_lambert_2diff_new` (189
submeshes, 71,105 tris, 13.5 M m2; Vita has `fc07_blend_vertred_lambert_2diff`
on 203 submeshes, 75,597 tris), `dc_rocks` (32 submeshes, 308,818 tris),
`crowd_diffuse_normal` (28 submeshes, 624,228 tris), `tree_wind_new` (139
submeshes, 220,870 tris), `flag_new` and `crowd_diffuse_normal_shadow`. The
`_new` suffix is the same material re-authored. On the shared basenames Omega
has fewer triangles than Vita (310,797 against 407,345 on altima), so this is
not one material given more detail.

None of the five names above appears in any Vita environment: the census walked
the base, DLC1 and patch-v1.04 `environments/` paths of PCSA00015 (248 distinct
material basenames) and found no submesh using them.

**Cells (50 m square of the x/z plane, all submeshes, altima).** Each occupied
cell classed by Vita's and Omega's triangles and area:

| class | cells | Vita tris | Omega tris | Omega area minus Vita, m2 |
| --- | --- | --- | --- | --- |
| identical (same triangles, same area) | 1,402 | 69,582 | 69,582 | 0 |
| same area, more triangles (subdivision) | 173 | 42,135 | 178,701 | 0 |
| more area | 1,220 | 264,803 | 1,553,523 | +9.5 M |
| less area | 223 | 39,240 | 106,282 | -0.95 M |
| Omega only / Vita only | 101 / 53 | 0 / 3,263 | 4,535 / 0 | |

Subdivision accounts for 136k of the 1.49 M extra triangles (9%). The rest sits
in cells where Omega has more surface, not more triangles on the same surface.

**Packing (refuted for these three circuits).**

- No Vita sibling file holds the Omega-only groups (above). The altima Vita
  siblings are `trackZone` (858 k triangles, identical to Omega's own
  `trackZone`) and `trackpart_startgridanims*` (14 k). Omega `track.final`
  (1.91 M) is not Vita `track` plus `trackZone` (1.28 M).
- The Zone overlap is inherited, not packed. By triangles and bounds, 62% of
  Vita's own `track` (260,796 of 419,023) matches a Vita `trackZone`
  submesh. Omega's `track.final` matches Zone on 10% of its triangles
  (198,103), a smaller share.
- No LOD stacking. Submeshes sharing a material and bounds: 26 groups in Vita's
  `track`, 14 in Omega's.

**Submesh correspondence.** By triangles and bounds, 1,801 of Omega's 3,379
`track.final` submeshes (16.5% of its triangles) match a Vita `track` submesh,
and 1,578 (1.60 M triangles) do not. The first 92 world-space submeshes of the
two files are identical; from there the order diverges, so index pairing is
meaningless past that point.

**Open.** HD's circuits are not in this measurement: the example reads HD
triangles only, not positions or material names, so HD's share of the picture
is unmeasured. The bind-matrix placement of node-bound meshes is taken from
`psp2::nodes` and not separately checked here. Whether Omega's new material
groups are reused from another Omega circuit, such as `dc_rocks` on several,
was not counted.

## Weapon bodies (2026-10-26, `omega-from-hd-wire`)

**What loads.** Omega's table (`oag_omega::weapons::WEAPON_MODELS`) names HD's body
set from Omega's own `data00`: the Rocket, Mine, Bomb, the Cannon's muzzle flash,
the Plasma blast's ring, sphere and halo, and the Missile blast, each a `.vex` with
its PS4 `.rcsmodel` beside it. `oag-raceplay`'s weapon loader takes the PS4
container through `mesh::rcs::psp2` (it asked `mesh::rcs::build` only, and failed on
`version 0xedad5cca, expected 0x000a0000`, the line the shield shell still reports),
and passes the material state word to the same back-face cull rule HD's models use
(`load::weapon_models::cull_as_authored`; the PS4 word is HD's, inherited, not
measured on PS4). `omega_weapon_bodies_ground_truth` pins that all eight build and
none is reported absent.

**Which root.** `Rocket_Construct` (`0x01368610`) formats `Data/Weapons/%s` when
`DAT_01f999e4 < 0x17` and `Data/Weapons2048/%s` otherwise (decompiled again this
lane; the mode, not the circuit, decides). Every mode the port launches is below
`0x17`, so `Data\Weapons\` serves every race, an `environments2048` circuit
included. The two roots ship 22 body names in common, none byte-equal (the
rocket's `.rcsmodel` is 12,265 bytes under `Data/weapons`, 5,033 under
`Data/weapons2048`), so a 2048-lineage mode would need a per-mode root.

**Against HD, same circuit and ship.** Talon's Junction, Feisar, HD's own scenario for
each body (`hd_weapon_scene_ground_truth`, `hd_missile_blast_ground_truth`,
`hd_bomb_blast_ground_truth`, `hd_leach_ball_ground_truth`): the craft is at the same
position at tick 304 on both titles (`(11.9, -51.5, -195.8)`; speed Omega 23.94, HD
23.98), and the rocket volley is the same mesh with the same decals, darker because
Omega's `Looks` carry none of HD's lit-output or weapon-light laws. The mine and
bomb bodies match; the mine's green rays are thinner. The Plasma blast is the same
dome in a darker violet. The Missile blast draws its structure without HD's
white-out. The cannon's flash sits at the left `cannon_flash` locator, as HD's does.

**Held back, and why.** The Plasma bolt's head, the Bomb's blast set and the
LeachBeam's ball load and build, then draw wrong or not at all, and a body that
draws invisibly makes the load report lie, so each stays `None`:

| Body | What draws on Omega | HD |
| --- | --- | --- |
| `HD_plasma_ball` | a barely visible ghost above the nose | black core, white rim |
| `HD_bomb_*` blast set, `HD_bomb_halo` | a brown ball with a red wash over the road, a red dome round the laid bomb | white-hot ball, pink-white shell |
| `hd_leachbeam_ball_bloomring` | a grey bubble at the target | a white-cyan glow |

HD shades them with fragment programs `mesh::rcs::rim_glow` recognises by fingerprint
on the PS3's RSX code (`RIM_EDGE`, `RIM_GLOW`, `BOMB_FIRE`). The PS4 builder sets none
of those bits: `rg slots:: crates/mesh/src/mesh/rcs/psp2.rs` finds three, none a
program bit. The bloom ring paints `hd_bombfire_bloomring_fireramp_128x32` (a colour
lookup that HD's program samples by alpha) as a diffuse. A name-keyed inheritance in
the style of `psp2::lineage_blend` is the cheap route; the PS4 shaders are unread.

**The arc wake.** HD-lineage modes build the procedural wake
(`ships-effects.md`, "One effect or two"). Its two textures are the executable's own
literals, `data/Tex/HD_electric_arc_8x8.gnf` (`0x01837096`) and
`data/Tex/HD_ElectricArc_Contact.gnf` (`0x018370b7`); the loader's texture decode
gained a `.gnf` arm, and a craft first meets a strip with the wake drawn at tick 1,115
on Talon's Junction under the autopilot.

**The LeachBeam strip: checked, differs.** Omega's `leachbeam_triangle.rcsmodel` names
`hd_leechbeam.gnf` (128x32) and `hd_waketrail_clouds.gnf` (128x128) where HD's names
`hd_leechbeam_glow` (256x256), a file no Omega archive holds, and the executable's
ribbon-model table is HD's in HD's order (`0x018332bc`..`0x018332f7`). Played with HD's
strip law, the pair draws a line the pixel difference finds and the eye does not at
player size; the field stays `None` until the PS4 `HD_Leachbeam` material's samplers
are read.

**Adverts: the pass is on, and plays `321Go_2048.vex` (2026-11-10, `omega-gantry`).** `gantry::build` reads Omega's `321Go`
files through the Vita/PS4 container (`mesh::rcs::psp2::build_with_vex`, the `.vex` supplying node motion;
`start_gantry_omega_ground_truth.rs`). The 2026-10-26 attempt turned `adverts` on and compared Vineta K against the shadPS4
start-grid capture: placement matched (the track's own `billboard7/8` quads), the look did not. The countdown board drew a
static red face because HD's digits are material curves (`curve_track::material_anim_tracks`) and the PS4 material
container's curves are unread, and the side cards drew dark teal because `fx350_nomip` is a `.gnf` here. Now:

- `.gxt` and `.gnf` spellings are in `oag_render::gantry` (`is_slot_placeholder`, `mount`, `is_fx350_texture`: by stem).
- `adverts` is HD's pass, inherited, with `gantry_boards` naming `321Go_2048.vex` (the node-driven board: frame 0 on tick 73, so
  `GO` slides in on the release, chosen). The side cards show their logos on Vineta K.
- The HD-lineage board `321Go_StartFinish.vex` **plays** (2026-11-10, `omega-billboards`): its digits ride UV keys
  authored in the `.vex` `Mesh` nodes, not a curve in the `.rcsmodel` (`TexAnim_UpdateTransform`, `0x0172ba00`,
  confidence 88; `docs/ghidra/functions/ps4-omega-eu/billboards.md`, "The UV keys"). A Zone race shows `321Go_Zone.vex`.
  `docs/rendering/start-gantry.md`, "Omega and 2048: the HD-lineage board counts down".
- shadPS4 draws this card's face blank white throughout the countdown on Altima (Race Box, GAME MODE WipEout 2048,
  `scripts/shadps4-omega-countdown.py`), so the digits and their phase are not measurable on it; the board's place is.
  Evidence, numbers and the box: `docs/rendering/start-gantry.md`, "Omega and 2048".
- **2048: ported**, the same Altima through `oag-game <PCSF00007>`; `park` ships no `billboard8.gxt` in the base package.

**The plume's two flags.** `front_faces_only` stays `false` (open). `engine_light` is wired, below.
Omega's loader reads `EngineLightData.xml` the way HD's does (`FUN_0130e090`: `Distance`
to `ship+0xbc`, `Radius` to `ship+0xc0`; the `hdships` files hold HD's range, `-0.4` to
`1.2` and `1.0` to `2.0`, not 2048's `3`/`30`), and with the light on a grid still
gains the warm cast the original's hull has at the nozzle. **The consumer is read
(2026-11-16, confidence 85) and wired (lane `engine-light-wire`)**: `EngineFlare_SubmitVertexLight` appends one
point light per craft to a 32-slot table, and each draw binds up to six as
`DiffuseVertexLight<N>`. The colour is a constant picked by the craft: `(4, 10, 40)`,
`(40, 10, 4)` on a Fury skin, `(3.0, 0.5, 0.1)` for a `GameVersion="2048"` team, and
`(1.5, 0.1, 3.0)` over a mag strip in a 2048 mode
([engine-light.md](../ghidra/functions/ps4-omega-eu/engine-light.md)). So HD's law is
right for the `hdships` craft and wrong for the 2048-era ones, whose `3`/`30` light is
red and dim, not white.
**Confirmed live the same day** (native port, four boots): the table holds eight records
of exactly those colours, the hull's own programs take the light
(`(ambient + vertexLight + sun) * texture * vertexColour`), and the housing is lit on
some frames and dark on others because the range is redrawn as `Radius +- 1.0`. The
glow inside the nozzle is the flare model, not the light. A single captured frame is one
draw of that flicker (same page, "Live check").
Both are `Title` data now (`oag_omega::TITLE`'s `Flare`, lane `engine-light-wire`).
`front_faces_only` changes nothing visible on either still, because the flame is not
visible at these two ticks.

**Effect names located, triggers not.** `WO_SHIP_SPARK_DAMAGE_WEAPON`
(`ShipCollisionFx_Trigger`, `0x01723ae0`, `kind 1`), `WO_DAMAGE_MILD`, `_MODERATE`
and `_CRITICAL` (`FUN_016f3ef0`), `WO_WEAPON_ABSORB` (`FUN_01722810`, a per-ship object
built by `FUN_01307560`), `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` (`FUN_016f0f20`) and
`WO_BOMB_RAYS` (`FUN_013594d0`, a 107 KB decompile). What raises each is unread, so none
of HD's seven effect triggers is wired.

**2048 against Omega.** **Checked, applies, not wired** for the bodies:
`data.psarc` ships the same names under `Data/Weapons2048/` only, with its own bytes,
so 2048 needs its own paths read off its own executable. The leach strip's texture
names are **checked, differs** on both titles. See
[`weapon-lineage.md`](../gameplay/weapon-lineage.md).

## Where the WipEout 2048 mode pairs' error is (2026-11-16, `omega-2048-holistic`)

The five worst 2048-mode grid pairs ([compare-page.md](../tools/compare-page.md), "WipEout 2048 mode") sit at 44 to
50 whole-frame mean grey. Question asked: is one cause most of that, and can it be fixed across all five? **No.**
Ours re-rendered from the pairs' own flags (`compare-rerender-ours.rerender` on copies) reproduces the stored scores
to the hundredth.

**The raw score is the wrong instrument here.** Edge correlation is about 0 on every pair, a per-channel affine fit
removes 3 to 12 grey and its gains (0.15 to 0.56) are regression to the mean, and pixels much brighter in theirs
and much brighter in ours carry about equal shares. Under that decorrelation, correct detail a few pixels out of phase
scores worse: anisotropy 16x raises all five (Queens Mall 50.07 to 50.57, Metro Park 44.30 to 46.10), forcing
MIP 0 raises all five (Metro Park to 49.17). A wrong-signed result on the raw score is not evidence against a fix.

**Instrument used instead** (`scripts/compare-decompose.py`): each material family's pixels in ours are the pixels
an `OAG_SKIP_MATERIAL=<family>` render changes by more than 25 grey after the frame gain the adaptive exposure adds
is divided out; the own hull (`feisar2048`), `crowd`, `flag` and the HUD boxes are left out of every score (the
livery and the crowd differ boot to boot in the port). Per region: `LF`, mean |blur(ours) - blur(theirs)| at sigma
12; `bias`, ours minus theirs; `G`, Sobel gradient energy ours over theirs.

| Pair | static LF | road family | road area, LF share, bias, G | sky LF share, bias | scenery LF share, bias |
| --- | --- | --- | --- | --- | --- |
| Queens Mall | 29.7 | `road_alphaspec_normal_emissive_panelmap1` | 27.5 %, 44 %, -18.0, 0.61 | 8 %, -50.7 | 47 %, -19.8 |
| Sol (A class) | 34.0 | the same | 32.5 %, 51 %, -38.5, 0.24 | 19 %, -26.7 | 30 %, -9.4 |
| Metro Park | 21.3 | `diffuse_normal_specular` (brick) | 29.3 %, 40 %, +18.6, 0.79 | 2 %, -12.8 | 59 %, -17.2 |
| Empire Climb | 29.9 | `tarmac_*` | 54.5 %, 66 %, -7.5, 0.84 | 11 %, -66.6 | 34 %, -36.3 |
| Capital Reach | 39.9 | `tarmac_*` | 36.6 %, 34 %, -5.5, 0.67 | 4 %, -32.5 | 62 %, +21.6 |

Ranked, by what each would remove:

1. **The track surface: 34 to 66 % of the static low-frequency error on every pair, but three families.** Over all
   ten 2048-mode pairs the road is `tarmac_*` on five (Capital Reach, Empire Climb, Subway, Downtown, Arena),
   `road_alphaspec_normal_emissive_panelmap1` on four (Queens Mall, Sol, Altima, Unity Square), brick on Metro Park.
   So no one fix reaches all five pairs. Per family:
   - `road_alphaspec_..._panelmap1` (Sol's road 38.5 dark, `G` 0.24): the original draws its panels **displaced**,
     recessed with lit walls and bevel highlights, and a sky-blue sheen at grazing angles; ours draws flat panels with
     outline lines. Forcing MIP 0 lifts Sol's road `G` only to 0.37 and Queens Mall's to 0.78, so the detail is in the
     family's pixel program (428 instructions, `Cg_PanelDisplacement_1In1Depth`, `SlopeWidth`, `Aspect`; Queens Mall
     authors `0.2, 4.0, 3.5, 1.8` in its four parameters), not in the texture chain. The largest single
     family by error. Its `EmissiveMap` (`track_a_floor_e`, Queens Mall's cyan strips) is the same program.
     **Read 2026-11-16 (`panelmap-re`,
     [panel-displacement.md](../ghidra/functions/ps4-omega-eu/panel-displacement.md); law 85 to 88, no live run):**
     no vertex moves. The pixel program sphere-traces the view ray (10 samples) into a signed distance field, the
     `R16F` panel map (`Depth` 3.5 and `SlopeWidth` 1.8 in hundredths of a texture height, `Aspect` the map's
     width over height), and samples diffuse, normal and emissive at the shifted coordinate; the recessed parts
     are the field's positive chevron strips, 14 % of the road. The four values above are `Blurrines`, `Aspect`,
     `Depth`, `SlopeWidth`. The sheen is a fresnel reflection
     `ReflectivityAmount * albedo.a * (ReflectivityMin + ReflectivityScale * (1 - N.V)^ReflectivityPower)` of a
     baked, pre-blurred cube array the engine loads beside every circuit (`skyCube.gnf`, four blur levels, layout
     decoded), plus a sun specular lobe under the same alpha; Sol authors a weight of 0.3 face-on and 2.35 at
     `N.V` 0.2, which is its 38.5 grey. **Wired 2026-11-16** (`mesh::rcs::psp2::panel_map`, the page's
     "Implemented where"): the march, the cube sheen and the emissive draw; the lobe's colour
     (`Lighting.Sun specular colour * intensity`) is read by the race loader (`panelmap-sun`, 2026-11-16) and
     draws almost nothing at the pairs' poses: 339 pixels move on Sol (largest step 7 grey), 527 on Queens Mall
     (1), the road scores unchanged; a x100 probe shows the path live, a highlight under the camera's reflection
     of the sun only. Road at 16x anisotropy, the before render's region masks: Sol `LF` 32.7 to 18.0, `bias` -32.4 to
     +16.1, `G` 0.35 to 0.75; Queens Mall 30.7 to 14.5, -16.9 to +8.5, 0.71 to 0.85; Altima and Unity Square
     within 1.1 grey and 0.03 `G`. **The sheen and the lobe are gated by the diffuse alpha** (`sheen-gloss-wire`,
     the panel block reads `first.a`, not the red lane `texel.a` is on a PS4 slot word): Sol `LF` 17.0 to 8.8,
     `bias` +13.8 to -3.0, `G` 0.74 to 0.79; Queens Mall `bias` +8.6 to -0.6, `LF` 14.7 to 15.5, `G` 0.85 to 0.83;
     Altima and Unity Square within 0.3 grey. **2048: checked,
     differs** (no panel map or `_disp` texture on the Vita archives; the displaced road is a PS4 addition).
   - `tarmac_*` and the brick: anisotropy 16x takes the road's `G` from 0.84 to 1.07 (Empire Climb), 0.79 to 1.00
     (Metro Park), 0.67 to 0.89 (Capital Reach) with `LF` unchanged: their sharpness gap is texture filtering, so
     the port runs anisotropic filtering where the pairs record ours with `--anisotropy off`. **Settled 2026-11-16
     (`port-aniso`), [below](#the-ports-anisotropy-2026-11-16-port-aniso): the game itself asks for 16x and the port
     honours it, so the `omega` profile now renders ours at 16x** (the numbers above are that run). The dirt layer
     (`OAG_TARMAC_DIRT=1`) moves no pair.
2. **The scenery: 30 to 62 %, no single term.** A frame gain fitted to ours (a ceiling for any exposure, light or tone
   term) removes 13 % of the static `LF` on Queens Mall and 27 % on Sol, and 0 to 1 % on Metro Park, Empire Climb
   and Capital Reach; on Queens Mall and Sol it lifts road and sky and makes the scenery worse. Static colour ratios
   (R/G, B/G) differ in sign from pair to pair. Empire Climb's scenery (-36) is the original's sun glare and lens
   flare round the low sun (`Lighting.Use Lens Flare` 1 and its keys, unread here).
3. **The sky: darker in ours on all five** (-13 to -67), 2 to 19 % of the static `LF`. `Lighting.Sky brightness`
   (1.56 to 3.0 on these circuits) is read by nothing here. In the 1.07 eboot its registrar row writes
   `Sky height offset`, `Sky scale` and `Sky brightness` at `+0x158`, `+0x15c` and `+0x160` of the object
   `FUN_015eda20` fills (the `LEA RSI` before each key's `LEA RDX` at `0x015edcee` to `0x015edd43`; confidence 75 for
   the offsets); the consumer was not found (the function has over 50 callers). Scaling the dome by it, linear or
   gamma, moves the five by -2.4 to +4.6 raw grey with mixed signs: not wired. **Read since**: its consumer is the dome's pixel program, `SkyBrightness * texture` in linear light,
   and the gain is wired for Omega ("The sky dome's pixel program"); re-measured with it, three of the ten 2048-mode pairs improve by a
   grey or more (Sol 32.71 to 29.57, Queens Mall 43.72 to 42.58, Empire Climb 44.97 to 43.94), four do not move, two move by 0.01 and
   Metro Park gets worse by 0.22.
4. **The crowd sprites draw white**: Sol's stands +112 against the original, Queens Mall +57, Subway +68, Arena +64;
   drawing no sprite at all takes Sol from 45.55 to 42.48. **Half read** (2026-11-16, `omega-crowd-shadowed`): the
   `_shadow` materials are lit by their `Shadowed` and `Ambient` (below) and moved; the plain
   `crowd_sprite_diffuse_normal_alpha`, which draws most of Sol's crowd pixels (46,650 under its own skip needle against the `_shadow` sprites' 3,376), declares neither and drew white because its alpha read the red lane: **closed** by the alpha lane
   ([below](#the-alpha-lane-2026-11-19-omega-road-programs); Sol +115.7 to +38.2, Queens Mall +48.5 to +4.7); the
   plain program's light inputs are still unread. Crowds only reach the pairs that show stands.
5. **Pose, Capital Reach only**: the Auricom banner sits about 480 px further left in ours at 1920 and the road lines
   lean the other way: ours' grid slot or heading differs from the original's. Its scenery +21.6 is that, so this pair
   cannot judge a render change until the pose is matched.

**Closed negatives.** Exposure is not still ramping at the grab: the static bias is the same at ticks 30, 100 and 300
(Queens Mall -20.8, -20.9, -21.2). A scalar specular fed from `Sun specular intensity` (ours reads HD's `Sun specular
scale`, absent on every Omega file, so specular is 0) raises Queens Mall to 55.2 and Empire Climb to 53.2: Omega's
RGB specular is not HD's scalar term and does not feed it (its colour reaches only the panel-map road's lobe). `Lighting.Ship diffuse brightness` (0.25 to 1, registered at
`+0x164`, key `0x0188d117`) is read by nothing here, but the hull's bias does not follow it (Sol, at 1.0, is the
brightest hull, +69), so it is not the hull's cause. **Measured live on the original (2026-11-16, `omega-hull-tone`,
[hull-tone.md](../ghidra/functions/ps4-omega-eu/hull-tone.md))**: the hull does read it, as a separate neutral term worth about 20 %
of Modesto's housing box at 0.25 (ours lacks it); it is not a scale on the hull, and the 1.8 to 2.4 times excess is in how ours' hull combines its diffuse terms (albedo, normal or the
bound ambient: open, confidence 55), with an equal sun constant, no specular and no reflection on the box. The stored pair's two craft differ in part set (rear arc, pylons); the housing box is clear of them.

**The crowd's law** (static, confidence 70; wired for the `_shadow` materials, `mesh::rcs::psp2::crowd`).
`crowd_sprite_diffuse_normal_alpha_shadow` (Sol 128,452 triangles) and `crowd_diffuse_normal_shadow` (Arena 380,285,
Altima 32,857) author `Shadowed` (`0x36997860`, `~crc32`) and `Ambient` (`0x735324ab`), per circuit: the sprite's Sol
values are `0.12178` and `(0.777, 0.777, 0.777)`, Queens Mall's `Shadowed` is `0.497`, Arena's
`crowd_diffuse_normal_shadow` is `0.4` with a coloured `Ambient` `(0.138, 0.071, 0.247)`. Ten of the sprite's fifteen
pixel programs load `Shadowed`. The sun-lit one (`data00.psarc` `Shdr` `0x4224`, 122 instructions) ends, after an alpha
test at 0.5:

```text
rgb = Shadowed * T.rgb * (Ambient + constantAmbientColour + liveLighting0diffuse * sat(N.L))
out = fogColour + fog * (rgb - fogColour)
```

(`v_mul_f32 v6, s24, v6` and its two twins put `Shadowed` on the texel at `0x130` to `0x150`; `Ambient` is `s1..s3`,
`constantAmbientColour` `s4..s6`, the sun `s8..s10`; `0x1c8` to `0x1fc` sum them). Its vertex-light twin `0x5064`
multiplies the finished sum by `Shadowed` (`0x1ec` to `0x1f4`). `crowd_diffuse_normal_shadow`'s `0x3c84` (113
instructions) has the same instruction shape: `Shadowed` loaded at `0xc8` and multiplied onto the three texel channels at
`0x118` to `0x130`, `Ambient` `s1..s3`, `constantAmbientColour` `s4..s6`, the sun `s7..s9` (read as a shape, not
instruction by instruction, so it carries the sprite's confidence, not more). Which permutation the race selects is not
read; `Shadowed` is not a string in the eboot, so no engine code sets it by name.

**What the wiring moved** (crowd-region bias, ours minus theirs in grey). Reproduce: on the build before the change run
`uv run --with numpy --with pillow --with scipy python3 scripts/compare-decompose.py --pair data/reference/compare-page/omega/<pair>
--work <dir> --binary <oag-game> --road road_alphaspec --crowd-mask <dir>/<pair>.npy` (it writes the mask: the pixels the
`crowd` skip render changes by more than 25 grey, undilated), then again on the new build with the same `--crowd-mask`
(the file is loaded, so both builds are scored on the same pixels; a darker crowd would otherwise shrink its own mask).
Before the wiring (the `panelmap-wire` tip, 16x anisotropy) to after: Sol +117.6 to +103.3, Queens Mall +53.4 to +40.6,
Subway +63.9 unchanged (it ships no `_shadow` sprite), Arena +62.2 to +15.1 (0.03 % of the frame, too small to judge).
The per-material Sol numbers are the same score over the pixels one material's own skip render changes
(`OAG_SKIP_MATERIAL=<file name>`; a one-off script, `--needle` cannot give them because `dynamic` removes the crowd
from it), the `_shadow` mask taken from a debug build that drew that sprite flat magenta, which also showed those draws
reach the branch: the `_shadow` sprites' 3,376 pixels go +160.0 to +57.6, the plain sprite's 46,650 go +121.5 to +116.6.

**The plain sprite was the alpha lane, then a missing light term** (2026-11-19, `omega-road-programs`).
`crowd_sprite_diffuse_normal_alpha` (Sol: 12 submeshes, 780 triangles) and `crowd_diffuse_normal` author no
parameter, and their programs (`0x41a4`, 112 instructions) declare no `Shadowed` or `Ambient`:
`T * (constantAmbientColour + sun * sat(N.L))`, which is what ours' generic path computes. Ours drew them white because
its slot word read the texture's **red** as the alpha ([below](#the-alpha-lane-2026-11-19-omega-road-programs)), not
because a factor was missing. With the lane fixed the plain crowd goes (crowd-region bias, ours minus theirs, same
pixels before and after): Sol +115.7 to +38.2, Queens Mall +48.5 to +4.7, Metro Park +14.4 to -10.8 (Subway and Arena
were not re-measured by region). What remains is Sol's +38.2 and Empire Climb's -37.2 (that crowd is under a low sun the original
lights and ours does not): the plain program's own light inputs (`constantAmbientColour`, `liveLighting0diffuse`, the
normal-map `N.L`) are still unread. The diagnostic of giving the plain sprite the `_shadow` sprite's constants
(`0.12178`, `0.777`) reached Sol +39.7 and Queens Mall +6.4, which the lane alone now matches, so that stand-in is
not needed.

**The plain crowd's light inputs, read (2026-11-20, `omega-crowd-light`): the program is ours' formula; the inputs are not
readable statically.** Three different things sit under the old "crowd region" number, and only one is a plain sprite:

- **Sol** (`environments2048/sol`): the plain `crowd_sprite_diffuse_normal_alpha` is 780 triangles in two instances
  (`#345` 9 submeshes, `#346` 3, both with no parameter): the few tall foreground figures. The 128,452-triangle dense
  crowd is the `_shadow` sprite. **Empire Climb** (`tower`) has no `_shadow` material: its crowd is the 3D
  `crowd_diffuse_normal` (335,577 triangles) and 2,155 plain sprite triangles. **Capital Reach** (`bridge`) is the
  `_shadow` pair and 944 plain sprite triangles; its crowd is 0.0 % of the frame (716 px), under the pose mismatch
  of item 5, so it cannot judge a change.
- **Program** (`gcn_dump` of the material, `Shdr` `0x41a4` sprite and `0x3c04` `crowd_diffuse_normal`, confidence 92): both
  are `T * (constantAmbientColour + liveLighting0diffuse * sat(N.L))` under fog, `N = nx T + ny B + nz V` from the
  vertex program's own frame (`0x3bc4`: `B = cross(N0, T) * tangent.w * worldScaleInv`, `V = eye - position`; this lane first
  read the third axis as the vertex normal `N0`, corrected in "The crowd sprite's frame" below), normalised, `N.L` in world
  space. `0x3c04` **never reads the diffuse alpha** (`dmask:0x7`) and has no kill: `crowd_diffuse_normal`'s alpha is
  its "specular in alpha" lane, not coverage. A probe that routed the plain materials through the `_shadow` path with
  `Shadowed = 1`, `Ambient = 0` (the same formula without the generic path's prelit, vertex-light and mask terms)
  left every pixel of all three pairs unchanged: **ours' generic path already computes the program**, so the
  discrepancy is in what it is fed. The static candidates for a permutation are `0x41a4` (sun), `0x4f44` (plus one
  vertex light) and the SH-probe twin `0x7904`; probes are per craft slot (`hull-tone.md`, "The probe file"), so a
  static crowd draw does not take it.
- **Per material, signed bias (ours minus theirs, grey, pixels owned by that material's skip render, masks pinned
  from the committed build), `mean|err|`** at 1920x1080: Sol plain sprite 15,198 px **+64.5, 78.9** (ours 111.1, theirs
  46.6); Sol `_shadow` sprite 11,955 px +9.4, 40.7; Sol `crowd_diffuse_normal` 344 px +32.8, 79.2; Empire plain sprite
  6,420 px -6.0, 44.0; Empire `crowd_diffuse_normal` 8,441 px **-51.2, 69.9** (ours 45.3, theirs 96.5); Capital
  `_shadow` 454 px -86.9, 88.1. The merged crowd-region numbers of the alpha-lane lane (Sol +38.2, Empire -37.2,
  Capital -55.8) are weighted sums of these.
- **Sol, what ours draws** (debug renders of `N.L`, the bound ambient and sun, the raw normal-map and diffuse taps,
  the frame's `fract(uv)`): every plain-sprite figure is either fully sun-lit or fully dark (`N.L` is near 1 or near 0
  per figure, 15,198 px with 2,351 of them at 231 grey against the original's 51), the sun term is
  `(7.6, 5.4, 3.7) * N.L` against an ambient near 0.8, and the original draws none of them sun-lit. Scaling the
  plain sum by the `_shadow` sprite's `Shadowed` 0.1 gives Sol plain +64.5 to -11.1, so the original's plain sprites
  carry about a tenth of ours' light on this circuit; negating `N.L` gives +52.1 and moves Empire's plain sprite
  -6.0 to -14.9 (not the law). Both are diagnostics, not wired. The atlas (`crowd_avatars_22x4`, BC7 sRGB; its `_n`
  BC7 UNORM, `2x - 1`) decodes to real figures and normals, so it is not a texture fault. **Open and odd:** a set of
  plain-sprite figures (1,287 white pixels that the plain skip render removes) did not respond to any debug return
  placed at the end of the lit stage, while the `Shadowed` probe darkened them: they take another route through the
  fragment function than the lit sum. That is the first thing to chase.
- **Empire Climb is not a crowd program.** The non-crowd structure beside the crowd (the same 8 px-dilated band, the
  crowd masks removed) is ours / theirs 0.69 against the crowd's 0.61, and the frame's sky is -64.7: the dimness is
  the scene-wide exposure and sun, the thread `omega-hd-exposure` works. It is not claimed for the crowd.
- **What would settle the inputs: a live read of the crowd draw's constant block.** The pixel block is `dw0
  constantAmbientColour`, `dw4 liveLighting0diffuse`, `dw8 liveLighting0direction`, `dw12 fogColour`, `dw16
  fadeResCoeffs` (80 bytes; many prop programs share it). On the native port (`scripts/wipeout-pc-road-sheen-live.py`'s
  walk to a 2048 grid, `--game-mode 2048 --track-steps 6 --class-steps 2` for Sol) find the staged copies by Sol's
  exact `liveLighting0direction` followed after four bytes by `fogColour` and read the three floats at `dw0` and `dw4`:
  they say whether the crowd's sun and ambient are per draw or the global frame constants. Global means the cause is
  ours' `N` or a route, per draw means a per-object light the engine computes. **Run 2026-11-20 (`omega-crowd-live`): global, and equal to ours' ([below](#crowd-and-scene-light-inputs-read-live-2026-11-20-omega-crowd-live)).**
- **2048: checked, differs** (as below: Vita draws crowds through `fc06_lambert_alpha`/`fc08_effects_crowd`). Omega
  ships a PS4 `fc08_effects_crowd` on Sol (336 triangles, 120 px, bias +9.6) whose program is unread.

**2048 (Vita) crowd: checked, differs.** Vita's circuits (`data/art/published/environments/<circuit>/track.rcsmodel`,
Sol, Queens Mall, Altima, Arena read with `psp2_material_dump`) draw their crowds through `fc06_lambert_alpha` and
`fc08_effects_crowd` (technique names `CrowdGenShader_crowd_avatars_22x4SG`), not a `crowd_sprite_*` or `crowd_*` material,
and none of those materials authors `Shadowed` or `Ambient` (the only parameters are `0x18a719bf`, `0xf139ff6d`,
`0x906b67ba`, the crowd-bustle values). The law has no Vita counterpart to read.

**2048 (Vita) lineage of the leading cause.** The road families are **checked, differs**: 2048's roads are
`fc16_phong_alphaspec_normal_emissive` with `track_a_floor_ds` (above, "Altima's road drew flat pink") and carry no
`tarmac_*` material (the tarmac section above), so neither Omega road program has a 2048 counterpart to read.

## Crowd and scene light inputs, read live (2026-11-20, `omega-crowd-live`)

**Claim tested:** the light inputs bound to Sol's crowd draws (`constantAmbientColour`, `liveLighting0diffuse` and its
direction, `Shadowed`, `Ambient`) can be read from the original, and the sky and fog inputs on Sebenco Climb where
reachable; each equals what ours binds or differs by a stated factor. **Read: yes. Result: equal, ratio 1.000 on every
input, on both circuits.** The plain crowd's dimness (Sol +64.5, the figures lit all or nothing) is therefore not the
bound ambient, sun or `Shadowed`.

**Method** (`scripts/wipeout-pc-crowd-light-live.py`, reads only; walk and rig as `wipeout-pc-omega-pair.py`, seat 2, about
150 s a boot). The port is frozen with `SIGSTOP` when the walk's grab loop ends (about five or six grabs after the HUD shows, still before GO: TOTAL reads `0.00.00` in `frozen.png`) and its guest memory is read through
`/proc/<pid>/mem`: (1) the frame-constant table (an entry is `~crc32(name)` with the value pointer at `+0x10`, as in
`wipeout-pc-road-sheen-live.py`; 125 to 420 entries per name across boots, the most common pointer into the image or heap is the
live one, a stale entry reads junk in its fourth lane); (2) every copy of the live `fogColour` xyz in memory, dumped with the
64 bytes before it and the 48 after it. The `_shadow` crowd sprite's pixel constant block is 96 bytes
(`gcn_dump Crowd_Sprite_Diffuse_Normal_Alpha_Shadow` `0x21e4`, 122 instructions): `dw0 Shadowed`, `dw1-3 Ambient`,
`dw4 constantAmbientColour`, `dw8 liveLighting0diffuse`, `dw12 liveLighting0direction`, `dw16 fogColour`, `dw20
fadeResCoeffs`; the plain programs (`0x41a4`, `0x3c04`) are its last 80 bytes. A block that starts with Sol's authored
`Shadowed 0.12178` and `Ambient 0.776742` is therefore the crowd sprite's by construction.
Written before the capture: "global" is falsified by a staged block whose `dw4`, `dw8` or `dw12` differs from the table,
"equal" by a value outside 0.92 to 1.08 of ours' per channel.

**Sol (`environments2048/sol`, 2048 mode, speed class A), original against ours' bound value** (ours: `authored_light`
passes the file's `Lighting.*` keys through unchanged; the sun is colour times `Sun diffuse intensity`; **measured**, not only read: `oag_raceplay::load` of `environments2048\sol` gives `ambient [0.8, 1.0, 1.2]`, `sun [7.608, 5.352, 3.72]`, `dir [-0.105345, 0.574048, -0.812017]`, `sun_specular [6.096, 3.2627, 1.6]`, and of `environments\10_sebenco_climb` `ambient [0.1804, 0.2275, 0.3529]`, `sun [0.3, 0.3, 0.3]`, `dir [-0.31888, 0.93355, -0.16370]`, `sun_specular [12, 12, 12]`, both with `output_scale 1`, because Omega leaves `lit_output_halved` unread; a one-off test, not kept):

| Input | Original, live | Ours (file) | Ratio |
| --- | --- | --- | --- |
| `constantAmbientColour` | `0.8, 1.0, 1.2` | `Constant ambient colour` `0.8 1 1.2` | 1.000 |
| `liveLighting0diffuse` | `7.608, 5.352, 3.72` | `0.951 0.669 0.465` times `8` | 1.000 |
| `liveLighting0direction` | `-0.10534, 0.57405, -0.81202` | `Sun direction`, normalised | 1.000 |
| `liveLighting0specular` | `6.096, 3.2627, 1.6` | `Sun specular colour` times `1` | 1.000 |
| `shadowLightDirection` | `0.79075, -0.37501, -0.48381` | `ShadowLight direction` `0.79073 -0.375 -0.4838` | 1.000 |
| `fogColour` | `0.34, 0.49, 0.566`, `w 0.00012` | `Fog colour` `0.34 0.49 0.566 0.000117` | 1.000 |
| crowd sprite `Shadowed`, `Ambient` | `0.12178`, `0.77674` x3 | the material records `0.12178`, `0.776742` | 1.000 |

The crowd sprite's block was found 54 times (boot A) and 52 times (boot B), 8 and 9 of them with the main view's
`fadeResCoeffs`: every one carries `constantAmbientColour 0.8 1 1.2`, `liveLighting0diffuse 7.608 5.352 3.72` and the
frame direction. **There is no per-draw light**: among the plain-layout blocks (the frame direction before `fogColour`)
of the main view, the ones that differ are the hull's probe blocks (`probeOffsetAndDiffuse` `x/8, K, K` in the slot where
the plain layout has the ambient), offset copies of them, and junk. One further `constantAmbientColour`, `0.5, 0.8, 1.0`
(114 and 123 blocks, the same sun), appears only with `fadeResCoeffs (100, 0.0333, 0.00391, 0.00391)`, the 256 x 256
pass (its `zw` is `1/256`; the main view's `(40000, 0, 0.00052, 0.00093)` is `1/1920, 1/1080`): a probe or reflection
capture with its own ambient, not the picture. The same `fadeResCoeffs` pair splits the crowd sprite's own blocks 46/8 and
43/9 between the 256 pass and the main view. A second `Shadowed 0.2`, `Ambient 0.0688 0.1199 0.183` block (78 + 14
copies, same frame constants) is another crowd material on the circuit; 46 `Shadowed 0.2` records exist.

**Sebenco Climb (HD mode, `10_sebenco_climb`), three boots** (the third after the pointer filter below):

| Input | Original, live | Ours (file) | Ratio |
| --- | --- | --- | --- |
| `constantAmbientColour` | `0.18039, 0.22745, 0.35294` | `Constant ambient colour` | 1.000 |
| `liveLighting0diffuse` | `0.3, 0.3, 0.3` | `Sun diffuse colour 1 1 1` times `0.3` | 1.000 |
| `liveLighting0direction` | `-0.31888, 0.93355, -0.1637` | `Sun direction` | 1.000 |
| `liveLighting0specular` | `12, 12, 12` | `4 4 4` times `3` | 1.000 |
| `fogColour` | `0.64639, 1.18384, 1.85203`, `w 0.0005` | `Fog colour` (HDR, above 1) | 1.000 |
| lighting object at `0x800eb2898` | `Sky height offset -85.102`, `Sky scale 3.13`, `Sky brightness 1.51`, `Ship diffuse brightness 0.5` | the same four keys | 1.000 |

Staged plain-layout blocks carry the same values (5,975, 5,480 and 4,402 blocks with the frame ambient, sun and
direction in the three boots). So the 2.3 to 2.5 times sky, the 1.4 stands and the 1.6 stripe road of the luma read are **not** carried by
any bound ambient, sun, direction, specular, fog colour or `Sky brightness` value: ours feeds the same numbers. The HDR fog
colour (`1.18, 1.85`) reaches the pixel constant unclamped, so a clamp, if there is one, is in a program or in the target,
not in the constant. What differs is therefore inside the programs and their textures: the sky material's program and the
sky cube, the consumers of `Sky brightness` and `Fog colour` (still unread in the 1.07 eboot), and for the crowd the
normal `N` the lit sum is given.

**What this leaves for the Sol plain sprite.** The program is ours' formula and its inputs are ours' inputs, so the
original's "no figure sun-lit" and ours' "each figure fully lit or fully dark" come from `N.L`: the normal-map decode, the
vertex program's TBN (`0x3bc4`, `worldScaleInv`), the world matrix of the crowd instances, or the permutation the draw
selects (`0x41a4` against `0x4f44`/`0x7904`). The next read is the crowd draw's **vertex** constant block (`world`,
`worldScaleInv`, `eyePositionWorldSpace`), found the same way (a copy of Sol's `worldScaleInv` followed by `fogColour`).

**Also answers the Omega 2048 road's globals.** The panel-map road programs read the same frame constants; on Sol they
equal the file's (sun, specular, ambient, `ShadowLight` direction, fog). The road's material parameters are separate.

**Confidence.** The frame constants equal the file's authored values: 92 (two Sol boots, three Sebenco boots, the table and
thousands of staged copies agree). The `_shadow` crowd sprite's draw reads exactly these: 80 (identified by the staged
block's `Shadowed`/`Ambient` prefix, 8 and 9 copies with the main view's fade; a copy in memory is a staging copy, not a
captured bind). The **plain** sprite (`0x41a4`, the `+64.5` figures) has no prefix to identify its block: its inputs are
inferred from every main-view plain-layout block carrying the frame constants, 70. That ours' loaded rig holds the same
numbers: 92 (measured from `Loaded.light` above; the shader's own use of it is the earlier debug render's
`(7.6, 5.4, 3.7)` and the code read of `shade.wesl`'s crowd path). One caution on the table
read: a hit of the hash that is not an entry can sit in the most common pointer (boot B of Sebenco picked a pointer that
reads zeros); the script keeps only pointers into the image or heap, and the staged blocks, which do not depend on it,
agree.

**2048: checked, differs.** Vita's crowds are `fc06_lambert_alpha`/`fc08_effects_crowd` (no `Shadowed`/`Ambient`); the
read is of Omega's native port, and Vita3K's constants for them were not read. HD: the front end and the sky/fog consumers
are the open part, not checked here.

## The crowd sprite's frame (2026-11-20, `omega-crowd-normal`)

**Claim tested:** the plain crowd sprites on Sol are lit the way the original lights them because ours builds their normal by the route their vertex program uses. **Half true.** The route is read and wired (the programs take the eye vector, not the vertex normal, as the map's `z` axis); the Sol plain-sprite box moves from +64.5 to +47.6, not to 0, and **the white silhouettes that make that box look wrong are not that material's pixels** (below).

**The route** (`gcn_dump` of `Crowd_Sprite_Diffuse_Normal_Alpha`, `oag_rcs::gcn::third_axis_is_eye`, confidence 85). Vertex program `0x3bc4` (base compile; the patch's `0x3ea4` is the same code with `eyePositionWorldSpace` in the second constant buffer, `ViewDep`) exports `param2 = cross(N_w, T_w) * tangent.w * worldScaleInv` (the bitangent), `param3 = eyePositionWorldSpace - world position` (three `v_sub_f32`), `param4 = N_w` and `param5 = T_w`. Pixel program `0x41a4` (the sun; `0x4f44` adds one vertex light, same frame) declares the vertex outputs in slots 0, 1, 2, 4, 5 (uv, bitangent, `param3`, `param5`, dead), not slot 3 (`param4`, the normal), and builds `N = nx * T + ny * B + nz * V` from the decoded `2t - 1` map: the `z` lane is multiplied by `attr3`, the interpolant of slot 2, which is `eye - position`. **The vertex normal is never read by the pixel program**; it only builds `B`. The normalised sum is dotted with `liveLighting0direction` as before. With the map's `z` near 1 the lit normal is the unit vector to the camera, bent by `x T + y B` over the camera distance (hundreds of units): each figure is lit as turned to the camera, which is why none shows its card's own facing.

*How the slot is read, and why it is trusted.* A pixel `attrN` is the Nth entry of the program's input list (position first); an input's slot byte names the vertex output with the same slot byte; that output's `__atg_paramK` name is the `exp paramK` that carries it. Checked on the 12 shadow-receiving, specular and Nova permutations of the same two materials (`0x87c4`, `0x9704`, `0xa8c4`, `0xb9a4`, `0xca44`, `0xdd64` and the patch's six): in every one the `z` weight resolves to an export that depends on the `normal` input alone, and the two unshadowed ones resolve to an export that depends on `position` alone. A mapping that was wrong would not give twelve consistent frames. The dead slot is the last vertex output in both compiles (`param6` here, `param7` on the vertex-light variants), declared by the pixel program and never read. The vertex-program constant buffer is told from the descriptor registers it loads through (a vertex program's header usage records are unread): the distinct descriptor registers, ascending, against the buffers by API slot, ascending; refused when the counts differ (confidence 70, unit-checked on the patch compile's `ViewDep`).

**Which materials.** `third_axis_is_eye` over a file's vertex and pixel pairs, **leaving out the permutations that read `shadowLightDirection`** (they weight `z` by the normal): `crowd_sprite_diffuse_normal_alpha`, `crowd_diffuse_normal` and their `_shadow` twins are eye on both compiles (2 pairs each, 2 eye); the skinned `_BoneCount4` crowd is not (0 of 3). The original picks the unshadowed permutation for the crowd draws: the `_shadow` sprite's staged pixel block is the 96-byte plain layout without `shadowLightDirection` (`omega-crowd-live`, confidence 80 for the `_shadow` sprite), and the plain materials are inferred to pick it too (70). **Not wired, and a lead:** the same census marks the unshadowed permutations of non-crowd props eye too (`Diffuse_Normal`, `tracktexture_with_normal`, `grassmudrock_blend_diffuse_normal_conditional`, `fc10_lambert_normal_alpha_uscroll`, `cb_treeleaf_normal`, `nr_crowd_bustle*`; ship materials in part). Applying the eye frame to every Omega normal-mapped draw as a probe moved Capital Reach 47.26 to 42.89 and Empire Climb 44.97 to 54.17, so it is not a general law until the permutation each draw takes is known; the crowd's is the one with evidence.

**Wired** (`mesh::rcs::psp2::crowd`, `hull_normal.wesl`): a crowd material whose file reads eye gets a `PROGRAM_CROWD` entry with `offset = 1`; the plain materials are the `_shadow` twins' sum with `Shadowed = 1`, `Ambient = 0` (the earlier probe left every pixel unchanged, so no formula moves); `shade.wesl` takes `scene.fog.camera - in.world` as the frame's third axis where the entry says so (`hull_frame`). The loader report says `{n} with the eye frame`. Tests: `gcn_ground_truth::the_crowd_materials_weight_the_normal_maps_z_by_the_eye_vector` (both compiles, and the skinned crowd as the control), `omega_crowd_shadowed_check` (every crowd vertex of Sol, Queens Mall and Rockway carries the entry with the eye frame), `omega_crowd_lit_path::the_eye_frame_lights_a_figure_turned_away_as_if_it_faced_the_camera` (drop the shader's third axis and the figure reads dark).

**Measured** (`scripts/compare-decompose.py`'s pair renders, 1920x1080, masks pinned per material from the committed build's skip renders; ours minus theirs, grey; before is main `6a33cf67d`):

| Pair, material (pinned px) | Bias before | Bias after | Mean abs before | Mean abs after |
| --- | ---: | ---: | ---: | ---: |
| Sol plain sprite (15,198) | +64.5 | +47.6 | 78.9 | 64.0 |
| Sol `_shadow` sprite (11,955) | +9.4 | +3.3 | 40.7 | 36.8 |
| Sol `crowd_diffuse_normal` (344) | +32.8 | +11.5 | 79.2 | 67.4 |
| Empire plain sprite (6,420) | -4.7 | -24.0 | 44.1 | 36.6 |
| Empire `crowd_diffuse_normal` (8,441) | -49.9 | -54.8 | 69.3 | 67.8 |
| Capital `_shadow` sprite (454) | -86.8 | -90.1 | 88.1 | 90.5 |
| Whole frame mean abs: Sol / Empire / Capital | | | 32.71 / 44.97 / 47.26 | 32.54 / 44.94 / 47.26 |

**"None in full sun" is not met:** under the Sol plain mask the pixels at or above 230 grey, the sun-lit figures (231 grey against the original's 51 in the earlier read), fall from 1,206 to 357, and 160 to 230 from 3,028 to 1,988; a figure still reads sun-lit where the eye is on the sun's side of its card. The mean absolute error falls on five of six boxes; the signed bias goes more negative where ours was already too dark (Empire, Capital: the scene-wide exposure of the thread `omega-hd-exposure`). Whole frames are within noise (under 0.3 grey). HD `hd-race/talons-pack` and `2048-vita/moa-therma`: 0 differing pixels before and after (the entry is built for a PS4 model only).

**What the box cannot show.** Under the Sol plain-sprite mask the original is a flat 45 to 55 grey whatever ours draws there (binned by ours' value, theirs stays 45 to 55): the original's crowd stands in other places (the port randomises it boot to boot), so the mask holds ours' figures against the original's floor. The box is a bias of a placement difference as much as of lighting. **The white silhouettes** in the Sol frame (13,122 near-white pixels in the stands; **closed**: [the crowd-entity shapes](#the-crowd-entity-shapes-2026-11-20-omega-sol-silhouettes), `fc01_colour_emissive` placeholders, no longer drawn) are **in neither the plain nor the `_shadow` mask**, are untouched by `OAG_SKIP_MATERIAL=crowd` and by a debug return at the end of `lit_texel` for every `PROGRAM_CROWD` draw, and are not `fc06_lambert_emmissive_alpha`. They are another draw (the animated `Data\crowd` rigs are the first candidate: the skinned `_BoneCount4` materials, the one family `third_axis_is_eye` says is not eye, are not admitted by `crowd.rs`); finding which draw is the next read. The earlier lane's "1,287 white pixels that take another route" are **not** these: they were removed by the plain skip render, so they are plain-sprite pixels, and with a debug return at the end of `lit_texel` no pixel under the plain mask stays white (0 of 15,198 above 240 grey), so every in-mask pixel reaches the lit sum. The silhouettes are a separate draw outside every crowd mask.

**Open:** the permutation the original takes for the plain sprite is unread (70); a live read would stage the pixel block of a plain sprite draw (the five-field layout) against the shadow-receiving one (`shadowLightDirection` between the direction and the fog).

**2048 (Vita): checked, differs.** Vita draws crowds through `fc06_lambert_alpha` and `fc08_effects_crowd` (no `crowd_*` material), and its GXP programs' operands are unread, so the frame is not checkable there. HD (PS3): the hull programs build `N = nx T + ny B + nz N0` (`hd-ship-materials.md`, "Hull normal maps"); a Cg frame read for HD's crowd is not done.

## The crowd-entity shapes (2026-11-20, `omega-sol-silhouettes`)

**Claim tested:** the white silhouettes in Sol's stands (13,122 near-white pixels the original draws dark or as lit figures) are drawn the way the original draws them. **False as stated, and closed another way: they are placeholders the original does not draw, and Omega no longer draws them.** The original draws animated `Data\Crowd\Rigs` figures there; those are not loaded (open, below).

**What draws them (confidence 90).** An `OAG_SKIP_SLOT` bisection over `Data\environments2048\sol\track.final.rcsmodel`'s 376 material slots (each half-skip rendered, the white count under the mask read) puts them in **slot 5, `shared/objectlibrary/materials/fc01_colour_emissive.rcsmaterial`**: no texture, one parameter `Colour` (hash `02ab9f07`) = `(1, 1, 0)`, 203 submeshes, 60,697 vertices, every one of them a mesh object named `trackcrowd:...crowdentityN_MShape`. Its program (`gcn_dump`, vertex `0x29c4`, 55 instructions, position only; pixel `0x2d64`) exports `mix(Colour, fogColour.rgb, fog)` with no sampler and no light: a flat placeholder colour. That is why `OAG_SKIP_MATERIAL=crowd` (needle on the material and texture paths), the tint switch and a `lit_texel` return did not touch them. The other slots whose skip drops the white count (4, 66, 105, 272, 331, 339, 0.6 to 5 kpx each) are real scenery whites (balloon, panel-map road, billboard) in the raw white mask, not the silhouettes.

**The marker is the mesh object's flags, not a name (confidence 95).** Every crowd-entity shape carries `+0x0a` flags `0x0401`; the track's other mesh objects carry `0x0101` or `0x0201`, the crowd rigs `0x0306`, `LODs.rcsmodel`'s 119 shapes `0x0501`. Census over every `.rcsmodel` of `data00` to `data04`: **17,701 mesh objects carry `0x0401` and 17,701 are named `crowdentity`, the same objects** (319 on `data00`, 3,860 on `data01`, 11,104 on `data02`, none on `data03`, 2,418 on `data04`). Compared whole, not as a bit: `LODs.rcsmodel` has `0x0501`. `oag_rcs::rcsmodel::psp2::nodes::MeshObject::is_crowd_entity` reads it; `crates/raceplay/tests/omega_race_ground_truth.rs` (`check_crowd_entity_flag`, run in every archive's census) holds it.

**What the original does with them (confidence 70).** Omega's track loader (`v1.07 eboot`, `FUN_0162a390`; strings at `0x01890cc1` `Data/Crowd/Rigs/`, `0x01890ce9` `|crowdentity%d|LOD1|LOD1Shape`, `0x01890dbf` `CrowdEntity`) first looks up sixty `|crowdentity%d|LOD1|LOD1Shape` and `LOD2Shape` mesh objects by hash in a library model (`LODs.rcsmodel`: 119 meshes), then walks the track model's mesh objects for a name containing `CrowdEntity`. For each it reads the digits and the `w`/`s` suffix letters to pick a crowd group, draws a random value for the instance, pushes an instance record (`FUN_0163d3c0`) and **clears draw bits of the object's runtime flag word (`& 0xfffffff9` when animated crowds are on, `& 0xfffffff8` when off)**. The bit meanings are not read, so the claim is: the loader replaces each entity with a rig instance and takes the placeholder out of the draw; it is not read what the flag word's bits are. The frames agree: at those positions the original shows lit coloured figures and no yellow or white flat shapes. The game has an option for it (`Animation.Animated crowd in split-screen`, `0x0188dbb0`).

**Wired (Omega only).** `psp2::build` does not emit a submesh of a mesh object with `is_crowd_entity()` when the model is PS4's, counts them apart from `hidden` (`Report::crowd_entities`: Sol 203, Amphiseum 907, Vineta K 758). Each is replaced by a rig figure: [the crowd rigs](#the-crowd-rigs-2026-11-20-omega-crowd-rigs).

**Boxes** (Sol, 1920x1080, ours minus theirs in grey; mask = the pixels `OAG_SKIP_SLOT=5` changes by more than 25 grey after the frame gain is divided out, outside the HUD boxes, pinned from the render before the change: 23,774 px, of which 7,967 are at or above 235 grey before):

| mask | bias before | after | mean abs before | after |
| --- | ---: | ---: | ---: | ---: |
| slot-5 pixels (23,774) | +75.9 | -21.8 | 95.7 | 35.8 |
| of them white before (7,967) | +200.7 | -13.6 | 200.8 | 38.2 |

The brief's 13,122 was counted with a looser threshold on the whole frame; the raw `ours > 235 and ours - theirs > 60` mask is 15,571 px and holds 4 to 5 kpx of the balloon and the road. Whole frame 29.41 to 28.81 (bias -2.61 to -3.67). The 8 px ring around the mask is unchanged (mean abs 46.1 to 46.3). **What is left is not the placeholder's: ours is 22 grey too dark under the mask** (26.5 against 48.3), because the original draws rigs there and ours draws the dark stand. Across all 24 Omega race pairs the whole-frame mean abs against the original is flat or better (Metropia 32.30 to 31.17, Chenghou 27.36 to 26.71, Vineta K 25.77 to 24.82, Amphiseum 49.20 to 48.71, Moa Therma 35.95 to 35.29, Sol 29.41 to 28.81; Sebenco Climb +0.04, Modesto +0.02); `anulpha-pass`, `sol-2` and `zone-vineta-k-z5` draw no crowd entity and are pixel-identical. `hd-race/talons-pack` and `2048-vita/moa-therma`: 0 differing pixels.

**Amphiseum.** The 907 grey node-bound people of the thread `omega-amphiseum-spectators-draw-grey-and-the-stands-stay-light` (the `emissive` material, `EmissiveColour` 0.5566) are these shapes: "a runtime swap per crowd entity" is the loader above. They no longer draw; each is replaced by a rig.

**2048 (Vita): checked, differs.** Not one mesh object of any Vita archive (`data`, `dlc1`, `dlc2`, `data1`, `data2`) carries `0x0401` or the name `crowdentity` (0 of every parsed `.rcsmodel`; `vita_models_carry_no_crowd_entity_shapes`): its crowds are sprite materials, so nothing of this rule applies to a Vita frame. HD (PS3): not checked (a Cg model, no `psp2` reader); its crowds are `NR_crowd_bustle` materials.

**Open.** Moved to [the crowd rigs](#the-crowd-rigs-2026-11-20-omega-crowd-rigs).

## The crowd rigs (2026-11-20, `omega-crowd-rigs`)

**Claim tested:** Omega's `Data/Crowd/Rigs` people stand at the track's `crowdentity` positions on Sol, as the original draws them, and under Sol's silhouette mask ours moves from 26.5 toward the original's 48.3. **Partly true.** A figure stands at each of Sol's 203, Amphiseum's 907 and Vineta K's 758 entities, in the bind pose; the mask mean and bias move toward the original (26.7 to 29.1 against 48.5, bias -21.7 to -19.3) and the mean absolute grey does not (35.8 to 36.8): the figures are thin, high-contrast and posed unlike the original's animated ones, so a pixel the empty stand matched by being dark is now a mismatched figure edge. The whole frame moves 28.65 to 28.66 (under the 0.3 noise floor). Steps 1 to 3 below landed; the clip does not play.

**Step 1: the 82 unpaired pointers are the rigs' and `start.rcsmodel`'s (confidence 85).** 41 records, vertex pointer 200 bytes past the index pointer, the declaration two pointers deep: [`2048-rcsmodel.md`](2048-rcsmodel.md#omegas-skinned-record-a-fourth-gap-200-bytes-2026-11-20-omega-crowd-rigs). `data00` closes to 0 unpaired (625 files, 38,403 submeshes); the 40 rigs decode with weights summing to one, bone indices inside their skeleton and node ids equal to the skeleton's.

**Which rig stands where (`FUN_0162a390`, v1.07 eboot; confidence 85, static read of the loader, its init table and strings, not a live instance).** The tail from `0x0163138a` loads 40 rigs, `Data/Crowd/Rigs/<name>` `.rcsmodel` and `.rcsskeleton`, named by the pointer table at `0x0198fb20` (strings from `0x01890af5`): slots 0 to 4 `Male01..05`, 5 to 9 `Female01..05`, 10 to 19 their `_Shadow` twins, 20 to 29 `Male06..10`, `Female06..10`, 30 to 39 their twins. Per mesh object it finds `CrowdEntity` by a case-insensitive scan, `sscanf`s the digits `N` (1 when none), looks for `s` and `w` among the two characters after the digits, and forms the group index `N - 1 + 15 * (s + 2 * w)`, clamped to 59. The 60-entry group table is written by `FUN_0163b720` (a static initialiser, `0x0201e500`, one `u32` pair per group: rig slot, clip): groups `0..14` are slots `0 1 1 2 2 3 3 4 4 0 5 6 7 8 9` with clips `0 0 1 1 2 2 3 3 4 4 5 5 5 5 5`, and each later block of 15 adds 10 to the slot (`s`: the shadow twin; `w`: the second texture group; both: the twin of that). **So the digits pick the person, `s` the shadowed twin and `w` the `06..10` set; nothing random picks the rig** (`rand()` draws the clip's start offset, below). On Sol: `6` is `Male04`, `9` `Male05`, `11` `Female01`, `14` `Female04`, and the 51 `s` entities their twins. The six clips (`FUN_0163d6f0` strings at `0x0201bf18`): `male_01_folded_arms`, `male_02_frontrow_aggressive`, `..._aggressive2`, `..._aggressive3`, `male_05_aggressive_long`, `female_05_cheering`. `psp2::crowd_rigs::group` and `rig_stem` replay this and are unit-tested against it; the plain rigs `male01..05` and `female01..05` share a geometry per gender and half (four shapes, 452, 565, 492 and 524 vertices), told apart by their UVs into `crowd_group_01d`/`02d`.

**How it draws.** `psp2::crowd_rigs::instance` merges one rig submesh per entity into the decoded model before the build plans it: vertices times 100 (the skeleton's root bone is scale 100; the figures are 1.69 to 1.75 tall after it), node-bound to the entity's own node so the existing placement bakes its bind matrix (Sol's entities carry uniform scales of 2.28 to 2.41, and the placeholder is 1.74 tall in the node's space, so a figure is about 4 units tall), the rig's material appended to the table, `mesh` left `None` (so nothing reads the rig as the entity or a track shape) and the entity's mesh index carried as the draw's PVS chunk (it is culled with its placeholder). Confidence 70 for the placement: the frame agrees (figure heights on the right stand's front row match the original's within about 10 % by eye), and the instance record's reciprocal of the node's rms scale (`fVar25`, stored beside the group and the start offset) is **read and its use is not**. A hidden or unplaced entity node draws no rig, as its placeholder did not.

**The skinned materials take the crowd program.** `Crowd_Diffuse_Normal_BoneCount4`, `..._NormalizeTS` and `Crowd_Diffuse_Normal_Shadow_BoneCount4_NormalizeTS` have the same pixel programs as their unskinned twins instruction for instruction (`gcn_dump --summary`: 52/119/125/115 for `_NormalizeTS`, 52/104/110/100 for the bare family, 60/128/131/123 shadowed) and vertex programs that add `skinPalette` and read `eyePositionWorldSpace`; `third_axis_is_eye` reads the skinned vertex programs as not eye-framed (its pattern does not see through the palette), so `psp2::crowd` treats the three names as eye-framed by that identity (confidence 80). Two disc quirks the census pins: `female09` ships the bare `Crowd_Diffuse_Normal_BoneCount4`, and `male07_shadow` the plain `_NormalizeTS` material, not the shadow one. `Shadowed`/`Ambient` come from the `_shadow` twin's own two parameters (a crowd entry carrying the `male01_shadow` rig's `Shadowed` is pinned in `omega_panel_map_ground_truth`).

**Boxes** (Sol 1920x1080; mask = the pixels `OAG_SKIP_SLOT=5` changes by more than 25 grey on a build that draws the placeholders, **rebuilt**: 23,213 px, no frame-gain division, no HUD exclusion, against the previous report's 23,774; the same baseline reproduces its numbers to within 0.2: ours 26.7 against 26.5, theirs 48.5 against 48.3, mean abs 35.8):

| | before | after |
| --- | ---: | ---: |
| ours, mean grey (theirs 48.5) | 26.7 | 29.1 |
| ours minus theirs, bias | -21.7 | -19.3 |
| mean absolute grey | 35.8 | 36.8 |
| whole frame mean absolute grey | 28.65 | 28.66 |

Before the crowd program was given to the skinned materials the same figures drew through the generic lit path: mask mean 41.6, bias -6.9, mean abs 43.7 (brighter, worse where it counts). **All 24 Omega race pairs** (before = this branch's binary with the rigs left out by a temporary environment switch, not `main`; all 1920x1080, `compare-rerender-ours` flags on copies of the pairs): `anulpha-pass`, `sol-2` and `zone-vineta-k-z5` are pixel-identical (no entity); 21 move, none by more than 0.13 grey of whole-frame mean absolute grey against the original (all under the 0.3 noise floor): better `modesto-heights` -0.13, `vineta-k` -0.05, `metropia` -0.04, `chenghou-project` -0.03, `talons-junction` -0.02, `subway-2048` -0.01; worse `the-amphiseum` +0.06, `moa-therma` +0.06, `altima-2048` +0.04, `queens-mall-2048` +0.03, `square-2048` +0.03, `tech-de-ra` +0.02, `empire-climb-2048` +0.02, `sebenco-climb` +0.02, `sol-2048` +0.01, `ubermall` +0.01, `metro-park-2048` +0.01; level `arena-2048`, `capital-reach-2048`, `downtown-2048`, `zone-capital-reach-2048-z5`. Pixels differing by more than 10 grey: 17.9 k on Modesto, 16.8 k Amphiseum, 15.9 k Vineta K, 14.7 k Moa Therma, 13.0 k Metropia. **`hd-race/talons-pack` and `2048-vita/moa-therma`: 0 differing pixels.**

**Open (nothing here is a guess drawn as fact).**
- **The pose.** Bind pose: arms out at about 30 degrees. The group's clip is unread in use: per-vertex skinning (the weights and indices are decoded, `SubMesh::skin_weights`) with a skeleton and clip frame (`rcsskeleton`, `rcsanimclip` read; `rig::NodeMotion::sample`) would pose a frozen frame on the CPU; playing it needs a palette in the vertex stage. The start offset is `-(rand * 4.5 + 0.5)`-shaped, from the pair `(5.0, 0.5)` at `0x0201bf4c` (the product `rand * -9.313226e-10` is `rand / 2^30`; the exact range is unread).
- **The draw bits** `FUN_0162a390` clears on the placeholder's flag word at `+0x24` (`& 0xfffffff9` when animated crowds are on, `& 0xfffffff8` when off; the switch is `DAT_0201280c`/`DAT_02012811`/`DAT_02012810`/`DAT_02212c38`) are unread; with animated crowds off the loader pushes no instance and what draws in the placeholder's place is unknown (ours draws the rig in every case).
- **Density.** The original's stands look fuller than ours at the same camera (hundreds of figures on Sol's left stand against about 100): whether one entity stands for more than one instance is unread.
- **LODs.** `LODs.rcsmodel`'s 60 `LOD1`/`LOD2` quads per group are the distance shapes; unused.
- **2048 (Vita): checked, differs.** 2048's base package lists no `crowd/rigs` path (0 hits of 18,430 entries), none of its `.rcsmodel` has a `0x0401` shape (`vita_models_carry_no_crowd_entity_shapes`) and no Vita file has a gap-200 pair; its crowds are sprite materials. **HD (PS3): checked, differs**: none of the seven `DATA00..06.PSARC` lists a `crowd/rigs` path (0 hits of 11,564 entries); its crowds are `NR_crowd_bustle` sprite materials.

## The sky dome's pixel program (2026-11-20, `omega-sky-program`)

**Claim tested:** Omega's HD-mode sky is drawn by the law its own sky and fog programs compute, so the two sky boxes improve
(`omega/sebenco-climb` snow sky `1100,40,300,160`: theirs 162.3, ours 97.3, bias -65.0; `omega/modesto-heights` sky gap
`800,160,240,100`: theirs 192.4, ours 156.4, bias -36.0). **Modesto's box: true. Sebenco's box: false for the sky program.** The dome is drawn by one tiny program,
`SkyBrightness * texture`, which ours did not apply; wiring it cuts the sky-pixel mean absolute error on Moa, Sebenco and Modesto
(38.4 to 13.5, 37.5 to 14.3, 43.4 to 33.8) and moves Modesto's box 156.4 to 207.7 (an overshoot, below). **Sebenco's box is not the dome**: the dome owns 15 % of its pixels, the snow
mountain materials 48 % (below), so that box moves 97.3 to 100.1 only.

**Census** (`OAG_SKIP_MATERIAL`, 1920x1080, a pixel counts when it changes by more than 25 grey): the dome (`skycube.rcsmodel`,
six faces, `SkyCube.rcsmaterial` and its `_1`/`_2` twins) draws 79 % of Modesto's box, 15 % of Sebenco's and 1.6 % of
Sebenco's frame (6.0 % of Modesto's, 1.5 % of Moa's). The rest of Sebenco's box is `RT_snow1` (a material needle `rt_snow`
brightens the box 97 to 109 when skipped), the "snow" family as a whole 48 % of the box pixels. The dome material's pixel
programs carry no fog and no vertex colour.

**The program** (`gcn_dump data08.psarc data/environments/10_Sebenco_Climb/SkyCube/SkyCube.rcsmaterial --at 0x9e4`; the three
pixel variants `@0x9e4`, `@0x1864`, `@0x2304` are the same 52 instructions, on `environments2048` and `modesto_heights` too;
data02, data05 and data08 ship three revisions of it; confidence 90, decoded GCN, no run):

```text
t   = image_sample(Texture1Sampler, attr1.xy)              @0x2c, dmask 0x7
out = SkyBrightness * t.rgb                                 @0xd0-@0xd8, mrt0 as f16
```

The rest is the two motion-vector targets (`mrt1`, `mrt2`: the position over `max(0.5, w)` against `fadeResCoeffs`). The face
`.gnf` is BC7 `ChannelType::Srgb` (descriptor word `0x26900008`; 4096x4096 with mips by its 22,374,656 bytes), so the multiply is in linear light.
`SkyBrightness` is **engine-bound, not material-authored**: the material table holds no record for it (its `~crc32` appears
only in the three pixel programs' reflection, `@0xd50`, `@0x1bd0`, `@0x2670`). The lighting object's `Sky brightness` (`+0x160`,
read live as 1.51 on Sebenco, equal to the key: "Crowd and scene light inputs, read live") is the candidate; the frame-constant
table's own `SkyBrightness` entry was not read (**read recipe**: `scripts/wipeout-pc-crowd-light-live.py`'s table walk for
`~crc32("SkyBrightness")`, or the staged 32-byte block with `SkyBrightness` at dw0 and the main view's `fadeResCoeffs`
`(40000, 0, 1/1920, 1/1080)` at dw4 to dw7). The fit below is the evidence the two are equal (confidence 75).

**Wired** (`omega-sky-program`): `Looks::sky_brightness` (Omega, `Measured`) and `psp2_sky_model` multiplying the dome's vertex
colour by `SceneStore::texel_gain(Lighting.Sky brightness)`. The dome's shade is `decode(texel * colour)` in the gamma domain
(`shade.wesl`, the stand-in path), so a linear gain `B` is carried as the sRGB encode of `B` (1.198 for 1.51, 1.428 for 2.264); that
is exact at no texel and reads 1.9 % (1.51) and 3.4 % (2.264) low at a texel of 0.5, 6 % and 11 % low at 0.2, never high
(`a_texel_gain_decodes_to_about_the_gain`). **An exact multiply after `decode` needs a per-draw field the uniform block has no
free slot for** (34 test files mirror its layout); recorded as the follow-up. Tests: `omega_sky_brightness_ground_truth`
(Sebenco of the HD lineage and Sol of the 2048 lineage: the loaded dome's vertex colour is the authored gain and the loader line names it),
`texel_gain` unit test. The 2048 (Vita) dome keeps colour 1.0.

**Measured, sky-only pixels** (the pixels `OAG_SKIP_MATERIAL=sky` changes by more than 25 grey, HUD boxes left out; mean grey,
ours minus theirs; base to wired; theirs in brackets):

| Pair | sky px | authored `Sky brightness` | bias | mean abs grey | mean RGB ours (theirs) |
| --- | --- | --- | --- | --- | --- |
| Moa Therma (control) | 1.48 % | 2.0 | -36.4 to -2.3 | 38.4 to 13.5 | 62.6/108.1/108.5 to 81.4/146.4/146.7 (95.1/145.9/143.1) |
| Sebenco Climb | 1.57 % | 1.51 | -36.8 to -12.8 | 37.5 to 14.3 | 75.9/117.2/159.2 to 90.7/143.0/192.7 (111.0/154.2/199.8) |
| Modesto Heights | 5.96 % | 2.264 | -14.5 to +23.1 | 43.4 to 33.8 | 121.4/142.1/169.3 to 158.7/180.9/196.2 (134.5/157.3/180.8) |

**The two named boxes** (`compare` boxes, ours re-rendered at the pair's recorded flags; mean grey, bias ours minus theirs, mean absolute grey):

| Box | theirs | ours before | ours after | bias before to after | mean abs before to after |
| --- | --- | --- | --- | --- | --- |
| Sebenco snow sky `1100,40,300,160` | 162.3 | 97.3 | 100.1 | -65.0 to -62.1 | 65.2 to 62.5 |
| Modesto sky gap `800,160,240,100` | 192.4 | 156.4 | 207.7 | -36.0 to +15.2 | 46.0 to 30.9 |

**Why Modesto overshoots, and why that is not the law.** Sebenco's tone map is pinned (`E` 0.555 and `t1` 0.612 equal ours'), so its
sky is a direct read: the G and B channels land (143.0/192.7 against 154.2/199.8) and the red is short by 20 (the original's bloom
adds the same light to every channel and a dark channel gains most after the curve; ours draws none there). On Modesto ours'
exposure is `E` 0.868 against the original's 0.60 (the dim-scene symptom of the luma read), and an exposure forced to 0.6 on a
prototype build (a temporary `Tonemap` override, not kept, with the gain `B^(1/2.4)` = 1.406, 1.5 % below the committed sRGB-encode
factor 1.428, so the committed build reads about 2 to 3 grey higher) gives sky pixels 133.6/157.5/184.4 against 134.5/157.3/180.8:
**bias +0.2, every channel within 4**. The same override without the gain reads bias -41.0 (113.1 against 154.1). So the law is right and
Modesto's remaining sky error is the rest of its scene being too dark for `LAvg`.

**Whole frames, all 22 Omega race pairs re-rendered** (raw mean grey ours / theirs, mean absolute grey to the original; before to
after). HD mode: Anulpha 21.19 to 20.94, Chenghou 27.44 to 27.36, Metropia 32.14 to 32.13, Modesto 40.77 to 39.08, Sebenco 40.23 to
39.86, Sol 2 32.54 to 31.40, Talons 27.89 to 26.89, Tech De Ra 27.76 (0 pixels differ), Amphiseum 49.11 (unchanged), Ubermall 20.28
to 20.19, Vineta K 28.45 to 25.80, Moa Therma 36.39 to 35.95. 2048 mode: Altima, Arena, Subway unchanged, Capital Reach 47.26
(unchanged), Downtown 26.72 to 26.71, Empire Climb 44.97 to 43.94, Metro Park 43.67 to 43.89 (+0.22, the one worse), Queens Mall 43.72
to 42.58, Sol 32.71 to 29.57, Square 25.96 to 25.95. Seven pairs move the whole frame more than 0.3 grey (Modesto -0.25 is the
nearest miss); the raw mean moves toward the original on all seven, and Metro Park's mean absolute grey is the one that gets worse.

**Fog: Omega's curve is read now.** The vertex program of `RT_snow1`'s lit variant (`@0x12424`, 120 instructions) computes
`t = fogColour.w * v7`, `F = exp2(1.4427 * (-t * t))` and passes it as `attr1.w`; the pixel program ends
`out = lit * F + fogColour.rgb * (1 - F)` (`@0x7b0-@0x7d8`) with `fogColour` unclamped. That is `exp(-(density * depth)^2)`,
ours' curve (confidence 85; one material, `v7` taken to be the view depth, whose definition was not traced). The loader still
labels Omega's fog **INHERITED** from HD; the label is stale for the HD-lineage circuits' materials and left for the loader's next
change. Fog is not what the Sebenco mountain misses: drawing no fog moves the box 100.1 to 96.5, four times the density
128.4, a brightening by the HDR fog colour.

**What owns Sebenco's box: the snow programs (open, not wired).** `RT_snow1`'s lit variant is 419 instructions with constants
`snownormDetailStrength/Scale/LargeScale`, `Fresnel{Min,Scale,Power,Strength,Colour}`, `SpecStrength`, `SpecColour`, `SpecTwinkleScale`,
`DiffuseWrap`, `SpecPower`, `liveLighting0specular` (12, 12, 12 on this circuit), `shadowLightSpecularDiffuse` and a shadow map, over
five samplers (`Texture1`, `DriftNormalTexture`, two snow-normal detail scales, `Texture2`). The siblings (`RT_snow`, `RT_snow_ref`,
`Sebenco_snow`, `Sebenco_snow_just_snow` and their `_new`/`_1`/`_2` twins) run 255 to 485 instructions. Ours draws these through the
generic path: HD's `sebenco_snow` has its own wiring (`mesh::rcs::snow_rock`, read off PS3 microcode) and the GCN programs have none.
In the frame the original's mountain is white with a bright sheen where ours is lilac, which a sun specular of 12 times
`SpecStrength` and a Fresnel colour would produce and the generic ambient-plus-sun sum cannot. **Read and wired by `omega-snow-program` ("The snow program", below).**

**Cross-checks.** 2048 (Vita): **checked, applies, not wired.** `altima`'s `SkyCube.rcsmaterial` carries the parameter name
`SkyBrightness` and the same circuits author `Lighting.Sky brightness`, but its pixel program is USSE and unread, so the multiply is
not stated for it and its dome keeps the plain texture (`2048-vita/moa-therma` re-rendered before and after: 0 pixels differ).
HD: **checked, differs** (its sky is a `sky.gtf` cubemap with `Sky colour` bytes and no `Sky brightness`,
`renderer.md`, "The sky law, read off live draws"); the HD race pair (`hd-race/talons-pack`) re-rendered before and after: 0 pixels
differ.

## The snow program (2026-11-20, `omega-snow-program`)

**Claim tested:** Omega's `RT_snow1` snow family on Sebenco Climb is drawn by the law its own GCN program computes, so the
snow-owned part of the box `1100,40,300,160` on `omega/sebenco-climb` improves (box 100.1 against theirs 162.3, bias -62.1,
mean abs 62.5, 48 % snow by the sky lane's census). **True.** The box reads **133.2** (bias -29.1, mean abs 30.4), the snow pixels of the
whole frame go from mean 93.9 against theirs 186.3 (bias -92.4, mean abs 92.7) to **176.2** (bias -10.1, mean abs 15.3), and the whole frame's
mean abs against the original goes 39.86 to 36.15 (raw mean grey 81.85 to 85.77, theirs 103.77).

**The materials in the frame.** `rt_snow1` is 23 submeshes and 84,203 triangles of the circuit model (`track.final.rcsmodel`, data02), the one
snow material the race draws besides `rt_snow_ref` (one submesh, three more samplers, left generic); the `Sebenco_snow*` family is named by
no model. Every `rt_snow1` submesh declares `position`, `normal`, `tangent`, `Uv1`, `VertexColour1`, `VertexColour2` and `NOVAColor` and no
`lightmapUV` (`psp2_material_dump` `DECL_SUB`). The file ships ten pixel permutations (189 to 419 instructions); the inputs pick the one
**with `NOVAColor`, no lightmap, no shadow map and no vertex light: pixel `@0xaa24` (308 instructions) with vertex `@0xa1e4` (111)**
(the pixel's nine inputs are the vertex's nine outputs; the 419-instruction `@0x110e4` this thread named first is the lightmapped, shadow-mapped
permutation, whose vertex program takes `lightmapUV`, and the `@0x12424` fog read in the sky section is its sibling's). Which permutation the race
binds for the vertex-light count is unread (confidence 70): the 1-vertex-light permutation `@0xf6e4` is the same law with one more term.

**The law** (confidence 85, decoded GCN `oag_rcs::gcn`, no run; the shared core was read on `@0x110e4` first, then `@0xaa24` against it):

```text
vertex:  attr1 = (VertexColour2.rgb, VertexColour1.x)      attr2.w = NOVAColor.a      attr4 = scale * |NOVAColor.rgb|^power + bias
         attr5 = bitangent  attr6 = eye - position  attr7 = N  attr8 = T            fog F = exp(-(density * depth)^2) in attr0.w
pixel:   m   = attr1.w                                      drift mask
         d   = lerp((0.21952, 0.21952, 1), 2 DriftNormal(uv) - 1, m)
         s   = 2 (Detail(uv * snownormDetailScale) + DetailLarge(uv * snownormDetailLargeScale)).xy - 2     h = 1 + Strength (2 (b1 + b2) - 3)
         n   = normalise(Strength d.z s + d.xy h,  d.z h)  as  n.x T + n.y B + n.z N
         F   = FresnelMin + FresnelScale (1 - N.V)^FresnelPower
         w(x) = sat((x + DiffuseWrap) / (1 + DiffuseWrap))
         Fsl = 0.75 + 0.25 w(N.SL) + SLSD.y w(R.SL)^SLSD.x                       SL, SLSD the circuit's ShadowLight
         D   = w(N.L) liveLighting0diffuse attr2.w + attr4 Fsl                   (attr2.xyz is 0 in this permutation)
         out = fog(F FresnelStrength FresnelColour attr1.rgb Texture1 D
                   + SpecStrength SpecColour liveLighting0specular Texture2(uv * SpecTwinkleScale).a attr2.w w(R.L)^SpecPower)
```

`Texture1` and `Texture2` are both `and_snow3alpha` (one picture, its alpha the specular gloss); the drift normal is `glacier_detail_001_less_n`,
the small detail `powdersnow_less_n`, the large one `ground_beach_wet_sand_uk_001_n`. **The constants are the material's own** (`~crc32` of their
names; the record authors thirteen): `DiffuseWrap` 2.624, `FresnelMin` 1, `FresnelScale` 2.151, `FresnelPower` 1.749, `FresnelStrength` 0.709,
`FresnelColour` `(0.787, 0.850, 0.897)`, `SpecStrength` 0.1, `SpecColour` `(1, 0.876, 1)`, `SpecPower` 10, `SpecTwinkleScale` 3.78,
`snownormDetailStrength` 1.631, `snownormDetailScale` 14.66, `snownormDetailLargeScale` 12.29. **Two terms carry the move, by ablation** (each zeroed once on a temporary build, box and snow pixels; wired 133.2 and 176.2): the diffuse wrap set to 0 gives box 119.7
and snow pixels 142.2 (mean abs 45.8); the Fresnel factor set to 1 gives 118.0 and 141.3 (mean abs 46.9); neither, the generic path, 100.1 and 93.9. A surface
turned 90 degrees from the sun takes 0.73 of it with the wrap (ours took none), and the Fresnel factor multiplies the whole diffuse by 1 to 3.15 (the picture is also scaled by
`FresnelStrength * FresnelColour`, 0.56, at a facing normal); ours' generic path gave a constant ambient plus `sat(N.L)`, and Omega's NOVA programs carry no constant ambient.
`liveLighting0specular` is the sun's (12 on this circuit),
so the lobe is `0.1 * 12 * gloss` of a near-white colour: a bright sheen where the normal turns the sun to the eye.

**Not read, assumed.** `reflectiveBlurCoeffs.z` is engine-bound and read as 0 (it reads 0 live on the sea, `panel-displacement.md`): the lobe is the plain
`SpecPower` and its scale is 1; with `z` at 1 the exponent falls to 2.5 and the lobe to 0.4 of it. The Fresnel factor uses the same bumped normal as
the lighting; the program builds a second normal from `cross(N, T)` for `N.V` and one from the interpolated bitangent for the rest, which agree where the
tangent's sign is +1 and the scale is 1. The track's shadow map (`S`, the road's `LM.a * S`) is not applied to the snow: the permutation has none, but the
sun is gated by the baked `NOVAColor.a`, which ours does carry.

**Wired** (`omega-snow-program`): `oag_rcs::rcsmodel::psp2::SubMesh::{ps4_vertex_colour2, ps4_nova_colour}` (additive readers), the loader's `psp2::snow`
(admits a material whose pixel programs declare exactly the five snow samplers and the lit constants, by `oag_rcs::gcn::scan`, not by name; every
submesh must carry the attributes), three glow-table entries of mode `PROGRAM_OMEGA_SNOW` (21) holding the constants above, the drift normal third and the
details fourth and fifth, and `omega_snow.wesl` called from `shade.wesl`'s normal, light-sum and specular sites. Gated off by the shader's `simplified_lighting` (below).
Test: `omega_sky_brightness_ground_truth::sebenco_climb_dome_is_scaled_by_its_authored_brightness` also pins the entries' constants against the model's own
material and the vertices that name them (dropping the wire leaves 0 vertices naming the entry and fails it).

**Measured** (`omega/sebenco-climb`, 1920x1080, the pair's recorded flags, ours re-rendered with the binary before and after; grey = Rec. 709 luma):

| | theirs | before | after |
| --- | --- | --- | --- |
| box `1100,40,300,160` mean grey, bias, mean abs | 162.3 | 100.1, -62.1, 62.5 | 133.2, -29.1, 30.4 |
| box mean RGB | 149/164/183 | 85/102/127 | 117/135/159 |
| snow pixels (3.30 % of the frame, `OAG_SKIP_MATERIAL=rt_snow1` changes them by more than 25 grey, HUD boxes left out): mean, bias, mean abs | 186.3 | 93.9, -92.4, 92.7 | 176.2, -10.1, 15.3 |
| snow pixels' mean RGB | 170/189/213 (their frame's, same mask) | 79/95/129 | 158/179/207 |
| whole frame raw mean grey | 103.77 | 81.85 | 85.77 |
| whole frame mean abs against the original | | 39.86 | 36.15 |

Seen: the mountain goes from a lilac wash to white snow with blue shadow, with the drifts and ridges of the original's; the rocks and the boulders
(`dc_rocks1`, not `rt_snow_ref`, which this frame does not draw; generic here, drawn by "The rock program" below) are unchanged. **All 24 Omega race pairs re-rendered before and after** (`scripts/compare-rerender-ours.py`'s `rerender` on copies, the 22 of the sky lane's sweep plus the two Zone pairs): 23 are pixel-identical (0 pixels differ), Sebenco Climb differs on 107,725 pixels (5.20 % of the frame).

**What is left of the box** (30.4). The 59.5 % above matched the needle `snow` against texture paths as well as material names, so it counted materials that only
name a snow *picture*; `omega-snow-rest` ("The rock program", below) splits it. The rest: the dome's bloom (the red channel 20 short, the sky lane), the far scenery, the
snow's remaining 6 % (the original's bloom; the vertex-light count of the permutation the race binds), and the HUD region the box overlaps.

**Smaller facts.** The Nova decode is the vertex program's per vertex (`scale * pow(NOVAColor.rgb, power)` interpolated, then the bias in the fragment, which `vs_main` already
computes into `lit_light.yzw`); doing it per pixel instead moved 131 pixels by one level. `--circuit-lighting simplified` moves 514,030 pixels of this frame before and after the
change and none of the 68,500 snow pixels (the option does not reach these draws), so the generic fallback the mode leaves under `simplified_lighting` is not exercised in a render here
(unverified; it reads `NOVAColor.a` as the sun mask and draws no normal map).

**Cross-checks.** HD: **checked, differs** (HD's `sebenco_snow` is its own wired program on rcsmaterial microcode, `mesh::rcs::snow_rock`; `rt_snow1` is Omega's); `hd/sebenco-climb` and
`hd-race/talons-pack` re-rendered before and after: **0 pixels differ**. 2048 (Vita): **checked, differs**: the DLC1 pack's `Sebenco_Climb` (`PCSA00015` and `PCSF00007` `dlc1.psarc`) ships
`fc07_SnowBlendVertColour` with `and_snow3alpha`, no `rt_snow1` or `rt_snow` in the base, DLC or patch archives (listed), and its programs are GXP with unread operands; the plan is `ps4`-gated;
`2048-vita/sebenco-climb` and `2048-vita/moa-therma`: **0 pixels differ**. The same `NOVAColor` + `VertexColour2` declaration is used by other Omega materials on this circuit; they stay generic.

## The rock program (2026-11-20, `omega-snow-rest`)

**Claim tested:** the snow-named materials other than `RT_snow1` that own about 40 % of Sebenco Climb's box `1100,40,300,160` are drawn by their own pixel
programs, so the box (133.2 against the original's 162.3, bias -29.1) and those materials' own pixels move toward the original. **False as worded, and the part
that could be wired does not move the mean.**

**Census** (`OAG_SKIP_MATERIAL=<needle>`, pixels that change by more than 25 grey against the unskipped frame, `rt_snow1` already wired; the frame is 1920x1080):

| Needle | Material | Box `1100,40,300,160` | Frame |
| --- | --- | --- | --- |
| `snow` (all) | everything below, by name or by picture path | 46.0 % | 8.62 % |
| `rt_snow1` | the snow mountain (wired) | 29.8 % | 2.78 % |
| `snow_norway` / `dc_rocks` | **`dc_rocks1`**, matched only by its `snow_norway_drift_n` sampler: the boulders and cliffs | **18.0 %** | 0.55 % |
| `materials/sebenco_snow` | `Sebenco_snow`, `_just_snow` (slots 231, 264, 265, 266, 283) | 1.1 % | 0.55 % |
| `snowfade` | `jd_*2uvoverlay*` buildings that overlay `jd_sebenco_snowfade_*` | 1.5 % | 3.51 % |
| `snowtile` | `jd_sebenco_snowtile_01` | 0.0 % | 0.03 % |
| `rt_snow_ref` | one submesh | 0.0 % (not drawn in this frame) | 0.00 % |

So the "other snow" is mostly **one rock material**; the materials with `snow` in their name own about 2 % of the box. `Sebenco_snow*` is drawn (the previous section's
"named by no model" was wrong: slots 231 to 283 of `track.final.rcsmodel`), and the boulder is `dc_rocks1`, not `rt_snow_ref`.

**The program** (`gcn_dump data02.psarc data/environments/10_Sebenco_Climb/Materials/DC_Rocks1.rcsmaterial --at 0xca64`; confidence 85, decoded GCN, no run). The race
submeshes declare `position, normal, tangent, uv1, lightmapUV` (no `NOVAColor`), so the lightmapped permutations: vertex `@0xc424` (94 instructions) with pixel `@0xca64`
(290, no shadow map), or `@0xab44` with `@0xb264` (395, shadow map). Which one the race binds was not read (70); `@0xca64` is read and drawn:

```text
n1 = RockColourNormal(uv)      n2 = RockDetailNormal(uv * DetailScale)       z = 2 n1.z - 1     h = 1 + DetailNormalStrength (2 n2.z - 2)
L  = (h (2 n1.xy - 1) + DetailNormalStrength z (2 n2.xy - 1),  z h)          N = normalise(T L.x + B L.y + N0 L.z)
w(x) = sat((x + Wrap) / (1 + Wrap))
Fsl  = 0.75 + 0.25 w(N.SL) + SLSD.y w(R.SL)^SLSD.x                           (the tarmac roads' factor)
D    = w(N.L) liveLighting0diffuse LM.a + nova Fsl                           nova = scale pow(LM.rgb, power) + bias
spec = liveLighting0specular LM.a w(R.L)^e 0.06981317 / max(0.06981317, 0.17453294 k)     e = SpecPower (1 - 0.75 k)
out  = fog(rock D + rock SpecStrength (2 n2.z - 1) (spec + ReflectivityStrength refl cube))
```

`k = reflectiveBlurCoeffs.z + ReflectivityBlur * reflectiveBlurCoeffs.w`; the engine's constant reads `(1.72285, 2, 0, 1)` live (`panel-displacement.md`), so `k = ReflectivityBlur`
(0.8 on `dc_rocks1`), the exponent 6.8 and the lobe's scale 0.5. The program's second normal (`T L.x + (B + cross(N, T)) L.y + N0 L.z`) is used only for the cube reflection's
Fresnel and is not drawn. **Constants** of `dc_rocks1`: `DetailNormalStrength` 1, `DetailScale` 10, `ReflectivityBlur` 0.8, `Wrap` 0.5, `SpecStrength` 0.1, `SpecPower` 17.02,
`ReflectivityMin` 0.163, `ReflectivityScale` 0.1, `ReflectivityPower` 1, `ReflectivityStrength` 0.2. **Not drawn:** the cube reflection (`cubeReflectionTex` is bound by the
engine; it enters multiplied by `ReflectivityStrength * 0.1 * rock`, at most about 0.005 of the picture, so it is small; recipe to read it: the table walk of
`scripts/wipeout-pc-crowd-light-live.py` for the cube's slot) and the shadow term `S` of the shadow-mapped permutation.

**Wired** (`omega-snow-rest`): the rock is a second family of `psp2::snow` (`declares(file, &ROCK)` on the pixel programs' samplers and constants, not a name; every submesh must
carry a lightmap pair and a tangent), one glow-table entry of mode `PROGRAM_OMEGA_ROCK` (22), the colour normal third and the detail normal fourth, `omega_rock.wesl` called from
`shade.wesl`'s normal, light-sum and specular sites, off in a Zone frame and under CIRCUIT LIGHTING `simplified`. Test: `omega_sky_brightness_ground_truth` (the entry's constants
read back from the model, more than 10,000 vertices naming it, the loader line); with the admission dropped it fails.

**Measured** (frozen mask: the 11,427 pixels the `dc_rocks` skip changes; theirs 127.4):

| | mask mean / bias / mean abs | box mean / bias / mean abs | frame raw mean / mean abs |
| --- | --- | --- | --- |
| before | 102.0 / -25.4 / 25.5 | 133.2 / -29.1 / 30.4 | 85.77 / 36.15 |
| after | 99.9 / -27.5 / 28.0 | 133.0 / -29.2 / 30.6 | 85.76 / 36.16 |
| ablation: `Fsl` forced to 1 | 107.1 / -20.3 / 20.9 | 134.2 / -28.1 / 29.5 | 85.80 / 36.12 |
| ablation: no sun, no lobe | 90.2 / -37.2 / 37.2 | 131.0 / -31.2 / 32.5 | 85.71 / 36.22 |
| ablation: normal only (generic light) | 102.1 / -25.3 / 25.4 | 133.2 / -29.1 / 30.4 | 85.77 / 36.15 |

**The mean does not move toward the original; the structure does.** The rock's high-pass contrast (5x5 box, interior of the mask) goes 4.78 to 6.99 against the original's
6.75: the two normal maps give the surface its grain. The light terms are the program's, and `Fsl` (0.75 to about 1) darkens by 7 grey, which `Fsl = 1` would not; it is
kept because the road and the snow carry the same factor. The rock sits 22 grey under the original at every distance beyond 16 pixels from the snow (and 44 under within 6),
and the whole frame sits 18 under (103.77 against 85.77): an offset every surface shares, the original's bloom and exposure, not this program. All 24 Omega pairs, `hd/sebenco-climb`, `hd-race/talons-pack` and the two Vita pairs
re-rendered before and after: Sebenco differs on 12,244 pixels (0.59 %), **the other 23 Omega pairs (including both Zone pairs), `hd/sebenco-climb`, `hd-race/talons-pack`,
`2048-vita/sebenco-climb` and `2048-vita/moa-therma` are pixel-identical**: `DC_Rocks` is not drawn on another Omega pair's frame, or not through this permutation (Moa Therma and Tech De Ra ship the family; the reason they do not move was not traced).

**Open, in box order:** `Sebenco_snow*` (1.1 % of the box, programs `@0x33e4`-`@0x10804` over `snowSampler`, `textureforblendSampler`, `rockSampler`, `snownormSampler`,
`rocknorm1Sampler`, a different program from `RT_snow1`'s, 154 to 369 instructions); the `snowfade` overlay buildings (1.5 % of the box but 3.5 % of the frame, 27 grey mean abs:
check `PROGRAM_UV_OVERLAY` first); the shadow term `S`; the cube. The box's residual is the original's frame-wide brightness, not a material.

**Cross-checks.** 2048 (Vita): **not checkable** here: its programs are GXP with unread operands, and no Vita `DC_Rocks` file was listed; the plan is `ps4`-gated and
`2048-vita/sebenco-climb` and `moa-therma` re-render with 0 pixels differing. Omega's 2048-mode library (`environments2048/shared/objectlibrary/materials/DC_Rocks*.rcsmaterial`)
is the same family and goes through the same admission; no 2048-mode pair moved. HD: **checked, differs** (`hd-track-tone.md` reads HD's `DC_Rocks` on rcsmaterial microcode with
its own blend); `hd/sebenco-climb` and `hd-race/talons-pack` 0 pixels differ.

## The diffuse coordinate that writes `1 - v` (2026-11-20, `omega-uv-flip`)

**Claim tested:** Omega materials whose GCN vertex program writes `(u, 1 - v)` are drawn with the flip, decided per file from the program, and the pairs that use one improve. **Half true: the reader and the wiring exist; no Omega circuit draws a file that flips, so no pair moves.**

**The reader** (`oag_rcs::gcn::flips_v`, `crates/rcs/src/gcn/texcoord.rs`, confidence 85). A vertex program fetches `Uv1` into `vU, vU+1` and a flipping one computes `v_sub_f32 d, 1.0, vU+1` and exports `(vU, d)`: Modesto's `Track_Wall` (`gcn_dump`, `Shdr @0x2e44`) is `c  v_sub_f32 v0, 1.0, v9` ... `100  exp param1 v8, v0, v1, v1`, with `v9` overwritten later (`0x48`), after the subtract. Read: an `exp paramN` whose first lane is `vU`, never written, and whose second lane was last written by a plain `v_sub_f32` of the literal 1.0 and `vU+1` (no negate, abs, omod or clamp), `vU+1` itself untouched before that subtract. A file flips when it has a program that fetches `Uv1` and every such program does (the rule `gxp::texcoord_flip` has). Other shapes read as not flipping: a multiply-add that folds the flip (`neg=1` `v_mad_f32` rows), a flip after a transform of the coordinate, `v_sub_f32` of a constant. Wired in `mesh::rcs::psp2::uv_flip::plan`, which now picks the GCN or the GXP reader by the model's platform.

**Census** (`gcn_ground_truth`, every `.rcsmaterial` of `data00`-`data05`, `data07`-`data09` of the base and the patch, a ratchet per archive): **9 files flip, 0 are mixed, every other file has no flipping program.** They are 3 materials with their `_1` and `_2` permutations (the PS4 Pro compile), in `data02` (base) and again in `data05` and `data08` (patch): Modesto Heights `Track_Wall`, Talons Junction `Track_Wall`, and Ubermall's `Materials_DLC/D_S_N_R_uvflip` (named for it). 2048's Amphiseum and Tech De Ra `track_surface` twins do **not** flip on Omega (**checked, differs**): no Omega `track_surface` program matches the shape

**Why nothing moves.** No `.rcsmodel` of Omega's base or patch archives names any of the nine files (a byte search of every model of `data00`, `data01`, `data02`, `data04`, `data05`, `data08` for `track_wall.rcsmaterial`, `track_wall_1`, `track_wall_2` and `d_s_n_r_uvflip`: no hit; the `track_wall` string that does occur in the circuits' models is a shape name). The loader's report line `N material(s) write 1 - v` is absent for all 22 Omega circuits the race pairs use (`Environments`: `01_vineta_k` to `modesto_heights`, `tech_de_ra`, `amphiseum`, `talons_junction`, `05_ubermall`; `Environments2048`: `altima` to `tower`). The files ship with the circuits and are not drawn. Omega's Modesto right wall already shows the panel and hex rows at the authored `v`, and re-rendering `omega/modesto-heights` before and after is **0 differing pixels** (mean grey 40.72 both). `2048-vita/modesto-heights` (28.59 to 21.81, the flip still read from its GXP programs) and `hd-race/talons-pack` are 0 differing pixels against a build without this change.

**Pinned by** `gcn_ground_truth` (the 9 files per archive; empty with the reader dropped), unit tests on hand-built instruction words in `gcn::texcoord` (including a `vU+1` overwritten before the subtract), and `uv_flip::tests` (Omega's `Track_Wall` flips through the GCN reader, its `Track_Surface` and the Vita reader do not).

**Also found, not wired:** Amphiseum's `01_Normal_Diffuse_SpecularonAlpha_LH` (and `_1`, `_2`; `LH` for left-handed) exports `(1 - u, v)` (`v_sub_f32 v0, 1.0, v8`, `exp param1 v0, v9, v1, v1`), a mirror in `u` rather than a flip. No Omega model names it either.

**2048 (Vita): ported** (`gxp::texcoord_flip`, own ground truth). **HD: ported** (`skin::flips`).

## The alpha lane (2026-11-19, `omega-road-programs`)

**Claim tested:** Omega's materials sample their diffuse map's **alpha** as the surface alpha; ours read the red lane.
**True.** `mesh::rcs::psp2` built the slot word as 0 or `SECOND_IS_LIGHTMAP` for every Vita and PS4 material, and
in `shade.wesl` bits 3 and 4 of that word choose which channel of the first texture is `texel.a`: 0 is red. The
readers are the output alpha (`texel.a * in.colour.a`: every blended and cut-out draw), the generic specular mask,
the glass pane's cover and clear, the snow term and the hull's probe reflection. The crowd sprites, the flags and
banners, the trees and shrubs on Metro Park and the red slab that stood under the HUD on the HD-mode circuits were all
drawn with the red channel as coverage. Omega's programs gate by the diffuse alpha (the panel road, `0xe144`, confidence
94; the crowd's alpha test at 0.5, `0x4224`, `0x41a4`; "The crowd's law" above). The build now gives every PS4
material `slots::DEFAULT` (lane 3); Vita keeps lane 0 except the alpha-lit families, whose programs are read to take
`T.www` (`lit_alpha`) and, since `vita-race-pairs` (2026-11-19), Moa Therma's reflective window and awning
(`2048-material-params.md`, "The reflective window and the awning").

**Measured** (`scripts/compare-rerender-ours.py`'s `rerender` on copies of every Omega, `2048-vita` and `2048` pair, the
binary before and after; the raw mean grey is dominated by decorrelation, so read the signs and the regions):

- Omega, 29 pairs: 17 move by more than 0.05 grey down, 2 up (`menu-ship-select` +0.09, `zone-vineta-k-z5` +0.07),
  mean over the 24 race pairs -0.72. Largest: `anulpha-pass` 24.31 to 21.19, `tech-de-ra` 30.36 to 27.76, `moa-therma`
  38.40 to 36.39, `subway-2048` 32.28 to 30.38, `sol-2048` 34.57 to 32.71, `queens-mall-2048` 44.82 to 43.72,
  `talons-junction` 28.68 to 27.89, `the-amphiseum` 49.86 to 49.11, `metro-park-2048` 44.11 to 43.67.
  **`capital-reach-2048` (47.00 to 46.98) and `empire-climb-2048` (45.11 to 45.11) do not move**: their error is the
  tarmac road and the scenery, not an alpha lane.
- Regions (`scripts/compare-decompose.py`, region masks and crowd mask pinned across builds), ours minus theirs:
  crowd bias Queens Mall +48.5 to +4.7, Sol +115.7 to +38.2, Metro Park +14.4 to -10.8; Empire Climb -24.8 to -37.2 and
  Capital Reach -51.3 to -55.8 (0.0 to 0.6 % of the frame, a sun-lit crowd ours still draws dark). Queens Mall's road
  bias +1.8 to +5.7 and LF 13.7 to 11.9, its sky -15.3 to -10.9, come from the adaptive exposure lifting the whole frame
  once the crowd stopped being white: they are not a road fix.
- Boxes at 1920x1080, mean |ours - theirs| grey and bias, before to after: Queens Mall left stands 52.1 to 49.2,
  bias +13.3 to +1.4; right stands 50.2 to 50.3, +5.4 to +2.6; banner strip 69.3 to 67.5, +12.7 to +0.6. Metro Park left
  trees and shrubs (absent before) 76.6 to 73.5, bias +36.2 to -1.8; flags and far shrubs 44.8 to 47.1, bias -9.6 to
  -12.3 (now drawn, a few pixels out of phase with the original's: decorrelation, not a worse colour); right stands
  47.6 to 47.3. Sol left stands 58.2 to 53.7, right stands 48.4 to 40.6, bias +22.8 to +11.3.
- By eye (every mover above, theirs, before, after at 640 px wide): Metro Park gains the trees, shrubs and the flags
  over the stands; Queens Mall's stands go from white cut-outs to dark rows with colour, as in the original; the red
  slab under the HUD on `moa-therma`, `the-amphiseum` and `tech-de-ra` becomes the original's thin teal bar. Nothing the
  original shows vanishes on those pairs, `anulpha-pass`, `subway-2048` or `sol-2048`. Sol's stands still carry white figures in the rows nearest the camera.

**Test.** `psp2_twin_blend_ground_truth` (a pass over every 2048 and Omega circuit that ships the material) holds that
at least 95 % of every Omega circuit's vertices carry lane 3 and, on Vita, that a vertex carries it exactly when it is
alpha-lit; dropping the build's `slots::DEFAULT` turns Anulpha Pass's count into 31,714 of 947,447.

**2048 (Vita): ported for the alpha-lit families, checked, applies, not wired for the rest.** The same re-render on the
Vita pairs with lane 3 everywhere moved `anulpha-pass` 16.71 to 11.14 (the alpha-lit families, wired), and
`moa-therma` 32.55 to 34.23, `tech-de-ra` +0.25, `mall` +0.15, `empire-climb` +0.09, `ubermall` -0.43, `talons-junction`
-0.28: mixed. **Corrected 2026-11-19 (`vita-race-pairs`): `moa-therma`'s change was not the hull** (the hull box
does not move, 38.12 either way) **but the Feisar awning, `dc_diffusetexturealphaemissive`, drawn by its red lane**: its
program writes the texel's alpha out and adds `D^2` (`2048-material-params.md`, "The reflective window and the awning"),
now wired with the window family, and lane 3 everywhere then gives `moa-therma` 27.53 against 27.56 with the two wired, while
`modesto-heights` +0.07 and `empire-climb` +0.09 stay worse. 2048's own programs are GXP: which others gate by the diffuse
alpha is unread, so Vita stays on lane 0 outside the alpha-lit families and these two until each family's gate lane is read.

## The tarmac roads' lit programs (2026-11-20, `omega-2048-road`)

**Claim tested:** Omega's 2048-mode road is drawn by the law its own GCN road program computes, so the road box on
`capital-reach-2048` (46.98) and `empire-climb-2048` (45.11) improves measurably. **False.** The one term the program
computes that ours did not draw and that could be wired from authored data, the shadow-light factor, moves the two
frames +0.28 and -0.14 grey (under the 0.3 bar both ways): the gap on these two roads is not in this program. The sun
lobe was read, wired, measured and **taken out again** (below).

**Census.** Capital Reach (`bridge`) and Empire Climb (`tower`) draw their road through
`tarmac_markings_dirt_markingsnormal` (36.0 % and 60.0 % of the frame, `compare-decompose.py --road tarmac`); the
`road_alphaspec_normal_emissive_panelmap1[_mask]` materials the circuits also carry, and the speed and weapon pads,
change no pixel by more than 25 grey on either pose (0.0 %). Every road submesh (`DECL_SUB` on `bridge` sub 3658, `tower`
sub 2881) carries `lightmapUV`, `tangent`, `uvMarkings` and `uv1_uvDirt` and no `NOVAColor`: the lightmapped variants.

**The program** (`gcn_dump data05.psarc data/environments2048/shared/objectlibrary/materials/tarmac_markings_dirt_markingsnormal.rcsmaterial --at <offset>`;
the patch's copy ships): the lightmapped lit variants are `@0xbd44` (241 instructions, no shadow map) and `@0xa764`
(342, `shadowMapSampler` and `shadowCoeffs`; `data00`'s are `@0xbc44` and `@0xa684`). The two are read to the same
law apart from the shadow term (confidence 88, decoded GCN, no run):

```text
nova  = pow(LM.rgb, power) * scale + bias                         (the authored Nova triple; unchanged)
Fsl   = 0.75 + 0.25 sat(N.SL) + SLSD.y sat(R.SL)^SLSD.x          (0x4e4 `0.75`, `0.25 sat(N.SL)`)
lit   = nova * Fsl + liveLighting0diffuse * LM.a * S * sat(N.L)    S = the shadow map's term, 1 in @0xbd44
G     = mix(Dm.a (1 - Dirt.a), MarkingsSpec Dm.a, Markings.a)
spec  = liveLighting0specular * G * LM.a * S * sat(R.L)^SpecPower   (reflectiveBlurCoeffs.z reads 0 live: plain power)
out   = fog(Dm.rgb mixed by Dirt, Markings * lit + spec)
```

`N` is the normal-mapped world normal, `R` the view reflected about it. `SL` and `SLSD` are the circuit's authored
`Lighting.ShadowLight direction` (normalised, as the executable does) and `Lighting.ShadowLight specular power scale`:
the live reads on Sol `(0.7908, -0.375, -0.4838)` and Queens Mall `(-0.3068, 0.4453, -0.8412)`
([panel-displacement.md](../ghidra/functions/ps4-omega-eu/panel-displacement.md), "Frame constants read live")
equal the authored triples (Sol exactly, Queens Mall `(-0.31, 0.45, -0.85)` after normalising), so **no live read is
needed for the factor** (the thread's "its source in the lighting data" is the envsettings). Capital Reach authors
`(-0.414, 0.943, -0.850)` and `(10, 1)`, Empire Climb `(0, -0.320, -0.947)` and `(10, 1)`. `liveLighting0specular` is the
authored specular colour times its intensity (Sol `6.096 3.263 1.6` times 1 equals the live read), which ours already carried.
Which variant the race binds on a tarmac draw was not read; on the panel road all eight staged blocks had the shadow-map layout,
so `@0xa764` is assumed here (confidence 70).

**Settled 2026-11-20 (`omega-shadow-map`):** the race binds the shadow-map permutation on the tarmac. Forcing `S` to 0 through
`shadowCoeffs` on the native port takes Capital Reach's sunlit streaks from 154 to 96 grey (confidence 90), and forcing it to 1
moves nothing on that grid: no craft shadow is visible there, so **Capital Reach's whole-frame gap is not `S`**
([track-shadow.md](../ghidra/functions/ps4-omega-eu/track-shadow.md), [shadows.md](../rendering/shadows.md),
"Omega's own `S`").

**Wired** (`omega-2048-road`): `Light::shadow_light` and `shadow_light_specular` (`Light::with_shadow_light`, filled by
`envsettings_light` on PS4 only) and the factor on the prelit term in `shade.wesl`'s mode-14 path. Tests:
`omega_tarmac_lit_path::the_shadow_light_factor_scales_the_prelit_term`, `omega_lightmap_ground_truth` (the loader
reaches the rig from Tech De Ra's file), `envsettings::tests::a_pair_is_exactly_two_numbers`.

**Read, not drawn: the sun lobe.** It was wired (an entry carrying `SpecPower` and `MarkingsSpec`, the lobe weighted by the
gloss `G` with the dirt alpha unconditional) and measured against the factor alone: Capital Reach 47.26 either way, Empire
Climb 44.97 with the factor alone and 45.25 with the lobe, Subway, Downtown, Arena and the Zone pair unchanged by it. The
lobe is `LM.a * S` in the original and ours has no `S`: it draws crack glints on Empire's near road, including inside the
craft's shadow, that the original's smoother road lacks, and scored no better on any of the five tarmac pairs, so it was removed
(the shader block, its test and the entry fields). Draw it with the factor once the track receives `S` (below).

**Measured** (the pairs' recorded flags, 1920x1080, ours re-rendered; `compare-decompose.py` and fixed boxes, mean abs
grey and bias, ours minus theirs). Factor alone, before to after:

| | Capital Reach | Empire Climb |
| --- | --- | --- |
| whole frame (raw mean grey) | 46.98 to 47.26 | 45.11 to 44.97 |
| road region `LF`, `bias`, `G` | 27.6 to 28.3, -3.5 to -5.5, 0.91 to 0.95 | 29.8 to 29.5, -8.2 to -9.4, 1.08 to 1.09 |
| box 1 mean abs grey, bias | near left `440x240+200+560`: 28.54 to 29.03, -12.4 to -15.3 | tarmac `700x120+620+380`: 42.60 to 42.87, -20.9 to -22.7 |
| box 2 | mid `600x160+380+400`: 64.49 to 65.99, -45.6 to -48.1 (the 480 px pose offset pollutes it) | near left `400x200+160+640`: 25.67 to 25.31, -0.9 to -3.7 |
| box 3 | right `360x200+1500+560`: 17.26 to 17.42, +0.8 to -0.7 | right `400x200+1380+560`: 39.04 to 38.95, +6.2 to +1.7 |

The other tarmac-road pairs' whole frame, before to after: `subway-2048` 30.38 to 30.38, `downtown-2048` 26.97 to 26.72,
`arena-2048` 33.15 to 32.84, `zone-capital-reach-2048-z5` 68.96 to 69.17. Three of the six improve, two worsen by 0.2 to 0.3,
none leaves the noise band except Arena's -0.31. The factor is the program's: the originals' road is brighter than ours on both
pairs (Empire's far road, box 1, is the sun glare and exposure the earlier lanes found; Capital's near road is neutral grey
in the original and warm in ours, R/G 1.07 against 1.14).

**Where the gap is instead.** The race's program takes the shadow map: its sun term and its lobe are `LM.a * S`, with
`S = min(1, pcf(shadowMap) * exp2(clamp(c0 + c1 z, c2, c3)))` (`@0xa764` `@0x14c`, `@0x1a8`, `@0x488`-`@0x4b0`; the
coefficients are `shadowCoeffs`). Ours has the baked `LM.a` only on the track, so Capital's sun on the road is over-present:
with the road's sun term switched off (a probe, not kept) Capital's whole frame goes 46.98 to 45.41 (Empire 45.11 to 56.86,
which needs its sun). Next: the track as a shadow receiver (a shadow-map pass over the track casters; ours' hull map is
`sun_occlusion`), then the lobe, then the dirt layer's colour.

**2048 (Vita): checked, applies, not wired.** Vita authors `Lighting.ShadowLight direction` and its nova prelit programs
read `shadowLightDirection` as the same shading uniform (`ship-shadows.md`, "Which light"), but 2048 carries no tarmac family
and its light table (`FUN_8101d068`) is not this lane's. **HD and `2048-vita/moa-therma`:** re-rendered before and after
(`hd-race/talons-pack` 23.06, `2048-vita/moa-therma` 27.56): pixel-identical.

## The port's anisotropy (2026-11-16, `port-aniso`)

**Claim tested:** the native port samples track textures with anisotropic filtering at a level N read from its sampler
creation. **True, N = 16, and it is the game's own request, not a setting of the port.**

**Falsifier written first.** "The original requests anisotropy" would be false if no sampler built during the race load
of a `tarmac_*` circuit (Empire Climb, 2048 mode) and of Metro Park (brick) carried an `Aniso*` filter or a `max_aniso`
above 1x; "the capture had aniso" would be false if, in addition, the port's forced level was 0 (N = 1). Neither held.

**What the port does** (`gpu/shadps4/video_core/texture_cache/sampler.cpp`, port commit `e56904d` read; the pairs were
captured at `84690ed`: `sampler.cpp` and `amdgpu/resource.h` are byte-identical between the two, the settings reader
differs by two HDR keys, and in `texture_cache.cpp` (442 lines changed, `GetSampler`'s file) and `vk_rasterizer.cpp` the
only diff line naming a sampler, LOD bias or anisotropy is a `GarbageCollectSamplers();` call; the stored Metro Park
`theirs.png` also sits at this boot's shipped sharpness, below). `Sampler::Sampler` sets `anisotropyEnable` when the game's
S# has `xy_mag_filter` or `xy_min_filter` in {`AnisoPoint`, `AnisoLinear`}, or when the menu's forced level
(`Gow3Settings::startup_anisotropy`, `gow3.ini` key `anisotropy`, 0 = the game's own) is above 1 on a filtered,
mipmapped colour sampler. `maxAnisotropy = clamp(max(S# max_aniso, forced), 1, device maximum)`. The capture script's
`gow3.ini` carries only `engine_fps` and `render_resolution`, so the forced level was 0 and every aniso sampler is the
game's. The port's extra LOD bias is non-zero only with an upscaler (`SceneMipBias`, `vk_rasterizer.cpp:1675`); the
pairs ran `upscaler none`, so it was 0 (the log's `extra` column is 0.0 on every sampler).

**What the game asks** (`scripts/wipeout-pc-sampler-log.py`: the probe under `gdb`, a silent breakpoint on the sampler
constructor, one line per unique S#, decoded with the S# layout of `resource.h`; port `e56904d` build, the Omega EU 1.07
game folder; the boots are cold, on a fresh save, 2048 mode at C class, Feisar2048):

| Boot | Unique samplers | `AnisoLinear` x16, mip Linear | of those with LOD bias +1.0 | Bilinear `max_aniso` 1x (mip None, Point or Linear) |
| --- | --- | --- | --- | --- |
| Metro Park 2048 (brick), boot 1 | 17 | 7 | 2 | 10 |
| Metro Park 2048, boot 2 | 17 | 7 | 2 | 10 |
| Empire Climb 2048 (`tarmac_*`) | 19 | 8 | 2 | 11 |
| Vineta K, HD mode | 16 | 5 | 1 | 11 |

The aniso samplers are always `AnisoLinear` mag and min, `mip Linear`, `max_aniso` field 4 (16x); the other samplers
are `Bilinear` with `mip Point`/`None`/`Linear` and 1x (point-sampled lookups, render targets, depth compares). **No
sampler anywhere has a negative LOD bias**: 0.0, and +1.0 (softer, not sharper) on 1 or 2 of each boot's aniso samplers. So the game's
request is the whole story: there is no LOD sharpening hiding behind the filter. Confidence 85 that the road materials
are among the aniso samplers (the A/B below shows it by the picture; which S# each material binds was not read, only
that stripping the aniso samplers softens the road).

**It is the filter that draws the road's sharpness.** The same boots with each S#'s `Aniso*` filter rewritten to
`Point`/`Bilinear` by the breakpoint (`--strip`, so `anisotropyEnable` is false for that sampler), Laplacian standard
deviation of a road crop on the grid frames before GO (relative units; three grid frames each):

| Circuit (crop at 1920x1080) | as shipped | aniso stripped | stored pair `theirs.png` |
| --- | --- | --- | --- |
| Metro Park, brick (`800x90+700+400`) | 0.468 / 0.469 / 0.462 and, second boot, 0.467 / 0.464 / 0.464 | 0.284 / 0.289 / 0.278 | 0.456 |
| Empire Climb, tarmac (`700x120+620+380`) | 0.454 / 0.456 / 0.455 | 0.276 / 0.278 / 0.279 | 0.383 (pose differs slightly) |
| Vineta K, HD mode (`500x120+900+440`) | 0.432 / 0.436 | 0.303 / 0.303 | not stored at this crop |

Stripping the filter takes about 40 % off the road crop's Laplacian deviation (0.46 to 0.28) and reads as the blurry far field of an
unfiltered trilinear lookup (the frames were read by eye: brick courses and tarmac cracks vanish into a smear at the
distance where the shipped frame still resolves them). Metro Park's stored `theirs.png` sits at the shipped value (0.456 against 0.462-0.469), so that pair was captured with the
filter on; Empire Climb's stored frame (0.383) is another pose and is not an A/B, so its evidence is the live boots and
the decomposition `G` numbers below. Two boots agree to 0.005 (Metro Park), seen once on the other two circuits.

**Ours.** `oag_mesh::mesh_render` builds its sampler with one `anisotropy_clamp` from the `--anisotropy` setting (`crates/mesh/src/mesh_render.rs:1082`),
a single level for the whole model, where the game asks 16x on 5 to 8 of its 16 to 19 sampler states (31 to 47 %) and 1x on the rest.
The pairs' `omega` profile had it `off` (`compare-environment.toml` `[ours_defaults]`). Before and after, the same
binary and pair, only `--anisotropy off` to `16x` in the recorded flags (`scripts/compare-decompose.py`, road family
region; `LF` is unchanged to within 0.3 on every row, `G` is Sobel gradient energy ours / theirs):

| Pair | road family | road `G` off | road `G` 16x | road `LF` off / 16x | raw mean grey off / 16x |
| --- | --- | --- | --- | --- | --- |
| Empire Climb | `tarmac_*` | 0.84 | **1.06** | 26.2 / 26.2 | 43.89 / 44.71 |
| Capital Reach | `tarmac_*` | 0.68 | **0.91** | 27.7 / 27.5 | 46.86 / 47.12 |
| Subway | `tarmac_*` | 0.70 | **0.82** | 21.4 / 21.8 | 35.79 / 35.86 |
| Downtown | `tarmac_*` | 0.77 | **1.06** | 16.8 / 16.5 | 30.20 / 30.58 |
| Metro Park | brick | 0.78 | **0.97** | 21.4 / 21.4 | 43.70 / 45.49 |
| Queens Mall | `panelmap1` | 0.61 | 0.71 | 30.3 / 30.7 | 49.15 / 49.64 |
| Sol (A class) | `panelmap1` | 0.27 | 0.35 | 32.5 / 32.7 | 43.42 / 43.63 |

The five `tarmac_*`/brick pairs' road `G` goes from 0.68-0.84 to 0.82-1.06 with `LF` unchanged, the detail axis the
filter owns; the two `panelmap1` pairs rise a little (their gap is the pixel program, not the filter). The raw mean grey
rises on all seven, as the older note predicted (correct detail a few pixels out of phase scores worse on a frame whose
edge correlation is about 0), so the raw score is not the test. The decomposition ran on the main checkout's debug
binary (`6882828ba`), both settings on the same file.

**Confidence:** 90 that the game requests 16x on mipmapped colour samplers and the port applies it (source read, then the live S# log, two boots on Metro Park and one each on Empire Climb and Vineta K); 85 that the road materials are among them (A/B by picture and by Laplacian deviation).

**Decision.** `[platform.omega] anisotropy = "16x"` in `scripts/compare-environment.toml` (`scripts/compare_env.py` reads
a platform-level `anisotropy` like `shadows`; `scripts/check-compare-environment.py` checks it, and that every other
platform stays `off`). Ours is applied to every sampler of the model rather than only the 31 to 47 % the game asks it of; the
other samplers are lightmaps, masks and point lookups of low frequency, so this is a superset that costs nothing the
metric can see, and a per-sampler law is not wired. **Race grids only:** the `omega/` folder also holds front-end pairs (`menu-*`, `gap-*`); nothing there was measured, so
`compare-rerender-ours.py` leaves a menu or loading pair's recorded `--anisotropy` as it is. **Ours at 16x slightly
overshoots the original on Metro Park's brick crop** (Laplacian deviation 0.53 against 0.46 to 0.47), the road region's
`G` being 0.97: the filter is closed, a small excess of high-frequency energy is left and not explained here.
**Not changed:** the player-facing default (`--anisotropy`, the
settings menu) stays the player's; whether Omega's `Title` data should default to 16x (the original's own request,
like the HD LOD bias in `oag_hd::effects`) is a separate wiring item, named in the thread. Omega's looks carry no
material LOD bias, consistent with the S# bias above. **The existing Omega pairs under `data/reference` still record
`--anisotropy off`, and a recorded flag beats the settings file**, so `compare-rerender-ours.py` now sets a race pair's
`--anisotropy` to the profile's level (`compare_env.profile_anisotropy_flags`, checked by
`scripts/check-compare-environment.py`) and records it in `settings_readback`. Verified on an unedited copy of
`metro-park-2048` in scratch: the new `ours.flags` carry `16x`, mean grey 43.70 to 45.49 and road crop deviation 0.354
to 0.530, the same render as the hand-edited A/B above. The lead's step after this merges:
`just compare-rerender-ours --only omega` (`--only` is a substring of the pair directory path, so it matches the
`omega/` folder; menu and loading pairs keep their flags).

**2048 cross-check: not checked (Vita).** These are Omega's samplers, and Omega's 2048-mode circuits, like its HD-mode
circuit (Vineta K), all request 16x. The Vita 2048 sets its sampler state through GXM texture calls that were not read
for a filter or an anisotropy level (the Vita eboot is in Ghidra and Vita3K runs it, so it is checkable); the `2048`
profile keeps `off`, an unmeasured default. The decisive check: the filter and anisotropy arguments of the GXM texture
init calls on a road material's draw, and whether Vita3K honours one (its anisotropic override setting). **Open:** which S# each road material binds (the material's `+0x20` state word
`0x522800` is the likely key and was not mapped to the S#); the same A/B on the Zone and the remaining HD-mode
circuits (only Vineta K was logged in HD mode).

## HD-mode frame brightness: the tone map is not the gap (2026-11-20, `omega-hd-exposure`)

**Claim tested:** Omega's HD-mode race frames are darker than the original's because ours applies the wrong exposure, tone
map or scene light, with The Amphiseum ("the original is much brighter than ours"), Modesto Heights, Sebenco Climb and
Moa Therma as the evidence. **Result: false as a tone-map claim, and false as a brightness claim on two of the four.**
Method: ours re-rendered from each pair's own flags (main `13b22ac77`, 16x anisotropy, as the page records) against the
stored `theirs.png`; a local, uncommitted readback of the chain's `(LAvg, t0, t1, E)` and of its 256x256 luma image at the
screenshot frame (a `COPY_SRC` on the state buffer and the ladder's first target, mapped in `Drop`; the patch is not kept).

**Signed bias and mean absolute grey, 1920x1080, whole frame** (Rec.709 grey, ours minus theirs; the stored pair's
`ours.png` and the re-render are identical to the digit):

| Pair | theirs mean | ours mean | signed bias | mean abs | ours `LAvg` | ours `E` | regime |
| --- | --- | --- | --- | --- | --- | --- | --- |
| The Amphiseum | 83.5 | 88.6 | **+5.0** | 49.1 | 0.133 | 2.00 | pinned at `Exposure maximum` 2 |
| Modesto Heights | 90.8 | 91.7 | **+0.9** | 40.8 | 0.261 | 0.868 | unclamped |
| Sebenco Climb | 103.8 | 81.5 | **-22.3** | 40.2 | 0.175 | 0.555 | pinned at `Exposure minimum` 0.555 |
| Moa Therma | 110.2 | 96.1 | **-14.2** | 36.4 | 0.384 | 1.00 | pinned at `Exposure maximum` 1 |

The mean absolute grey (the 49.1 the gallery notes) is dominated by pixel decorrelation, not by brightness: The Amphiseum
and Modesto have no brightness gap at all (their luminance quantiles 5/25/50/75/95 read 9/36/67/119/230 against
8/34/70/131/234 and 5/34/67/130/236 against 11/40/74/123/229), and the Amphiseum's gallery note "the original is much
brighter than ours" does not hold on main (ours is 5 grey brighter). Over all twelve HD-mode pairs the signed bias is
Vineta K -4.9, Anulpha Pass -4.1, Chenghou Project +4.8, Metropia -16.6, Ubermall -1.5, Sol 2 -7.3, Modesto +0.9,
Talon's Junction -1.9, Tech De Ra -0.6, Sebenco -22.3, Moa -14.1, The Amphiseum +5.1: no common gain.

**Named boxes** (`x,y,w,h` of the 1920x1080 frame, clear of the HUD and the craft; theirs, ours, signed bias, mean abs;
box means move with pose decorrelation, read the sign and the spread between boxes, not the digit):

| Pair | Box | theirs | ours | bias | mean abs |
| --- | --- | --- | --- | --- | --- |
| Amphiseum | ceiling/sky `300,110,460,130` | 103.0 | 142.9 | +39.9 | 55.4 |
| | stands right `1150,190,560,130` | 85.2 | 110.4 | +25.1 | 62.5 |
| | road left `60,520,480,230` | 106.8 | 103.7 | -3.2 | 54.0 |
| | wall left `0,380,576,120` | 74.4 | 80.9 | +6.5 | 47.2 |
| Modesto | sky gap `800,160,240,100` | 192.4 | 156.4 | -36.0 | 46.0 |
| | left structure `0,120,600,200` | 65.2 | 58.7 | -6.5 | 37.6 |
| | right wall `1160,380,640,180` | 73.5 | 74.0 | +0.5 | 34.8 |
| | floor left `100,600,460,120` | 128.7 | 135.6 | +6.8 | 31.1 |
| Sebenco | snow sky `1100,40,300,160` | 162.3 | 97.3 | -65.0 | 65.2 |
| | building left `0,200,640,200` | 93.3 | 51.8 | -41.5 | 50.6 |
| | yellow floor `700,480,560,100` | 121.0 | 104.3 | -16.8 | 23.7 |
| | wall right `1200,380,600,200` | 67.6 | 54.5 | -13.0 | 21.3 |
| Moa | sky/city `1160,60,440,120` | 126.9 | 126.2 | -0.7 | 37.8 |
| | stands left `0,200,500,200` | 116.1 | 100.0 | -16.1 | 56.1 |
| | floor left `300,640,360,220` | 141.3 | 139.0 | -2.3 | 33.2 |
| | stripe road `660,500,500,80` | 96.0 | 81.4 | -14.6 | 23.7 |

**Why a gain is not the cause.**

1. **Ours' exposure is the executable's law and is not ramping.** The grab is stable from tick 10 to 300 (Sebenco 81.5, 81.4,
   81.5, 81.2). That only shows ours is stable: ours starts settled by construction (`omega_tonemap`, "The first frame starts
   settled"), so it says nothing about the original's first frames.
2. **"The original is still adapting at the grab" fails as a common cause.** Starting the history at zero as the executable
   does and stepping the chain N times per frame, Sebenco matches theirs at N about 20 (107.1 against 103.8) and Moa at 10 to
   20, but Modesto at N=20 reads 120.7 against theirs 90.8, Talon's 122.2 against 93.6, Tech De Ra 135.1 against 99.5, and
   they match theirs only once settled (N 40). One frame count cannot fit both groups (confidence 80).
3. **Where ours is clamped, a brighter original needs brighter scene light, and the tone map is the same.** Sebenco's author
   pins `Exposure minimum` 0.555 and `b` 0 (`E = 0.079 / LAvg`), so ours is at the minimum: a darker or brighter scene moves
   the output one for one. Forcing ours' `E` to 0.9 reproduces the original's luminance quantiles
   (q10 to q95: 35/57/95/141/230 against 33/58/97/140/219), which is a x1.6 scene factor, **but it is not uniform**: at
   `E` 0.9 the snow sky is still -38 and the far building -25 while the yellow floor is +16 and the right wall +4.5. Moa
   (`Exposure maximum` 1, ours at 1.0) would need about x1.4 from `E`, which its clamp forbids (`t1` grows with `LAvg`, so
   a lower `LAvg` does brighten it through the curve: 96.1 rises to 108 as the unprimed sweep's `LAvg` falls), but its
   sky/city, floor and wall boxes already match (-0.7, -2.3, -1.0), so any global gain, through `E` or `t1`, would
   overshoot them; only its stands (-16.1) and stripe road (-14.6) are dark. The Amphiseum (ours at the maximum 2.0) is
   5 grey bright.
4. **Ours' luma input is not inflated by an HDR tail.** The luma image's 99th percentile is 0.82 and no texel reaches 0.97 on
   Sebenco (the other circuits reach 1.6 to 8; where Sebenco's luma sits relative to the sky or far geometry was not
   located, so whether the HDR fog colour is clamped there is open), skipping the sky changes `LAvg` 0.175 to 0.170, and the centre 64x64 or
   128x128 means are 0.237 and 0.183 against the full mean 0.176 (so which ladder level the coefficient pass samples,
   `tonemap.md`'s confidence-60 reading, moves `LAvg` by at most 35 % on Sebenco and cannot explain a x1.6 gap).

**What the data supports: per-surface light, a different surface on each pair** (confidence 85 that it is not a global
exposure or gain: the signed bias spans -22 to +5 over twelve pairs, and at a forced `E` the Sebenco boxes still split from
-38 to +16, which does not depend on `LAvg`). Sebenco: sky and far scenery (snow -65 and far building -41 against floor and
wall -13 to -17). Modesto: the sky only (-36 against floor +7 and wall +0.5). Moa: the stands (-16.1) and the stripe road
(-14.6), sky level; the stands belong to the queued Omega crowd-lighting lane. The Amphiseum: its ceiling truss (+39.9) and
stands (+25.1) are brighter in ours, and its frame carries no sky (the decomposition's sky region is 0.0 % of the frame). `Lighting.Sky brightness` (Sebenco 1.51, Modesto 2.264, Moa 2.0, Metropia 2.2,
Amphiseum 0.5) does not predict the sign (Moa's sky is level at 2.0, the Amphiseum's ceiling is 40 grey bright at 0.5), the
same mixed result the 2048-mode lane recorded ("Where the WipEout 2048 mode pairs' error is", sky). Not wired.

**The one value that would separate "same scene, different exposure" from "different scene" is the original's own luma
history** (read since: [the next section](#hd-mode-frame-brightness-the-originals-luma-ring-read-2026-11-20-omega-luma-read)). Read recipe, not run at the time (no pad seat in this lane's brief, and a native-port boot needs the 47 GiB game copy):
in the native port launched through `scripts/wipeout-pc-omega-pair.py` the object is a heap pointer held at guest
`0x021dbdb8` ([patch-v107-addresses](../ghidra/functions/ps4-omega-eu/patch-v107-addresses.md), "The ToneMap object, read live
through the pointer"; host address image base `0x800000000` plus the offset from Ghidra's `0x01000000`); after the pair
script's `SIGSTOP`, read the history ring `ToneMap_SetHistoryWeights` sizes at the object's `+0x1d8` (N-1 slots, 4 on these
circuits; the begin/end pointer layout is inferred from the weights vector at `+0x1f8`, which was read live, the ring itself
was not). `ToneMap_UpdateCoefficients` is unresolved on 1.07, so **confirm the ring's offset before trusting a mean**, and
whether the slots hold raw frame luma or past `LAvg` is not pinned (they agree once settled). Its mean is the original's
`LAvg` on the same frame as `theirs.png`, to compare with ours' `frameL` above (Sebenco 0.1756, Moa 0.3851, Amphiseum 0.1333,
Modesto 0.2621): a ratio near 1 means the scene is the same and the gap is how pixels are shaded; a ratio near 1.6 on
Sebenco means the original's scene is brighter before the curve. The coefficient texture (`LAvg`, `E`) is read back by the
CPU on hardware (`ToneMap_UpdateCoefficients`), so a direct read of it is not tried here, not ruled out.

**Closed negatives:** a global exposure or gain (the signed bias spans -22 to +5 over twelve pairs); an HDR tail inflating
ours' `LAvg`; the original still adapting at the grab as a common cause; `Sky brightness` as a sky multiplier (sign
mixed). **Unchanged by this lane:** no code, no table; the HD and 2048 frames are untouched. **2048: checked, differs** (2048
runs `wo_composite`'s exposure, not this cubic chain, and its references are drawn at the held 1.15 because Vita3K never
writes the bloom gate's target back, `oag_raceplay::Options::exposure_hold`).

## HD-mode frame brightness: the original's luma ring, read (2026-11-20, `omega-luma-read`)

**Claim decided:** at the grab frame of `omega/sebenco-climb`, `modesto-heights`, `moa-therma` and `the-amphiseum`, the original's adapted
luminance and exposure are either about equal to ours (so the dark sky and scenery is per-surface shading) or about 1.6 times brighter in
scene terms (so the scene's light level is the cause). **Written down before the first boot:** the deciding number is the ratio of the
original's `LAvg` to ours' (Sebenco 0.1756, Moa 0.3851, Modesto 0.2621, Amphiseum 0.1333), not `E`, because `E` is pinned on three of the
four circuits (a brighter scene leaves a pinned `E` unchanged); bands 0.85 to 1.18 "about equal", 1.4 or more "about 1.6 times", between
"partial"; the claim would have been unreadable if the ring were zero or constant, if its length were not `N - 1` for the circuit's own
`Tonemap.Exposure time`, or if `+0x80..+0x9c` did not match the circuit's `.EnvSettings`. None of those happened.

**Result: neither alternative holds across the four.** The original's whole-frame luma, read live, is **1.23 to 1.64 times ours on Sebenco,
Moa and Modesto and equal on The Amphiseum**, and the pixel-level split inside each frame shows it is concentrated in some surfaces. The
exposure is not the lever: `E` is identical to ours on three circuits (pinned) and 0.60 against ours' 0.868 on Modesto, where ours' dim
scene is compensated by a larger `E` than the original needs. So the gap is the scene's light level, surface by surface (confidence 80),
not `E` and not a global gain.

### What was read, and how

The port (`scripts/wipeout-pc-omega-pair.py`'s `Rig`, the same walk as the gallery pairs, a fresh save, Feisar, HD mode, `OAG_PAD_SEAT=2`)
was booted once per circuit and again (eight boots), and `/proc/<gow3-probe pid>/mem` was read in a loop (1 ms sleep, a row kept
whenever the object or the ring changed, 2,500 to 3,500 rows a boot; `scripts/wipeout-pc-tonemap-ring.py`, one boot of about 80 s: `OAG_PAD_SEAT=N UV_OFFLINE=1 python3 scripts/emu-run.py -- python3 scripts/wipeout-pc-tonemap-ring.py --scratch <dir>/inst/a --game <the port's game directory> --port <the port's checkout> --out <dir>/runs/<circuit>-a --track-steps N`, with N the Rights on Track Select from Vineta K: Moa 2, Sebenco 5, Modesto 8, Amphiseum 9; it prints the ring, its mean, `E` and `t1` and writes `samples.jsonl`, `frozen.json` and the frames) from MAIN MENU until the frame the pair script would pick (first HUD
frame plus 2), then the port was `SIGSTOP`ped and the object and every pointer in it dumped. The loop only reads: no stop, no write.

- **Pointer:** guest `0x021dbdb8` (host `0x800000000 + 0x11dbdb8`) holds the `ToneMap_Instance` pointer, `0x210` bytes
  ([patch-v107-addresses](../ghidra/functions/ps4-omega-eu/patch-v107-addresses.md), "The ToneMap object, read live through the
  pointer"). Its value is not fixed across boots (`0x100243d740` on six of the eight, `0x10024695e0` and `0x1002a32220` on two): always read
  it through the global.
- **The ring, confirmed by reading it** (confidence 90; one binary, eight boots, capped at 94 by the rubric): `+0x1d8` begin, `+0x1e0` end,
  `+0x1e8` capacity, a `float[4]` (`N - 1` with `N = round(0.08333334 * 60) = 5`, `+0xc4` = 0.2 on all eight boots), the weights vector at
  `+0x1f8..+0x208` (five `1.0f`). **Each ring write is one frame**: the slots are written round-robin, about one every 30 to 40 ms in this
  headless port (27 to 30 frames a second of ring writes under the headless compositor, whatever the engine's 240 FPS profile steps by), and the
  only other field of the object that moves is `+0xc0`: its byte `+0xc2` is the index of the slot written last (0, 1, 2, 3 in turn, checked
  against the slot that changed on 14 consecutive writes), and `+0xc1` alternates 0, 1 (the coefficient texture's ping-pong; reading, not
  confirmed).
- **What a slot holds: raw per-frame luma, not the adapted value.** Across the race-start camera cut on Sebenco the slots step from 0.351 to
  0.228 in one write (a drop of 0.12), where the executable's rate limit `response / 60` would allow 0.004 a frame
  (`tonemap.md`, "The law"). Fades read 1.0 (0.97 to 1.0 for about 20 s of loading and the white-out before the race), so the
  slot is the whole-frame mean luma of the scene target, fade and all. That agrees with `tonemap.md`'s "stores it in the ring" (the
  luminance chain's mean, and with `tonemap.md`'s own law line `target = (L + sum of the previous N-1 frames' L) / N`) and not with a past `LAvg`
  read back from texel 1 of the coefficient target (confidence 80). Two more readings: mid fly-over on Sebenco (22.6 s before the grab) the four
  slots read 0.3608, 0.3843, 0.3739 and 0.3674, a spread of 0.024 where four consecutive rate-limited `LAvg` values (0.0042 a frame) could
  differ by at most 0.0125; and the cut's step is seen on each of the eight boots.
- **State proof per boot** (the pair script's name-crop and Feisar logo references no longer exist, so the object proves the circuit):
  `+0x98/+0x9c` (`Exposure minimum/maximum`), `+0x80` (lum a) and `+0x88 * 60` (response) equal each circuit's own patch `.EnvSettings`
  (Sebenco 0.555/4, 0.079, 0.25; Moa 0/1, 0.4, 0.5; Modesto 0.5/2, 0.18, 0.4; Amphiseum 0.4/2, 0.25, 0.3), `+0xb8` is 1.0 (the SDR set),
  `+0x8c` is 1.0 on all eight boots. All eight picked frames were read by eye in one montage: the right circuit each time, the grid before GO (timer 0.0.0, 0 KM/H, no countdown digit).
- **Adapted luma at the grab.** The original's `LAvg` (the GPU's coefficient texture, not readable here) was replayed from the ring's
  own per-frame luma sequence, one ring write a frame, from the object's creation at `P = 0` as the executable starts, with the law of
  `tonemap.md` (target over the last 5 values or the last 4, both give the same answer to 0.001). At the frozen moment the replay and the
  ring mean differ by 0.0001 to 0.0011 on every boot: **the original is settled at the grab** (a rate limit of 0.0042 to 0.0083 a frame
  against 27 to 89 frames since the last cut: no adaptation transient, confidence 80; it is a replay of a law, not a read of `P`).

### The original's numbers

Ring mean (`LAvg` settled), boot A then boot B; `E` and `t1` derived from it with the object's own coefficients (`E = clamp((a + b * LAvg) /
LAvg, min, max) * cL`, `t1 = max(srcA + srcB * LAvg, 0.01)`); ours from the `omega-hd-exposure` readback:

| Pair | original `LAvg` (A, B) | original `E` | original `t1` | ours `LAvg` | ours `E` | ours `t1` | ratio `LAvg` | band |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| The Amphiseum | 0.1324, 0.1325 | 2.000 (max) | 1.000 | 0.1333 | 2.000 (max) | 1.000 | 0.99, 0.99 | equal |
| Sebenco Climb | 0.2290, 0.2299 | 0.555 (min) | 0.612 | 0.1756 | 0.555 (min) | 0.612 | 1.30, 1.31 | partial |
| Moa Therma | 0.4720, 0.4740 | 1.000 (max) | 1.570, 1.573 | 0.3851 | 1.000 (max) | 1.465 | 1.23, 1.23 | partial |
| Modesto Heights | 0.4197, 0.4299 | 0.609, 0.599 | 1.000 | 0.2621 | 0.868 | 1.000 | 1.60, 1.64 | about 1.6 |

Boot to boot the ring mean agrees to 0.1 to 2.4 % (a ninth boot, the committed script's first run, read the Amphiseum at 0.1318) (Modesto's boot A was still falling 0.01 a second, its scene settling). The ring's history
before the grab, mean of the four slots, relative to the pick grab (seconds; run A; loading and the fly-over are the intro, not the grid):

| Pair | -30 s | -20 s | -10 s | -5 s | -2 s | -1 s | -0.5 s | grab |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Amphiseum | 0.989 | 0.089 | 0.069 | 0.086 | 0.076 | 0.132 | 0.133 | 0.133 |
| Sebenco | 0.989 | 0.196 | 0.369 | 0.444 | 0.373 | 0.352 | 0.230 | 0.230 |
| Moa | 0.989 | 0.645 | 0.703 | 0.632 | 0.577 | 0.588 | 0.472 | 0.471 |
| Modesto | 0.989 | 0.445 | 0.804 | 0.712 | 0.697 | 0.434 | 0.425 | 0.419 |

(0.989 is the loading screen's fade; the camera cut from the circuit presentation to the grid is the step in the last second.)

**Sampling is not the explanation** (confidence 70): the Amphiseum's ring mean (0.1324) matches ours' whole-frame mean (0.1333) to 1 %
while ours' centre 64x64 mean is 0.0859 and its centre 128x128 mean 0.091. No one centre-crop statistic of ours' luma image gives a
constant ratio across the four (centre 64: 0.97, 1.90, 1.46, 1.54 against the Sebenco, Moa, Modesto, Amphiseum originals), so the
confidence-60 reading in `tonemap.md` (the coefficient pass samples the whole-frame mean) is now **supported by the one circuit
where ours and the original agree** and not contradicted by the others.

### Where the original's scene is brighter: the output frames, inverted (corroboration, confidence 65)

The original's `theirs.png` is the curve's output (sRGB-encoded) plus bloom. Inverting it with the object's own parameters
(sRGB decode, the cubic through `(0, 0)` and `(t1, 1)` with the end slopes of `+0x90/+0x94`, divide by `E`, Rec.601 luma) gives a scene
luma that can be compared with the same inversion applied to ours' frame, since both pass through the same law; ours' inversion
recovers 96 % (Sebenco), 94 % (Modesto), 80 % (Moa) and 83 % (Amphiseum) of ours' own known `LAvg` (the shortfall is the clip at `t1`).
Whole frame, theirs over ours: Sebenco 1.41, Moa 1.28, Modesto 1.48, Amphiseum 0.93, against the ring's 1.30, 1.23, 1.60, 0.99. The
two independent readings agree on the sign and order. **Per box** (the `omega-hd-exposure` boxes; ranges are boot A to boot B):

| Pair | box | theirs over ours (inverted scene luma) | saturated pixels, theirs |
| --- | --- | --- | --- |
| Sebenco | snow sky / building left | 2.3 to 2.5 / 2.3 to 2.5 | 6 to 9 % / 0 % |
| | yellow floor / wall right | 1.15 to 1.21 / 1.08 to 1.41 | 0 % |
| Moa | sky/city / floor left | 1.04 / 1.01 to 1.05 | 0 % / 5 % |
| | stands left / stripe road | 1.42 / 1.6 | 12 % / 3 % |
| Modesto | sky gap / left structure | 2.2 / 1.80 | 22 to 42 % / 2 % |
| | right wall / floor left | 1.72 / 1.42 | 4 % / 9 to 10 % |
| Amphiseum | ceiling / stands right | 0.67 / 0.73 | 5 % / 4 % |
| | road left / wall left | 1.05 / 0.90 | 4 % / 1 % |

Caveats that bind these ratios: bloom adds to the original's bright pixels after the curve and the inversion reads it as scene (it
inflates the sky and ceiling rows); the clip at `t1` makes saturated boxes lower bounds; the pose differs by a few pixels, so a box ratio is
good to about 0.2, not to the digit. What survives them: **Sebenco's sky and far scenery are about 2.3 times ours and its near surfaces
1.1 to 1.4; Modesto is brighter everywhere (1.4 to 2.2, sky most); Moa's sky and floor match and its stands and stripe road are 1.4 to 1.6;
the Amphiseum's ceiling and stands are brighter in ours (0.67 to 0.73).** That reproduces `omega-hd-exposure`'s box table (sky and far
scenery dark on Sebenco, sky on Modesto, stands and stripe road on Moa) as a scene-radiance gap, and puts a number on it that the tone map
passes through one for one on Sebenco (`E` pinned, `b = 0`) and the Amphiseum; on Moa `t1` moves with `LAvg` (1.57 against 1.465) and on Modesto `E` does.

### What this decides

1. **Exposure and the tone map are not the cause** (confidence 85): `E` equals ours on Amphiseum, Sebenco and Moa (all pinned), `t1`
   on Amphiseum and Sebenco (1.57 against 1.465 on Moa), and ours' Modesto `E` (0.868) is 1.45 times the original's (0.60): ours' dim scene (`LAvg` 0.262 against 0.42)
   asks for more gain than the original needs, which is a symptom, not a cause.
2. **The scene's light level differs from ours, by surface and not by one factor** (confidence 80):
   whole-frame scene luma is 1.3 (Sebenco), 1.23 (Moa), 1.6 (Modesto) times ours and equal on the Amphiseum, with sky and far scenery
   about 2.3 times ours on Sebenco and a 1.4 to 2.2 spread on Modesto. A lighting term that is a constant times ours' would be wrong on
   the Amphiseum (0.67 to 0.73 on two boxes) and on Moa's sky and floor (1.0).
3. **The earlier claim "scene luma equal, the gap is shading" is false** for Sebenco, Moa and Modesto (ring ratios 1.23 to 1.64 over
   two boots each); the earlier claim that it is **not a global gain** stands, now on the original's own ring.

**Not decided here:** which scene term carries it. The sky and fog consumers in the 1.07 eboot are still unread
(`FUN_015eda20` is the registrar), `Lighting.Sky brightness` still does not predict the sign across the five circuits, and a 2.3 times
sky on Sebenco with a 1.1 times floor does not look like the sun or ambient terms; sky radiance (and bloom, which ours does not draw)
are the leads. **Not wired.** No code, no table.

**2048 cross-check: checked, differs.** 2048 reads no `ToneMap` object (its `wo_composite` has no luma ring), so the read does not port; the
Omega object's `+0x1d8` ring is Omega's. The 2048-mode pairs' brightness (`omega-2048-holistic`) was not re-read.
**Repeats:** each circuit twice, one boot apart, ring means within 2.4 %; **confidence** per row above; one binary (the 1.07 patch eboot),
one pose per circuit (the grid before GO), HD mode only.

## HD-mode frame offset: which stage carries it (2026-11-20, `omega-frame-offset`)

**Claim tested:** Omega's frame-wide darkness against the original (Sebenco Climb about 18 grey under, Moa Therma 14 under) is
carried by one named stage of the post chain (bloom, the tone curve, the exposure `E`, or the gamma and encode), not by the scene.
**Written down before the first boot:** bloom would be a *minority* of the offset (the ring puts scene luma at 1.30 times ours on
Sebenco with `E` pinned and `b = 0`, so a post stage can only be the remainder); it would be *the* stage if the original's frame
with its bloom knocked out sat at least 12 grey under the original's own on Sebenco while ours' bloom stayed under 4; it would be
*not* the stage if that difference were under 6; a poke that leaves the frame byte-identical hit a dead copy and is not a result.
**Result: no post stage is the offset.** The tone curve and `E` are the original's law with the original's numbers on three of the
four pairs, the encode is the shared sRGB encode, and bloom is **3.5 grey of Sebenco's 18** (ours draws none there). The rest, 14.6 grey
on Sebenco, is before the curve: scene radiance (the ring puts the original's scene luma 1.2 to 1.3 times ours) and an unattributed
4 to 6 grey that a global gain does not reach (the closure below).

### Method

Ours: the four pairs re-rendered from their own flags with the current tree (`408f6fe6e`, anisotropy 16x as the pairs record it)
and, for the bloom stage, a local uncommitted switch that zeroes `BloomScale` (the render is otherwise identical; not kept). Theirs:
the native port (`scripts/wipeout-pc-bloom-knockout.py`; seat 5, own sway, own profile, a copy of the port checkout, no audio). The
script walks to the grid as the pair script does, freezes the port (`SIGSTOP`), scans its memory for the circuit's own `Bloom.*`
values read from its `.EnvSettings` (`Blend factors`, and `Threshold, Offset, Scale, Power, cBrightness x3, AlphaScale` as one
block), then alternates: poke while frozen, run 0.06 s, freeze, `grim` grab, restore, run, freeze, base grab. **`nobloom`** writes
`Blend factors[0]` (the composite's `BloomScale`) to 0 in the env object's copy and the level array's (`+0` of a `0x108` stride):
2 sites, held (read back after the run) on every sample counted. **`nomask`** writes `AlphaScale` to 0 in the block's guest copies
(4 to 22 hits; some are overwritten per frame, so it is a lower bound on the mask's share). The effect is the knockout's Rec.709
grey (sRGB values) minus the mean of its two neighbouring base grabs, as `scripts/omega-bloom-knockout-effect.py` computes it,
because the grid camera drifts: two bases 0.06 s apart differ by 8 to 14 grey per pixel and by under 1 in the frame mean. A sample is
dropped when its bases are identical (the port had ended: after about 14 to 30 stop and continue cycles its GPU log reads "Wrong PM4
type 1" and the last frame stays up) or when the poke did not hold. Boots counted for bloom: Sebenco 2 (D, E), Modesto 3, Moa 2, Amphiseum 2. Not counted: two Sebenco boots that wedged on the first
poke of an earlier script (a poke into the host's copies of the constants), one that failed on a write, one Modesto boot that raced
another circuit, and two Sebenco boots (F, G) that ran the exposure poke (below) before the bloom knockout: the exposure answers a
poke several frames late, so their base grabs after it are contaminated and their `nobloom` reads +6.9 and +9.3 (brighter, impossible
for an additive term).

A boot proves its circuit by the scan: the circuit's own bloom constants are found (1 `Blend factors` hit, 1 level-array hit) or
the script finds none. One Modesto boot raced another circuit (Track Select stepped wrong) and found 0 hits: discarded.

### Per stage, ours against the original, same pairs

| Stage | Sebenco Climb | Modesto Heights | Moa Therma | The Amphiseum |
| --- | --- | --- | --- | --- |
| Frame grey, theirs / ours / signed bias | 103.8 / 85.7 / **-18.1** | 90.8 / 91.5 / +0.7 | 110.2 / 96.3 / **-14.0** | 83.5 / 87.8 / +4.3 |
| 1. Scene luma before the curve (`LAvg`), theirs against ours (ours at `13b22ac77`) | 0.229 against 0.176: **1.30** | 0.42 against 0.262: 1.60 | 0.472 against 0.385: 1.23 | 0.132 against 0.133: 0.99 |
| 2. Exposure `E` (min, max pins), theirs / ours (`13b22ac77`) | 0.555 / 0.555 | **0.60 / 0.868** | 1.0 / 1.0 | 2.0 / 2.0 |
| 3. Curve end `t1`, theirs / ours (`13b22ac77`) | 0.612 / 0.612 | 1.0 / 1.0 | 1.571 / 1.465 | 1.0 / 1.0 |
| 4. Bloom the original adds (whole frame, grey) | **+3.5** sd 0.4 (boots +3.15, +3.73; 10 samples) | 0.2 to 5.1 (boots +5.1, +0.4, +0.2) | -0.8 to +1.4, about 0 | 1.4 to 4.8 (2 boots, 7 samples) |
| 4. Bloom ours adds | **0.00** | +3.42 | +0.63 | +6.49 |
| 5. Flat HUD yellow (position hexagon, R G B median), theirs / ours | 233 233 39 / 219 219 16 | 233 233 35 / 217 217 15 | 233 233 35 / 217 217 14 | 233 233 33 / 219 218 15 |

Stages 1 to 3 are the earlier lanes' live reads (`omega-luma-read`, the ring and the object's own coefficients). **The "ours" values of rows 1 to 3 are from `13b22ac77`**, before the sky, snow and rock programs brightened ours' frame (Sebenco 81.5 to 85.7 grey, its snow box 97.3 to 133.0); they were not re-read on `408f6fe6e`. An inversion of the current bloom-free frames (a lower bound: the clip at `t1` hides the brightest surfaces; it recovered 96 % on Sebenco on the old tree) puts ours' Sebenco `LAvg` at 0.187 or more (ratio 1.23 or less, about 1.18 at the old recovery), and Modesto's, Moa's and Amphiseum's inversions are too lossy to use (74 to 83 % recovery). Stage 4 is this lane's. Sebenco's bloom by box (grey the knockout removes, two boots): building left **-9.1 and -11.7** by boot (the lit Senco sign),
snow sky -1.4, yellow floor and right wall 0.0. Moa's sky/city reads -4.1 and +2.0 on its two boots (it flips sign: drift, so no
claim); Modesto's boxes move together with the frame mean (drift again).

**The mask term carries most of it.** Zeroing `AlphaScale` alone (`nomask`, samples with at least one copy held) removes -2.37 and -3.18 of the boot means -3.15 and -3.73 (75 and 85 %; 4 and 5 samples; only 1 of 4 and 1 of 6 `full`-block copies held, so the true share is at least this), building left -6.4 and -10.3 against -9.1 and -11.7. The
threshold term (`Threshold 0.944`, `Offset 1`, `Scale 2.55`) is what ours draws; the per-material mask `AlphaScale * mask.a`
(`AlphaScale 0.928` on Sebenco) is what it does not ([`tonemap.md`](../ghidra/functions/ps4-omega-eu/tonemap.md), "The mask, not
drawn"). The mask's contribution is therefore real and sign-correct on Sebenco, but it is a 3.5 grey term, not the offset.

**Closure, Sebenco (a model, confidence 50).** Ours' no-bloom frame decoded, its curve inverted (`t1 0.612`, the angles of
`.EnvSettings`), the scene scaled by a gain, run through the original's curve and encoded; then the original's measured bloom
(3.5) added. The ring's ratio against the **current** tree is not re-read (rows 1 to 3 above), so the gain is a range:

| Gain on ours' scene | Frame grey | Bias against theirs 103.8 | With the original's bloom +3.5 | Bias |
| --- | --- | --- | --- | --- |
| 1.00 | 85.7 | -18.1 | 89.2 | -14.6 |
| 1.18 (ring ratio at the old recovery) | 93.9 | -9.8 | 97.4 | -6.3 |
| 1.23 (the inversion's upper bound) | 96.1 | -7.7 | 99.6 | -4.2 |
| 1.30 (the old-tree ratio; stale: it predates the sky, snow and rock programs) | 98.9 | -4.8 | 102.4 | -1.3 |

So of Sebenco's 18.1 grey, bloom is 3.5, a scene-radiance gain of 1.18 to 1.23 is 8 to 10, and **4 to 6 grey is unattributed**:
a global gain understates a gap that sits in the sky and far scenery (2.3 times ours, `omega-luma-read`), which the 1.0 to 1.3
range does not reach. Moa the same way with its old-tree ratio 1.23 gives 101.9 (bias -8.3, its bloom about 0), Modesto's ring 1.60
with the original's lower `E` is a net 1.12 on the output and overshoots (96.5 against 90.8; the gap is the sky alone, -36 against
+0.5 on the wall, `omega-hd-exposure`), and the Amphiseum needs no gain (0.99: 80.8 against 83.5, ours' bloom on top 87.3 against
ours' own 87.8). These three used old-tree ratios and are indicative only.

**Render lag, checked on the same data.** The exposure poke below answers late, so the bloom knockout was checked for it: in
Sebenco boots D and E the base grab after a knockout reads the same as the one before it (whole-frame grey 98.6 then 98.6, 97.1 then
96.8, 98.3 then 97.3; the mean of the 9 pairs differs by 0.3), while the knockout grab itself reads 3.4 to 4.3 lower, and `nomask`
and `nobloom` reach the same level (94.6 against 94.8). So the knockout grab is the settled state, not a frame the pipeline had half
applied, and 3.5 is not an underestimate from lag (confidence 70).

### What this settles, and what it does not

1. **The tone curve and the exposure are not the stage** (confidence 85): identical numbers on Sebenco, Moa (`t1` 1.571 against
   1.465, from the higher `LAvg`), Amphiseum; Modesto's `E` is the symptom of its dim scene.
2. **Bloom is a stage, and it is 3.5 grey on Sebenco, not the offset** (confidence 80: two boots, ten samples, a sd of 0.4, with
   the poke read back). The prediction written before the boots held: a minority (19 %). On Modesto, Moa and Amphiseum the knockout
   is below this method's noise (about 1 to 2.5 grey on the whole-frame mean) and the boots disagree: **unread** there. Ours already
   draws bloom on those three (+3.4, +0.6, +6.5), none on Sebenco (nothing in its frame reaches `Threshold` 0.944 after the curve),
   so wiring the mask would add to Sebenco, and Amphiseum (`AlphaScale 6`, ours already 5 grey bright, ring ratio 0.99) is the
   circuit where it must not overshoot.
3. **The remaining offset is before the curve** (the ring, confidence 80, stands from `omega-luma-read`): about 8 to 10 grey on
   Sebenco is a scene-radiance gain and 4 to 6 is unattributed, in the sky and far scenery; it is per surface, not one gain (Moa and
   Modesto above).
4. **Encode and presenter: not shown to be the stage** (confidence 60). The shared sRGB encode and the cubic are the same law in
   ours and in the inversion that recovers 96 % of ours' own `LAvg`; the one flat-colour comparison available (the HUD, drawn after the
   composite in both, since its yellow is 233 on every circuit in theirs and 217 to 219 in ours) shows ours' flat HUD yellow
   **14 to 16 levels lower in red and green and 19 to 24 lower in blue**, the same across all four circuits, so it is a HUD colour
   difference (ours 6 % darker and more saturated), not a scene stage: Amphiseum and Modesto have no scene gap and show it equally.
   **Open, not this lane's:** the HUD yellow.
5. **Not done:** the exposure poke (`ehalf`: `cL` at the ToneMap object's `+0x8c` written to 0.5, held on every sample) did not give
   the frame the curve predicts (predicted grey 0.675 times the base; observed 1.02 and 1.16 times, one of twelve samples within
   3 grey), and its effect outlives the restore by several frames (the grabs after it carry it), so the original's `E` answers
   a write late or through a second path (the coefficient texture is
   read back by the CPU): the curve and encode were not validated against the original by a knockout. A poke of the bright-pass
   `Threshold` to 2 (`highthr`, which leaves only the mask) wedged the port's GPU queue twice, on the host's copies; the script now
   writes only the guest's own range and does not run it by default.

**Comparison environment.** `scripts/compare-environment.toml` sets no bloom, tone map or exposure key for Omega (only 16x
anisotropy, 1920x1080, `shadows = "original"`), and the pair flags carry none: ours runs the title's own chain with the circuit's
authored `Bloom.*` and `Tonemap.*`. No mismatch. **Stale text found:** `crates/omega/src/lib.rs`'s comment on `EXTRA_CANDIDATES` and
this page's "`.EnvSettings` is read through 2048's reader" bullet say the patch's copy carries no bloom block so the chain is off in a
race; the patch's `data08` copies of all four circuits carry `Bloom.*`, `BloomHDR.*` and `Tonemap*.*` and ours draws the chain on three
of them (the measured +3.4, +0.6, +6.5 above). **Not run:** the HD and 2048 paths. **2048: checked, differs** (no bloom block in its
`.envsettings`, [`tonemap.md`](../ghidra/functions/ps4-omega-eu/tonemap.md)); HD: not checked (`hd_bloom` is another chain).

**Repeats and limits.** Sebenco two boots,
Modesto three valid and one wrong circuit, Moa two, Amphiseum two (one had a single valid sample); one binary (1.07 eboot), HD mode,
the grid before GO. Raw captures are kept off the tree.

## See also

- [`omega-frontend.md`](omega-frontend.md) - the front-end census this page's
  own findings extend: the GUI XML, the boot chain, the colour-skin switch,
  the 2048 campaign branch.
- [`psarc.md`](psarc.md#the-ps4-omega-collection-family) - the container-level
  block-data-location trap, and `data/README.md` for the extraction commands.
- [`hd-frontend.md`](hd-frontend.md) - the file this crate's own numbers are
  compared against, field by field.
