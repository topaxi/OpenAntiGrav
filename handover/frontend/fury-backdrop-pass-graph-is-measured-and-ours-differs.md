---
status: ready-to-wire
titles: [hd]
updated: 2026-11-05
summary: "Fury backdrop's pass graph is wired but shipped level is 15 against the reference's 48; level gap, alpha bloom, equaliser and mode 11 remain"
categories: [frontend, rendering]
---

# The Fury backdrop's pass graph is wired; what the measured frame still has that ours does not

The laws and the numbers are on `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`, "The frame,
measured" and "The pass graph, wired". Blend, glow, wave, composite, band, scanlines, the final composite's half
texel and the linear pipeline landed. On Main Menu, frame 484, as shipped: mean red `2.2` to `15.0` against the
reference's `48.3`. The structure matches; the level does not yet.

## Open

1. **The level gap, unexplained.** The paused state read bands at zero, yet the frame reads three times what that
   gives; forcing a band of `0.95` brings it to `53.0` (fitted, not shipped). About half the difference is
   dot-fine, so a blur alone cannot be most of it. Find what the state read missed before wiring anything to it:
   read `particleColour` out of the frame's own ring (`scripts/hd-draw-dump.py`) beside the picture.
2. **The alpha bloom over the field.** The menu's ladder is live: gate `rgb * a * 0.75`, two blurs, added at `1`
   by the final composite. Ours keeps the field's alpha through every pass for this. `oag_post::bloom` already
   runs in `oag_game::render`'s glow path, so reuse that, not a second chain; its gate and blur weights need
   checking against `0x01111fc0`, `0x01111880`, `0x01111c00`. Its share of item 1 is unmeasured. One oag-wire lane,
   half a day (confidence 45).
3. **The equaliser tap** (a separate lane; nothing here stands in for it). Bands `+0x1b4..+0x1f0`, banks `+0x19c`
   and `+0x1dc`, threshold `g+0x1708`. Shipped silent: pulse `0.23`, glow at its floor, pivot off by `-0.05`.
4. **Mode 11's mip chain and quads**: only once morph paths draw at all. 250 quads at 720p, 501 at 1080p.

## Next Steps

1. Settle item 1, wire item 2, and re-run the frame-484 table's method: with `rpcs3_debugger.Debugger`, pause, read the state, grab
   the frame two seconds later. A shot of the running game lags the clip counter by about 100 frames.
2. Not to wire: a `40%` fade (20 dumped frames and both builds' float scans find none), the 720p field's 2x1 AA
   as anything but MSAA shaded once, and RPCS3's `sampleTexture2DMS` bug.
3. The `3` pixel sideways residual belongs to the dynamic-width thread.
