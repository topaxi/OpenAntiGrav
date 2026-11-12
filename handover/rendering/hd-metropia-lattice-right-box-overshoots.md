---
status: re-first
titles: [hd]
updated: 2026-11-12
summary: "Metropia's lattice wires reflect the sky now; the right floor box overshoots the original (86.8 against 72.4)"
size: S (30)
difficulty: medium (40)
---

# HD Metropia: the lattice's right floor box overshoots the original

The wires are `mesh_colour_constant` block #7 wired as `window_glass` `Mode::Gated` (sky reflection through the
`Normal` map's alpha, confidence 88; `hd_sky_reflection_ground_truth::metropias_lattice_reflects_the_sky_through_its_normal_maps_alpha`).
Evidence and numbers: `docs/formats/rcsmaterial.md`, "Metropia: the lattice floor".

## Open

- Box x .70-.88, y .78-.96 on the pair `data/reference/hd-capture/metropia-lattice/` reads mean grey 86.8 against the
  original's 72.4 (73.0 before the reflection). Likely cause: ours reflects about the unperturbed vertex normal, where
  the program reflects about the normal-mapped `N`, so the wires there sample a brighter part of the sky. The original's
  craft and HUD also cover part of that box, so the content is not fully matched.
- `ours-after.png` in the pair is still the frame before the reflection; re-render it from main.

## Next Steps

1. Re-render the pair from main and mask the original's craft and speed readout out of the box before comparing.
2. If the gap holds, test the reflection angle: the tangent frame for track vertices (shared with `ds_booth_glass` and
   the window families, `mesh::rcs::window_glass`'s "Left out, named").
