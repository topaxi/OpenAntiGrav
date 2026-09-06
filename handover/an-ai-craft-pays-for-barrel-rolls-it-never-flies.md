# An AI craft pays for barrel rolls it never flies, and on Outpost 7 that kills it

`crates/game/tests/race_ground_truth.rs::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
went red on `07_Track` on 2026-09-06. It is a real playability regression, it
bisects to one commit, and the mechanism is measured rather than inferred.

## The bisect, verbatim

Reproduction, from a checkout with `data/images/pulse-psp-usa.chd` present:

```sh
OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
  -E 'test(a_lone_craft_gets_round)'
```

At `40fb50c6` (the session's starting point) it **passes**, and `07_Track` is
the weakest row on the board but a passing one:

```
07_Track     clean lap  49.9s  laps 3   respawns 0   of 3216 lost at []
clean laps: [... "07_Track"]
no clean lap: []
test a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round ... ok
```

At `ffd702f3` ("feat(physics): wire the barrel roll's tap gesture to a real
input snapshot"), which is the first code commit after `40fb50c6`, it **fails**:

```
07_Track     clean lap   none  laps 2   respawns 0   of 3216 lost at []
no clean lap: ["07_Track"]
assertion `left == right` failed: only 11 of 12 circuits saw a clean lap, and all of them used to: ["07_Track"]
  left: 11
 right: 12
