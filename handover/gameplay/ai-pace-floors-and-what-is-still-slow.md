# AI pace: the plan's floors are gone where a better plan wins; sol, 06 and the rest are still slow

2026-10-11, lane `ai-pace`, opened on the maintainer's report that the AI
"slows down a lot" on certain circuits. The measurement, the cause, the change
and the numbers are on [ai.md, "Floors in the speed plan"](../../docs/gameplay/ai.md#floors-in-the-speed-plan-2026-10-11).
Tooling: `crates/game/tests/ai_speed_plan_sweep.rs` (every title, level,
scenario, a per-section profile, the campaign's Speed Lap gold, and
`plan_floor_report`).

## Open

- **2048 `sol`: no opponent ever laps.** Every craft hits a `ResetZone` about
  65 ticks from the grid. It is put back at its last on-track sample at
  standstill and hits the same volume again about 68 ticks later, creeping on
  about 5 samples a time: 176 respawns per craft in 12,000 ticks.
  - The first exit is the driver's: the autopilot player flies off the edge
    at about 20 s, where the road bends left while the craft is in the air,
    and no plan verifies there.
  - The loop is rescue placement in `crates/raceplay/src/respawn.rs`, the same
    family as `29_Track` reversed (one craft at sample 45, 52 respawns in one
    Ace VENOM field seed).
  - A human is not measured.
- **Floors still in use** (plans holding a ceiling at or under 20 u/s): Pulse
  `07_Track` VENOM (2141, 2144-2145, 2375), HD `04_Track` PHANTOM (2307), Pure
  Citta Nuova VENOM (1493). The trough plan does not win there.
- **Pulse `06_Track` RAPIER: tried 2026-10-12 (lane `ai-rapier`), not
  adopted.** Cause measured: the 0.968 Ace takes speed pad 4 at sample 729,
  which the plan lap misses, and reaches the corner at 779 at 171 against 155
  u/s. Hardening the trough plan at a 0.96 share fixes the row (1728 ticks, 0
  contact, everything else on Pulse, HD and Pure bit-identical), but flips 2048
  `cathedral` PHANTOM onto a plan where the Ace's barrel-roll payout sends it
  into a wall at sample 223 and it is destroyed. Numbers and the driver-side
  attempts are on ai.md, "Where only the trough plan verifies". The code is on
  local branch `ai-rapier-harden-attempt` (`adb9bab82`). Ways forward, each the
  maintainer's call:
  - Ship it anyway, taking the `cathedral` PHANTOM loss (and Omega's, which
    re-ships it; not measured).
  - Make a roll plan-aware first, for example no roll on a flight whose
    landing the plan lap took far slower, then re-run the 2048 matrix.
  - Fix the rescue gap below, then re-measure `cathedral`.
- **No rescue for a craft grinding a wall at about 1 unit/s.** On the hardened
  `cathedral` PHANTOM plan the lone Ace sat against the wall at sample 223 for
  about 28 s with 0 respawns until destroyed. Not looked into.
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

- **Decided 2026-10-11 (maintainer): campaign cells topping out at Elite (`tune_at_scale` clamps at 3.0, no cell reaches Ace) is intended.** Not a gap.
1. `sol`: find where the craft leaves the road around 20 s, the same
   landing-aim lever as `06_Track` VENOM. Then decide where a rescue should
   place a craft in front of a reset volume it cannot clear from standstill.
   About 2 hours.
2. `06_Track` VENOM: aim the run-up at the landing sample. About 2 hours.
3. `06_Track` RAPIER: pick one of the three ways forward above.
