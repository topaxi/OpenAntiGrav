# Outpost 7 loses 34-35 shield a lap to a corner no craft can hold at the speed the AI picks

Split out of `an-ai-craft-pays-for-barrel-rolls-it-never-flies.md` on
2026-09-06, whose other three findings all landed with the grounded gate in
`oag_physics::barrel_roll::advance_gesture`. **This is the half that did not
land, and it is the larger term.**

## What was measured

One lone Ace craft, `Mode::SingleRace`, 18,000 ticks, `07_Track`, no rolls
armed at all: it sheds **34.1 then 35.0 shield per lap purely to wall
contact**. That is the only circuit on the disc measured with wall attrition on
that scale, and the only one whose wall term could be read on its own, because
every other circuit's per-lap figure has roll charge and shield-pad pickups
mixed into it.

Traced tick by tick, the craft is down to 2.7 units/s at driver index ~2,145
and to 23-30 units/s at ~2,400 on every lap, shedding 8-9 shield each time.
Two clusters, both reproducible every lap.

The circuit still banks a clean 49.9s lap and still ends the run on **0.0
shield**, destroyed, on lap 3 - it is the weakest row on the board and it has
been for as long as the measurement has existed. It passes
`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
on about 8 shield of margin, which is why an unrelated 22.8 shield of barrel-roll
charge was enough to turn it red in September.

`13_Track` is the next thinnest on the same measurement, finishing on **2.7**
shield, and its cause has not been looked at at all.

## Step 1 is done: the line is neither into a wall nor above the floor

Measured 2026-09-06 against driver indices 2,050-2,250, lone Ace craft,
`pulse-psp-usa.chd`, with a scratch dump over `race::Race` that was deleted
after the run. **What was measured is this repository's loaded collision world
and racing line against the shipped track file's own authored widths** - not
the original's behaviour, which nothing here observes. Confidence **90** on
that reading; it says nothing about whether the original's craft goes where
ours does.

- **Under the line.** The cast
  `race_ground_truth::the_racing_line_has_track_under_it_where_it_is_known_to`
  makes hits `Floor` at every index in the window, 7.94-9.29 units below the
  cast origin, with no gap and no step. Consecutive line samples are 0.89-1.77
  units apart, so there is no junction stitch either.
- **Beside the line.** Casting sideways from the line along the sample's own
  `lateral`, the nearest wall tracks the sample's authored `half_width_left`
  within a few units wherever one exists (12.5 against 11.56 at 2,100; 33.0
  against 28.25 at 2,145; 18.0 against 16.80 at 2,160). **The collision load
  agrees with the track file.**

What actually happens is that the *craft* leaves the line. Between index 2,100
and 2,145 it goes from 0.6 units off the line to **30.85 units left of the
spline centre, where the authored left half-width is 27.3**, while accelerating
83.6 to 101.0 units/s up a climbing left-then-right sequence. It hits `Wall`
collider 148, normal horizontal (`|n.up|` 0.00-0.01).

The cost splits into a **4.36 shield impact that takes the craft from 101.0 to
2.7 units/s in 18 ticks**, and ~180 ticks of grinding at 0.02-0.05 shield a
tick. Cluster 1 (2,100-2,199) is 24.08 shield over three laps; cluster 2
(2,350-2,399) is 26.43; together a little over half the per-lap figure.

**The AI's corner-speed model is ruled out.** Dropping `Tuning::lateral_accel`
from the measured 260 to 90 - a third, roughly Novice's belief - moves the
2,100-2,199 loss from 24.08 to **24.11**. The craft hits the same wall 50
samples later and grinds longer at a lower speed for the same total. A driver
that believes it has a third of the grip runs just as wide here.

Full evidence, tables and the reproduction recipe:
`~/.claude/projects/-home-topaxi-projects-OpenAntiGrav/scratch/outpost7-walls.md`.

## Step 2 is done: it is neither the steering nor the grip. The corner is untakeable.

Measured 2026-09-06 on the merged tree, same lone Ace, same circuit. **Neither
of the two candidates step 1 left open is the cause, and both were ruled out by
measurement.**

- **Yaw does not saturate below its cap.** `Tuning::max_turn_rate` (1.8) clamps
  the rate pure pursuit *requests* in `Driver::steering`; 1.55-1.62 is the rate
  the *hull* achieves. Nothing clamps the second to the first. The hull's
  ceiling is arithmetic off recovered constants and `max_turn_rate` is not one
  of them: `steer * Turning.amount / (5 * I_yy)` = `100 * 1.68 / 108` =
  **1.556 rad/s**, and `docs/ghidra/functions/psp-pulse-usa/engine.md` measures
  the *original* at 1.42-1.51 under full lock. We are within a few per cent of
  the original hull. `Tuning::max_turn_rate`'s own doc already said 1.8 was
  chosen to stop binding.
- **The craft is not sliding.** Slip - velocity against the craft's own forward
  - is **-8.4 degrees** and flat through the whole departure, and `grounded` is
  **1.00 on every tick** with `time_airborne` at 0.000, so the
  `grip_ground`/`grip_air` split is never crossed. The "53 degrees against 30"
  step 1 recorded is `vel_err` against `head_err`, both taken against the
  *line's* tangent; their difference is that same 8.4 degrees. What grows is the
  heading: the craft is pointed up to 26 degrees off where the line goes and the
  velocity follows the body faithfully.

**What binds is kinematics.** A craft at speed `v` on a line of curvature `k`
must yaw at `v * k`; with the hull's ~1.55 rad/s the fastest any craft can hold
that line is `v_yaw = 1.55 / k`. The racing line's **local** curvature (chords
of 4, 8 and 16 samples, all three agreeing) peaks at **0.047 at index 2,119** -
radius 21 units, `v_yaw` **33 units/s**. The craft is doing 94.

The departure begins where the speed crosses `v_yaw`, **within three samples**:
at 2,098 the craft is at 82.4 against a `v_yaw` of 80, `lat` is +0.2 there, 0.0
at 2,100 and negative from 2,101 to -30.9 at the wall.

**And the driver never sees the corner.** `corner_target` is built on
`Line::max_curvature(index, window, look * 0.5)`, and at 80 units/s that span is
24, so `Line::curvature`'s chord triple covers ~72 units of track while the
tight arc is ~50 long. It reports **0.0155-0.0186** where the local value is
0.047 - understating it **2.5-3x** - and puts its own peak at index 2,090,
thirty samples before the real one.

**This is also why the `lateral_accel` probe came back null.** `v_grip(90)` at
`k = 0.047` is 43.8 and `v_yaw` is 33: a driver that believes it has a third of
the grip is still asking for a speed the hull cannot rotate at. The grip knob
cannot reach this corner from either end.

Fraction of each circuit yaw-limited (`k > 1.55^2 / 260`), worst sample:
06 26 % / `k` 0.055 / `v_yaw` 28; **07 20 % / 0.047 / 33**; 16 16 % / 0.028 / 55;
03 15 %; 02 15 %; 09 13 %; 13 12 % but its worst still admits 90 units/s;
14 9 %; 10 8 %; 05 7 %; 04 7 %; 01 3 %. **13's 2.7 shield is not this bug.**

Full evidence, tables, per-tick dumps and the two probes below:
`~/.claude/projects/-home-topaxi-projects-OpenAntiGrav/scratch/steering-saturation.md`.

## Step 3 is done: probe B's regression was a latent estimator bug, and it is fixed

Two probes, each measured on the whole twelve-circuit board, then reverted.
**Both were measured on the pre-fix tree**, so the 06 row below is the
symptom this section goes on to explain, not a property of the probe.

- **A: the yaw-rate term alone.** `corner_target` returns
  `min(sqrt(lateral_accel * commitment / k), max_turn_rate / k)` - kinematics,
  not a tuned constant. Board-wide worth having (16 32.78 -> 48.75 shield, 09
  59.20 -> 72.10, 14 41.41 -> 48.38) and it **does not fix 07 at all**: the
  cluster moves from 2,100-2,149 to 2,150-2,199 for the same 24.1, because A
  caps against the understated curvature.
- **B: A, plus capping the estimator's span at 10 units** so it resolves a
  50-unit corner. This is the one that reaches 07 - 27.4/26.4/28.0 a lap, and
  the craft survives to lap 4 - and it transforms half the board (16 71.69,
  09 84.54, 05 73.09, 14 65.91). **It breaks 06**, which loses its clean lap and
  takes three respawns. `min(10.0)` is a number chosen to make 07 behave, which
  is exactly the invisible damage the thread's own rule forbids.

**B's damage was two *located* events, and as of 2026-09-06 both are
explained. Neither is probe B's.**

- **06's 34.72 was a bug in `Line::curvature`, now fixed.** The estimator
  chained its three walks off each other's landing *index*, and `Line::ahead`
  returns an interpolated point but the index of the *segment* it landed in.
  Where one segment is longer than `span`, all three walks land inside it and
  return the same index, the outgoing chord is the zero vector, and `acos(0)`
  is charged as a right-angle turn over a fraction of a unit. **Every circuit
  on the disc has two path seams of the shape `0.30, 0.11, ~7.0` units, and
  06's is the only jump longer than ten - 14.78 - which is the whole of why B
  broke 06 and nothing else.** B pinned `span` at 10, so the phantom fired at
  every speed: the craft was told to hold 1.2 units/s, held it, crawled into
  the seam's own unsupported stub at 1196-1199 and ground there at 0.01-0.08
  shield a tick until the rescue took it. Fixed in
  `crates/ai/src/line.rs`, pinned by `a_long_segment_is_still_straight`.
  **With it fixed, 06 under B is 69.27 shield and zero respawns** - 11 better
  than baseline.
- **14's is a marginal one-tick contact, not the unsupported line.** The
  hypothesis below was wrong in its mechanism. 14 has no seam over 10 units and
  no curvature spike anywhere, so B's cap cannot fire on it; the craft passes
  index 751 at **134 units/s under B and 132-134 in baseline**, and the 7.60 is
  a single tick on a single lap at a speed the other three laps take without a
  scratch. The line there does run 9-24 units above the floor (the recorded
  684-765 run), but nothing is slowed over it. It belongs to the wall-response
  item in `## Open`, not to the span.

