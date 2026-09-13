# Talon's Junction's "missing floor" is a glass floor drawn with the wrong texture: rendered, not absent

2026-08-24, branch `worktree-hd-glass-floors`. Full evidence: [rcsmaterial.md](../../docs/formats/rcsmaterial.md), "Talon's Junction's missing floor is a glass floor" and "A texture slot names its sampler". **The finding worth more than the bug**: a material's textures are its own **input table**, entries `0x20` apart, `(hash, kind, ..., path)` - a hash eight bytes after a path belongs to the *next* entry, and the earlier reading was off by one. Each entry carries a **name hash**, and that, not its ordinal, is what a variant maps to a texture unit: **only 776 of 8,021 readable slots disc-wide land on their own ordinal**. There is no fixed number of slots either - 442 materials carry **1,291** entries, up to seven. `skin::picks` binds two **by role rather than position**; `SECOND_IS_LIGHTMAP` blocks both second-slot roles, without which the perforated trackside barrier loses its holes. **The lightmap is now bound wherever it sits** (85 -> 275 materials here; amphiseum 359, sol_2 309). Three roles are identified by what they bind disc-wide, 85-88 confidence: a **normal map**, a **specular map** and a **facing ramp**, so `FacingRamp` is named and countable. `skin::NOT_A_PICTURE` uses that list so the albedo fallback skips a lookup: on `etched_glass_tech` entry 0 *is* the ramp, and painting it at the road's UV was the floor's rainbow bands. **Nothing samples these in their own role yet.** `rcsmaterial.md`'s "the roles are the opposite of the slot names" is retired: a correct microcode trace over a wrong pairing. The tone gap is still open: we clip 21 % to white against a reference's 15 %.

## Open

- The facing-ramp combine for this exact material (`etched_glass_tech`, the
  glass floor) is now fully traced - see
  [hd-needs-a-per-material-shader-path-and.md](hd-needs-a-per-material-shader-path-and.md),
  which is where its two implementation blockers (an unrouted sampler bind, a
  missing paraboloid reflection probe) are tracked so they sit beside the
  renderer's other missing inputs rather than being duplicated here
- Tone gap: renders clip 21 % to white against a reference's 15 % - a
  same-camera comparison is unblocked as of 2026-09-13 (the capture
  harness's camera pick is fixed, see
  `docs/reverse-engineering/rpcs3-capture.md`'s "The pick is fixed, and a
  rendered overlay confirms it"), but a precise re-measurement against a
  Talon's Junction pair framing this material was not taken this session -
  still open, just no longer blocked on the harness
- A same-camera overlay taken 2026-09-13 (unrelated goal: verifying the
  capture harness's camera pick) caught this material's other symptom by
  accident: a road panel a few dozen units ahead renders flat black in
  `oag-game` rather than the wrong-coloured ramp described above, consistent
  with [hd-needs-a-per-material-shader-path-and.md](hd-needs-a-per-material-shader-path-and.md)'s
  "no route to draw it" finding. Not confirmed as the same chunk; see that
  thread's own note for the evidence paths (gitignored)
- **Player report, 2026-09-13: the black panel is where a mag-strip section
  and the glass floor coincide** - a case unique to this circuit, not the
  general glass-floor gap. Treat it as a reliable oracle, and turn it into a
  measurement before touching anything: (1) check whether the black chunk's
  collision class is `MagFloor` (the `Floor`/`MagFloor` filter
  `docs/rendering/shadows.md` already uses reaches it) and whether its
  material is `etched_glass_tech` or a second, magstrip-specific material;
  (2) if it is a distinct material, its variant may be declaring a class or
  permutation the resolver never asks for (the same shape as the ship-hull
  `VertexColour1` miss fixed in `432ec4aa`) - the load report's
  "resolved to a shipped shader variant" line for `track.vex` says which;
  (3) what would falsify the report: the same material drawing correctly on a
  non-magstrip glass panel elsewhere on the circuit, which would make the
  magstrip the trigger rather than a coincidence. A matched frame
  (`data/reference/hd-capture/talons-matched/03`) already frames the panel

## Next Steps

- Pick up [hd-needs-a-per-material-shader-path-and.md](hd-needs-a-per-material-shader-path-and.md)
  for the shading work - normal map, specular map and this material's facing
  ramp all live there now
- The tone gap's own harness blocker is gone (see `## Open` above) - a
  precise same-camera re-measurement is what is left, not a capture-harness
  fix
