---
categories: [frontend]
---

# Pulse's EndRace screens are read, not drawn

2026-09-14. A campaign race in the original ends on three authored screens
this project had never opened before this pass: `EndRace Results` (the
per-lap table), `EndRace Rewards` (the medal glyph and the loyalty ticker)
and `EndRace Menu` (`RETURN TO GRID`/`RACE AGAIN`/.../`NEW GHOST RECORD`).
This project's own build draws its own results table instead
(`crates/game/src/scoreboard.rs`) - not touched by this thread, which is a
reading-only pass. This is the implementation brief for whoever picks the
drawing side up.

Read, in this order:

- [`docs/formats/endrace-screens.md`](../../docs/formats/endrace-screens.md) -
  every widget, per screen, with its authored position and its runtime
  source.
- [`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/endrace-screens.md) -
  the decompiled functions behind them, confidences, and what remains open.
- [`docs/ui/campaign-screens.md`](../../docs/ui/campaign-screens.md)'s "After
  a campaign race, measured" section - the one live capture (PPSSPP,
  `grid0_3_2`, Time Trial, no medal) both pages above are checked against:
  `results-01.png`/`results-02.png`/`results-03.png` under
  `data/reference/psp-campaign-screens/` (gitignored, not committed).

## What is now established

**All three screens are fully authored** in
`Data\Plugins\PI001\GUI\EndRace_Definition.xml` (`Data.wad`), previously
listed but never opened (`fe-menu-definitions.md`'s own 17-file census). Ten
`<Screen>` blocks total; seven are plain `Dialog`/`MemoryStick` support
screens needing no code binding (`EndRaceAutoSaveDisabled`/`Failed`,
`EndRaceSaveGhost`, `EndRaceDeleteGhost`, `TournySaveWarning`,
`Kill Game Transition`), and the three with real content are read down to
every widget and its runtime source. Headline findings, all in the two docs
pages above:

- **`EndRace Results`**: `RESULTS` header, a mode-driven headline
  (`race complete!` -> `TIME TRIAL COMPLETE!` for our one capture), and a
  9-row per-lap table (`Lap`/`Time`/a still-unidentified third numeric
  column, plus a totals row and per-lap "perfect lap" icons).
- **`EndRace Rewards`**: `REWARDS` header, a medal-award phrase
  (`No medal awarded` etc.) and - the interesting mechanism - **the medal
  colour is which of three separate 3D trophy models gets unpaused, not a
  re-sourced 2D icon**. A scrolling ticker shows this race's own loyalty
  award (`"90 Points"`) and then cycles through a fixed vocabulary of "why"
  reason strings, before showing the team's own persistent running total
  (`"Total loyalty: <n>"`) and a fill bar. **2026-09-14: the award's own
  law is now decompiled and confirmed on two live races** -
  `laps*30 + perfectLaps*50` for Time Trial/Speed Lap (`laps*15+perfectLaps*25`
  for the Race family, `laps*10+perfectLaps*20` otherwise), plus a per-kill
  term, a Race-family-only difficulty multiplier, and a doubling for a
  suggested-ship race. See `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`.
- **`EndRace Menu`**: the option list (`RETURN TO GRID`/`RACE AGAIN`/
  `VIEW RESULTS AGAIN`/`SAVE GHOST`/`DELETE DATA`) is built conditionally per
  mode, and `NEW GHOST RECORD` reads this run's own best lap, separate from
  the existing on-disk ghost `NO GHOST TIME FOUND` reports on.

## What maps onto this project's existing pieces, and what doesn't yet

**`crates/game/src/scoreboard.rs`** draws this project's own results table
today - the natural target for `EndRace Results`'s per-lap grid. The `Lap`
and `Time` columns map directly (this project already has per-lap times in
whatever `RaceStage`/`Observation` already tracks for the scoreboard); the
third column does not map onto anything yet, since its own meaning is
unresolved (see Open below) - **do not invent a value for it**, per this
project's own rule against drawing a plausible-looking stand-in. Leaving it
blank, or omitting the column, is the honest choice until it is read.

**`crates/game/src/records.rs`'s `[[campaign]]` table and
`Observation::campaign_medal`** (`docs/architecture/persistence.md`) already
carry the medal a campaign cell earns - `EndRace Rewards`'s medal-award
phrase and trophy selection is a direct draw off that same value, once
resolved to the same `0 = gold, 1 = silver, 2 = bronze, none` ordinal this
project's own `Cell::evaluate_medal` (`oag_tables::race_campaign`) already
produces. **The loyalty award and the "Total loyalty" running total have no
home in this project's schema at all yet** - `records.toml` has no loyalty
field, and nothing in `crates/tables::race_campaign` models a per-team
persistent counter the way the original's `DAT_08b31774` store does. A
drawing pass either needs that schema extended first, or has to draw
`EndRace Rewards` without the loyalty half (again: absence, not invention,
per this project's own rule) until the award computation itself is
understood - see Open.

**`NEW GHOST RECORD`/existing-ghost comparison** has no home either - this
project's own ghost/replay system (if any exists by the time this is
picked up; check `docs/overview/roadmap.md` for what milestone that is)
would need to expose "this run's own best lap" and "does a saved ghost exist
for this track/class" the way `EndRaceMenu_PopulateExistingGhost`/
`EndRaceMenu_PopulateOptions` do.

## Open

- ~~The loyalty-award computation's own writer~~ **Closed 2026-09-14**:
  `Race_ComputeLoyaltyAward` (`0x0880ac50`), decompiled in full and
  confirmed on two independent live races (`laps*30 + perfectLaps*50` for
  Time Trial/Speed Lap, `laps*15+perfectLaps*25` for the Race family,
  `laps*10+perfectLaps*20` otherwise, a per-kill term, a Race-family-only
  difficulty multiplier, doubled for a suggested-ship race) - see
  `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`. The
  per-team persistent total's own writer, `Loyalty_AccumulateTotal`
  (`0x08807884`), is a plain capped accumulate (`+= award`, ceiling
  `100000`) - a drawing pass now has both laws, not just the display field.
- **The per-lap table's third column** (`docs/formats/endrace-screens.md`'s
  own "the third column does not" section) - a real, stored per-lap 16-bit
  field, meaning still unresolved. Candidates ruled out: a finishing
  position (exceeds the 8-craft field), the `boostimg` icon (unconditionally
  hidden on every single-player mode path decompiled), and - new
  2026-09-14, from a second live race - **weapon/pickup counts** (both
  captures are `Weapons="off"` cells, so neither could carry one). A
  speedup/boost-pad count is the leading remaining candidate, unconfirmed.
- **The Tournament/Zone/Elimination/split-screen variants of `EndRace
  Results`'s own populate** (`FUN_088dad90`/`FUN_088db574`/`FUN_088db1ec`/
  `FUN_088d9588`) were not opened - `Line2`..`Line8` and `boostimg`'s real
  firing condition likely close once one of these is read.
- **The generic confirm-swallow-on-a-locked-tile mechanism** - found while
  reading these screens (`StateMachine_EvaluateRedirect`,
  `0x088c8798`, ruled out as the predicate itself), and narrowed further
  2026-09-14: the real `Cell Selection` dispatcher (`ConfirmButton_Update`,
  `0x088c8e88`) is found and cleared too, pinning the gate to input
  consumption upstream of it - is a `race-campaign.md` thread, not this
  one; see
  [`the-race-campaign-is-authored-shape-not-content.md`](the-race-campaign-is-authored-shape-not-content.md)'s
  own `Open`/`Next Steps` for where that stands.
- **Two no-medal Time Trial captures now exist**, but **no medal-earning
  run** - `MedalImg`'s own static-icon-under-the-trophy question is still
  open, and the Race-family/Zone/Elimination loyalty branches are still
  decompiled-only.

## Next Steps

**All three screens now draw**, 2026-09-14 - `oag_ui::endrace` (model/draw/
pointer), `oag_game::endrace` (the disc read), `crate::race_stage::endrace`
(the runtime a finished race holds open, drawn inside `Stage::Race` over the
already-frozen scene rather than through `MenuStage`) and
`crate::main::session::endrace` (the flow). See
[`docs/ui/endrace-screens.md`](../../docs/ui/endrace-screens.md) for what
draws, what is left blank and why, and its own measured-vs-chosen table.
`--menu-page endrace-results`/`endrace-rewards`/`endrace-menu` captures
against the reference frames, and one live Xvfb walk confirmed `EndRace
Results` draws correctly over a real, just-finished race. What is left:

- **The loyalty row now draws for the two live-verified branches**
  (Time Trial/Speed Lap) - `Race_ComputeLoyaltyAward`/`Loyalty_AccumulateTotal`
  (see Open above) unblocked this once the law landed in main. The
  Race/Zone/Elimination branches are decompiled but not live-verified, drawn
  under the same law with that caveat stated on screen and in the docs page.
- **The trophy model is not wired.** `oag_game::preview::model` can load an
  arbitrary `.vex` the same way a picker's own ship preview does, and
  `TrophyPanel`'s own `OriginX="145.0" OriginY="60.0"` is a real number
  `EndRace_Definition.xml` authors - but `Mode3D`/`Model` widgets are not yet
  collected by `oag_ui::screen::Screens::collect_widgets`, and no capture
  this project holds is a medal-earning run to check the result against
  anyway. See `docs/ui/endrace-screens.md`'s own Open section.
- **A live-walk discrepancy is unresolved**: after a campaign-launched race,
  a single confirm at `EndRace Results` landed on the ordinary `Main Menu`
  rather than stopping at `Rewards` or reaching `Menu`'s own
  `RETURN TO GRID` with the campaign context intact - consistent with
  `RaceStage::campaign_cell` reading `None` at the point the EndRace flow was
  built, despite the race having launched through `Cell Selection`. Not
  root-caused (each repro cycle costs a multi-minute autopiloted race); see
  `docs/ui/endrace-screens.md`'s Open section for the exact next diagnostic
  step (two `log::info!` calls and one repeat of the live walk).
- Decide, once `data/images/pulse-psp-usa.chd`'s save/ghost system (if any)
  exists in this project, whether `EndRace Menu`'s ghost comparison is worth
  reproducing at all versus staying results-only, the way `RECORDS`'s own
  `fe-menu-definitions.md` section already chose to drop TAG/TEAM columns
  this project's schema cannot back honestly.
