---
status: measure
titles: [omega]
updated: 2026-11-12
summary: "Omega tarmac markings on, dirt off by default (OAG_TARMAC_DIRT=1): read the .gnf sRGB decode before the lerp"
---

# Omega's `tarmac_*` roads (diffuse fixed; dirt, markings and emissive wired, not yet matching)

Evidence and numbers: [`omega-status.md`](../../docs/formats/omega-status.md), "The `tarmac_*` road
families draw the wrong coordinate set" and "The `tarmac_*` pixel programs combine four pictures".

## Next Steps

0. State: markings and emissive on; the dirt layer is off by default (`OAG_TARMAC_DIRT=1` draws it). Turn it on by default once step 1 explains the darkness.

1. Isolated: dirt alone causes the darkening, markings change nothing in the boxes, the linear domain closes about a third of it, the program squares nothing. Still open: why `arena-2048` reads darker than the original after the layers (right box 131,138,128
   theirs, 117,125,118 ours; ahead 124,132,123 against 83,114,107). Candidates: the dirt's weight
   (mixed on gamma-domain taps; try the linear domain), the diffuse being brighter in the original,
   the lighting term. Isolate by drawing the dirt alone then the markings alone (`OAG_ONLY_MATERIAL`
   plus a temporary mode switch), and find a frame where a painted symbol shows.
2. Re-render the compare pairs of the eight circuits (`just compare-rerender-ours --only omega`, on
   the lead's tree).
3. `OAG_ALBEDO_ONLY` and `OAG_TINT_MATERIALS` do not reach the psp2 build (`rcs/psp2.rs` never reads
   `isolate::unlit()`/`tinting()`); one line in the `None => ([1.0; 4], 1.0, role)` arm makes the
   unlit switch reach it, which step 1 wants.
4. `tarmac_dirt` and `tarmac_dirt_emissive` (samples read, combine not): read their programs, then
   admit them in `psp2/tarmac.rs`. `MarkingsSpec` (specular weight) and `MarkingsNormal` are unread.

## Open

- **Unity Square** is not black. Its road, `road_alphaspec_normal_emissive_panelmap1`, draws the
  diffuse panels and the same baked sun wedge the original has; it lacks the original's glowing
  cyan panel-edge lines (also on Queens Mall) and the panel displacement
  (`Cg_PanelDisplacement_1In1Depth/SlopeWidth/Aspect`). The lines are probably the family's
  `EmissiveMap` (`track_a_floor_e`); which path would draw it is unread (confidence 55).
- 2048 cross-check: **checked, differs** (no `tarmac_*` material in 2048's arena, square, altima or
  mall; its roads name `Uv1`); the Vita census and pins are unchanged.
