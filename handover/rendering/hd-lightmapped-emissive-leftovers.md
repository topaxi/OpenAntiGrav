---
status: ready-to-wire
titles: [hd, omega, 2048]
updated: 2026-11-27
summary: "Modesto's glow table is full; lightmapped emissives with other laws, and Omega's and 2048's copies, unwired"
difficulty: medium (30)
---

# What the lightmapped emissive census left dark

Laws, census and the full list:
[`docs/rendering/hd-zone-recolour.md`](../../docs/rendering/hd-zone-recolour.md), "The other
lightmapped emissive families". Code: `oag_mesh::mesh::rcs::lightmapped_emissive`.

## Open

- **Modesto Heights' glow table is full** (63 entries, the six index bits of the slot word, four
  kept for the readers after material setup): every lightmapped `and_diffuse_emissive*` and
  `uvanim_diffuse_emissive` slot stays dark. Needs a wider index or fewer per-material entries.
- **Other laws, read and not wired**: `lambert_spec_mult_emissive_scroll`,
  `uvanim_diffuse_emissive_bloom`, `dc_flashing_lights`, the two `u`-scrolled
  `mt_uvanim_diffuse_emissive3tokey*` compiles.
- **`mr_shopwin_reflecttranspemis` in Zone**: its Zone variants sample the emissive; unread.
- **`mageffect08_lh`**: mask `0x1202d8df` with a normal map third; unread.
- **Omega and 2048** ship several of these family names; their `gcn` and `psp2` programs are unread.
- **No frame of the original** shows these lights in a matched pose.

## Next Steps

1. Free or widen the glow-table index, then add Modesto's slots to the census test's expectation.
2. Capture a PS3 frame of an Amphiseum floor or a Talon's support strut and pair it.
