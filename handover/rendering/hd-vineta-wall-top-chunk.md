---
status: ready-to-wire
titles: [hd]
updated: 2026-10-27
summary: "Cause found: the original culls back faces (material state bit 4) per triangle, ours culls none of HD track; honour bit 4 per draw, scored on 3 pairs"
difficulty: medium (40)
---

# Vineta K's wall-top warm pool: the original culls the slab's back faces

Evidence and numbers: `docs/ghidra/functions/ps3-hdfury-eu/chunk-cull.md` and `docs/rendering/hd-track-tone.md`, "Resolved: the original
culls back faces per triangle".

## Open

- **Wire it.** Our HD track draws every triangle of every chunk; the original draws only the triangles that are counter-clockwise
  (window space, front face CCW) when the material's state word (`material + 0x10`) has **bit 4** set, and inside the frustum.
  Chunk 1556 (slot 633) has only clockwise triangles, so it vanishes. This is not a chunk skip and not an unreferenced-pass change.
- The survivor count is `3 x` the front-facing in-frustum triangles for 1,139 of 1,523 culling surfaces; 349 draw a few triangles
  fewer (the SPU `EdgeGeom` cull's own frustum test, unread: `chunk-cull.md`, "What is not explained"). Not needed for the wiring.
- Open exceptions: the Amphiseum's additive `lightbarrierpanel_new_additive` (chunk 573) is absent with triangles in view; weapon pads
  (chunk 829, Talon's) are predicted drawn and absent.
- 2048 and Omega: census bit 4 of their material state words and whether their tracks cull the same way (no draw list exists).

## Next Steps

1. **Per-draw back-face culling from material state bit 4, HD track chunks** (`oag_mesh::mesh::rcs` sets a `DrawCall` flag from
   `Material::state & 0x10`, like `render_state.rs` does for bit 2 `writes_depth`; `oag_mesh::mesh_render` builds the opaque, cutout and
   blended pipelines with `cull_mode: Some(Face::Back)` per `(blend, cull)` key). Front face is wgpu's default CCW; no flip was needed.
   The prepass's `reversed_indices` (`crates/raceplay/src/drawable.rs`) reverses triangle order, not vertex order within a triangle,
   so winding survives. A blanket test (`cull_back = true` for the track `Drawable` at `crates/raceplay/src/scene.rs`, the `false` before
   `Velocity::Write`) already improves three pairs: Vineta K pool core error 34.3 -> 3.6, footprint 7.96 -> 5.55, full-frame rmse
   0.0618 -> 0.0598; the Amphiseum 0.0674 -> 0.0635; Sebenco 0.0639 -> 0.0624. Per material, 8 of 1,531 matched surfaces have bit 4
   clear and must stay double-sided.
2. **Tests that fail if the law is dropped**: a ground-truth test that every triangle of Vineta K chunk 1556 is clockwise from the
   start-grid eye so its surfaces cull to nothing, and a unit test that a bit-4 material builds a culling draw and a bit-4-clear one does not.
3. Then the per-triangle frustum part (18 of the 30 census exceptions are boxes inside the frustum whose triangles all lie outside it):
   only worth wiring if a pair shows it; a GPU frustum clips them anyway.
4. Read the `EdgeGeom` SPU cull (`data/extracted/ps3/spu-jobs/edgegeom-0x007f6d80.dis`) for the 349 short counts.
