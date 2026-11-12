---
status: ready-to-wire
titles: [hd]
updated: 2026-11-12
summary: "Metropia's wire-mesh floor (mesh_colour_constant, slot 510) draws as an opaque black sheet; the original shows a lit lattice over the city below."
size: XS (20)
difficulty: easy (30)
---

# HD Metropia: the lattice floor draws as a black sheet

Maintainer, from play: Metropia (forward) tick 900 at `[-311.2, 101.1, -13.8]` "floor texture is also wrong". Measured
2026-11-12 on a pair (two RPCS3 boots), written up in `docs/formats/rcsmaterial.md`, "Two surfaces the maintainer saw wrong".

## Open

- Surface: slot 510, `mesh_colour_constant.rcsmaterial`, `dds2/ds_mesh_n.gtf` (normal map, alpha = the wire mask),
  lightmap `ile_mesh_combine6-lmap.gtf`, blended. Lower 65 % of the frame under grey 12: original 2.8 %, ours 41.2 %
  (3.9 to 5.2 % with the slot skipped). Cause, confidence 75: the program writes the map's alpha as output alpha (`MOV H0.w,
  R0`, `@0x3e`) and gates the reflection by it; the family is bound as a black constant with no alpha.

## Next Steps

1. Add `mesh_colour_constant` to `skin::ALPHA_DIFFUSE` as `AlphaUse::Gate` (constant `0x02ab9f07`), so the picture keeps the
   map's alpha, and check the blend reads it as coverage.
2. Re-render the pair at `--pose=-354.28,110.69,-53.17` with the camera in the doc and compare the two floor boxes
   (original 102.6 to 103.5 and 81.3 to 82.4 mean grey; ours 1.5 and 17.9 before).
3. The family exists only in `02_track` on HD (census of the seven archives); Omega (`Mesh_Colour_Constant{,_1,_2}`) and 2048 (`dlc1.psarc`) ship it too: checked, applies, not wired.
