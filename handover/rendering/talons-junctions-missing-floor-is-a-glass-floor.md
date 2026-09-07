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
  same-camera comparison needs
  [viewproj-is-located-the-capture-harness-still.md](viewproj-is-located-the-capture-harness-still.md)'s
  own open item (`viewProj` is located; the harness's finder does not read it yet) before this can be measured
  precisely rather than eyeballed off the existing `data/reference/hd-talons-glass/`
  capture

## Next Steps

- Pick up [hd-needs-a-per-material-shader-path-and.md](hd-needs-a-per-material-shader-path-and.md)
  for the shading work - normal map, specular map and this material's facing
  ramp all live there now
- Pick up [viewproj-is-located-the-capture-harness-still.md](viewproj-is-located-the-capture-harness-still.md)
  first if the goal is the tone gap specifically
