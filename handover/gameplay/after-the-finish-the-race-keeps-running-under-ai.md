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
  camera cutting. **Maintainer decision 2026-10-02, "hold, then results"**: ours keeps the world running after a Single Race wreck and shows
  the same legend and panels on the line's timing - the legend at all, and its delay, **chosen, not measured, contradicted by the original**.
  **Camera, settled 2026-10-02 (lane pulse-spectator-cam):** the original stays in **mode 5 for good** (`Ship_SetState` case 4 clears `cam+0x26c`, the flag
  `Camera_PickRandomMode` tests), the subject changes at `k+240` when the wreck's state-6 timer crosses zero and the picture at the next 60-unit cut
  (`k+279`, `k+290` seen); ours now does the same from 240 ticks after the wreck call (210 after the race ended, pinned by a test), not the old single-sample 259. Open: what the original's wreck race
  waits for, a wreck drained rather than injected, the pause menu from that state, Zone and Eliminator.
- ~~**Modes `2` and `3` of the spectator camera**~~ **Ported 2026-10-02**: mode 2 is a rigid rear view (6 behind, 2.5 above), mode 3 a rigid front view
  (12 ahead, 3 above, looking back), 65 degrees, measured on PPSSPP (407 frames, rotation exact, eye to `6e-5`). Still unread: what sets `cam+0x274`, cases
  `0`, `1`, `4` and `8` of `Camera_UpdateSpectatorView`, and whether a barrel roll is inside the craft matrix the two views ride on.
- **A station camera still draws a wall the original sees past** (2026-10-02, **open, localised**): after the mask fix below, Talon's Junction's station 0 on a wreck at
  `(-104.2, -49.7, -180.3)` shows a paneled dark wall and lid across the frame in ours, where the original shows the stand and the `AG-SYS` banner beyond the two silver
  pillars (`data/scratch/pulse-spectator-cam/cmpw2.png`, original on the left; the original's own frames `wreckB/k*.png`). **Not** the section mask (the original reads
  section 11 there, ours too, and the 64-bit row is **identical**, `08000003f0003c02`, read live at `g_display+0x5bb8+11*8` against `TrackPvs::visible_from(11)`), **not** PVS on
  or off, **not** the near plane (`8.0` instead of `1.0` changes nothing), **not** the huge node-545 meshes. The wall is a handful of **opaque, back-face-culled 12-triangle
  boxes**: bisecting the opaque draw list found `#572` (node 524, texture 51, section bit 30, centre `(-137.5, -31.0, -142.1)`, radius 45, **on the line from the eye to the craft**)
  and its neighbours (nodes `518`, `524`, `527`, `530`, `535`, `537`, at `x = -137` and `x = -52`, `z` about `-146`); with all opaque draws off the picture is the original's stand.
  A cheap one first: nodes 518-537 come in pairs at `x = -137` and `x = -52`, drawn back-face culled as boxes that look like their insides: print whether either node of a pair has a mirrored (negative-determinant) transform, which would flip the winding when its vertices are baked into world space.
  Candidates: the second-tier section box test (`oag_render::pvs::section_view`, ours is never narrower than the picture), a state mesh of the start structure the original
  hides by animation, or a draw flag. `scripts/psp-wreck-capture.py --ge-dump-k` on the original at that pose, and a draw list of its frame, is the next measurement.
- **Zone and Eliminator wrecks** use the destroy camera too, which now masks by the wreck's section: **changed, not seen**.
- **Untested call site**: `scene/frame.rs`'s `race.visible_set(..)` (drop it and `visible_set` is never read; `station_camera_section` and `set_exact` are tested, the wiring is not).
- **The wall frames of the manual wreck run** (`pulse-end-photo/sheet_manual3.png`, solid teal and olive): found and fixed. A camera on an authored station
  (the destroy camera, the director) sits hundreds of units from the craft, where our camera section lookup failed and fell to "draw everything": the shot from
  inside a structure showed its inside. The original masks with the section of the craft it draws (`cam+0x1e8`, published through `FUN_08878644` in every mode).
- **The other endings**: an Eliminator target and a Zone run still stand still in ours (Zone: cosmetics only); the original was not measured there.
- **Audio after the flag** was not measured (emulator muted); ours spins the engine voice down under the panels while the craft flies on.
- **A player shot down after the finish**: the spectator camera is checked before the destroy camera in ours; unmeasured in the original.
- The camera node lists of the other 23 circuits were not read live; `16_Track` matches to 0.1 unit.
- HD/Fury (where the maintainer saw it), Pure and 2048 were not checked.

## Next Steps

1. A Single Race finished in a lower place, to read the throttle's source
   (`AI_ComputeOpponentThrust`'s rank terms are the first suspect).
2. ~~Read `FUN_08880c04`'s cases `2` and `3`, then replace the chase stand-in.~~ Done 2026-10-02 (see Open).
3. ~~Read `Race End Photo`'s enter and X handler~~ done for the line (measured, not read); what a wreck waits for is next, see Open.
4. Name `FUN_088418e0` (the per-entity update; the finish rule is in it) once its other duties are read.
5. Find why a station camera still draws a wall the original sees past (Open, above): take a GE dump of the original at that pose (`psp-wreck-capture.py --place -104.18,-49.69,-180.31 --ge-dump-k 30`) and list which of nodes `518`-`537` it submits.
