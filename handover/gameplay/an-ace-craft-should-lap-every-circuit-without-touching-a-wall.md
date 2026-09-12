# An Ace craft should lap every circuit, in every speed class, without touching a wall

Opened 2026-09-12 as the umbrella the four narrower AI threads sit under. The
standard this thread is measured against, stated by the user:

> An elite/Ace AI should be able to drive every track perfectly when alone on
> the track, without wall contact, preferring airbrakes over slowing down, and
> for each speed class.

## Why a new thread rather than a step on an existing one

[outpost-7-loses-35-shield-a-lap-to-its-own-walls.md](outpost-7-loses-35-shield-a-lap-to-its-own-walls.md)
carries six completed steps of exactly this work on **one circuit at one speed
class**. That thread stays scoped to `07_Track`; this one is the board that says
which *other* rows are in the same state, and it opened three axes that had
never been measured at all - speed class, wall contact as its own quantity, and
team.

## Step 1 is done: wall contact is now a measured quantity

`crates/game/tests/ai_clean_lap_board.rs` is the harness and
`Race::wall_contact_ticks_of` / `wall_inbound_ticks_of` /
`wall_shield_charged_of` are what it reads. Three per-slot counters on
`RaceSim`, on exactly the footing `rolls_armed`/`rolls_spent` already sit on:
bookkeeping, outside `state_hash`, written in `step_opponents` where
`Evaluated::wall` is live. The measurement cannot be taken from a test -
`RaceSim::collision` is `pub(super)` - which is why this is in `crates/game/src`
rather than in the test alone.

Two choices, both settled by measurement rather than assumed:

- **`WallResponse::contacts`, not `ShipState::wall_contact_prev`.** The second
  is `impact`, an *inbound* test (`normal_speed < 0.0`), and a craft grinding
  along a wall stops being inbound long before it stops being in contact - the
  Outpost 7 decomposition is "a 4.36 shield impact **and ~180 ticks of
  grinding**". Measured on `07_Track` at VENOM on the pre-change tree: **1,445
  contact ticks against 1,378 inbound**, so the inbound reading would have
  undercounted by 5 %. Small, but the wrong quantity.
- **`damage::contact_damage(impulse_sum)`, not a shield delta.** The pool also
  moves for barrel-roll charge and shield pads, which is the whole reason
  `07_Track` was the only circuit whose wall attrition could ever be read by
  hand. Summing what physics itself charges separates the wall term
  structurally. It is what the wall *charged*, not always what the pool *lost*:
  a craft at zero is charged the same and loses nothing, so it is reported
  beside the end-of-run pool, never instead of it.

**A contact counted here is a wall by construction.** `wall::responds` excludes
`Floor` and `MagFloor` - the hover spring owns those - so a `WallResponse`
contact can only be `Surface::Wall` or `Surface::Reset`. That is a
surface-**tag** test and strictly stronger than the `|n.up|` near-zero
geometric proxy a by-hand reading had to use.

**The counters are gated on the craft still racing, and that gate is the whole
difference between a measurement and a fiction.** `step_opponents` *releases* a
wrecked opponent rather than skipping it, so `oag_physics::step` keeps
integrating a hull that has settled against a wall - at a frozen `driver.index`.
Ungated, `10_Track` at PHANTOM read **7,157** contact ticks against 328 after a
change that halved what the walls charged it, and `07_Track` charged **122.05**
against a pool of **95.00**. Both numbers were the wreck, not the driving. The
board's `racing ticks` column is the denominator, and it is printed because a
row wrecked at tick 5,585 and a row that finished at 18,000 are not comparable
without it.

**Tick budget: 18,000 ticks holds every class, but not with room to spare.**
`Mode::SINGLE_RACE_LAPS_BY_CLASS` is `[3, 4, 4, 5]`, and a craft that crosses
its last line stops being `Racing` and coasts, so a row measured past the flag
is not comparable with one still racing. The board carries a `fin` column for
exactly that. On the post-change tree every VENOM, FLASH and RAPIER row
finishes except `07` (RAPIER and FLASH) and `13` (PHANTOM); at PHANTOM five
minutes is enough for all but `07` and `13`. **The rows that do not finish are
the rows that were destroyed**, not rows that ran out of clock.

## Step 2 is done: the team question. `Tuning::max_turn_rate` was wrong for every craft on the disc

