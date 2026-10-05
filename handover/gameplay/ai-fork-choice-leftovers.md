# AI fork choice: what is left after the coin

2026-10-05 (ai-forks lane). Opponents flip the original's coin at every fork
and drive the route they draw, on every title; lap progress is read off the
route. Evidence: `docs/ghidra/functions/psp-pulse-usa/ai-branch-choice.md`,
`docs/gameplay/ai.md` "Branch choice at a fork".

## Open

- **Routes our craft cannot drive, so the coin never sends anyone down them.**
  Pulse `07_Track` (the centre ramp: a lone Ace stalls at route sample 196,
  about 15 units/s, no wall touch); 2048 `cathedral` (both routes verify with
  failures) and `sol` (no route laps; the ring's own plan does not verify
  either). The original sends half its field down each. A physics or driving
  question on those roads, not a fork one: `cargo run --release -p oag-game
  --example fork_trace -- <source> <track> <pre-fork path> <from> <to>`.
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

1. Look at 07's centre ramp under `fork_trace`: is it hover, the plan, or the
   ramp's collision? If it is driveable, its route joins the coin by itself.
2. Find the writer of 2048's circuit record `+0x150` (start from the
   `Definition.xml` track plugin parse).
