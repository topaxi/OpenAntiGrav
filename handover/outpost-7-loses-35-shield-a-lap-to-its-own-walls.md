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

## The fix is a design call, probed and deliberately not taken

Two probes, each measured on the whole twelve-circuit board, then reverted.

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

**B's damage is two *located* events, not conservative cornering**, and the
obvious reading - "B corners slowly and 06 has the tightest corner" - is wrong.
Shield by 50-index bucket, per lap:

- **06, indices 1,200-1,249: nothing at all in baseline, 34.72 on lap 1 under
  B** with three respawns, then 4.43 and 3.45. That bucket holds 06's worst
  corner, `k` 0.0547 at index **1204**, `v_yaw` **28 units/s** - the tightest
  sample on the disc. Every other bucket on 06 improved. So B is doing something
  specific at 06:1204, most likely wedging a craft it has slowed to a crawl.
  **Cause not established.**
- **14, indices 750-799: nothing in baseline, 7.60 on lap 3 under B** and
  nothing on laps 1-2. `crates/game/tests/race_ground_truth.rs` already records
  14 as having **82 racing-line samples with nothing under them at 684-765**,
  which that bucket overlaps - a craft slowed over an unsupported stretch is a
  different failure from one that flies it. 14 is a large net win under B all
  the same: 29.09 total against baseline's 53.59, its worst bucket (2800-2849,
  at its tightest corner, index 2785) falling from ~6 a lap to under 1.

So the fix wants its own thread: it is a change to which limit law the AI uses
and at what resolution it reads the line, it moves every circuit, the span has
no measured value yet, and **the one measurement that would anchor a sweep -
06's shield - is dominated by a located event nobody has looked at.**

## Open

- **The AI's cornering model has no yaw-rate term and its curvature estimator
  cannot resolve a corner shorter than its own span.** Both established above.
  The fix is A plus a *measured* span rule, swept the way `lateral_accel` was
  (`sweep_grip` in `race_ground_truth.rs` is the harness shape), with 06 as the
  circuit that discriminates rather than 07.

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

1. **Find out what probe B does at 06:1204 before sweeping anything.** 34.72
   shield and three respawns, one bucket on one lap, at a place baseline never
   loses anything: that is a located failure, not conservative cornering, and a
   span sweep anchored on 06's shield would be optimising against a cause nobody
   has looked at.
2. Then sweep the curvature estimator's span the way `lateral_accel` was swept,
   with probe A's yaw-rate term in place. A span chosen against 07 alone is a
   span fitted to one circuit.
3. Measure `oag_physics::wall::resolve` in isolation: a craft at 100 units/s
   grazing a flat wall at 28 degrees, one contact a tick, and compare the
   tangential bleed against the recovered 0.035. Independent of step 1 and
   downstream of it - the craft reaches that wall at 101 because it was never
   slowed for a corner that admits 33, so a cheaper scrape would hide the cause
   rather than fix it.
4. Do **not** trim `07_Track` out of the test's known-good list: it has always
   passed and the assertion is right. Do not move `lateral_accel`, `grip_ground`
   or `grip_air` - the first cannot reach this corner and the last two are
   authored per-craft data that the `grounded == 1.00` measurement clears.