**A table read, not a sweep**, and it answers the axis the brief called the
highest-value single check. `yaw_ceiling_by_team_and_class` in the same file.

The steady state of the yaw axis is `omega = steer * Turning.amount / (damping
* I_yy)`: `oag_physics::engine::steering` is `steer * Turning.amount` with no
speed factor, `passive::YAW_DAMPING` damps yaw *momentum* at `-5`, and `steer`
runs to `controls::CONTROL_RANGE` = 100.

**`I_yy` is not per-craft, and the premise this thread opened under said it
was.** `forces::YAW_INVERSE_INERTIA`'s own disassembly settles it: the inertia
box `(12, 8, 12)` and the mass `0.9` it is built with are **code literals at a
single call site** in the ship-entity constructor, and `Misc`
`width`/`length`/`height` reach the *collider*, not the tensor. Every craft in
the game has `I_yy = 21.6`. The only per-craft term is `<Turning amount>`.

Read off `handlingstats.xml` for all eight teams and all four classes:

| team | `Turning.amount` | ceiling (rad/s) | against `Tuning::max_turn_rate` 1.8 |
| --- | ---: | ---: | ---: |
| Feisar | 1.80 | 1.6667 | -7.4 % |
| Assegai, AG Systems | 1.68 | 1.5556 | -13.6 % |
| Qirex | 1.55 | 1.4352 | -20.3 % |
| EGX, Goteki | 1.42 | 1.3148 | -27.0 % |
| Triakis, Piranha | 1.30 | 1.2037 | -33.1 % |

Two findings, and both change what the board has to track:

1. **`Turning.amount` is identical across the four speed classes** on every one
   of the eight teams. So the team axis and the class axis are independent, and
   the board does not need a team column per class.
2. **The spread between teams is 38.5 %, and no craft reaches 1.8.** The
   driver's kinematic corner limit was asking every craft on the disc for a
   speed its hull could not rotate at - by 7 % on the best team and 33 % on the
   worst. The 1.5556 row is the one Outpost 7 step 2 measured by hand, and
   `docs/ghidra/functions/psp-pulse-usa/engine.md` measures the *original* hull
   at 1.42-1.51 under full lock, so this arithmetic lands within a few per cent
   of the original's own behaviour.

**This settles the tuning doc's own caveat** - that the 1.8 sweep "flattens
either side of 1.8 rather than continuing to improve, which is the shape of a
constraint that has stopped binding". The measurement says the constant was the
wrong *shape*, not the wrong value: that sweep measured the *steering clamp*,
where 1.8 genuinely has stopped binding, while the same constant was doing a
second, unrelated job in `corner_target` where it never could bind correctly
because it was above every hull on the disc. The fix is the split, not a new
number - see step 3.

## Step 3 is done: `corner_target` now reads the flown craft's own hull

`Context::yaw_ceiling` carries it, `oag_ai::hull_yaw_ceiling(&Handling)` derives
it, and `corner_target` takes `min(hull, tuning.max_turn_rate)` - the
permission still caps, so `Difficulty::tune`'s ladder survives and a Novice does
not corner like an Ace. `None` is exactly the pre-change reading, which is what
the synthetic closed-loop tests stay on.

**`Driver::steering`'s clamp is deliberately left on `Tuning::max_turn_rate`.**
That one is a permission on what pure pursuit may *request* during a recovery,
and the 1.2 row of its own `16_Track` sweep is the measurement that says
lowering it makes excursions worse. The two uses are different questions and
now read different numbers.

Board-wide, on the full 48 rows:

| class | contact ticks | wall-charged (capped) | end-of-run pool | respawns | eliminated | racing ticks |
| --- | --- | --- | --- | --- | --- | --- |
| VENOM | 1,252 -> **1,154** | 255.5 -> **224.4** (-12.2 %) | 838.9 -> **908.0** | 1 -> 1 | 1 -> 1 | 210,968 -> **212,844** |
| FLASH | 2,008 -> **3,305** | 476.7 -> **432.5** (-9.3 %) | 655.7 -> **684.7** | 1 -> **7** | 2 -> 2 | 205,552 -> **206,865** |
| RAPIER | 3,124 -> **2,764** | 869.6 -> **726.7** (-16.4 %) | 255.9 -> **382.9** | 4 -> **13** | 5 -> **3** | 180,175 -> **195,135** |
| PHANTOM | 4,627 -> **3,701** | 1032.2 -> **893.7** (-13.4 %) | 101.4 -> **216.0** | 17 -> **14** | 4 -> **5** | 185,171 -> **173,027** |

