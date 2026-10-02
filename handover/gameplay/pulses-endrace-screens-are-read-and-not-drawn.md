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
- ~~**The per-lap table's third column**~~ **Closed 2026-09-30**: it is the number of
  speedup pads the player entered on that lap, headed by the `boostimg` icon (which the
  earlier passes had misread as hidden - bit `0x4` is the *visible* bit). A live write
  watchpoint on `craft+0x994 + lap*0x10` caught its only writer at `0x0884910c` inside
  `Ship_ApplySpeedupPad`; this build now tallies it on the same new-pad edge
  (`crate::race::RunStats::boosts_by_lap`) and draws it. **Still open**: lap 1 reads `6`
  against `9`-`10` on the laps after it in both captures, unexplained (how `craft+0xac8`
  moves at the start line is not pinned), and this build's own per-lap counts have not been
  compared with the original's on the same circuit.
- ~~**The Tournament/Zone/Elimination/split-screen variants of `EndRace
  Results`'s own populate**~~ **Closed 2026-09-30**: all four are read, the
  Eliminator table and the sort behind it are confirmed on a live PPSSPP frame, and
  both Zone and Eliminator now draw (`oag_ui::endrace::modes`). `FUN_088d9588` is the
  *network-play* table (`g_game_mode > 0xd`), not split-screen. `boostimg` is shown by
  the lap table only; `Line2`..`Line8` are never filled in any mode.
  **Still open inside them**: Zone's `Laps cleared` and `Perfect laps` (what steps
  `craft+0x911` bit 0 and `craft+0x860 & 0x200000`; both draw a label and no value);
  who writes the `+0x140` word that would print `DNF` on the Eliminator table (it read
  `0` on all eight live records); and a **Zone live frame** - Zone is greyed on a
  fresh profile and a forced `g_game_mode = 6` hangs the loader, so that table is
  decompile-only.
- **The generic confirm-swallow-on-a-locked-tile mechanism** - found while
  reading these screens (`StateMachine_EvaluateRedirect`,
  `0x088c8798`, ruled out as the predicate itself), and narrowed further
  2026-09-14: the real `Cell Selection` dispatcher (`ConfirmButton_Update`,
  `0x088c8e88`) is found and cleared too, pinning the gate to input
  consumption upstream of it - is a `race-campaign.md` thread, not this
  one; see
  [`the-race-campaign-is-authored-shape-not-content.md`](the-race-campaign-is-authored-shape-not-content.md)'s
  own `Open`/`Next Steps` for where that stands.
- ~~`MedalImg` under an earned trophy~~ **Settled 2026-09-30 (80)**: a live gold run shows no static glyph under the trophy (this build already hides it), and the original's trophy spins (about a 6 s period) where this build holds frame 0; the spin is the remaining gap. Third-column note from the same run: a one-lap race read `6`, so lap 1 reads 6 on all four captures; this build's autopilot counts 7/7/7 on the same circuit.
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

- **The loyalty row now draws, including a persisted per-team total**
  (`records.toml`'s new `[[loyalty]]` table, `Store::record_loyalty`/
  `loyalty_total`) - `Race_ComputeLoyaltyAward`/`Loyalty_AccumulateTotal`
  (see Open above) unblocked this once the law landed in main.
  `SingleRace`/`Zone`/`Eliminator` are decompiled but not independently
  live-verified, drawn under the same law with that caveat stated in prose
  (`docs/ui/endrace-screens.md`), not on screen. "Perfect lap"/"perfect
  zone" counts and the campaign's own `easy`/`medium`/`hard` difficulty
  multiplier are always `0`/unapplied - this project keeps no running tally
  for the first two and has no honest mapping from its own four-tier
  `[ai] difficulty` to the original's three-tier scale for the third.
  ~~**A real, unresolved mismatch**: `loyaltybar`'s fill width~~ **Closed
  2026-10-02 (88)**: the bar is `total * 0.00124` *pixels* wide (cropped), not a
  fraction, so a total of `90` shows only `loyaltybg`, as the reference does -
  see `docs/ui/endrace-screens.md`. **Still open**: a PPSSPP poke of the team
  total to `50000` to pin the slope past the near-zero point, and the bar/`loyaltynum`
  being hidden until the ticker finishes in the original (drawn from frame one here).
