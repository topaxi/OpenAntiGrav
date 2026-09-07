# The hover cushion lets go about seven ticks late

**Re-measured 2026-09-07.** This file used to be
`the-hover-probe-takes-only-the-deepest-hit.md`, claiming eight hover probes resolved
to one contact, no `cross(r, impulse)` angular response, and `grounded` collapsing to
`0` after ~75 ticks. All three are false against the current crate. What is left is
much narrower and needs the *reseeded* comparison to see at all: our liftoff events
happen, last about the right length, and start late.

## What was refuted, with evidence

- **There are two hover probes, not eight, and both are used.**
  `crates/physics/src/hover.rs:570` is `probes: [HoverProbe; 2]`; `:571-572` counts
  contacts `0..=2`; `crates/physics/src/forces.rs:639-643` applies *every* contacting
  probe's force, and `hover.rs:1041-1049` averages the normals of all of them. The
  "eight" belonged to the wall path all along -
  [`determinism.md`](../../docs/architecture/determinism.md) lists "the
  deepest-of-eight hull probe comparison" beside `CollisionWorld::colliders()`'s
  nearest-hit scan, under order-dependent reductions, and that path went multi-contact
  with `crates/physics/src/wall.rs:380-381`'s `hull_contacts` and its
  `dropped_contacts` counter (`wall.rs:270`).
- **The `cross(r, impulse)` angular response exists and always did on this path.**
  `crates/physics/src/ship.rs:135-138`: `add_force_at_point` adds
  `(point - self.position).cross(force)` to the torque accumulator, and `forces.rs:641`
  is the call. The wall path has its own at `wall.rs:566`.
- **`grounded` does not decay.** Re-run at `--script-lead 2` (the lead
  [`oag-trace.md`](../../docs/tools/oag-trace.md) confirms for every capture under
  `data/traces/`) against the very capture the 75-tick claim came from,
  `talons-junction-time-trial-lap.csv`, 3,146 ticks, single-seeded:
  `grounded  max error 0.000e0  ...  exact`. Exact on 3,146 of 3,146.

## Open

`talons-junction-clean-lap.csv` is the capture that says anything, because the
original itself leaves the ground on 31 of its 2,977 ticks - `0.5` on 23, `0` on 8.
**Read it reseeded.** `--reseed 60` snaps the craft back onto the recording's own pose
every 60 ticks, so the contact test is asked about the right pose rather than about a
craft that has drifted elsewhere:

| recorded | simulated | ticks |
| ---: | ---: | ---: |
| 1 | 1 | 2,940 |
| 0.5 | 0.5 | 15 |
| 0 | 0 | 3 |
| 0.5 | 1 | 8 |
| 1 | 0 | 6 |
| 0 | 0.5 | 4 |
| 0 | 1 | 1 |

**We agree on 18 of the original's 31 non-grounded ticks, and 19 ticks disagree in
both directions.** The three events, tick by tick:

- **432-439.** Original `0.5` throughout; ours `0.5` on 432-437 and back to `1` on
  438-439. A near-exact match, two ticks short.
- **1571-1579.** Original `0.5` throughout; **ours matches on all nine ticks.**
- **1595-1614, the one that carries the whole disagreement.** The original is airborne
  1595-1608 (14 ticks: `0` at 1595, `0.5` through 1601, `0` 1602-1608) and back to `1`
  at 1609. Ours is airborne 1602-1614 (13 ticks) and lands at 1615. **Nearly the same
  airborne duration, started about seven ticks late and ended about six ticks late.**

So the shape is a **phase lag on a single crest**, not a contact test that refuses to
let go. Note the direction: 6 of the 19 disagreements are ours lifting off where the
original is on the ground, so this is not one-sided.

**The single-seeded run is not evidence about contact and should not be read as
such.** Without reseeding our `grounded` reads `1.0` on all 2,977 ticks and all 31
disagreements point one way, which looks like a cushion that never lets go. That is
the force-law drift `oag-trace.md` already documents - by tick 432 the two craft are
not on the same part of the track - not a contact bug.

Two unmeasured candidates for the ~7-tick lag, and nothing separates them:

1. Our reach or target height is slightly generous at a crest. The probe raycast
   reaches exactly `ride_height` (`hover.rs:167-172`), or the swept contact test added
   by `9c7fed3b` catches a hit the original's instantaneous test would have missed.
2. Residual crest-phase offset. Tick 1595 sits 35 ticks after the reseed at 1560, so
   several units of position error have rebuilt by then; a reseed restores the pose,
   not the phase of the crest the craft is climbing.

Also open, and cheap:

- **`exceeded` in `oag-trace run`'s field table is a tick number, not a count**
  (`crates/trace/src/compare.rs:996-1014`, from `first_exceeded_tick` at `:568`; `-`
  means never). Misreading it as a count turns 19 disagreeing ticks into "438
  disagreeing ticks". Cost time on this pass.
- The one single-hit thing left in hover is the penetration-escape *teleport*
  (`hover.rs:1079-1088`), which takes the larger of the two probes' corrections. It is
  already labelled **a choice, not a finding** in its own comment, with no confidence
  score, and it is not the force path. Unrelated to the lag above.

## Next Steps

- **Capture one crest, not a lap.** Candidate 2 above cannot be excluded from a
  whole-lap replay at any reseed interval, because a reseed restores position and not
  crest phase. A short scripted run at ticks 1560-1620 of `16_Track`, reseeded at
  every tick or seeded fresh at 1590, would answer it directly: if the lag survives a
  pose that is exact on the tick before liftoff, it is our reach.
- **`talons-junction-clean-lap.csv` already contains airborne data**, which is a
  partial answer to
  [`nothing-airborne-has-ever-been-captured.md`](../tooling/nothing-airborne-has-ever-been-captured.md)
  from the other direction: 31 ticks of the original off the ground, over three
  events, in a capture that is already committed. Thin - 14 ticks is the longest
  event - but it is not nothing, and it is the fixture the crest capture above should
  be built around.
