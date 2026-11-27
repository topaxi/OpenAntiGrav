---
status: ready-to-wire
titles: [pure, pulse, hd, 2048, omega]
updated: 2026-11-27
summary: "Far-field texture level: PS2 single level, TEXTURE DETAIL, 2048 bias and the PSP stored K per texture (the flyby's -9.0506) ready to wire"
---

# Far-field texture level, title by title

Lane `farfield-lod-titles`, 2026-11-27. The laws, evidence and confidences are in
[texture-level-per-title.md](../../docs/rendering/texture-level-per-title.md). Wire nothing in the
shared `oag-mesh` level path until lane `pulse-farfield-lod` has merged; each item below then wires
read values only, as `Title` data, with a test, a break-it, and the other titles' pairs byte-identical.

## Open

- **2048: `material_lod_bias` -1.5, ready to wire** (confidence 90, at the maintainer's gate of 90). In a
  Queens Mall race all 269 distinct disc textures bound over two frames carry the loader's state (bias -1.5,
  trilinear, no anisotropy), none differs (`docs/ghidra/functions/vita-2048-eu-v104/gxt-loader.md`, "In a
  race"). Bound state is from one boot; a second boot agrees on resident descriptors only. Not measured live:
  Zone and Detonator track skins (static read: mip filter off), HD/Fury circuits, a second circuit.
  Wiring: one `material_lod_bias: Rule { on: Any, origin: Measured }` in `crates/2048/src/lib.rs`'s `Looks`,
  and a test like `oag-hd`'s `the_material_lod_bias_follows_its_rule`. Then re-render the three 2048 pairs.
  Check which model draws 2048's pads: `load/lod_bias.rs` leaves separate pad models at 0, and 2048's pad
  textures read -1.5. Omega must stay at bias 0 (its S# read 0.0).
- **Pulse PS2: one level, ready to wire** (confidence 92, over the maintainer's gate of 90). A race reads
  the base level at every distance: `TEX1` `MXL` 0 and `MMIN` 1 on 242,433 textured primitives (both fields) of four
  in-race dumps, and all 14,005 shipped texture objects author 0 levels beyond the base
  (`docs/rendering/texture-level-per-title.md`, "Pulse PS2 in a race"). Ours uploads a box chain: in
  `crates/mesh/src/mesh/ps2_textures.rs` pass `Some(1)` to `ModelTexture::rgba8`, PS2 path only, with a test
  that a distant PS2 texel reads the base level. PSP pairs must stay byte-identical. Not "PS2 inherits
  PSP": this is a PS2-measured value. Two things to know when wiring:
  - The original law is "the file header's `flags & 0x1f` levels beyond the base"
    (`docs/ghidra/functions/ps2-pulse-eu/texture-level.md`), 0 on every disc texture, so `Some(1)` matches
    all of them. `oag_texture::ps2_texture` refuses a blob with levels, so nothing else needs a path.
  - The original is one bilinear tap. A one-level upload under our sampler still takes anisotropic taps at
    the player's setting, and the PS2 compare profile should say what it runs.
- **TEXTURE DETAIL on the hardware path, ready to wire.** `scene.fog.texlod_shift` is read only on the
  PSP slope path, so `high`/`maximum` do nothing on PS2, HD, 2048 and Omega, though the row is shown
  there. On PS2 a one-level upload leaves the row nothing to move (it already reads the finest level), so
  once the item above lands it is a row with no choice there. Wiring: add the shift to `material_sample`'s bias, to `material_sample_grad`'s `exp2` scale
  and to the plain material `textureSample` reads. `original` must stay byte-identical on every
  title.
- **Pulse and Pure PSP: a stored `K` per texture, ready to wire** (Pulse 93, Pure 85). The `-9.0506`
  is the pre-race flyby camera's (`docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`, "`K = -9.0506` is
  the pre-race flyby camera's"; `docs/ghidra/functions/psp-pure-usa/texture-level.md`). The rule:
  each `.vex` texture holds a `K`, unset at load; at a draw it takes the drawing camera's
  `-log2(272 * m11)` when the two differ by more than 0.2; the flyby's draws count. Our flyby's field
  of view already gives `-9.050638`. It is view-side state, not `World`. Two limits to record, not
  solve: the original's residency rebuild is not modelled (17 of 102 flyby-built textures ended at the
  race value), and `--race`, captures and headless runs skip the flyby, so they keep the race `K` and
  the compare pairs will not move unless the capture runs the flyby's visibility. Worth about 2 to 3
  sixteenths of a level on the textures concerned.
- **Pure: the per-texture bias and mode-0 textures follow Pulse's wiring.** Same functions, K stored at
  texture `+0x2a0` with the 0.2 rebuild.
- **Not read:**
  - Omega's two +1.0-bias samplers' materials;
  - 2048's Zone and Detonator track skins and an HD/Fury circuit, live;
  - Pure's four textures at `-9.0491`, and whether Pure has Pulse's residency rebuild.

## Next Steps

1. After `pulse-farfield-lod` merges: wire the PS2 single level (about an hour with its test and the PS2
   pairs), then TEXTURE DETAIL on the hardware path (about an hour, a shader change with a level test per
   preset).
2. Wire 2048's `material_lod_bias` (about an hour with its test and the three pairs).
3. Decide whether the stored `K` is worth wiring (an afternoon with a level test; the visible size is
   2 to 3 sixteenths of a level on part of the scenery).
