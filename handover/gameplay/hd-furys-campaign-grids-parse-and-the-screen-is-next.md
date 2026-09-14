# Wipeout HD/Fury's campaign grids now parse; the Grid Selection screen is next

2026-09-14. The discriminating question was: does `oag_tables::race_campaign::from_blob`
parse HD's `plugins/grids/grid_*.xml` unchanged, or does the schema need a
variant? **It needs extending, additively.** Done: `oag_tables::race_campaign`
now reads all 32 grid files across HD's four archives (`DATA00`, `DATA02`,
`DATA04`, `DATA06`), Pulse's own ground truth is unchanged, and
`crates/hd/src/campaign.rs` / `crates/hd/tests/campaign_grids_ground_truth.rs`
/ `docs/formats/race-campaign.md`'s new HD section carry the schema diff, the
counts, and what is and is not measured. Full detail is in that doc section -
this thread is the open work, not a restatement of what landed.

## What landed

- `race_campaign::Cell` gained `difficulty_targets: Option<DifficultyTargets>`
  (three `MedalTargets` triples: easy/medium/hard) and
  `nitro_elimination_targets: Option<(i64, i64, i64)>` (the raw
  `NitroElimNovice`/`Skilled`/`Elite` triple). `gold`/`silver`/`bronze` keep
  meaning the medium rung when the per-difficulty shape is authored, so no
  existing caller (`evaluate_medal`, `crates/game/src/records.rs`) changed.
- `race_campaign::Mode` gained `Other(String)` for `"NitroBattle"` and
  `"Detonator"`, HD's two mode spellings with no Pulse ordinal.
  `Mode::ordinal()`/`as_str()` changed signature (`&self`, `Option<u32>`) -
  contained inside `oag-tables`, nothing outside it calls either.
- `race_campaign::Grid` gained `campaign`/`title_color`/`text_color`/
  `flyer_name`/`billboard_name`, all raw `Option<String>`.
- `skill` reads `skillMedium` as a fallback attribute name.
- New: `crates/hd/src/campaign.rs` (entry names, the four-archive table),
  `crates/hd/tests/campaign_grids_ground_truth.rs` (four `#[ignore]`d tests,
  all green against `hdfury-ps3-eu-dec.iso`).
- One real, on-disc finding: `grid_04.xml`'s own `<Values>` tag is missing a
  closing `>` in all three of its copies, so this project's parser correctly
  reads zero cells for a grid that authors five. Not fixed - see the doc
  section for why.

## Open

- **Which target set a mode's own medal law reads is not measured.** A
  per-difficulty cell carries both a per-difficulty `Gold`/`Silver`/`Bronze`
  (dummy `1`/`2`/`3` on a non-counting mode) and a `NitroElimNovice`/
  `Skilled`/`Elite` triple (dummy `1`/`1`/`1` on a non-counting mode, real
  values on `Elimination`/`NitroBattle`). Nothing here decides whether
  `evaluate_medal` should read the second triple for those two modes instead
  of the first - that is a decompiled-HD-executable question, explicitly out
  of this pass's scope per the driving brief.
- **Whether HD's own front end tolerates `grid_04.xml`'s broken `<Values>`
  tag** - i.e. whether the real game shows five cells there or none - is the
  same kind of question, also not chased.
- **Only the EU Fury PS3 pressing was measured** (`hdfury-ps3-eu-dec.iso`). A
  base-HD-only disc (pre-Fury, no `DATA00` grid_08..15) was not available to
  this pass.

## Next Steps

The next step is drawing HD's `Grid Selection` screen off
`plugins/frontend/gui/*.xml`, once Pulse's own campaign screens (in progress
in `crates/ui` as of this thread) land and the `Draw` vocabulary they use
exists to build against. HD's equivalent of Pulse's `CellMode_Definition.xml`
is `Data\Plugins\Frontend\Gui\CellMode_Definition.xml`, on `DATA02.PSARC`
(42,548 bytes - the copy `oag_assets::Archives`'s own precedence reaches, `fe`
= `DATA02`) and, disagreeing, on `DATA06.PSARC` (59,361 bytes - larger,
presumably a later build; not diffed against the DATA02 copy this pass, so
treat any difference between them the same open-precedence-question way
`crates/hd/src/campaign.rs` already treats `Definition.xml`'s own five
copies). Read with `oag_assets::psarc::Archive::read_path`, the same way
`campaign_grids_ground_truth.rs` reads a grid file - it is plain UTF-8, not
name-shortened, so no `oag_tables::fexml` dictionary expansion is needed
first.

