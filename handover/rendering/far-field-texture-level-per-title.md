---
status: ready-to-wire
titles: [pure, pulse, hd, 2048, omega]
updated: 2026-11-27
summary: "Far-field texture level: the PSP stored K per texture (the flyby's -9.0506) and 2048's own material mip levels ready to wire"
---

# Far-field texture level, title by title

Lane `farfield-lod-titles`, 2026-11-27. The laws, evidence and confidences are in
[texture-level-per-title.md](../../docs/rendering/texture-level-per-title.md). Each item below wires
read values only, as `Title` data, with a test, a break-it, and the other titles' pairs byte-identical.

## Open

- **2048: material textures sample a box-filtered chain, not the file's levels.** The loader's bias is
  wired ([gxt-loader.md](../../docs/ghidra/functions/vita-2048-eu-v104/gxt-loader.md), "Ours"), so it
  selects among levels of our own making. `ModelTexture::from_gxt` drops all but the base;
  `Gxt::lower_levels_rgba` reads the rest. `Texels::Chain` cannot carry them: it also turns on the PSP
  slope law, so the model needs a form that brings levels without it. Omega's `.gnf` and HD's `.gtf`
  already upload their own chains.
- **2048 pairs: the original is anti-aliased and its profile runs ours at MSAA off**
  ([texture-level-per-title.md](../../docs/rendering/texture-level-per-title.md), "2048 wired: what moved in the
  pairs"; inferred from the spectrum, 60). Read the render target's multisample mode in the executable or off a
  RenderDoc capture, then set the profile. A top-band excess stays at 4x, mostly along y; the box-filtered material
  chain above is the first thing to test against it.
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
  - 2048's Detonator track skins and an HD/Fury circuit, live;
  - Pure's four textures at `-9.0491`, and whether Pure has Pulse's residency rebuild.

## Next Steps


1. Upload 2048's own material levels (an afternoon with a level test and the 2048 pairs; confidence 50).
2. Decide whether the stored `K` is worth wiring (an afternoon with a level test; the visible size is
   2 to 3 sixteenths of a level on part of the scenery).
