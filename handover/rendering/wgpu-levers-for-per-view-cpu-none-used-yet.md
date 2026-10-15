---
categories: [rendering]
---

# wgpu levers for the per-view CPU: none of them is used yet

2026-10-15, drive lead, from the split-screen CPU measurements in
[render-bench.md](../../docs/tools/render-bench.md) (the 2026-10-15 notebook sections) and
[split-screen.md](../../docs/rendering/split-screen.md) ("Cost"). Per view of a split race on
the notebook's NVIDIA dGPU, `encoder.finish` (wgpu validating and tracking the recorded
commands) is 40-50% of the view's CPU and the `queue.write_buffer` uniform uploads 25-40%.
Those costs are in a one-view race too.

A tree-wide search on wgpu 30.0.1 finds **no render bundles, no push constants, no
multi-draw-indirect, no texture binding arrays and no staging belt**. The levers below are
ranked by how directly they hit those two costs. None is tried; the expected effects are
readings of how wgpu works, not measurements.

## Open

1. **Render bundles for view-independent draws.** A bundle is validated once when it is
   recorded; executing it inside a pass skips most per-command validation. The circuit,
   scenery and static props do not change between frames, so recording them once and
   replaying them every frame and every view goes straight at `finish`. Prerequisite: a draw
   in a bundle cannot take a per-view uniform written per drawable, so the camera data must
   come from a shared per-view buffer first (lever 3, landed for both bind groups it needs).
   Largest expected gain on paper, **but not cheap**: a bundle's commands are fixed when it is
   recorded and every drawable's draw list changes each view with the culling and the level of
   detail (and its dynamic offsets differ per view), so only draws that never change, which the
   circuit's are not, could be replayed.
2. **Push constants** (`Features::PUSH_CONSTANTS`). Per-draw data such as a model matrix goes
   in the command stream: no `write_buffer`, no bind group. Native only, not WebGPU, so the
   web build keeps a fallback path.
3. **Dynamic offsets into one large uniform buffer, filled by one `write_buffer` a frame.**
   Replaces about 40 small writes a view, and fewer distinct resources is also less for
   wgpu's resource tracking at `finish`. Works on every backend including the web.
   **Landed on `encoder-finish` (2026-10-15)** for the scene block (bind group 2: one block per
   view and kind, 33 writes a view fewer on Pulse) and for every drawable's matrices (bind group 0:
   one region per view, written in runs; Pulse 56 to 36 writes a view in one view, 93 to 45 in two).
   Measured: about 4 to 5% of Pulse's CPU for the scene blocks, a further 3 to 8% from the matrices
   and the craft animation tables written once a frame. It did **not** move `finish`, which counts
   commands, not uploads.
4. **Texture binding arrays** (`TEXTURE_BINDING_ARRAY` and
   `SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING`). One bind group holding a
   drawable's textures, indexed in the shader, removes most of HD's bind switches (its
   circuit draws about 800 batches). Vulkan on the Steam Deck is expected to support it,
   unchecked; WebGPU does not, so a fallback stays. **Measured ceiling (`encoder-finish`,
   2026-10-15):** a timing-only hack that never re-binds group 1 takes the second view's
   `encoder.finish` from 216 to 96 us on Pulse, 273 to 98 on HD, 133 to 39 on Pure (55 to 65% of
   it, 20 to 25% of a view's CPU). Not built: sorting by texture moves pixels (the 2026-10-03
   coplanar-tie note), a `texture_2d_array` needs one size and format, an atlas breaks repeat
   wrapping, and `binding_array` is native only (two mechanisms) with five bindings in group 1,
   one of them a per-material sampler, in every title's mesh shader. Merging contiguous
   same-texture batches into one `draw_indexed` was tried and reverted: it took 19 us out of HD's
   `finish` and nothing out of Pulse's.
5. **Multi-draw indirect** (`MULTI_DRAW_INDIRECT`, `MULTI_DRAW_INDIRECT_COUNT`). Many draws
   per command; with a GPU cull it would also take the CPU frustum cull off the frame. The
   largest rewrite: only if 1-4 leave the Deck short.

**A check before reading any dev-build number as a release cost:** `oag_game::adapter::instance`
builds `InstanceDescriptor::new_without_display_handle()` with default flags, which in wgpu
take their validation/debug flags from the build configuration. Release benches are
unaffected; `just play` and `test-data`'s `dev` profile are not. Confirm what flags a dev
build actually runs with.

Also related and already written down elsewhere: concurrent per-view recording on
`oag-thread` (the "Concurrency share" table in
[split-screen-cpu-is-the-drivers-encoder-finish-now.md](split-screen-cpu-is-the-drivers-encoder-finish-now.md)),
which levers 1 and 3 make easier because a bundle or a shared read-only uniform has no
per-view mutable state.

## Next Steps

1. Read `encoder-finish`'s result (levers 3 and 4, and 1 if it got there) and strike what
   landed.
2. Render bundles for the circuit and static scenery, judged by what they remove from the
   `finish` probe span, one view and 2h, pictures byte-identical.
3. Push constants for per-draw matrices on native, with the web path unchanged.
4. Measure each on the Deck (the notebook's hybrid CPU needs pinned, interleaved A/B runs).
