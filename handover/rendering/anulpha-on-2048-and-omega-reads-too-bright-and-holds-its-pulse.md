---
status: measure
titles: [2048, omega]
updated: 2026-11-14
summary: "Anulpha on 2048/Omega: colour law, chevron mask and centre lane wired; 2048 strips sunlit too bright, pulse Colour and offsets held"
difficulty: medium (40)
---

# Anulpha Pass on 2048 and Omega reads too bright and holds its pulse

2026-11-14, lane `anulpha-tone`. Wired on both titles, each read on its own programs:
the walls', plates' and chevrons' albedo-alpha law (`slots::LIT_ALPHA`), the chevrons'
emissive mask, and the centre lanes' `E.a * wave` with Omega's `Uvset1` coordinate.
Programs, addresses and before/after regions: `docs/rendering/hd-track-tone.md`,
"2048 and Omega". Omega Anulpha's pair went 30.28 to 25.03 mean grey.

## Open

1. **2048's floor-chevron strips are lit by the full sun.** Region `140x18+30+282` of
   `2048-vita/anulpha-pass`: chromaticity matched (dB/G -0.002) but G 140 against 59. Not the
   specular, not the baked colour: the strips' atlas region (`nova_mesh_combine_track_12-lmap.gxt`,
   `lmuv` 0 to 0.62 by 0.67 to 0.75) is dark in RGB and mostly sunlit in alpha (169 of 255).
   Next: compare our sun term (`liveLighting0diffuse`, direction, `N.L` on a Vita submesh with no
   normal map) at that surface against the original; 2048 #16 words 22 to 27 are the light.
2. **2048's alpha-lit surfaces read 2 to 3 times too bright, raised by the flag itself**
   (road plate G 61 before, 98 after, original 57; whole-frame mean grey 15.13 to 17.67). Ruled
   out: the specular (zeroed, frame identical), a different alpha cook (2048's alpha bytes average
   the same as HD's). Lighting `alpha * alpha` would match (road 54, mean grey 14.33) but no
   instruction read squares it. Next: re-read #16 around word 24 for a square of `pa10.w` (an
   earlier `mul` on the whole texel, or a sampler swizzle/format that decodes alpha). The `.gxt`
   file carries no gamma mode (`oag_texture::gxt` reads none): on the Vita it is set at runtime by
   `sceGxmTextureSetGammaMode`, so look for that import's callers in 2048's executable. Omega's right wall
   is also brighter (86 against 59), likely the light or exposure there.
3. **The chevron's `Colour` is held at zero** (chosen): the authored value paints green the
   native-port original never shows; the live value is unread (HD's came off a hooked draw).
   Omega's executable in Ghidra is where to find who writes the `Colour` constant.
4. **Both families' `offset` is held** (`AnimTrack::Held`): the curve is unread on both titles.
5. Omega's lane submeshes declare a 4-float `tangent` (`psp2_material_dump` reports `tangents 0`),
   so their normal map is not applied.
6. 2048 permutations #33, #35, #59 and #61 of the alpha-lit files lack the `light * T.www`
   multiply; identify them before claiming every permutation.

## Next Steps

1. Item 1: read our sun's magnitude at the strip (`OAG_ALBEDO_ONLY` against a probe of the
   lit term) and the original's at the same pose.
2. Item 3: find the `Colour` constant's writer in Omega's `eboot.bin`.