**The totals above cap each row's charge at its own 95-unit pool, and the
uncapped ones are different enough to matter.** The `Racing` gate stops a
*wreck* accruing; it does not stop a craft that is still racing at **zero**
shield, where `damage::subtract` clamps the loss to nothing while the counter
keeps charging. Uncapped, the boards carry 277.3 and 96.5 units of over-pool
charge and the cut reads 18.5 %; capped it is **13.5 %** (2,633.9 -> 2,277.3),
and part of even that is fewer rows being ground alive at zero rather than fewer
walls hit. Capped is the honest number.

Charged shield falls at every class, the
end-of-run pool rises at every class, contact ticks fall at three of four, and
**zero-contact rows double, 2 -> 4** (`02` and `10` at VENOM join `03` at VENOM
and FLASH). Eliminations go 12 -> 11.

The price is lap time, 0.5-3.0 s slower per circuit, and **respawns, which rise
23 -> 35 board-wide** - the craft brakes earlier, wedges at low speed and is
rescued rather than crashing at speed. `01_Track` at RAPIER is the worst row: it
stops being eliminated and starts taking ten recoveries with no clean lap at
all. Two rows die that did not: `05` at FLASH and `10` at PHANTOM.

**`charged` saturates on a dead row.** Every eliminated row reads ~95, the whole
pool, so it does not discriminate between them - `racing ticks` does, and lower
is worse. That column is also the denominator the others are counted over. A
*live* row can read over 95 (`01` at RAPIER 121.36, at PHANTOM 162.56): that is
the same clamp seen from the other side, and it is why the class totals are
capped.

