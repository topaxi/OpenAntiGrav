---
status: ready-to-wire
titles: [pure, pulse, hd, 2048, omega]
updated: 2026-11-27
summary: "Far-field texture level: TEXTURE DETAIL on hardware path ready to wire; PS2 single level (75, wire at 90), 2048 and Pure K to measure"
---

# Far-field texture level, title by title

Lane `farfield-lod-titles`, 2026-11-27. The laws, evidence and confidences are in
[texture-level-per-title.md](../../docs/rendering/texture-level-per-title.md). Wire nothing in the
shared `oag-mesh` level path until lane `pulse-farfield-lod` has merged; each item below then wires
read values only, as `Title` data, with a test, a break-it, and the other titles' pairs byte-identical.

## Open

- **2048: `material_lod_bias` -1.5, measure first.** `GxtLoader_LoadTexture` (`0x812f0ec4`) gives every
  texture it builds `sceGxmTextureSetLodBias(0x13)` = -1.5, trilinear, no anisotropy (confidence 85,
  `docs/ghidra/functions/vita-2048-eu-v104/gxt-loader.md`). That race materials keep those bits is
  inferred (60). The Zone and Detonator track skins have their mip filter switched off after load.
  First read one live race-material `SceGxmTexture` in Vita3K, about 45 minutes. Then wiring: one `material_lod_bias:
  Rule { on: Any, origin: Measured }` in `crates/2048/src/lib.rs`'s `Looks`, and a test like
  `oag-hd`'s `the_material_lod_bias_follows_its_rule`. Then re-render the three 2048 pairs. Omega must
  stay at bias 0 (its S# read 0.0).
- **Pulse PS2: one level, measure first, do not wire below 90** (maintainer: Pulse PSP was once
  read as base-level-only and that was false). The GS samples level 0 (`MXL` 0, no level on any of 7,020
  blobs; confidence 75), but every dump is a front-end frame, and blobs without levels do not rule out
  levels built at load or upload time. To reach 90: an in-race GS dump (near and far surfaces) reading
  `TEX1` `MXL`/`LCM`/`K` and `MIPTBP1`/`MIPTBP2` per draw, plus a read of the PS2 texture upload path
  for any runtime level build. Ours uploads a box chain (`crates/mesh/src/mesh/ps2_textures.rs`,
  `ModelTexture::rgba8(.., None)`). Wiring: `Some(1)` there, the PS2 path only, with a test that a
  distant PS2 texel reads the base level. PSP pairs must stay byte-identical. Not "PS2 inherits PSP":
  this is a PS2-measured value.
- **TEXTURE DETAIL on the hardware path, ready to wire.** `scene.fog.texlod_shift` is read only on the
  PSP slope path, so `high`/`maximum` do nothing on PS2, HD, 2048 and Omega, though the row is shown
  there. Wiring: add the shift to `material_sample`'s bias, to `material_sample_grad`'s `exp2` scale
  and to the plain material `textureSample` reads. `original` must stay byte-identical on every
  title.
- **Pure: the per-texture bias and mode-0 textures follow Pulse's wiring, with one caveat.** Same
  functions (`docs/ghidra/functions/psp-pure-usa/texture-level.md`), K stored at texture `+0x2a0`
  with the 0.2 rebuild. The Modesto bytes fit K about -9.05, not the drawn camera's -8.32. A Pure
  wiring on current-camera K would be 11 to 12 sixteenths coarse on 72 of 88 binds. Read where Pure's
  K comes from (`Camera_StoreTextureLodTerm`'s three callers) before wiring Pure.
- **Not read:**
  - which camera built the Modesto textures' stored K (-9.05, against the race camera's -8.32);
  - Omega's two +1.0-bias samplers' materials;
  - a live 2048 race-material descriptor (the loader's value is confirmed on the front-end canvas
    only);
  - an in-race PS2 GS dump. The race save states need a PCSX2 v2.9.168 re-save: walk to Track Select
    with `scripts/pcsx2-pulse-walk.py`.

## Next Steps

1. After `pulse-farfield-lod` merges: the PS2 in-race measure (above; wire only at 90+), then TEXTURE DETAIL on the
   hardware path (about an hour, a shader change with a level test per preset).
2. Measure: 2048's live race-material descriptor (about 45 minutes in Vita3K), then its `Rule` line.
3. Read: Pure's K source (about 30 minutes in Ghidra).
