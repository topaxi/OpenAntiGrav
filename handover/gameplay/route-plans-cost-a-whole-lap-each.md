---
status: measure
titles: [pulse, 2048, omega]
updated: 2026-10-31
summary: "Route plans learn a whole lap each; the stretch-only learner (0.45x) regressed de Konstruct Black PHANTOM, three leads"
size: M
difficulty: hard
---

# Route plans cost a whole lap each

2026-10-31, lane `route-plan-trim`. Learning a fork route's reference plan only on the stretch a craft drives
it (switch to merge) cut its steps to 0.45x over every forked circuit and newly verified 2048 `cathedral`'s
two routes, but was not adopted: de Konstruct Black (`ai_dekonstruct_black_ground_truth`, Pulse `05_Track`
forward, PHANTOM) destroyed 3/2/3 craft against main's 0/1/1, over seed 1's bound of 2. The whole account,
cost table and the three findings that hold either way are in `docs/gameplay/ai.md`, "A route's plan learned
on its driven stretch only". The attempt is commits `317b83bba` and `3a14dcf42` on branch `route-plan-trim`
(reverted in `dc00a054a`).

## Open

- **The stretch learner's 05 PHANTOM loss is not explained.** The reference craft forced down 05's route at
  PHANTOM went 594.0 to 607.3 ticks a traversal with 4 to 8 wall ticks. Pinning or freeing the lead-in made no
  difference (nothing was lowered there); compare the two plans' ceilings over the route's first jump.
- **The de Konstruct Black bounds are fragile.** Main itself destroys 5, 4, 5 and 5 forward craft at PHANTOM
  under `OAG_PERTURB` of +-1e-5 and +-2e-5 (seed 3 hits 3, over its bound). A bound frozen from the
  unperturbed run cannot tell a 25-against-21 change over 15 runs from noise; a perturbed-median bound would.
- **A rescue-placement loop on 2048 `altima`'s ring near sample 1,550** (PHANTOM, Elite, a craft forced down
  route 0): 34 rescues with the placement creeping back about 60 samples each time. Main reaches the spot (2-3
  rescues) and escapes; the stretch build fell into the loop unperturbed. The same family as `sol`'s.

## Next Steps

1. Measure any route-plan change with the board, against main built the same way, over the four classes and
   seeds 1-3, plus `OAG_PERTURB` of +-1e-5 for the spread (`route_plan_board.rs`'s doc has the command; about
   18 minutes a run, release, load 2-9):
   `OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 OAG_ROUTE_BOARD_OUT=board.tsv cargo nextest run --release -p oag-raceplay --run-ignored all --no-capture route_plan_board`
2. For the 05 lead: `OAG_ROUTE_BOARD_ONLY=pulse/05_Track OAG_ROUTE_BOARD_CLASSES=PHANTOM OAG_ROUTE_BOARD_WHERE=1`
   prints every wall tick and switch of the forced runs.
3. Gate any adoption on `ai_dekonstruct_black_ground_truth` at the perturbed offsets as well as at 0.
