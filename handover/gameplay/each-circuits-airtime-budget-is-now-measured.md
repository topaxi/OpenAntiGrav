# Each circuit's airtime budget is now measured; Moa Therma confirms the maintainer

2026-09-07. The maintainer's own framing, on why loosening `roll_caution` (the
barrel roll's per-tier caution multiplier, in `oag_ai::Difficulty`) is
premature: "Depends on tracks as well, there is tracks with few or no
jumps/airtime (for example moa therma is very difficult if possible at all to
squeeze in a barrel roll)." No measurement of that existed - a prior pass
(`docs/gameplay/ai.md`, "Does it fire on a full grid?") had only measured
`09_Track` in isolation. This thread is the table across all 24
circuit-directions, and it settles the Moa Therma question decisively.

New file: `crates/game/tests/airtime_budget_ground_truth.rs`
(measurement-only, `OAG_SWEEP`-gated, `#[ignore]`d - never runs in CI or
`just test-data`; three tests, all print rather than assert). Full data and
narrative: scratch report at
`airtime-budget.md`
(not committed - a scratch path, per this project's own convention for a
long finding). No axis was tuned this pass - `roll_caution`, `roll_airtime`,
`roll_chance`, `roll_floor` all untouched. That decision is the maintainer's,
and needed this table first.

## What was measured

