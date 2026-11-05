---
status: measure
titles: [hd, pulse]
updated: 2026-11-05
summary: "The Deck confirmed the track folds; only docked 4K still gains: f16, HD VGPRs, bloom, the per-craft pass merge"
categories: [rendering, tooling]
---

# Hi-res GPU levers: what is left

The tables, method and every lever's result are in `docs/rendering/hires-gpu-levers.md`. Lever 1
landed (`slots_present`) and its three successors landed 2026-11-04: the emissive table fold, a
pipeline for a circuit's plain draws, and the rig and Zone folds. Levers 2 (blur tile skip) and 3
(our 4K present against a 1080p gamescope window) were measured and dropped. Every pass of
`Scene::render` has a span (`docs/tools/render-bench.md`, "Every pass of one frame by name").

## Open

- **`between passes`** (x28 on HD, 0.5 ms on the Deck). Split it by what it holds
  (the race pass's clears and resolve first). Measure: read the span on the Deck, then move a mark.
- **Advert cards**: five are drawn every frame, 400 to 550 us and 900 k fragments on the iGPU.
  Whether the original redraws them every frame is unread; a lower rate moves pixels.
- **The per-craft sun-occlusion and self-shadow passes** stay separate passes:
  `handover/rendering/hd-sun-occlusion-and-self-shadow-passes-in-a-split.md`. The atlas now skips
  craft out of view, in one view only.
- **f16 in `lit_texel`** was not tried: nothing here can read an AMD register count, and on Intel it
  says nothing about the Deck. Try it on a branch with the Deck's `shaderstats` beside it, judged
  by the pairs (`scripts/compare-rerender-ours.py`'s `rerender` on copies of the pairs).
- **The iGPU's bloom and present composite** (+3.6 and +2.0 ms at 4K, bandwidth-shaped).
- **Gamescope on a real 4K output.** Headless gamescope does no GPU work and the Deck had no 4K
  display connected, so whether gamescope scales a 1080p window with a display plane (near-free) or
  a composite (3.8 to 4.3 ms measured nested) is unknown. Measure: a 4K TV on the dock, the game in
  Game Mode at a 1080p and a 4K window, `gpu_busy_percent` and `pp_dpm_sclk` at a 60 fps limit.
  Only worth doing if the answer is a plane: then a 1080p window saves 2.4 (HD) to 2.9 (Pulse) ms of
  GPU at the price of a softer HD HUD.
- The Deck's 4K clock bimodality (27 to 33 ms in one run) hides real 4K differences; a 4K Deck
  comparison needs a fixed clock or interleaved A B A B with many rounds.

## Next Steps

1. Only docked 4K still gains from the GPU (`docs/rendering/hires-gpu-levers.md`, "The Deck's verdict"):
   f16 in `lit_texel`, and fewer than about 104 VGPRs on HD's rich pipelines for more waves than 8.
