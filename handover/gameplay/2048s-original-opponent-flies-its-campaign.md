# 2048's original opponent flies its campaign: what it does not do yet

2026-10-12, lane `orig-ai-2048`. 2048's opponent is HD's driver plus four laws
keyed on the game mode's lineage; the thrust law reproduced on 528 of 528
Vita3K samples at two skills, and every 2048 campaign event flies it.
Evidence: [vita-2048-eu-v104/ai-controls.md](../../docs/ghidra/functions/vita-2048-eu-v104/ai-controls.md)
(map, measurements, Omega) and [gameplay/ai.md](../../docs/gameplay/ai.md)
("2048: HD's driver, four laws of its own, flown"). Tool: `scripts/vita3k-mem.py`.

Settled: `oag_ai::original::Laws` gains `every_tick`, `event_ring_rate`,
`coin_ordered_search` and `preferred_follows_choice`;
`oag_2048::race::ORIGINAL_DRIVER` carries 2048's, its `event_ai`
(`oag_title::original_driver::EventAi`: skill, two AI skills, level off `SP.xml`
over the constructor's defaults) included; `Campaign2048Progress::ai`
takes them to `Race::use_original_ai` and `Race::set_original_event_ai_skills`.

## Open

- **Weapon-pad tracking and weapon avoidance are not ported** (`Ai_TrackWeaponPads`
  `0x8119c0e0`, `0x8119b7b6`, the `driver+0x160` decay that also scales the steering
  gain and sidesteps the lane point). Both are on by default in every 2048 event
  (the two skills read `1.0`); a port needs the pad list and the threats in
  `oag_ai::original`'s input.
- **Sol** (`"2050 - Event 2"`): every craft respawns in a loop on lap 1, under this
  project's own driver as much as the original's (213 to 218 respawns each in
  240 s). A rescue or track-load fault, not the driver; see
  `ai-pace-floors-and-what-is-still-slow.md`.
- **Arena** (`"2049 - Event 6"`): the original field finishes with one to three
  respawns a craft. Not compared fairly with this project's driver (that run had
  weapons destroying craft).
- **Elimination events (26)** keep this project's driver: 2048's own
  Eliminator-family thrust law (`0x81199800`) is unread. `GameMode_IsEliminatorFamily`'s
  flags `1`/`4` may cover more event kinds.
- **The rate is chosen**: 2048 ticks at 30 fps, this engine at 60 Hz; with
  `every_tick` the planner's per-call terms run twice as often a second.
- **The race-time lag** the Vita3K samples needed (`t - 1/30`) is unexplained: a
  lagged clock or the stop's position in the frame. A breakpoint at the law's
  return would settle it.
- **Super Phantom events (20)** keep this project's driver: no fifth
  `oag_tables::handling::SpeedClass`. The class's 2048 tables exist and parse.
- **Not ported, named**: the section-change re-seed in the planner, the chase mode
  under `driver+0xcc` (its writer not found), 2048's own Eliminator-family thrust
  law `0x81199800`, the Eliminator spin, the Duel blocks, split screen.
- **Not measured**: the steering, the planner's choice, the rail, the finished and
  Eliminator branches on 2048; the speed floor (the parked player held every
  opponent above it).
- **Pulse's search bound**: `Planner::local_best`'s up pass stops before candidate
  `8`; 2048's runs through it. Whether Pulse's does was not re-read.
- **Omega**: its executable inlines 2048's `Ai_Construct` and event tuning; checked,
  applies, not wired.

## Next Steps

- **Decided 2026-10-12 (maintainer): keep `original_opponent` on for 2048**, despite Arena's
  respawns and Sol's lap-1 respawn loop. Those are rescue-placement gaps to fix, not a reason
  to fly this project's driver in the campaign.

1. Read `0x81199800` (2048's Eliminator-family thrust) and fly Elimination events.
2. Port the pad tracking: read `Ai_TrackWeaponPads` (`0x8119c0e0`) against this
   project's pad list, then the avoidance (`0x8119b7b6`) and its threat loop.
3. Sample a 2048 event whose two AI skills are `0` (`"2048 - Event 7"`, Square,
   Rapier) with `scripts/vita3k-mem.py ai-sample` to see the ring rate fall to `0.001`.
4. Let a capture run past an opponent's finish to exercise the finished branches on
   2048.
