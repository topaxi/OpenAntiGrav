---
status: blocked-maintainer
titles: [pulse]
updated: 2026-11-07
summary: "Not reproduced: ours draws Pulse PSP hull, pads, then list B like the original; the symptom is the over-floors or HD order. Needs the maintainer spot"
---

# Pulse's blended scenery draws before craft and pads

The maintainer's report from play, 2026-11-07: in Pulse, trees render behind ships, speed pads and
weapon pads, even when the tree or bush is in front. Observed on Platinum Rush White
(`13_Track`).

## Open

- **The thread's first hypothesis is false for the default.** Our PSP order is the circuit (solid,
  sky, shine), pads, gantry, shadows, hulls, ribbons, the blended list, then the rest, whenever
  `Race::ribbons_first()` holds, which it does for Pulse and Pure at `trail_order = original`, the
  maintainer's own setting. The original's measured order is the same: hull `0x45000000`, speed and
  weapon pads `0x4a000000`, list B `0x4d000000` plus depth, the trees writing no depth
  (`docs/rendering/draw-order.md`, "Hull, pads and trees on Platinum Rush White").
- **The symptom is the other order.** With `ribbons_first()` false the blended list draws before the
  pads and the hulls and a later hull or pad overwrites a nearer tree. That is `trail_order =
  over-floors`, and the HD tube path (HD, Omega, 2048). Not reproduced on Pulse PSP at the default:
  three matched poses (start grid, speed pad 2's approach, spline row 2430) and one synthetic pose
  with a tree over the craft in ours.
- **What is needed:** the maintainer's spot (which pad or part of the lap, player or AI craft, which
  camera: their settings hold `camera_view = "far"`), or the build they were running. Platinum Rush
  Black was not reached: two Track Select presses (`--track-down 18` and `19`) came up on the same
  start frame, so which of White and Black each is stays unread.
- **Not read:** the weapon pads' submit (`0x0890cdc8`), the ship object's prims (`0x46000000`), PS2,
  Pure's keys, HD's, 2048's and Omega's queue keys.

## Next Steps

1. Ask the maintainer for the spot; take the pair there (`scripts/psp-pulse-pair.py --field`, a state
   from `psp-drive.py place` on the spline row nearest it).
2. If it reproduces at `original`, read the frame's order with the key bump of the doc; if it does not,
   close the thread and say so on the page.
3. For HD, Omega and 2048, read each title's own queue keys before moving a blended list: their path
   draws it before the craft today, which is the symptom's order.
