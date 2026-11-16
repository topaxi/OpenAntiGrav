# Wipeout 2048's race-ending pages

**Status: two of the five pages draw, XP block included**, off the disc's own
`NEWGUI/EndRace_Definition.xml`, after a real race through the front-end flow.
`RaceSummary` and `ObjectiveSummary` are read and drawn; `Results`, `Podium`,
`Badges` and `ParadeLaps` are named gaps (below). This is the picture half; the
decompiled law behind it is
[`vita-2048-eu-v104/endrace-summary.md`](../ghidra/functions/vita-2048-eu-v104/endrace-summary.md),
and Pulse's and HD's own screens are [endrace-screens.md](endrace-screens.md)
and [`hd-endrace-screens.md`](../formats/hd-endrace-screens.md).

Implemented in [`oag_ui_screens::endrace::touch`](../../crates/ui-screens/src/endrace/touch.rs)
(model, draw list, pointer), [`oag_game::endrace::touch`](../../crates/game/src/endrace/touch.rs)
(the disc read and the wording of a finished race),
[`race_stage::endrace_touch`](../../crates/game/src/main/race_stage/endrace_touch.rs)
(the runtime and its flow) and
[`session::endrace_touch`](../../crates/game/src/main/session/endrace_touch.rs)
(building it, driving it, retry and exit).

## Which file, and which copy

2048 ships two end-race files, and they are not alternatives:

