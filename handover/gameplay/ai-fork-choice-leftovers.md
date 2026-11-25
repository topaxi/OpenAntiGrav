---
status: re-first
titles: [pulse, 2048]
updated: 2026-11-25
summary: "2048's +0x150 override writer unread, 2048's re-commit, pad seeking on a route; undriven routes moved to their own threads"
---

# AI fork choice: what is left after the coin

2026-10-05 (ai-forks lane). Opponents flip the original's coin at every fork
and drive the route they draw, on every title; lap progress is read off the
route. Evidence: `docs/ghidra/functions/psp-pulse-usa/ai-branch-choice.md`,
`docs/gameplay/ai.md` "Branch choice at a fork".

## Open

- **Routes our craft cannot drive** are located (2026-10-31, `docs/gameplay/ai.md`, "Where each undriven
  route fails its verification") and have threads of their own: `outpost-7s-route-runs-into-the-ring-bridge.md`,
  `cathedrals-routes-fail-only-their-standing-start.md`. `square`'s is closed (2026-11-11: its ring plan and
  every route verify, ai.md, "A road under the line is a failure").
  `sol` is not done. **2026-11-25 (`floor-followers`):** it now reddens
  `sol_ground_truth::a_field_of_seven_laps_omega_sol_at_ace`, which is `#[ignore]`d behind
  `OAG_REVIEW_FINDINGS` as an open defect. HD's parallel floor arm changed the weapon history, the
  RNG draws shifted with it, and the fork coin put slot 1 on the route at path sample 5168. The craft
  cannot drive it: it slows near sample 1289, falls into a reset zone near 1350, is put back at
  sample 941 before the same fork, and repeats, 28 resets in 12,000 ticks. It holds a Cannon and is
  never hit. Deterministic; `OAG_PERTURB` does not reach this test.
- **2048's per-circuit override.** `Ai_ChooseBranch` (`0x8119ae90`) can force a
  side after the coin from the circuit record's `+0x150`/`+0x154`/`+0x158`
  and a per-craft table at `+0x2b2c4`. What fills them is not found.
- **2048's own re-commit** (the player-proximity switch with the `ai+0x160`
  timer) and its construction coin `ai+0xc8 = rand() % 2` (reader unknown).
- **Pad seeking on a route.** Weapon-pad indices are ring indices, so a craft
  on a route seeks no pad until it is back (ours).
- **Runtime confirmation.** A PPSSPP watchpoint on `ai+0x60` over a few races
  of `05_Track` would lift `Ai_ChooseBranch` past 90.
- **Load cost.** A route plan is built per route per race: `sol` spends about
  250,000 steps on three routes it then discards.

## Next Steps

1. Find the writer of 2048's circuit record `+0x150` (start from the
   `Definition.xml` track plugin parse).
