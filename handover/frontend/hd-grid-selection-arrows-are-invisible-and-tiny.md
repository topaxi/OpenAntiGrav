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
  21x43 px on a 1280x800 screen: too small for a finger.

- Tried: adding `tilted_arrows.gtf` to `OTHER_TEXTURES` draws both arrows (HD, `--menu-page grid-select`,
  1920x1080) at the authored rects, about 18 px off the card's right edge against the original's
  about 22 px, so the place is right. **The colour is wrong**: the authored `Color="0xFF8AC0CA"` (HD
  blue) draws as `(138,192,202)`, where the original's right arrow reads `(100,100,100)` (grey) in
  `data/reference/compare-page/hd/menu-grid-select/theirs.png`, and its left arrow is a faint grey
  shape at the card's left edge. The change was not committed; the original does not paint this
  image with its authored `Color`, and why (a tint, a state, an attribute it ignores) is unread.

## Next Steps

1. Find why the original draws the arrows grey (`Block` colour state, a tint), then add the art to `OTHER_TEXTURES` (Omega's loader already falls back from `.gtf` to the `.gnf` sibling) and compare the frame.
2. Decide whether a drawn arrow gets a larger tap rect than its authored one (chosen, not measured).
