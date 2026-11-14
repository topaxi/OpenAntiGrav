---
status: ready-to-wire
titles: [2048, omega]
updated: 2026-11-14
summary: "Anulpha Pass on 2048/Omega: albedo-alpha law wired; chevron emissive read not drawn, 2048 chevron strips too bright"
difficulty: medium (40)
---

# Anulpha Pass's chevron emissive and the 2048 strip light

2026-11-14, lane `anulpha-tone`. Both titles' Anulpha walls, plates and chevrons now light
the albedo's alpha (`slots::LIT_ALPHA`, `oag_mesh::mesh::rcs::psp2`'s `lit_alpha`); the blue
cast and Omega's cyan bars and purple stripes are gone. Programs, addresses and before/after
regions: `docs/rendering/hd-track-tone.md`, "2048 and Omega".

## Open

1. **The chevron emissive, read and not drawn.** Both titles add, unlit, after the light:
   `max(E.rgb, Colour * mix(E.rgb, W.rgb, E.a))` (`E` the `Emiss_Mask` sampler, squared on
   2048; `W` the `Wave` sampler at `uv * UVScale + offset`). 2048 `cf_chevron_pulse` #3 words 12
   to 28; Omega `cf_chevron_pulse_displacement2out` `data01` `@0x105c4` (`@0xcfc` wave,
   `@0xd40` to `@0xdd0` mix and max), `data08` `@0x10fc4`. Not HD's emissive
   (`mesh::rcs::chevron_pulse`), so do not reuse its glow mode as it stands. Without it Omega's
   chevron lanes read dark navy (`srgb` 35, 49, 60) where the native-port original glows cyan
   (34, 91, 107; box `200x70+100+650` of `omega/anulpha-pass`). The `offset` uniform animates:
   its curve is unread on both titles (HD runs it on the scenery clock, chosen). A static
   offset at the authored value (Omega 0.0139 forward, 1.0 reversed) may be a fair first draw
   if `E.rgb` dominates; measure against the pair's seven frames (`frame00`..`frame06`).
2. **2048's floor-chevron strips are too bright.** After the law, region `140x18+30+282` of
   `2048-vita/anulpha-pass` has the original's chromaticity (dB/G -0.003) but G 139 against 59.
   Not the specular (zeroed: frame identical). The light term is the same shape as the plain
   floor's (`#16` words 24 to 29) and binds the same atlas, over its own `lmuv` region (0 to 0.62
   by 0.67 to 0.75). Next: sample the atlas at those UVs, and check which coordinate set the
   chevron submeshes' declaration gives the loader as `lightmapUV`
   (`DECL_SUB=1005 cargo run -p oag-rcs --example psp2_material_dump -- ...`).
3. **The alpha-lit surfaces read brighter than the originals** (2048 road plate G 95 against
   57; Omega right wall 85 against 59). Both programs light the raw alpha, so the excess is in
   the light or the exposure.
4. Unidentified 2048 permutations #33, #35, #59, #61 of the three files lack the `light *
   T.www` multiply; identify them from their parameter lists before claiming every permutation.

## Next Steps

1. Item 1 on Omega first (the native-port pair is the reference), then 2048 with the squared
   mask; one commit per title, each against its pair's chevron-lane box.
2. Item 2: the atlas sample at the strip's UVs against the plain floor's.
