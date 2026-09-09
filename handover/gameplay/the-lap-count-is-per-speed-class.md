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
arrives as `oag_tables::handling::SpeedClass`, which `oag-race` already depends
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

**2026-09-09, second pass: Time Trial gets the identical per-class table, live
rather than census-only, and Speed Lap's `7`-vs-`None` conflict is resolved in
`None`'s favour.** Both required a PPSSPP session (`pulse-psp-usa.chd`,
Talon's Junction White, Xvfb-headless, `scripts/psp-drive.py` plus a one-off
driving script for the class/race-type selectors it doesn't parameterise) -
full method in `/home/topaxi/.cache/oag/reports/laps.md`.

- **Time Trial: `Lap 1 of 3`/`4`/`4`/`5` on Venom/Flash/Rapier/Phantom, one
  Custom Race per class, no campaign cell in play.** Matches the census
  exactly at all four rungs - a stronger result than Single Race's own, which
  only has the census plus a single Venom live point. `Mode::TIME_TRIAL_LAPS_BY_CLASS`
  replaces the flat `TIME_TRIAL_LAPS`, confidence 88.
- **Speed Lap: the `7` is real outside the campaign too - a Custom Race Venom
  speed lap also reads `Lap 1 of 7` - but it still does not end the race.**
  The decisive evidence is the in-race **pause menu**: Speed Lap's carries a
  seventh row, `END SESSION`, that Time Trial's otherwise-identical six-row
  menu does not (both screenshotted). A mode with a dedicated way to
  deliberately conclude an open run is a mode that doesn't conclude one on its
  own - agreeing with `MSC_EVENT_SL`'s own "never ends, escape leaves." Driving
  past lap 7 to watch it not-end directly was attempted and abandoned: an
  open-loop scripted replay (`verification/scenarios/talons-junction-clean-lap.inputs`,
  repeated) did not complete even one lap in nine real-time minutes - the same
  drift `docs/tools/oag-trace.md` already names as fatal past one lap - so the
  pause-menu reading, not a lap-8 crossing, is what this rests on.
  `Mode::SpeedLap.laps_target` is unchanged (`None`); only its doc comment and
  `race-modes.md` changed. Zone's `0` is still untouched.

## Open

- ~~**Time trial still returns a flat `3`.**~~ **Settled 2026-09-09, live.**
  See "What is now established" below - `Mode::TIME_TRIAL_LAPS_BY_CLASS`
  replaces the flat constant.
- ~~**Speed Lap's census `7` against its recovered `None` is a real conflict.**~~
  **Settled 2026-09-09, live, and the answer is not the hypothesis this file
  used to record.** See below - `None` stays, and the `7` is real but cosmetic.
  Zone's `0` is still untouched.
- **The table is a fallback, not the authority.** A campaign race should read
  its own cell's `laps` - `oag_tables::race_campaign::Cell` already parses it -
  and that value should win over the array. Nothing selects a cell yet, which is
  the same blocker `Mode::ELIMINATOR_KILL_TARGET_DEFAULT` carries and the same
  launch-path tracing the race-campaign thread names as its own next step.
- ~~**`docs/formats/hd-hud.md`'s prose is now stale**, around its
  `MAX_RECORDED_LAPS` justification: it says "this crate's lap targets" are both
  `3`.~~ **Fixed in `d7c95972` (2026-09-09, before this pass started).** The
  paragraph now names `Mode::SINGLE_RACE_LAPS_BY_CLASS` and the Phantom
  fifth-lap case directly; this bullet and Next Step 4 below were the only
  things still stale.

## Next Steps

- ~~**Confirm a time trial's lap count above Venom.**~~ Done, 2026-09-09: see
  "What is now established".
- ~~**Reconcile Speed Lap's `7`.**~~ Done, 2026-09-09, and not the hypothesis
  this file used to name (campaign-bounded vs Custom-Race-unbounded was
  falsified - both are `7`; see "What is now established" for the real
  discriminator). Whether the race would actually resume or stop crossing lap
  8 is still not directly observed - the pause-menu evidence is what stands in
  for it - so a closed-loop drive past lap 7 would still be worth doing if
  anyone builds one for this circuit.
- **Retire the table when a cell can be raced.** Trace the campaign launch path
  first (see the race-campaign thread), then have the cell's own `laps` override
  `SINGLE_RACE_LAPS_BY_CLASS`.
- ~~**Fix `hd-hud.md`'s stale sentence** about both lap targets being `3`.~~ Done
  in `d7c95972`, before this pass started.