```

`lost_at` is empty and `respawns` is `0` in both, so the craft is neither
respawn-looping nor being rescued. It is **destroyed**: `ShipState::shield`
reaches `0.0`, `CraftState` goes to `Eliminated`, and the craft sits at
`(-1076.2, -0.6, 21.7)`, driver index 2901, from tick 4,940 to the end of the
run. A destroyed craft staying out of a single race is correct - see
[race-modes.md](../docs/gameplay/race-modes.md)'s "Respawn belongs to
Eliminator" - so the bug is the destruction, not the absence of a recovery.

## What the commit changed, measured over the whole disc

`ffd702f3` made `oag_physics::barrel_roll::advance_gesture` reachable from a
live race. The crate cannot tell a pilot from an `oag_ai::Driver` - both arrive
as `ShipControls` - and the axis leg has no human-only gate, which that commit
recorded, and `af033908` then measured against `oag-ai`'s own closed-loop
fixtures: zero arms on the ordinary oval, three across eight drivers on the
pathological one, "about once a minute".

**On the disc's own circuits the rate is an order of magnitude higher, and the
rolls never pay out.** One lone Ace craft, `Mode::SingleRace`, 18,000 ticks,
every forward circuit:

| circuit | net shield lost per lap | roll arms | shield spent on rolls | payouts | airborne ticks | outcome |
| --- | --- | --- | --- | --- | --- | --- |
| `03_Track` | 0.0, 0.0, 0.0 | 0 | 0.0 | 0 | 52 | finished |
| `02_Track` | 1.1, 0.0, 0.0 | 0 | 0.0 | 0 | 287 | finished |
| `05_Track` | 2.7, 10.2, 2.3 | 3 | 22.9 | 2 | 1,559 | finished |
| `06_Track` | 14.2, 6.1, 14.8 | 4 | 30.7 | 6 | 270 | finished |
| `14_Track` | 15.4, 6.9, 15.5 | 5 | 38.1 | 6 | 347 | finished, min shield **3.9** |
| `09_Track` | 13.7, 14.0, 15.1 | 6 | 46.3 | 10 | 446 | finished, min shield 9.5 |
| `10_Track` | 12.0, 12.7, 13.6 | 6 | 45.9 | 14 | 999 | finished, min shield 10.8 |
| `04_Track` | 5.6, 24.5, 15.0 | 6 | 45.8 | 6 | 191 | finished, destroyed after the flag |
| `01_Track` | 13.7, 24.0, 25.4 | 7 | 54.0 | 4 | 476 | finished, destroyed after the flag |
| `13_Track` | 26.7, 27.9, 26.6 | 7 | 53.4 | 3 | 69 | finished, destroyed after the flag |
| `16_Track` | 25.3, 23.5, 23.9 | 8 | **53.6** | **0** | **0** | finished, destroyed after the flag |
| `07_Track` | **56.9** | 5 | 38.0 | **0** | **0** | **destroyed on lap 2** |

**The per-lap column is net and not decomposable**: it includes the roll
charge and it is net of shield pads picked up along the way, which is why
`10_Track` can spend 45.9 on rolls while its three laps sum to 38.3 - it flies
enough (999 airborne ticks) to cross pads and regain some. Only the wall term
of `07_Track` was isolated, and only because the `40fb50c6` run arms **zero**
rolls: 34.1 then 35.0 shield per lap, purely from contact.

The shield pool is `95.0`. So an Ace opponent spends **30 to 54 of its 95
shield on barrel rolls in a three-lap race** on ten of the twelve circuits, and
on the two where it never leaves the ground (`07_Track` and `16_Track`) it
receives **nothing at all** for them: `barrel_roll::release` is called on the
airborne-to-grounded transition, that transition never happens, so
`ShipState::roll_payout_timer` is never armed and none of the three payout
consumers (the 1.5x lateral grip, the rebound override, the turbo add) ever
runs. `ShipState::roll_phase` also stays latched at `-1.00` from tick 1,200 to
the end of the run for the same reason - a render consequence exists and is
**not** chased here; the drawing side is somebody else's thread.

Four rows show `eliminated` at lap 4 with an elimination tick *after* their own
`finish_tick` (`16` 12,298 against 7,836; `04` 14,209 against 7,449; `01`
10,588 against 6,722; `13` 8,278 against 7,089). Those are post-flag and
harmless. `07_Track` is the only circuit with no `finish_tick` at all.

## Why `07_Track` and not the others

Two things stack there, and only one of them is new.

1. **Pre-existing, and the larger term.** `07_Track` loses 34.1 then 35.0
   shield per lap to wall contact at `40fb50c6`, with zero rolls armed - the
   only circuit measured with wall attrition on that scale, and the only one
   where the wall term could be read on its own, since every other circuit's
   per-lap figure has roll charge and pad pickups mixed into it. Traced tick
   by tick, the craft is down to 2.7 units/s at driver index ~2,145 and to
   23-30 units/s at ~2,400 on
   every lap, shedding 8-9 shield each time. It was already being destroyed at
   `40fb50c6`, at tick 8,100 on **lap 3**, having banked exactly one clean lap
   first. `docs/gameplay/ai.md` already records 07 as having 13 racing-line
   samples with nothing under them at indices 2,364-2,376, which is the second
   of those two clusters.
2. **New.** Three of the five roll arms happen inside lap 1, at driver indices
   2,147, 3,124 and 949, charging 22.8 shield. Lap-1 loss goes 34.1 -> 56.9.
   That brings the destruction forward to tick 4,940 on **lap 2**, before the
   craft has ever banked a clean lap, which is exactly the assertion that
   fired.

**The roll is a symptom of the crash, not its cause, and the trajectory proves
it.** The arm at index 2,147 lands inside the index-2,145 damage cluster, where
the craft is flailing at full lock at 2.7 units/s. And the whole trajectory is
**bit-identical** between `40fb50c6` and today at every sampled tick up to the
death - same position to 0.1 units, same speed to 0.01, same driver index - with
the shield reading differing by exactly a multiple of 7.6 (8% of 95). Shield
feeds no force in this engine, so the arm charge is the *only* difference, and
the same craft on the same line dies one lap sooner purely because it paid for
three rolls it never flew.

`14_Track` is the next one over the edge: it finishes with **3.9** shield left,
having spent 38.1 on rolls. `09_Track` and `10_Track` finish on 9.5 and 10.8.
Any of the three is one extra roll arm from `07_Track`'s outcome.

## Open

- **Whether the original's AI reaches the gesture by this path at all.**
  `barrel_roll::advance_gesture`'s own doc comment already names this as the
  open question, and the disc measurement above sharpens it from "about once a
  minute" to "a third to a half of every opponent's shield pool per race". If
  the original's opponents do not barrel roll off their own steering, something
  gates them and this project has not found it.
- **Whether the original refuses to arm on the ground.** The module doc says
  "nothing in the recovered chain" checks it and names `advance_gesture` as
  where the gate would go. `07_Track` and `16_Track` are the case that makes it
  matter: 0 airborne ticks, 0 payouts, 53.6 and 38.0 shield spent. Needs a
  trace, not a guess.
- **`07_Track`'s 34-35 shield per lap of wall attrition**, which predates all of
  this and is the larger term. Two clusters, at driver index ~2,145 and
  ~2,400, both reproducible every lap. The second sits on the documented
  unsupported-racing-line run at 2,364-2,376; the first has no recorded cause.
- **`ShipState::roll_phase` latched at `-1.00` for the rest of a race** on a
  craft that never goes airborne. Same untraced release rule; recorded here
  only because the measurement turned it up.

## Next Steps

1. Decide the airborne gate on evidence: find whether `Ship_*`'s roll arming
   path checks a grounded flag before charging shield. That is the one change
   that would close `07_Track` and `16_Track` without inventing an AI-only
   rule, **if** the original has it.
2. Failing that, trace whether the original's opponents drive their input block
   through the same tap history a pad does.
3. Separately and independently of the roll: chase `07_Track`'s index-2,145
   damage cluster. It is the pre-existing half and it is the half that makes
   the circuit fragile in the first place.

**Do not trim `07_Track` out of the test's known-good list.** It was passing at
`40fb50c6` on about 8 shield of margin, the assertion caught a real regression,
and the list is right.
