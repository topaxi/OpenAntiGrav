# A texture coordinate's orientation is per material, and it is in the vertex microcode

2026-08-24, branch `worktree-hd-glass-floors`. Written up in [rcsmodel.md](../docs/formats/rcsmodel.md), "A texture coordinate's orientation is per material", and [rcsmaterial.md](../docs/formats/rcsmaterial.md), "The vertex program says which way up a texture coordinate is". This row is what is not there. **The symptom** was two flat black bands flanking Talon's Junction where the reference has light grey barrier walls - 25,469 sampled pixels at mean luma 14.8/255, and what "bigger structures missing" looked like from the seat. **The cause is not in the geometry**: `track_wall`'s vertex block computes `o[TC6].y = c[206].y - Uv1.y`, `c[206].y` pinned to 1.0 by the `t * 2 - 1` tangent unpack four instructions later, so what its fragment program samples with is `1 - v`. The chunks store two v values and no others, 0.724 and 1.000; flipped they land on `ds_wall_cs.gtf`'s pale concrete instead of the near-black, alpha-0 band at its bottom. **Per material**: 18 of `track_wall`'s 94 vertex blocks flip and none of `track_surface`'s 64 do; **one of 283 resolved variants** flips. A global flip fixes the walls and breaks what was already right - that experiment was run, which is why the reading lives in `rcsmaterial::vertex::Program::flips`. **Crushed pixels fall 9.5 % -> 0.3 %** against the reference's 1.1 %; mean luma 164 -> 178 against 173. **Six explanations were measured and died first and the table on the page keeps them**, so nobody repeats the sweep. **The new module**, `oag_formats::rcsmaterial::vertex`, decodes NV40 vertex microcode the way `fragment` decodes the pixel side, narrowly - only "does this negate that attribute into an output texcoord" - with the false positives as tests. Two claims were made and withdrawn the same day, both from reading a tinted frame by hue: that the black bands hid the missing scenery, and that a tapered shape was the terminal building (it is slot 441, the air-traffic lanes). **Match a tinted pixel against `hd_slot_list`'s slots-that-actually-draw, never against all 400 ordinals** - most never draw, and a blended draw's hue is composited with what is behind it. **New diagnostics**, env-gated and off by default: `OAG_ONLY_SLOT` keeps one material ordinal; `OAG_ALBEDO_ONLY` draws real pictures with no light, which proved the wall's darkness was its texel and not its rig. New probes: `hd_light_probe` (the light equation on the CPU, **area-weighted**, `OAG_PER_DRAW`), `hd_alpha_census`, `hd_state_census`, `hd_flip_census`, `hd_slot_list`, `hd_params`, `hd_strings`, `hd_submesh_dump`, `hd_cat`. **Free from the state-word census** (7,241 records, 13 distinct words): bit 7 of `Material::state` was thought to be carried by exactly the `*_atoc` family. Measured precisely on 2026-08-25 (see Open below): it is not a material-name family at all, but a per-instance flag that tracks which texture a material samples - 113 of 115 `_atoc`-textured records carry it, and 113 of 145 bit-7 records sample an `_atoc` texture, with the exceptions concentrated in one over-reused material name rather than scattered. "Alpha-to-coverage" specifically remains a name-based inference, not confirmed. `Transparency::Mode2` holds `fence_alpha`, `nr_crowd_bustle` and `jd_alphalambert_alphatest`, the alpha-test reading rcsmodel.md calls unrecovered. Neither is wired; `track_wall` is `0x7c`, plain opaque.

## Open

- ~~Bit 7 of `Material::state` (alpha-to-coverage) is "very likely" but not confirmed.~~
  Measured 2026-08-25: it is a real per-instance flag that tracks which
  texture a material samples, not the material's name - confidence 84, in
  [rcsmodel.md](../docs/formats/rcsmodel.md#bit-7-tracks-the-texture-not-the-material-name---confidence-84).
  **What it specifically selects is still open**: "alpha-to-coverage" is a
  confidence-55 inference from the content classes it always lands on
  (foliage, distant crowds, traffic sprites, on-screen text), not a reading
  of the executable - no RSX register write has been tied to it.
- `Transparency::Mode2`'s alpha-test reading, which rcsmodel.md calls unrecovered, is not resolved.
- Neither bit 7 nor `Transparency::Mode2` is wired into `oag_render`. For bit 7
  specifically, wiring depends on HD's render path running multisampled -
  `mesh_render::build`'s `sample_count` is 1 outside a race, so
  `alpha_to_coverage_enabled` would be a conditional no-op there and only take
  effect during a race with MSAA on. Worth wiring anyway once the "what it
  selects" question above is settled, since it degrades gracefully.

## Next Steps

- Tie bit 7 to an RSX method write in `ps3-hdfury-eu`'s Ghidra database, to
  move "alpha-to-coverage" from a content-class inference to a decompiled
  read. The database is a fresh import with no material/RSX-state functions
  named yet, so this starts cold - from method-constant pattern matching in
  the render layer's own address range, not from an existing name to search
  for. See `docs/reverse-engineering/toolchain.md` and the per-function TOC
  trap on PS3 Ghidra work.
- Separately: dump the fragment programs for `Transparency::Mode2`'s six
  material names (`nr_crowd_bustle`, `jd_alphalambert_test`, `fence_alpha`,
  `emissive_alpha_heathaze_test`, `cf_alpha4glow`,
  `uv_anim_diffuse_alpha_emissive`) with `scripts/ps3-microcode.py` and look
  for a kill/comparison at a threshold that isn't 0.5 - 0.5 is already
  refuted by the crowd experiment on rcsmodel.md's page. This is microcode
  work, unlike bit 7 above: a comparison-and-kill is visible in the fragment
  program itself, where fixed-function GPU state like alpha-to-coverage is
  not.
