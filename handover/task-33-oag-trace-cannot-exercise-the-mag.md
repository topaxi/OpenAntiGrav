# Task #33: `oag-trace` cannot exercise the mag-lock hold

**The locator plumbing landed.** `replay`, `drive`, `drive_with` and
`plan::to_gate` all take an `Option<&[oag_formats::track::Sample]>` now,
located fresh every tick from the ship's own position - `crate::replay::locate`,
the same "nearest table entry and its successor" pair
`oag_game::race::Race::tick` reads for the player, resampled independently
because `oag-trace` cannot depend on `oag-game` (rule 2,
`scripts/check-dependency-rules.py`). `oag-trace run`/`drive`/`plan` all pass
their already-loaded spline through; `None` (no `--source`, so no disc to read
a `.vex` from) reproduces the old zero-blend behaviour exactly.
`crates/trace/src/replay/tests.rs` pins the mechanism itself with a synthetic
magstrip fixture: `a_locator_lets_a_driven_run_reach_the_magstrip_hold` shows
the blend ramping over a tagged floor once a locator is supplied,
`no_locator_leaves_the_hold_at_zero_even_over_a_magstrip` shows it staying at
zero without one. A real-disc smoke run (`oag-trace drive` against
`pulse-psp-usa.chd`'s default track) confirms the plumbing compiles and runs
against the actual format, not just the fixture.

**What is still open is the measurement, not the wiring.** Nobody has run a
real lap capture through the inverted section with a locator attached yet -
`data/traces/` has no Talon's Junction capture in this checkout to try it
against - so the hold's effect on a real trajectory through a magstrip is
still unmeasured by this tool. `crates/game/tests/maglock_ground_truth.rs`
remains the one place that measures it, one tick at a time.

## Open

- The hold's effect on a real lap through Talon's Junction's inverted section
  is still unmeasured by `oag-trace` - the plumbing exists but nobody has run
  a capture with it yet (`data/traces/talons-junction-*.csv` was absent from
  this checkout; regenerate from `verification/scenarios/talons-junction-*.inputs`
  per `HANDOVER.md`'s note on derived data)
- The residual left after the hold's 49.5 % may come from locator fidelity -
  our 4-per-segment resampled spline may not match the original's evaluated
  curve. Now that a comparison run can carry a locator at all, this is
  measurable rather than blocked, but nobody has taken the measurement.

## Next Steps

- Run `oag-trace run data/traces/talons-junction-time-trial-lap.csv --source
  <disc> --track "Data\Environments\16_Track\track.vex"` (regenerating the
  capture first if needed) and check whether the simulated trajectory through
  the inverted section changes now that the hold can fire - the divergence
  before/after the locator was added is the measurement this task was blocked
  on.
