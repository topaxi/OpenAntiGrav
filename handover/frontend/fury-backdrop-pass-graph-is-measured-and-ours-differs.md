---
status: ready-to-wire
titles: [hd]
updated: 2026-11-05
summary: "Fury menu backdrop's real pass graph is measured (02.00, RPCS3); ours tints in the wrong pass and lacks band, scanlines, glow gain"
categories: [frontend, rendering]
---

# The Fury backdrop's pass graph is measured, and ours has it wrong in five places

All the laws are on `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`, "The frame, measured".
Size: one oag-wire lane, a day (confidence 60). `crates/game/src/render/backdrop.rs` and
`crates/ui/src/backdrop.rs` are the only code it touches.

## Open

1. **Tint in the wrong pass.** Ours scales the composite (`fs_composite`, `encode(tint *
   decode(sum))`). The original multiplies the fresh field by the tint in the trail blend, before
   the `0.647` feedback, so a low tint also shortens what accumulates. The composite has blending
   off. Replacing this retires the `encode(tint * decode)` law and its 0.07-0.09 miss.
2. **No linear pipeline.** Passes 1 to 25 decode on read and encode on write. Ours is gamma-space
   (ADR-0020) with one shader doing it by hand. Keep ADR-0020 for the page, run the backdrop's own
   targets linear (an sRGB view or the same by hand). A real decision: say which in the lane report.
3. **Glow gain missing.** The half-target add with `CONSTANT_COLOR` is modelled as `srcScale` in
   the blend. Gain is `0.02` to `0.15` (one byte: 5 to 38), driven by the `+0x1dc` band mean. With
   no equaliser it sits at the floor.
4. **Wave grid, band, scanlines absent.** Four quads around a moving pivot; `FunkLayer_DrawScanBand`
   with its `0.01` a frame on a 200-frame wrap; every fourth row times `(0.05, 0.1, 0.25)`.
5. **Mode 11's mip chain and quads**: only once morph paths draw at all.

Bonus, small: `crates/ui/src/backdrop.rs` says the bands sit at `Music Pulse Base`, and in another
place at zero. The page says zero. Read the code before trusting either comment.

## Next Steps

1. Wire 1, 2, 3 and 4, in that order, each commit a before-and-after at 1920x1080 against
   `data/reference/compare-page/hd/menu-*` (720p frames are biased, `rpcs3-capture.md`).
2. Test that fails if the tint moves back to the composite: tint `0` still leaves the decaying trail.
3. The equaliser tap (bands `+0x19c..`, `+0x1dc..`) is a separate lane; do not stand in for it here.
4. Not to wire: a `40%` fade. A third party describes one; 20 dumped frames and both builds' float
   scans find none.
