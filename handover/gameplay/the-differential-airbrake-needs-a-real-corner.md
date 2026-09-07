# The differential airbrake needs a real-corner threshold, not a straight/not-straight one

Closes `the-ai-brakes-but-never-airbrakes-and-clips.md`: which of three candidates
caused "AI and autopilot does not seem to utilize airbrakes, and bump into walls in
tight corners" is now measured and answered. What is left is a fix, and it is a
tuning question rather than a logic one - which is why it is its own thread rather
than folded into the RE session that found it.

## What the trace established

Reproduced headlessly: `race::Options` on `oag_pulse::race::DEFAULT_TRACK` (Talon's
Junction), VENOM/Ace, one craft solo, two laps (~5,336 ticks), instrumented with a
temporary `eprintln!` inside `oag_ai::driver::Driver::drive` (removed before this
landed - see `crates/ai/src/driver.rs`'s `git log` if it needs re-adding).

- **Candidate 3 (a test froze the behaviour) - resolved, not the cause.**
  `line_following_tests.rs:31-32`'s `airbrake_left/right == 0.0` assertion drives
  `straight()`, a genuinely zero-curvature line. `corner_target` returns
  `f32::INFINITY` there and always will; the assertion is correct and is not a
  constraint on anything else.
- **Candidate 1 (tuning fitted against a craft that cannot airbrake) - not the
  lost-command cause.** `driver::trail` (the differential) is pure driver-side
  arithmetic over `Steer` and `f32`s; it returns identically whether or not
  `Handling::airbrake` has any physical effect. `probe.rs`'s warning about
  `closed_loop.rs`'s default fixture is real but orthogonal to why the command
  itself is never issued.
- **Candidate 2 (unreachable without an open brake) - confirmed, mechanism
  relocated.** The thread's literal framing (`airbrakes()`'s `floor`/`limit` gate)
  is **refuted algebraically**: with `brake == 0.0`, `airbrakes()` sets `limit =
  0.0`, so `low = 0`, `high = |differential|` - a one-sided command still reaches
  the physics layer with no brake at all. The actual gate is one level up, in
  `driver::trail`'s own `if speed < target { return 0.0; }`. Measured on the real
  corner: `steer.command` pinned at `±1.0` (full lock, past `trail_saturation =
  0.85`), `steer.rate_error` 0.5-0.6 rad/s (past `trail_deadband = 0.15`), for
  over ten consecutive ticks, while speed collapsed from 111 to 39.5 units/s and
  `target` sat at 160-180 throughout. `speed < target` held on every one of those
  ticks, so `differential` was `0.0` on every one of them - the differential
  airbrake, which exists precisely for "the steering loop has run out of
  authority of its own" (`trail`'s own doc comment), sat out the one corner on
  the lap that needed it most.
- **Autopilot shares the AI's driver - confirmed, not just consistent.**
  `Race::autopilot_controls` (`crates/game/src/race/field.rs`) calls
  `ship.driver.drive` with the same line, tuning and pilot slot
  `Race::step_opponents` gives every AI opponent. One fix or one measurement
  covers both.
- **The wall-bumping half of the report is not reproduced in this configuration.**
  VENOM/Ace, solo, zero respawns over two laps - the craft self-recovered inside
  the corridor. What is measured is "never airbrakes"; a collision is not, and a
  RAPIER or PHANTOM run (a one-line class change to the same scratch harness) is
  the next place to look for it, not assumed from the thrust of the report alone.

## Why the obvious fix does not survive contact with a real track

Tried: replacing `trail`'s `speed < target` with `!target.is_finite()` - off only
on a genuine straight (where `corner_target` returns infinity), on rather than
"has the craft not yet measurably exceeded a modelled target" everywhere else.

Passed every test in `crates/ai/tests/closed_loop.rs`, **including the stability
guard** `a_differential_braking_driver_does_not_ring`. That pass is misleading:
`closed_loop.rs`'s `oval_of` builds its straights from exactly collinear points, so
`curvature()` returns exactly `0.0` and `target` is exactly `f32::INFINITY` on
them - 303 such rows in an 1,800-tick run. **A real racing line's three sampled
points are never exactly collinear.** Over the full two-lap Talon's Junction trace,
`target` was `f32::INFINITY` on **zero** of 5,336 ticks; the minimum finite target
seen was ~101, the maximum over 1,100, and every value in between, but never
literally infinite. `!target.is_finite()` is therefore a real discriminator on the
synthetic oval and **an inert one on disc geometry** - it does not gate anything
out on a real track, so the change amounted to removing the gate outright there.

