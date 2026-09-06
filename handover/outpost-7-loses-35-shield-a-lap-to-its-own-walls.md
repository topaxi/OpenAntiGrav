# Outpost 7 loses 34-35 shield a lap to its own walls, and nothing explains the first cluster

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

## Open

- **The cluster at driver index ~2,145 has no recorded cause.** The one at
  ~2,400 sits on the unsupported racing-line run at indices 2,364-2,376 that
  [ai.md](../docs/gameplay/ai.md) already records (13 samples with nothing
  under them); the first cluster is not on that list and nothing accounts for
  it.
- **Whether this is the disc's data or this repository's collision loading**
  is unsettled, the same open question `ai.md` carries for the deeper
  unsupported-line cases on `09`, `02`, `05`, `14`, `01` and `07`.
- **`13_Track`'s 2.7 shield** has never been decomposed.

## Next Steps

1. Dump the craft's position, the racing-line sample and the surface under
   both against driver index 2,100-2,200 on `07_Track`, and say whether the
   line is running into a wall, running above the floor, or neither.
2. Only then decide whether the fix is the line, the collision load or the
   wall response. Do **not** trim `07_Track` out of the test's known-good
   list: it has always passed and the assertion is right.
