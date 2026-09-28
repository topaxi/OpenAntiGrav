# The three EndRace screens: `Results`, `Rewards`, `Menu`

**Status: all three draw**, off the disc's own `EndRace_Definition.xml`, for
Pulse. This is the *picture* half - what this build actually draws and why;
[`docs/formats/endrace-screens.md`](../formats/endrace-screens.md) is the
reading half (every widget, its authored position, its runtime source) and
[`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`](../ghidra/functions/psp-pulse-usa/endrace-screens.md)
is the decompiled law behind each value; this page does not repeat either.
Implemented in [`oag_ui::endrace`](../../crates/ui/src/endrace.rs) (model,
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
by this work) - the world simply stops being ticked, and
`oag_game::scoreboard::Overlay` already draws its own results table over that
frozen frame this same way. `crate::race_stage::endrace::EndRaceRuntime` reuses
that same mechanism rather than transitioning to `Stage::Menu`: it is drawn from
inside `RaceStage::draw_hud`, over the race's own last frame, with its own FE
sheet/skin/frame built once when the race finishes
(`Session::build_endrace`) rather than borrowed from `MenuStage`. **Chosen**,
not the only shape this could have taken - see that module's own doc for the
trade against a `Stage::Menu`-based design.

## What each screen draws, and what it leaves blank

### `EndRace Results`

| Widget | Draws | Why |
| --- | --- | --- |
| `BigTopText` (`ER_RES`) | Yes | direct idstring, resolved generically |
| `Line1` (headline) | Yes, for `TimeTrial`/`SpeedLap`/`SingleRace` | `SingleRace` uses the finishing-position idstring (`ER_1STP`..`ER_8THP`); `Zone`/`Eliminator` draw nothing (`Headline::Unresolved`) - their own populate helper (`FUN_088db574`/`FUN_088db1ec`) is undecompiled |
| `lap0.0`/`lap0.1` (header) | Yes | `RC_LAP`/`PRO_TIME` |
| `lap0.2` (header) | **No** | the column's own meaning is unread |
| `lap{n}.0`/`lap{n}.1` | Yes, up to [`oag_race::MAX_RECORDED_LAPS`] (4) | off `Standing::lap_splits`, which is itself capped at 4 - see below |
| `lap{n}.2` | **No, any row** | same reason as the header |
| totals row (`PRO_STATS_TOT` + `tablebg{n+1}`) | Yes | `Results::total_ticks`, the player's own finish tick |
| `tablehighlight` | Yes, repositioned onto the totals row | **chosen, not measured** - no decompile of this screen's own row-highlight positioning exists; see the doc on `results_draw_list` |
| `perfectlap{n}` | **No** | the flag's own direction (does nonzero mean "perfect" or the reverse) is unread. **Its own `idstring="MSC_PL"` text overlay used to leak through regardless** - found and fixed 2026-09-28: the overlay is a nested, unnamed `<Text>` inside the `<Image name="perfectlap{n}">` element (`docs/formats/endrace-screens.md`), so the by-`name` skip this row already gave the image half never caught the text half; `results_draw_list` now also skips any text whose `idstring` is `MSC_PL`. Visible as a faint "TP" (this source's own French) past each row in a pre-fix capture. |
| `boostimg` | **No** | the decompile's own "hidden unconditionally on every single-player path" reading - see [Open](#open) for why this project's one capture disagrees |
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
through a new model, [`oag_ui::endrace::TournamentResults`]
(`oag_ui::endrace::tournament_results_draw_list`), built in
[`crate::race_stage::endrace::tournament_results`](../../crates/game/src/main/race_stage/endrace.rs)
off this build's own [`oag_race::tournament`]/[`crate::race::tournament::Progress`]
(established, not re-derived by this pass) and the leg's own grid roster
(`crate::race::slot_teams`, recomputed at EndRace-build time - see that
function's own doc for why it is a second call rather than a threaded-through
field).

[`oag_ui::endrace::TournamentResults`]: ../../crates/ui/src/endrace.rs
[`oag_race::tournament`]: ../../crates/race/src/tournament.rs

| Widget | Draws | Why |
| --- | --- | --- |
| `BigTopText` | Yes | `ER_END_TOUR` on the last leg, otherwise `"%s %d/%d"` of `ER_RES` and the leg counter - `Progress::leg_number`/`leg_count` |
| `Line1` | Yes | `ER_RACE_STAN`/`ER_TOUR_STAN`, whichever page is current |
| `lap0.0`/`lap0.1`/`lap0.2` (header) | Yes | `PRO_POS`/`ER_TEAM`/`ER_POINTS` |
| `lap{n}.0` | Yes, one row per grid slot | the row's own 1-based index - `EndRaceResults_PopulateTournamentTable` does not sort, so this is simply which row a craft's own data landed on |
| `lap{n}.1` | Yes, when a team is known | the craft's own team name - `None` (this project keeps no per-slot team name past the race that just finished, unlike a live `Race`) draws the cell absent, never a guessed `"SLOT n"` |
| `lap{n}.2` | Yes | this leg's own points (leg page) or the running total (standings page) - `oag_race::tournament::points_for_finish`/`Progress::points` |
| `tablehighlight` | Yes, on the player's own row | measured at a different `y` pitch than the per-lap table's own (`93 + 20*row`, not `92 + 20*row`) - see `tournament_highlight_y`'s own doc |
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