| File | Screens | Used for |
| --- | --- | --- |
| `NEWGUI/EndRace_Definition.xml` | `SkipRaceConfirm`, `EndRaceTipsScreen`, `ParadeLaps`, `EndRace` (holding `RaceSummary`, `Podium`, `Results`, `Badges`, `ObjectiveSummary`, `NearWait`, `NearUpload`) | the live race ending - the eboot builds `RaceSummary`/`ObjectiveSummary` from it |
| `NEWGUI/LegacyEndRace_Definition.xml` | `EndRace Menu`, `EndRace Results` (HD's names) | not reached by the end-of-race chain; `Skin.xml` loads it, nothing in the chain redirects to it |

**Copy lineage:** the base `data.psarc` copy is 22,076 bytes; `patch-v104/data1.psarc`
carries a 22,098-byte one that differs by a single attribute
(`name="continueButton"` on the tips screen's tick). The patch copy is the one
served (2026-10-07: the v1.04 archives are mounted ahead of the base), pinned
by `the_patch_copy_of_the_file_is_the_one_served`, and the difference touches no
widget this build draws. `Legacy` has a third copy in
`data2.psarc` (14,267 bytes against the base's 12,382 and `data1`'s 12,593).

`oag_2048::TITLE.front_end.endrace_entry` names the file and `endrace_style` its
dialect ([`EndRaceDialect::Touch`](../../crates/title/src/endrace.rs)), which is
what `oag_game::endrace` dispatches on in place of a comparison against Wipeout
HD's name; Pulse and HD carry `Pulse` and `Field`, Pure and Omega `None`.

## What draws

The `EndRace` shell (a translucent top bar, the 600x347 `endrace_bg` panel and the
tile strip) under `RaceSummary`, then `ObjectiveSummary` five seconds later.

| Widget | Draws | Source |
| --- | --- | --- |
| `RaceSummaryTitle` | the mode's title (`FE_ENDRACE_SUMMARY`, `FE_END_TIMETRIAL_SUMMARY`, ...) | measured, `FUN_810cf5fe` |
| `MessageBox`, `TotalXPBox` | a bar in the verdict's colour: `Pass2048`, `ElitePass2048`, `0xffcd0102` (fail), `0xff525e84` (no verdict) | measured |
| `Message` | `ER_CONGRAT`, or `ER_END_TOUR_7` on a fail; empty with no verdict | measured |
| `Objective` | the chain's first objective, worded as the event card words it | measured |
| `ResultBox` + `RaceResultText` / `SpeedResultText` | the result line, in a box (a speed lap: bare) | box and shape measured; **the wording is chosen** |
| `PostRaceMedal` + `PostRaceMedalText` | the medal cell (`128`/`256`/`0` texels in `Post_Race_Medals`) and `FE_PASS`/`FE_ELITE_PASS`/`FE_FAIL` | measured |
| `Objective1`, `Pass0`/`ElitePass0`/`Fail0` | the one row of the chain earned and its tick or cross | measured, `FUN_810d0c46` |
| `PassBonusXPBox` + `PassBonusXP` | the completion award banked (`PASS_BONUS_XP`, a number), when there is one | measured box and format; shown when the finish banked a completion award, which is on improvement only, as the original does ([`xp-award.md`](../ghidra/functions/vita-2048-eu-v104/xp-award.md), "Completion pays on improvement") |
| `RaceXPBox` + `RaceXP` (`SpeedRaceXPBox` + `SpeedRaceXP` on a speed lap) | `FE_EVENT_XP` and the race's tally less the bonus, counted up from `0` at `200`/s | measured, `EndRaceSummary_UpdateXpBlock` |
| `TotalXP` (both pages) | `RESULTS_EVENT_TOTAL` with the tally while it counts, then `RESULTS_EVENT_TOTAL` and `TOTAL_NUMBER_XP` (the profile's total) every 3 s, `RESULTS_NEW_RANK` for 3 s on a rank-up; the disc's English | measured |
| `RankItem` + `RankText` + `TotalXPBg`/`TotalXPFill` | the rank of the counting total (title's own `XPRanks.xml`), the shield flipping on a rank-up, the bar `trunc(48 * (total - thr(rank)) / (thr(rank + 1) - thr(rank)))` px | measured; the flip's `cos` is confidence 65 |
| `ResultBox` | shortened to end 4 px above the topmost XP box when the bonus shows | measured |
| `RestartRaceTouchButton`, `QuitTouchButton` / `QuitTouchButtonPass` | a `Blue2048` box and its icon, the tick once the event is passed | measured; the box and cursor ring are the touch front end's own idiom (`oag_ui::frontend::cursor_ring`) |

**Fills fold their own offset.** The disc authors every box as an `Image` with
its own `OffsetX`/`OffsetY` and a colour but no `src`;
`oag_ui::screen::Screens::from_xml_folding_fill_offsets` adds that offset in
where the default reader does not. The default is kept for Pulse, whose
`tablebg{n}` rows are placed by their own table
(`oag_ui_screens::endrace::table`), and moving them would move a measured picture.

## What does not draw, by name

- **The XP block of a race that armed no tally** (a `--race` launch, an event with no
  `XPAwards.xml`): hidden, as are the XML's development placeholders for it
  (`TOTAL_4325_XP_TEST`, `99`; a code-filled widget never shows its authored string,
  `placeholder_text_never_draws`).
- **The tap that snaps the count** to its end is any page turn here (**chosen**); the original's
  tap on the panel also plays `newGUI_confirm`. What arms the count at the start (`+0xe8`) is not
  read, so it starts armed (**chosen**).
- **`Results`' table** (`<RaceResults>`, native code not decompiled), **`Podium`**
  and **`Badges`** (online), **`ParadeLaps`** (a free camera this build does not
  have, so its tile is not drawn), **`NearWait`/`NearUpload`** (online), and
  **`EndRaceTipsScreen`**/`SkipRaceConfirm` (a skip-event prompt and a ship tip,
  neither reached by this chain).
- The auto-leave timer `FUN_810cd0b8` runs once a flow has more than one page
  (`10.0` seconds, then `KillGameVita`) is **not implemented**; unverified.

## Flow and input

`Session::build_endrace` hands a Touch-dialect title to `build_endrace_touch`
once per finished race. The runtime holds the page (`RaceSummary`, and
`ObjectiveSummary` when the event authors an objective; an event with none keeps
the one page), turning on its own after `5.0` s - measured - and by Up/Down
(**chosen**) or a tap on the panel (**chosen**). Left/Right moves the cursor
between the two tiles, Cross or Start presses the one under it (**chosen**), and a
click on a tile presses it. A confirm in the first second of the finish does
nothing, so the throttle held across the line does not dismiss a page nobody has
seen (**chosen**).

- **Exit** (`QuitTouchButton`, or the tick once passed) leaves through
  `Session::leave_finished_race`: this build's own menus, the same destination
  the built-in results table's dismissal gives. **Open:** the original returns
  to the campaign map; this build keeps no way back into the map once the race
  has launched, and rebuilding the front end mid-session is its own lane.
- **Restart** hands the event's name back to the launcher
  (`Session::pending_event`) and goes through `finish_launch`, as a tap on the
  map did.

## Verification

**No reference capture of 2048's post-race exists** (`data/reference/2048-frontend/`
has none) and no Vita emulator was run for this lane: the colours, layout and chain
come from the XML and the decompile, not from a comparison against an original frame.

- Disc-backed: [`vita_2048_endrace_ground_truth.rs`](../../crates/game/tests/vita_2048_endrace_ground_truth.rs)
  (`just test-data`) reads the real file, checks every texture decodes into the
  sheet, races `2048 - Event 1` under the autopilot and words the real result,
  and checks the fail and elite paths against the disc's own strings.
- Stills: `oag-game <2048 source> --menu-page endrace-summary --event "2048 - Event 1"`
  races the event and draws what it came to (about 55 s in a debug build);
  `endrace-summary@SECONDS+PRIOR` raises the race's tally into a profile that already held
  `PRIOR` XP and draws the page `SECONDS` into its count (`@0.5` mid rank flip, `@2` the new-rank
  line, `@7` the event total, `@30` the profile total); `endrace-summary-pass`/`-elite`/`-fail`
  and `endrace-objectives-*` draw
  **chosen** facts (a third place against "finish at least 3rd") so each tone
  can be looked at without racing to it.
- Live: one windowed run in a private Xvfb with software Vulkan, walked
  `GameModeChoice` -> the event card -> launch, finished under `--autopilot`, and
  put the pages over the frozen race; Exit left for the menus. The window
  (1440x816) was larger than the Xvfb screen (1280x720), so the tile strip is
  clipped in those frames; the `--menu-page` stills show it whole.

## Omega

**Checked, applies, not wired; its own page differs.** Omega's `data09.psarc` ships
`data/plugins/frontend/gui/vita/vita_EndRace_Definition.xml`, **byte-identical**
(22,098 bytes, `cmp`) to 2048's v1.04 `NEWGUI/EndRace_Definition.xml`, next to
its own HD-lineage `endrace_definition.xml` (49,368 bytes) and
`campaign2048_definition.xml`; the base `data00.psarc` carries the same
`endrace_bg` and `Rank_Icon_Endrace` textures (as `.gnf`). Neither Omega's HD
front-end XML nor `campaign2048_definition.xml` names the `vita/` file, so
nothing in Omega's own front end reaches it. Omega keeps `endrace_entry: None`
and `endrace_style: None`: its HD-lineage screens stay the read-and-not-wired
item in [omega-status.md](../formats/omega-status.md), and racing is out of
scope. **The XP block, checked against Omega's own page: differs.** Omega's HD-lineage
`endrace_definition.xml` (`EndRace Results`, `data08`/`data09`, 49,368 bytes) authors the same widget
vocabulary in HD's 1920 layout: `eventxp_block` (`PassBonusXPText`/`PassBonusXP`,
`RaceXPText`/`RaceXP`, `RaceTotalXPText`/`RaceTotalXPNum`) and `totalxp_block` (`CO_TOTAL_XP`,
`TotalXP`, `TotalXPNum`, `RankItem` at 96 px with `RankText`, `TotalXPBg`, `TotalXPFill`) at
`(1180, 368)`, plus a `RankingBlock`. It is a different layout (three lines, not two) of the same
numbers, built by `FUN_0149f910`, which this lane did not read, so the count law above is not
assumed for it; the `vita/vita_EndRace_Definition.xml` copy is 2048's page and would carry this
block unchanged if Omega drew it. If Omega races its 2048 events, these pages are the ones its own data
brings for them; wiring would be a `Touch` style on Omega and a loader for the
`.gnf` spellings, and is left open.
