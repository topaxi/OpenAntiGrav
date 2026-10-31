---
status: ready-to-wire
titles: [2048, omega]
updated: 2026-10-31
summary: "cathedral's routes verify except one lap-1 rescue at the merge, where the line lies under the road; a narrow clean rule admits them"
size: S (60)
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

1. Wire: in `Race::route_speed_plan` (`crates/raceplay/src/speed_plan.rs`), stop failing a route plan for
   a rescue on lap 1 on the ring stretch before the lead-in (line index below `kept - 320`). That needs the
   verification to say where it rescued: a list of lap-1 rescue indices on `oag_ai::plan::Report`, filled by
   `SpeedPlan::verify_stuck`, read by nothing else. Flying-lap rescues already count in `verify_failures` and
   clear the lap time, so they stay fatal.
2. Predicted (census, seed 1, every forked circuit, four classes): exactly the 16 `cathedral` route plans change
   verdict, and no other route or ring plan changes a bit. Check with `route_verify_probe` across all circuits
   (3.5 min) before and after, then seeds 2-3.
3. Then the board (`route-plans-cost-a-whole-lap-each.md`, Next Steps 1-3) for what full fields do on the two
   routes with the whole-lap plans (the stretch learner's 390 traversals were with different plans).
