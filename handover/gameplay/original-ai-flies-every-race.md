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
- **The speed plan is still learned at every race start** (`Race::start`,
  `start.rs`), though nothing uses it when the original flies. Release cost of
  the field plan alone: Pulse `16_Track` 0.5 to 1.0 s of a 1.3 to 1.9 s headless
  start, HD 3.0 to 3.7 s of 5.2 to 5.9 s (route plans come on top, unlogged).
  Skipping needs a `Setup` field (`options.rs`, `options/headless.rs`, `load.rs`,
  `start.rs`), because the choice is made after `start`; the same plan drives
  `--autopilot` (`field.rs`), and a fallback to our driver would fly without it.
- **No second human in a race.** `original_ai_begin_tick` and `step_opponents`
  treat every slot from 1 as an opponent; a split-screen mode must filter
  `World::human_slots` out first.
- **Classes without an original table keep opponents off** (RACE REMIX on Super
  Phantom): `ai.md`'s class coverage is unchanged by this lane.
- Ghosts recorded before this change raced opponents flown by our driver; they
  still play as poses.

## Next Steps

1. Chase `13_Track` and `05_Track` on the original field: which slot, where it
   leaves the line (`Race::respawns_of`, the same trace the `13_Track` Novice
   chase used for ours). Either a mapping error in `original_ai.rs` (nearest
   sample, contact list) or a track the original's own driver also loses on.
2. Decide the speed-plan skip (the lead has the numbers) and, if yes, thread a
   `Setup` field through start, with `--autopilot` building its plan lazily.
3. Add a round-the-lap ground truth over every Pulse circuit for the original
   field, next to `a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`.
