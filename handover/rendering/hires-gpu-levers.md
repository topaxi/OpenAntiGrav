---
status: measure
titles: [hd, pulse]
updated: 2026-10-31
summary: "Leftovers of the hi-res GPU pass table after the three levers (track solid landed, blur skip and 1080p gamescope window dropped): unnamed Scene::render time, MSAA cost, gamescope on a real 4K output"
categories: [rendering, tooling]
---

# Hi-res GPU levers: what is left

2026-10-31. The tables, method and the three levers' results are in
`docs/rendering/hires-gpu-levers.md`. Lever 1 landed (`slots_present`, `mesh_render::slots_present`);
levers 2 (blur tile skip: 0.0 % of race pixels can skip) and 3 (our 4K present against a 1080p
gamescope window) were measured and dropped.

## Open

- **2.0 ms (Pulse) and 3.7 ms (HD) of `Scene::render` on the Deck that no span names**, even with
  shadows off. Measure: the span recipe in the doc with `--shadows off`, then add perf-probe marks
  around the unspanned passes. Size 40, difficulty 40 (confidence 30).
- **MSAA 4x costs +2.8 ms on the HD race pass at 1080p on the Deck** (the resolve lands inside the
  race pass span). Measure: `--msaa 4x` against off with the span recipe. Size 40, difficulty 50
  (confidence 30).
- **The iGPU's bloom and present composite** (+3.6 and +2.0 ms at 4K, bandwidth-shaped).
- **Gamescope on a real 4K output.** Headless gamescope does no GPU work and the Deck had no 4K
  display connected, so whether gamescope scales a 1080p window with a display plane (near-free) or
  a composite (3.8 to 4.3 ms measured nested) is unknown. Measure: a 4K TV on the dock, the game in
  Game Mode at a 1080p and a 4K window, `gpu_busy_percent` and `pp_dpm_sclk` at a 60 fps limit.
  Only worth doing if the answer is a plane: then a 1080p window saves 0.5 (HD) to 2.9 (Pulse) ms of
  GPU at the price of a softer HD HUD.
- The Deck's 4K clock bimodality (27 to 33 ms in one run) hides real 4K differences; a 4K Deck
  comparison needs a fixed clock or interleaved A B A B with many rounds.

## Next Steps

1. The unnamed `Scene::render` time on the Deck: biggest remaining flat cost.
