---
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

- **The Steam Deck (RADV) is unchecked**: whether wgpu 30 exposes the three features and the
  binding-array and immediate limits there, and whether its pipelined split frame stays level. It is
  GPU-bound like the iGPU, so that frame decides. Commands: `docs/tools/render-bench.md`, "Bindless
  textures".
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

1. On the Deck: the first command block in `docs/tools/render-bench.md` ("Bindless textures"); if
   the race log names drawables on binding arrays, the A/B block.
2. If the Deck lacks a feature, nothing changes there; if its pipelined frame rises, gate the variant
   per adapter and say why in `bindless.rs`.
