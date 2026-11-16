---
status: ready-to-wire
titles: [2048, omega]
updated: 2026-10-31
summary: "cathedral's routes are offered since 2026-10-31 (route_clean forgives a lap-1 rescue on the ring stretch); the full-field cost board on them is open"
size: S (50)
difficulty: easy (60)
---

# `cathedral`'s routes fail only their standing start

2026-10-31, lane `route-coverage`. Evidence in `docs/gameplay/ai.md`, "Where each undriven route fails its
verification". Both routes on 2048 and Omega, at every class: 0 failures, 0 contacts, a completed flying lap,
and one rescue on lap 1 at line index 0-3, the merge. The line point there is under the road (+2.13 at ring
1436, +0.32 at ring 614), so the at-rest start falls into the reset volume below. `route_speed_plan` rejects
the plan for `verify_respawns == 1`.

## Open

- The race's own rescue pose at those ring samples (the racing line at spawn height) is inside the road too.
  Unmeasured whether a race ever rescues a craft there.

## Next Steps

1. ~~Wire `route_clean`~~ landed 2026-10-31 (lane `cathedral-route-start`), census in `docs/gameplay/ai.md`.
2. ~~Census~~ exactly the 16 cathedral plans change verdict, seed 1 (seeds 2-3 in the report). Their other hulls' adapted copies are now judged by the same rule: count their verdicts and `Race::plan_steps` for the board.
3. Then the board (`route-plans-cost-a-whole-lap-each.md`, Next Steps 1-3) for what full fields do on the two
   routes with the whole-lap plans (the stretch learner's 390 traversals were with different plans).
