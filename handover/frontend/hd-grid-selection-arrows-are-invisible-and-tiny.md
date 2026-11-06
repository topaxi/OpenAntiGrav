---
status: measure
titles: [hd, omega]
updated: 2026-11-06
summary: "Grid Selection arrows draw and tap; open: does the pad wrap (the original dims at range ends), Omega blend, HD left arrow"
size: S (60)
difficulty: easy (60)
---

# HD and Omega Grid Selection: the paging arrows

Landed: the two `Flyer Left/Right Arrow` images draw with `FlyerSelection`'s own per-frame colour (`0x646464`,
alpha `0xFF`, or `0x7F` with no page to go to) and each has a grown tap rect
(`docs/ui/campaign-screens.md`, "The flyer arrows draw and can be tapped";
`docs/ghidra/functions/ps3-hdfury-eu/flyer-selection-arrows.md`).

## Open

- **Does the original's pad wrap at the end of a range?** Its update dims the left arrow on the first card of a range and
  the right on the last, which reads as "nothing to page to". `GridSelection::step` wraps (chosen) and a dimmed arrow
  here still pages. A frame of a pad press on the last card settles it.
- **Omega's translucent arrow reads lighter than ours**: `omega/menu-grid-select`'s left arrow is `190` over a `243`
  backdrop; `0x7F` alpha blended in sRGB gives `178` (ours), in linear light `189`. HD's frame blends in sRGB space (50 over
  black, linear would be about 71), so this is a PS4 renderer difference in the blend, not the law. One frame; check
  another translucent Omega widget before changing a blend.
- **HD's left arrow cannot be compared box for box**: in `hd/menu-grid-select` it sits under the flyer's particle glow
  (the card art's error, not the arrow's).
- **A page flip** also writes the arrows' colour in the original's update; no frame caught one.

## Next Steps

1. Capture the original with the pad's right press held to the last card of the base campaign and read the right arrow's
   alpha and whether a further right press wraps.
2. If it does not wrap, make `GridSelection::step` clamp on the HD screens (Pulse's own pager keeps its measured wrap).