**No live capture of this page exists yet** - `--menu-page
endrace-results-tournament-leg`/`endrace-results-tournament-standings` draws
it off a synthetic four-team field (`crates/game/src/capture/endrace_page.rs`),
since a real tournament leg run to a finish is the same multi-minute
autopilot cost `tournament.md`'s own "Live verification" section names, not
attempted again this pass. Kept under `data/scratch/pulse-tourney/shots/`
(gitignored - game content).

### `EndRace Rewards`

| Widget | Draws | Why |
| --- | --- | --- |
| header (`ER_REWARD`) | Yes | direct idstring |
| `RewardLine1` (medal-award phrase) | Yes | `ER_GMA`/`ER_SMA`/`ER_BMA`/`ER_NMA` off `Rewards::medal` |
| `MedalImg` (hex-dash glyph) | Yes, campaign + no medal only | the one measured case (`results-02.png`) |
| `MedalImg` under an earned trophy | **No** | confidence 55 on whether it stays visible - not determined by any capture this project holds |
| trophy (`TrophyPanel`, `g_trophy`/`s_trophy`/`b_trophy`) | **Yes, 2026-09-28**, on a campaign race that earned a medal | the medal's own model, `oag_game::endrace::Trophy`, drawn with the disc's `Mode3D` camera - see "The trophy" below |
| `RewardLine2`/`RewardLoyaltyActive`/`loyaltynum` (the loyalty row) | **Yes, 2026-09-14** | `Race_ComputeLoyaltyAward`/`Loyalty_AccumulateTotal` landed in main (confidence 95/90) - see [`oag_ui::endrace::Loyalty`](../../crates/ui/src/endrace.rs) and `crate::race_stage::endrace::loyalty_award`. `None` (nothing draws) only when a launch names no team at all |
| `loyaltybg`/`loyaltybar` | Yes, alongside the row | `loyaltybar`'s own fill width scales by `total * 0.00124` - see [Open](#open) for a visual mismatch against the reference frame this pass found and did not resolve |
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
| `loyaltybar`'s own fill width | Chosen shape (linear scale of the decompiled `0.00124` fraction), **visual mismatch against the reference frame** | see [Open](#open) |
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

## Open

- **`loyaltybar`'s own fill width does not match the reference frame.** The
  decompiled fraction (`total * 0.00124`,
  `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`) puts a `90`
  total at ~11% - a handful of bright segments and the rest showing the
  dimmer `loyaltybg` underneath, which is what this build's own capture
  draws (`/tmp/oag-drive/endrace/endrace-rewards.png`). `results-02.png`
  shows the **whole** bar lit at the same total. Not resolved this pass:
  either the `0.00124` constant means something other than "a linear
  fraction of the authored width" (a notch count? a different scale
  entirely?), or the reference frame's own bar renders every segment above
  some low floor rather than a literal proportional fill, or this project's
  two-layer bright/dim compositing is not how the original's own bar reads
  a partial fill at all. The arithmetic (`90`/`90` in the text) matches the
  reference exactly; only the bar's own width does not. Left for whoever
  next opens this screen with a Ghidra bridge, since it is a rendering-law
  question, not a drawing-code one - `crate::race_stage::endrace`'s own
  fraction is a direct, documented read of the one number
  `endrace-screens.md` gives.
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
- **`boostimg`'s own condition disagrees with the decompile.** `results-01.png`
  shows a flag/pennant glyph in the header row at approximately `boostimg`'s
  own authored position (`x=320 y=77`), but
  `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md` reads it as
  unconditionally hidden on every single-player path this pass decompiled.
  This build follows the decompile (draws nothing) rather than the one
  capture, on the reasoning that a widget's own firing condition is this
  project's `endrace-loose-ends` lane's call, not this one's - flagged here
  so it is not lost.
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
Implemented in [`oag_ui::endrace::hd`](../../crates/ui/src/endrace/hd.rs)
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
position. `EndRace Podium` is inventoried (see the formats page) but not
drawn: its three `pod_head.{1,2,3}` widgets sharing one idstring reads as an
authoring placeholder rather than something this build could draw correctly.
Full "what does and does not draw, and why" is
[`oag_ui::endrace::hd`](../../crates/ui/src/endrace/hd.rs)'s own module doc.

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
| Loyalty row (`RewardLine2`, `RewardLoyaltyPoints`, `RewardLoyaltyActive`, `loyaltybar`) | nothing | HD's loyalty law is not recovered; Pulse's is the PSP's, and the placeholders (`"test"`, `"points!"`, `"line 2"`) never draw |
| Confirm prompt (word) | the resolved `FE_CONFIRM` text, at the authored position | **measured, since 2026-09-25** - `crate::screen::Screens::collect_widgets` now walks a `NavigationController` the same as any other container |
| Confirm prompt (icon glyph) | nothing | `font="buttons"` - a `Buttons`-role atlas now loads (2026-09-25, HD's campaign footer), but this screen's own draw path does not yet route to it; see `docs/formats/hd-endrace-screens.md`'s own doc for the gap and why a wrong glyph would be worse than none in the meantime |

Pointer: the screen has nothing to select, so it needs no target list - a
click anywhere is its confirm, the rule `oag_ui::endrace::pointer`'s module
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
  The walk that verified it found a third bug: Up moved the cursor *down*,
  because the shared option list is Pulse's order (`RETURN TO GRID` first)
  while HD draws each option at its own Block's `y`.
  `oag_ui::endrace::hd::hd_screen_order` now steps them in screen order and
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
