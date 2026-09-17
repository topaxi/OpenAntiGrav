# lane/test-tail-split: both named targets were already split

## Summary

The brief describes `ai_roll_ground_truth.rs` and `spawn_heading_ground_truth.rs`
as unsplit matrix tests needing the "chained ordering is its own pairwise links"
/ "ordered `assert_eq!` filters with its list" treatment CLAUDE.md names. Both
files are already at exactly that split, landed weeks ago on this branch's
ancestry (`ce106ad9`, confirmed an ancestor of this worktree's `main` merge
point `8999b7ae`). No test-file changes were made in this lane. What changed:
one stale comment in `.config/nextest.toml`, corrected, plus a short filterset
note added to the same file. Committed as `8a84bca6`.

## What is already true of `ai_roll_ground_truth.rs`

Six tests, one per (seed, adjacent-tier-pair) - the full list, unchanged
before and after this lane's work:

```
a_full_grid_of_ace_arms_no_fewer_rolls_than_elite_at_the_default_seed
a_full_grid_of_ace_arms_no_fewer_rolls_than_elite_at_the_held_out_seed
a_full_grid_of_elite_arms_no_fewer_rolls_than_skilled_at_the_default_seed
a_full_grid_of_elite_arms_no_fewer_rolls_than_skilled_at_the_held_out_seed
a_full_grid_of_skilled_arms_no_fewer_rolls_than_novice_at_the_default_seed
a_full_grid_of_skilled_arms_no_fewer_rolls_than_novice_at_the_held_out_seed
```

plus four more in the same file not part of the tail (`no_tier_rolls_itself_
down_to_nothing`, `an_ace_actually_rolls_somewhere_on_the_disc`,
`an_ace_pilot_rolls_more_than_a_novice_pilot_in_the_same_race`, `sweep_rolls`).
`cargo nextest list -p oag-game --run-ignored all -E 'binary(ai_roll_ground_
truth)'` reports **10 tests**, identical before and after this lane's commit
(the commit touches `.config/nextest.toml` only).

