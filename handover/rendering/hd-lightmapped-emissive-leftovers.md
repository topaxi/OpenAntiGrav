---
status: ready-to-wire
titles: [hd, omega, 2048]
updated: 2026-11-27
summary: "Lightmapped emissives with other laws, and Omega's and 2048's copies, unwired; no original frame matched"
difficulty: medium (25)
---

# What the lightmapped emissive census left dark

Laws, census and the full list:
[`docs/rendering/hd-zone-recolour.md`](../../docs/rendering/hd-zone-recolour.md), "The other
lightmapped emissive families". Code: `oag_mesh::mesh::rcs::lightmapped_emissive`.

## Open

- **`mt_uvanim_diffuse_emissive3tokey_v_scroll`** (Modesto Heights, Sol 2) declares the keyed
  family's `0x1c5de6ec`; compare its program with the keyed one (`PROGRAM_LIGHTMAPPED_KEYED`) and
  list it if they match.
- **`chevron_pulse` shares one curve row per material name** (`rows` in `chevron_pulse.rs`); the
  keyed emissive family had slots of one name with different curves. Whether a circuit's chevron
  slots do is unchecked.
- **Other laws, read and not wired**: `lambert_spec_mult_emissive_scroll`,
  `uvanim_diffuse_emissive_bloom`, `dc_flashing_lights`.
- **`mr_shopwin_reflecttranspemis` in Zone**: its Zone variants sample the emissive; unread.
- **`mageffect08_lh`**: mask `0x1202d8df` with a normal map third; unread.
- **Omega and 2048** ship several of these family names; neither their programs nor what `gcn` and
  `psp2` bind for them today is read.
- **No frame of the original** shows these lights in a matched pose.

## Next Steps

1. Capture a PS3 frame of an Amphiseum floor, a Talon's support strut or a Modesto saucer tower and
   pair it: nothing here is checked against the original.
2. Compare the `_v_scroll` program with the keyed block and wire it if equal (small).
