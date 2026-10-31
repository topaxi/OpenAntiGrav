---
status: measure
titles: [hd, 2048, omega]
updated: 2026-10-31
summary: "Bit-4 back-face culling is wired on HD (12 pairs, none worse); open: frustum part, EdgeGeom short counts, 2048 extra-mesh lead"
difficulty: medium (40)
---

# Vineta K's wall-top warm pool: the original culls the slab's back faces

Evidence and numbers: `docs/ghidra/functions/ps3-hdfury-eu/chunk-cull.md` and `docs/rendering/hd-track-tone.md`, "Resolved: the original
culls back faces per triangle" and "Wired per material".

## Open

- ~~**Wire it.**~~ Landed 2026-10-27 (`hd-backface-cull`): `honour_render_state` sets `DrawCall::culled` from state bit 4, `mesh_render::build`
  builds a culling twin of the opaque, prepass and cutout pipelines, `Drawable` switches per draw. Twelve HD pairs re-rendered, none worse
  (Vineta K 0.062 -> 0.059, Amphiseum 0.067 -> 0.062, Talon's 0.054 -> 0.048).
- The survivor count is `3 x` the front-facing in-frustum triangles for 1,139 of 1,523 culling surfaces; 349 draw a few triangles
  fewer (the SPU `EdgeGeom` cull's own frustum test, unread: `chunk-cull.md`, "What is not explained").
- Open exceptions: the Amphiseum's additive `lightbarrierpanel_new_additive` (chunk 573) is absent with triangles in view; weapon pads
  (chunk 829, Talon's) are predicted drawn and absent.
- ~~2048 and Omega~~ ported 2026-10-31 (cull bit only, inherited from HD): title rule + `psp2` builder fill + `Model::honour_cull`.
  Effect is tiny (Vineta K 24 pixels). HD's chunk 1556 counterpart on 2048 is chunk 47 and faces the eye at HD's pose, so it is
  still drawn; the maintainer's "extra meshes in 2048 Vineta K" is something else, open: locate it on a 2048 frame.

## Next Steps

1. ~~Per-draw back-face culling from material state bit 4~~ and ~~the tests~~ (done: `render_state` unit test, `mesh_render::pipeline_cache`
   twin test, `hd_light_bar_ground_truth::vineta_ks_wall_top_slab_has_no_front_facing_triangle_from_the_start_grid`).
2. Then the per-triangle frustum part (18 of the 30 census exceptions are boxes inside the frustum whose triangles all lie outside it):
   only worth wiring if a pair shows it; a GPU frustum clips them anyway.
3. Read the `EdgeGeom` SPU cull (`data/extracted/ps3/spu-jobs/edgegeom-0x007f6d80.dis`) for the 349 short counts.
4. ~~Omega/2048 cull wiring~~ done. Find the extra 2048 Vineta K mesh the maintainer sees (not culled by bit 4).
