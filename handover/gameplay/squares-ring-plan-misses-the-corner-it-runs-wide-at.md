---
status: ready-to-wire
titles: [2048]
updated: 2026-10-31
summary: "2048 square: the plan runs wide off the edge at 577-641 with no failure counted, then grinds walls 950 samples later"
size: M (40)
difficulty: medium (35)
---

# `square`'s ring plan misses the corner it runs wide at

2026-10-31, lane `route-coverage`. Evidence in `docs/gameplay/ai.md`, "Where each undriven route fails its
verification". At FLASH and above, the craft runs wide through ring samples 577-641 at 153-156 u/s with no plan
limit. It goes 9 units past the corridor's right bound, steering near full, leaves the edge and drops about 20
units onto a lower level. Nothing counts that as a failure (no wall, no rescue, within the off-line distance).
The craft then grinds walls at 1592-1671, 20 units under its line, and the learner lowers the ceilings there
(1,092 to 1,660 times) to no effect. The routes fail at the same place: it is on the ring.

## Open

- VENOM's separate scrape at 3392-3405 (1-2 wall ticks a lap at 148-162 u/s, unresolved after 14 lowerings) is
  unexplained. Dump it with `OAG_ROUTE_PROBE_RING=1 OAG_ROUTE_PROBE_DUMP=<dir>` and read offset against the
  corridor there.

## Next Steps

1. Candidate (ours, chosen): count "left the line's level" as a failure in `Course::tick`
   (`crates/ai/src/plan.rs`), for example grounded and more than a few units under the line point. Then the
   learner lowers the ceilings over the 45 ticks before the drop (577-641), not 950 samples later. The rule
   binds every plan, so the census decides it: `route_verify_probe` over all circuits with
   `OAG_ROUTE_PROBE_RING` and without, before and after, and the speed-plan pin
   (`speed_plan_ground_truth.rs`).
2. Measure: `square`'s ring plan verified at FLASH to PHANTOM is the target. The board (Next Steps of
   `route-plans-cost-a-whole-lap-each.md`) then judges its routes.
