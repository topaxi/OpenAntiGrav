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
| `RewardLine2`/`RewardLoyaltyActive`/`loyaltynum` (the loyalty row) | **Yes, 2026-09-14** | `Race_ComputeLoyaltyAward`/`Loyalty_AccumulateTotal` landed in main (confidence 95/90) - see [`oag_ui::endrace::Loyalty`](../../crates/ui/src/endrace.rs) and `crate::race_stage::endrace::loyalty_award`. `None` (nothing draws) only when a launch names no team at all |
| `loyaltybg`/`loyaltybar` | Yes, alongside the row | `loyaltybar`'s own fill width scales by `total * 0.00124` - see [Open](#open) for a visual mismatch against the reference frame this pass found and did not resolve |
| `LoyaltyImg` | Yes, alongside the row | |
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
| Confirm prompt | nothing | nested in a `<NavigationController>`, which `oag_ui::screen` does not walk - the same on Results and Menu |

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
`EndRace Results` is real hardware's job:

```sh
DISPLAY=:1 WAYLAND_DISPLAY= XDG_CONFIG_HOME=<isolated> \
    cargo run --release -p oag-game -- data/images/hdfury-ps3-eu-dec.iso --autopilot
# xdotool click through Main Menu -> RACE CAMPAIGN -> Grid Selection (click the
# flyer) -> Cell Selection (click the selected hex) -> let the race run to the
# flag -> screenshot EndRace Results, click Menu, screenshot EndRace Menu
```
