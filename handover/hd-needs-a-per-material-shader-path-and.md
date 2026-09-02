# HD needs a per-material shader path, and two general rules for finding one have been refuted

2026-08-20. The RE is on [rcsmaterial.md](../docs/formats/rcsmaterial.md) - "What selects a variant", "The permutation word is read", "The fragment microcode decodes in Rust" - and in [renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md)'s lit-material sections. This row keeps only what is not there. **Where it stands.** `mesh.wgsl` has one lit path for all HD geometry and the disc does not: `track_surface` lights with `(pow(lightmap, power) * scale + f[TC1]) * albedo - bias`, `detonator_ship_rich_iridescent` adds a view-driven dodge/burn and an environment lookup, and **neither has a Lambert diffuse**. `ship.rcsmodel` is 57 meshes with 4 colour sets and no `lightmapUV`, so 53 fall to `albedo * ambient` and the hull renders dark: a missing material, not a missing light. **Built and verified**: `fragment::Program` decodes **37,461 of 37,461 blocks** disc-wide, agreeing with `scripts/ps3-microcode.py`; `skin::variants` resolves a variant per slot (**283 of 301 drawn materials, 913 of 978 chunks**) onto `Model::material_variants`. That is still not read for a per-material *lighting* branch - `model_probe` shows **0 pixels** moved by the variant itself - though its vertex half now is, for the coordinate flip below. **Two general rules tried and rejected; do not look for a third.** (1) Splitting families on what a `SHO` header *declares* puts 447 of 978 drawn chunks in a "neither" bucket alongside `track_wall` and `glasstest`, because a surface lit only by the interpolated term declares neither - both its light sources are **interpolators, not uniforms**. (2) A dataflow over the microcode (`output_lit_by`) is a correct primitive but a poor classifier: 745/168, with `cf_billboard1` landing **lit** because a billboard is modulated by an interpolated scroll too. **Which interpolator carries scene light is a per-material fact** (`f[TC1]` on `track_surface`, `f[TC0]` on `bluemetal`), not a property of the program's shape. Seven materials read that way (renderer.md, "The sun is real and it is masked") show five of them - **275 chunks against `track_surface`'s 15** - carrying a **Lambert sun diffuse** gated by a sun-occlusion scalar, and **the colour set's fourth byte is that scalar** (confidence 86). The sun removal was therefore a crude fix for a missing mask; the faithful fix landed the same day and moves 88.6 % of a fixed-tick frame, mean brightness 128.7 -> 137.3, not yet checked against RPCS3 pixel-for-pixel. A bug fixed in passing: `output_lit_by` read the taint on the *final* instruction, and most lit programs end on `MOV H0.w, {const}`. **Inputs the renderer still lacks**: a tangent frame, a **paraboloid** reflection map, shadow maps. The specular exponent is a labelled stand-in at 32; the disc uses 5, 10, 32, 26.156, 40 and 300.

## Open

- No per-material lighting branch exists: `mesh.wgsl` has one lit path for all HD geometry, and 53 of 57 ship meshes fall to `albedo * ambient` (dark hull)
- **A second, independent `RigidBody` consumer surfaced 2026-09-02**: `crates/render/src/mesh/rcs/skin.rs`'s `variants()` hardcodes `Class::Static` for every chunk on every file, so a material whose only variant is `Class::RigidBody` - the ship's own class, per `rcsmaterial::Class`'s doc - never resolves regardless of what permutation it ships. `Data\FE\FrontEndScene\FrontEndScene_HD_ATG.vex`'s one material (`basic_vertexemissive.rcsmaterial`, one `RigidBody` variant) is the same gap from a different file; see [hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md](hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md). Worth trying `RigidBody` (and `StaticQuake`) alongside `Static` in `variants()` once this thread's per-material lighting branch exists to feed - resolving the key with nothing to shade it differently would only move the count, not the picture.
- Two general classification rules (SHO-declared families, `output_lit_by` dataflow) were tried and rejected as classifiers
- Specular exponent is **wired per material now, 2026-09-01 - see below.** `mesh.wgsl`'s shared `32` remains only the fallback, for a literal `0.0` (patched at draw time) or a material the decoder could not resolve.
- Tangent frame, a paraboloid reflection map, and shadow maps are still missing inputs
- **The one member of the glass family with its shading fully traced still has no route to draw it.** `etched_glass_tech`'s block #7 (Talon's Junction's glass floor) is a view-angle sheen, not a light: `TEX H2.xyz, -R2.wwww unit2` at `dot(V, N)`, combined as `vertexLight * (ramp + c) + ramp`, plus the etched grid texture's red as an additive term and a `paraboloidReflectionTex` tint weighted by that same red, with the grid's RGB never sampled and its red the output alpha - see [rcsmaterial.md](../docs/formats/rcsmaterial.md#three-sampler-roles-identified-by-what-they-bind-and-the-glass-floor-stops-painting-a-ramp). Two blockers, not one: the grid texture (`glass_etched_tech.gtf`) is bound in the model record to sampler hash `lightmap`, which none of this material's 30 variants declares, so nothing routes it to a unit at all; and the reflection tint needs the paraboloid probe this bullet already lists as missing. A reference capture to verify against exists at `data/reference/hd-talons-glass/talons-glass-floor.png` (gitignored, supplied 2026-08-24). See [talons-junctions-missing-floor-is-a-glass-floor.md](talons-junctions-missing-floor-is-a-glass-floor.md) for the rest of that material's read.
- Checking the Lambert sun diffuse fix against RPCS3 pixel-for-pixel is blocked on [an-rpcs3-capture-harness-a-reference-frame-paired.md](an-rpcs3-capture-harness-a-reference-frame-paired.md)'s own open item: `viewProj` is not located yet, so the capture harness has no working camera. Not this thread's dependency to close.
- The `0.0` bucket (573 of ~1,050 non-Zone chains, the largest) falls back to the shared `32` rather than resolving to a real value - closing it needs the `patch fslot` to `const@slot` decoder `rcsmaterial.md` already flags as owed.
- `200`/`250`/`260`/`35` are wired as read (confidence 80, same as `32`/`40`/`300`) but not independently confirmed the way the ship's `40` is - tracing each chain's `DP3`'s other operand would tell a real half-vector specular term from a Fresnel/falloff curve sharing the same idiom, and is the one thing that could still retract a wired value rather than just add confidence to it.

