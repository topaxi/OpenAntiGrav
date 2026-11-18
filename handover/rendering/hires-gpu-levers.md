---
status: measure
titles: [hd, pulse]
updated: 2026-11-18
summary: "HD native 4K still misses 60 Hz on the Deck: the shadow redraw the original's way, an 8-bit scene, the craft pass and f16 are left"
categories: [rendering, tooling]
---

# Hi-res GPU levers: what is left

The tables, method and every lever's result are in `docs/rendering/hires-gpu-levers.md`. Lever 1
landed (`slots_present`) and its three successors landed 2026-11-04: the emissive table fold, a
pipeline for a circuit's plain draws, and the rig and Zone folds. Levers 2 (blur tile skip) and 3
(our 4K present against a 1080p gamescope window) were measured and dropped. Every pass of
`Scene::render` has a span (`docs/tools/render-bench.md`, "Every pass of one frame by name").

## Open

- **2026-11-18, `gpu-hires`** (`docs/rendering/hires-gpu-levers.md`, "The 4K frame on the Deck pass
  by pass"). HD at 4K on the Deck: circuit shading 48 % of the spans, the bloom resolve 16 %, the
  craft 13 %. Landed: `lit_by_rig` (-7.4 % of the circuit's shading at 4K, whole-race GPU p50 18.82
  to 18.36 ms). Dropped with numbers: the resolve through a decoded copy, four cuts of the craft
  shadow lookup, the light curves behind `if`s. What is left for 4K, largest first:
  1. **The craft shadow lookup, 1.34 ms of 11.4 ms at 4K (0.9 ms of a frame back to back).** No
     per-pixel shortcut pays (four measured). The original's structure does: redraw the chunks
     within reach of a craft through a sunless permutation, blended, so no other pixel asks. It
     composites on encoded values, so shadowed pixels move: a rendering change with a pair check,
     not a speed lane. Measure: the `track solid, shaded` span, same command as the page.
  2. **The bloom resolve, 3.85 ms at 4K.** Its cost is four sRGB decodes a pixel and the encode.
     The original's scene is `A8R8G8B8` and its units decode on fetch; an 8-bit scene target read
     through an sRGB view would make the resolve one hardware tap (about 1.2 ms measured for a
     one-tap resolve, so about 2.6 ms off the span, less at the race's clock). It rounds the scene
     to 8 bits as the console does, which moves the picture: the maintainer's call.
  3. **The craft pass, 3.08 ms at 4K**, 4.5 ns a fragment against the circuit's 1.4. Not opened:
     its overdraw is unmeasured, and the `lit_by_rig` fold is not applied to it because a craft
     draws under a scene block of its own (`ship_scene`), which needs an audit first.
  4. **The sun-removal arithmetic, 0.46 ms at 4K**: computed for every pixel, used where a shadow
     covers one.
  5. Fourteen `pow` a pixel remain in the plain pipeline, all authored (lightmap curve, vertex
     light curve, albedo decode, specular, fog, encode).
- An RGP capture of an HD 4K frame was not obtained (headless gamescope wrote no trace).

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

1. Decide items 1 and 2 above (both move the original's picture toward the console's own
   structure); either is worth more at 4K than every exact cut left.
2. Only docked 4K still gains from the GPU (`docs/rendering/hires-gpu-levers.md`, "The Deck's verdict"):
   f16 in `lit_texel`, and fewer than about 104 VGPRs on HD's rich pipelines for more waves than 8
   (the plain pipeline is already at 40 VGPRs, 24 waves).