One craft alone on a circuit (everyone else switched off, same isolation
`ai_roll_ground_truth.rs`'s own benchmark uses), 18,000 ticks, `BALANCED`
pilot forced and tempered per tier, `VENOM` class unless noted. All 24
circuit-directions (12 circuits x forward/reversed - the disc's catalogue has
24 race entries, not sixteen). For each: how many airborne-window episodes,
and their min/median/max duration, with windows contaminated by a respawn
(relaunch drop, or a fall that never lands near a jump) filtered out - see the
test file's module docs for the exact two cutoffs, both **chosen, not
measured**. A row where the filter discarded at least as much as it kept is
marked NOT MEASURABLE rather than given a distribution - that is a circuit's
own recovery pathology (a craft that cannot get round it at that tier at all),
not a fact about its jumps.

## The headline findings

1. **Moa Therma (`03_Track`, confirmed against `docs/formats/skycube.md`,
   `docs/formats/psp-audio.md`'s `MOA_THERMA_ENV.bnk` mapping and
   `docs/formats/track.md`, independently) offers essentially nothing.**
   Longest airborne window across every tier, both directions (`03_Track`
   and its reverse `19_Track`), at both `VENOM` and `PHANTOM` (fastest class,
   checked specifically because the maintainer's play experience is a human
   on an unknown class): **11 ticks / 0.18s**. Against the single most
   permissive threshold on the disc - Ace tier, the least-demanding built-in
   pilot's own floor, 24 ticks / 0.40s - that is 46% of the minimum any craft
   at any tier would need. **Agrees with the maintainer's play experience
   precisely, and the stronger reading of it**: not "very difficult", flatly
   impossible, in the simulation as it stands.
2. **`13_Track`/`29_Track` (its reverse) is a second circuit this measurement
   found to be just as roll-proof** - max 18-21 ticks at every tier where
   measurable - and nobody had said so before this pass.
3. **`09_Track` locking out Novice/Skilled/Elite (the finding
   `docs/gameplay/ai.md`'s "Does it fire on a full grid?" section documents)
   turns out to be a fact about one direction, not the circuit.** Its reverse,
   `25_Track`, clears every tier's threshold cleanly (103-104 ticks max,
   `resp 0`, raced clean) - including Novice's 58-tick floor, which almost
   nothing else on the disc clears. `25_Track` is the strongest **clean**
   candidate for a lower-tier roll on the whole disc. (`17_Track` shows
   longer maxima still - 174-176 ticks - but both its rows sit on
   respawn-touched races and its Novice/Elite rows are unmeasurable outright,
   so it is reported, not counted; see Open below.)
4. This harness independently reproduces the prior member's `09_Track`
   measurement: 0.47-0.60s max here versus their "every jump on `09_Track`
   lands in 0.47 to 0.62 seconds" - same circuit, same order of magnitude,
   two independent runs.

Full per-circuit table, the threshold derivations, and the caveated/short-race
rows are in the scratch report above - it is long (24 circuit-directions x 4
tiers) and belongs there rather than duplicated here.

## Open

- `17_Track` (01_Track reversed) shows the two longest maxima in the entire
  sweep - 174 ticks/2.90s at Skilled, 176 ticks/2.93s at Ace - and would
  clear every threshold including Novice's. Not counted as a candidate: both
  rows sit on respawn-touched races (2 and 9 respawns; Ace's `clean_laps 0`),
  and its Novice and Elite rows are NOT MEASURABLE outright. Wants a clean
  re-run before anyone designs against it, same as `05_Track`/novice below.
- The table is `BALANCED`-pilot only. A pilot's own line (`line_bias`,
  `inside`, `wander`) changes how it enters a jump, so `AGGRESSIVE`,
  `PASSIVE` and `SHY` were not re-swept across all 24 circuit-directions x 4
  tiers (4x the wall clock for a table this size already). Not expected to
  flip the Moa Therma or `13_Track`/`29_Track` findings (both are near-zero
  by a wide margin), but could matter for the borderline rows (`30_Track`,
  `14_Track`, `10_Track` sit just under Novice's floor at `BALANCED`).
- `19_Track` (Moa Therma reversed) was not re-measured at `PHANTOM` -
  `03_Track`'s own result made it low-priority. Flagged, not chased.
- `05_Track`/novice (73 ticks, clears Novice's floor) ran a short,
  respawn-touched race (`laps 2`, one respawn) rather than a clean one - the
  second-best Novice candidate on the disc needs a clean re-run before
  anyone relies on it.
- `13_Track`/Novice and `29_Track`/Novice could not be measured at all - the
  same pre-existing `grip_believed` pathology `docs/gameplay/ai.md` already
  documents for `13_Track` (a Novice-tuned craft cannot get round the
  authored jump at `grip_believed` 0.30). `29_Track` having the identical
  pathology was not previously documented.
- `HANDOVER.md`'s "Open threads" index has no line for this file yet - adding
  one is outside this pass's grant (measurement-only, `handover/` files
  only); the orchestrator should add it when this thread's file is filed.

## Gates (confirmed 2026-09-07)

- `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`:
  passed, unaffected - all twelve circuits clean, `01_Track`'s one respawn
  still at index 794.
- `cargo nextest run -p oag-game airtime_budget` (no `--run-ignored`): 0
  tests run - every test in the new file is `#[ignore]`d.
- Full `just`: **EXIT:0** - 3220 tests passed, 671 skipped, every check OK.
- `OAG_REQUIRE_GAME_DATA=1 just test-data`: **EXIT:100, exactly 5 failures**
  - the *current* baseline on `main` (revised since this thread started, to
  include `lap_times_ground_truth` alongside `opponent_weapons_ground_truth`,
  both other members' in-flight work): `lap_times_ground_truth`,
  `opponent_weapons_ground_truth`, `pure_dlc_ground_truth` x2,
  `shuriken_ground_truth`. No 6th failure. All three new tests pass as fast
  no-ops (`OAG_SWEEP` unset in this run).

## Next Steps

1. The maintainer reads the scratch table and decides what, if anything,
   `roll_caution`/`roll_airtime` should do differently per circuit - this
   thread's whole purpose was to make that a comparison against real numbers
   rather than a guess. Not this pass's call.
2. If a per-circuit or per-direction adjustment is wanted, `25_Track` (clean,
   clears every tier) and Moa Therma / `13_Track`+`29_Track` (clears nothing,
   any tier) are the two ends of the range to design against.
3. A clean re-run of `05_Track`/novice, and the `AGGRESSIVE`-pilot sweep
   noted above, would tighten the borderline rows if the maintainer's next
   step turns out to depend on them.

## From the HANDOVER.md index (moved 2026-09-25)

the AI barrel roll is gated on a minimum airborne duration, and on a full grid only **Ace** ever armed one. The cause is the track, not the propensity: **Moa Therma (`03_Track`) offers a longest airborne window of 11 ticks / 0.18 s** against Ace+`AGGRESSIVE`'s floor of 24 t / 0.40 s - 46 % of the minimum, in either direction, either speed class, any tier. That **agrees with the maintainer's own play report** ("very difficult if possible at all to squeeze in a barrel roll") and rules out a flight-model gap. The lockout is **directional, not a circuit property**: `09_Track` clears 31-36 t while its own reverse `25_Track` clears 103-104 t. Eleven of twenty-four circuit-directions clear nothing at any tier (03/19, 04/20, 07/23, 13/29, 16/32), and `25_Track` is the sole clean Novice candidate. **Nothing was tuned** - relaxing `roll_caution` is one constant across sixteen circuits with wildly different budgets, so it would make Ace roll constantly on a jump-heavy track and still change nothing on a flat one. The table is `crates/game/tests/airtime_budget_ground_truth.rs`; the design decision is the maintainer's
