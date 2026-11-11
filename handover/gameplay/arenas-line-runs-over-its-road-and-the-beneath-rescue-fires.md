---
status: ready-to-wire
titles: [2048, omega]
updated: 2026-11-11
summary: "2048 arena: the line runs 21 units over the road for 250 samples and the race's beneath rescue puts a lone opponent back there once in three laps"
size: S (25)
difficulty: easy (25)
---

# `arena`'s line runs over its road, and the beneath rescue fires on it

2026-11-11, lane `ace-walls`, found while giving the speed plan the same rule. Evidence in
`docs/gameplay/ai.md`, "A road under the line is a failure" ("No layout got worse" and "What
was tried and not kept").

On 2048's `arena` the authored line leaves the road at ring samples 4520-4550 (a few
unsupported samples, too few for a takeoff) and runs 21 units above a flat road until about
4800, where the road comes back up to it. Every craft drives that road, centred in the
corridor (12 units either side). The samples count as supported, so
`Race::beneath_the_line` (`crates/raceplay/src/respawn.rs`) reads the craft as on a road
under the circuit. A lone opponent, seed 1, 9,000 ticks: one `Beneath` respawn at sample 4679
(VENOM) and 4699 (RAPIER), in three laps.

The speed plan does not fail there: its beneath failure asks that the craft left the line's
corridor first (`oag_ai::plan::Failure::Beneath`). The race's rescue has no such test.

## Open

- Why only one rescue in three laps when every lap spends 250 samples there: the cooldown,
  the give-up gate or the dwell at speed. Not read.
- What is at the line's level over that road (the samples are supported, so some surface is
  within a probe reach of the line).
- Omega's `arena` was not raced. Its ring plan verifies as 2048's does.

## Next Steps

1. Count `Beneath` respawns per lap for a field of seven on 2048 and Omega `arena`, every
   class (`Race::last_respawn_cause_of`). 20 minutes.
2. Candidate (ours, chosen): give the race's rescue the plan's corridor test, or measure the
   depth against the surface under the line point and not the line point. Either moves 2048
   and Omega race hashes; the census is `beneath_census` (`ai_plan_probe.rs`) on Pulse and
   the same loop on 2048. About 2 hours.
