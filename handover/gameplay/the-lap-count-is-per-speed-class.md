# The lap count is per speed class, and three of four classes raced short

2026-09-09. `Mode::SINGLE_RACE_LAPS = 3` was the weakest number in
`crates/race/src/mode.rs` and said so in its own doc comment: a guess carried
over from the time trial because a race has to end somewhere. It was right for
Venom and wrong for the other three rungs, so Flash, Rapier and Phantom single
races all finished one or two laps early.

Read, in this order:

- [`docs/gameplay/race-modes.md`](../../docs/gameplay/race-modes.md)'s "Single
  race" section - the finding, the table, and what the plumbing now looks like.
- [`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`](../../docs/ghidra/functions/psp-pulse-usa/race-campaign.md)
  - the census itself, and the parser offsets behind it.

## What is now established

**3 Venom, 4 Flash, 4 Rapier, 5 Phantom.** Confidence **90**, a flat census over
the 236 authored `PI_Cell` records in `Data\Plugins\grids\grid_00.xml` ..
`grid_15.xml`: every cell carries a `laps` attribute and across all 236 it is
exactly those four values, no exception.

**The plumbing exists now, which is what this pass added.**
`Mode::laps_target` takes a speed class and indexes
`Mode::SINGLE_RACE_LAPS_BY_CLASS`; `RaceState::new(mode, class)` takes it too,
so a construction site has to answer rather than defaulting silently. The class
arrives as `oag_formats::handling::SpeedClass`, which `oag-race` already depends
on - `oag-title` was deliberately **not** added, because it carries class
*names* (a ladder's length is per-title measured data) and resolving a name is
not a race-rules crate's job. The resolution happens one layer out, in
`oag_game::race::Race::start`, which already carries the name.

**Venom is the default class and its measured count is 3**, which is what the
flat constant already produced, so the committed determinism reference did not
move and was not touched.

**A Phantom race's fifth lap is counted but its split is not recorded.**
`MAX_RECORDED_LAPS` stayed `4` deliberately - four is what the disc authors
(HD/Fury's `HUD_lap_times.xml`), widening it would invent a row no measured
title has, and it would change `World`'s size and so the state hash to store a
number nothing can display. `crates/race/src/state.rs`'s doc comment now states
that absence plainly instead of claiming every bounded mode fits.

## Open

- **Time trial still returns a flat `3`, and the census says it should be
  per-class too.** The campaign's 47 `Time Trial` cells carry the same 3/4/4/5
  table. It was left alone because `Mode::TIME_TRIAL_LAPS` has independent live
  evidence (the original's counter reading `Lap 1 of 3`) and the two are
  *consistent* - Venom is the default class and its census value is 3 - rather
  than contradictory. Nothing here is known to be wrong; what is missing is a
  live time trial in a class above Venom.
- **Speed Lap's census `7` against its recovered `None` is a real conflict and
  is untouched.** Same for Zone's `0`. Both readings are sourced and they
  disagree; `race-modes.md`'s Speed lap section records the tension and the
  likeliest (untested) shape.
- **The table is a fallback, not the authority.** A campaign race should read
  its own cell's `laps` - `oag_formats::race_campaign::Cell` already parses it -
  and that value should win over the array. Nothing selects a cell yet, which is
  the same blocker `Mode::ELIMINATOR_KILL_TARGET_DEFAULT` carries and the same
  launch-path tracing the race-campaign thread names as its own next step.
- **`docs/formats/hd-hud.md`'s prose is now stale**, around its
  `MAX_RECORDED_LAPS` justification: it says "this crate's lap targets" are both
  `3`. The constant's *reasoning* is unaffected (four rows is what the disc
  authors), only the sentence about the targets. Not edited here - it was
  outside this pass's lane.

## Next Steps

- **Confirm a time trial's lap count above Venom.** One run of the original in
  Flash or Phantom and one screenshot of the lap counter settles whether
  `TIME_TRIAL_LAPS` joins the per-class table. `Mode::laps_target` already takes
  the class, so this is one match arm.
- **Reconcile Speed Lap's `7`.** The hypothesis worth testing first is that a
  campaign speed lap is a bounded 7-lap event while a Custom Race speed lap is
  unbounded. Neither half has been checked.
- **Retire the table when a cell can be raced.** Trace the campaign launch path
  first (see the race-campaign thread), then have the cell's own `laps` override
  `SINGLE_RACE_LAPS_BY_CLASS`.
- **Fix `hd-hud.md`'s stale sentence** about both lap targets being `3`.
