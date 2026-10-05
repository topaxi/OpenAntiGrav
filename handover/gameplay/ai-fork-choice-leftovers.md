# AI fork choice: what is left after the coin

2026-10-05 (ai-forks lane). Opponents now flip the original's coin at every
fork and drive the route they draw, on every title; lap progress is read off
the route. Evidence: `docs/ghidra/functions/psp-pulse-usa/ai-branch-choice.md`,
`docs/gameplay/ai.md` "Branch choice at a fork".

## Open

- **A speed plan per route.** The plan is built for the ring alone, so a craft
  on a route drives the corner model. On the lone-craft board the three Pulse
  fork circuits' best laps grew 0.3-2.3 s (05: 34.8 -> 37.1 s). Building a plan
  per route costs one verification run per route at load.
- **2048's per-circuit override.** `Ai_ChooseBranch` (`0x8119ae90`) can force a
  side after the coin from the circuit record's `+0x150`/`+0x154`/`+0x158`
  and a per-craft table at `+0x2b2c4`. What fills them is not found.
- **2048's own re-commit** (the player-proximity switch with the `ai+0x160`
  timer) and its construction coin `ai+0xc8 = rand() % 2` (reader unknown).
- **Pad seeking on a route.** Weapon-pad indices are ring indices, so a craft
  on a route seeks no pad until it is back (ours).
- **Runtime confirmation.** A PPSSPP watchpoint on `ai+0x60` over a few races
  of `05_Track` would lift `Ai_ChooseBranch` past 90.
- **A Pulse picture.** `05_Track`'s fork is roofed by scenery, so a top-down
  `--camera-pose` frame shows no craft; altima's does
  (`cargo run -p oag-game --example fork_probe` prints the tick and pose).

## Next Steps

1. Plan per route: `Race::field_speed_plan` per `RouteLine`, used when
   `driver.branching.route` names it.
2. Find the writer of 2048's circuit record `+0x150` (start from the
   `Definition.xml` track plugin parse).
