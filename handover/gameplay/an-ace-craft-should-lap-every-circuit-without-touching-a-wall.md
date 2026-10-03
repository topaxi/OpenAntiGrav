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

## Step 6 is done: the airbrake never fires in the hairpin at all, and the cause is a latch

Opened on a player report from the user, watching the merged build:

> I see airbrake usage more now, but for U-turns I'd still expect it to be used
> more heavily, more akin to how one would "drift" in more classical racing
> games

**They are right, and it is worse than timid.** `what_the_airbrakes_do_through_a_hairpin`
dumps the ramped `ShipState` airbrake values tick by tick through `07_Track`'s
documented 2,150-2,199 cluster - the tightest corner on the board's worst row.
Lone Ace, VENOM, on the merged tree:

```
tick   idx    speed   L      R      imbal  brake  steer   air    gnd   contact
979    2150   99.6    0.0    0.0    0.0    0.0    86.2    0.000  1.00  WALL
...
991    2163   138.1   0.0    0.0    0.0    0.0    91.2    0.000  1.00
992    2164   114.0   0.0    0.0    0.0    0.0    99.6    0.000  1.00  WALL
```

**`L = 0.0` and `R = 0.0` on every tick.** Not capped, not small - zero. The
craft accelerates 99.6 -> 138.1 units/s into the corner on full thrust, at full
lock, `grounded` 1.00 and `time_airborne` 0.000 throughout, and grinds the wall
for the rest of the window. No airbrake of any kind is commanded.

**`Tuning::trail_max` is therefore not the constraint.** It is a ceiling on a
quantity that never leaves zero, and 0.6 versus 1.0 cannot matter to it. That
was the first candidate and it is ruled out.

### Which gate, measured

Temporary instrumentation on `pace::trail`'s five gates (added, run, reverted -
not committed), 6,000 ticks:

| gate | rejects | share |
| --- | ---: | ---: |
| `recovering` | 0 | - |
| `curvature <= trail_curvature_floor` | 0 | - |
| **`curvature < peak * trail_exit_decay`** | **4,486** | **74.8 %** |
| `steer.command < trail_saturation` | 1,287 | 21.5 % |
| `rate_error <= trail_deadband` | 0 | - |
| **fires** | **227** | **3.78 %** |

Through the hairpin itself **four of the five gates pass**: `cmd = 1.000` -
full lock - with `rate_error` 0.22-0.53 rad/s, which is exactly the state a
differential exists for. The fifth rejects it: `k = 0.0139` against an exit
bound of `0.0320`, less than half.

### The root cause: the high-water mark never resets

`pace::track_peak_curvature` resets the mark when the curvature falls to
`Tuning::trail_curvature_floor` - "since the line last went straight". **On this
disc's geometry the line never goes straight by that definition.** Over the same
6,000 ticks the smallest windowed curvature seen is **0.00127** against a floor
of **0.00100**, and the reset branch fires **zero** times. The mark is a monotone
running maximum over the whole race - 81 distinct values, all increasing,
latching at `0.04566` on the circuit's tightest corner.

So the exit gate's real meaning is **"this is not the tightest corner seen so far
this race"**, which after lap one is true almost everywhere. That is the whole of
what the user is seeing.

### The fix works as a mechanism and does not pay as a tuning

`Tuning::trail_peak_decay` makes the mark leaky. At `0.99` on the same circuit
the exit gate's share falls **74.8 % -> 24.2 %** and the differential fires
**3.78 % -> 11.58 %**, three times as much airbrake. Board-wide, 48 rows, charge
capped at each row's pool:

| decay | contact ticks | charged | end pool | respawns | eliminated | clean laps | mean lap |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| **1.0 (latch)** | **10,924** | **2,277.3** | **2,191.6** | 35 | **11** | 44/48 | 38.88s |
| 0.998 | 14,228 | 2,373.4 | 2,057.1 | 40 | 10 | 44/48 | 38.95s |
| 0.995 | 13,902 | 2,317.3 | 2,129.6 | 34 | 12 | 43/48 | 38.76s |
| 0.99 | 14,973 | 2,339.5 | 2,112.7 | 34 | 12 | 43/48 | 38.72s |
| 0.95 | 13,917 | 2,302.2 | 2,181.8 | 34 | 11 | 45/48 | 38.72s |

