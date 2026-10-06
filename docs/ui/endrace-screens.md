# The three EndRace screens: `Results`, `Rewards`, `Menu`

**Status: all three draw**, off the disc's own `EndRace_Definition.xml`, for
Pulse. This is the *picture* half - what this build actually draws and why;
[`docs/formats/endrace-screens.md`](../formats/endrace-screens.md) is the
reading half (every widget, its authored position, its runtime source) and
[`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`](../ghidra/functions/psp-pulse-usa/endrace-screens.md)
is the decompiled law behind each value; this page does not repeat either.
Implemented in [`oag_ui_screens::endrace`](../../crates/ui-screens/src/endrace.rs) (model,
layout, draw, pointer), [`oag_game::endrace`](../../crates/game/src/endrace.rs)
(the disc read, mirroring [`oag_game::campaign`](../../crates/game/src/campaign.rs)),
[`crate::race_stage::endrace`](../../crates/game/src/main/race_stage/endrace.rs)
(the runtime a finished race holds open) and
[`crate::main::session::endrace`](../../crates/game/src/main/session/endrace.rs)
(the flow - advancing screens, `RETURN TO GRID`/`RACE AGAIN`/`VIEW RESULTS AGAIN`).
This build's own results table
([`oag_game::scoreboard`](../../crates/game/src/scoreboard.rs)) stays as the
fallback for a title with no `EndRace_Definition.xml`, or whose read fails -
see that crate's own module doc. Pure was not checked for this file this
pass; it is not assumed to carry it. **Wipeout HD/Fury's own copy is now
read and drawn** - see ["Wipeout HD/Fury: `Results`/`Menu`, off a completely
different file"](#wipeout-hdfury-results-menu-off-a-completely-different-file)
below.

## Why this draws inside `Stage::Race`, not `Stage::Menu`

The reference frames (`results-01..03.png`) show the three screens over a
still-visible, darkened race scene - the picture `crate::main::menu_stage`'s
own `frozen_race` compositing already gives a *paused* race. This build does
not reuse that mechanism: a finished race's own scene is already left exactly
as the finishing tick left it (`RaceStage::draw_hud`'s pre-existing doc, unchanged
by this work) - the world stopped being ticked (it keeps being ticked since 2026-10-01 when the player
crossed the line, as the original does; see [after-the-finish.md](../gameplay/after-the-finish.md)), and
`oag_game::scoreboard::Overlay` already draws its own results table over that
frozen frame this same way. `crate::race_stage::endrace::EndRaceRuntime` reuses
that same mechanism rather than transitioning to `Stage::Menu`: it is drawn from
inside `RaceStage::draw_hud`, over the race's own last frame, with its own FE
sheet/skin/frame built once when the race finishes
(`Session::build_endrace`) rather than borrowed from `MenuStage`. **Chosen**,
not the only shape this could have taken - see that module's own doc for the
trade against a `Stage::Menu`-based design.

## `Race End Photo`, before the first panel (2026-10-02)

A Pulse race that ended on the line does not open `EndRace Results` at once. The original sits in `Race End Photo` first
(measured and written up in [after-the-finish.md](../gameplay/after-the-finish.md)); ours does the same, in
[`oag_ui_screens::endrace::photo`](../../crates/ui-screens/src/endrace/photo.rs) (timing and draw list) and
[`race_stage::endrace_flow`](../../crates/game/src/main/race_stage/endrace_flow.rs) (which screen is on top):

| Part | What ours does | Source |
| --- | --- | --- |
| Screen | `Race End Photo` in `Data\Plugins\PI001\GUI\InGame_Definition.xml` (hash `31b50f2e`), read by `oag_game::endrace::load` next to the three panels; a file that will not read logs once and the panels come up at once | the disc |
| Clean view | 61 ticks from the finish: nothing drawn, the HUD already gone | measured, four captures + one per-frame run |
| Legend | `ProceedMessage` (`FE_PRESS_TO_CONT`, y 242) and `PhotoMessage` (`ER_PRESS_SELECT_PHOTO`, y 212): `Stats` font, left-aligned at x 20, white. Strings come from the disc's language file, so `check-strings` has nothing to add | the disc |
| Fade | straight ramp, 42 ticks, from tick 61 | measured (a photograph every second frame) |
| Leaving | X, Start or a click, from tick 61; the press is spent before that | the clock is measured; accepting a press in the first second is moot, the original is not on the state yet |
| SELECT | nothing (`Session::frame` spends the press instead of cycling the camera view) | photo mode is not built |
| Pointer | a click anywhere, like the other two confirm-only screens | chosen |

A finish by the line and a **Single Race wreck** get it (the wreck on the maintainer's 2026-10-02 decision "hold, then results", the original having no
such state there). An Eliminator target and a Zone run keep the panels at once, and Wipeout HD/Fury is unchanged (its equivalent was not read).

## What each screen draws, and what it leaves blank

### `EndRace Results`

| Widget | Draws | Why |
| --- | --- | --- |
| `BigTopText` (`ER_RES`) | Yes | direct idstring, resolved generically |
| `Line1` (headline) | Yes, for `TimeTrial`/`SpeedLap`/`SingleRace` | `SingleRace` uses the finishing-position idstring (`ER_1STP`..`ER_8THP`). `Zone` and `Eliminator` have tables of their own, below; `Headline::Unresolved` remains only for HD's grid, whose populate for them is unread |
| `lap0.0`/`lap0.1` (header) | Yes | `RC_LAP`/`PRO_TIME` |
| `lap0.2` (header) | **No text, but the header icon draws** | the cell is blanked in the original too: `boostimg` is the header - see the `boostimg` row |
| `lap{n}.0`/`lap{n}.1` | Yes, up to [`oag_race::MAX_RECORDED_LAPS`] (4) | off `Standing::lap_splits`, which is itself capped at 4 - see below |
| `lap{n}.2` | **Yes, 2026-09-30**: pads entered on that lap | `crate::race::RunStats::boosts_by_lap`, counted on the edge `Ship_ApplySpeedupPad` bumps the original's own counter on (a *new* pad, the human craft only) - the writer was found by a live write watchpoint. Blank for a lap with no count |
| totals row (`PRO_STATS_TOT` + `tablebg{n+1}`) | Yes | `Results::total_ticks`, the player's own finish tick; the third cell is the sum of the laps' pad counts (`+0x1148`) |
| `tablehighlight` | Yes, on the totals row; hidden for Speed Lap | **measured 2026-09-30**: `PopulateLapTable` ends on `y = laps * 0x14 + 0x5d`, i.e. `93 + 20 * laps`, and hides it in mode 10, which has no totals row. It was `92 + 20 * laps`, chosen, before |
| `tablebg{n}` rows | **Only the rows in use**, 2026-09-30 | `ResetTable` hides all eight and `PopulateLapTable` shows one per lap and the totals row; this build drew all eight on every table until the bit-`0x4` correction (`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`). A race with no completed lap hides the whole `table` group |
| `topbarcenter` | Yes | shown by `ResetTable` |
| `perfectlap{n}` | **No** | the flag's direction is settled (nonzero shows the icon, 2026-09-30) but this build keeps no per-lap "perfect" flag and does not know what makes a lap perfect, so none is ever set. **Its own `idstring="MSC_PL"` text overlay used to leak through regardless** - found and fixed 2026-09-28: the overlay is a nested, unnamed `<Text>` inside the `<Image name="perfectlap{n}">` element (`docs/formats/endrace-screens.md`), so the by-`name` skip this row already gave the image half never caught the text half; `results_draw_list` now also skips any text whose `idstring` is `MSC_PL`. Visible as a faint "TP" (this source's own French) past each row in a pre-fix capture. |
| `boostimg` | **Yes, 2026-09-30** | the third column's header icon, shown by `EndRaceResults_PopulateLapTable` and by no other populate. The earlier "hidden unconditionally" reading had `\|= 4` and `&= ~4` backwards; the one capture (`results-01.png`) always agreed with the corrected one. The column under it counts the pads entered on each lap, and the totals cell sums them |
| `ContinueButton`/`ControlTextConfirm` | Yes | direct idstrings |

