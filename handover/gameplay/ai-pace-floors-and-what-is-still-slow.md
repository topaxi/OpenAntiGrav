---
status: ready-to-wire
titles: [all]
updated: 2026-11-03
summary: "AI pace: 2048 sol has its floor now, a road gap at 3553-3620 still loops the field (air-pitch lever unproven, queued); 06 and others still slow"
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

- **2048 `sol`: the floor is in (2026-11-03, lane `2048-sol-ai-laps`); a gap
  in the road still loops 2048's field.** The 153-respawn loop was a missing
  floor, not the driver: the `.vex`'s lone `Reset` node kept `track_col.col`
  out (`docs/formats/2048-collision.md`, "Which circuits read the container";
  `crates/raceplay/tests/sol_ground_truth.rs`). Left: ring samples 3553-3620
  are a gap after a speed pad (~3530), shared by every route. It clears from
  about 170 u/s at the lip; the verifier reaches 173 (cruise 129.4 u/s), the
  field 166-169 (cruise 124-125, plain seed-0 driver 127.6). Bit 0 covers line
  3384-3647, so the original's relocation puts the craft back at ~3274 and it
  meets the lip at the same speed: the rescue is faithful, the miss is the
  driver's. Numbers on `docs/gameplay/ai.md`, "Floors in the speed plan".
  2048 after the floor, field of seven, VENOM, 12,000 ticks, respawns per craft:
  Novice 6/35/12/34/34/3/33, Skilled 19/21/4/35/8/8/22, Elite 21/2/2/21/3/13/6,
  Ace 36/9/9/13/1/22/5 (craft with a flying lap: 2, 3, 4, 3 of 7). Omega's
  `sol` (HD hulls): Elite 0/1/1/3/5/1/4 and Ace 3/2/1/2/1/3/3, every craft
  laps; Novice and Skilled 1-35.
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
  load. It re-ships 2048's circuits, so `sol` is the first thing to check
  there.
- **Elite (the default)** costs 7.5 % of the Ace's lone lap on `07_Track` and
  `23_Track`, against 1-2 % elsewhere. A design question, not a defect.

## Next Steps

- **Decided 2026-10-11 (maintainer): campaign cells topping out at Elite (`tune_at_scale` clamps at 3.0, no cell reaches Ace) is intended.** Not a gap.
1. `sol` gap (queued follow-up, opus). A one-off test held a constant air
   pitch on every airborne tick in place of the trim (`steer_y`, positive
   nose up): Ace field respawns summed 95 with the trim, 79 at 0, 45 at +0.5,
   34 at +1, 33 at -1, and both signs cleared and fell from 167-169 u/s lips.
   No sign effect: the gap is marginal. First find which held air pitch,
   applied only over 3553-3620, changes the outcome, and its sign; and why a
   race craft cruises 2-5 u/s under the verifier on the same handling
   (personality wander costs ~2; four of 2048's seven opponents fly the
   player's `feisar2048\3` because the native team ids lack their variant,
   `handover/tooling/2048s-track-vex-parses-but-two-binary-formats-changed.md`).
   Then a narrow rule (unsupported line run only) with the lone board,
   `ai_clean_lap_gate` and the determinism hashes unchanged, and the claim
   test: a field of seven at every level laps on 2048 `sol` within 12,000
   ticks. About 3 hours. Size M (40), difficulty medium (40).
2. `06_Track` VENOM: aim the run-up at the landing sample. About 2 hours.
3. `cathedral` PHANTOM: a plan-aware roll or the rescue gap, then re-run
   the 2048 lone and field PHANTOM slices. About 2 hours.
