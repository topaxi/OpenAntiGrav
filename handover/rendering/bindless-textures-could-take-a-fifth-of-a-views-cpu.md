---
status: measure
titles: [all]
updated: 2026-11-04
summary: "Bindless textures on the Deck: 5 to 10% less frame-thread CPU, no clash with the split finish; 1080p and 4K and GPU time unmeasured"
categories: [rendering]
---

# Bindless textures: built for native, the Deck unchecked

2026-10-15 lead's plan, built 2026-10-16 (`bindless`). Evidence and numbers:
`docs/tools/render-bench.md` "Bindless textures", `docs/rendering/split-screen.md` "Cost".
Related: [wgpu-levers-for-per-view-cpu-none-used-yet.md](wgpu-levers-for-per-view-cpu-none-used-yet.md)
(its lever 4) and
[split-screen-cpu-is-the-drivers-encoder-finish-now.md](split-screen-cpu-is-the-drivers-encoder-finish-now.md).

Landed: on a device with `TEXTURE_BINDING_ARRAY`, `PARTIALLY_BOUND_BINDING_ARRAY` and `IMMEDIATES`,
every race drawable binds group 1 once (its distinct textures and samplers as binding arrays) and
each draw pushes its slot's five indices as immediates (`oag_mesh::mesh_render::bindless`). One WESL
source with a `bindless` feature; the default `mesh.wgsl` is byte-identical to before, and the web,
GL and featureless devices draw exactly as before. `OAG_BINDLESS=0` selects the per-slot path on one
binary. NVIDIA 2h: the second view's `finish` Pulse 208 to 124 us, HD 283 to 160, Pure 138 to 88;
the frame's CPU -7 to -16%; the iGPU's pipelined frame unchanged. Twelve stills byte-identical on
both paths; `oag_raceplay::tests::bindless` guards the slot-to-texture mapping.

**The feature set differs from the plan.** The plan named `TEXTURE_BINDING_ARRAY` and
`SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING`. Non-uniform indexing is not used:
a slot index from a varying made naga mark every array read `NonUniform`, and the iGPU's frame rose
6 to 16% for it, so the index is an immediate (uniform) instead. `PARTIALLY_BOUND_BINDING_ARRAY` is
needed because padding the arrays to a fixed length made `finish` slower than the switches.

## Open

- **The Steam Deck, measured 2026-11-04 (CPU at 800p; GPU time, 1080p and 4K not yet).** RADV VANGOGH
  exposes the features (the arrays are in use). Pure on Vineta K, a 1280x800 window drawn at 1920x1200 (the Deck profile's render scale 150),
  headless gamescope,
  `vsync = "off"`, `frame_limit = "unlimited"`, dist build (znver2), `OAG_BINDLESS=0` against the
  default on one binary, 3 interleaved rounds, mean frame-thread ms a frame (`frame-phases` total):
  two-player split 8.19 -> 7.33 with the split finish on, 7.95 -> 7.15 with `OAG_SPLIT_FINISH=0`;
  one view 4.53 -> 4.27 with it on, 5.15 -> 4.86 off. The same 0.8 ms in a split and 0.3 ms in one
  view with the split finish on or off, so the two do not clash. fps was 102.6 (split) and 120.5
  (one view) in every arm, paced by something other than this CPU time, so these are CPU headroom
  figures. Open: the GPU side (`gpu_ms`) on the Deck, 1080p and 4K, HD and Pulse.
- **The web smoke test** (`just check-web-shaders`) skips the variant by its `enable
  wgpu_binding_array;` directive; it ran only in CI, not in this lane.
- **What is left of the ceiling**: the arrays take 53 to 70% of what the no-switch hack removed.
  What remains is a group-1 bind per drawable call, an immediate per slot change, and wgpu's
  per-element tracking each time a group is bound (a drawable with 190 distinct textures pays 190
  checks a bind). Not profiled further.
- **A bench that runs in one process**: an A/B costs a race load per run, so this lane's first full
  A/B took about 50 minutes. Loading once and alternating the two paths frame by frame would take
  most of that out (`OAG_BINDLESS` is read at build, so the drawables would need both groups).

## Next Steps

1. On the Deck: the GPU side of the A/B (`OAG_RENDER_GPU_BENCH`, the block in
   `docs/tools/render-bench.md`, "Bindless textures") at 1080p and 4K, and HD and Pulse beside Pure.
2. If the Deck's GPU time rises at a size, gate the variant per adapter and say why in `bindless.rs`.
