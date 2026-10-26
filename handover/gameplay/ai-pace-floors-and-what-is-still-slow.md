---
status: ready-to-wire
titles: [all]
updated: 2026-10-12
summary: "AI pace: 2048 sol never laps (ResetZone loop), 06 and others still slow; find where the craft leaves the road"
---

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
- **2048 `cathedral` PHANTOM (forward): destroyed on the hardened trough
  plan, accepted.** Pulse `06_Track` RAPIER is closed: the hardened trough plan
  was adopted on the maintainer's decision of 2026-10-12 (lone Ace 38.35 to
  28.80 s, 65 to 0 contact; Pulse, Pure and HD otherwise byte-identical). The
  same change flips `cathedral` PHANTOM onto a plan where the Ace's barrel-roll
  payout sends it into the wall at sample 223: lone Ace 4 laps at 51.80 s to 0
  laps, destroyed; field of seven destroyed 3 to 5. Omega's `cathedral` is
  unaffected (its whole-window plan verifies). Numbers on ai.md, "Where only
  the trough plan verifies". Ways to win it back:
  - Make a roll plan-aware, for example no roll on a flight whose landing the
    plan lap took far slower, then re-run the 2048 matrix.
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
- **Omega**: loads now (2026-10-12: `OAG_SWEEP_TITLE=omega
  OAG_SWEEP_ONLY=cathedral`, about 15 s a slice), but its pace was not
  measured against gold. A full forward lone slice runs 10-17 minutes under
  load. It re-ships 2048's circuits, so `sol` is the first thing to check
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
3. `cathedral` PHANTOM: a plan-aware roll or the rescue gap, then re-run
   the 2048 lone and field PHANTOM slices. About 2 hours.