The estimator fix landed on its own, ahead of any of this: it is a bug with a
disc-free reproduction, and its board movement is 0.5-2.7 shield either way with
no lap or respawn change anywhere. It moved `oag-ai`'s committed driver
reference - the chord spacing changes every reading the estimator takes, not
only the seam - and that move is in its own commit with the isolation recorded
in `crates/ai/tests/determinism.rs`'s own History. `oag-core`, `oag-physics` and
`oag-gameplay`'s gates did not move. Baseline lap times shifted by up to a tick
or two with it: **07 is 49.8s where it was 49.9, 06 37.8 where it was 38.1**, all
twelve still lap clean and 01 keeps its single respawn.

What still wants its own thread is A and the span: a change to which limit law
the AI uses, moving every circuit, with the span's value still unmeasured. What
has changed is that the sweep is no longer anchored on a cause nobody had looked
at.

## Open

- **The AI's cornering model has no yaw-rate term.** Established above, and
  probe A is the shape of the fix. The estimator's resolution is no longer the
  second half of that sentence: `Line::curvature` is fixed, and a 10-unit span
  now costs nothing anywhere on the board. **The span sweep is unblocked** -
  what is left is choosing the number, which is still a number and still wants
  measuring rather than picking.

- **Whether the wall response should stop a craft dead.** 101.0 to 2.7 units/s
  in 18 ticks with one contact resolved a tick, against the 3.5 per cent
  per-frame tangential bleed
  `docs/ghidra/functions/psp-pulse-usa/contact-response.md` recovers. Some of
  the gap is the normal impulse killing an into-wall velocity the craft's own
  28-degree heading error keeps restoring, so this needs its own measurement
  against `oag_physics::wall::resolve` in isolation before it is called a bug.
  It is the same wedging mode `crates/game/tests/race_ground_truth.rs`'s module
  docs already record.
