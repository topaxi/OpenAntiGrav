---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-12
summary: "Our speed plan leaves the walls at 06_Track 1232-1242 unresolved; with Pulse's body damping RAPIER fails verification on 6 of 17 offsets (NOT_CLEAN)"
size: M (40)
difficulty: medium (40)
---

# Speed plan: 06_Track RAPIER after the gap at 1196-1200

`speed_plan_ground_truth::speed_plan_rapier_forward` carries `("06_Track", "RAPIER")` in
`NOT_CLEAN` since Pulse flies its body damping (`docs/gameplay/ai.md`, "06_Track RAPIER with
the body damping"; `docs/physics/body-damping.md`, "What it moved").

## Open

- Across the 17 scales of `perturb::BOUND_OFFSETS` and `perturb::HELD_OUT`, the RAPIER plan
  verifies clean on 15 undamped and 11 damped. Failing verifications meet walls at samples
  1232-1242 (unperturbed damped: 22 verify failures, unresolved `[(1234, Wall), (1232, Wall)]`).
- Those walls are `unresolved` in clean builds both ways, so the planner never finds an answer
  for them; the verification lap only misses them by luck of the landing after the gap at
  1196-1200. Our plan only (`crates/ai/src/plan.rs`); the AI flies the player's physics.

## Next Steps

1. Print the verification lap's speed, height and lateral offset over samples 1190-1245 for a
   failing and a clean damped build (scales `0` and `2e-3`): does it land long, short or wide
   before the wall? 1 hour.
2. Teach the planner an answer for a wall reached from a landing (the `Cause::LeftTheRoad`
   lookback before the flight, `docs/gameplay/ai.md`, "A road under the line is a failure", is
   the mechanism to reuse). 2-4 hours.
3. Acceptance: remove `("06_Track", "RAPIER")` from `NOT_CLEAN` in
   `crates/raceplay/tests/speed_plan_ground_truth.rs` and restore the module doc's "All 96
   verify"; `speed_plan_rapier_forward` green, and the 06 RAPIER build clean on at least 16 of
   the 17 scales; the other 95 plans, `ai_dekonstruct_symptoms_ground_truth` and the lone-craft
   gate unchanged. An `oag-ai` change regenerates the AI hashes in its own commit.
