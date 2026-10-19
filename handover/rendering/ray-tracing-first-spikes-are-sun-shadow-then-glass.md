---
categories: [rendering]
---

# Ray tracing: first spikes are a traced sun shadow, then glass reflections

2026-10-19, `raytrace-spikes`, on its own branch (not merged). Evidence and ranking:
[docs/rendering/ray-tracing.md](../../docs/rendering/ray-tracing.md). Only the read-only probe
`crates/gpu/examples/ray_query_probe.rs` was written. Every cost number on that page is an estimate.

wgpu 30.0.1 exposes `EXPERIMENTAL_RAY_QUERY` on Vulkan, DX12 and Metal (not GL, not WebGPU), and this machine's RADV
reports it on both the 7800 XT and the Raphael iGPU. `shadows.md`'s "effectively Vulkan-only" is out of date. The
feature needs an `unsafe` token under a workspace that denies `unsafe_code`.

## Open

- Steam Deck, NVIDIA, the notebook's Intel iGPU and the Galaxy S24 (Xclipse 940) have not run the probe. The Deck and S24
  rows are inferred.
- Triangle counts per circuit, BLAS size and per-frame TLAS cost are unmeasured; memory figures are orders of magnitude.
- Whether draws can be separated per `AnimNode` so animated scenery is one instance per node, not a rebuild.
- Whether a traced shadow on the Deck beats `mapped` on cost (estimate says it does not, 2 to 4 ms against 1 to 1.6 ms);
  the case for the tier there is quality and split scaling.
- Alpha-tested geometry (cutout fences) casts solid in an opaque-only first spike.

## Next Steps

1. Lane A (about a day): request the features behind the settings-off path with a scoped `unsafe` token, `BLAS_INPUT`
   on the vertex and index buffers when the tier is on, build the track BLAS and craft TLAS, draw a `--ray-debug`
   picture, print triangle counts and BLAS size per circuit per title. Decide on an ADR for the token.
2. `graphics.shadows = traced` on HD, Omega and 2048 (one ray per pixel in `mesh.wesl`, multiplied with the lightmap
   alpha), measured with `just bench-render hd --tag base|rt --views 1,2h,2v` against `--shadows mapped`.
3. Run `cargo run -q -p oag-gpu --example ray_query_probe` on the Deck (under `deck.lock`), the notebook and an APK build.
4. Only then glass reflections (HD Vineta K, Talon's Junction floor), after the original's own glass paths are done.
