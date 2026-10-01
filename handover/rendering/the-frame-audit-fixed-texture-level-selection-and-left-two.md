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

- **The hull's sheen: drawn 2026-09-30, gap partly closed.** ~~Lead: the single
  `Glass_ADD` batch.~~ The cause is the `0x2000` extra pass - six batches per hull
  drawn under environment mapping with their material's second texture
  (`envtest4bit.tga`), `model+0x1a8 == 1` so the basis is the fixed world-space
  pair (read live; `mesh-draw.md`, "The hull's extra pass"). `oag_render::shine`
  draws it; it closes roughly a third to a half of the measured spine gap
  (`frame-audit.md` section 2). Closed 2026-10-01 on a second GE dump of `07_Track`
  ("Re-read on a second dump" in `mesh-draw.md`): ~~replace-then-add against our
  single additive redraw~~ (the sums are equal; the canopy's depth test is on, not
  off), ~~fog on the pass~~ (on, colour `0`; written), ~~a track's `*_shinemap`
  batches~~ (rows 0 and 1 of the view matrix, confirmed at two yaws; drawn,
  `--no-track-shine`). Still open: the airbrakes' flap deflection on the pass, and the
  wreck model (`+0x8b8`, same flags) - neither started (pulse-glow did not reach them).
- ~~**The tunnel rim's neon strip.**~~ **Closed 2026-10-01** (`pulse-glow`). It
  was not the shine pass and not a missing batch: the arch lights and rim light
  are **blended batches with the glow bits**, which the original stamps into the
  bloom's mask (`0xfa`, their texture's own byte) and ours left at `4`, so they
  had no bloom at all. `oag_render::mesh_render::stamp` now writes it
  (`docs/rendering/glow-mask.md`, "Transparent batches stamp"); a frame of ours
  at a bright phase of the strip's brightness cycle (about 300 ticks; which texture
  drives it is unread) has 1,584 pixels over 225 in the strip's box against the
  original's 1,581, from 186. The
  yellow-green streak at the *yawed* pose is a different thing: prims 627/628 of
  `yaw.ppdmp` are WAD entry 1078, `pSphereShape1` with `cage_collision2_ADD.tga`
  (a collision-cage sphere about 19 units from the craft), present only in that
  dump. Not drawn by us and not read: what spawns it, and why it sat there while
  the craft was merely yawed, are open.
- ~~**Factory roofs on the Talon straight.**~~ **Fixed 2026-09-30.** The roofs are
  draws 1479..1494 (four animated slab transports, `track.vex` nodes 789, 793, 804,
  806), not 1381..1391; the original's GE list never contains them (`psp-ge-dump.py`,
  seven dumps, four poses, two boots), and `oag_render::pvs::visible` had exempted
  every moving draw from the section mask as well as the frustum. The mask now
  applies (`frame-audit.md` section 3). Still open on it: 21 to 24 moving draws at
  rest poses that pass the mask and the original does not submit (its frustum uses a
  bound for moving meshes that we lack), and one circuit only.
- **Whether real PSP hardware shows level 1 past about 256 units.** The game
  programs slope mode with slope `1/256` and bias 1; the renderer now applies
  that law to the disc's levels, and PPSSPP's GPU backends do not, so no frame of
  the emulator can confirm the far field. Two things stay unread: that our
  `clip.w` is the GE's `|z|`, and `Texture_BuildBindList`'s per-texture mode and
  bias. A hardware capture or PPSSPP's software renderer would say.
- **Wipeout Pure shares the slope rule**; its pre-swizzled textures keep a
  synthesised chain. Unmeasured against a Pure capture.
- The cyan strip lights and the start gantry differ between the two sides by
  timing only; a comparison that needs them should pin the same tick after GO.

- **The start-line laser's mask is about 1.5x too wide** on Talon's second grid
  (original 710 pixels at `0xaf`, ours 1,061) even with the additive class's colour
  test: where the GE places that test relative to the fog is unread, and ours tests
  the lit texel before the fog.
- **The blended-glow stamp resubmits every transparent draw of a stamping model**
  (`Drawable::draw_stamps`; a batch without the glow bits discards whole in
  `fs_main_stamp`). `Model` carries no per-draw flag for it, so the GPU cost is
  unmeasured and a per-texture or per-draw table would cut it.
- **The bloom spread around a lit hull (`leachbeam-fidelity.md`) is blocked on a
  stationary absorb frame**: `psp-fire-weapon.py` has no absorb bit, the pickup grant
  path is unread, and firing the LeachBeam halts PPSSPP. The method that now works
  for a mask comparison is documented in `docs/rendering/glow-mask.md`; it needs a
  craft at rest.
- **The wreck model (`+0x8b8`) and the airbrake flap deflection on the hull's extra
  pass** - the first Open bullet's remainder - were not started.

## Next Steps

- Repeat the roofs measurement on another circuit (Metropia, Tech De Ra) with
  `scripts/psp-ge-dump.py`: the rule was measured on Talon's Junction alone, and a
  moving draw the original submits but a section hides would show as a matched
  signature lost by the mask.
- Capture Pure at one pose with `psp-trace.py` and confirm or scope the slope
  rule; read `Texture_BuildBindList`'s mode/bias branch.
