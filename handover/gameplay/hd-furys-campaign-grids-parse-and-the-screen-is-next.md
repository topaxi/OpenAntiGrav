# Wipeout HD/Fury's campaign grids now parse, and `Grid Selection`/`Cell Selection` draw

2026-09-14, later the same day. Both screens now draw off the real disc -
see `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury" section for the full
picture (what's measured, what's chosen, two shared-parser fixes, capture
paths). This section is the historical record of the parsing-only pass;
the screen-drawing pass's own detail lives in the docs page, not repeated
here.

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

## 2026-10-08, `hd-campaign` lane: the HD campaign plays through to a persisted medal

Walked live (Xvfb, `--autopilot`, own XDG): `Main Menu` -> `Campaign Selection` ->
`Grid Selection` -> `Cell Selection` -> `Team Selection` -> race (Anulpha Pass Speed
Lap, `grid0_3_2`) -> "SILVER MEDAL AWARDED" at 37.03 -> `records.toml` `[[campaign]]`
(`best_medal`, `best_difficulty = "easy"`) -> `Cell Selection` shows `SILVER`, `2/18
POINTS`, neighbouring cells unlock. A locked cell refuses Confirm. **Fixed**: the base
campaign read `DATA02`'s flat targets (= `DATA06`'s hard rung) for every rung; it now
reads `DATA06` (`Title::campaign.grid_archive`). **Fixed**: an HD `Elimination` cell scored
gold for one kill and launched with kill target `1`; it now awards nothing and keeps the
race's own target. Ground truth: `crates/game/tests/hd_campaign_ground_truth.rs`.
Evidence: `docs/formats/race-campaign.md`, `docs/overview/status.md`.

**Still open**: (1) the unit and tier law of `NitroElim*` (200..300 on `Elimination`,
12..26 on `NitroBattle`; kills or score points?) - an RPCS3 `Elimination` cell played to
a medal would settle it; (2) leaving a Speed Lap/Zone cell with Escape lands on the main
menu (a parked race behind it), not `Cell Selection`, shared with Pulse; (3) `Tournament`
cells launch by `plan_cell` but were not walked; (4) base-campaign target numbers were not
compared against an RPCS3 frame; (5) `NitroBattle`/`Detonator` (25 cells) refused, no rules.

## Open

- **Which target set a mode's own medal law reads is partly measured now,
  2026-09-21.** `EBOOT-ps3-hdfury-eu.elf`'s own `PI_Cell` attribute table
  (`0x008ae898`) lists `NitroElimNovice`/`Skilled`/`Elite` as three of its own
  fields, immediately after `EasyGold`..`EasyBronze` - confirming this is a
  real `PI_Cell` attribute, not a coincidental string reuse from an unrelated
  subsystem, and a second string (`0x00779a98`) measures
  `Novice`/`Skilled`/`Elite` as this title's own words for
  `Easy`/`Medium`/`Hard`. `Cell::nitro_elimination_target_for_difficulty` now
  reads the measured number for a rung. **Still open**: the actual medal
  comparison consumer was not found - `PI_Cell`'s binder is
  reflection-driven, so nothing in the image references these attribute
  names at their point of use, and reaching the consumer needs the
  attribute table's own per-entry encoding decoded first. Also open: whether
  the triple represents three medal tiers at all, or one pass/fail target
  per rung (the disc's own vocabulary - `NitroElim*` spelled in rung words,
  not tier words - argues for the latter, but this is not independently
  confirmed). See `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md` for
  the full evidence and `docs/formats/race-campaign.md`'s HD section for the
  data-format summary.
- **Whether HD's own front end tolerates `grid_04.xml`'s broken `<Values>`
  tag** - i.e. whether the real game shows five cells there or none - is the
  same kind of question, also not chased.
- **Only the EU Fury PS3 pressing was measured** (`hdfury-ps3-eu-dec.iso`). A
  base-HD-only disc (pre-Fury, no `DATA00` grid_08..15) was not available to
  this pass.

## Next Steps

Done, this same day: both screens draw, pointer-driven, off the real disc -
see `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury" section. What is
still open moved there too (the archive-precedence/RPCS3 question, the
stray hex artifact, the undrawn flyer model, the launch path's live
verification) rather than being duplicated in both places - this thread's
own `## Open` above is the parsing-level items only, unchanged by the
drawing pass.

