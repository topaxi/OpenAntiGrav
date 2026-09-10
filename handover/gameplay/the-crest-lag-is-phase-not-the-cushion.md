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

- **Resolved 2026-09-10, and narrowed rather than closed.** The force-term pose
  walk this thread's own "Next Steps" called for
  (`crates/trace/tests/force_term_pose_walk_ground_truth.rs`) found that none
  of `oag_physics::forces::evaluate`'s eleven terms, individually rescaled,
  explains the crest-approach residual - the largest best fit (rolling
  resistance) cuts 3.1% of the residual's own energy, every other term under
  2%. The drift instead traces almost entirely to one discrete event: tick
  1583's recorded velocity turns through a large angle at nearly constant
  speed, the signature of a wall or kerb contact `oag_physics::wall::resolve`
  resolves *after* the integrator, outside anything a pose walk of forces can
  see. Weighted to the crest, that one tick alone accounts for 82% of the
  position-error yardstick an eight-tick liftoff shift needs; the other 39
  ticks in the window together account for under 3%. Full measurement, per-tick
  table and the yardstick arithmetic: `docs/physics/README.md`, "The
  crest-phase drift traces to a wall event, not a force term".
- Unchanged from the previous pass, and still true: **do not read the
  single-seeded run as contact evidence.** There `grounded` is `1.0` on all 2,977
  ticks and every disagreement points one way, which is that same drift.

## Next Steps

- **The next target is `oag_physics::wall::resolve` around tick 1583, not a
  force term.** The same pose-walk technique generalises one step further:
  walk the recorded poses and the recorded *wall* geometry around the
  1571-1579 hop's landing and ask what `wall::resolve` does there, the same
  way this pass asked what `forces::evaluate` does - separating "our wall
  response is wrong" from "our trajectory is elsewhere" for a projection
  rather than an accumulator term. Nothing about which geometry feature is
  involved, or whether it is a kerb, a barrier or something else, has been
  measured yet.
- The `exceeded` column in `oag-trace run`'s field table is a **tick number, not
  a count** (`crates/trace/src/compare.rs:996-1014`); documented in
  [`docs/tools/oag-trace.md`](../../docs/tools/oag-trace.md) now. Cost the
  previous pass time; left here because it is still the easiest misreading in the
  tool.
