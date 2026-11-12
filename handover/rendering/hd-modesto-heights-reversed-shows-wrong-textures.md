---
status: measure
titles: [hd, omega, 2048]
updated: 2026-11-12
summary: "Modesto banner wired (0.26 % over grey 200, original 0.1-0.3 %); open: units' gamma nibbles, Omega/2048 wiring, Amphiseum copy unseen."
size: S (25)
difficulty: medium (40)
---

# HD Modesto Heights: the banner band

Landed 2026-11-12 (`hd-modesto-banner`): `mesh::rcs::uv_overlay` binds the grille as the albedo and adds the logo overlay
after the light. Evidence, read values, before and after numbers and the cross-title outcomes are on
`docs/formats/rcsmaterial.md`, "Two surfaces the maintainer saw wrong" (Modesto part). Tests:
`hd_uv_overlay_ground_truth`, `hd_uv_overlay_lit_path`.

## Open

- The gamma nibbles of the three units (clouds, grille, logo): decoded like every colour tap on this path, **chosen, not
  measured**; reading them raw lifts the band's mean grey by about 2. A live `SET_TEXTURE_ADDRESS` read on this draw decides it
  (RPCS3, `rpcs3-capture.md`).
- The bloom copy's output alpha (`@0x26 MOV H0.w, H0`, the logo's alpha) is not wired.
- Amphiseum's `...plusoverlay` (slots 355/445 forward, 357/447 reversed) binds the same way but no frame showed it: find a spot
  where it is visible and compare with the original.
- Omega (`data02.psarc`, GNM) and 2048 (`dlc2.psarc`, the `...bloom` copy, GXP): same inputs declared, programs unread, not wired.

## Next Steps

1. Pick an Amphiseum spot that shows slot 355 or 445 (the draw bounds are in node space; a pair from RPCS3 gives the pose) and
   re-render against it.
2. Read the three units' gamma nibbles live on Modesto reversed and replace the "chosen" decode in `shade.wesl`'s overlay block.
3. Omega/2048: disassemble one `plusoverlay` program per title and wire it through the psp2 builder if the law matches.
