---
status: re-first
titles: [hd]
updated: 2026-11-12
summary: "Metropia's mesh floor now draws as a lattice; its wires are darker than the original's grey (lit specular H4 unread)"
size: S (30)
difficulty: medium (35)
---

# HD Metropia: the lattice wires are darker than the original's

The lattice itself is wired (`mesh_colour_constant` in `skin::ALPHA_DIFFUSE` as `AlphaUse::Gate` on
sampler `Normal`; `hd_lightmapped_albedo_ground_truth::metropias_lattice_floor_keeps_its_maps_alpha_over_its_constant`).
Evidence and numbers: `docs/formats/rcsmaterial.md`, "Two surfaces the maintainer saw wrong".

## Open

- The wires read darker than the original's grey: left box 94 against 100, 13 % under grey 12 against 7 %.
  The constant is black, so ours lights them by reflection and specular alone. The lit specular term
  (`H4`) of the fragment program, and whether it reads the lightmap through the map's alpha, is unread.
- Omega and 2048 ship the family through `mesh::rcs::psp2`, which has no reading for it (checked, applies,
  not wired; same doc section).

## Next Steps

1. Read `H4` in the program (same `fp` block as the `MOV H0.w, R0` at `0x3e`): which inputs light the wire,
   and does the lightmap enter through the alpha.
2. Wire it and re-measure on the pair `data/reference/hd-capture/metropia-lattice/` (`theirs.png`,
   `ours-after.png`), same boxes as the doc.
