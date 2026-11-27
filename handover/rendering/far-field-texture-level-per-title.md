---
status: ready-to-wire
titles: [pure, pulse, hd, 2048, omega]
updated: 2026-11-27
summary: "Far-field texture level: 2048 -1.5 bias, PS2 single level, TEXTURE DETAIL on hardware path ready to wire; Pure follows Pulse"
---

# Far-field texture level, title by title

Lane `farfield-lod-titles`, 2026-11-27. The laws, evidence and confidences are in
[texture-level-per-title.md](../../docs/rendering/texture-level-per-title.md). Wire nothing in the
shared `oag-mesh` level path until lane `pulse-farfield-lod` has merged; each item below then wires
read values only, as `Title` data, with a test, a break-it, and the other titles' pairs byte-identical.

## Open

- **2048: `material_lod_bias` -1.5, ready to wire.** `GxtLoader_LoadTexture` (`0x812f0ec4`) gives every
  `.gxt` `sceGxmTextureSetLodBias(0x13)` = -1.5, trilinear, no anisotropy (confidence 80;
  `docs/ghidra/functions/vita-2048-eu-v104/gxt-loader.md`). Wiring: one `material_lod_bias:
  Rule { on: Any, origin: Measured }` in `crates/2048/src/lib.rs`'s `Looks`, and a test like
  `oag-hd`'s `the_material_lod_bias_follows_its_rule`. Then re-render the three 2048 pairs. Omega must
  stay at bias 0 (its S# read 0.0).
- **Pulse PS2: one level, ready to wire.** The GS samples level 0 (`MXL` 0, no level on any of 7,020
  blobs; confidence 75). Ours uploads a box chain (`crates/mesh/src/mesh/ps2_textures.rs`,
  `ModelTexture::rgba8(.., None)`). Wiring: `Some(1)` there, the PS2 path only, with a test that a
  distant PS2 texel reads the base level. PSP pairs must stay byte-identical. Not "PS2 inherits PSP":
  this is a PS2-measured value.
- **TEXTURE DETAIL on the hardware path, ready to wire.** `scene.fog.texlod_shift` is read only on the
  PSP slope path, so `high`/`maximum` do nothing on PS2, HD, 2048 and Omega, though the row is shown
  there. Wiring: add the shift to `material_sample`'s bias, to `material_sample_grad`'s `exp2` scale
  and to the plain material `textureSample` reads. `original` must stay byte-identical on every
  title.
- **Pure: the per-texture bias and mode-0 textures follow Pulse's wiring.** Same functions
  (`docs/ghidra/functions/psp-pure-usa/texture-level.md`), K stored at texture `+0x2a0` with the
  0.2 rebuild. A Pure pair should move with Pulse's change. Check that
  `pulse-farfield-lod`'s Title data covers Pure.
- **Not read:**
  - which camera built the Modesto textures' stored K (-9.05, against the race camera's -8.32);
  - Omega's two +1.0-bias samplers' materials;
  - a live 2048 race-material descriptor (the loader's value is confirmed on the front-end canvas
    only);
  - an in-race PS2 GS dump. The race save states need a PCSX2 v2.9.168 re-save: walk to Track Select
    with `scripts/pcsx2-pulse-walk.py`.

## Next Steps

1. After `pulse-farfield-lod` merges: 2048's rule (about 30 minutes with the pairs), PS2's single level
   (about 45 minutes), then TEXTURE DETAIL on the hardware path (about an hour, a shader change with a
   level test per preset).
