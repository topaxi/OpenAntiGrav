# Opponents fly their own team's handling, so the AI needs a speed plan per hull

Every opponent on every title flies its own team's `handlingstats.xml` for the race's class
(`oag_raceplay::load::family::opponent_handling`, `Setup::opponent_handling`); HD/Fury opponents
also fly the player's exact variant (`TeamVariants::opponents_match_player_variant`, static read,
confidence 55: `docs/ghidra/functions/ps3-hdfury-eu/opponent-handling.md`). Campaign opponents
(`original_ai`) take the same handling, the controller reads none. Facts:
[`hd-status.md`](../../docs/formats/hd-status.md), "Opponents follow the player's hull family".

The AI follows a reference plan (the player's hull's, else the most agile verifying hull's), and
every other hull on the grid re-learns it at race start (`oag_ai::SpeedPlan::adapt`), ring and
routes, threaded natively; a hull whose copy does not verify follows the reference at
`SpeedPlan::share_for`. Design, measurements, race-start cost and the shield table:
[`ai.md`, "A plan per hull"](../../docs/gameplay/ai.md#a-plan-per-hull-2026-10-13). 13
`ai_clean_lap_gate` rows, Black cells and the stall launch tick were regenerated for the slot-1
craft now being Triakis, one reason per row in each file.

## Open

- **Pretrained plans per track, shipped** (maintainer, 2026-10-13). A plan searched from
  scratch per hull is the better plan where it verifies (`02_Track` RAPIER 0 contact ticks
  against 290 adapted; 12 gate rows red against 13) but costs 12-24x a race start's CPU, so
  train ahead and ship, for every circuit including the original discs'; only custom tracks
  train on first use. Allowed by `docs/overview/legal.md` as long as the shipped form is
  per-sample values only (speeds, holds, tight flags, pace, braking, learned yaw) indexed by
  the circuit's spline samples, with **no positions, line geometry or sample spacing**. Design
  recorded in [`ai.md`](../../docs/gameplay/ai.md#a-plan-per-hull-2026-10-13); adapted plans
  stay the fallback for anything unshipped.
- **The race driver drifts off a verified line at full lock** (`02_Track` FLASH, Triakis,
  samples 1614-1636: 266 contact ticks on the adapted plan, 280 on its own, 87 with no line
  slack). The verifying driver is neutral; the race's Ace spends a little character and the
  hull has no turn left to recover. A plan cannot fix it; the `tight` mark (wall within a unit)
  does not see it. Marking full-lock samples tight was tried: 123 ticks, not 0.
- **Race-start cost of adapted plans**: 2.5-10x main's CPU (release, `01_Track` VENOM 4.5 s
  against 0.4-0.9 s), +0.6-1.0 s wall on a 16-core host. Phones and the web will feel it.
- **Read HD's live rival controller**: `AIControlStats.xml` is unread for HD and
  `docs/formats/hd-ai-netconfigs.md` says the `.nnt` networks are dormant.
- **Per-team handling on Pure, 2048 and Omega** was not exercised with a field; a team whose file
  fails to load keeps the player's handling and the load report says so.
- **Live RPCS3 check of the HD variant rule**, and a screenshot of an opponents' grid (the
  `--camera-pose` frames showed no ships).

## Next Steps

1. The pretrained-plans lane: an offline builder (an example over every title, circuit,
   class and team) writing per-sample values only, and a loader that prefers a shipped plan
   whose key and sample count match, a day or two.
2. The full-lock drift: verify a plan with the race's own driver spread (a few personality
   seeds) instead of the neutral driver alone, and measure on `02_Track` FLASH, an afternoon.
3. Read `AIControlStats.xml` on HD: a decompile-and-read lane, an afternoon.
