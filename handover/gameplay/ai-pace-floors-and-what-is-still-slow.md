---
status: ready-to-wire
titles: [all]
updated: 2026-11-17
summary: "AI pace: sol's field laps at every level (2048, Omega); a relocated craft still loops at sol's earlier gap; 06 and others still slow"
size: M (40)
difficulty: medium (40)
---

# AI pace: the plan's floors are gone where a better plan wins; sol, 06 and the rest are still slow

2026-10-11, lane `ai-pace`, opened on the maintainer's report that the AI
"slows down a lot" on certain circuits. The measurement, the cause, the change
and the numbers are on [ai.md, "Floors in the speed plan"](../../docs/gameplay/ai.md#floors-in-the-speed-plan-2026-10-11).
Tooling: `crates/game/tests/ai_speed_plan_sweep.rs` (every title, level,
scenario, a per-section profile, the campaign's Speed Lap gold, and
`plan_floor_report`).

## Open

- ~~**2048 `sol`: a gap in the road loops the field.**~~ Closed 2026-11-17,
  lane `sol-gap`: a field of seven laps at every level on 2048 and Omega
  (`sol_ground_truth::a_field_of_seven_laps_*`, eight tests). The miss was not
  marginal lip speed on one handling: the plan's lap is flown on slot 7's hull
  and four of seven fly a slower one that does not clear the gap on the plan's
  own line, and air pitch does nothing (the flight is ballistic). Full nose up
  on the ground over the run-up lands every hull. The rule, the per-hull
  numbers and the before/after tables: `docs/gameplay/ai.md`, "A lip on a
  descent is taken nose up".
- **`sol`, what is left** (same section, "Still open on `sol`"):
  - A craft relocated to about 957 loops at the earlier gap (ring 1118-1306,
    not a diving takeoff): it reaches the lip at 178-179 u/s against a flying
    craft's 191, lands 21 units left and 20 under the line and meets a reset
    volume. 2048 Ace, slot 7: 12 times in a row. It is why that row's claim is
    decided at tick 11,787 of 12,000.
  - The gap at 3556 is still missed 5 to 10 times a run on 2048 (1 to 3 on
    Omega): a craft in traffic more than about 7 units off the line misses the
    pad. Dropping the traffic exception summed 83 respawns against 98 over the
    eight runs on an earlier form of the rule, not separated from scatter.
  - Reset-volume respawns at 4100-4199, 2 to 7 a run at Novice and Skilled.
    Not looked into.
  - The feisar hull's own ring plan was not re-checked for verifying with the
    rule; the field drives the reference's adapted or fallback copy.
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
- **Pulse `06_Track` VENOM**: its plan verifies since 2026-11-11 (79 of 85
  builds, every offset's majority; `docs/gameplay/ai.md`, "A road under the
  line is a failure"), so it no longer drives the corner model; its pace
  against gold is not re-measured. Before: crawling at 15-33 u/s over
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
  load. Its `sol` field is measured since 2026-11-17 (laps at every level).
- **Elite (the default)** costs 7.5 % of the Ace's lone lap on `07_Track` and
  `23_Track`, against 1-2 % elsewhere. A design question, not a defect.

## Next Steps

- **Decided 2026-10-11 (maintainer): campaign cells topping out at Elite (`tune_at_scale` clamps at 3.0, no cell reaches Ace) is intended.** Not a gap.
1. `sol`'s earlier gap after a relocation (ring 1118-1306). Trace it with
   `OAG_SWEEP=1 OAG_SOL_SLOT=7 OAG_SOL_FROM=930 OAG_SOL_TO=1330
   OAG_SOL_TICKS=12000 ... sol_ground_truth::gap_run_up_trace`. Two things to
   weigh: where the relocation puts the craft (bit 0 crumbs, the original's)
   against the run-up the two pads at 990 and 1110 need, and the line the
   craft takes off on (it lands 21 units left). Then widen the claim test's
   margin (2048 Ace has 213 ticks to spare). About 2 hours. Size S (25),
   difficulty medium (40).
2. `06_Track` VENOM: aim the run-up at the landing sample. About 2 hours.
3. `cathedral` PHANTOM: a plan-aware roll or the rescue gap, then re-run
   the 2048 lone and field PHANTOM slices. About 2 hours.