What `DATA02`'s copy authors, so that pass can start cold:

- **`<Screen type="FlyerSelection" name="Grid Selection" ...>`** is the
  screen's own name and type, nested inside a `<Flyer name="FlyerModel">`
  block that sources `Data\FE\Flyers\00_flyer.vex` - the 3-D flyer model
  behind the hex grid, matching `docs/formats/race-campaign.md`'s
  `FlyerName`/`BillboardName` grid attributes (per-grid flyer stem and
  billboard, composited into this one shared model/screen).
- **`<GridController name="Grid">`** is the hex-grid widget itself. Inside it,
  every cell position `_R_C` (row/column, matching `Cell::grid_coords`'s own
  `_x_y` reading) repeats five image layers - `Bg_R_C`, `Outline_R_C`,
  `Lock_R_C`, `Medal_R_C` (each a 3-5-frame flip-book keyed by medal tier,
  `Hexmedal_HD.mip`) - plus one shared `Selector` overlay
  (`Hexagon_HD_THICK_OUT.mip`, tinted `0x3fffffff`). The three hex textures
  are `Hexagon_HD.mip` (a cell), `Hexagon_HD_OUTLINE.mip` (its border) and
  `Hexlock_HD.mip` (the locked overlay, alongside a `Padlock.gtf` used
  elsewhere in the screen).
- **The right-hand detail column** (`Event Emblem`, `Track Emblem`, `Speed
  Class Emblem`, `Weapons Emblem`, each a `Bracket`-framed `Text`+`Image`
  pair) names the four facts a selected cell shows: event type
  (`RB_EVENT_TYPE`), track (`RC_TRACK`), speed class (`RC_SC`), weapons
  (`FE_WEAPONS`) - all via `NonSelectable_Arrow_HD.gtf` as the row's own
  bullet, `Subtitle_Arrow_HD.gtf` elsewhere. A `Track Reverse` icon
  (`reverse_icon_mini.gtf`) sits beside the track row for a reversed-grid
  cell.
- **A three-target row** (`Target0`/`Target1`/`Target2`, each a
  `Hexmedal_HD.gtf`-backed medal icon plus a value `Text`) is exactly the
  medal-target triple this project's own `Cell`/`DifficultyTargets` now
  parses - `IG_HUD_TARGET` titles it, `IG_HUD_GOLD`/`SILVER`/`BRONZE` are the
  three tier strings. **This row is three wide, not nine** - it shows one
  target triple at a time, which is the strongest screen-side evidence yet
  that a difficulty selector switches which triple this row (and the medal
  icons) reads, rather than all nine ever being on screen together.
- **`<Text name="DifficultyButton">`** (`"Change Difficulty"`, a `δ`-glyph
  icon in the `buttons` font) is on **both** archive copies of this file,
  including `DATA02`'s - the one this project's own read precedence reaches
  for `grid_00.xml`..`grid_07.xml`, which is the **flat**-schema copy with
  only one target triple per cell. So the screen offers a difficulty toggle
  even where the underlying grid data (as this project reads it) has nothing
  for that toggle to change - either the flat grids reuse one target triple
  across all three difficulties on screen, or the real game reads
  `DATA04`'s/`DATA06`'s per-difficulty copies of those same eight grids
  instead of `DATA02`'s, contradicting `oag_assets::Archives`'s generic
  precedence for this one path. Worth resolving before the screen pass
  decides how to feed this widget.
- **Left/right column** (`RC Laps`, `Record`, `Points`, `Best`, each
  `Subtitle_Arrow_HD.gtf`-bulleted) and **unlock box** (`unlockbox`,
  `unlockboxbrackets`, `RC_POINTSTOUNL`/`RC_POINTSACH`/`RC_TOTPOINTSAV`) round
  out the screen - a grid's required/earned/total points, matching
  `Grid::required_points` and the points-possible law already in
  `Grid::max_points`.
- Redirects at the bottom (`<Redirect name="Cell Mode Redirect Team">`,
  goto `"Team Selection"`) are the wiring into what a cell launches into -
  relevant once a Cell Selection pass exists to chain off this one; one
  redirect (`Cell Mode Redirect Game` -> `"Launch Game"`) is commented out on
  the disc itself (`<!--ZLIU removed...-->`), a real, authored deactivation
  rather than something to reactivate.
