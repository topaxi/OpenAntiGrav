# Pure's original opponent: a sibling driver, its throttle measured, not flown

2026-10-12, lane `orig-ai-pure`. Pure's opponent is the earlier design Pulse grew
out of, not Pulse's with branches. Evidence:
[psp-pure-usa/ai-controls.md](../../docs/ghidra/functions/psp-pure-usa/ai-controls.md)
(the map against Pulse, the parser, the live leg) and
[gameplay/ai.md](../../docs/gameplay/ai.md) ("Pure: a sibling driver").

Settled: the legacy block of `AIRaceStats.xml` is live (the `<NewStats>` switch
reads `0` live and has no writer on either pressing); `PosBalancing`, `TailZone`,
`ShipSpacingVariation` and `<ControllerPO>` never reach a craft; `Position` is the
nearest opponent's distance ahead of the player. The throttle law (per-slot cruise
draw, per-track speed tweak from `stats.xml`, field-wide zone multiplier, fatigue,
launch blend) reproduced on 1,688 of 1,689 PPSSPP samples over Venom and Vector.
Built: `oag_tables::pure_ai_stats`, `oag_ai::original::cruise`,
`crates/game/tests/pure_original_ai_ground_truth.rs`.
`RaceDefaults::original_opponent` stays `false` for Pure.

## Open

- **Nothing flies it**: this project has no Pure campaign (`oag_pure`'s `Campaign`
  is Pulse's dialect with no definition entry). The original opponent is the
  campaign's, so a Pure campaign comes first, or a decision that Pure's Single
  Race flies it.
- **The rest of the driver is read, not built**: the ringless planner (one
  current lane, a bang-bang lateral velocity, other-craft scoring with an
  `atan2` bearing, and `FUN_08918d30`'s track-object bumps whose object list is
  unread), the position-only rail, the pitch fade (`1030..1180`), the 2 s fork
  hold, and the every-frame update. The steering is Pulse's and already built.
- **Not measured live**: the `LeadZone` branch (the parked player never led), the
  steering, the planner, the `x 0.7` flag (`craft+0xd0 -> +0x134`, unread), the
  `35` floor, the speed floor, the rail's velocity (static: never written).
- **One sample** of 1,689 kept the controller's default `100` (Vector, 45.1 s,
  slot 5): read as the lookahead-failure exit, not chased.
- **The two `rand()` draws** (tweak, eight cruise speeds) use the seeded `Rng`:
  chosen, not measured.
- **Cross-title**: HD, 2048 and Omega ship `_kart` files whose text equals Pure's
  pair; no executable was found reading them and the reader was not run on them.

## Next Steps

1. Decide where Pure's original opponent flies (a Pure campaign, or Single Race).
2. Port the planner: start from `Ai_PlanLane` (`0x08919b6c`); read `FUN_08918d30`'s
   object list (`FUN_0891bb74` builds it from `"AI track data"`) first.
3. A PPSSPP run with the player teleported ahead (as `scripts/psp-ai-thrust.py
   --teleport` does on Pulse) to exercise `LeadZone` with
   `scripts/psp-pure-ai-thrust.py`.
