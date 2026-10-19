# The original AI flies every race with opponents: what is left

2026-10-13, lane `original-ai-default`. Maintainer's decision: until
`oag_ai::Driver` is solid, every race with opponents flies the recovered
original driver, with `oag-game --ai ours` as the developer switch back. Built
in `oag_game::ai_choice`; the rules, the chosen level mapping and the log lines
are in [gameplay/ai.md](../../docs/gameplay/ai.md) ("Which AI flies").

## Open

- **A lagging human makes the leader fall on `13_Track`: the original's does too
  (measured 2026-10-13, PPSSPP, two boots; docs/gameplay/ai.md, "The original's
  own field falls ...").** Falls at the authored jump in every capture. Speed at
  spline row 19: falls 66 to 103.6, clears 101.8 and up; the leaders' held thrust
  is 47.1 and their speed 89.6 to 103.6 when they fall (16 of 39 entries at 47.x),
  this engine's 91 to 100. What clears is the speed pad just before the jump (pad
  6, 45 units ahead of it, lateral -1.9 to -9.8): at held thrust 47, 20 of 20
  on-pad entries cleared, 16 of 19 off-pad fell. The original's fall loops end
  (13 in a row at skill level 0, at most 2 at level 2 before the finish); this
  engine's slot 3 looped 129 times. **Hypothesis only:** our leader sits on the
  pad's off side. Not measured here: our signed lateral at the pad and whether
  our pad test boosts an AI craft there. `05_Track` (4 resets parked, 1 with the
  default-skill autopilot, a reset volume on lap 4) was not captured on the original.
- **A second reproduction of an original-driver jump fall: Pure, Chenghou
  Project** (2026-10-19, lane `campaign-ai-weapons`). Single race, VENOM, seed
  1, player parked, `use_original_ai(None, 1)`. Slot 2 climbs the ramp at line
  index ~1694-1719 on lap 3 at speed 169-191 (held thrust 44), goes airborne
  around tick 8031 and never lands: it falls (speed up to 324), strikes at tick
  8098 (shield 63 to 55) and is rescued at tick 8139 after 1.75 s airborne.
  Nothing hit it in the 3,100 ticks before. It surfaced once opponents started
  firing (`docs/gameplay/ai.md`, "An original opponent never fired, and now
  does"), which changed the field state; `pure_original_ai_ground_truth`'s
  Chenghou walk now allows 1 respawn a craft and names this cause.
- **The absorb half of `WeaponAi_DecideFireOrAbsorb`** (Pulse, recovered on
  `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`) was ported in commit
  `3b631f07a` and reverted in `4963adbfa` on 2026-10-19 (lane
  `campaign-ai-weapons`). It moved 11 lone-craft cells: `ai_clean_lap_gate`'s
  `track_01_phantom`, `track_02_flash`, `track_06_phantom`, `track_06_rapier`
  (CleanLap to NoCleanLap), `track_07_flash`, `track_07_phantom`,
  `track_14_rapier`, `track_14_phantom` (Eliminated to CleanLap), and
  `ai_dekonstruct_black_ground_truth::phantom_seed_1` (2 of 7 destroyed),
  `ai_dekonstruct_symptoms_ground_truth::de_konstruct_symptoms_phantom`, plus
  Omega's Amphiseum walk. **The cause is this project's Turbo policy**: a lone
  craft holding a forward weapon used to keep it forever; absorbing it frees
  the slot for the next pad's Turbo, which `spend_opponent_pickup` fires into
  walls. With absorb off all 51 cells pass. It did not reduce wrecks, and its
  decision cadence (every tick or four a second) is unconfirmed. Maintainer's
  decision: its own AI follow-up lane, with the Turbo policy fixed first.
- **A forked circuit still learns the speed plans** when the original flies:
  the route plans gate which routes the coin offers
  (`Race::steer_branching`) and are learned on the ring plan's reference hull.
  Skipping them for the original field means deciding what offers a route
  there (every route, or a rule of the original's own), which changes how an
  original opponent goes round a fork, so it was not done. Everything else
  skips them: `Race::start` no longer learns them, `Race::ensure_speed_plans` does.
- **Races with more than one human keep this project's driver**
  (`ai_choice::Reason::MultipleHumans`): the original field's slot, place and
  rubber-band handling with several humans is unread.
- **Classes without an original table keep opponents off** (RACE REMIX on Super
  Phantom): `ai.md`'s class coverage is unchanged by this lane.
- Ghosts recorded before this change raced opponents flown by our driver; they
  still play as poses.

## Next Steps

1. Compare this engine with the capture: log the signed lateral (the `lat`
   axis of `oag-trace track`) of the original-driver leader at spline row 3073
   (pad 6) and each zone entry's speed, parked player, VENOM, seed 1. If it
   sits at +1.4 to +2.6 (off the pad, where the original's crafts at +2.6 are
   never boosted), the difference is the lane (wander/spread) or the pad test
   for an AI craft, not the formation law or the jump; fix that, then re-count
   respawns against the original's (13 in a row at most, 2 at level 2). Only if
   it still loops, decide with the maintainer on a floor this project chooses
   (labelled chosen, not measured).
2. Decide what offers a route to the original field without the plan gate, then
   skip the plans on forked circuits too.
3. Add a round-the-lap ground truth over every Pulse circuit for the original
   field with a Skilled `--autopilot` player, next to
   `a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`;
   `13_Track` will fail it until 1 is settled.
4. Read the original field with several humans (slot, place, rubber band), then
   lift `Reason::MultipleHumans`.
