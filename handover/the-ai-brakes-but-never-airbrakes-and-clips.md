# The AI brakes but never airbrakes, and clips walls in tight corners

2026-09-05, **reported from play by the maintainer**: *"AI and autopilot does not
seem to utilize airbrakes, and bump into walls in tight corners."* This project
treats a from-play observation as a real oracle, so the report is the evidence
that opens the thread - what is not yet established is the *cause*, and there are
at least three candidates that would each produce exactly this symptom.

**This is not a missing feature.** `the-ais-six-stage-plan-line-follower-to.md`
records airbrakes as stage one of six, landed 2026-08-11 and verified against the
disc, and `oag_ai::driver` really does compute them: `airbrakes(brake,
differential, tuning.brake_floor)` at `crates/ai/src/driver.rs:330` feeds
`airbrake_left`/`airbrake_right` on every tick. So the control path exists and is
wired. Either it is never *commanded* under the conditions a tight corner
produces, or it is commanded and does nothing.

## Open

- **Which of the three candidates below is the cause is unknown.** Nothing here
  is measured yet; the report is the only observation.

- **Candidate 1: the AI's own tuning was calibrated against a craft whose
  airbrakes do nothing.** `crates/ai/src/probe.rs`'s own doc comment says so
  outright, about the *other* fixture: `closed_loop.rs`'s default ends
  `..Handling::ZERO` and "so leaves every airbrake and brake term at zero", and
  "its regression bounds were calibrated against a craft whose airbrakes do
  nothing". If the line-follower's gains were fitted on that fixture, they are
  fitted to a craft that cannot airbrake, and would not have learned to need
  them. `probe.rs` exists precisely because "a determinism gate that leaves a
  whole control path at zero hashes a path nobody drives" - the same sentence is
  a warning about the tuning.

- **Candidate 2: airbrakes are only reachable through the brake path, and the
  brake only opens past an overspeed margin.** `driver.rs`'s `airbrakes()` takes
  `limit = if brake > 0.0 { floor } else { 0.0 }`, so with the brake off, one
  side rises from nothing - yaw authority, no deceleration. And the brake itself
  only climbs past `tuning.brake_margin` (`driver.rs` ~line 790). A corner tight
  enough to need a differential airbrake *before* the craft is measurably
  overspeed would therefore get no airbrake at all. Whether that is what a tight
  corner actually produces is unmeasured.

- **Candidate 3: `line_following_tests.rs` pins airbrakes at exactly zero.**
  `crates/ai/src/driver/tests/line_following_tests.rs:31-32` asserts
  `controls.airbrake_left == 0.0` and `airbrake_right == 0.0`. That may be
  correct for the straight-line case it covers - or it may be a test that froze
  the current behaviour rather than the intended one. Check what case it
  actually drives before treating it as a constraint.

- **Whether the autopilot shares the AI's driver at all is unconfirmed.**
  `Race::autopilot` is a bool on the race (`crates/game/src/race.rs:817`) and the
  maintainer reports the same symptom for both, which is consistent with a shared
  driver - but consistent is not confirmed, and if the autopilot has its own
  simpler follower then it needs its own answer.

- **What the original does in the same corner is not established here.** The
  cornering ground truth (`docs/physics/cornering-ground-truth.md`,
  `scripts/trace-cornering.py`) reads *grounded* behaviour; whether it captures
  an original AI craft's airbrake use through a tight corner is unchecked.

## Next Steps

1. **Reproduce it headlessly before reading any code.** `just play --race
   --autopilot --screenshot` plus a trace is the cheapest instrument, and
   `--no-audio` (landed 2026-09-05) lets a headless run reach later frames than
   it used to. Log `airbrake_left`/`airbrake_right`, `brake` and the cornering
   overspeed per tick through a known-tight corner and see which of the three
   candidates the numbers pick. **Do this first** - all three are plausible from
   a reading alone, and only a trace separates them.
2. Pick the tightest real corner to measure against rather than a generic lap.
   `race_ground_truth::the_racing_line_has_track_under_it_where_it_is_known_to`
   already classifies circuits by how well the authored line fits; Talon's
   Junction is the most-used reference elsewhere in this repo.
3. Once the cause is known, fix the cause and not the symptom. Raising a gain
   until the craft stops hitting walls would be tuning against one corner on one
   circuit; the fix belongs wherever the trace says the command is lost.
4. If the answer turns out to be candidate 1, the tuning needs re-fitting against
   a craft whose airbrakes work - and that is a bigger change than it looks,
   because the AI's regression bounds and the world-hash history were both built
   on the current numbers. Split it into its own thread rather than folding it in.

## Notes

- `oag-ai` depends on `oag-core` and `oag-physics` only, deliberately - keep it
  that way.
- The simulation is determinism-bound: `f32` only, `TickClock` never the wall
  clock, seeded `Rng` only. **Any change to AI control output will move the
  committed state hash.** That is expected here rather than alarming, but it must
  be explained and the reference constants in `crates/core/src/hash.rs` must
  never be edited to make a test pass - see
  [determinism.md](../docs/architecture/determinism.md).
- Airbrake physics itself lives in `crates/physics/src/airbrake.rs`; if the trace
  shows the command is issued and nothing happens, the fault is there rather than
  in the driver.
