# AI pace: the plan's floors are fixed; sol, 06 and the rest are still slow

2026-10-11, lane `ai-pace`, opened on the maintainer's report that the AI
"slows down a lot" on certain circuits. The measurement, the cause, the change
and the numbers are on [ai.md, "Floors in the speed plan"](../../docs/gameplay/ai.md#floors-in-the-speed-plan-2026-10-11).
Tooling: `crates/game/tests/ai_speed_plan_sweep.rs` (every title, level,
scenario, a per-section profile, the campaign's Speed Lap gold, and
`plan_floor_report`).

## Open

- **2048 `sol`: no opponent ever laps.** Every craft hits a `ResetZone` from
  the grid and is put back at its last on-track sample at standstill. About 68
  ticks later it hits the same volume again, creeping on about 5 samples a
  time: 176 respawns per craft in 12,000 ticks. This is rescue placement in
  `crates/raceplay/src/respawn.rs`, the same family as `29_Track` reversed
  (one craft at sample 45, 52 respawns in one Ace VENOM field seed). Not the
  driver. Check whether the player hits it too before fixing it.
- **Pulse `06_Track` RAPIER is a one-line switch, and it is the maintainer's
  call.** Only the trough search verifies it. On that plan the lone Ace laps
  29.16 s against 38.61 s on the corner model (gold 24.0), but the
  `ai_clean_lap_gate` row goes 173 to 304 contact ticks, at samples 750-850.
  The cause is measured: the Ace's commitment margin (0.938, so 3 % under the
  plan). The plan's own driver, and an Ace at the full plan, touch nothing
  there.
- **Pulse `06_Track` VENOM**: corner model, crawling at 15-33 u/s over
  1100-1200. The gap at 1196 clears at about 60 u/s. From 90 u/s the craft
  lands drifting right into the wall at 1238, and too slow it stalls in the gap
  (`ai_plan_probe::plan_probe` at fixed speeds). A landing-aim lever: steer the
  takeoff at the landing point.
- **HD `06_Track`**: plan-driven, 19-47 % over gold, with real low ceilings
  over 1250-1450, 300 and 700. Unexplained.
- **2048 `bridge`**: no flying lap in 12,000 ticks. Check lap counting first.
- **Pulse `07_Track` VENOM**: a pad at the hairpin exit (around 2180) throws
  the craft from 40 to 90 u/s into the wall at 2185. It keeps the old plan
  with its 16 u/s floors.
- **Omega**: not measured. A lone slice did not load Omega's layouts inside
  500 s. It re-ships 2048's circuits, so `sol` is the first thing to check
  there.
- **Elite (the default)** costs 7.5 % of the Ace's lone lap on `07_Track` and
  `23_Track`, against 1-2 % elsewhere. A design question, not a defect.

## Next Steps

1. `sol`: log the `ResetContact` (collider, swept flag) at tick 65, and check
   whether the player's autopilot loops too. Then decide where a rescue should
   place a craft in front of a reset volume it cannot clear from standstill.
   About 2 hours.
2. Put the `06_Track` RAPIER trade to the maintainer. If they accept it,
   `SpeedPlan::build_within` takes the trough plan where only it verifies, and
   the gate row's baseline moves in a separate commit.
3. `06_Track` VENOM: aim the run-up at the landing sample. About 2 hours.