All three hard gates hold:
`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
is twelve clean with `01_Track`'s single pre-existing respawn still at driver
index **794**, and both field ground-truth tests
(`every_opponent_that_laps_has_a_lap_time`,
`a_field_racing_with_real_pads_does_not_mine_itself_to_death`) are green.

## The board

Per-circuit, per-class, lone Ace, `Mode::SingleRace`, Pulse PSP USA, 18,000
ticks, **on the post-step-3 tree**. `ticks` is wall-contact ticks, `charged` is
what those contacts charged the pool, `cluster` is the worst 50-sample
driver-index bucket and what it charged, `end` is the pool at tick 18,000.
Read `charged` and `end` as different scales - and note that a row that dies
early accumulates *fewer* contact ticks than one that survives grinding, so
`ticks` alone ranks nothing.

| circuit | class | clean lap | resp | state | racing ticks | contact ticks | charged | worst cluster | end |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | --- | ---: |
| 03 | VENOM | 43.8s | 0 | Racing | 18000 | 0 | 0.00 | - | 95.00 |
| 02 | VENOM | 43.5s | 0 | Racing | 18000 | 0 | 0.00 | - | 95.00 |
| 10 | VENOM | 38.4s | 0 | Racing | 18000 | 0 | 0.00 | - | 87.40 |
| 09 | VENOM | 51.5s | 0 | Racing | 18000 | 11 | 1.81 | 3000-3049 @1.4 | 93.19 |
| 14 | VENOM | 45.8s | 0 | Racing | 18000 | 11 | 1.88 | 2400-2449 @1.9 | 93.12 |
| 16 | VENOM | 42.1s | 0 | Racing | 18000 | 57 | 10.50 | 3150-3199 @10.5 | 84.50 |
| 05 | VENOM | 39.6s | 0 | Racing | 18000 | 60 | 11.40 | 2400-2449 @9.3 | 83.60 |
| 04 | VENOM | 39.3s | 0 | Racing | 18000 | 84 | 14.70 | 950-999 @14.7 | 80.30 |
| 06 | VENOM | 46.8s | 0 | Racing | 18000 | 109 | 16.19 | 1350-1399 @14.7 | 78.81 |
| **01** | VENOM | **43.6s** | **1** | **Racing** | 18000 | 121 | 24.38 | 500-549 @9.9 | 70.62 |
| 13 | VENOM | 38.1s | 0 | Racing | 18000 | 322 | 48.57 | 1600-1649 @25.5 | 46.43 |
| **07** | VENOM | **50.7s** | **0** | **Eliminated** | 14844 | 379 | 95.11 | 2150-2199 @27.1 | 0.00 |
| 03 | FLASH | 39.8s | 0 | Racing | 18000 | 0 | 0.00 | - | 95.00 |
| 02 | FLASH | 40.1s | 0 | Racing | 18000 | 23 | 3.89 | 50-99 @2.0 | 91.11 |
| 10 | FLASH | 36.0s | 0 | Racing | 18000 | 46 | 13.87 | 2550-2599 @5.6 | 81.13 |
| 09 | FLASH | 47.2s | 0 | Racing | 18000 | 49 | 11.10 | 3050-3099 @4.9 | 83.90 |
| 14 | FLASH | 40.7s | 0 | Racing | 18000 | 75 | 15.59 | 2400-2449 @6.4 | 79.41 |
| 04 | FLASH | 35.2s | 0 | Racing | 18000 | 85 | 23.73 | 950-999 @18.2 | 71.27 |
| 06 | FLASH | 43.6s | 0 | Racing | 18000 | 93 | 17.93 | 1350-1399 @15.6 | 77.07 |
| 16 | FLASH | 38.3s | 0 | Racing | 18000 | 93 | 21.67 | 3150-3199 @14.6 | 73.33 |
| **01** | FLASH | **35.7s** | **5** | **Racing** | 18000 | 227 | 52.23 | 500-549 @14.8 | 27.57 |
| **07** | FLASH | **48.2s** | **0** | **Eliminated** | 10010 | 368 | 96.32 | 2150-2199 @24.8 | 0.00 |
| 13 | FLASH | 34.4s | 0 | Racing | 18000 | 474 | 90.12 | 1600-1649 @32.4 | 4.88 |
| **05** | FLASH | **36.0s** | **2** | **Eliminated** | 16855 | 1772 | 87.41 | 600-649 @39.3 | 0.00 |
| 03 | RAPIER | 36.4s | 0 | Racing | 18000 | 14 | 4.44 | 2150-2199 @2.1 | 90.56 |
| 02 | RAPIER | 37.2s | 0 | Racing | 18000 | 92 | 21.24 | 50-99 @12.3 | 73.76 |
| 06 | RAPIER | 41.2s | 0 | Racing | 18000 | 151 | 33.76 | 1350-1399 @22.1 | 61.24 |
| 05 | RAPIER | 32.0s | 0 | Racing | 18000 | 162 | 43.71 | 2400-2449 @21.4 | 51.29 |
| 10 | RAPIER | 32.1s | 0 | Racing | 18000 | 175 | 63.87 | 2550-2599 @17.7 | 15.93 |
| 16 | RAPIER | 34.7s | 0 | Racing | 18000 | 188 | 59.22 | 3150-3199 @25.4 | 35.78 |
| 09 | RAPIER | 43.3s | 0 | Racing | 18000 | 207 | 45.23 | 3050-3099 @8.6 | 49.77 |
| 04 | RAPIER | 31.2s | 0 | Racing | 18000 | 315 | 82.81 | 950-999 @30.8 | 4.59 |
| **07** | RAPIER | **45.5s** | **0** | **Eliminated** | 7128 | 329 | 95.22 | 2150-2199 @19.2 | 0.00 |
| **14** | RAPIER | **36.2s** | **0** | **Eliminated** | 16091 | 346 | 87.42 | 2400-2449 @36.6 | 0.00 |
| **13** | RAPIER | **31.9s** | **3** | **Eliminated** | 10279 | 381 | 95.39 | 2850-2899 @20.5 | 0.00 |
| **01** | RAPIER | **none** | **10** | **Racing** | 17637 | 404 | 121.36 | 500-549 @28.5 | 0.00 |
| 03 | PHANTOM | 34.9s | 0 | Racing | 18000 | 77 | 24.46 | 3350-3399 @16.8 | 70.54 |
| 02 | PHANTOM | 35.8s | 0 | Racing | 18000 | 131 | 39.47 | 1100-1149 @16.9 | 55.53 |
| **10** | PHANTOM | **31.0s** | **0** | **Eliminated** | 10765 | 180 | 79.87 | 2550-2599 @14.9 | 0.00 |
| 05 | PHANTOM | 29.6s | 0 | Racing | 18000 | 202 | 61.46 | 2400-2449 @22.3 | 33.54 |
| 16 | PHANTOM | 33.0s | 0 | Racing | 18000 | 234 | 85.60 | 3150-3199 @30.9 | 9.40 |
| **13** | PHANTOM | **none** | **4** | **Eliminated** | 9025 | 272 | 95.32 | 2850-2899 @20.0 | 0.00 |
| 09 | PHANTOM | 41.7s | 0 | Racing | 18000 | 285 | 72.44 | 1200-1249 @12.9 | 22.56 |
| **14** | PHANTOM | **33.9s** | **0** | **Eliminated** | 11045 | 327 | 95.16 | 2400-2449 @29.1 | 0.00 |
| **04** | PHANTOM | **29.8s** | **0** | **Eliminated** | 11600 | 352 | 95.01 | 950-999 @23.4 | 0.00 |
| **01** | PHANTOM | **none** | **10** | **Racing** | 17007 | 480 | 162.56 | 500-549 @33.3 | 0.00 |
| 06 | PHANTOM | 40.8s | 0 | Racing | 18000 | 493 | 55.37 | 1350-1399 @19.4 | 24.43 |
| **07** | PHANTOM | **none** | **0** | **Eliminated** | 5585 | 668 | 95.05 | 750-799 @19.0 | 0.00 |

**Four of 48 rows meet the standard** - `03` at VENOM and FLASH, `02` and `10`
at VENOM, all with zero wall contact. Every other row touches a wall.

Pure and HD have not been run: the brief's own scope is Pulse for the full grid
and the other titles only where a problem shows, and there are thirty problem
rows on Pulse to work through first. Teams are not on the board because step 2
found `Turning.amount` class-invariant and now *derived* - the team axis is
accommodated rather than tracked, and the check that it stayed accommodated is
the derivation itself.

## Step 4: "prefers airbrakes over slowing down" - read, not built

The interpretation is settled by the code, and it was verified against
`oag_physics::controls::update` rather than taken from `pace.rs`'s doc comment
alone. `update` ramps `ShipState::brake` toward `CONTROL_MAX` only while **both**
airbrake inputs are strictly positive, and the ramp rate is
`Brakes::gain`/`falloff` - it never reads the commanded *level*. So a command of
`0.35` and a command of `1.0` decelerate identically. What climbs with the
command is `max(L, R)`, which is what the lateral-grip coefficient reads.

**In this engine the airbrake is the brake.** There is no second deceleration
mechanism, and the brake level is a dial on how much cornering grip the
deceleration is bought with. The only operational content the user's preference
can have is: *hold thrust and airbrake, rather than cut thrust to zero*. Today
`pace::throttle` returns `(0.0, 0.0)` in the margin band and `(0.0, brake)` past
it - thrust cut to zero in both.

**Not built, deliberately, and the reason is in the evidence.** Outpost 7 step 6
measured `brake = 0.000` and `throttle` at `(1.0, 0.0)` on *every* sampled tick
in both `07_Track` crash windows: the brake gate never opens there, so a policy
change behind it cannot reach those rows and would be tuning a path that never
fires. It is the right next experiment once a row is found whose loss happens
*with the brake on* - the board's `charged` column is now the thing that can
find one.

## The gate, as it stands on this branch

`just` is green in full. `OAG_REQUIRE_GAME_DATA=1 just test-data` is **4,185
run, 4,137 passed, 48 failed, and all 48 are 2048/Vita or Pure-DLC** on a
checkout that has neither image - `oag-rcs::start_gantry_2048` and
`psp2_rcsmodel*`, `oag-game::vita_2048_*`, `race_remix`, `zone_grade`,
`pure_dlc`, `oag-vex::kdcol`, `oag-texture::gxt`, `oag-rcs::gxp`,
`oag-tables::effectsettings`, `oag-vex::shadow_occluder`. Nothing in AI,
physics, race, weapons or lap times is red. `shuriken_ground_truth` and
`stall_rescue_ground_truth`, which the handover expected to be red, both passed.

**`check-test-budget` fired and it is not settled**: 710 s against the 450 s
ceiling, five `ai_roll_ground_truth` grid tests reported at 545-578 s. The
machine was never idle - load average 54 during the run, 68 and then 138 while
the A/B was attempted - which is exactly the contended reading CLAUDE.md's own
worked example for this ceiling describes, using these same tests. Re-measured
in isolation at load 6.84 the test is **174.7 s**, under the 300 s per-test
ceiling but still 2.2x CLAUDE.md's recorded 76-79 s. The A/B that would settle
it (the same binary with `yaw_ceiling` back to `None`) was abandoned rather than
reported from a load-138 machine. Mechanism argues against it being the yaw
change - one `min()` per craft per tick, and contact ticks are *down* at three
of four classes - but that is an argument, not a measurement.

## Open

- **Respawns are the new residual.** Board-wide 23 -> 35 under step 3, with
  `01_Track` at RAPIER and PHANTOM the worst rows (ten recoveries each, no clean
  lap). The mechanism is the trade the change makes: a craft that brakes earlier
  wedges at low speed and is rescued, where before it crashed at speed and was
  charged for it. Whether the answer is in the driver or in
  `Race::lost_off_the_circuit`/the stall rescue is unread.
- **`07_Track` is still the weakest row on the board at every class.** Step 3
  moved it barely at all at VENOM - 375 -> 379 contact ticks, both rows charged
  out at ~95 - but it does survive 1,876 ticks longer (12,968 -> 14,844) and it
  is still destroyed. Its worst cluster is **2,150-2,199** at VENOM, FLASH and
  RAPIER, which is exactly where Outpost 7 step 6 left it. Its named residual is
  unchanged: `corner_target`'s windowed curvature understates the true local
  apex by 1.66x/1.85x even at `curvature_span = 11`. A different *estimator* -
  the max over a short span rather than a chord over a long one - is the untried
  idea, and it is real work of its own that has to hold both field ground-truth
  tests green.
- **`13_Track` is the second-worst row at every class** and has never been
  decomposed. Its VENOM and FLASH cluster is 1,600-1,649 and at RAPIER and
  PHANTOM it moves to 2,850-2,899, so it is not one corner.
- **`05_Track` at FLASH is a new elimination** that VENOM does not show, cluster
  600-649. `05` is also the circuit with 134 unsupported racing-line samples
  (161-211, 811-893) recorded in `race_ground_truth.rs` - a **different thread**,
  noted here because 600-649 is not one of those runs and so is probably not it.
- **Lap time is the unmeasured cost of step 3.** 0.5-3.0 s a circuit, and
  nothing on the board or in the gate asserts on it. A row that laps 3 s slower
  and survives is better by this thread's standard and worse by a racer's; no
  measurement here separates "the AI is slower" from "the AI is correctly
  slower".
- **`Tuning::curvature_span = 11` is no longer supported by the sweep that chose
  it.** That table - and its criterion, "the best point in a band bounded by two
  field tests going red at span 10" - was measured with `corner_target` capping
  at 1.8. It now caps at 1.204-1.667 depending on team, so every row of it
  measured a different function from the one shipping. Nothing is broken (both
  field tests are green at 11 today), but the *choice* wants re-establishing,
  and Next Step 2 has to do it anyway.
- **The counters charge a live craft sitting at zero shield.** The `Racing` gate
  stops a wreck, not that. Capping at the pool is the workaround the totals use;
  a cleaner counter would gate on `physics.shield > 0.0` too.
- Pure and HD are unmeasured at every class.

## Next Steps

1. **Attack `07_Track`'s 2,150-2,199 cluster.** It is the largest bucket on the
   board's worst row at three of four classes, and it is the corner Outpost 7
   step 6 already decomposed - so the next move there is the estimator below,
   not another measurement of the same window.
2. **The estimator**, not the span: `corner_target` is fed
   `Line::max_curvature(index, window, span)`, a chord long enough to dodge the
   seam bug and therefore long enough to average an apex down against its
   shoulders. Try the max over a short span. Hold both field ground-truth tests
   green - span 10 failed them for an unrelated craft-wedging reason, so a
   smaller number alone is not a guarantee.
3. **Then the respawn residual**, which is now a larger board-wide loss than it
   was (23 -> 35), and the two rows that newly die: `05` at FLASH (cluster
   600-649) and `10` at PHANTOM (cluster 2,550-2,599).
4. Do **not** re-tune the wall response, `lateral_accel`, `grip_ground` or
   `grip_air` - Outpost 7 steps 1, 2 and 5 close all four.
