# HD's LeachBeam body and ghost static still ask for Pulse's names

2026-10-06, logwarn-hd. Two warnings left in an HD race's log, both honest and
both naming the wrong asset.

## What was measured

- **LeachBeam body.** The log says `Data\Weapons\Textures\pulse_leechbeam1_ADD.mip:
  not in the archive set - the LeachBeam draws with no ribbon body`. That is
  Pulse's literal (`LeachBeam_LoadTexture`). HD's `EBOOT.elf` names no such
  texture and no `leechbeam` string but the cockpit damage model and rumble
  files (Ghidra `search_strings`). HD's beam is its own ribbon:
  `/data/ribboneffects/leachbeam_triangle.vex` + `.rcsmodel`,
  `materials/hd_leachbeam.rcsmaterial`, `textures/hd_leechbeam_glow.gtf`, all in
  `DATA02` (census, `crates/render/examples/hd_psarc_find.rs`), one of the four
  ribbons in `docs/rendering/trail-ribbon.md`. `DATA02` also carries
  `/data/weapons/textures/pulse_leechbeam1_add.gtf` as an orphan: nothing in the
  executable names it. So "draws with no ribbon body" is true on HD, and the fix
  is the HD ribbon, not a `.mip` to `.gtf` rename of Pulse's texture.
- **Ghost static.** `Data\Tex\staticglow.mip: absent - the ghost ship's static
  draws nothing`. HD has a ghost ship (`GhostShip.cpp`, `ship_ghost`,
  `MeshNode_Ghost`) and ships `/data/tex/staticglow.gtf` and `static.gtf`, but no
  `EBOOT.elf` string names either, so whether HD's ghost has a static pass, and
  with which texture, is unread. Binding `staticglow.gtf` to it would be a guess.

## Open

- A per-title name for the beam's ribbon, as `Title` data with provenance, once
  the HD ribbon is built (`leach_beam_texture` takes no title today).
- What `GhostShip.cpp` draws over a ghost on HD.

## Next Steps

1. Read the `leachbeam_triangle` ribbon the way `enginetrail_triangle` was read
   (`hds-engine-trail-is-one-of-four-ribbons.md`): `.vex` nodes, the
   `hd_leachbeam` material's program, then build it beside the other three.
2. Decompile `GhostShip.cpp`'s draw and see whether it samples a texture at all.