**2026-09-21, later: `Campaign Selection` itself now exists.** The
2026-09-21 RPCS3 pass above found the real disc has a `Campaign Selection`
screen ahead of `Grid Selection` that this build had no state for at all,
defaulting to `Fury` and reaching this build's own base-HD-only grids by
neither input nor screen. `oag_ui_screens::campaign::selection::CampaignSelection`
now models it, read off `DATA06`'s own copy of `CellMode_Definition.xml`
(the precedence-resolved `DATA02` copy has neither this screen nor `Grid
Selection Fury` at all - see `oag_hd::campaign::SCREEN_ENTRY`'s own doc),
and `crate::campaign_stage::CampaignStage` opens on it for HD, slicing the
sixteen grids into `grid0`..`grid7`/`grid8`..`grid15` per the campaign
confirmed. `right` is measured, three RPCS3 boots, as the toggle to the
base `Wipeout HD` campaign - see `docs/ui/campaign-screens.md`'s own
"Wipeout HD/Fury: `Campaign Selection`" section for the full read,
including the `~50` confidence still open on `up`/`l1`/`r1`.

**Left open by this pass**:
- ~~**A live, interactive walk was not completed**~~ **Closed 2026-09-28**:
  walked live on Xvfb, `Main Menu` -> `Campaign Selection` (HD and Fury) ->
  `Grid Selection` -> `Cell Selection` -> race -> `EndRace Results` ->
  `EndRace Menu` -> `RETURN TO GRID` -> `Cell Selection`. See
  `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: walked live, end to
  end, 2026-09-28". Found on that walk and still open: ~~no `Team Selection`
  on HD~~ (**closed 2026-09-29**, see the section at the end of this file),
  `EndRace Menu`'s rows have no visible cursor on the light panel,
  and the 8th Results row overlaps the panel's footer bar. The earlier
  note follows. `Main
  Menu` -> `RACE CAMPAIGN` -> pick a campaign -> `Cell Selection` is proven
  by a headless `--menu-page campaign-select` capture and by model-level
  unit tests, but not by a running session: an X11 input-delivery gap
  under Xvfb (confirmed with a `std::fs::write` sentinel showing
  `Session::open_campaign` is never reached despite `XSetInputFocus`
  reporting the right window focused) stopped this pass short of it. See
  `docs/ui/campaign-screens.md`'s own "Verification" section for exactly
  what was tried. Whoever next has a working interactive Xvfb setup can
  close this directly.
- **Closed, 2026-09-27**: `DATA06`'s own `Cell Selection` diverged from the
  `DATA02` copy this build used to read (an extra `bBg_x_y` background
  layer, `Target0/1/2 Image`/`Title` replaced by a shared `Target Title`
  header plus per-rung `Target0/1/2 Medal` icons, a repositioned
  `GridController`, a reflowed detail column). `oag_game::campaign::load_hd`
  now reads `DATA06` for `Grid Selection`/`Cell Selection` too, on a
  general last-wins archive-overlay argument plus a direct per-widget
  RPCS3 confirmation - see `docs/ui/campaign-screens.md`'s "Wipeout
  HD/Fury: the TARGET block reads `DATA06` too" section for the full
  writeup, including three more bugs the switch's own captures surfaced
  (a `.gtf`/`.mip` sprite-key mismatch that left `Target0/1/2 Medal`
  undrawn, two overly-narrow `Weapons`/`Target` mode gates, and the
  mode-dependent `TARGET` header text). The grid files themselves
  (`grid_00.xml`..`grid_15.xml`) still read through the unchanged
  precedence - deliberately not extended, a separate, bigger question (see
  that doc section's own "not settled this pass" note).
- **Closed, 2026-09-27**: the latent medal-glyph bug this file's own earlier
  note left for "whoever next drives a podium finish" - `Medal_{x}_{y}`
  drawing the whole 1024x256 `Hexmedal_HD` atlas stretched over one hex,
  tinted a second time on top. Fixed without needing a real podium finish
  (seeded `records.toml` instead); the crop rect came from `DATA06`'s own
  `Target0/1/2 Medal` widgets - only their crop numbers were borrowed at
  the time, not the wider widget-name switch, which a later same-day pass
  made (see the bullet above). `EPoints Title`'s own
  `gold_medals`/`cell_count` mix-up (should read `points_earned`/`max_points`,
  confirmed against a live RPCS3 frame) was found and fixed the same pass.
  See `docs/ui/campaign-screens.md`'s "That latent bug was real and is
  fixed" section for both. `EndRace Results` was checked too, as the more
  obviously-reachable screen (no
  medal needed to see a `TimeTrial`/`Zone`/`Elimination`/`SpeedLap` cell's
  own target row) - ruled clean: its own `Target0/1/2 Image` authors the
  identical `Hexmedal_HD` crop (a third confirmation of `hd_medal_frame`'s
  numbers), but this build's own asset loader never puts `Hexmedal_HD` on
  that screen's sprite sheet at all, so the whole row draws nothing rather
  than something wrong - an honest absence, not a second bug.
- **`Campaign Selection`'s own medal fraction denominator is unexplained**:
  an RPCS3 frame reads `"0 / 87"` on the Fury side on a fresh profile, and
  this project's own parse of `DATA00`'s eight Fury grids totals 80 cells
  (`campaign_grids_ground_truth.rs`) - a 7-cell gap. This build draws only
  the earned numerator, no denominator, rather than guess at the 87.

## 2026-09-28: medals are per difficulty, with a per-difficulty icon shape - confirmed

The maintainer's own play observation (HD medals reflect three
difficulties, and each difficulty's own medal has a different icon shape)
is confirmed, not falsified. Full writeup:
`docs/ui/campaign-screens.md`'s "One shape per difficulty" section (the
texture/render evidence) and `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s
"The medal-evaluate function is found" section (the executable side).

**What landed, player-facing**: `Data\FE\Images\Hexmedal_HD.gtf`'s live
copy is `DATA04`'s `1024x768` atlas (three per-difficulty blocks), not
`DATA02`'s `1024x256` one archive precedence used to read for both
`Medal_{x}_{y}` (the grid overview badge) and `Target0/1/2 Medal` (the
Cell Selection target row) - two different widget-name shelvings of the
same file, both needed the fix (`oag_game::campaign::read_hd_texture`).
`oag_ui_screens::campaign::hd::hd_medal_frame` now crops the block matching a
difficulty, not always block 0. `oag_game::records::CampaignRecord` gained
`best_difficulty`/`last_difficulty` (backward compatible, `#[serde(default)]`);
`Store::record_campaign` tie-breaks a harder rung over a better medal at an
easier one - **chosen, not measured**, see that function's own doc for the
reasoning and its limits. `oag_tables::race_campaign::Difficulty` replaces
the bare `u8` difficulty-index convention across the tables/ui crates
(`Cell::skill_for_difficulty`/`targets_for_difficulty`/
`evaluate_medal_for_difficulty`, `CellSelection::difficulty`) - a
same-session refactor, not a separate pass, since it was small and the
codebase already has the identical duplicated-enum idiom for `Medal`
(`oag_tables::race_campaign::Medal` vs `oag_game::records::Medal`) to
follow for the new persisted field.

**What is measured versus chosen, stated once here so it does not drift
across the two doc pages**:

- **Measured**: the atlas is genuinely `3x` taller on the per-difficulty
  archive and holds three different icon *shapes*, not just three colours.
  `DATA02`'s only shape is the swirl.
- **Measured**: every medal-evaluate function this pass found in the
  executable (`0x001e25c0`, `0x001e1028`) reads one shared
  `GameState+0xdc`-held difficulty, not a per-cell stored one - so the
  original engine's own "current difficulty" concept is (at least for
  evaluation) global/browsed, not frozen per earned medal.