**Every leak value costs contact ticks and end-of-run shield and buys 0.16 s of
mean lap.** Shipped at `1.0`, which changes nothing, with the sweep beside the
constant.

**Why it does not pay, which is the useful half.** `trail` spends the
differential *reactively*, once `trail_saturation` says the steering loop has
already run out of authority - and `trail`'s own doc records that it "cuts
lateral grip exactly as hard as holding both sides would". Spending grip without
**banking** the higher corner speed it permits is a pure loss, and the speed is
only banked if `corner_target` raises its target *because* the differential is
planned (`v = omega_steer / (k - C)`, step 5). It does not. **This is board
evidence for the design change step 5 scoped out**, not an argument against it.

### And a second reason the hairpin stays shut, which is already on the board

Even with the leak, `07`'s hairpin is still exit-gated: `k = 0.0139` against a
bound of `0.0170`. The windowed curvature **falls monotonically** through the
whole window - `0.01387 -> 0.00770` - so the estimator tells the driver the
corner is opening up while the craft is at full lock hitting a wall. That is the
**same estimator understatement Outpost 7 step 6 named** (true local apex 1.66x
and 1.85x the span-11 reading), now seen feeding a second consumer. The exit
gate's logic is not wrong here; its input is.

## Step 7 is done: the estimator's two knobs are both exhausted, measured

Step 6 left the curvature estimator blocking **two** consumers - `corner_target`'s
speed (1.66x/1.85x under the true apex) and `trail`'s exit gate (reporting a
hairpin as *opening* while the craft is at full lock into a wall). Before
building anything on top of it, both of its knobs were swept on the current
tree with `sweep_curvature_span`, whose field columns are the guard a span of 10
tripped in step 4.

**Knob 1, the span's value.** Re-swept because the tree has moved a long way
since step 4 - the seam fix, the per-craft yaw ceiling, `look_speed`:

| span | solo total | respawns | clean | mean lap | field worst (floor 0.45) |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 4 | 860.75 | 3 | 11 | 44.4s | 0.80 |
| 6 | 830.42 | 2 | 11 | 43.6s | 0.74 |
| 8 | 827.14 | 11 | 10 | 44.6s | **0.38** |
| **11** | **907.97** | **1** | **12** | 43.6s | 0.57 |

**`11` is still the best and still the only span with twelve clean laps**, and
`8` still turns a committed field ground-truth test red. The value axis is
exhausted for the third time, now against the current tree.

**Knob 2, the estimator's shape - and this one had never been tried.**
`Line::max_curvature` advances by the same number it measures with, so one
constant sets both the **resolution** of each reading and the **density** of
the sampling, and shrinking it does both at once. `Line::max_curvature_stepped`
separates them and `Tuning::curvature_chord` reaches it:

| chord | step | solo total | respawns | clean | mean lap | field worst |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 4 | 11 | 844.01 | 5 | 11 | 44.7s | 0.54 |
| 6 | 11 | 815.53 | 10 | 10 | 44.4s | 0.59 |
| 8 | 11 | 798.39 | 10 | 10 | 44.4s | 0.63 |

**Decoupling does not help either, and it says which half the cost is.**
Holding the step at 11 and shortening only the chord scores *below* shortening
both, at every value tried - 844.01 against 860.75 at 4, 815.53 against 830.42
at 6, 798.39 against 827.14 at 8. So the short chord's cost is its
**resolution**, not its sampling density. Decoupling does repair one thing: a
chord of 8 at step 11 lifts the field floor from a red `0.38` to `0.63`.

`curvature_chord` ships at `None`, which makes
`max_curvature_stepped(i, d, span, span)` bit-identical to the old
`max_curvature`. It is kept because it is the apparatus this negative was
measured with, and because the one repair it does make is worth knowing about.

**What this retires**: the chord-length theory of the understatement. The defect
is real and measured, and neither knob this estimator has can fix it without
costing more than it saves. **What is left is a different estimator, not a
different number** - and that is the open item, no longer a guess at which
constant to move.

