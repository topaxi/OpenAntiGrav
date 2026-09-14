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
see that crate's own module doc. Pure and HD were not checked for this file
this pass; neither is assumed to carry it.

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
| `perfectlap{n}` | **No** | the flag's own direction (does nonzero mean "perfect" or the reverse) is unread |
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

### `EndRace Rewards`

| Widget | Draws | Why |
| --- | --- | --- |
| header (`ER_REWARD`) | Yes | direct idstring |
| `RewardLine1` (medal-award phrase) | Yes | `ER_GMA`/`ER_SMA`/`ER_BMA`/`ER_NMA` off `Rewards::medal` |
| `MedalImg` (hex-dash glyph) | Yes, campaign + no medal only | the one measured case (`results-02.png`) |
| `MedalImg` under an earned trophy | **No** | confidence 55 on whether it stays visible - not determined by any capture this project holds |
| trophy (`TrophyPanel`, `g_trophy`/`s_trophy`/`b_trophy`) | **No, this pass** | not wired - see [Open](#open) |
| `RewardLine2`/`RewardLoyaltyActive`/`loyaltynum` (the loyalty row) | **No, at all** | the award law is a separate lane's still-open decompile (`g_endrace_result + 8`'s own writer was not located) - drawing a number here would be inventing what the disc has not yet given up, per this project's own rule |
| `loyaltybg`/`loyaltybar` | **No** | same reason - a bar at its authored width would be a fabricated ~806-point loyalty total |
| `LoyaltyImg` | **No** | belongs to the same row |
| `BigPos` | **No** | a finishing-position figure this model carries no place for - would need threading a `place` into `Rewards` that nothing else on this screen needs |
| `ContinueButton` etc. | Yes | direct |

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
| Medal-earned trophy placement/size | **Not attempted** | see Open |
| `MedalImg` under an earned trophy | **Not drawn either way** | confidence 55, undetermined |
| Loyalty row (whole row) | **Drawn nothing at all** | a different lane's open decompile |
| A confirm anywhere on `Results`/`Rewards` advances | Matches the disc's own `ContinueButton`/cross-or-start reading | `docs/formats/endrace-screens.md` |
| `RETURN TO GRID` re-launches through the existing campaign session flow, not a parallel path | Chosen, reusing `oag_game::campaign`/`Session::open_campaign`/`CellSelection::select_by_name` | this project's own "don't fork it" convention |

## Captures

`--menu-page endrace-results` / `endrace-rewards` / `endrace-menu`
(`pulse-psp-usa.chd`, 960x544 = 2x PSP), fed the one live capture this
project holds (`docs/ui/campaign-screens.md`'s "After a campaign race,
measured": `grid0_3_2`, Time Trial, 3 laps, no medal) rather than a real
race's own outcome - a `--menu-page` capture has no `Session`/`RaceStage`
behind it to read one from. Kept at `/tmp/oag-drive/endrace/endrace-{results,rewards,menu}.png`
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

- **The trophy model is not wired this pass.** `oag_game::preview::model` can
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
- **Nothing here was checked against a medal-earning run.** Every capture
  this pass has - live and headless alike - is a zero-medal profile, the
  same limitation `docs/ui/campaign-screens.md` already records for the hex
  swatch colours. `MedalImg`'s own state under an earned trophy (see the
  table above) cannot be settled without one.
- **A discrepancy surfaced in the live walk, not root-caused this pass**: a
  single confirm at `EndRace Results`, after a campaign-launched race
  (`grid0_3_2`), landed the player on the ordinary `Main Menu` rather than
  stopping at `EndRace Rewards` or reaching `EndRace Menu`'s own
  `RETURN TO GRID` with the campaign context intact - behaviour consistent
  with `Rewards::campaign`/`RaceStage::campaign_cell` reading `false`/`None`
  at the point `Session::build_endrace` ran, despite the race having been
  launched through `Cell Selection`. Not reproduced with logging in this
  pass (each cycle costs a multi-minute autopiloted race), and no evidence in
  the run's own log narrows it further - no `RACE CAMPAIGN`/`cannot open`
  warning fired, which rules out `Session::return_to_campaign`'s own
  cell-search failing silently. **The next session's fastest path**: add a
  temporary `log::info!` to `Session::build_endrace` printing
  `stage.campaign_cell.is_some()` and to
  `Session::handle_endrace_menu_option` printing the `MenuOption` it
  received, then repeat this page's own live-walk recipe once. Until this is
  settled, `RETURN TO GRID`'s own live behaviour is unverified even though
  its code path (`Session::return_to_campaign`,
  `oag_ui::campaign::CellSelection::select_by_name`) builds and is exercised
  by nothing but a walk that did not reach it.
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
