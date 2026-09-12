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

| class | wall-charged shield | end-of-run pool | respawns | eliminated |
| --- | --- | --- | --- | --- |
| VENOM | 260.5 -> **251.5** | 838.9 -> **908.0** | 1 -> 1 | 1 -> 1 |
| FLASH | 488.1 -> **438.2** | 655.7 -> **684.7** | 1 -> **7** | 2 -> 2 |
| RAPIER | 913.1 -> **776.5** | 255.9 -> **382.9** | 4 -> **13** | 5 -> **3** |
| PHANTOM | 1334.3 -> **1049.9** | 101.4 -> **216.0** | 17 -> 14 | 4 -> **5** |

Charged shield falls at every class (2,996 -> 2,516, a 16 % cut) and the
end-of-run pool rises at every class. **Two more circuits reach zero wall
contact at VENOM** (`02` and `10` join `03`). The price is lap time, 0.5-3.0 s
slower per circuit, and **respawns, which rise 23 -> 35 board-wide** - the
craft brakes earlier, wedges at low speed and is rescued rather than crashing
at speed. `01_Track` at RAPIER is the worst row: it stops being eliminated and
starts taking ten recoveries with no clean lap at all.

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

| circuit | class | clean lap | resp | state | contact ticks | charged | worst cluster | end |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 03 | VENOM | 43.8s | 0 | Racing | **0** | 0.00 | - | 95.00 |
| 02 | VENOM | 43.5s | 0 | Racing | **0** | 0.00 | - | 95.00 |
| 10 | VENOM | 38.4s | 0 | Racing | **0** | 0.00 | - | 87.40 |
| 09 | VENOM | 51.5s | 0 | Racing | 11 | 1.81 | 3000-3049 @1.4 | 93.19 |
| 14 | VENOM | 45.8s | 0 | Racing | 11 | 1.88 | 2400-2449 @1.9 | 93.12 |
| 16 | VENOM | 42.1s | 0 | Racing | 57 | 10.50 | 3150-3199 @10.5 | 84.50 |
| 05 | VENOM | 39.6s | 0 | Racing | 60 | 11.40 | 2400-2449 @9.3 | 83.60 |
| 04 | VENOM | 39.3s | 0 | Racing | 84 | 14.70 | 950-999 @14.7 | 80.30 |
| 06 | VENOM | 46.8s | 0 | Racing | 109 | 16.19 | 1350-1399 @14.7 | 78.81 |
| 01 | VENOM | 43.6s | **1** | Racing | 121 | 24.38 | 500-549 @9.9 | 70.62 |
| 13 | VENOM | 38.1s | 0 | Racing | 322 | 48.57 | 1600-1649 @25.5 | 46.43 |
| **07** | VENOM | 50.7s | 0 | **Eliminated** | 3119 | 122.05 | 650-699 @37.7 | 0.00 |
| 03 | FLASH | 39.8s | 0 | Racing | **0** | 0.00 | - | 95.00 |
| 02 | FLASH | 40.1s | 0 | Racing | 23 | 3.89 | 50-99 @2.0 | 91.11 |
| 10 | FLASH | 36.0s | 0 | Racing | 46 | 13.87 | 2550-2599 @5.6 | 81.13 |
| 14 | FLASH | 40.7s | 0 | Racing | 75 | 15.59 | 2400-2449 @6.4 | 79.41 |
| 04 | FLASH | 35.2s | 0 | Racing | 85 | 23.73 | 950-999 @18.2 | 71.27 |
| 06 | FLASH | 43.6s | 0 | Racing | 93 | 17.93 | 1350-1399 @15.6 | 77.07 |
| 16 | FLASH | 38.3s | 0 | Racing | 93 | 21.67 | 3150-3199 @14.6 | 73.33 |
| 09 | FLASH | 47.2s | 0 | Racing | 49 | 11.10 | 3050-3099 @4.9 | 83.90 |
| 01 | FLASH | 35.7s | **5** | Racing | 227 | 52.23 | 500-549 @14.8 | 27.57 |
| 13 | FLASH | 34.4s | 0 | Racing | 474 | 90.12 | 1600-1649 @32.4 | 4.88 |
| **05** | FLASH | 36.0s | **2** | **Eliminated** | 1799 | 87.46 | 600-649 @39.3 | 0.00 |
| **07** | FLASH | 48.2s | 0 | **Eliminated** | 1337 | 100.61 | 2150-2199 @24.8 | 0.00 |
| 03 | RAPIER | 36.4s | 0 | Racing | 14 | 4.44 | 2150-2199 @2.1 | 90.56 |
| 02 | RAPIER | 37.2s | 0 | Racing | 92 | 21.24 | 50-99 @12.3 | 73.76 |
| 06 | RAPIER | 41.2s | 0 | Racing | 151 | 33.76 | 1350-1399 @22.1 | 61.24 |
| 05 | RAPIER | 32.0s | 0 | Racing | 162 | 43.71 | 2400-2449 @21.4 | 51.29 |
| 10 | RAPIER | 32.1s | 0 | Racing | 175 | 63.87 | 2550-2599 @17.7 | 15.93 |
| 16 | RAPIER | 34.7s | 0 | Racing | 188 | 59.22 | 3150-3199 @25.4 | 35.78 |
| 09 | RAPIER | 43.3s | 0 | Racing | 207 | 45.23 | 3050-3099 @8.6 | 49.77 |
| 04 | RAPIER | 31.2s | 0 | Racing | 315 | 82.81 | 950-999 @30.8 | 4.59 |
| **01** | RAPIER | **none** | **10** | Racing | 411 | 123.05 | 500-549 @28.5 | 0.00 |
| **14** | RAPIER | 36.2s | 0 | **Eliminated** | 2232 | 93.01 | 2400-2449 @36.6 | 0.00 |
| **13** | RAPIER | 31.9s | **3** | **Eliminated** | 6110 | 104.85 | 700-749 @22.8 | 0.00 |
| **07** | RAPIER | 45.5s | 0 | **Eliminated** | 4888 | 101.30 | 2150-2199 @19.2 | 0.00 |
| 03 | PHANTOM | 34.9s | 0 | Racing | 77 | 24.46 | 3350-3399 @16.8 | 70.54 |
| 02 | PHANTOM | 35.8s | 0 | Racing | 131 | 39.47 | 1100-1149 @16.9 | 55.53 |
| 05 | PHANTOM | 29.6s | 0 | Racing | 202 | 61.46 | 2400-2449 @22.3 | 33.54 |
| 16 | PHANTOM | 33.0s | 0 | Racing | 234 | 85.60 | 3150-3199 @30.9 | 9.40 |
| 09 | PHANTOM | 41.7s | 0 | Racing | 285 | 72.44 | 1200-1249 @12.9 | 22.56 |
| 06 | PHANTOM | 40.8s | 0 | Racing | 493 | 55.37 | 1350-1399 @19.4 | 24.43 |
| **01** | PHANTOM | **none** | **10** | Racing | 810 | 204.06 | 2500-2549 @41.8 | 0.00 |
| **04** | PHANTOM | 29.8s | 0 | **Eliminated** | 5513 | 111.42 | 2000-2049 @24.9 | 0.00 |
| **14** | PHANTOM | 33.9s | 0 | **Eliminated** | 5995 | 106.84 | 2400-2449 @40.8 | 0.00 |
| **10** | PHANTOM | 31.0s | 0 | **Eliminated** | 7157 | 87.99 | 700-749 @21.0 | 0.00 |
| **13** | PHANTOM | **none** | **4** | **Eliminated** | 8903 | 105.00 | 2850-2899 @29.7 | 0.00 |
| **07** | PHANTOM | **none** | 0 | **Eliminated** | 7523 | 95.76 | 750-799 @19.7 | 0.00 |

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

