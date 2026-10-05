# Motion blur grain: the tile wobble is gone, the tap dither is what is left

2026-10-05. A maintainer report from play ("quite grainy and not smooth").
`docs/rendering/motion-blur.md`, "Grain, and what replaced the tile wobble",
has the measurements.

## Open

- The tap dither (`jitter()` in `crates/post/src/motion_blur.wgsl`) is still
  per-pixel white noise, static in screen space. About 1.3 of the remaining
  3.44 hp-RMS on a Pulse frame; the whole of the grain on a synthetic
  high-contrast ramp. A different hash (interleaved gradient noise, Bayer 4x4)
  measured no better.
- `BLUR RESOLUTION half` reads about 0.7 grainier than full on top; not fixed.
- Not judged by a player yet: the bilinear lookup landed on measurement and
  captures only.

## Next Steps

1. Ask the maintainer whether the smear still reads grainy at `high`.
2. If so, animate the dither with a frame index (a uniform field, which
   changes the 112-byte layout test) and accumulate it through the FSR 3
   history, or add a depth-aware 3x3 filter on the blurred region only.
3. Cheapest alternative, measured: `TAP_SPACING_PX` 3.0 and `TAPS` 21, 3.16
   hp-RMS for about 1.4x the gather.

## Ghosting follow-up (2026-10-05)

Open: ghosting not reproduced. Uniform-velocity scenes read the same on old and
new shader (see motion-blur.md). Next: a velocity-gradient scene (ramp from
`(v,0)` to `(0,v)`, short segments not infinite lines) measuring row-to-row
coherence of tap gaps, old vs new; then try nearest lookup with jittered taps.
