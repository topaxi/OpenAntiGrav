# HD's culled weapon draws keep the faces that turn away from the eye

2026-09-25, found while drawing `HD_plasma_ball` through its own program
(`slots::RIM_EDGE`, [hd-unlit-programs.md](../../docs/rendering/hd-unlit-programs.md)).
It is why `blast_models::HD_PLASMA_BALL_DRAWN` is still `false`.

## What was measured

- `HD_plasma_ball` is two coincident spheres. `crates/render/examples/hd_unlit_probe.rs`
  on the raw `.rcsmodel`: 760 of 760 triangles counter-clockwise about their
  own vertex normal in both shells, 760 normals outward in one and 0 in the
  other. `--built` on what `mesh::rcs` hands the GPU: 760 of 760
  counter-clockwise in both draws, so nothing between the file and the vertex
  buffer mirrors it.
- Drawn with `mesh.wgsl` temporarily outputting `@builtin(front_facing)` and
  the sign of `dot(normal, eye - position)` for `RIM_EDGE` fragments (HD race,
  `--give plasma` with a grid-shot script, tick 118): every fragment that
  survives the cull is front-facing, and at the disc's centre its normal points
  **away** from the eye. The same with `plasma_ball_model_matrices` replaced by
  a bare translation, so it is not a reflection in the placement. Debug frames
  under `data/scratch/hd-unlit/` (`crop-ff2-118.png`, `crop-ff3-118.png`).
- On a convex shell wound counter-clockwise about outward normals, the
  surviving faces should face the eye under `mesh_render`'s counter-clockwise
  front face. They do not, so every culled HD `.rcsmodel` draw is suspect:
  `cull_as_authored` covers the Rocket, the Bomb and the Plasma blast's ring,
  sphere and halo. The halo's `blast_models::facing_away` sign was chosen
  under the same cull.
- The original, read on `EBOOT.elf`
  ([material-state.md](../../docs/ghidra/functions/ps3-hdfury-eu/material-state.md),
  2026-09-25 section): `Rsx_SetFrontFace` is only ever called with `0x901`
  (`GL_CCW`) and `Rsx_SetCullFace` with `0x405` (`GL_BACK`), `0x404` in the
  shadow passes only.

## Open

- Why this renderer's cull disagrees with the geometry on HD when it agrees on
  Pulse (measured there: `mesh_render.rs`, the "Front faces are
  counter-clockwise" comment). Candidates, none checked: a handedness flip
  somewhere in the HD race's view or projection that the unculled HD scenery
  would never show; the viewport `y` sign the original programs, which decides
  what window-space `GL_CCW` means in clip space.
- `LeachBall` does not depend on this: its two materials disagree on the cull
  bit, so it draws both faces, and the program's own arithmetic makes the far
  faces add nothing.

## Next Steps

1. Read the original's viewport scale (`NV4097_SET_VIEWPORT_SCALE`, method
   `0x0a30`) where `Gcm_InitDevice` sets it, and its sign on `y`.
2. Check the Rocket and the Plasma blast shells under the same `front_facing`
   debug output; if they show the same inversion, flip the HD cull (a per-title
   front face, not a per-model one) and re-judge all five against the
   programs.
3. Then flip `HD_PLASMA_BALL_DRAWN`, or delete it, and look at a bolt in
   flight at player size.
