# HD needs a per-material shader path, and two general rules for finding one have been refuted

2026-08-20. The RE is on [rcsmaterial.md](../docs/formats/rcsmaterial.md) - "What selects a variant", "The permutation word is read", "The fragment microcode decodes in Rust" - and in [renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md)'s lit-material sections. This row keeps only what is not there. **Where it stands.** `mesh.wgsl` has one lit path for all HD geometry and the disc does not: `track_surface` lights with `(pow(lightmap, power) * scale + f[TC1]) * albedo - bias`, `detonator_ship_rich_iridescent` adds a view-driven dodge/burn and an environment lookup, and **neither has a Lambert diffuse**. `ship.rcsmodel` is 57 meshes with 4 colour sets and no `lightmapUV`, so 53 fall to `albedo * ambient` and the hull renders dark: a missing material, not a missing light. **Built and verified**: `fragment::Program` decodes **37,461 of 37,461 blocks** disc-wide, agreeing with `scripts/ps3-microcode.py`; `skin::variants` resolves a variant per slot (**283 of 301 drawn materials, 913 of 978 chunks**) onto `Model::material_variants`. That is still not read for a per-material *lighting* branch - `model_probe` shows **0 pixels** moved by the variant itself - though its vertex half now is, for the coordinate flip below. **Two general rules tried and rejected; do not look for a third.** (1) Splitting families on what a `SHO` header *declares* puts 447 of 978 drawn chunks in a "neither" bucket alongside `track_wall` and `glasstest`, because a surface lit only by the interpolated term declares neither - both its light sources are **interpolators, not uniforms**. (2) A dataflow over the microcode (`output_lit_by`) is a correct primitive but a poor classifier: 745/168, with `cf_billboard1` landing **lit** because a billboard is modulated by an interpolated scroll too. **Which interpolator carries scene light is a per-material fact** (`f[TC1]` on `track_surface`, `f[TC0]` on `bluemetal`), not a property of the program's shape. Seven materials read that way (renderer.md, "The sun is real and it is masked") show five of them - **275 chunks against `track_surface`'s 15** - carrying a **Lambert sun diffuse** gated by a sun-occlusion scalar, and **the colour set's fourth byte is that scalar** (confidence 86). The sun removal was therefore a crude fix for a missing mask; the faithful fix landed the same day and moves 88.6 % of a fixed-tick frame, mean brightness 128.7 -> 137.3, not yet checked against RPCS3 pixel-for-pixel. A bug fixed in passing: `output_lit_by` read the taint on the *final* instruction, and most lit programs end on `MOV H0.w, {const}`. **Inputs the renderer still lacks**: a tangent frame, a **paraboloid** reflection map, shadow maps. The specular exponent is a labelled stand-in at 32; the disc uses 5, 10, 32, 26.156, 40 and 300.

## Open

- No per-material lighting branch exists: `mesh.wgsl` has one lit path for all HD geometry, and 53 of 57 ship meshes fall to `albedo * ambient` (dark hull)
- Two general classification rules (SHO-declared families, `output_lit_by` dataflow) were tried and rejected as classifiers
- Specular exponent is a labelled stand-in at 32; the disc actually uses 5, 10, 32, 26.156, 40 and 300 - **narrowed 2026-09-01, see below**
- Tangent frame, a paraboloid reflection map, and shadow maps are still missing inputs
- Checking the Lambert sun diffuse fix against RPCS3 pixel-for-pixel is blocked on [an-rpcs3-capture-harness-a-reference-frame-paired.md](an-rpcs3-capture-harness-a-reference-frame-paired.md)'s own open item: `viewProj` is not located yet, so the capture harness has no working camera. Not this thread's dependency to close.

**2026-09-01: `fragment::Program::specular_exponent()` reimplements the sweep
(`crates/formats/src/rcsmaterial/fragment.rs`), gated on a saturated-dot-fed
`LG2` and on the chain reaching the program's output - both necessary, per a
disc-wide re-sweep that does not reproduce the six published counts and says
why not: `5`/`10` turn out to be entirely Zone's `rim^10`/`rim^5` (nothing
survives excluding Zone-declaring blocks), and the largest surviving bucket
(`0`, 573 blocks) reads as `SpecularPower` patched at draw time rather than a
real fifth value - `pow(x, 0) = 1` is not a plausible authored shininess.
Full numbers and reasoning in
[renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), the
paragraph after the original sweep. **Only `32`, `40` and `300` still read as
plausible real exponents after this pass** - the identical instruction shape
also computes `exp(x)` via `log2(e)` and at least one Fresnel/falloff-shaped
curve, which a saturated-dot gate alone cannot tell apart from a genuine
`pow(N.H, e)` without tracing the dot's *other* operand.

## Next Steps

- Check the Lambert sun diffuse fix against RPCS3 pixel-for-pixel - blocked, see Open above; pick up [an-rpcs3-capture-harness-a-reference-frame-paired.md](an-rpcs3-capture-harness-a-reference-frame-paired.md) first
- Trace what the `DP3`'s other operand is for each `specular_exponent()` hit, to separate a real half-vector specular term from a Fresnel/falloff curve sharing its shape - this is what would let `32`/`40`/`300` (or more) be wired per-variant into `mesh.wgsl` instead of staying a shared stand-in
- Decode the `patch fslot` to `const@slot` table (`rcsmaterial.md`, already flagged as owed) to check the `0`-bucket theory directly: does `SpecularPower`'s patched slot actually correspond to one of these dead-literal `LG2`/`MUL`/`EX2` chains?
