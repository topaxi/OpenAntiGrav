# Outpost 7 loses 34-35 shield a lap to its own walls, and it is not the line

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

## Open

- **Which of the two remaining causes binds.** The craft's yaw rate is pinned
  at 1.55-1.62 rad/s against `Tuning::max_turn_rate` 1.8 through the whole
  departure, *and* its velocity is further off the line's tangent than its body
  is (53 degrees against 30), so the steering is near saturation and the
  lateral grip is not holding. Which one to move is a design call and was
  deliberately not made.
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

1. Measure `oag_physics::wall::resolve` in isolation: a craft at 100 units/s
   grazing a flat wall at 28 degrees, one contact a tick, and compare the
   tangential bleed against the recovered 0.035. That is the one open question
   with a committed evidence page behind it, and it decides whether the grind's
   ~5 shield a lap is legitimate.
2. Only then look at the steering saturation. Do **not** trim `07_Track` out of
   the test's known-good list: it has always passed and the assertion is right,
   and do not move `lateral_accel` - the probe above shows it does not touch
   this corner.
