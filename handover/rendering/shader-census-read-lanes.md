---
status: ready
titles: [hd, omega, 2048]
updated: 2026-11-27
summary: "Read lanes ranked by the shader census: 2048's fc07 emissive maps and HD's base_diffusespecular specular map are declared by programs no matcher claims"
categories: [rendering]
---

# Read lanes from the shader census

The census and its ranked tables are `docs/rendering/shader-census.md`. A lane reads a program,
wires a matcher, and moves the baseline under `crates/mesh/tests/shader_census/` with the page.

## Open

- **`fc07_lambert_emmisive` and its `startanim_` twin** (2048 `ddf9428625f9109e` 3,651,
  `00b742c321dbfa6d` 1,727, `e6d1f521b74be851` 1,200 on DLC; Omega `32e7a1b05c5cde23` 1,559,
  `a16ea690c930d19d` 906, `f3264151ac78c906` 544). The program declares `EmissiveMap` and
  `Tint`; `psp2::glow::layer_of` takes only materials authoring an offset and scale, and no
  other planner claims them, so the emissive is very likely not drawn (inferred from the code,
  not seen in a frame; confidence 75). Check a frame first.
- **HD `base_diffusespecular`** (`48929cc84561b734` 1,846 and `c6f387aa871dd7c6` 1,620; Modesto
  Heights and Amphiseum): `SpecularColour` and `SpecularTexture` are declared and the generic
  reading binds no specular map.
- **HD `defuse_occulsion_vert_col_tint`** (666 chunks, three circuits) ships no row for the
  lit-race key: unread which row the original draws.

## Next Steps

1. 2048 emissive: render a frame with a `fc07_lambert_emmisive` sign, read the fragment program
   (`scripts/vita-gxp.py`) and Omega's (`gcn`), then wire both.
2. HD specular map: `scripts/ps3-microcode.py` on
   `DATA00:data/environments/amphiseum/materials/base_diffusespecular.rcsmaterial` at `0x2b60`.
