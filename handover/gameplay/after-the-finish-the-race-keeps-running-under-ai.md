---
categories: [rendering, frontend]
---

# After the finish the race keeps running under AI, and ours stops

2026-10-01, from play (the maintainer): in the original, once a race is over the race
keeps playing in the background, the player's craft driven by AI. How the camera behaves there is not known. **The maintainer remembers it from HD/Fury**
and presumes the earlier titles do the same, so HD is the observed title and Pulse the
presumed one: a Pulse capture showing the craft stopping falsifies the presumption for
Pulse, not the report. Nothing in `docs/` records this yet: the end-of-race pages
([endrace-screens.md](../../docs/ui/endrace-screens.md)) cover the panels drawn over
the race, not what the race underneath them does.

Play reports on this project have been a reliable oracle, so the claim is taken as true
and the work is to turn it into measurements, not to tune toward a memory.

## Open

- **When the hand-over to AI happens**: at the player's own finish line crossing, at
  the last craft's, or when the results panel opens.
- **Who drives the player's craft**: the ordinary opponent AI (`oag-ai`'s driver, which
  per the maintainer's standing rule obeys player physics) or a separate post-race
  routine. Whether its speed or weapons differ from a racing opponent's.
- **Whether the opponents still racing finish their laps**, and whether the field keeps
  lapping after everyone has finished.
- **The camera**: chase, the circuit's own cameras (the destroy camera just ported
  picks the circuit's Camera node nearest the wreck; a post-race view may use the same
  nodes), a replay-style cut sequence, or something else. Whether it cuts, and on what.
- **Whether the HUD hides**, and what audio does (engine loop, music).
- Which titles do it: seen in HD/Fury, presumed in Pulse; Pure and 2048 unknown.

## Next Steps

1. Capture on PPSSPP (Pulse PSP; if Pulse shows no post-race AI, capture HD/Fury on
   RPCS3 before closing anything, since HD is where it was seen): finish a Single Race and a Time Trial, then record
   frames and the player craft's input/control state for at least 30 s past the line
   while the results panels are up. Discriminating question: does the player's craft
   keep moving with no input, and is its control state the AI's? Falsified if the craft
   stops or coasts to rest.
2. Read the hand-over in the executable: the race-state change at the finish
   (`Ship_SetState` and its callers) and the camera mode it selects
   (`Camera_SetMode`, see `docs/ghidra/functions/psp-pulse-usa/camera.md`).
3. Implement: hand the player's slot to the AI driver at the measured moment, keep
   the simulation ticking under the end-of-race panels, and port the camera the
   original selects. Determinism and the regression gate apply as for any sim change.
