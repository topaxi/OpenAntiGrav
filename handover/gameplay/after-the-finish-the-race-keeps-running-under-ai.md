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
- ~~**The `Race End Photo` state**~~ **Ported 2026-10-02 (pulse-end-photo), line finishes only.** The state is entered at `F+61` and the legend
  (`InGame_Definition.xml`'s own two `Stats` texts) fades in over 42 frames; ours holds the clean view, draws the disc's lines and leaves on
  X/Start/click; SELECT does nothing. **Still open on it**: photo mode itself (`forward="select"` goes to `InGame Photo`), why the fade is
  0.7 s, and the real `Pulse_14.fnt` atlas (ours is the menu face scaled, narrowed by a chosen 0.90 that matches width, not height).
- **Noticed, not investigated:** in one windowed run the HUD was French while the `Race End Photo` legend and `Results` were English, and in the next the legend was French. The EndRace string table and the HUD's language did not always agree within a session.
- **A player wreck never reaches `Race End Photo`** (measured 2026-10-02, `data/scratch/pulse-end-photo/wreck/log.json`, `wreck-watch/log.json`): mode
  state goes to 3 and the HUD hides at `k+31`, then the front end stays on `InGame` for 17,800+ frames with the field racing on and the spectator
  camera cutting (the camera object takes another craft at `k+290`, mode 5, `wreck2/log.json`). **Maintainer decision 2026-10-02, "hold, then
  results"**: ours keeps the world running after a Single Race wreck, hands the camera to the director 259 ticks after the race ended, and shows
  the same legend and panels on the line's timing - the legend at all, and its delay, **chosen, not measured, contradicted by the original**.
  Open: what the original's wreck race waits for, a wreck drained rather than injected, the pause menu from that state, Zone and Eliminator.
- **Modes `2` and `3` of the spectator camera** (`above`, `front`): craft-relative, in the large
  `FUN_08880c04` switch, not read. The chase camera stands in (**chosen, not measured**), so about
  half the cuts differ from the original. Also unread: what sets `cam+0x274`.
- **The other endings**: an Eliminator target and a Zone run still stand still in ours (Zone: cosmetics only); the original was not measured there.
- **Audio after the flag** was not measured (emulator muted); ours spins the engine voice down under the panels while the craft flies on.
- **A player shot down after the finish**: the spectator camera is checked before the destroy camera in ours; unmeasured in the original.
- The camera node lists of the other 23 circuits were not read live; `16_Track` matches to 0.1 unit.
- HD/Fury (where the maintainer saw it), Pure and 2048 were not checked.

## Next Steps

1. A Single Race finished in a lower place, to read the throttle's source
   (`AI_ComputeOpponentThrust`'s rank terms are the first suspect).
2. Read `FUN_08880c04`'s cases `2` and `3`, then replace the chase stand-in.
3. ~~Read `Race End Photo`'s enter and X handler~~ done for the line (measured, not read); what a wreck waits for is next, see Open.
4. Name `FUN_088418e0` (the per-entity update; the finish rule is in it) once its other duties are read.
