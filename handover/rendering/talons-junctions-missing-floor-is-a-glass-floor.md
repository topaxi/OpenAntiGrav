# Talon's Junction's "missing floor" is a glass floor drawn with the wrong texture: rendered, not absent

2026-08-24, branch `worktree-hd-glass-floors`. Full evidence: [rcsmaterial.md](../../docs/formats/rcsmaterial.md), "Talon's Junction's missing floor is a glass floor" and "A texture slot names its sampler". **The finding worth more than the bug**: a material's textures are its own **input table**, entries `0x20` apart, `(hash, kind, ..., path)` - a hash eight bytes after a path belongs to the *next* entry, and the earlier reading was off by one. Each entry carries a **name hash**, and that, not its ordinal, is what a variant maps to a texture unit: **only 776 of 8,021 readable slots disc-wide land on their own ordinal**. There is no fixed number of slots either - 442 materials carry **1,291** entries, up to seven. `skin::picks` binds two **by role rather than position**; `SECOND_IS_LIGHTMAP` blocks both second-slot roles, without which the perforated trackside barrier loses its holes. **The lightmap is now bound wherever it sits** (85 -> 275 materials here; amphiseum 359, sol_2 309). Three roles are identified by what they bind disc-wide, 85-88 confidence: a **normal map**, a **specular map** and a **facing ramp**, so `FacingRamp` is named and countable. `skin::NOT_A_PICTURE` uses that list so the albedo fallback skips a lookup: on `etched_glass_tech` entry 0 *is* the ramp, and painting it at the road's UV was the floor's rainbow bands. **Nothing samples these in their own role yet.** `rcsmaterial.md`'s "the roles are the opposite of the slot names" is retired: a correct microcode trace over a wrong pairing. The tone gap is still open: we clip 21 % to white against a reference's 15 %.

## Open

- **The facing-ramp combine for this exact material (`etched_glass_tech`, the
  glass floor) is traced and now drawn - resolved 2026-09-17, `lane-hd-glass`.**
  See [hd-needs-a-per-material-shader-path-and.md](hd-needs-a-per-material-shader-path-and.md)
  for the fix (a `Pick`-routing bug, not the hash-mismatch either open
  hypothesis there named) and what it still leaves out (the `c` parameter,
  `vertexLight`'s exact composition, the missing paraboloid probe).
  `mag_effect_loop_opaque` - this file's own magstrip finding below - still
  does not classify into that combine and stays undrawn on its own terms.
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
  `oag-game` rather than the wrong-coloured ramp described above. **Not the
  same finding as [hd-needs-a-per-material-shader-path-and.md](hd-needs-a-per-material-shader-path-and.md)'s
  "no route to draw it"** - resolved later the same day, see the next bullet
- **Resolved 2026-09-13, and confirmed by measurement: the player report was
  right that this is a magstrip-specific material, not `etched_glass_tech`,
  and it was black for a third reason neither open hypothesis named.** The
  black chunk is `materials/mag_effect_loop_opaque.rcsmaterial` - Talon's
  Junction's magstrip floor is a two-surface chunk, this opaque material
  underneath a blended `mageffectloop.rcsmaterial` glow - identified by
  `OAG_TINT_MATERIALS` (squared colour error 9.4 against a 20.7 runner-up)
  and by triangle-level distance (0.98 units from the camera's own forward
  ray, `hd_near_probe.rs`). It read **`no resolved variant`**, not because
  its permutation key misses every shipped row, but because
  `mesh/rcs/skin.rs`'s `variants()`/`flips()` counted a material's chunks by
  walking `model.meshes` directly, which only reaches a chunk's *first*
  surface - `mag_effect_loop_opaque` is exclusively an **extra surface**
  (`oag_rcs::rcsmodel::Mesh::extra_surfaces`, a second material painted over
  the same geometry), so it was never even asked to resolve, the same way a
  genuinely-unused material is skipped. **Fixed**: both maps now walk
  `Mesh::surfaces()` instead, the same walk the real emit loop already uses
  to draw the extra surface at all. Disc-wide on this one circuit: 286 of 302
  drawn materials resolved → 423 of 439 (929 of 983 chunks → 1,759 of 1,813) -
  every extra-surface material on the circuit was subject to the same bug,
  confirming this is a general resolver gap and not a magstrip-specific one
  (a ship hull sample gained covered chunks too, 12 of 12 → 16 of 16, with
  `variants_unshipped` still 0 both times). Full evidence, the screenshot
  verdict and the remaining gap: [rcsmaterial.md](../../docs/formats/rcsmaterial.md),
  "Talon's Junction's magstrip floor was black because its material was
  never asked to resolve at all". **Not fully closed, and it is not simply
  `etched_glass_tech`'s own now-fixed gap either** - see the 2026-09-17
  bullet above. `mag_effect_loop_opaque`'s resolved block only resembles
  `etched_glass_tech`'s as a family: measured directly, it declares five
  units at different assignments (grid at unit 1, ramp at unit 4, a fourth
  real texture at unit 3 this renderer has never named), wraps a fog idiom
  neither material needed traced before, and its alpha lane does not resolve
  to a unit at all (`Texel::Untraced`) where `etched_glass_tech`'s does. So
  `etched_glass_tech`'s fix does not reach it, and reusing that combine here
  would be inventing a program this project has not read - it stays
  classified as "a different combine" and undrawn, per `CLAUDE.md`

## Next Steps

- `etched_glass_tech`'s own facing-ramp shading is done (2026-09-17). What is
  left on [hd-needs-a-per-material-shader-path-and.md](hd-needs-a-per-material-shader-path-and.md):
  the normal map, the specular map, `mag_effect_loop_opaque`'s own untraced
  combine, and the three named omissions in the glass-sheen fix itself (`c`,
  `vertexLight`'s exact composition, the paraboloid probe)
- The tone gap's own harness blocker is gone (see `## Open` above) - a
  precise same-camera re-measurement is what is left, not a capture-harness
  fix
