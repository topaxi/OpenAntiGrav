---
status: ready-to-wire
titles: [hd]
updated: 2026-11-05
summary: "Fury backdrop's pass graph is wired; the alpha bloom over it, the equaliser tap and mode 11's chain remain"
categories: [frontend, rendering]
---

# The Fury backdrop's pass graph is wired; what the measured frame still has that ours does not

The laws and the numbers are on `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`, "The frame,
measured" and "The pass graph, wired". Blend, glow, wave, composite, band, scanlines, the half-row present and
the linear pipeline landed; on Main Menu, frame 484, mean red `12.6` to `52.8` against `48.3` and scanline rows
match to two bytes.

## Open

1. **The alpha bloom over the field.** The menu's ladder is live: gate `rgb * a * 0.75`, two blurs, added at `1`
   by the final composite. Ours keeps the field's alpha through every pass for this. `oag_post::bloom` already
   runs in `oag_game::render`'s glow path, so reuse that, not a second chain; its gate and blur weights need
   checking against `0x01111fc0`, `0x01111880`, `0x01111c00`. Expect it to close `R q99` (`192` against `255`).
   One oag-wire lane, half a day (confidence 55).
2. **The equaliser tap** (a separate lane; nothing here stands in for it). Bands `+0x1b4..+0x1f0`, banks `+0x19c`
   and `+0x1dc`, threshold `g+0x1708`. Shipped silent: pulse `0.23`, glow at its floor, pivot off by `-0.05`, which
   is why the as-shipped level is `14.9` against a music-lit `48.3`.
3. **Mode 11's mip chain and quads**: only once morph paths draw at all.

## Next Steps

1. Wire item 1 and re-run the frame-484 table's method: with `rpcs3_debugger.Debugger`, pause, read the state, grab
   the frame two seconds later. A shot of the running game lags the clip counter by about 100 frames.
2. Not to wire: a `40%` fade (20 dumped frames and both builds' float scans find none), the 720p field's 2x1 AA
   as anything but MSAA shaded once, and RPCS3's `sampleTexture2DMS` bug.
3. The `3` pixel sideways residual belongs to the dynamic-width thread.
