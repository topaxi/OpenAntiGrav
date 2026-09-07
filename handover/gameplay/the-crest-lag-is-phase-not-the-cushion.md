# The crest lag is crest phase, not the cushion

**Resolved 2026-09-07, by the experiment the previous pass designed.** This file
was `the-hover-cushion-lets-go-late.md` (and before that
`the-hover-probe-takes-only-the-deepest-hit.md`). Two candidates were left for a
seven-tick liftoff lag on one crest of `talons-junction-clean-lap.csv`: **(a)**
our hover reach is too generous, or **(b)** residual crest phase. It is (b), and
(a) is refuted with no room left in it.

## What the discriminating experiment measured

`crates/trace/tests/hover_contact_ground_truth.rs`, new, on the
`wall_contact_ground_truth.rs` template: walk the **recorded** poses - the
original's own position and basis, tick by tick - and ask
`oag_physics::hover::evaluate` what it finds there. No integration, no
accumulated error, so trajectory phase cannot enter the answer by construction.

- **0 disagreements over all 2,976 comparable ticks.** Our contact test
  reproduces the recording's entire `grounded` column, including all 23 ticks of
  `0.5` and both airborne events. Ours at pose `t` against the recording's column
  at `t + 1` - not a fitted offset but the frame ordering the replay harness
  already emits under (`oag_trace::replay::replay` pushes its row *before*
  stepping, and `speed_cached` is documented as the previous frame's value at
  199/199 for the same reason).
- **On the crest itself**, ours from the recorded poses reads `0` at 1594, `0.5`
  on 1595-1600, `0` on 1601-1607, `1` at 1608 - which shifted by that one tick is
  exactly the original's `0` at 1595, `0.5` on 1596-1601, `0` on 1602-1608, `1`
  at 1609.
- **Confirmed a second way, integrator and sweep included** - neither of which
  a pose walk exercises, since `hover::sweep` runs only when both probes miss.
  `oag-trace run --reseed 2` on the same capture: `grounded  max error 0.000e0
  ... exact`, and the emitted crest column is the original's tick for tick. Read
  it as half a column: at reseed 2 the even rows are the reseed echo (emitted
  before the step, `grounded` copied off the recording by `initial_state`), so
  only the odd rows carry a stepped verdict - they do cover the transitions at
  1595, 1601, 1603, 1605 and 1607.
- **The lag itself, re-measured here rather than inherited**: at `--reseed 60`
  ours is airborne 1603-1615 against the original's 1595-1608, liftoff eight
  ticks late, plus a three-tick event at 1590-1592 the original does not have.
  The previous pass read 1602-1614; the shape is the same.

**The falsification criterion the previous pass wrote down was explicit** - if
the lag survives a pose that is exact on the tick before liftoff, it is our
reach. It does not survive. The reach, the surface classes, the probe offsets and
the fast-path branch are all right on this circuit; the eight ticks are where the
craft *is* by the time it reaches the crest.

Written up permanently in [`docs/physics/README.md`](../../docs/physics/README.md)
("The contact test itself is exact against a capture"), so nothing here needs to
survive this file.

## A correction that fell out

`docs/gameplay/ai.md` said that above `FAST_PROBE_SPEED` "`grounded` cannot read
`0.5`", because the fast path copies the front probe's hit flag onto the derived
rear one. **All 23 of the capture's `0.5` ticks are above that threshold**, with
`speed_cached` - which *is* `craft+0x2ec`, the branch's own input - between 76.4
and 111.6. The guarantee is one-directional and this crate already implements it
correctly: the copy only happens when the front ray *hits*; a front miss falls
through to casting the rear for real, which is the state the original is in on
every one of those ticks. The page is corrected; the branch, the flag copy and
the `6.0` are untouched.

## Open

- **Why the phase drifts eight ticks of crest in the 35 ticks after a reseed.**
  This is the force-law drift [`oag-trace.md`](../../docs/tools/oag-trace.md)
  already documents, now with a sharp instance attached to it: at `--reseed 60`
  the craft reaches the crest late enough to move liftoff from 1595 to 1603,
  while its *contact test* is exact. Nothing about the cushion will move it.
- Unchanged from the previous pass, and still true: **do not read the
  single-seeded run as contact evidence.** There `grounded` is `1.0` on all 2,977
  ticks and every disagreement points one way, which is that same drift.

## Next Steps

- **Attack the drift with the pose walk, not with a replay.** The technique that
  settled this generalises: for any per-tick quantity that disagrees, walk the
  recorded poses and evaluate our term there. It separates "our law is wrong"
  from "our trajectory is elsewhere", which no reseed interval can. The obvious
  next target is the force terms themselves - which of them, evaluated at the
  original's own pose over ticks 1560-1600, integrates to the position error that
  carries the crest phase.
- The `exceeded` column in `oag-trace run`'s field table is a **tick number, not
  a count** (`crates/trace/src/compare.rs:996-1014`); documented in
  [`docs/tools/oag-trace.md`](../../docs/tools/oag-trace.md) now. Cost the
  previous pass time; left here because it is still the easiest misreading in the
  tool.
