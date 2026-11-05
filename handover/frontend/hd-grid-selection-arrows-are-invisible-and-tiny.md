---
status: ready-to-wire
titles: [hd, omega]
updated: 2026-11-05
summary: "Grid Selection swipes on touch; its Flyer arrows are never drawn (tilted_arrows art not loaded) and their rects are too small to tap"
size: S (60)
difficulty: easy (60)
---

# HD and Omega Grid Selection: the paging arrows are invisible and tiny

The vertical swipe landed (`docs/ui/campaign-screens.md`, "HD and Omega `Grid Selection` on a touch
screen"), so a touch player pages grids. What stays open is the arrows themselves.

## Open

- `Data\FE\Images\tilted_arrows.gtf` is not in `oag_hd::campaign::OTHER_TEXTURES`, so the two
  `Flyer Left/Right Arrow` images are never drawn on HD, and Omega (whose copy is `tilted_arrows.gnf`)
  reads the same list. `hd_grid_targets` still makes invisible targets of them.
- Their authored rects (`32x64` at `710,540` and `96x64` at `1700,540` of 1920x1080) are about
  21x47 px on a 1280x800 screen: too small for a finger.

## Next Steps

1. Add the art to the HD and Omega texture lists and look at the frame beside the original's capture.
2. Decide whether a drawn arrow gets a larger tap rect than its authored one (chosen, not measured).