- **Measured, 2026-09-28, confidence 90**: which atlas block is which
  difficulty (`easy=plain hex`, `medium=cane`, `hard=swirl`). An RPCS3
  capture toggling `DifficultyButton` on `grid8_3_1` (`Fury`'s default
  `Cell Selection` cell, `Race`/Talon's Junction) through two full
  `NOVICE -> SKILLED -> ELITE` cycles read exactly this mapping off the
  `Target0/1/2 Medal` row, paired to each frame's own `AI DIFFICULTY
  (<rung>)` footer text - see `docs/ui/campaign-screens.md`'s "Which block
  is which difficulty" section and `docs/reverse-engineering/rpcs3-capture.md`'s
  "Cell Selection: `DifficultyButton` toggle" section for the full capture,
  recipe and confidence accounting. One boot, one cell, one mode - not
  reproduced on a second boot or an `Elimination`/`NitroBattle` cell.
- **Chosen, not measured**: that this project's own persistence stores a
  per-cell "earned at" difficulty at all, and that a harder rung should
  outright beat a better medal at an easier one. `SaveData_MigrateCellMedalsToHardElite`
  writes a per-cell record byte that is consistent with "the rung this
  cell's value should be judged against" (only touches modes whose value
  doesn't depend on AI skill), but no evaluator this pass found reads that
  byte back - see the Ghidra page's own account of what would settle this
  and was not attempted (the `Medal_%d_%d` widget-name strings have only
  `[DATA]` xrefs, the same reflection-bound shape `PI_Cell`'s own attribute
  table already has).

**Also found, not chased**: `0x0001c6d0` builds a 192-bit per-cell medal
mask by calling the evaluator on a whole cell list - plausibly a
profile-wide "has any medal" aggregate (unlock/stats), not the grid-badge
draw itself. Its own caller and the list it walks are unidentified.

**Confirmed by direct check, cheaply**: no other title has this. Pulse
never authors a per-difficulty target triple at all. Pure's own campaign
was not reachable (this build never gets past a Time Trial boot on Pure).
2048's `Pass`/`Elite` is a two-rung score bar, not a selectable-difficulty
medal law - see `docs/formats/race-campaign.md`'s own new section for the
citations.

**Left open, in priority order**:
1. ~~An RPCS3 capture toggling `DifficultyButton`...~~ **Done, 2026-09-28**:
   see the measured bullet above. Left by that pass: a second boot (this one
   ran on an existing, non-fresh save - moving it aside was refused by that
   session's own permission classifier as a write outside the repository,
   though the reading itself turned out to be save-independent, see the
   capture doc's own account) and an `Elimination`/`NitroBattle` cell, whose
   own target triple is separately named `Novice`/`Skilled`/`Elite` rather
   than `1st`/`2nd`/`3rd` - `scripts/rpcs3-drive.py browse`'s own `--nav` is
   dead once `--screen` is reached, so reaching a cell other than the
   default needs a different driver. **Same pass, unplanned**: the footer's
   own `DIFFICULTY (<rung>)` prompt reads `AI DIFFICULTY (<rung>)` on this
   cell (`Race`) but bare `DIFFICULTY (<rung>)` on `grid8_3_2` (`Eliminator`,
   the cell the 2026-09-25 section's own capture used) - a real,
   cross-checked-against-a-third-boot mismatch, not a crop artifact. **Both
   words are genuine disc strings**: `DATA04.PSARC`'s own `entries.xml`
   carries `RB_DIF="DIFFICULTY"` and `RB_AI_DIF="AI DIFFICULTY"` as two
   distinct idstrings (`psarc_grep`, direct read) - correcting the
   2026-09-25 section's own claim that the bare word was unauthored runtime
   text - though which idstring a given mode picks is chosen by the
   executable at draw time, not by an XML-authored redirect near
   `DifficultyButton`'s own widget. The same `entries.xml`'s
   `UPDATE_ANNOUNCEMENT` string plausibly explains the split: `AI
   DIFFICULTY` is the original `Single Race`/`Tournament`-only "adjust your
   opponents' skill" mechanic, bare `DIFFICULTY` the Update 1.20 addition
   giving every other mode its own selectable target-threshold rung. Bears
   directly on `pulse-cellsel`'s own runtime-prompt work; see
   `docs/ui/campaign-screens.md`'s "Which block is which difficulty" section
   for the full three-boot account and the correction's own full text. Not
   this thread's/lane's to fix - for the lead to route.
2. The saved-record struct's own field layout, precisely enough to settle
   whether `SaveData_MigrateCellMedalsToHardElite`'s own `+0x6c` write and
   `0x001e1138`/`0x001e1188`'s own `+8`/`+9` reads are the same field -
   would settle whether a per-cell "earned at" difficulty is real in the
   original at all, which is this project's whole storage model's own open
   assumption.
3. `0x0001c6d0`'s own caller and the list `FUN_0015e638` returns - would
   likely explain what the 192-bit mask is actually for.

## 2026-09-28, later the same day: the square-button prompt and the fresh-profile default, both closed

Routed here from the "Left open" item 1 above: HD Cell Selection's
`DifficultyButton` now draws computed `AI DIFFICULTY (<rung>)`/`DIFFICULTY
(<rung>)` text instead of the disc's static `"Change Difficulty"`, and this
project's own default rung changed from `Difficulty::Medium` to
`Difficulty::Easy` on HD (Pulse's own `Medium` default is untouched - it was
already correct, `Profile_SetDifficultyRC(profile, 1)`).

**What landed, player-facing**: `oag_ui_screens::campaign::hd::hd_difficulty_button_line`
(new) picks `RB_AI_DIF` for `Race`/`Head2Head`, `RB_DIF` for
`TimeTrial`/`Zone`/`Elimination`/`SpeedLap`/`Mode::Other` (`NitroBattle`/
`Detonator`), and returns `None` (disc's static string stays) for
`Tournament`/`CustomGrid`/`AiRace`. `oag_ui_screens::campaign::CellSelection` gained
`with_default_difficulty`, called with `Difficulty::Easy` from both
`oag_game`'s live session (`CampaignStage::open_cell_selection`, gated on
`is_hd()`) and the `--menu-page cell-select` still capture path
(`crate::capture::campaign_page`), so a still and a live session agree.

**Measured, confidence 90, on a genuinely fresh profile** - not merely
unread this time: `~/.config/rpcs3/dev_hdd0/home/00000001/savedata/` was
verified empty before boot (the lead moved the existing `BCES00664-AUTO-`
save aside for this pass specifically). `scripts/rpcs3-drive.py capture
--nav-shots` walks the same default path every earlier pass used (`Main
Menu` -> `Campaign Selection` -> `Grid Selection Fury` -> `Cell Selection`,
landing on `grid8_3_1`) and photographs the *settled* arrival frame - no
`DifficultyButton` press, no comb-artifact risk the way `browse`'s own
unpressed frame carried on every earlier attempt. That frame reads `AI
DIFFICULTY (NOVICE)` and `TARGET (NOVICE)` both - settling the fresh-profile
default (`Easy`) and independently corroborating the `Race`-mode
`RB_AI_DIF` reading the 2026-09-28 "Which block is which difficulty"
section above already had at the same confidence from a *different*
(non-fresh) boot. See `docs/reverse-engineering/rpcs3-capture.md`'s same
section, updated, and `docs/ui/campaign-screens.md`.

**What the Ghidra side settles and does not**: `CellSelection_UpdateDifficultyButton_q`
(`0x0021db80`, found through a real `bl` xref to the newly-named
`Profile_SetDifficultyRC`/`Profile_GetDifficultyRC` pair) confirms the mode
set where `DifficultyButton` cycles at all on `Cell Selection` -
`{Race, TimeTrial, Zone, Elimination, Head2Head, SpeedLap, 0xd, 0xe}`, **not**
`Tournament` - which is what `hd_difficulty_button_line`'s own `None` cases
are grounded in. It does **not** settle which idstring gets chosen: that
call site was hunted for and not found this pass, `RB_AI_DIF`/`RB_DIF`'s own
choice therefore extended by capture + `UPDATE_ANNOUNCEMENT` reasoning
alone. **A real, binary-specific Ghidra trap was hit and documented**:
`get_xrefs_to` on this ABI (`PowerPC:BE:64:A2ALT-32addr`) can resolve to a
wholly unrelated function when a TOC-relative load's real per-function `r2`
differs from whatever base the reference analyzer assumed - see
`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s "The TOC-xref trap"
section for the full account and what to do instead.

**Still open**: the `RB_AI_DIF`/`RB_DIF` call site itself (see above);
`Profile_GetDifficultyRC`'s own fallback global's initializer (would be a
second, independent line of evidence for the default rung, not read this
pass); and everything `Tournament`-shaped this rung's own gate excludes -
whether `Tournament` cells author a working `DifficultyButton` at all, on
some other screen this pass never read.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-14. `oag_tables::race_campaign` reads all 32 `plugins/grids/grid_*.xml` across HD's four archives, additively: flat-schema grids parse unchanged, per-difficulty grids (Fury's `grid8-15`, `DATA04`/`06`'s own `grid0-7`) land in `Cell::difficulty_targets`/`nitro_elimination_targets`, `Mode::Other` holds `NitroBattle`/`Detonator`. 157 cells over 16 grids under the archives' own precedence, ground-truthed against the EU disc (`crates/hd/tests/campaign_grids_ground_truth.rs`, [race-campaign.md](../../docs/formats/race-campaign.md)). `grid_04.xml`'s `<Values>` tag is malformed on disc in all three copies and reads zero cells - documented, not patched. Open: which target triple the medal law reads per mode (an HD-executable question), whether the real game tolerates `grid_04`'s tag, base-HD pressing unmeasured. Next: draw HD's `Grid Selection` off `Data\Plugins\Frontend\Gui\CellMode_Definition.xml` on `DATA02` - the thread inventories its widgets and textures

## 2026-09-29: `Team Selection` is read, drawn and walked on HD

The original shows a ship screen between `Cell Selection` and the race
(confidence 95: `TTY.log`'s cold-boot sequence, `screen-Team-Selection.png`
on three RPCS3 boots, both `CellMode` copies' `Cell Mode Redirect Team`, and
`DATA00`'s live skin including `DATA06`'s `Team_Selection_Definition.xml`).
It now opens on HD from `Cell Selection` and from the RACE page's START:
title, headings, team logo, per-model stat bars, Confirm/Back legend,
left/right team, up/down livery, pointer, Back to the same cell and rung,
and the picked livery reaches the race. Full writeup:
`docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: `Team Selection`,
2026-09-29"; code in `oag_ui_screens::picker::hd`.

**Left open, in priority order**:
1. **The 3-D ship in the `SHIP MODEL` frame.** The `ShipModel` pose is read
   (`oag_ui_screens::picker::hd::ShipModel`); HD ships no `ship_FE.vex` (its own
   per-team `screen.xml` names one), so the preview needs the race's
   `ship.vex` + `ship.rcsmodel` path. An afternoon if the race loader's
   model build can be called with an archive and an entry name alone.
2. **The `HexSelection` grid** - the per-team `FE\thumb0..3.gtf` look like
   its ship icons; the hex art itself is the widget class's own, unread.
3. ~~**Which direction moves what**~~ - measured 2026-09-29 (confidence
   90): left/right the team, up/down the model row, and the row is kept
   across a team step. Same walk settled Track Creation's up/down = direction.
4. **Pulse's own `Team Selection` Back** still reopens `Grid Selection`;
   whether Pulse's original returns to `Cell Selection` the way HD's
   `goto`-less `TeamRedirectBack` does was not checked.
5. **HD's track screen** (`Track_Selection_Definition.xml`, `DATA06`) was
   read and wired on 2026-09-29 (next section); the RACE page's TEAM,
   VARIANT and TRACK rows are dropped for it.
6. ~~**The default livery**~~ - a fresh profile opens on `concept1` on both
   routes (measured, confidence 90) and this build does now
   (`Race::opening_variant`). **Still open**: what the original remembers on
   a profile that already holds a save (it opened on `normal`), and the
   fresh profile's team - Feisar there, `settings.race.team` (`assegai`)
   here.

## 2026-09-29: HD's `Track Creation` (track screen) read, drawn and walked

The circuit screen sits on the RACEBOX path only (`Single Player` ->
`Track Creation` -> `Team Selection` -> race; the campaign skips it, confidence
90). It draws the title, headings, emblem frame, each circuit's own
`TrackSelectEmblem_Fury.gtf`, the name, `CIRCUIT LENGTH`/`RACE DISTANCE`, the
RECORDS table (`---` cells) and, on a reverse circuit, the authored
`ReverseIcon`s. Left/right wrap along twelve circuits, up/down switch
direction; pointer and Back work; Back from `Team Selection` returns here.
Writeup: `docs/ui/campaign-screens.md`, "Wipeout HD/Fury: `Track Creation`,
2026-09-29"; code in `oag_ui_screens::picker::hd::track`, `Picker::with_rows`,
`oag_raceplay::catalogue::direction_rows`.

**Corrections it made**: `right` most likely wraps at twelve (confidence 80:
press 12 is Vineta K again, cursor on the same top hex row, no reverse glyph in
13-24), with the two rows as directions chosen, not the 24-entry list; `Track Creation` is not in `racebox_definition.xml`, which only names it.

## Open

1. **The three things the frame shows that this build does not**: the circuit
   wireframe (`TrackModel`; HD ships no `FE\forward.vex`, so it must come from
   the racing circuit's own geometry), the fly-by (`preview.bik`, Bink), and the
   `TrackHexSelection` grid (per-circuit emblems in dim hexes, selected red with
   a white outline; cell pitch unmeasured). The hex grid is the largest empty
   area; measuring its pitch off `track-carousel/*.png` and drawing the
   authored `Hexagon_HD*.gtf` cells is about an afternoon.
2. ~~**Up/down = direction** and the **reverse row's order** are chosen.~~ Measured 2026-09-29 (confidence 90, one walk).
3. **RECORDS cells**: no per-circuit record in (name, team, time) shape.
4. **The `Squares` page dots** (12, first red) are code-filled.
5. **Zone**: Zone skips the ship screen here as on Pulse, but HD's
   `TrackRedirect` sends it to `Team Selection` like every other mode; unmeasured
   whether the original does too.