- **Whether hull damage should be charged on every tick of a sustained
  contact.** ~5 of the 8 shield a lap in cluster 1 is the grind rather than the
  impact. `FUN_088418e0` is the evidence page for what the original does.
- **Why the wall beside this stretch is short.** Repeating the sideways cast at
  0, 2, 5 and 11 units above the line finds the left wall at h0 and **nothing
  within 80 units at h2 and above** almost everywhere in the window - the
  responding geometry is roughly 3-5 units tall measured from the road. Shipped
  geometry, a `Surface` tag that stops at a height, or a sloped face the cast
  clips: not chased. It bears on the impact, which may be against the top edge
  of a low wall rather than the face of a tall one.
- **The AI corridor is inverted over 2,083-2,119.** `ai_bound_right` goes
  negative (to -1.796 at 2,107) while `ai_bound_left` stays near zero, so the
  two bounds cross and `crates/game/src/race/spline.rs`'s `.min(0.0)`/`.max(0.0)`
  rebasing clamps both to zero. `spline.rs`'s doc comment's straddle-by-0.1
  guarantee is about *control points*; these are interpolated samples between
  them. Under 1.8 units on a track 50 wide, and **not a contributor** - a
  zero-width corridor makes the driver drive the exact line, which is the
  tightest case `Tuning::corridor_use` can produce. Recorded so it is not
  chased.
- **The undecomposed buckets**: 1,900-1,949 (10.23 over three laps),
  650-749 (9.26), 2,850-2,949 (8.22), 1,100-1,149 (5.03).
- **`13_Track`'s 2.7 shield** has never been decomposed.

## Next Steps

1. **Sweep the curvature estimator's span with probe A's yaw-rate term in
   place**, the way `lateral_accel` was swept (`sweep_grip` in
   `race_ground_truth.rs` is the harness shape). This is now a straight
   measurement: the thing that was dominating 06's shield is fixed, so 06 can
   be the circuit that discriminates. Measured on the fixed tree, A + a
   10-unit cap gives 06 **69.27**/0 respawns, 14 **79.45**, 16 71.90, 09 84.45,
   05 73.62, 04 59.68, 02 86.93, 03 95.00, 10 58.56, 13 8.08, 01 38.96/1
   respawn, and 07 reaches **lap 4** instead of dying on lap 3. Nothing
   regresses. That is one point on the sweep, not the answer.
2. Measure `oag_physics::wall::resolve` in isolation: a craft at 100 units/s
   grazing a flat wall at 28 degrees, one contact a tick, and compare the
   tangential bleed against the recovered 0.035. **14's 7.60 at index 751 is
   the cheapest case to start from** - one tick, one index, one lap in four, at
   a steady 134 units/s, and it flips on a sub-unit positional difference.
3. Do **not** trim `07_Track` out of the test's known-good list: it has always
   passed and the assertion is right. Do not move `lateral_accel`, `grip_ground`
   or `grip_air` - the first cannot reach this corner and the last two are
   authored per-craft data that the `grounded == 1.00` measurement clears.
