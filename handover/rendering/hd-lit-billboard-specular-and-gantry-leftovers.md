---
categories: [rendering]
---

# HD: lit materials with no `pow` chain still draw the stand-in specular; the card's remaining leftovers

2026-10-18, `gantry-hd-alpha`. The gantry blur lane is closed (card sampler wraps now,
`docs/rendering/start-gantry.md`, "the card sampler wraps"). What it left:

## Open

- **No-chain programs draw a specular the original does not.** `light_inputs::specular_exponent`
  returns `NO_SPECULAR` only for a `colour_factor` (hull) material; every other HD material
  whose program has no `LG2` (so no resolvable exponent) falls back to `32.0`, and the shader's
  `no_specular` is `specular_exponent <= 0 && colour_factor`. `billboarddiffuse` and
  `cf_vex_billboard` are two such materials. Dropping `colour_factor` from both places moved
  `10_sebenco_climb` by 2,197 px beyond the board (and `02_track` 264, `talons_junction` 7), so
  it is a title-wide HD picture change. Needs: a census of which HD materials resolve no chain
  and whether their drawn blocks (not the whole file) carry an `LG2`, then a pose-matched frame.
- **`tv_glow.gtf` on `02_track`.** `cf_vex_billboard` binds it as a second texture and it does
  not reach the shader (no `ADD_SECOND` bit, slots word `0x18`).
- **Omega.** `adverts: None`, so no card route; the shared lineage suggests the same quads.
  Not checkable on a sampler (no PS4 capture path).

## Next Steps

1. `rg -c LG2` per drawn block over the HD materials that report `specular_exponent_unresolved`.
2. If the drawn blocks are chain-free for a whole family, gate `NO_SPECULAR` on that family
   and regenerate any HD golden in its own commit.
