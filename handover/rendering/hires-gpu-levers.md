---
status: measure
titles: [hd, pulse]
updated: 2026-10-31
summary: "Three measure lanes from the 4K-vs-1080p pass table: track solid cost bisect, blur gather tile skip, our 4K present vs a 1080p gamescope window"
categories: [rendering, tooling]
---

# Hi-res GPU levers: three lanes from the measured pass table

2026-10-31. The table, method and numbers are in `docs/rendering/hires-gpu-levers.md`.
Re-measure any lane with `OAG_RENDER_GPU_BENCH=100 OAG_RENDER_PERF=1 target/release/oag-game
<image> --race --mode single_race --seed 1 --autopilot --no-audio --size WxH --ticks 2500
--screenshot out.png` (release `--features perf-probe`; the Deck is a `dist` znver2 build from the
`oag-appimage-build` container, run over ssh offscreen), median of 3 processes. Read GPU spans, and
the Deck's 800p and 1080p rows; its 4K rows are unstable to about 15 % (dpm `auto`).

## Open

- **Lane A: bisect the track solid pass's cost on the Deck.** Claim: HD's track solid draw is
  8.2 ms of the 10.4 ms race pass at 1080p (3.1 ns a fragment at 1.0x overdraw, about 5 chip
  cycles), anisotropy is only 0.3 ms and the velocity write is gated, so the cost is ALU, texture
  fetch count or ROP bandwidth. Method: diagnostic-only runs (env switches, never committed) that
  (1) output a constant colour, (2) sample one texture, (3) skip the colour write, each against the
  baseline span. Then cut the largest piece without changing a pixel (`cmp` of 6 frames per title
  against the baseline binary). Acceptance: track solid span down 5 % or more on the Deck at 1080p,
  HD and Pulse, 3 processes, picture byte-identical. Estimate 5 to 15 % of the pass; size 55,
  difficulty 60 (estimate from the span table, no profile of the shader yet).
- **Lane B: skip the motion blur gather where the tile cannot smear.** Claim: the gather
  (`motion blur reconstruct`) is 85 to 92 % of the blur chain (Deck HD 1080p 5.6 of 6.6 ms; +10.2 ms
  at 4K) and the tile-max pass already knows each tile's largest velocity. Method: an early-out in
  the gather for pixels whose tile-max is under the minimum blur length (a per-tile branch, no
  indirect dispatch, no unsafe wgpu). Acceptance: reconstruct span down 15 % or more on a race
  tick on the Deck at 1080p; picture identical outside a sub-pixel tolerance, judged with the
  frame-compare tooling on the same ship and track at three race ticks; no change when the ship
  is at full speed in a corner. Estimate 1 to 3 ms at 1080p; size 40, difficulty 40.
- **Lane C: our 4K present against a 1080p gamescope window.** Claim: with the game given a
  4K window under gamescope and drawing 1906x1080, our upscale and composite add 1.7 ms (HD) and
  3.0 ms (Pulse) of game frame time against a 1080p window that gamescope scales (HD 11.57 against
  9.88 ms, Pulse 9.04 against 5.99). Method: measure gamescope's own pass (its GPU time with the
  game at 1080p, e.g. `gpu_busy_percent` and a frame-time A/B at a fixed frame limit) to see whether the work moves or
  goes away; then the HUD at 4K versus scaled (ADR-0036's reason for compositing at presentation
  size). Acceptance: the total GPU time of game plus gamescope, A/B, at a 4K output; a frame-compare of the
  HUD both ways. If gamescope's scale costs as much, drop the lane. Estimate 0 to 3 ms; size 30,
  difficulty 30.
- Not top three: 2.0 ms (Pulse) and 3.7 ms (HD) of `Scene::render` on the Deck that no span names,
  even with shadows off; MSAA 4x (+2.8 ms HD race pass at 1080p on the Deck); the iGPU's bloom
  and present composite (+3.6 and +2.0 ms at 4K, bandwidth-shaped).
- The Deck's 4K clock bimodality (27 to 33 ms in one run) hides real 4K differences; a 4K Deck
  comparison needs a fixed clock or interleaved A B A B with many rounds.

## Next Steps

1. Lane B first: smallest and the most contained (one shader, picture-checkable).
2. Lane A needs a Deck session with the diagnostic env switches; do it in the same session as lane C.
