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
  kept for the readers after material setup). Dark, over both layouts: 16 `and_diffuse_emissive`,
  4 `and_diffuse_emissive_spec` and 20 `uvanim_diffuse_emissive` slots.
- **Vineta K, 4 slots (2 per layout) of `mt_uvanim_diffuse_emissive3tokey`**: this compile declares
  `0x1c5de6ec` and no `time`; its law is unread and the family is not wired.
  The census test pins both sets by circuit, family and count.
- **Other laws, read and not wired**: `lambert_spec_mult_emissive_scroll`,
  `uvanim_diffuse_emissive_bloom`, `dc_flashing_lights`, the `u`-scrolled
  `mt_uvanim_diffuse_emissive3tokey_v_scroll`.
- **`mr_shopwin_reflecttranspemis` in Zone**: its Zone variants sample the emissive; unread.
- **`mageffect08_lh`**: mask `0x1202d8df` with a normal map third; unread.
- **Omega and 2048** ship several of these family names; neither their programs nor what `gcn` and
  `psp2` bind for them today is read.
- **No frame of the original** shows these lights in a matched pose.

## Next Steps

1. Widen the glow-table index to 7 bits, then move Modesto's slots from the census test's dark list
   to its bound counts. Size, all in `oag-mesh` bar the last item (medium, about half a day):
   - `GpuVertex::slots` (`mesh/vertex.rs`) has no free bit: 0-23 roles, 24-29
     `MATERIAL_SHIFT`/`MATERIAL_MASK`, 30 `ZONE_PLAIN`, 31 `LIT_ALPHA`. The seventh bit needs a
     second vertex word, or spare high bits of `anim`/`xform`, carried flat to the fragment stage.
   - The index is decoded inline as `(in.slots >> 24u) & 63u` at 24 places in `shade.wesl`,
     `mesh.wesl` and `holo.wesl`; `types.wesl`'s `Emissives` arrays and `EMISSIVE_LIMIT` go to 128.
   - 54 places write `<< MATERIAL_SHIFT`; 47 source files and 60 test files in the workspace name
     the shift, the mask's reader or the limit (`oag-render`'s `gantry.rs` and `oag-raceplay`'s
     `pad_glow.rs` are the users outside `oag-mesh`).
   - The cheaper alternative is to reclaim entries from Modesto's 53 plain-glow ones (2-3 h);
     whether enough can be merged is not measured.
2. Capture a PS3 frame of an Amphiseum floor or a Talon's support strut and pair it.