[`oag_race::MAX_RECORDED_LAPS`]: ../../crates/race/src/state.rs

**A race longer than four laps keeps only its own last four laps' worth of
splits** - `oag_race::Standing::lap_splits` is a fixed `[Option<u32>; 4]`,
sized to what HD/Fury's own `HUD_lap_times.xml` authors four rows for (that
constant's own doc). This project's own `EndRace Results` table can show at
most four real rows for that reason, regardless of how many the disc's own
XML authors slots for (eight). Not this pass's call to widen - see that
constant's doc for why.

### `EndRace Results`, on a Tournament leg: two cycling pages, not the per-lap table

**Status: draws, 2026-09-28.** A Tournament leg does not show the per-lap
table above at all - `EndRaceResults_OnEnter`'s own `case 4: case 0x10:`
(`docs/ghidra/functions/psp-pulse-usa/tournament.md`) replaces it with a
per-craft table that cycles, every three seconds, between this leg's own
placings (`ER_RACE_STAN`) and the running tournament standings
(`ER_TOUR_STAN`) - two genuinely different pages, not one table shown two
ways. This build reuses the same three shared table widgets
(`lap0.0`..`lap8.2`, `tablehighlight`) the per-lap table above draws,
through a new model, [`oag_ui_screens::endrace::TournamentResults`]
(`oag_ui_screens::endrace::tournament_results_draw_list`), built in
[`crate::race_stage::endrace::tournament_results`](../../crates/game/src/main/race_stage/endrace.rs)
off this build's own [`oag_race::tournament`]/[`crate::race::tournament::Progress`]
(established, not re-derived by this pass) and the leg's own grid roster
(`crate::race::slot_teams`, recomputed at EndRace-build time - see that
function's own doc for why it is a second call rather than a threaded-through
field).

[`oag_ui_screens::endrace::TournamentResults`]: ../../crates/ui-screens/src/endrace.rs
[`oag_race::tournament`]: ../../crates/race/src/tournament.rs

| Widget | Draws | Why |
| --- | --- | --- |
| `BigTopText` | Yes | `ER_END_TOUR` on the last leg, otherwise `"%s %d/%d"` of `ER_RES` and the leg counter - `Progress::leg_number`/`leg_count` |
| `Line1` | Yes | `ER_RACE_STAN`/`ER_TOUR_STAN`, whichever page is current |
| `lap0.0`/`lap0.1`/`lap0.2` (header) | Yes | `PRO_POS`/`ER_TEAM`/`ER_POINTS` |
| `lap{n}.0` | Yes, one row per grid slot | the row's own 1-based index - `EndRaceResults_PopulateTournamentTable` does not sort, so this is simply which row a craft's own data landed on |
| `lap{n}.1` | Yes, when a team is known | the craft's own team, **resolved to its display name** (`AG_Systems` -> `AG Systems`) through the string table at draw time, 2026-09-30 - the original's `localise(craft+0x798)`. `None` (a launch that named no team) draws the cell absent, never a guessed `"SLOT n"` |
| `lap{n}.2` | Yes | this leg's own points (leg page) or the running total (standings page) - `oag_race::tournament::points_for_finish`/`Progress::points` |
| `tablehighlight` | Yes, on the player's own row | `93 + 20 * (row - 1)` - the same step every populate uses, since 2026-09-30 (the lap table's own was `92 + ...`, chosen, until then) |
| `perfectlap{n}`/`boostimg` | **No** | per-lap concepts, no reading on a per-craft table |
| `ContinueButton`/`ControlTextConfirm` | Yes | direct idstrings, unchanged |

**The leg page's own row order is this leg's finish order**
(`oag_game::scoreboard::Board::rows`, already ordered by place), **not**
`oag_race::tournament::Standings`' own rank - `tournament.md`'s own "What is
not determined" names the behavioural evidence for this (every capture's
points column reads strictly decreasing top-to-bottom) without a traced
writer for the original's own equivalent array. The standings page's own
order **is** `Progress`' rank (cumulative points, descending, slot-order
tie-break) - the two pages are ordered by two different things, matching
what each is showing.

