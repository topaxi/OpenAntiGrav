---
status: measure
titles: [pulse]
updated: 2026-11-11
summary: "Platinum Rush trees draw in the original's order; the maintainer's overlap report needs their frame, then the fsr3 temporal path"
categories: [rendering]
---

# Platinum Rush trees: our order is the original's; the reported overlap is not a draw-order fault in a still

Measured and filed in `docs/rendering/draw-order.md`, "Trees overlapping trees on Platinum Rush White". Ours matches the
PPSSPP software frame in mesh order (7 frames, zero inversions) and in strip order, and pixels agree.

## Open

- **The maintainer's frame.** A tick (fly-by) or a position (race) and a screenshot of a tree drawn over a nearer one. Put the
  camera there with `--camera-pose` (race) or `--intro-ticks` (fly-by), run with `OAG_DEBUG_TINT_BLENDED=tree`, and compare the
  original at the same camera (`scripts/psp-flyby.py --frames N --dump-at K`, `psp-trace.py --camera`, `psp-ge-dump.py census`).
- **The temporal path.** Blended trees write no depth and a masked-empty velocity, so under `reconstruction = fsr3` and
  `motion_blur = high` a near tree's pixels carry the background's motion and a far tree's history can survive over it. A
  headless `--presented` frame has no history; a windowed run through the fly-by (the front-end flow, not `--race`, which opens
  on the grid) with the tint on and `reconstruction` flipped off then `motion_blur` off is the test. Untested.
- **Wide field of view.** The maintainer plays at `fov = 100`; a key is per mesh from the box centre, so a large strip whose
  centre lies behind or beside the eye draws first or last. Same rule as the original, more visible at 100 degrees. Untested
  with the tint.

## Next Steps

1. Ask for the frame; run it tinted at the maintainer's profile with `reconstruction` off, then fsr3.
2. If it only appears with fsr3: the fix is the reactive or transparency mask, or the trees' own velocity, not the order.
3. If it appears at `reconstruction = off`: a per-tree depth sort or alpha-tested depth write is a deliberate departure from the
   original (a pixel experiment changed 0.04 to 0.27 % of a frame); offer it as an `original`-default setting.