## Step 8 is done: the two changes only pay together, and `05_Track`'s line blocks them

Two cheap measurements before building a third estimator. One positive, one
negative, and between them they replace "build a better estimator" with a named
dependency.

### The attribution test: positive, and larger than predicted

The parked `ai/planned-differential` branch run with `curvature_chord = 4`.
48 rows, charge capped at each row's pool:

| | main | plan+hold | +chord 4 | +chord 6 | +chord 8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| contact ticks | 10,924 | 9,136 | 10,557 | **8,683** | 9,228 |
| charged | 2,277.3 | 2,419.0 | **2,022.5** | 2,158.6 | 2,240.0 |
| end-of-run pool | 2,191.6 | 2,036.4 | **2,394.0** | 2,325.4 | 2,206.2 |
| respawns | 35 | 24 | **20** | 38 | 36 |
| eliminated | 11 | 14 | **8** | 9 | 12 |
| clean laps | 44/48 | 44/48 | **45/48** | 43/48 | 43/48 |
| mean clean lap | 38.9s | **38.1s** | 39.0s | **38.1s** | 38.3s |

Eliminations go 14 -> **8**, below main's 11, and chord 4 beats main on *every*
safety column at lap-time parity.

**The important part is not the numbers but the shape.** Chord 4 alone measured
**worse** than the shipped span (step 7: 844.01 against 907.97). The planned
differential alone measured **deadlier** (11 -> 14 eliminations). Together they
are the best board this thread has produced. A sharper estimate tells the driver
a corner is tighter; the planned differential is the yaw that makes acting on
that reading possible. **Neither pays alone and both pay together**, which is
why three separate sweeps of the estimator on its own kept coming back negative.

### The ramp: negative, and the hypothesis is retired

Widening the hairpin probe to idx 2,080-2,199 shows the differential already at
its target `-20.8` and **steady from well before the apex**. The ramp climbs at
**15.0 per tick** - 0 to 20.8 in 1.4 ticks, 0 to 100 in under 7 - so there is no
arming-lead deficit to find. "Fewer contacts, harder ones" is not the ramp.

### The arithmetic, corrected

This thread previously wrote that a chord triple spreads the apex's turn across
`3 * span` of travel. **That is wrong and the code says so**: `Line::curvature`
divides by `(|into| + |out_of|) * 0.5`, roughly **one** span, and its own doc
records that dividing by the whole travelled distance was a real bug worth
1.41x. Backing the measured 1.66-1.85x out of a denominator near 11: **the
apex's turn accumulates over about 6 units**, which at 0.89-1.77 units between
samples is **4 to 7 samples**. A derived target for any minimum-interval knob,
not a free parameter.

### Why it cannot ship, and it is not its own fault