**A live capture now exists (2026-09-29)** - layout, columns, points and page cycle match, see `tournament.md`'s "Live capture of the drawn standings table"; the display-name and font differences listed there are open. The rest of this paragraph predates it: **No live capture of this page exists yet** - `--menu-page
endrace-results-tournament-leg`/`endrace-results-tournament-standings` draws
it off a synthetic four-team field (`crates/game/src/capture/endrace_page.rs`),
since a real tournament leg run to a finish is the same multi-minute
autopilot cost `tournament.md`'s own "Live verification" section names, not
attempted again this pass. Kept under `data/scratch/pulse-tourney/shots/`
(gitignored - game content).

### `EndRace Results`, on an Eliminator race and a Zone run: their own tables

**Status: draws, 2026-09-30.** `EndRaceResults_OnEnter` gives Eliminator (mode 8) and
Zone (mode 6) tables of their own, filled by `EndRaceResults_PopulateEliminationTable`
and `EndRaceResults_PopulateZoneTable`; the reading is on the [ghidra
page](../ghidra/functions/psp-pulse-usa/endrace-screens.md#the-variant-populates-2026-09-30).
Both go through the walk every table shares
([`oag_ui_screens::endrace::table`](../../crates/ui-screens/src/endrace/table.rs)), which shows only
the rows a mode fills. Models and draws are in
[`oag_ui_screens::endrace::modes`](../../crates/ui-screens/src/endrace/modes.rs); the builders are
`crate::race_stage::endrace::{elimination_results, zone_results}`.

**Eliminator - confirmed against a live PPSSPP frame** (`--menu-page
endrace-results-eliminator` draws the same eight records the original ended a race
on):

| Widget | Draws | Why |
| --- | --- | --- |
| `Line1` | Yes | `"%s %s"` of `ER_ELIM_COM` and the place's ordinal (`Eliminator complete -  8th place`, two spaces and all); blank outside places 1-8 |
| header | Yes | `ER_DEATHS` / `ER_TEAM` / `IG_HUD_KILLS`, at the authored columns 140 / 220 / 320 |
| rows | Yes, one per craft | **deaths, team, kills** - deaths sit in the *first* column, which is odd enough that the live frame is what settled it |
| order | kills descending, deaths ascending, the player first on a full tie | `Race_BuildEndRaceResult`'s sort, reproduced by `EliminationResults::new`. **It ranks the screen, not the race**: the simulation's own finishing place is neither read nor changed |
| team cell | display name | folder id through the string table, as in the Tournament table |
| `tablehighlight` | Yes, on the player's row | `93 + 20 * (row - 1)` |
| `ER_DNF` | **No** | printed in place of both numbers when a record's `+0x140` is `-1`; nothing writes that word on this path and it read `0` on all eight live records |
| `boostimg`, `perfectlap{n}` | No | the lap table's |

**Zone - decompile only** (no Zone frame is reachable; `--menu-page
endrace-results-zone` draws **chosen** numbers):

| Row | Draws | Why |
| --- | --- | --- |
| `Line1` | `ER_ZONE_COM` | direct |
| header row and `topbarcenter` | **Hidden**, as in the original | Zone has no header |
| 1 `ER_ZONE_CLEAR` | label and `RaceState::zone` | `+0x1a10`, the zone number |
| 2 `ER_PERF_ZONE` | label and a running count | `RunStats::perfect_zones`, ticked on the `perfect_zone` edge - on the view side of `Race`, so no hash moves |
| 3 `ER_LAPSC` | label, **value blank** | what steps `+0x1a14` (`craft+0x911` bit 0) is not recovered |
| 4 `MSC_DATA_PLAP` | label, **value blank** | what steps `+0x1a16` (`craft+0x860 & 0x200000`, also worth 2000 points) is not recovered |
| 5 `ER_TOP_SPEED` | label and `"<n> KM/H"` | `RunStats`' running maximum of the player's `\|dot(velocity, forward)\|`, kept as `Zone_Update` keeps it (`u16(speed * 100)`, printed `* 3600 / 100000`). **Chosen, not measured**: sampled per tick rather than on the original's frame, and the maximum is taken over the whole run, not only while the mode was racing |
| 6 `ER_ZONE_SCORE` | label and `RaceState::score` | `+0x1a1c`. This build's own score omits the 2000-point row-4 bonus, so it can read low |
| `tablehighlight` | No | `OnEnter` hides it after the populate |

**Live, 2026-09-30** (Xvfb `:92`, software Vulkan, `pulse-psp-usa.chd`, isolated
`XDG_CONFIG_HOME`, keyboard and pointer through the real front end - `RACEBOX`, a mode
value, `START`, track, team). **Zone**: a real run steered into a wall - `Zone
session complete!`, `Total zones cleared: 26`, `Perfect zones: 16`, both lap rows
labelled and blank, `Top speed: 847 KM/H`, `Zone score: 24014`, no header bar - then
the flow carried on to `EndRace Menu`. **Eliminator**: the field is 8 craft and the
kill count is 10, which the AI did not reach in 8 game-minutes with the player
parked (the original's did with 5 in 85 s), so the finish was reached by a
**temporary local change of `ELIMINATOR_KILL_TARGET_DEFAULT` to 0**, reverted and
not committed. It ended the race on tick 0 and showed the table over the real race
scene: `Eliminator complete -  1st place` (an all-zero field, the player first on the
tie), `Deaths: | Team | Kills`, eight rows with **display names** (`AG Systems`, `EG-X`,
`Goteki 45`), the player's row highlighted; `Confirm` reached `EndRace Menu`, and a
**pointer click on `VIEW RESULTS AGAIN`** brought the table back. The numbers in a
populated Eliminator table are therefore covered by the unit tests and the
`--menu-page endrace-results-eliminator` still, laid against the original's frame, not
by a live race with kills in it. The original's frame for comparison:
`data/scratch/pulse-endrace-modes/shots/original-eliminator-results.png` (gitignored).

Both tables' cells draw in the upper-case front-end font, as the lap table's already do,
where the frames show the mixed-case default face - the shared-table font gap
`tournament.md` records, not a fault of these.

### `EndRace Rewards`

| Widget | Draws | Why |
| --- | --- | --- |
| header (`ER_REWARD`) | Yes | direct idstring |
| `RewardLine1` (medal-award phrase) | Yes | `ER_GMA`/`ER_SMA`/`ER_BMA`/`ER_NMA` off `Rewards::medal` |
| `MedalImg` (hex-dash glyph) | Yes, campaign + no medal only | the one measured case (`results-02.png`) |
| `MedalImg` under an earned trophy | **No - settled 2026-09-30, confidence 80** | a live gold-medal run (campaign `grid0_3_2`, one lap, 1.19.84, `Gold medal awarded`) shows only the gold trophy in the medal square and **no static hex glyph under it**; the trophy **spins** about its vertical axis (seven frames 0.9 s apart, edge-on to full face and back, about a 6 s period) where this build holds its first frame - `StartPaused` and the animation stay **chosen, not measured** here |
| trophy (`TrophyPanel`, `g_trophy`/`s_trophy`/`b_trophy`) | **Yes, 2026-09-28**, on a campaign race that earned a medal | the medal's own model, `oag_game::endrace::Trophy`, drawn with the disc's `Mode3D` camera - see "The trophy" below |
| `RewardLine2`/`RewardLoyaltyActive`/`loyaltynum` (the loyalty row) | **Yes, 2026-09-14** | `Race_ComputeLoyaltyAward`/`Loyalty_AccumulateTotal` landed in main (confidence 95/90) - see [`oag_ui_screens::endrace::Loyalty`](../../crates/ui-screens/src/endrace.rs) and `crate::race_stage::endrace::loyalty_award`. `None` (nothing draws) only when a launch names no team at all |
| `loyaltybg`/`loyaltybar` | Yes, alongside the row | `loyaltybar` is `total * 0.00124` **pixels** wide and as many texels of the sheet (not a fraction of the authored width; the `100000` cap is the authored `124`), so a total of 90 shows only `loyaltybg`, as the reference frame does - see [The loyalty bar](#the-loyalty-bar-is-pixels-not-a-fraction) |
| `LoyaltyImg` | Yes, alongside the row | |
| `BigPos` | **No** | a finishing-position figure this model carries no place for - would need threading a `place` into `Rewards` that nothing else on this screen needs |
| `ContinueButton` etc. | Yes | direct |

### The trophy (2026-09-28)

`oag_ui::screen` collects every `<Mode3D>`'s `<Model>`s
(`Screen::models`, `oag_ui::screen::Mode3dModel`), `oag_game::endrace::load`
decodes `TrophyPanel`'s three through `oag_game::preview::model`, and
`EndRaceRuntime` draws the one the campaign medal names with
`Preview::draw_mode3d` after the screen's own widgets.

| Claim | Status | Basis |
| --- | --- | --- |
| which model is which medal | decompiled, 80 | the widget names, the switch `EndRaceRewards_OnEnter` makes on the ordinal (`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`) |
| camera and pose | measured | `Mode3D_ReadValues`' camera law and its `nearZ`/`farZ` defaults `20`/`100` (`docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`), `TrophyPanel`'s authored origin, the model's `z="-75"` |
| where it lands | **cross-checked** | `OriginX/Y = 145/60` puts each model's origin inside `MedalImg`'s own 32x32 square - two widgets the file places independently agreeing. Pinned by `crates/game/tests/endrace_trophy_ground_truth.rs` |
| held at its first frame | **chosen** | every model authors `StartPaused="yes"`; the `\|= 4` `OnEnter` sets on the chosen one reads as the widget's visible bit, not an animation release. Whether the original spins it is unmeasured |
| `Enabletransition="2.0"` | **not applied** | the fade/grow this names is not read; the trophy appears at once |

Captures: `--menu-page endrace-rewards-gold`/`-silver`/`-bronze` (new page
names), and a live campaign race that earned bronze -
`data/scratch/drive-2026-09-28/erp/shots/p11a.png`/`p11b.png` (gitignored).

### `EndRace Menu`

| Widget | Draws | Why |
| --- | --- | --- |
| `Endrace Options` rows | Yes | `RETURN TO GRID` (campaign) or `RETURN TO MENU` (not), `RACE AGAIN`, `VIEW RESULTS AGAIN` - `ER_NEXT_RACE`/`ER_SAVE_QUIT` never appear (no Tournament mode), `ER_SAVE_GHOST`/`MSC_DEL_DATA` never appear (see below) |
| `GhostLine2`/`GhostTime2` (`ER_NEW_GHOST`) | Yes | this run's own best lap, off `Standing::best_lap_ticks` |
| `GhostLine1`/`GhostTime1` (existing-ghost row) | **No** | this project keeps no on-disk ghost yet - the same absence `docs/ui/fe-menu-definitions.md`'s own `RECORDS` section already chose for a column it could not back |
| `Endrace Difficulty` list | **No** | its own binding was never traced (`docs/formats/endrace-screens.md`, confidence 55) |
| `ContinueButton` etc. | Yes | direct |

## Measured-vs-chosen, in one table

| Claim | Confidence | Basis |
| --- | --- | --- |
| Widget positions, idstrings, per-mode option list | Measured, per `docs/formats/endrace-screens.md`'s own table | direct XML/decompile read |
| The totals-row highlight's own position | **Chosen** | no decompile of the row-highlight positioning; matched to the row visually |
| Medal-earned trophy placement/size | Measured camera law, cross-checked against `MedalImg` | see "The trophy" above; its first-frame hold is chosen |
| `MedalImg` under an earned trophy | **Not drawn either way** | confidence 55, undetermined |
| Loyalty award/total (the numbers) | Measured, confidence 95/90 | `Race_ComputeLoyaltyAward`/`Loyalty_AccumulateTotal`, confirmed on two live Time Trial races - `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md` |
| Loyalty award/total, `SingleRace`/`Zone`/`Eliminator` branches | Decompiled, **not independently live-verified** | same page's own "Still open"; drawn under the same law regardless |
| `SingleRace`'s own difficulty multiplier | **Never applied** (`multiplier` fixed at `1`) | this project's `[ai] difficulty` (4 tiers) has no honest mapping to the original's campaign-cell `easy`/`medium`/`hard` (3 tiers, `AI_ResolveSkillScale`, unimplemented) - see `crate::race_stage::endrace::loyalty_award`'s own doc |
| "Perfect lap"/"perfect zone" counts feeding the award | **Always `0`** | this project keeps no running tally for either - see [Open](#open) |
| `loyaltybar`'s own fill width | Measured, confidence 93: `total * 0.00124` pixels, cropped (width and U extent together); live points at 90, 20080 and 50080 | [The loyalty bar](#the-loyalty-bar-is-pixels-not-a-fraction) |
| A confirm anywhere on `Results`/`Rewards` advances | Matches the disc's own `ContinueButton`/cross-or-start reading | `docs/formats/endrace-screens.md` |
| `RETURN TO GRID` re-launches through the existing campaign session flow, not a parallel path | Chosen, reusing `oag_game::campaign`/`Session::open_campaign`/`CellSelection::select_by_name` | this project's own "don't fork it" convention |

## Captures

`--menu-page endrace-results` / `endrace-rewards` / `endrace-menu`
(`pulse-psp-usa.chd`, 960x544 = 2x PSP), fed the one live capture this
project holds (`docs/ui/campaign-screens.md`'s "After a campaign race,
measured": `grid0_3_2`, Time Trial, 3 laps, no medal) rather than a real
race's own outcome - a `--menu-page` capture has no `Session`/`RaceStage`
behind it to read one from. `endrace-menu-tournament` is a fourth page name,
Pulse only: the identical `EndRace Menu`, but with `ER_NEXT_RACE` in place
of `RACE AGAIN` - the row a Tournament cell's own non-last leg offers, per
[`race-modes.md#tournament`](../gameplay/race-modes.md#tournament); a
capture-only knob, since driving a real tournament leg to a finish is not
(`docs/ghidra/functions/psp-pulse-usa/tournament.md`'s own "live
verification" names the autopilot cost this would take). Kept at `/tmp/oag-drive/endrace/endrace-{results,rewards,menu}.png`
(not committed - game content). Digit-for-digit against the reference
frames' own seconds figures except the last centisecond on two of three
splits (`1.32.48` vs `1.32.49`, `0.49.33` vs `0.49.34`) - an accepted,
documented artifact of the 60 Hz-tick-to-centisecond conversion not
round-tripping losslessly (`crate::capture::endrace_page::seconds_to_ticks`'s
own doc), not a drawing bug; the total (`0.03.00`/`3.11.76` scale) and the
third split (`0.49.93`) land exactly.

### Live, 2026-09-14

Driven under Xvfb (`:98`, 1280x720) with `xdotool`, `oag-game --autopilot
data/images/pulse-psp-usa.chd`, an isolated `XDG_CONFIG_HOME` (the shared
`~/.config/oag/settings.toml` authors a `1600x900` window, wider than the
Xvfb screen - copied with `window_size` overridden to `1200x680` rather than
edited in place). `RACE CAMPAIGN` -> `Grid 1` -> `grid0_3_2` (`Time Trial`,
Talon's Junction White, Venom, 3 laps, gold target `1:55.00`) -> `Team
Selection` -> confirm `Assegai` -> autopilot raced it to `2:10.36`, over
target (an honest miss, same as the earlier campaign-screens.md pass).

**`EndRace Results` confirmed working live**: `ERGEBNISSE` (RESULTS,
German - this machine's source has no English table), `ZEITRENNEN
ABSOLVIERT!` (TIME TRIAL COMPLETE!), the per-lap table with laps 2/3 and the
highlighted `GESAMT` (Total) row at `2.10.36`, drawn over the still-visible
race scene exactly as the reference frames show. This is real, on-screen
confirmation the drawing path works end to end, not only in a `--menu-page`
still.

## The loyalty bar is pixels, not a fraction

Closed 2026-10-02 (`pulse-loyaltybar`). The old note said the decompiled `total *
0.00124` fraction put a `90` total at ~11 % and that `results-02.png` showed the whole
bar lit. Both halves were wrong. `EndRaceRewards_Update` writes `total * 0.00124` into
the bar's **width in pixels** and its **U extent in texels** (the pair
`Image_DenormalizeUv`/`Image_NormalizeUv` brackets the second write), so `90` is
`0.11` pixels wide, and `FEScreen_SetStatBar` does the same with `fullWidth * value /
max` - see [the decompile
page](../ghidra/functions/psp-pulse-usa/endrace-screens.md#the-loyalty-bars-fill-pixels-not-a-fraction-2026-10-02-pulse-loyaltybar).
The reference frame's bar is uniformly dim (~130 on every segment, no bright column):
that is `loyaltybg` alone at half alpha, not a lit bar. `100000 * 0.00124 = 124` is the
authored width, which is why it looked like a fraction.

**Drawn:** `oag_ui_screens::endrace::draw`'s `loyalty_bar_draw` sets the width and the texture
width to `total * 0.00124` (a crop, where the old code narrowed the rectangle and left
the whole 124-texel window in it, squashing all twenty segments into a few pixels).
`--menu-page endrace-rewards` at total `90` now reads ~126 on every segment along the
bar, against the reference's ~130, with no bright column. A unit test pins the slope
at `90`, `50000` and `100000`. **Confidence 88, raised to 93 on 2026-10-02
(`pulse-cursor-live`)**: a PPSSPP poke of the team record's `+8` gave totals of 50080 and
20080 on the real `EndRace Rewards`, and the bright fill measured 123 and 49 screen pixels
against the predicted 124.2 and 49.8 (within one pixel), cropped mid-segment exactly as
ours is at 50080 - see [the decompile page's
measurement](../ghidra/functions/psp-pulse-usa/endrace-screens.md#the-slope-measured-at-two-totals-2026-10-02-pulse-cursor-live).
No code change was needed. **Also unchased:** the original sets the bar, its
background and `loyaltynum` visible only once the ticker finishes, this build draws
them from the first frame.

## Open

- **`EndRace Rewards` loyalty row text (seen 2026-10-02, `pulse-cursor-live`, not chased):** the reference reads one
  mixed-case string `Assegai Loyalty: 80 Points` and `Total loyalty: 50080`; this build draws upper-case
  `ASSEGAI LOYALTY` and `80 POINTS` as two columns, the same shared-table font gap as above.

- ~~**The trophy model is not wired this pass.**~~ **Wired 2026-09-28** -
  see "The trophy" above; the struck text is kept for its history.
  `oag_game::preview::model` can
  load an arbitrary `.vex` by path the same way a picker's own ship preview
  does (`Data\FE\trophies\gold.vex`/`silver.vex`/`bronze.vex`), and
  `TrophyPanel`'s own `OriginX="145.0" OriginY="60.0"` is a real, measured
  number `EndRace_Definition.xml` authors - but `Mode3D`/`Model` are not yet
  collected by `oag_ui::screen::Screens::collect_widgets` (no `"mode3d"` arm),
  and threading the medal-earned case through `oag_game::preview` was not
  reached in this pass's time budget. No stand-in was drawn in its place -
  the row is simply absent for a medal-earning race, which this project's own
  zero-medal profile cannot exercise to check against anyway (see the next
  point).
- **A medal-earning run exists now, of this build only** (2026-09-28: a
  live Pulse campaign race, 3rd, bronze, trophy drawn). No capture of the
  *original* earning a medal exists, so the trophy's animation and
  `MedalImg`'s state under it are still unmeasured. The older text: Every capture
  this pass has - live and headless alike - is a zero-medal profile, the
  same limitation `docs/ui/campaign-screens.md` already records for the hex
  swatch colours. `MedalImg`'s own state under an earned trophy (see the
  table above) cannot be settled without one.
- ~~**A discrepancy surfaced in the live walk**: a single confirm at `EndRace
  Results` after a campaign race landed on `Main Menu`.~~ **Root-caused and
  fixed, 2026-09-28 (`pulse-campaign-flow` lane) - and it was not
  `campaign_cell`.** `Session::frame`'s tick loop
  (`crates/game/src/main/session/frame.rs`) still carried the built-in
  results table's own dismiss block - any Cross/Start rising edge on a
  finished race called `Session::escape`, i.e. `Main Menu` - and it ran
  *before* the finished-race arm that reaches `Session::tick_endrace`, with
  no guard on the EndRace flow being built. So the pad's confirm never
  reached `EndRace Results` at all, on every race with a built flow,
  campaign or not; only a mouse click advanced it (`pointer::press_for_click`
  was already skipped once `endrace` is `Some`). The block is now gated on
  `race_stage::endrace::results_table_takes_confirm(finished, endrace_built)`,
  pinned by `race_stage::endrace::tests::a_built_endrace_flow_keeps_the_confirm_from_the_results_table`.
  **Verified live** (Xvfb, `pulse-psp-eu.chd`, keyboard only, autopilot):
  `RACE CAMPAIGN` -> `grid0_3_1` (Single Race, Moa Therma White, Venom, 3
  laps) -> `Team Selection` -> raced to 3rd, `2.16.05` -> Enter at `EndRace
  Results` -> `EndRace Rewards` ("bronze medal received") -> Enter ->
  `EndRace Menu` with `RETURN TO GRID` as the default row -> Enter ->
  `Cell Selection` back on `grid0_3_1`, its `Best` now `Bronze`. So
  `RaceStage::campaign_cell` does survive the drain in
  `Session::advance_race_build`. **Seen on the same walk, fixed
  2026-09-28**: `EndRace Rewards` drew no loyalty row. Cause, confirmed by
  reading the code: `Session::launch_campaign_cell` built `race_options`
  without a team, and the campaign's `Team Selection` Confirm went straight
  to `Session::finish_launch`, skipping `launch_from_settings`, the only
  writer of `race::Options::team`. A session's first campaign race so flew
  `team = None` (no loyalty row, nothing banked, and `race::load`'s own
  default ship rather than the one picked); after a RACE-page race it flew
  that race's stale team instead. The team/variant/skin resolution is now
  `menus::team::apply_race_team`, shared by the RACE page and a new
  `Session::launch_campaign_race` tail that both campaign exits take (the
  picker's Confirm and `launch_campaign_cell`'s no-picker fallback). Pinned
  by `session::menus::team::tests::*`, at the helper only: `Session` needs a
  GPU and has no headless harness, so the two call sites are covered by
  the live walk alone. **Verified live** (Xvfb `:93`, `pulse-psp-usa.chd`,
  fresh records, keyboard): `grid0_3_1` -> `Team Selection` moved off the
  stored `Assegai` to `Feisar` (3/12) -> slot 0 loaded
  `Data\Ships\Feisar` -> 5th, `2.16.83` -> `EndRace Rewards` drew `NO
  MEDAL AWARDED` and `FEISAR LOYALTY 45 POINTS`, `TOTAL LOYALTY: 45`, and
  `records.toml` banked `[[loyalty]] team = "feisar" total = 45` -> `EndRace
  Menu` -> `RETURN TO GRID` -> `Cell Selection`.
- ~~**`boostimg`'s own condition disagrees with the decompile.**~~ **Closed
  2026-09-30**: it did not - `|= 4` is *visible*. The header icon draws on the lap
  table now, and the column under it is the per-lap **speedup-pad count**
  (`Ship_ApplySpeedupPad`, found with a live write watchpoint), which this build
  tallies on the same pad-entry edge. Open: lap 1 reads `6` on both captures against
  `9`-`10` on the laps after, unexplained, and this build's own counts have not been
  laid against a full lap of the original on the same circuit.
- **`EndRaceResults_Update` and the network table** are read but draw nothing here:
  this build has no network play (`EndRaceResults_PopulateMultiplayerTable`).
- **A parser quirk this pass routed around** rather than fixing:
  `Screens::collect_widgets` places a colour-only `<Image OffsetY=...>` wrapper at
  its own `y` without the tag's `OffsetY`, so the eight `tablebg{n}` backings landed
  on the top of the screen (`y` 0 and 1). `oag_ui_screens::endrace::table` puts the offset
  back for those eight; the shared reader is untouched, since nothing here measured
  another screen that depends on the current behaviour. A wrapper on some other screen
  with an `OffsetY` and no `src` would show the same fault.
- **The scrolling tip ticker / button-legend footer this screen's own frame
  chrome might carry** were not investigated - out of scope for this pass,
  which is about the three `EndRace` screens specifically, not the shared
  `FE Screen` frame around them.

## Wipeout HD/Fury: `Results`/`Menu`, off a completely different file

**Added 2026-09-21.** `Data\Plugins\Frontend\Gui\EndRace_Definition.xml` - a
named plugin, plain UTF-8, five of seven archives - is HD's own copy, at a
different path, a different authored resolution (1920x1080) and an almost
entirely different widget vocabulary from Pulse's file this page otherwise
describes. [`docs/formats/hd-endrace-screens.md`](../formats/hd-endrace-screens.md)
is the reading half; this section is the picture half plus what live
verification found, the same split the rest of this page keeps.
Implemented in [`oag_ui_screens::endrace::hd`](../../crates/ui-screens/src/endrace/hd.rs)
(model reuse, draw, pointer), `oag_game::endrace::load_hd`, and the
title-dispatched `ResultsModel` in
[`crate::race_stage::endrace`](../../crates/game/src/main/race_stage/endrace.rs).
`oag_title::FrontEnd::endrace_entry` is the per-title axis both titles now
go through.

### What draws

`EndRace Results` - title, headline, and the whole field's own standings
grid (position and finish time, one row per craft, the player's own row
highlighted); `EndRace Menu` - the applicable `race_again`/`return_to_grid`/
`return_to_menu`/`view_again` options, each at its own authored `<Block>`
position. `EndRace Podium` draws too (2026-10-06, `--menu-page endrace-podium`
only): the winner in the middle column, second left, third right, at the
positions the slot setter computes - see the formats page. The original's
entry into it is unread, so the live flow stays Results -> Menu.
Full "what does and does not draw, and why" is
[`oag_ui_screens::endrace::hd`](../../crates/ui-screens/src/endrace/hd.rs)'s own module doc.

**Every one of the three end screens' own `Confirm` prompt draws, since
2026-09-25** - see the Rewards table below for the walking mechanism.
`EndRace Results`' own needed a second fix on the same day: a malformed tag
earlier in that screen's own XML (three copies, `MedalModelGold`/`Silver`/
`Bronze`) used to corrupt this project's own parse of everything the file
authors afterwards, `Confirm` included, until `oag_tables::fexml`'s own
parser gained a general recovery for the shape - see
[hd-endrace-screens.md](../formats/hd-endrace-screens.md#a-malformed-tag-upstream-swallowed-navigationcontroller-on-this-screen-alone---fixed-2026-09-25)
for the full account.

### `EndRace Rewards`: drawn, never entered (2026-09-25)

HD's original never enters its own `EndRace Rewards` - no redirect in any
screen file names it and the executable registers no screen class for it
(evidence and confidence in
[`hd-endrace-screens.md`](../formats/hd-endrace-screens.md#endrace-rewards-authored-never-entered)).
So the live flow is left as it was: `Results` -> `Menu`, the route HD's own
`EndRaceMenuRedirect` authors, and `Session::build_endrace` builds no
rewards model on HD. The screen is drawn only by
`--menu-page endrace-rewards`, off `DATA02`'s own widgets:

| What | Drawn from | Measured or chosen |
| --- | --- | --- |
| Backdrop, title (`ER_REWARD`), three dividers | the widgets as authored | measured |
| `BigPos` | the player's finishing place | chosen, not measured - the name and its `"1"` placeholder are the only evidence it is a place |
| `RewardLine1` | `ER_GMA`/`ER_SMA`/`ER_BMA`/`ER_NMA` on a campaign race, nothing otherwise | chosen, not measured - the widget authors the bare label `ER_MEDAL_AWARD`; the four tier idstrings are in HD's own English table |
| `MedalImg`, `LoyaltyImg` | nothing | src-less 32x32 icons the original would texture at run time; a flat square of the authored colour would be a stand-in |
| Loyalty row (`RewardLine2`, `RewardLoyaltyPoints`, `RewardLoyaltyActive`, `loyaltybar`) | nothing | HD never enters this screen and shows its loyalty on `Results` (below); Pulse's law is the PSP's, not HD's, and the placeholders (`"test"`, `"points!"`, `"line 2"`) never draw |
| Confirm prompt (word) | the resolved `FE_CONFIRM` text, at the authored position | **measured, since 2026-09-25** - `crate::screen::Screens::collect_widgets` now walks a `NavigationController` the same as any other container |
| Confirm prompt (icon glyph) | nothing | `font="buttons"` - a `Buttons`-role atlas now loads (2026-09-25, HD's campaign footer), but this screen's own draw path does not yet route to it; see `docs/formats/hd-endrace-screens.md`'s own doc for the gap and why a wrong glyph would be worse than none in the meantime |

Pointer: the screen has nothing to select, so it needs no target list - a
click anywhere is its confirm, the rule `oag_ui_screens::endrace::pointer`'s module
doc already states for Pulse's Results/Rewards. It would apply unchanged if
the screen were ever wired in.

Verified headlessly in
[`crates/game/tests/hd_endrace_ground_truth.rs`](../../crates/game/tests/hd_endrace_ground_truth.rs)'s
`hd_endrace_rewards_draws_the_place_and_medal_off_the_real_definition`: the
real disc's definition and English table, `BigPos` fed place 3 (not its
authored `"1"`), `BRONZE MEDAL AWARDED` resolved, no loyalty placeholder and
no icon tile drawn. Captured at 1280x720
(`--menu-page endrace-rewards`, `hdfury-ps3-eu-dec.iso --mode single_race`,
ticks 0 and 120, identical as expected for a static screen): `REWARDS`, the
`1` at `BigPos`'s authored spot and `GOLD MEDAL AWARDED` beside it, the three
dividers, nothing else.

### Two real bugs, found by looking rather than by reading

Live verification (`--menu-page endrace-results`/`endrace-menu` against
`hdfury-ps3-eu-dec.iso`) surfaced two bugs the unit tests - built off a
synthetic fixture - could not have caught, both fixed in the same pass:

1. **`Line1`'s headline drew the literal idstring `ER_1STP`, unresolved.**
   HD's own English table (`Data\Plugins\Languages\English\entries.xml`,
   measured directly) names a finishing position `ER_1PLACE`..`ER_8PLACE`,
   not Pulse's `ER_1STP`..`ER_8STP` - a real, measured divergence between
   the two titles, even though the three other headline idstrings
   (`ER_TT_COM`/`ER_SL_COM`/`ER_SHIP_DES`) and every `EndRace Menu` Block's
   own idstring are byte-identical between them. `hd_headline_text` now
   branches on this.
2. **The Target/medal block and the loyalty block - both explicitly "not
   drawn this pass" in the module doc - were leaking through anyway**: the
   generic text-drawing fallback only excluded `Gridp.{row}`/online-only
   widgets by name, so `Target0`'s own literal `"value"` and
   `loyalty1.1`/`loyalty2`'s own `"834 POINTS"`/`"3745"` placeholders drew
   unconditionally. Excluded explicitly now.

A third issue found the same way, not a logic bug but a missing override:
every `Grid{col}.{row}` cell's own authored position is `x="0" y="0"` (see
the formats page), and the first pass called the generic text-drawing
helper with that unmodified position - so the position/time text existed in
the draw list but sat invisibly at the origin. Cells are now repositioned
the same way `GridHighlight` already was, reading column `x` off
`GridHead1`/`GridHead2` at draw time rather than a hand-transcribed
constant.

All three are covered by new tests: `oag_ui`'s own unit tests (a miniature
XML fixture) and
[`crates/hd/tests/endrace_screens_ground_truth.rs`](../../crates/hd/tests/endrace_screens_ground_truth.rs),
which asserts the `ER_{n}PLACE`-not-`ER_{n}STP` divergence directly against
the disc's own string table - the test that would have caught bug 1 before
a screenshot did.

### 2026-09-28: the Menu's cursor, and a Results grid that fits eight rows

A live HD campaign walk (`data/scratch/drive-2026-09-28/campaign-launch-walk.md`)
found two things wrong, both fixed from the executable rather than chosen:

- **`EndRace Menu` had no visible cursor** - white rows on the light panel,
  the selected one only brightened. The option Blocks author their own look
  and `Block_Update` says how a focused one draws: its `ActiveColor`
  (`0xff8ac0ca`, the constructor's, since the screen authors none) over the
  grey `Color`, sixty units wider at a sixth per tick, with a 32x32
  `HD_options_arrow.gtf` blinking eight ticks on, nine off, at `X + 8`; the
  label sits at `X + 40` in `TextColor`. See
  [menu-blocks.md](../ghidra/functions/ps3-hdfury-eu/menu-blocks.md#a-standalone-block-parse-selectable-update-2026-09-28).
  **On the Fury style the cursor stays light blue while `Results`'
  `GridHighlight` is red, and that is right**: the highlight authors
  `FEGlobals->HD_Blue`, which the served Fury archive resolves to red, and
  the Block's focus colour is a compiled-in literal, not that global (the
  constructors set no "is a global" bit for it). Do not "fix" one to match
  the other.
  The walk that verified it found a third bug: Up moved the cursor *down*,
  because the shared option list is Pulse's order (`RETURN TO GRID` first)
  while HD draws each option at its own Block's `y`.
  `oag_ui_screens::endrace::hd::hd_screen_order` now steps them in screen order and
  keeps the default focus.
- **The 8th Results row sat on the footer bar.** The file's frame is the
  Time Trial layout; on a race `EndRaceResults_LayoutGrid` hides
  `GridBottomBlock` and stretches the frame to a `487` bottom bar, and rows
  sit at `96 + 45 r`, not the `347 / 8` pitch this page used to call chosen.
  The time column is `x = 545` (`Grid2`), captioned by `GridHead2`'s label
  at its Block's `X + 40` - and the headers now draw as their grey Blocks,
  which is what makes the white `POS`/`TIME` legible. See
  [endrace-results-grid.md](../ghidra/functions/ps3-hdfury-eu/endrace-results-grid.md).
  Time Trial / Speed Lap keep the file's frame and the older chosen pitch:
  their fillers were not read.

**Live**, Xvfb `:95`, `hdfury-ps3-eu-dec.iso`, Fury campaign, Talon's
Junction, autopilot 4th: Results with all eight rows inside the frame
(`erp/shots/h05.png`, `h12.png`); Menu with the blue focused block and the
arrow on its lit phase in one of four consecutive frames (`h06-crops.png`);
Up, Down Down stepping top-to-bottom (`h13-15-crops.png`); pointer hover
moving the focus and a click on `RETURN TO GRID` landing on Cell Selection
(`h16-hover-crop.png`, `h17-after-click.png`). All under
`data/scratch/drive-2026-09-28/erp/` (gitignored).

### HD's loyalty block on `Results` (2026-10-02, `hd-endrace-loyalty`)

HD shows its loyalty on `Results`, not on a `Rewards` screen. The law is HD's
own - **not Pulse's** - recovered statically in
[`endrace-loyalty.md`](../ghidra/functions/ps3-hdfury-eu/endrace-loyalty.md)
(`Race_ComputeLoyaltyAward` `0x00023f98`, confidence 82; the Results ticker
`0x00224488`/`0x00225030` agrees on every rate): HD's Eliminator pays 15/25 a
lap, zones pay 5/15, Detonator pays 150 a stage with no kill term, a race with
an AI craft is x2/x3/x4 by the rung with no x1, and there is no suggested-ship
doubling. `oag_hd::loyalty` is that law, `crate::race_stage::hd_loyalty` maps
this project's race onto it, and `Session::build_endrace` banks the award
(`records::record_loyalty`, the same `100000` ceiling) and hands
`oag_ui_screens::endrace::FieldResults::loyalty` to the draw.

| Widget | Draws | Source |
| --- | --- | --- |
| `loyalty1.1` | `"<award> POINTS"` (`"%d %s"` of the award and `ER_POINTS`) | `0x00224488`'s zero-award state and `0x00225030`'s last step |
| `loyalty1.2` | nothing | the ticker ends on `""` |
| `loyalty2` | the team's total as a bare number | `"%d"`, `0x007940f0` |
| the bar | **nothing, on purpose** | HD's Results code has no `loyaltybar` (no such string anywhere in the executable); Pulse's `total * 0.00124` pixels is not ported |

An award of `0` still draws `0 POINTS` and the total (the executable's own
zero-award path). With no team on the launch (`--race` with no `--team`) the
block draws absent, never the disc's `834 POINTS`/`3745` placeholders.

**What is chosen, not measured:** a race with no campaign cell uses the `Easy`
rung (HD's own campaign model opens on it here; the executable always holds a
rung); `Head2Head`, which HD does not ship, counts as a single race; perfect
laps and perfect zones are never counted (nothing in this project tallies
either), so the award is short by what they would have paid. **Not
reproduced:** the ticker's animation (a line per reason, a fade and a cap on
the speed, `0x00225030`) - the final state only. **Not measured:** the whole
law is decompilation, no RPCS3 run (the cap on every row is 84).

**Colours are what the file authors** (`EndRace_Definition.xml`, the `Item` at
`1180,368`): the caption `ER_LOYSTAT` and `loyalty1.1` are `FEGlobals->HD_Grey`,
`loyalty2` is white and right-aligned at `x = 352`, and the two `<Block>`s
(`ER_LOY` at `y = 24`, `IG_HUD_TOTAL` at `y = 112`, `HD_Blue` fill, white
`TextColor`) carry the white labels. Those blocks author **no `name`**, so a
first capture drew them as bare white text on the light panel; they now draw
as blocks like the grid headers (`unnamed_block_at`). `"135 POINTS"` is grey
`HD_Grey` text on the light panel exactly as authored - dim, but nothing in the
file backs it with a fill. Unverified against a live frame. The two `4 px`
vertical bars (`x = 1` and `361`, `y = 44`, `70` tall, `HD_Blue`) are authored
`<Image>` fills and are the sides of that panel.

Capture: `--menu-page endrace-results --size 1280x720` against
`hdfury-ps3-eu-dec.iso` draws `LOYALTY` / `135 POINTS` / `TOTAL 4020` at the
block the file authors (`x = 1180`, `y = 368`) - the numbers are the capture's
own **chosen** sample (a three-lap single race on the medium rung banked onto
3885), since a `--menu-page` capture has no race behind it. Kept under
`data/scratch/hd-endrace-loyalty/` (gitignored).

### Captures

`--menu-page endrace-results`/`endrace-menu`, `hdfury-ps3-eu-dec.iso`,
`--track Data\Environments\Talons_Junction --mode single_race` (a
synthetic two-craft field - no live `Session`/`RaceStage` behind a
`--menu-page` capture, the same gap Pulse's own capture above has). Kept
under `data/scratch/lane-hd-endrace/shots/` (gitignored - game content).
`POS`/`TIME` headers, `1ST PLACE`, and the player's own row (`1`, `3.11.76`)
all draw at their own real positions; the applicable `EndRace Menu` options
(`RACE AGAIN`/`RETURN TO GRID`/`VIEW RESULTS AGAIN`) draw at their own
stacked `y`s with none of the unimplemented Tournament/multiplayer options
leaking through. Pulse's own `endrace-results` capture
(`pulse-psp-eu.chd`) was re-taken after this pass's changes and is
digit-for-digit identical to this page's own reference numbers above -
`RÉSULTATS`/`CONTRE-LA-MONTRE TERMINÉ!`/`1.32.48`/`0.49.33`/`0.49.93`/
`3.11.76` (this machine's own source has no English table, the same
"German, not missing" situation the Live section above records for
Pulse).

**Re-captured 2026-09-25 after the `fexml` malformed-tag fix**: `endrace-results`
now also draws `CONFIRM` bottom-left, the same position and style as
`EndRace Menu`/`EndRace Rewards` - see the formats page's own account of
the bug this closes.

**The disc-backed end-to-end path is verified without a GPU or a live
race**, in
[`crates/game/tests/hd_endrace_ground_truth.rs`](../../crates/game/tests/hd_endrace_ground_truth.rs):
a real HD single race, autopiloted to its own finish (headless simulation,
~18 s wall clock), its real `Board`, `EndRace_Definition.xml` read off the
same disc, and `hd_results_draw_list` fed both - the player's own place
lands in the drawn field. See that file's own module doc for why it stops
short of `EndRaceRuntime` (GPU-touching glue with its own fast unit tests,
`race_stage::endrace::tests::hd_field_rows_*`).

### Live: menus and a race launch, 2026-09-21 - not to a finished race

Driven under Xvfb `:94`, mouse-only, with `xdotool` and an isolated
`XDG_CONFIG_HOME` (`window_size` overridden to `1200x680` inside the
1280x720 screen, the same reason Pulse's own 2026-09-14 walk above does
it). **`WAYLAND_DISPLAY` has to be unset** or the window renders into an
invisible Wayland session while every capture tool points at Xvfb -
`docs/architecture/menus.md`'s own "DISPLAY against GRAPHICS" section
documents the identical symptom (black screenshots) and fix for Pulse;
this is the same fix confirmed a second time, on HD.

With that fixed: `Main Menu` (a real `<HorizMenu>` strip, 41-63 FPS) ->
click `RACE CAMPAIGN` -> `Grid Selection` (a real point-cloud flyer,
`Event 01`) -> click the flyer -> `Cell Selection` (`VINETA K`, Single
Race, 3 laps) -> click the selected hex -> `LOADING... VINETA K` -> race
scene built in **53.7 s**, first frame presented **5.2 s** after that.
Menus render fast and mouse-driven navigation works exactly as it does on
Pulse and on HD's own campaign screens
(`docs/ui/campaign-screens.md`'s 2026-09-21 entry, which reached the same
"race genuinely started" point by an identical route). **The race itself
was not driven to a finished `EndRace Results`**: at ~5 s/frame, a
multi-hundred-tick race would take hours of wall clock in this sandbox -
independently confirmed by two lanes now (the campaign-screens.md entry's
own `race scene built in 107.6s` / `741ms-8056ms per frame`, and this
pass's `53.7s` / `5.2s`), consistent with a rendering-path limitation
specific to a full race scene under this sandbox's display stack rather
than either lane's own code. The disc-backed ground-truth test above is
this pass's substitute end-to-end check; a live capture through to
`EndRace Results` is real hardware's job. **Corrected 2026-09-28**: it was
not. With the HD render profile cut to `render_scale = 50` and MSAA, motion
blur and shadows off, the same llvmpipe adapter ran a campaign race at 60
ticks a second, and the walk reached `EndRace Results` and `EndRace Menu` -
see `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: walked live, end to
end, 2026-09-28". The recipe as it was written:

```sh
DISPLAY=:1 WAYLAND_DISPLAY= XDG_CONFIG_HOME=<isolated> \
    cargo run --release -p oag-game -- data/images/hdfury-ps3-eu-dec.iso --autopilot
# xdotool click through Main Menu -> RACE CAMPAIGN -> Grid Selection (click the
# flyer) -> Cell Selection (click the selected hex) -> let the race run to the
# flag -> screenshot EndRace Results, click Menu, screenshot EndRace Menu
```
