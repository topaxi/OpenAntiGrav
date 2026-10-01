---
categories: [rendering]
---

# The pre-race flyby is not played

2026-10-01, from play (the maintainer): before a race the original plays a flyby of
the circuit, and ours goes straight to the grid. It may sit in the same corner of the
engine as the post-race camera
([after-the-finish-the-race-keeps-running-under-ai.md](after-the-finish-the-race-keeps-running-under-ai.md)):
both are cameras the race selects outside the chase view, and the player's destroy
camera (2026-10-01, `oag_render::camera::destroy`) already picks the circuit's own
`Camera` nodes. Nothing in `docs/` records the flyby yet. Not to be confused with the
front end's background camera flythrough (`BackgroundController_Item.cpp`, see
[envsettings.md](../../docs/formats/envsettings.md)), which is a menu backdrop, and
not with `~FLYBY_DIST`, an ambience sample
([psp-audio.md](../../docs/formats/psp-audio.md)).

The play report is taken as true; the work is to measure it, not to build a flyby
from memory.

## Open

- **What drives the path**: the circuit's own `Camera` nodes (as the destroy camera
  does), an authored spline or animation in the circuit's `.vex`, or a fixed rule
  along the track spline. Whether it cuts between shots or flies one path.
- **When it plays and how long**: every race or only some modes, its duration, whether
  a button skips it, and how it hands over to the grid and the countdown.
- **What else runs during it**: the crafts on the grid, the HUD, music and the
  announcer.
- Which titles have one. The maintainer's post-race memory is from HD/Fury, so the
  flyby may be too: check Pulse PSP first, and HD/Fury on RPCS3 if Pulse has none.

## Next Steps

1. Capture on PPSSPP (Pulse PSP): from race load to the countdown, frames plus the
   camera's eye and look-at every frame (`scripts/psp-camera-pair.py` reads the
   camera node), on two circuits. Discriminating question: does the camera path
   coincide with the circuit's `Camera` node positions, or with something else.
2. Read the camera mode the race selects before the countdown (`Camera_SetMode`,
   `docs/ghidra/functions/psp-pulse-usa/camera.md`) and its update.
3. Implement it in its own camera module beside `camera::destroy`, with the skip
   input and the hand-over to the grid as measured.