With `curvature_chord = 4` as the default,
`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
goes **red**. `05_Track` at VENOM turns a clean 39.6s lap into a craft destroyed
inside **one** lap - 2,314 contact ticks in 4,976 racing ticks, no clean lap,
wedging at driver index **200-249**.

That overlaps `05`'s recorded unsupported racing-line run at **161-211** (see
`race_ground_truth.rs`'s own table, 134 samples over 161-211 and 811-893). The
sharper chord resolves an apparent curvature spike in a stretch of line that is
itself broken, and brakes into it. **A different thread's defect, and it is now
this thread's blocker.** Chord 6 fails the same gate on `01_Track`; chord 8
clears it and is worth almost nothing. Both field ground-truth tests stay green
at every chord value tried.

**So the prize is measured and the dependency is named**: fix `05_Track`'s
unsupported racing line, and chord 4 plus the planned differential becomes
available - worth eliminations 11 -> 8, charged -11 %, end-of-run pool +9 %,
respawns 35 -> 20 and a clean lap, at lap-time parity. That is a far more
specific next step than "try a third estimator".

## Step 9 is done: `05_Track`'s line is not broken. It is a jump, and the driver has no jump model

Step 8 left `05_Track` at VENOM as the blocker, and the working theory was that
a sharp chord *reports* a kink a blunt one averages away - so the defect was in
the line and the estimator was the messenger. **Measured, and falsified on every
count.**

`what_is_wrong_with_the_line` casts down each sample's own `down` axis from one
probe reach above and prints the four things that separate the candidates. Over
idx 195-232:

| | finding |
| --- | --- |
| splice | **none** - `ai_order` is the identity, 195 -> 195, 232 -> 232 |
| seam | **none** - spacing a uniform **1.50-1.51** units the whole run |
| kink | **none** - `k4` is `0.0009-0.0020`, `k11` is `0.0015-0.0020`; the sharp chord sees what the blunt one sees, and both are tiny |
| height | drop below the line climbs 2.7 -> 43.9 over 195-209, back to 2.3-3.0 by 213 |

**And the craft is airborne.** `grounded` is `0.00` and `time_airborne` climbs
from idx 151. This is an **authored jump**, and the 134 "unsupported" samples
the committed cast records at 161-211 are that jump. The line is correct. There
is nothing to fix in the geometry and nothing to invent.

### What actually fails

The driver has **no model of a jump**, and a sharper estimator exposes it. Lone
Ace, `05_Track`, VENOM:

| | entry speed at takeoff | airborne | speed while airborne | outcome |
| --- | ---: | ---: | ---: | --- |
| chord `None` | **118.1** | 0.93s / 166 ticks | 86.9-118.3 | lands, clean 39.6s lap |
| chord 4 | **103.8** | **4.38s / 356 ticks** | **3.1**-103.8 | falls short, destroyed |

`k4` is three to four times `k11` on the run-up at idx 150-151, so the driver
targets a lower speed, takes off **14.3 units/s slower**, and does not clear the
gap. `Driver::drive` already has an airborne branch that holds thrust and
releases the brake - and its own doc names `13_Track`'s Novice jump pathology -
but **nothing stops the driver braking *into* a takeoff**. That is the missing
half.

**This moves the blocker off the disc and into the driver**, which is the better
outcome: the measured prize behind it (eliminations 11 -> 8, charged -11 %,
end-of-run pool +9 %, respawns 35 -> 20, a clean lap, at lap-time parity) is
reachable without touching authored data.

## Step 10 is done in part: de Konstruct White, the field rather than the lone Ace (2026-10-03)

Maintainer report: "the AI really struggles with de Konstruct Black". Black is the
reversed `21_Track` and White the forward `05_Track` (maintainer, 2026-10-03); the
lane that did this work read it the other way round, so what it measured and
fixed is **White**, and the report about Black is not reproduced (Black's field
lost 1-6 of 21 craft before the speed plan, and it now drives a verified, clean
plan at all four classes). On White a lone Ace at the pole clears it; a field of
seven loses 13-19 of 21 craft over five minutes in every class, against 1-6 on
Black. Cause of the first domino: the crest before the first jump read as a corner
(see `docs/gameplay/ai.md`, "de Konstruct White"). Step 9's "no jump model" is
now one: `Line::curvature` forgives the convex pitch on the 75 units before a gap of
40+ samples, and the caution lift is skipped there. Field destroyed 64 -> 49 over
twelve cells; `05` FLASH `Eliminated` -> `CleanLap`. Still open: the wedge at
586-600 (creepers the stall rescue never sees), and a second crest at idx 399-484.
`curvature_chord = 4` was **not** retried against this - it is the first thing to
try next, since it was blocked on exactly this jump.

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

## Step 5 is done: a held airbrake is worth +24.5 % corner speed, and the grip term does not take it back

Measured 2026-09-12 by `what_a_held_airbrake_is_worth_to_the_corner_ceiling`, a
table read and a closed-form evaluation rather than a sweep - the same shape as
step 2, and for the same reason: it sizes an opportunity before any code is
written for it. Opened on the user's own words, *"airbrakes should enable
tighter turns and less slowdowns, and make laps faster if done correctly."*

**The physics already models it, and step 3's ceiling models only half of it.**
Two torques write `acc.local_angular.y` and they **add**:
`engine::steering` is `-(steer * Turning.amount)`, speed *independent*, and
`airbrake`'s is `speed * Airbrake.turn * imbalance * 0.001`, speed
*proportional*. Through the same steady state:

```text
v * k = [S + v * T] / 108      S = 100 * Turning.amount
                               T = Airbrake.turn * imbalance * 0.001
