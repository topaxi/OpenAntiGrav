---
status: ready-to-wire
titles: [omega]
updated: 2026-11-12
summary: "Omega's tarmac_* road families name no Uv1, so the loader reads uvMarkings for the road picture: arena's black patch; read uv1_uvDirt.xy instead"
---

# Omega's `tarmac_*` roads take `uvMarkings` for their diffuse picture

Measured by `omega-arena-square` (2026-11-12). Evidence and numbers:
[`omega-status.md`](../../docs/formats/omega-status.md), "The `tarmac_*` road families draw the
wrong coordinate set". Compare pairs `arena-2048` and `square-2048` (originals captured from the
`wipeout_pc` port) are in the compare-page adoption batch.

## What is wrong (confidence 85 on arena)

Rockway Stadium's road shows a black, ragged patch around the craft. It is the road material
`tarmac_markings_dirt_markingsnormal_emis`, drawn with the wrong texture coordinate set:

- The material's vertex programs take `uvMarkings` and `uv1_uvDirt` (four halves: the tiling pair
  `.xy`, the dirt pair `.zw`). The vertex declaration has no `Uv1`.
- `oag_rcs::rcsmodel::psp2::vertex_decl::VertexDecl::own_diffuse_texcoord` then takes its
  confidence-70 fallback (first half pair that is not `lightmapUV`), which is `uvMarkings`: the
  markings atlas coordinate, u 0.79 to 0.99 over 250 units, so 4096 x 4096 `road_a_ds` is sampled
  as a few dozen texels of its dark top-right strip.
- Probe: reading `uv1_uvDirt` at its declared offset (28) as the diffuse coordinate gives grey
  asphalt, the yellow kerb line, lane dashes and the cyan edge line, as the original does.

Reach: 661 submeshes over mall, tower, square, park, cathedral, subway, arena, bridge (8 of the 10
`environments2048` circuits); `crates/rcs/examples/psp2_uv_census.rs` lists them per circuit and
material. Subway's road changes visibly with the probe, Mall's barely.

## Next Steps

1. In `crates/rcs/src/rcsmodel/psp2.rs` (`submeshes`, the `diffuse` attribute choice), when the
   declaration carries `uv1_uvDirt` (`name_hash("uv1_uvDirt")` = `0x273a_b13e`, four half components)
   and no `Uv1`, read the diffuse coordinate from its offset, first two components. Put the
   decision on `VertexDecl` (next to `own_diffuse_texcoord`), with a unit test on a declaration
   shaped like arena's (offsets 20, 24, 28, 32). Do not change a title that names `Uv1`.
2. Decide the markings (`uvMarkings`, offset 20) and dirt (`uv1_uvDirt.zw`, offset 32) sets: the
   pixel programs sample `MarkingsSampler` and `DirtSampler` with them. Wiring them draws the
   road decals and dirt splat; the road is correct without them, so this is a second change.
3. Ground truth: extend `crates/mesh/tests/omega_transparency_ground_truth.rs` with a check that
   no draw of a `tarmac_*` material reads a coordinate range outside `uv1_uvDirt`'s (arena: u -44 to
   41, v 0 to 1). Re-render the compare pairs of the eight circuits afterwards
   (`just compare-rerender-ours --only omega`, on the lead's tree).
4. `OAG_ALBEDO_ONLY` and `OAG_TINT_MATERIALS` do not reach the psp2 build (`rcs/psp2.rs` never
   reads `isolate::unlit()`/`tinting()`); `OAG_SKIP_MATERIAL`, `OAG_ONLY_MATERIAL`, the slot
   filters and `OAG_ALPHA_EMISSIVE_OFF` do. One line in the `None => ([1.0; 4], 1.0, role)` arm of
   `psp2.rs` makes the unlit switch reach it.

## Open

- **Unity Square** is not black. Its road, `road_alphaspec_normal_emissive_panelmap1`, draws the
  diffuse panels and the same baked sun wedge the original has; it lacks the original's glowing
  cyan panel-edge lines (also on Queens Mall) and the panel displacement
  (`Cg_PanelDisplacement_1In1Depth/SlopeWidth/Aspect`, uniforms `185b0018` 4.0, `95de1208` 3.5,
  `4e231706` 1.8 on the lit variant). The lines are probably the family's `EmissiveMap`
  (`track_a_floor_e`), not admitted by `emissive_add.rs` for it (confidence 55, not isolated).
- The black wedge and the road's lightmap on arena are fine once the coordinates are right: no
  lighting change is needed for the patch.
- 2048 cross-check: **checked, differs** (no `tarmac_*` material in 2048's arena, square, altima or
  mall; its roads name `Uv1`).
