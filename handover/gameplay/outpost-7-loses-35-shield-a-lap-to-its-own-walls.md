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
`outpost7-walls.md`.

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
`steering-saturation.md`.

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

## Step 4 is done: A and the span both landed, and the span was swept twice

Measured 2026-09-06 on the fixed tree. `sweep_curvature_span` in the new
`crates/game/tests/ai_span_sweep.rs` is the harness, `OAG_SWEEP_SPAN` taking a
comma-separated list of caps with `none` for the uncapped row. **The `none` row
without A is bit-identical to `main`** and reproduces every baseline figure this
thread quotes, and **A plus a 10-unit cap reproduces the known point to the
digit** - which settles what the prose left open: "capping the span" means
capping all three `Line::max_curvature` call sites, not the braking window
alone.

Solo columns are one lone Ace over twelve circuits for 18,000 ticks. Field
columns are the two committed field ground-truth fixtures set up exactly as they
set themselves up, reporting the quantities they assert on: `untimed` is
opponents past lap 2 with no lap time (**must be 0**), `mean`/`worst` are
opponent shield as a fraction of the pool after a minute (**floors 0.70 and
0.45**).

| span | solo total | resp | clean | mean lap | 07 laps | 13 end | untimed | field mean | field worst |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| (no A, none) | 562.04 | 1 | 12 | 41.0s | 3 | 2.90 | - | - | - |
| 4 | 689.31 | 3 | 11 | 43.6s | 4 | 13.91 | - | - | - |
| 5 | 738.77 | 1 | 12 | 43.3s | 4 | 14.82 | 0 | 0.90 | 0.63 |
| 6 | 726.01 | 1 | 12 | 42.8s | 4 | 13.41 | 0 | 0.78 | **0.29** |
| 7 | 715.32 | 1 | 12 | 42.7s | 4 | 9.18 | 0 | 0.78 | **0.25** |
| 8 | 709.29 | 7 | 11 | 43.5s | 4 | 9.89 | **1** | 0.91 | 0.78 |
| 9 | 709.51 | 1 | 12 | 42.7s | 4 | 7.89 | 0 | 0.83 | **0.35** |
| 10 | 725.91 | 1 | 12 | 42.8s | 4 | 8.08 | **1** | 0.84 | **0.41** |
| **11** | **705.68** | 1 | 12 | 42.7s | 4 | 6.52 | 0 | 0.88 | 0.64 |
| 12 | 704.18 | 1 | 12 | 42.6s | 4 | 5.34 | 0 | 0.90 | 0.63 |
| 13 | 675.59 | 1 | 12 | 42.5s | 4 | 7.31 | **1** | 0.91 | 0.81 |
| 14 | 688.94 | 1 | 12 | 42.4s | 4 | 4.36 | 0 | 0.83 | 0.45 |
| 15 | 683.72 | 1 | 12 | 42.4s | 3 | 4.28 | 0 | 0.87 | 0.69 |
| 16 | 669.70 | 1 | 12 | 42.2s | 3 | 7.97 | 0 | 0.84 | 0.57 |
| 18 | 649.39 | 1 | 12 | 42.2s | 3 | 4.17 | 0 | 0.93 | 0.78 |
| 20 | 660.42 | 1 | 12 | 42.2s | 3 | 6.16 | 0 | 0.88 | 0.77 |
| 25 | 620.39 | 1 | 12 | 42.0s | 3 | 3.13 | 0 | 0.86 | 0.76 |
| 32 | 620.28 | 1 | 12 | 42.2s | 3 | 2.16 | - | - | - |
| none | 613.52 | 1 | 12 | 42.2s | 3 | 2.90 | 0 | 0.83 | 0.50 |

**The two effects separate cleanly.** A alone is worth 51 shield (562.04 to
613.52) and reaches `07` not at all - 33.68/34.84 and still dead on lap 3,
because it caps against a curvature the estimator understates. The span is
worth another 92 and is the half that reaches `07`.

**A span of 10 was chosen first, on the solo board alone, and it turns two
committed disc-backed tests red** -
`lap_times_ground_truth::every_opponent_that_laps_has_a_lap_time` and
`opponent_weapons_ground_truth::a_field_racing_with_real_pads_does_not_mine_itself_to_death`.
One craft in a grid of eight wedges near the start and never completes a timed
lap; another is ground to 0.41 of its pool against a 0.45 floor. **Twelve
lone-craft circuits cannot see that, and `just` does not run the disc-backed
suite at all**, so the whole ordinary gate was green on it. The field columns
above exist because of that, and they are the durable half of this table.