v     = (S / 108) / (k - C)    C = T / 108
```

**The airbrake subtracts from the curvature rather than raising the ceiling**,
so its gain grows as `k` approaches `C` - largest exactly at the tight apexes
where the yaw term binds, which is where `07` and `13` lose their shield.

**The sign was checked, not assumed**, because this project shipped it backwards
for months in `5ad69f3`. `imbalance = left - right_brake` on the ramped `0..=100`
states, so braking the right side harder makes it negative, and a negative
`local_angular.y` is nose-**right** here - the same sign `-(steer *
Turning.amount)` gives a right steer. **They add.**

### The numbers, all eight teams

| quantity | value | spread |
| --- | --- | --- |
| `Airbrake.turn` | **10.0** | **flat** across all 8 teams and all 4 classes |
| `Airbrake.slidegrip` | 80 XML, **0.008** scaled | flat |
| `C` at a full differential | 0.00926 | flat |
| `C / k` at `07`'s binding `k = 0.047` | **0.20** | flat |
| corner-speed multiplier there | **1.25x** | flat |

**`Airbrake.turn` is not a team axis**, unlike `Turning.amount`. One number for
the whole disc.

### The counter-term does not swap, and that was the question

`corner_target` is `min(v_grip, v_yaw)`, and a held airbrake raises `v_yaw`
while cutting `v_grip` through `slidegrip`. **At `07`'s apex the yaw term binds
by a factor of two to three and a full differential does not close the gap**:

| | `v_yaw(0)` | `v_grip(0)` | `v_grip` at full | best combined `min` | gain |
| --- | ---: | ---: | ---: | ---: | ---: |
| Feisar | 35.5 | 74.4 | 66.5 | **44.2** | **+24.5 %** |
| Assegai, AG Systems | 33.1 | 74.4 | 66.5 | **41.2** | **+24.5 %** |
| Qirex | 30.5 | 74.4 | 66.5 | **38.0** | **+24.5 %** |
| EGX, Goteki | 28.0 | 74.4 | 66.5 | **34.8** | **+24.5 %** |
| Triakis, Piranha | 25.6 | 74.4 | 66.5 | **31.9** | **+24.5 %** |

Grip retained at a full differential is `1 - 100 * (0.01 - 0.008)` = **0.80**, so
`v_grip` only falls to `sqrt(0.8)` = 89.4 % of itself - still 1.5x above the yaw
limit. The combined `min` takes the **whole** multiplier, at every team, at full
differential. **The mechanism is real, it is exploitable, and the AI does not
use it.**

### The operating rule the measurement hands over

The two terms cross, under a full differential, at:

| team | crossover `k` |
| --- | ---: |
| Feisar | 0.0289 |
| Assegai, AG Systems | 0.0270 |
| Qirex | 0.0250 |
| EGX, Goteki | 0.0231 |
| Triakis, Piranha | 0.0215 |

**Above its own crossover a craft should hold a full differential through the
apex; below it, holding one is a net loss** because the grip term binds first.
That is a per-craft number off authored data, exactly like
`hull_yaw_ceiling`, and it is roughly 20x `Tuning::trail_curvature_floor`'s
0.001 - so it is a genuinely different gate from the one `trail` uses today.

### Why the driver change is *not* in this pass

Deliberately scoped out and handed over, because it is a design change rather
than an edit, and doing it badly is worse than not doing it:

- **`corner_target` may only promise a speed the driver will actually buy.** If
  it returns `(S/108)/(k - C)` and the craft arrives without the brake held, it
  has picked a speed it cannot rotate at - which is precisely the wall contact
  this whole thread is about, with extra steps.
- **`trail` fires *reactively* today** - gated on `steer.command` saturating and
  `rate_error` exceeding a deadband, i.e. once the steering loop has already run
  out of authority. The corner speed above needs the differential armed
  **before** the apex, as a *plan*, not as a rescue.
- **`controls::update` ramps the airbrake states at `Airbrake.gain`/`falloff`**,
  so the yaw is not available on the tick it is asked for. A steady-state
  ceiling is the right first model, but the arming distance is a real cost and
  nothing here has measured it.

The board already reports clean lap time in its `clean lap` column, which is the
column to read this change against when it is built - the user's stated goal is
faster laps, and step 3 cost 0.5-3.0 s a circuit that this should give back.

## Step 11 is done: the speed plan (2026-10-03)

The drivers follow `oag_ai::SpeedPlan`, a per-sample speed learned at race
start by driving the line in our own physics, braking zones from a measured
deceleration, jumps held, and used only when its own two-lap verification
is clean. Full account, constants and measurements:
[ai.md, "The speed plan"](../../docs/gameplay/ai.md#the-speed-plan).

On this thread's own board (`ai_clean_lap_gate`, 48 forward rows):
`Eliminated` 11 -> **1**, contact ticks 8,828 -> **1,243**, 38 rows at zero
contact (was 4). Over all 24 layouts x 4 classes, lone Ace: rows with no
contact, rescue or death **7 -> 84 of 96**, laps 2.5-5 s faster per class,
end-of-run shield 34-84 -> 84-93 by class. `race_ground_truth`'s
twelve-circuit gate stays twelve clean: ten circuits lap faster (`07_Track` 49.6 -> 45.0 s, `02_Track` 43.1 -> 35.4 s), `06_Track` is unchanged at 44.0 s (no verified plan at VENOM), and `10_Track` is slower, 38.9 -> 39.8 s.

What it retired from this thread: the planned differential, the jump model,
the curvature estimator, `07_Track`'s 2,150-2,199 cluster and the respawn
residual are all moot wherever a plan verifies - the plan replaces
`corner_target` there and its differential is the driver's own. They still
matter on the layouts whose plan does not verify, which drive the corner
model unchanged.

## Step 12 is done: `05_Track` forward's plan verifies, and the walls were a pit (2026-10-03)

The walls at 424-637 that a craft "touched at 15 u/s too" are not on the
circuit. A craft that falls short of the first jump (line samples 162-210 over
a gap) lands in a pit 30-80 units under the upper road, and the windowed
locator keeps it on upper-line indices while it meets that pit's walls and
dead end (586-614: the "wedge"). It fell short because the authored line
climbs the ramp on a **magstrip** and `oag_physics::maglock` holds the craft to
the ramp's convex top: 78 u/s of climb at the lip against 87 six units right on
plain floor. Three changes, all chosen, not measured:

1. The line leaves a magstrip on a takeoff run-up (`race/takeoff_line.rs`,
   `oag_ai::line_shift`); only `05_Track` forward has one. Its plans verify at
   all four classes: `speed_plan_ground_truth` 87 -> **91 of 96**.
2. The corridor there stops short of the strip, so a pilot handed its
   character back in traffic does not wander onto it.
3. `oag_race::recovery::BENEATH_LINE`: an opponent grounded more than 15 units
   below its own line (on a supported sample) is lost after `RESCUE_TICKS`.
4. A lower level's pace share is floored at 0.96 within 450 units of a takeoff
   run-up (a Novice at VENOM hit the far lip every lap).

White field destroyed of 21 (three seeds): VENOM 15 -> 0, FLASH 13 -> 0,
RAPIER 11 -> 1, PHANTOM 10 -> 3. Lone Ace laps 11-23 % faster, within 0.7 % of
the plan's own lap, no dead stop and no trough on either layout
(`ai_dekonstruct_symptoms_ground_truth`). Full account:
[ai.md, "de Konstruct White: the first jump is a magstrip"](../../docs/gameplay/ai.md#de-konstruct-white-the-first-jump-is-a-magstrip).

## Open

- **Five layout-class plans do not verify** (`speed_plan_ground_truth.rs`
  pins the set): `06_Track` VENOM and RAPIER, `14_Track` PHANTOM, `29_Track`
  RAPIER and PHANTOM. Those rows drive the corner model.
- **`05_Track` forward's second crest**: a lone Ace at RAPIER takes the 428 pad
  (it pushes left), flies 43 units left at 157 u/s and touches a `Reset` sheet
  at 509-511, three times in 18,000 ticks; the plan's own verification never
  meets it.
- **de Konstruct Black (`21_Track`)**, the layout the maintainer named, was not
  touched by this lane's changes and shows no dead stop for any level. Its
  "slow driving" at Novice and Skilled is the level handicap: 1,171-1,725 ticks
  a run under 75 % of the plan's pace at PHANTOM, lone.
- **Three verified rows still touch**: `17_Track` VENOM (1 tick), `09_Track`
  PHANTOM (6), `25_Track` PHANTOM (21).
- **The team axis on plans**: 89 (Feisar), 87 (Assegai), 83 (Piranha) of 96
  verify; the race builds one plan from slot 1's handling, which is right only
  while every opponent flies the player's handling (`Race::start` today).
- **Field wall contact against field spread**: a pilot's character returns
  inside 40 units of a rival (sticking 2,101 -> 523 pair-ticks), which costs
  field wall contact 32,440 -> 39,804 (113,203 before the plan). The range is
  chosen; a smarter overtaking line would beat both.
- **A reset-volume respawn loop** in the field, `29_Track` VENOM: one craft
  put back at sample 45 every 46 ticks, 61 times. Race rules, not the plan.
- **`10_Track` FLASH laps 1.8 s slower** on the gate, with its contact 66 -> 0.
- **The counters charge a live craft sitting at zero shield.** The `Racing` gate
  stops a wreck, not that. Capping at the pool is the workaround the totals use;
  a cleaner counter would gate on `physics.shield > 0.0` too.
- Pure and HD are unmeasured at every class.

## Next Steps

1. **`05_Track` forward's 428 pad**: a pad whose push points off the road on a
   crest; read whether the plan should steer across it or avoid it.
2. **The two rescue cases** (`14_Track` PHANTOM at 1247, `29_Track` at
   2402-2441): the search lowers ceilings and the craft still leaves; read
   whether a held run-up or a lower one is the answer.
3. **Short-horizon rollouts** are the next lane's design question: 6,480 steps
   per full-field replan is about 150-175 ms (`examples/physics_step_cost.rs`).
4. Do **not** re-tune the wall response, `lateral_accel`, `grip_ground` or
   `grip_air` - Outpost 7 steps 1, 2 and 5 close all four.

## From the HANDOVER.md index (moved 2026-09-25)

the umbrella the narrower AI threads sit under, opened 2026-09-12 against a stated standard: a lone Ace should lap every circuit clean, in every speed class, with **no wall contact**, preferring airbrakes over lifting off. **Carries the 48-row board** - twelve forward circuits by four speed classes on Pulse PSP USA - and **four of the 48 meet the standard**: `03` at VENOM and FLASH, `02` and `10` at VENOM, each with zero wall contact. `07_Track` is the weakest row at every class and `13_Track` the second weakest. Wall contact is now a *measured* quantity (`Race::wall_contact_ticks_of` and friends), counting `WallResponse::contacts` rather than the inbound-only `wall_contact_prev` and charging `damage::contact_damage(impulse_sum)` rather than differencing a pool that barrel rolls and shield pads also move - and **gated on the craft still racing**, because `step_opponents` releases a wreck and keeps integrating it, which read as 7,157 contact ticks on one row and charged another 122.05 against a pool of 95.00. The **team** axis is answered and closed: `I_yy` is a *code literal* shared by every craft (see `oag_physics::forces::YAW_INVERSE_INERTIA`), `<Turning amount>` is per team and **constant across the four classes**, and no craft on the disc can reach `Tuning::max_turn_rate`'s 1.8 - the ceilings span 1.204 to 1.667, so the kinematic corner limit was overstated for every craft by 7 to 33 %. `corner_target` now reads the flown craft's own hull and the permission only caps it, which cuts wall-charged shield 13.5 % board-wide (capped at the pool; 18.5 % uncapped, and the difference is a live craft at zero shield still being charged) and doubles the zero-contact rows, at a cost of 0.5-3.0 s a lap and respawns rising 23 to 35. What is still open: the respawn residual, `07`'s 2,150-2,199 cluster (the curvature *estimator*, not the span), `13`, and Pure/HD at every class. [The Outpost 7 thread](../gameplay/outpost-7-loses-35-shield-a-lap-to-its-own-walls.md) stays scoped to its own circuit and its step 6 names the residual this one inherits.