**2026-09-01: wired.** `fragment::Program::specular_exponent()` reimplements
the sweep (`crates/formats/src/rcsmaterial/fragment.rs`), gated on a
saturated-dot-fed `LG2`, on the chain reaching the program's output, and -
found while validating the disc-wide numbers - on the exponent literal not
being `log2(e)` (the `exp(x)` idiom sharing the same three instructions,
matched exactly rather than by magnitude). A disc-wide re-sweep with all
three gates does not reproduce the six originally published counts and says
why not: `5`/`10` turn out to be entirely Zone's `rim^10`/`rim^5` (nothing
survives excluding Zone-declaring blocks), and the largest surviving bucket
(`0`, 573 blocks) reads as `SpecularPower` patched at draw time rather than a
real value - `pow(x, 0) = 1` is not a plausible authored shininess. Full
numbers and reasoning in
[renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), the
paragraph after the original sweep.

**`mesh::rcs::skin::roles` now calls `specular_exponent()` per material slot
and `mesh.wgsl` reads `in.specular_exponent` instead of a literal `32.0`** -
`GpuVertex::specular_exponent`, threaded through `Surface` exactly as
`slots`/`roles` already are. Every resolved non-zero value is wired as read,
following CLAUDE.md's "recovered and faithful values/wiring is always
preferred" rather than held back a second time for an uncertainty (some of
`200`/`250`/`260`/`35` might be a Fresnel term, not a shininess) already
priced into the confidence-80 label. A literal `0.0` and an unresolved
material both fall back to the shared `32` stand-in
(`mesh::DEFAULT_SPECULAR_EXPONENT`).

**Verified against real disc data two ways, and a screenshot is the weaker
of the two here.** Building every `.rcsmodel`/`.vex` pair on the disc through
the real `mesh::rcs::build` path (643 checked) finds exactly **two** with a
resolved exponent other than `32.0`: `tech_de_ra/track.vex` and its reversed
twin, each carrying one material -
`materials/adverts/nr_greyscale2colour.rcsmaterial` - at `300.0`, on 2 chunk
surfaces. Talon's Junction and the `assegai` ship (`ship.vex`) both resolve
**every** material slot to exactly `32.0`, so a same-camera before/after
screenshot of either is pixel-identical - not because the wiring is inert
(the numeric check above proves it threads a real value through end to end),
but because `32.0` is genuinely the disc's own commonest value and the
fallback happens to equal it on both assets checked visually. A screenshot of
`tech_de_ra`'s advert billboard would show a real difference and was tried,
not found: `--race --track /data/environments/tech_de_ra/track.vex` at the
start line does not frame it, and `oag-view --mesh ... --only
nr_greyscale2colour` matches nothing - `--only` filters on node name or
texture label, neither of which is the material name, and `--draws` finds no
`greyscale`/`advert` string either, so the 2 chunk surfaces this material
covers may not be reachable from this circuit's node tree at all (the
"unresolved node phenomenon" `rcsmodel_ground_truth.rs` already tracks
disc-wide). Not chased further - a two-chunk advert billboard is not worth
the node-tree archaeology this would take. An isolated 40.0-vs-32.0 shader
literal on the ship's hull
(not the real per-material path, a scoped one-line experiment made and
reverted the same session) did show a real, if subtle, difference - 0.06 % of
a frame's pixels (732 of 1,175,040, 1440x816), concentrated on
specular-highlight edges - which is the shape a correct wiring's visible
effect should have when it does land on a non-default value.

## Next Steps

- Check the Lambert sun diffuse fix against RPCS3 pixel-for-pixel - blocked, see Open above; pick up [an-rpcs3-capture-harness-a-reference-frame-paired.md](an-rpcs3-capture-harness-a-reference-frame-paired.md) first
- Decode the `patch fslot` to `const@slot` table (`rcsmaterial.md`, already flagged as owed) to close the `0`-bucket theory directly and let those materials resolve to a real value instead of the `32` fallback
- Trace what each surviving chain's `DP3`'s other operand is, to independently confirm `200`/`250`/`260`/`35` (or retract them) the way the ship's `40` already is