This is the exact split CLAUDE.md cites as its own worked example ("a chained
ordering is its own pairwise links (`ai_roll_ground_truth.rs`)"). The file's
own doc comment (lines 537-621) documents why: a single-tier-chain test
measuring 525s landed 2026-09-06, was split 2026-09-09 into these six, and the
history behind the split is recorded in-file, not reconstructed here.

**There is no further split available without weakening the assertion.**
`git show 6f7f7711` (2026-09-13, deleting the file's last `BASELINE` row)
already wrote the argument down:

> the assertion is a *total* over every forward circuit: a higher tier arms no
> fewer rolls than a lower one, summed. Per-circuit is a strictly stronger
> claim and not the one the AI is tuned to - a tier can legitimately roll less
> on one track and more overall - so making the circuit the test axis would
> not be a split, it would be a different test.

The file's own doc comment gives the empirical reason this sample has to stay
whole: on 2026-09-08, adding `Weapon::LeachBeam` to `oag_gameplay::pickup::
IMPLEMENTED` - a change touching no roll gate, no `oag_ai` axis, no force law -
flipped the tier ordering with per-tier totals of 0-3 armed rolls. Confirmed by
isolation (removing the line fixed it, re-adding broke it again). That is
exactly the flake a per-circuit split would reintroduce. **Verdict: leave this
file's test bodies alone.** A preserved 10-test file beats a fast file that
checks something weaker.

## What is already true of `spawn_heading_ground_truth.rs`

Six tests, unchanged before and after:

```
every_forward_hd_craft_spawns_facing_the_way_its_circuit_runs
every_reversed_hd_craft_spawns_facing_the_way_its_circuit_runs
only_the_ps2_pressing_carries_a_stale_slot
the_ps2s_stale_slot_is_corrected_to_what_the_psps_own_file_authors
the_psp_pulse_pressing_carries_no_stale_slot
the_pure_pressing_carries_no_stale_slot
```

Already split forward/reversed (the "ordered `assert_eq!` filters with its
list" pattern - both `HD_CIRCUITS` and `HD_STALE_SLOTS` are grouped
forward-then-reversed, so filtering by direction keeps the assertion exact,
per the file's own doc comment on `hd_craft_spawn_facing_their_circuit`) and
per-source (psp/pure/ps2, "one test per source", each source's own circuit
count and stale list). `docs/architecture/workspace-layout.md`'s own
measurement table records both splits already landing: `147s -> 81s` for the
direction split, `106s -> 37s` for the source split. Nothing to do here
either; the brief's 243.4s/145.6s figures are consistent with this file
running the same code under today's contention, not a sign the split never
happened.

## The DO-NOT-TOUCH figures in the brief do not match this branch

The brief lists `ps2_source_ground_truth::an_uncapped_transcode_still_reports_
a_total_to_divide_by` (471s) as "the sole `BASELINE` entry, on purpose" and
`ram_ground_truth::a_ram_fires_only_where_there_is_room...` (468s) as a second
one. Checked directly:

- `scripts/check-test-budget.py`'s `BASELINE` dict is **empty** right now -
  confirmed by reading the file, not by inference. Its own comments record
  why: the ai_roll row (added 2026-09-09 at 305s) was deleted 2026-09-13 after
  two idle-machine `test-data` runs measured it at 158-165s, clear of
  `CEILING`. The ps2_source comment says outright: "measured 160s on
  2026-09-09 and needs no row" - it never had one.
- `rg -n "471|468"` across the repo (excluding `Cargo.lock` and unrelated
  `handover/` hits) returns **nothing**. These numbers do not appear anywhere
  in this codebase's docs, scripts, or test files.
- `.config/nextest.toml` itself had a stale comment claiming ps2_source's test
  was "the only entry in `check-test-budget.py`'s `BASELINE`" - true on
  2026-09-09, false since 2026-09-13. Fixed in this lane's commit.

I did not touch either `ps2_source_ground_truth.rs` or `ram_ground_truth.rs`,
per the brief's own instruction, and there is independently no cause to: their
own doc comments still describe the same load-bearing-transcode and
ratio-bound reasons `docs/architecture/workspace-layout.md`'s table already
credits (`ps2_source` is the wall-clock floor by design; `ram` was *merged*,
not split, because its bound is a ratio and slicing the sample rebuilt a flake
- see `RACES`'s own doc comment, 2026-09-08 incident, 6 races could not tell
9% from 17%).

**What this means for the brief's timing table.** Today's isolated measurement
of `ai_roll_ground_truth` (started at load average ~22, climbed past 40 as
another lane's full-workspace `test-data` gate ran concurrently) was killed
before completing cleanly - the same 6.6x load-swing the brief itself warns
about was visibly in effect (build and link contention alone stretched a
normally ~10s `cargo nextest list` compile past 60s). No isolated-duration
table is reported here because no clean sample was taken; see "Open question"
below.

## Open question, not resolved in this pass

Between 2026-09-13 (ai_roll measured 158-165s idle, ps2_source 160s) and today
(2026-09-17), substantial gameplay-simulation code landed on this branch's
ancestry - weapons (rocket, mine, plasma, disruptor, LeachBeam slowdown),
eliminator scoring, wrecked-opponent respawn, and a multiplayer per-slot input
snapshot change to `Race::tick`. Any of these could have genuinely raised the
per-tick CPU cost of every ground-truth test that runs a simulated race,
independent of load contention. This lane could not tell contention-inflation
apart from a genuine regression without an isolated reading, and no idle
window was available in this session (a concurrent lane was gating the merged
`main` throughout). **This needs a clean, single-lane measurement on an idle
machine to resolve** - not a test-file change, since neither target file has
a valid further split. If the isolated numbers come back near 158-165s as in
2026-09-13, this is a closed, negative-but-complete result. If they come back
materially higher, that is a "find what grew" finding pointing at
`crates/gameplay`/`crates/ai`/`crates/physics` production code this lane does
not own, not a test-splitting task.

## What changed on this branch

- `.config/nextest.toml`: corrected the stale "only entry in `BASELINE`"
  comment (nothing was there to be the only entry, and nothing has been since
  2026-09-13); added a short note recording nextest's own filterset support
  (`-E 'rdeps(oag-rcs)'`, `-E 'binary(...)'`) for local iteration, since
  neither this file nor `docs/architecture/workspace-layout.md` recorded it
  and a question came up about whether `cargo-test-changed`/`cargo-difftests`
  were needed (they are not - nextest already does selection, and selection
  cannot help the gate itself since it is tail-bound, not throughput-bound).
  Committed as `8a84bca6`.
- No changes to `crates/game/tests/ai_roll_ground_truth.rs`,
  `crates/game/tests/spawn_heading_ground_truth.rs`,
  `crates/game/tests/ps2_source_ground_truth.rs`, or
  `crates/game/tests/ram_ground_truth.rs`.
- No `BASELINE` row added anywhere. No ceiling changed
  (`CEILING = 300.0`, `SUITE_CEILING = 450.0`, both verified unchanged by this
  lane in `scripts/check-test-budget.py`).

## Gate status

Not run to completion in this session: another lane held the machine at load
averages of 20-40 throughout (`OAG_REQUIRE_GAME_DATA=1 just test-data` on the
merged `main`, per the team lead). A full `just` / `just test-data` run under
that contention would not be evidence of anything either way, per this
project's own documented history of contended runs being misread. `cargo
nextest run -p oag-game` (non-disc tests only, no `--run-ignored`) passed
963/963 with 467 skipped, confirming nothing in `oag-game`'s ordinary test
suite broke. `cargo fmt --all --check` is clean.

`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_
round` was not touched and was not run to completion in this session (it is
disc-backed and `#[ignore]`d); no reason to expect it changed, since no
production code was touched.
