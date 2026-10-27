---
status: ready-to-wire
titles: [all]
updated: 2026-10-21
summary: "Script nine gestures with mirrors on Pulse PSP's Talon's Junction and add the first-divergent-tick grid to oag-trace"
difficulty: medium (30)
---

# A player-input scenario matrix for physics, proposed and deferred

Proposed on 2026-10-21 by an outside review of the drive, and deferred by the lead while that
session worked on render fidelity only. Nothing is built yet.

## Open

- **The proposal.** Commit a matrix of player-input scenarios: per title, a handful of
  circuits, each class, and about nine gestures (held throttle, full stick each way, airbrake
  each side, a sideshift each way, pitch up and down, a barrel roll each way). Every signed
  gesture gets its mirrored twin. Each scenario is a scripted pad run on the original (never
  under its Autopilot, see `docs/physics/README.md`) and the same script in ours (`oag-trace
  run`). The output is a grid of the first tick where the two diverge past a tolerance, plus
  an aligned diff of events (wall hits, landings, pad hits). That grid becomes the physics
  queue, ranked by what a player feels.
- **Refusals it implies.** `oag-trace` should refuse an Autopilot-flown trace and a script
  whose analog axes are all zero unless told otherwise. On 2026-10-21 the race path of
  `oag-game --input-script` was found to drop analog axes, so "same input on both sides"
  comparisons had silently run ours with the stick centred (fixed that day).
- **Mirror tests.** For every signed input, a test asserts the world-space direction of the
  response under a held input and under its mirror. On 2026-10-21 the drawn lean and barrel
  roll shipped mirrored because the tests fitted magnitudes and never directions.

## Next Steps

1. Pulse PSP first: script the nine gestures with their mirrors on Talon's Junction, capture
   them under PPSSPP's virtual pad, and run them through `oag-trace run`.
2. Add the first-divergent-tick grid and the two refusals to `oag-trace`.
3. Extend to HD on RPCS3, then the other titles as their capture paths allow.
