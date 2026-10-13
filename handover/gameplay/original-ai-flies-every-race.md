# The original AI flies every race with opponents: what is left

2026-10-13, lane `original-ai-default`. Maintainer's decision: until
`oag_ai::Driver` is solid, every race with opponents flies the recovered
original driver, with `oag-game --ai ours` as the developer switch back. Built
in `oag_game::ai_choice`; the rules, the chosen level mapping and the log lines
are in [gameplay/ai.md](../../docs/gameplay/ai.md) ("Which AI flies").

## Open

- **The original field is not clean off `16_Track`.** A parked player, 9,000
  ticks, `Mode::SingleRace`, VENOM, seed 1, per-slot respawns of the seven
  opponents on Pulse PSP USA: `01`, `02`, `03`, `04`, `06`, `07`, `09`, `10`, `14`
  and `16_Track` all 0; `05_Track` 4 in total (slots 1, 3, 6 and 7); `13_Track`
  about 200 (slot 3 alone 129, six slots respawning by tick 9,000). This
  project's driver on the same runs: at most one respawn a track, 0 on `05` and
  `13`. Every other ground truth of the original driver flies `16_Track` (Pulse)
  or its title's default circuit. HD's default track: 0 for both. Foreign
  (Lion's Den): the original flies, respawns unmeasured.
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

1. Chase `13_Track` and `05_Track` on the original field: which slot, where it
   leaves the line (`Race::respawns_of`, the same trace the `13_Track` Novice
   chase used for ours). Either a mapping error in `original_ai.rs` (nearest
   sample, contact list) or a track the original's own driver also loses on.
2. Decide what offers a route to the original field without the plan gate, then
   skip the plans on forked circuits too.
3. Add a round-the-lap ground truth over every Pulse circuit for the original
   field, next to `a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`.
4. Read the original field with several humans (slot, place, rubber band), then
   lift `Reason::MultipleHumans`.
