# The original AI flies every race with opponents: what is left

2026-10-13, lane `original-ai-default`. Maintainer's decision: until
`oag_ai::Driver` is solid, every race with opponents flies the recovered
original driver, with `oag-game --ai ours` as the developer switch back. Built
in `oag_game::ai_choice`; the rules, the chosen level mapping and the log lines
are in [gameplay/ai.md](../../docs/gameplay/ai.md) ("Which AI flies").

## Open

- **A lagging human makes the original leader fall on `13_Track`.** Cause
  found, not fixed: the formation throttle holds the leader at `held_thrust` 47
  (speed 91 to 100) with the player far behind, and the authored jump (line
  indices 19 to 49, the one the Novice chase in ai.md found) needs about 125 in
  this engine; the leader lands short at idx 46 and the airborne reset puts it
  back before the jump, 129 times from one slot. Parked player: about 208
  respawns over the field in 9,000 ticks; `--autopilot` Skilled 62, Novice 113,
  default skill 0 (12,000 ticks). Lane, corridor and steering are normal and
  the authored speed floor is absent (`unk_0x66` is authored on `06_Track`
  only). `05_Track`: 4 parked (slot 7 at tick 1111, airborne at 190 over a long
  drop), 1 with the default-skill autopilot (a reset volume on lap 4), none with
  Skilled or Novice. This project's driver clears the jump at full throttle.
  Unmeasured: whether the original game's leader falls there too.
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

1. Decide with the maintainer: (a) accept it as the original's own behaviour
   until a PPSSPP capture of a far-behind player on `13_Track` says otherwise
   (the leader's speed and whether it clears idx 46), or (b) give the original
   field a floor this project chooses (labelled chosen, not measured): never
   hold a craft below the speed the next jump needs, taken from this project's
   speed plan, which is then needed for the original too.
2. Decide what offers a route to the original field without the plan gate, then
   skip the plans on forked circuits too.
3. Add a round-the-lap ground truth over every Pulse circuit for the original
   field with a Skilled `--autopilot` player, next to
   `a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`;
   `13_Track` will fail it until 1 is settled.
4. Read the original field with several humans (slot, place, rubber band), then
   lift `Reason::MultipleHumans`.
