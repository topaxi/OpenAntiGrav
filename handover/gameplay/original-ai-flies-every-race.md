# The original AI flies every race with opponents: what is left

2026-10-13, lane `original-ai-default`. Maintainer's decision: until
`oag_ai::Driver` is solid, every race with opponents flies the recovered
original driver, with `oag-game --ai ours` as the developer switch back. Built
in `oag_game::ai_choice`; the rules, the chosen level mapping and the log lines
are in [gameplay/ai.md](../../docs/gameplay/ai.md) ("Which AI flies").

## Open

- **A lagging human makes the original leader fall on `13_Track`.** With a
  parked player the leader is rubber-banded to about 91 and misses the jump at
  idx 38 to 46 (hits the landing wall, falls, is reset in front of the same jump,
  129 times; about 208 respawns over seven opponents in 9,000 ticks). With a
  racing player (`--autopilot`) none in 12,000 ticks; `05_Track` has one, a
  reset volume on lap 4 at idx 290 while grounded at speed 96. This project's
  driver has at most one respawn a circuit. Unmeasured: whether the original
  game's leader falls there too (a player parked on the grid at `13_Track`, or a
  capture of the leader's speed at the jump), and whether the authored speed
  floor at those samples (`PointAi::speed_floor`, read by the field) is set
  where the leader needs it.
- **A forked circuit still learns the speed plans** when the original flies:
  the route plans gate which routes the coin offers
  (`Race::steer_branching`) and are learned on the ring plan's reference hull.
  Skipping them for the original field means deciding what offers a route
  there (every route, or a rule of the original's own), which changes how an
  original opponent goes round a fork, so it was not done. Everything else
  skips them: `Race::start_deferred`, `Race::ensure_speed_plans`.
- **Races with more than one human keep this project's driver**
  (`ai_choice::Reason::MultipleHumans`): the original field's slot, place and
  rubber-band handling with several humans is unread.
- **Classes without an original table keep opponents off** (RACE REMIX on Super
  Phantom): `ai.md`'s class coverage is unchanged by this lane.
- Ghosts recorded before this change raced opponents flown by our driver; they
  still play as poses.

## Next Steps

1. Print `point_ai` (`speed_floor`) for `13_Track` idx 30 to 46 and the leader's
   throttle there; then capture the original's leader on `13_Track` with a
   parked player (PPSSPP) to see whether it clears the jump. If it does, the
   difference is our jump physics or the floor mapping; if not, a rubber band
   cap near jumps is this project's to choose.
2. Decide what offers a route to the original field without the plan gate, then
   skip the plans on forked circuits too.
3. Add a round-the-lap ground truth over every Pulse circuit for the original
   field, with a racing player (`--autopilot`), next to
   `a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`.
4. Read the original field with several humans (slot, place, rubber band), then
   lift `Reason::MultipleHumans`.
