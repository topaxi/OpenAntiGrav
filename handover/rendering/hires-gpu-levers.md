---
status: measure
titles: [hd, pulse, 2048, omega]
updated: 2026-11-18
summary: "Deck native 4K: HD misses 60 Hz (16.6 ms p50 after circuit cuts), 2048 holds it, Omega near the line. Left: shadow redraw, rich pipelines"
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
  2. ~~**The bloom resolve, 3.85 ms at 4K.**~~ Landed 2026-11-18 (`hd-scene-8bit`): HD's scene is
     8-bit by default (`graphics.scene_precision`), the resolve one sRGB-view tap; Deck 4K resolve
     3.88 to 1.10 ms, whole-race GPU p50 18.3 to 16.0 ms, pairs unchanged
     (`docs/rendering/hires-gpu-levers.md`, "HD's scene in 8 bits").
  3. **The craft pass, 3.08 ms at 4K**, 4.5 ns a fragment against the circuit's 1.4. Not opened:
     its overdraw is unmeasured, and the `lit_by_rig` fold is not applied to it because a craft
     draws under a scene block of its own (`ship_scene`), which needs an audit first.
  4. ~~**The sun-removal arithmetic, 0.46 ms at 4K**~~: moved under the coverage test
     2026-11-18 (`circuit-lighting-rungs`), byte-identical and 0.8 % slower on the Deck; dropped.
  5. Eleven `pow` a pixel remain in HD's plain pipeline (lightmap curve, albedo decode, specular,
     fog, encode); the vertex light curve now runs per vertex, the original's order.
- An RGP capture of an HD 4K frame was not obtained (headless gamescope wrote no trace).
- **The first bloom downsample's read law** (`hd-scene-8bit`): through the sRGB view (averaging
  light, what renderer.md says the ladder does) the pairs move away, mean grey +0.02 to +0.13 on 12
  of 14; the downsample keeps averaging encoded texels. Read which the original runs (its half-res
  pass's unit gamma nibble in an RPCS3 dump) before changing it. Measure: the 14 HD pairs as in
  `docs/rendering/hd-track-tone.md`, "The scene target is 8-bit by default".
- **Opaque draws through an sRGB attachment view**: bounded at about 1.5 % of an NVIDIA 4K frame and
  0.37 ms of a Deck 4K frame before the cost of splitting the race pass; not built
  (`docs/rendering/hires-gpu-levers.md`, "Not landed: opaque draws writing through an sRGB attachment view").

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

- **2026-11-18, `deck-vita-ps4-perf`** (`docs/rendering/hires-gpu-levers.md`, "2048 and Omega on
  the Deck, pass by pass"). Landed: a circuit's plain blended draws shade through their own
  pipelines (Omega `track blended` 6.70 to 2.55 ms at 4K, whole-race frame p95 19.85 to 15.92 ms)
  and 2048's exposure readback over 256 invocations (0.27 to 0.09 ms at 4K). Omega's 15.9 ms
  is offscreen; a presented 4K output adds about 1 ms (unmeasured for Omega), so measure it
  presented before calling Omega 4K60. At 800p both titles
  are bound by the CPU. Left, largest first:
  1. **Omega's depth prepass, 1.2 to 2.0 ms, flat** (vertex-bound). A position-only prepass entry
     needs `@invariant` and a depth-equal proof. Measure: the `track solid` span at 800p.
  2. **Omega's tone map apply, 1.6 ms at 4K**: skip the distortion fetch where nothing refracted.
     Measure: `omega tonemap apply`.
  3. **The circuits' opaque shading** (2048 6.7 ms, Omega 8.4 ms at 4K): read
     `RADV_DEBUG=shaderstats` for their plain pipelines first.
  4. **Race bench gap**: `gpu_us` misses `wo_composite` and Omega's tone map and bloom chain;
     timestamps around them would let a whole race show a cut there.

- **2026-11-18, `circuit-lighting-rungs`** (`docs/rendering/hires-gpu-levers.md`, "Circuit
  shading: exact cuts, the vertex curve and a lighting rung"). Landed: five exact cuts and HD's
  vertex curve in the vertex stage (the original's order, renderer.md "The vertex curve is per
  vertex"); Deck `track solid, shaded` at 4K HD 10.49 to 8.65 ms, 2048 6.71 to 5.15, Omega 8.38
  to 6.32. `graphics.circuit_lighting` (`original` default, `simplified`): a further
  19 % on HD at 4K, but it buys only about 5 % of render scale at a 4K output, and exact
  lighting at 90 % looks better. Approximated light curves: no gain on RDNA2, not added.
  Left:
  1. **The rich pipelines' `programs_present` override** (HD 0.58 ms, Omega 0.80 ms at 4K is the
     whole rich half), and the craft pass, all rich (3 ms at 4K). Measure: the `track solid,
     shaded` and `craft, weapons, shields` spans, same command as the page.
  2. **Work the rung finds on few pixels**: `simplified` saves 7.8 % of Omega's shading at 4K for
     0.01 % to 12 % of its pixels (`tangents_present` per plain list, as `slots_present` is, is
     one try). A census of which plain draws carry them,
     and a pipeline of their own, may take most of that exactly. Measure: the same span on Tech de
     Ra, pixels diffed.
  3. **B (albedo decode through an sRGB view)**: one small lane judged by the 13 HD pairs;
     **D (trilinear lightmap sampler)**: read the original's lightmap unit filter first.
  4. **A's pair check is mixed** (7 of 13 HD pairs toward the original, 5 away): the pairs where
     ours is already too bright get brighter; read why those pairs are bright before blaming A.

## Next Steps

1. Decide item 1 (it moves the shadowed pixels toward the console's own structure); it is worth
   more at 4K than every exact cut left.
2. Only docked 4K still gains from the GPU (`docs/rendering/hires-gpu-levers.md`, "The Deck's verdict"):
   f16 in `lit_texel`, and fewer than about 104 VGPRs on HD's rich pipelines for more waves than 8
   (the plain pipeline is already at 40 VGPRs, 24 waves).