## Open

- **Respawns are the new residual.** Board-wide 23 -> 35 under step 3, with
  `01_Track` at RAPIER and PHANTOM the worst rows (ten recoveries each, no clean
  lap). The mechanism is the trade the change makes: a craft that brakes earlier
  wedges at low speed and is rescued, where before it crashed at speed and was
  charged for it. Whether the answer is in the driver or in
  `Race::lost_off_the_circuit`/the stall rescue is unread.
- **`07_Track` is still the weakest row on the board at every class**, and step
  3 made its VENOM row *worse* on contact ticks (1,445 -> 3,119) while charging
  it more (100.0 -> 122.1). Its worst cluster also moved, from 2,150-2,199 to
  **650-699**, which is a stretch the Outpost 7 thread never decomposed. Its
  named residual is unchanged: `corner_target`'s windowed curvature understates
  the true local apex by 1.66x/1.85x even at `curvature_span = 11`. A different
  *estimator* - the max over a short span rather than a chord over a long one -
  is the untried idea, and it is real work of its own that has to hold both
  field ground-truth tests green.
- **`13_Track` is the second-worst row at every class** and has never been
  decomposed. Its VENOM cluster is 1,600-1,649; at RAPIER it moves to 700-749
  and at PHANTOM to 2,850-2,899, so it is not one corner.
- **`05_Track` at FLASH is a new elimination** that VENOM does not show, cluster
  600-649. `05` is also the circuit with 134 unsupported racing-line samples
  (161-211, 811-893) recorded in `race_ground_truth.rs` - a **different thread**,
  noted here because 600-649 is not one of those runs and so is probably not it.
- **Lap time is the unmeasured cost of step 3.** 0.5-3.0 s a circuit, and
  nothing on the board or in the gate asserts on it. A row that laps 3 s slower
  and survives is better by this thread's standard and worse by a racer's; no
  measurement here separates "the AI is slower" from "the AI is correctly
  slower".
- Pure and HD are unmeasured at every class.

## Next Steps

1. **Attack `07_Track`'s 650-699 cluster**, which step 3 created or uncovered
   and which no evidence page covers. It is the largest single bucket on the
   board's worst row.
2. **Then the estimator**, not the span: `corner_target` is fed
   `Line::max_curvature(index, window, span)`, a chord long enough to dodge the
   seam bug and therefore long enough to average an apex down against its
   shoulders. Try the max over a short span. Hold both field ground-truth tests
   green - span 10 failed them for an unrelated craft-wedging reason, so a
   smaller number alone is not a guarantee.
3. **Then the respawn residual**, which is now a larger board-wide loss than it
   was.
4. Do **not** re-tune the wall response, `lateral_accel`, `grip_ground` or
   `grip_air` - Outpost 7 steps 1, 2 and 5 close all four.
