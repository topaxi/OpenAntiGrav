---
status: measure
titles: [pure, pulse, hd, 2048, omega]
updated: 2026-11-27
summary: "Far-field texture level: 2048 bias to measure, Pure K to read, TEXTURE DETAIL row still shown on PS2"
---

# Far-field texture level, title by title

Lane `farfield-lod-titles`, 2026-11-27. The laws, evidence and confidences are in
[texture-level-per-title.md](../../docs/rendering/texture-level-per-title.md). Each item below wires
read values only, as `Title` data, with a test, a break-it, and the other titles' pairs byte-identical.

## Open

- **2048: `material_lod_bias` -1.5, measure first, do not wire below 90** (maintainer: the evidence is the loader plus one front-end texture; race materials need an in-race read, as for PS2). `GxtLoader_LoadTexture` (`0x812f0ec4`) gives every
  texture it builds `sceGxmTextureSetLodBias(0x13)` = -1.5, trilinear, no anisotropy (confidence 85,
  `docs/ghidra/functions/vita-2048-eu-v104/gxt-loader.md`). That race materials keep those bits is
  inferred (60). The Zone and Detonator track skins have their mip filter switched off after load.
  First read one live race-material `SceGxmTexture` in Vita3K, about 45 minutes. Then wiring: one `material_lod_bias:
  Rule { on: Any, origin: Measured }` in `crates/2048/src/lib.rs`'s `Looks`, and a test like
  `oag-hd`'s `the_material_lod_bias_follows_its_rule`. Then re-render the three 2048 pairs. Omega must
  stay at bias 0 (its S# read 0.0).
- **TEXTURE DETAIL is a row with no choice on Pulse PS2** (one level per texture, wired). Hiding it
  needs a per-platform menu condition, which `oag_ui`'s `drop_unavailable_*` methods do not have (they
  take a `Title`, and PSP and PS2 are one). About an hour.
- **Pure: the per-texture bias and mode-0 textures follow Pulse's wiring, with one caveat.** Same
  functions (`docs/ghidra/functions/psp-pure-usa/texture-level.md`), K stored at texture `+0x2a0`
  with the 0.2 rebuild. The Modesto bytes fit K about -9.05, not the drawn camera's -8.32. A Pure
  wiring on current-camera K would be 11 to 12 sixteenths coarse on 72 of 88 binds. Read where Pure's
  K comes from (`Camera_StoreTextureLodTerm`'s three callers) before wiring Pure.
- **Not read:**
  - which camera built the Modesto textures' stored K (-9.05, against the race camera's -8.32);
  - Omega's two +1.0-bias samplers' materials;
  - a live 2048 race-material descriptor (the loader's value is confirmed on the front-end canvas
    only).

## Next Steps

1. Measure: 2048's live race-material descriptors in a race (about 45 minutes in Vita3K); wire the `Rule` line only at 90+.
2. Read: Pure's K source (about 30 minutes in Ghidra).
