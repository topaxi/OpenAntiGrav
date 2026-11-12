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
- Amphiseum's `...plusoverlay` (slots 355/445 forward, 357/447 reversed) binds the same way (same block law) and draws a cyan
  screen without the Feisar logo the generic reading drew in the tower windows (`--pose=-524,46,-379`); compare with the original
  there.
- Omega (`data02.psarc`, GNM) and 2048 (`dlc2.psarc`, the `...bloom` copy, GXP): same inputs declared, programs unread, not wired.

## Next Steps

1. Capture the original at Amphiseum's tower windows (`place --pose=-524,46,-379`) and compare with ours: do the Feisar logos
   show?
2. Read the three units' gamma nibbles live on Modesto reversed and replace the "chosen" decode in `shade.wesl`'s overlay block.
3. Omega/2048: disassemble one `plusoverlay` program per title and wire it through the psp2 builder if the law matches.