Consequence, measured on the full seven-opponent field ground-truth test
(`crates/game/tests/opponent_weapons_ground_truth.rs`,
`a_field_racing_with_real_pads_does_not_mine_itself_to_death`, one real minute of
racing with the disc's own pads): the unmodified gate clears it at mean 85/95,
worst 59/95. With the inert replacement, mean fell to 58/95 and **worst fell to
0/95** - one opponent ground to nothing. `closed_loop.rs` cannot see this failure
mode because its own off-line recoveries only ever happen on its exactly-straight
segments; a real track has no such segment, so an off-line recovery mid-corner (a
spin, a bad line after a hit) now saturated the steering loop and got the
differential piled on top of it everywhere, not just at the one genuine corner the
fix targeted - precisely the "second path in parallel with a loop that already
oscillates" the differential's own doc comment warns is what the crate was
rewritten to remove.

The attempt was reverted in full (`crates/ai/src/driver.rs`'s `trail` is back to
`speed < target`, unchanged in behaviour) and only its doc comment kept, rewritten
to record this finding rather than describe code that is no longer there. The AI
determinism reference (`crates/ai/tests/determinism.rs`) did not move.

## Open

- **A working gate needs a threshold that discriminates on real curvature, not on
  "is this exactly a straight".** Candidates worth measuring, none tried: a floor
  on `curvature` itself (below it, treat as straight regardless of `target`'s
  exact value); a bound on cross-track error, so the differential engages only
  while the craft is still credibly *on* the corner rather than recovering from
  being nowhere near it; or a rate-of-change limit on `target` itself, since a
  spin's `target` swings wildly tick to tick while a real corner's does not.
- **This is fitted, not derived.** Whatever threshold is chosen needs the same
  12-circuit benchmark `Tuning::lateral_accel`'s own history used
  (`docs/gameplay/ai.md`'s sweep table), plus `opponent_weapons_ground_truth` and
  the rest of the disc-backed AI ground-truth suite as regression coverage - not
  just `closed_loop.rs`, which this investigation showed cannot see the failure
  mode on its own.
- **`closed_loop.rs` needs a real-geometry fixture.** Every corner it tests is a
  perfect circle or a perfect straight; nothing in it has the very slight,
  never-quite-zero curvature a disc track's "straight" sections actually have.
  Add one alongside `oval_of` before trusting a green `closed_loop.rs` run to mean
  a gate change is safe on real data again.
- **The wall-bumping half of the original report** - unreproduced here (VENOM/Ace
  solo, zero respawns). Worth one run at RAPIER or PHANTOM before assuming the
  fix above also closes it.

## Next Steps

1. Instrument `driver::trail` (or `Driver::drive`) to log `curvature`,
   `target`, `steer.rate_error`, and cross-track error together, across a real
   lap, and separate "saturated because of a genuine tight corner" from
   "saturated because of an off-line recovery" by eye first - the threshold
   candidates above are guesses until that separation is visible in real data.
2. Pick a threshold, add the real-geometry `closed_loop.rs` fixture the RE
   above says is missing, and confirm the fix on it before touching real data.
3. Re-run `opponent_weapons_ground_truth` and the rest of `just test-data`'s
   AI-adjacent suite (`race_ground_truth`, `lap_times_ground_truth`) against the
   change - this is exactly the class of regression `closed_loop.rs` alone did
   not catch once already.
4. Regenerate `crates/ai/tests/determinism.rs`'s `REFERENCE` deliberately
   (`cargo run -q -p oag-ai --example determinism_report`), and say in the commit
   which scenarios moved and why - `Scenario::Solo` moving is expected this time,
   since a single craft alone on a corner reaches `trail` on every saturated
   tick.
5. Only then: the RAPIER/PHANTOM run for the wall-bumping half, since a real fix
   changes what there is to measure.
