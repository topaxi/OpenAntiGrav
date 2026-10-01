---
categories: [rendering, frontend]
---

# After the finish the race keeps running under AI: the leftovers

2026-10-01. The maintainer's play report was right and is now measured and ported for
Pulse PSP: [after-the-finish.md](../../docs/gameplay/after-the-finish.md) has the timeline, what
ours does and what is chosen; [race-finish.md](../../docs/ghidra/functions/psp-pulse-usa/race-finish.md)
has the code. **The maintainer remembers it from HD/Fury** and presumed the earlier titles do the same:
Pulse PSP is now observed (the falsifier, a craft that stops, did not fire), HD/Fury is still unchecked.
In one line: from the frame after the player's last crossing the AI flies the
craft (the autopilot weight `craft+0x1d4` goes to `1.0`, the HUD hides, the mode state goes to 3),
the whole field keeps lapping, and 61 frames after the flag a spectator director takes the camera.

## Open

- **What produces the `56.7` post-finish throttle** (constant, against `100` for the pickup
  autopilot and `75-112` for opponents), and whether it is rank-dependent. Ours flies the finished
  craft at the AI's own pace, about 125-135 u/s against the original's 92-110. Measure a Single Race
  the player finishes **not first** (`scripts/psp-postrace.py` finishes it on the pickup; the player
  would have to be held back, e.g. by a `Craft_SetAutopilotBlend` write or a late start).
- **The `Race End Photo` state**: about a second after the flag the front end enters it, draws
  `PRESS SELECT BUTTON FOR PHOTO MODE / PRESS X TO CONTINUE` over the running race and waits for X
  before `EndRace Results`. Ours shows the panels at once. Its legend, its SELECT photo mode and the
  trigger on `F+61` (`FUN_08880788`) are unread.
- **Modes `2` and `3` of the spectator camera** (`above`, `front`): craft-relative, in the large
  `FUN_08880c04` switch, not read. The chase camera stands in (**chosen, not measured**), so about
  half the cuts differ from the original. Also unread: what sets `cam+0x274`.
- **The other endings**: a wreck, an Eliminator target and a Zone run still stand still in ours; the
  original was not measured there (a wrecked player's craft respawns after state 6 with the mode
  state already 3).
- **Audio after the flag** was not measured (emulator muted); ours spins the engine voice down under the panels while the craft flies on.
- **A player shot down after the finish**: the spectator camera is checked before the destroy camera in ours; unmeasured in the original.
- The camera node lists of the other 23 circuits were not read live; `16_Track` matches to 0.1 unit.
- HD/Fury (where the maintainer saw it), Pure and 2048 were not checked.

## Next Steps

1. A Single Race finished in a lower place, to read the throttle's source
   (`AI_ComputeOpponentThrust`'s rank terms are the first suspect).
2. Read `FUN_08880c04`'s cases `2` and `3`, then replace the chase stand-in.
3. Read `Race End Photo`'s enter and X handler, then give ours the same pause before the panels.
4. Name `FUN_088418e0` (the per-entity update; the finish rule is in it) once its other duties are read.
