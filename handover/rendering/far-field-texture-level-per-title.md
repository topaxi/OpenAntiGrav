---
status: ready-to-wire
titles: [pure, pulse, hd, 2048, omega]
updated: 2026-11-27
summary: "Far-field texture level: 2048 bias (92) and the PSP stored K per texture (the flyby's -9.0506) ready to wire"
---

# Far-field texture level, title by title

Lane `farfield-lod-titles`, 2026-11-27. The laws, evidence and confidences are in
[texture-level-per-title.md](../../docs/rendering/texture-level-per-title.md). Each item below wires
read values only, as `Title` data, with a test, a break-it, and the other titles' pairs byte-identical.

## Open

- **2048: `material_lod_bias` -1.5, ready to wire** (confidence 92; the maintainer's gate is 90). Every disc
  texture bound carries bias -1.5, trilinear, no anisotropy on Queens Mall (two boots, 269 and 311
  textures), Altima (184) and Vineta K (two boots, 228 and 216). Zone (two boots, 29 textures): only the
  Zone skins differ, mip filter off, one level
  (`docs/ghidra/functions/vita-2048-eu-v104/gxt-loader.md`, "In a race"). Not measured live: Detonator
  (same loop statically), Fury circuits, a native circuit's Zone.
  Wiring, by family:
  - track, scenery, lightmaps, craft: one `material_lod_bias: Rule { on: Any, origin: Measured }` in
    `crates/2048/src/lib.rs`'s `Looks`, and a test like `oag-hd`'s `the_material_lod_bias_follows_its_rule`;
  - pads and weapon models: `load/lod_bias.rs` leaves separate models at 0; on 2048 they read -1.5;
  - flame, absorb shell, weapon programs: `detail_sample` takes no title bias; their textures read -1.5.
    Particles and trails (`oag-fx`) read -1.5 too and take no shift;
  - Zone skins: add no bias. Check that ours loads the one-level `data2.psarc` skins.
  Then re-render the three 2048 pairs. Omega must stay at bias 0 (its S# read 0.0).
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


1. Wire 2048's `material_lod_bias` (about an hour with its test and the three pairs).
2. Decide whether the stored `K` is worth wiring (an afternoon with a level test; the visible size is
   2 to 3 sixteenths of a level on part of the scenery).