**Eleven landed.** Criterion, stated so it can be disagreed with rather than
re-run: hard gates first (twelve clean laps, solo respawns not over the
baseline's 1, `13` not below its 2.90, both field tests green), then a band from
the line's own geometry - above the ~7-unit path-segment scale, below the ~15 at
which `3 * span` stops resolving `07`'s ~50-unit arc - then the solo maximum
inside it, which picks 11 over 12 by 1.5 shield on a board of twelve. **Read
that last step as a tie-break and nothing more**: 11 and 12 pass the field gate
because a craft happened not to wedge, and 10 and 13 fail it for the same reason
in reverse. The field worst does cluster (`{6,7}` 0.25-0.29, `{9,10}`
0.35-0.41, `{5,11,12}` 0.63-0.64), so it is one craft switching between basins
rather than a coin flip - but which basin a span lands in is not a property of
the span.

`07` after: **28.19 / 29.08 / 29.69** a lap and reaching lap 4, from 33.46/35.11
and destroyed on lap 3. Full table, criterion and reproduction:
`curvature-sweep.md`.

## Open

- **The span is a chosen number, and it is no longer waiting on the wall or
  the driver's identity.** 11 is green and 10 is red on the field board, and
  nothing about the estimator explains that ordering. Step 5 settled that
  this item was not blocked on the wall: **the wall response is faithful, so
  nothing lands there and the span does not want re-sweeping on account of
  it** - no committed constant moved. Step 6 settled the "upstream" half:
  the two remaining crashes are `corner_target`'s windowed curvature (`span =
  11`) understating the true local apex by 1.66x/1.85x, the same mechanism
  as idx 2,119 in step 2 at smaller magnitude - **not** the differential
  trail-brake's documented corner-entry gate, which is confirmed live here
  too but never has an overspeed to correct because the symmetric brake
  never engages either. What remains open is whether a span change can
  resolve these two apexes without re-breaking the field board the way span
  10 did - unmeasured, and real work of its own. The table above is the
  harness and the field columns are the part to keep.

- ~~**Whether the wall response should stop a craft dead.**~~ **Answered by
  step 5: yes, and it is not the friction doing it.** Closed.
- ~~**Whether hull damage should be charged on every tick of a sustained
  contact.**~~ **Yes.** `Body_RecordContact` (`0x0884dbf4`) stores
  `p = j*n - f*v_t` - the friction impulse is *inside* the recorded magnitude -
  and `FUN_088418e0` charges `FUN_088439ac(|p| * 0.05 * 0.7, ...)` per record
  with **no magnitude threshold**. Ours reproduces it to the digit. Closed; see
  step 5.
- **Why the wall beside this stretch is short.** Repeating the sideways cast at
  0, 2, 5 and 11 units above the line finds the left wall at h0 and **nothing
  within 80 units at h2 and above** almost everywhere in the window - the
  responding geometry is roughly 3-5 units tall measured from the road. Shipped
  geometry, a `Surface` tag that stops at a height, or a sloped face the cast
  clips: not chased. It bears on the impact, which may be against the top edge
  of a low wall rather than the face of a tall one.
- **The AI corridor is inverted over 2,083-2,119.** `ai_bound_right` goes
  negative (to -1.796 at 2,107) while `ai_bound_left` stays near zero, so the
  two bounds cross and `crates/raceplay/src/spline.rs`'s `.min(0.0)`/`.max(0.0)`
  rebasing clamps both to zero. `spline.rs`'s doc comment's straddle-by-0.1
  guarantee is about *control points*; these are interpolated samples between
  them. Under 1.8 units on a track 50 wide, and **not a contributor** - a
  zero-width corridor makes the driver drive the exact line, which is the
  tightest case `Tuning::corridor_use` can produce. Recorded so it is not
  chased.
- **The undecomposed buckets**: 1,900-1,949 (10.23 over three laps),
  650-749 (9.26), 2,850-2,949 (8.22), 1,100-1,149 (5.03).
- **`13_Track`'s 2.7 shield** has never been decomposed.

## Step 5 is done: the bleed is exactly 0.035, and the bleed was never the cost

Measured 2026-09-06 on the merged tree. **`oag_physics::wall::resolve`'s
tangential bleed is `0.035` to five significant figures, agreeing with the
value `docs/ghidra/functions/psp-pulse-usa/contact-response.md` recovers off
two literals. Nothing changed.**

Two isolation measurements, both now pinned in `crates/physics/src/wall/tests.rs`:

- **`a_28_degree_graze_bleeds_the_recovered_friction_and_nothing_else`.** A
  craft at 100 units/s meeting a flat `Wall` at 28 degrees, one contact a tick.
  The tangential factor is `0.965` on **every** tick at every angle tried (0, 5,
  10, 28 degrees), and the *total* loss converges **down** onto `3.5000 %` -
  `12.84 %`, `5.31 %`, `3.81 %`, `3.55 %`, ... - as the normal component is
  spent. It approaches the floor from above and never crosses it, which is the
  same one-sided shape the recovered page's own capture has (`5.21 %` ...
  `3.560 %`, minimum `3.534 %` at 104 units/s). `0.036` is falsified by the
  tail, `0.030` by the floor.
- **`a_held_heading_makes_the_bounce_the_cost_not_the_friction`.** The same
  graze with the velocity re-pointed to 28 degrees every tick - a stand-in for
  what grip on a craft whose heading is 28 degrees off does, not the grip law.
  **100 units/s to 7.34 in 19 ticks, one contact a tick, no thrust**, a flat
  `12.84 %` a tick. Of that, `2.72 %` is the friction and `9.82 %` is
  `-(1 + 0.4) * v_n / D` - the measured `v_n` factor is `-0.391`, i.e. `-e`.
  **The bounce is 73 % of the cost and the friction 27 %.**

So the `101.0 -> 2.7 in 18 ticks` figure has its **mechanism** identified and
measured, and the mechanism is the normal impulse against a `v_n` the heading
error restores - not the coefficient this step was sent to check. The residual
between the isolated `7.34` and the recorded `2.7` is the 2-3-contact ticks and
thrust, and is **not decomposed**, because **the event no longer reproduces on
the span-11 tree**:

- Instrumented lone-Ace run, `07_Track`, 6,000 ticks: 234 contact ticks, and
  **no contact tick after tick 200 has the craft below 30 units/s**. The
  2,100-2,145 wedge does not happen at all - the craft comes through at 40-42
  units/s with no contacts. 54.37 shield over two laps, matching step 4's
  28.19/29.08/29.69.
- The two remaining large events are **not grazes**: idx 2,158 arrives with an
  `impact_speed` of **59.78** and idx 2,395 with **74.76** against a total speed
  of 132.37 - a third of the velocity straight into the wall. Those are crashes.
- **`14_Track`'s 7.60 at index 751 is gone too**: 18,000 ticks, four laps, zero
  respawns, 78.44 end shield, and **zero contacts anywhere in 740-760**. The
  cheapest case this step was told to start from cannot be started from.

Two worries retired permanently, from the same instrumentation:

- **`MAX_HITS_PER_PROBE` and `HULL_CONTACTS` - the two caps `wall.rs` labels
  inventions - never bind on real geometry.** `dropped_contacts` is `0` on all
  24,000 instrumented ticks across both circuits. The swept path never fires
  either.
- Contacts per tick on `07` are `1` on 200 of 234 contact ticks, `2` on 27, `3`
  on 5 and `5` on 2 - and the two 5-contact ticks are square-on crashes. So
  `contact-response.md`'s "five applications would cost `16.3 %`, which the
  capture forbids outright" is not contradicted: that argument is about scrape
  geometry, and a scrape here is one contact 88 % of the time.

And one reading that could have invalidated the whole thread and does not:
**`FUN_0883e64c` is `Ship_State`** (`names.tsv`, confidence 84), so reaction
#2's `== 1` gate is a **racing-state** test and not a local-player one.
Opponents take contact damage in the original, and
`oag_physics::damage::subtract`'s `craft_state != CraftState::Racing` early
return already reproduces the gate.

Full tables, the per-tick dumps and the reproduction:
`wall-bleed.md`.

## Step 6 is done: the two remaining crashes are the estimator, not the wall or the differential gate

Measured 2026-09-07 on the same post-step-5 tree (`Tuning::curvature_span =
11`). Instrumented `Driver::drive` (temporary, reverted) to print, per tick in
the two crash windows (idx 2,130-2,170 and 2,370-2,410, lone Ace, `07_Track`,
6,000 ticks, both laps): `speed`, `target`, the `curvature` `corner_target`
was called with, `steer.command`/`rate_error`, the exact `speed < target`
condition `pace::trail` gates its differential on, and the resulting
differential and symmetric brake.

**The differential-gate bug `pace.rs` already documents (`trail`'s `speed <
target` corner-entry gate) is confirmed live here too** - `gated=true` on
100% of sampled ticks in both windows, both laps, `diff=0.0000` throughout.
But it is not the cause of these two crashes: there is no overspeed for a
differential to correct, because **`brake=0.000` on every one of those same
ticks** - `throttle`'s identical `speed <= target` branch never leaves
`(1.0, 0.0)`. The craft is at full thrust into both walls; `target` never
drops below its actual speed until after the impact.

Cross-checked against `Race::racing_line().curvature(idx, span)` at spans 4,
8, 11 (what `corner_target` actually reads) and 16, on the same finished
race: **the true local peak (`span = 4`) is 1.66x higher than the windowed
`span = 11` value `corner_target` is fed, at the first corner (0.02305 vs
0.01387), and 1.85x at the second (0.02359 vs 0.01273).** Smaller than step
2's 2.5-3x understatement at idx 2,119 - a sharper, more isolated apex - but
the identical mechanism: a chord long enough to avoid `Line::curvature`'s
now-fixed seam bug is also long enough to average a tight apex down against
its shallower shoulders. `curvature_span = 11` was chosen in step 4 as the
best point in a band bounded by two field ground-truth tests going red at
span 10, not as a value proven to resolve every apex on the disc - these two
corners are the residual the compromise left standing.

Full tables, per-tick data and the two reproduction recipes (both
instrumentation additions reverted, neither committed):
`outpost7-corner-entry.md`.

## Next Steps

1. **Do not re-tune the wall response.** It is measured against the recovered
   law and agrees; step 5 is the evidence and the two tests are the guard. The
   thing that costs a craft its shield at a wall is the bounce, and the bounce
   is `-(1 + 0.4) * v_n / D` with `e` read off `body+0x388`.
2. **`Tuning::curvature_span` does not need re-sweeping on account of the wall**
   - no constant moved. Step 6 answers what was open on the span: the two
   remaining crashes are the curvature estimator understating the true local
   apex by 1.66x/1.85x even at `span = 11`, not the differential-gate bug
   (confirmed live but inert here - no overspeed exists for it to correct)
   and not the wall. A further span change is real work of its own, the same
   shape as step 4's sweep - it would need to hold the two field
   ground-truth tests green while resolving these two apexes, which is not
   guaranteed by a smaller number alone (span 10 already failed the field
   board for an unrelated craft-wedging reason). Not attempted here.
3. Do **not** trim `07_Track` out of the test's known-good list: it has always
   passed and the assertion is right. Do not move `lateral_accel`, `grip_ground`
   or `grip_air` - the first cannot reach this corner and the last two are
   authored per-craft data that the `grounded == 1.00` measurement clears.
4. **Do not judge an AI tuning change on `just` alone.** It does not run the
   disc-backed suite, and the two tests a span of 10 broke are both in it. Run
   `just test-data`; only `shuriken_ground_truth` and `stall_rescue_ground_truth`
   should be red.

## From the HANDOVER.md index (moved 2026-09-25)

steps 1-6 done by 2026-09-07 and all in the thread file: the loss is kinematic (a line of curvature `k` admits `1.55 / k`, 07 peaks at 0.047), the yaw-rate cap is in `driver::pace::corner_target`, `curvature_span` is 11 (10 wins solo and turns two field tests red), the wall bleed is exactly `0.035` and the stopping is 73 % bounce / 27 % friction. 07 now reaches lap 4 at 28-30 shield a lap. **Still open**: `corner_target`'s span-11 curvature understates the two remaining crash apexes (idx 2,158 / 2,395) by 1.66-1.85x; a span change that resolves them without re-breaking the field board is unmeasured. Do not trim `07_Track` from the known-good list; do not move `lateral_accel`, `grip_ground` or `grip_air`.
