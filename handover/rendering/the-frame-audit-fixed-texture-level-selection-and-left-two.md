# The Pulse frame audit fixed texture level selection and left two differences it could not trace

2026-09-30, lane `pulse-frame-audit`. Full table, method and numbers:
[frame-audit.md](../../docs/rendering/frame-audit.md). Three circuits matched to
the original at 480x272 with no filters on either side; sky, fog and tone agree
within 5 %, and the one measured difference a player would see across the board
was soft scenery, fixed by uploading a PSP `.vex` texture with the mip levels the
disc authors and selecting among them by the GE's recovered slope law
(`Texels::Chain`, `mesh_render::PSP_TEXLOD_SLOPE`; tests `psp_slope_lod.rs` and
`psp_texture_levels_ground_truth.rs`). Two differences are traced only as far as
a lead. The TEXTURE DETAIL setting (`original`/`high`/`maximum`) landed on top
of it the same day; see `frame-audit.md`.

## Open

- **The hull's sheen.** The original's yellow spine and inner wings carry a gloss
  ours lacks (or has less of). Ruled out: the light list, bloom, the idle flare.
  Lead: `Ship.vex`'s one transparent batch (`Glass_ADD.tga`, 8x8) draws under
  uvgen-2 environment mapping in the original and `oag_render::texgen` has no
  caller. A naive wiring (every transparent batch, matcap from the view rows) left
  the hull unchanged and erased the track's glass floor, so the wiring needs the
  per-model branch first: `model+0x1a8` decides fixed basis or view matcap, and
  the hull's value was not read.
- **Factory roofs on the Talon straight.** At `(161,-47,-185)` heading
  `-0.94,-0.09,-0.34` ours draws dark bowl shapes (track draws 1381-1391, PVS on)
  where the original shows terrain. Not traced: draw order versus terrain,
  a PVS section the original culls, or our section placement.
- **Whether real PSP hardware shows level 1 past about 256 units.** The game
  programs slope mode with slope `1/256` and bias 1; the renderer now applies
  that law to the disc's levels, and PPSSPP's GPU backends do not, so no frame of
  the emulator can confirm the far field. Two things stay unread: that our
  `clip.w` is the GE's `|z|`, and `Texture_BuildBindList`'s per-texture mode and
  bias. A hardware capture or PPSSPP's software renderer would say.
- **`--anisotropy` barely changes a PSP `.vex` model any more**: the slope law
  samples an explicit level, which takes no anisotropic footprint (602 pixels
  differ off against `16x` on the derivative path, 5.6 on the law; measured on
  the Talon straight, see `frame-audit.md`). `textureSampleGrad` with gradients
  scaled to the law's level would keep both; not tried.
- **Wipeout Pure shares the slope rule**; its pre-swizzled textures keep a
  synthesised chain. Unmeasured against a Pure capture.
- The cyan strip lights and the start gantry differ between the two sides by
  timing only; a comparison that needs them should pin the same tick after GO.

## Next Steps

- Live-read the hull model's `+0x1a8` and the `+0x48`/`+0x70` light lists off the
  trace's entity pointer (the `craft+0x8b4` read this pass did not resolve to a
  model), then wire uvgen 2 for `Ship.vex`'s glass batch alone and compare
  against `data/shots/frame-audit/talon-mid2`.
- Bisect the factory roofs by draw index (`OAG_LO`/`OAG_HI`-style filter on the
  opaque loop) against a capture with the terrain's own layer, before touching
  PVS placement.
- Capture Pure at one pose with `psp-trace.py` and confirm or scope the slope
  rule; read `Texture_BuildBindList`'s mode/bias branch.