- ~~**The trophy model is not wired.**~~ **Wired 2026-09-28**: `Screen::models`
  collects `<Mode3D><Model>`, `oag_game::endrace::Trophy` decodes the three,
  and `EndRaceRuntime` draws the earned one with the disc's `Mode3D` camera,
  which lands it inside `MedalImg`'s square (cross-checked by
  `endrace_trophy_ground_truth`). Live: a bronze campaign race drew it.
  **Still open**: held at its first frame (`StartPaused="yes"`, chosen - the
  original's animation is unmeasured), `Enabletransition="2.0"` not applied,
  and no capture of the *original* earning a medal. The old text:
  `oag_game::preview::model` can load an
  arbitrary `.vex` the same way a picker's own ship preview does, and
  `TrophyPanel`'s own `OriginX="145.0" OriginY="60.0"` is a real number
  `EndRace_Definition.xml` authors - but `Mode3D`/`Model` widgets are not yet
  collected by `oag_ui::screen::Screens::collect_widgets`, and no capture
  this project holds is a medal-earning run to check the result against
  anyway. See `docs/ui/endrace-screens.md`'s own Open section.
- ~~**A live-walk discrepancy is unresolved**~~ **Fixed 2026-09-28**: not
  `campaign_cell` - the built-in results table's own Cross/Start dismiss in
  `Session::frame` ran before `tick_endrace` and escaped to `Main Menu`. See
  `docs/ui/endrace-screens.md`'s Open section. ~~New, unchased: the live
  walk's `EndRace Rewards` drew no loyalty row on a campaign launch~~
  **Fixed 2026-09-28**: the campaign launch never copied the picked team
  into `race::Options::team`; now `Session::launch_campaign_race` does, and
  a live Pulse walk drew `FEISAR LOYALTY 45 POINTS` and banked it. Same
  Open section.
- **2026-09-30: Zone and Eliminator results draw**, and the lap table is corrected -
  only the rows a mode fills show, `boostimg` is the third column's header icon, the
  highlight sits at `93 + 20 * (row - 1)`, and per-craft team ids resolve to display
  names (the folder-id/display-name gap `tournament.md` recorded is closed for both
  per-craft tables). Judged live against the original's Eliminator frame; the Zone
  numbers on `--menu-page endrace-results-zone` are chosen. **A shared-parser quirk was
  routed around, not fixed**: `Screens::collect_widgets` places a colour-only
  `<Image OffsetY=...>` wrapper without its own `OffsetY` (eight `tablebg` backings
  landed on the top of the screen). If another screen draws a src-less wrapper with an
  offset it has the same fault. **The Eliminator HUD is a separate gap seen on the same
  frame**: the original draws `KILLS (5)` and a per-team kill list top-right, which
  this build's `Elimination_HUD.xml` draw does not (the widgets are unread on every
  layout). Nothing of it is wired.
- Decide, once `data/images/pulse-psp-usa.chd`'s save/ghost system (if any)
  exists in this project, whether `EndRace Menu`'s ghost comparison is worth
  reproducing at all versus staying results-only, the way `RECORDS`'s own
  `fe-menu-definitions.md` section already chose to drop TAG/TEAM columns
  this project's schema cannot back honestly.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-14. A campaign race in the original ends in three authored states this build never had: `EndRace Results` (per-lap table), `EndRace Rewards` (a 3D trophy model unpaused off `Cell_EvaluateMedal`'s ordinal - `gold`/`silver`/`bronze.vex`, a dash glyph for none - plus per-team loyalty with a bar) and `EndRace Menu` (`RETURN TO GRID` default, ghost record). `EndRace_Definition.xml` is read widget by widget ([endrace-screens.md](../../docs/formats/endrace-screens.md)) and the fill code decompiled ([ghidra page](../../docs/ghidra/functions/psp-pulse-usa/endrace-screens.md)). Open: the loyalty writer (30 points a lap fits one observation, capped at 70), the pennant column's meaning, four sibling populate functions (Tournament/Zone/Elimination/split-screen). Next: draw the three states off the XML in place of our own results table
