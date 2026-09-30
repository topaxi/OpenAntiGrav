# The Pulse frame audit fixed texture level selection and left two differences it could not trace

2026-09-30, lane `pulse-frame-audit`. Full table, method and numbers:
[frame-audit.md](../../docs/rendering/frame-audit.md). Three circuits matched to
the original at 480x272 with no filters on either side; sky, fog and tone agree
within 5 %, and the one measured difference a player would see across the board
was soft scenery, fixed by uploading a PSP `.vex` texture with one mip level
(`mesh::PSP_SAMPLED_LEVELS`, ground truth
`psp_texture_levels_ground_truth.rs`). Two differences are traced only as far as
a lead.

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
- **Whether real PSP hardware mips scenery past about 128 units.** The game
  programs slope mode with slope `1/256`; PPSSPP's GPU backends do not implement
  that law, and the fix reproduces what PPSSPP draws. A hardware capture or the
  software renderer would say.
- **Wipeout Pure shares the one-level line**, unmeasured against a Pure capture.
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
- Capture Pure at one pose with `psp-trace.py` and confirm or scope the
  one-level cap.
