# Rendering and shader review, 2026-11-23

A review pass over the GPU-facing rendering stack: `oag-gpu` (device vocabulary,
writes, deferred upload, reaper, split finish, timing), `oag-shader-check` (WESL
linking and naga validation), `oag-shader-check/shaders` (the shared `oag_shaders`
module: `target.wesl`, `colour.wesl`, `fullscreen.wesl`, `scene_vertex.wesl`),
`oag-mesh` shaders and `mesh_render` host code, `oag-render` shaders (caster,
ghost, loading, shadow, occlusion, probe_blend), `oag-post` (bloom, HD bloom,
tonemap, composites, FSR1/FSR3, motion blur, screen filters), `oag-fx`,
`oag-hud`, and `oag-present` shaders.

The shader layer (`.wesl`/`.wgsl`, roughly 6.5k lines in `oag-post` alone plus
`shade.wesl` at 2.3k lines) and `oag-gpu` were read in full. The host crates
(`mesh_render`, `post`, `render`, `fx`, `hud`, `present`, about 57k lines of Rust)
were reviewed by targeted inspection - override plumbing, bind-group lifetime,
uniform layout, panic paths and shader/host contract boundaries - not line by
line. Findings are graded with the same rubric as
[2026-08-18-code-review.md](2026-08-18-code-review.md).

Nothing here was fixed on the way past; this is findings only.

## What is already strong

Worth saying before the findings, because the review kept tripping over good
design rather than defects:

- Every WESL shader is linked and naga-validated at build time
  (`oag_shader_check::check_dir` / `link_each`), and runtime-authored mod shaders
  go through `shader_file.rs`'s validation before device creation. Generated WGSL
  lands in `OUT_DIR` and is `include_str!`'d, so a shader that fails validation
  cannot ship silently.
- `Writes::Log` preserves per-view queue-write ordering for split-screen views
  recorded concurrently, and `deferred_upload` chunks at 256 KiB while parking
  `Arc<dyn Bytes>` instead of copying - both correct and non-obvious.
- The `Reaper` polls without blocking on `Device::poll`'s snatch lock, and
  `split_finish` is opt-in behind a uniform-core topology check. Both are
  documented tradeoffs, not accidents.
- `target.wesl` is the single place the encoded/linear/packed colour contract
  lives, with `linear_out` derived from the target format at pipeline build
  (`mesh_render::is_linear_target`). The 5-6-5 pack paths distinguish
  linear-light storage from gamma-domain storage, which is the kind of detail
  most reimplementations get wrong.
- The codebase is unusually honest about provenance: comments consistently mark
  measured behaviour apart from "chosen, not measured", which makes the review
  faster and the approximations auditable.

## Findings

### R1. Implicit-derivative texture sampling in non-uniform control flow - suppressed, not resolved

**Impact**: Medium - wrong mip selection or driver rejection on stricter backends.
**Priority**: P2. **Confidence**: 85. **Faithfulness**: n.a. **Effort**: M.

WGSL derivative builtins (`dpdx`, `dpdy`, and therefore implicit-LOD
`textureSample`) are only defined in uniform control flow; elsewhere the result
is undefined and Tint/Dawn-style validators flag it. The mesh shaders hit this
repeatedly and answer it with `@diagnostic(off, derivative_uniformity)` rather
than restructuring:

- `crates/mesh/shaders/texture_lod.wesl` - `material_sample` is a plain
  `textureSample` (implicit derivatives) and is called from
  `omega_snow_normal`/`omega_rock`/`water_normal` after early returns and inside
  per-fragment conditionals.
- `sample_at_slope_level` and `uv_gradients` carry the same suppression with
  comments explaining why ("`lit_texel` calls it after an early return on a flat
  varying").
- `holo.wesl`'s `holo_billboard` and `own_shield.wesl`'s facet normal
  (`dpdx`/`dpdy` of `in.world`) are in the same class.

The project clearly knows the rule - the comments cite Chrome's Tint judgement -
and has already built the correct tool for the hardest case
(`material_sample_grad` with explicit gradients for Omega's marched panel-map
coordinates). The residual risk is backend-dependent: on Vulkan the
`derivative_uniformity` diagnostic is a warning and the hardware returns
whatever the quad's derivatives happen to be, which is usually fine but is not
guaranteed; on a stricter frontend it is a hard error. Every pipeline variant
that keeps an implicit-LOD `material_sample` call behind non-uniform flow is one
backend away from a wrong-LOD or rejected shader.

Suggested direction, not a demand: audit which `material_sample` call sites can
actually execute in non-uniform flow per pipeline override, and convert those to
`material_sample_grad`/`textureSampleGrad` with `uv_gradients`, the same pattern
already proven for the panel march.

### R2. Emissive-table reads can index past the 64-row table by construction

**Impact**: Medium - silent black/wrong output if a model ever places a program
entry in the last rows; no validation catches it.
**Priority**: P2. **Confidence**: 80. **Faithfulness**: n.a. **Effort**: S.

`emissive_tint`/`emissive_scale` index `emissives.tint_offset[entry]` and
`emissives.scale[entry]` where the arrays are
`EMISSIVE_LIMIT = 64` rows (`crates/mesh/src/mesh/rcs/emissive.rs:419`,
`crates/mesh/src/mesh_render/tables.rs:169-172`). The shader-side `entry` is a
6-bit field (`(in.slots >> 24u) & 63u` in `holo.wesl`), so it can name rows
0-63. But consumers read *ahead* of the entry:

- `holo.wesl` `holo_billboard`: `emissive_tint(entry + 1u)`,
  `emissive_scale(entry + 1u)` - entry 63 reads row 64.
- `water.wesl`: `emissive_tint(entry + 1u + ...)`, `emissive_tint(entry + 2u)`
  - entry 62 or 63 reads rows 64-65.
- `omega_snow.wesl`/`omega_rock.wesl`: `entry + 1u`, `entry + 2u` - same.

WGSL robust buffer access makes an out-of-bounds uniform read return zero
rather than fault, so the failure mode is a black hologram or dead water normal
- silent, and only for the models unlucky enough to pack an entry into the
tail. The host side shows the invariant is known but only partially enforced:
`glass_sheen.rs:138` and `damage_lights.rs:36` guard with
`table.len() + 1 < EMISSIVE_LIMIT` (reserving one row), but water needs *two*
rows of lookahead (`entry + 2`), and nothing in the shader clamps
`min(entry + 2, 63)`. Either the table builder must guarantee tail-room per
program kind (and say so where the +2 reads happen), or the shader should clamp.
Today it is a convention held up by the asset packer, not by the code that
reads it.

### R3. `Bloom::stamp_glow` allocates a fresh bind group every call

**Impact**: Low-Medium - a per-frame `create_bind_group` in a hot HUD path,
inconsistent with the caching used everywhere else in the same file.
**Priority**: P2. **Confidence**: 95. **Faithfulness**: n.a. **Effort**: S.

`crates/post/src/bloom/stamps.rs` (~line 76) calls
`device.create_bind_group` inside `stamp_glow` on every invocation, while the
scene path caches its group (`bright_group.get(scene, || scene_group(...))` in
`bloom.rs`). `stamp_glow` runs once per frame whenever a boost or absorb bar is
up, so this is a steady stream of bind-group allocations the driver must
reclaim. The group depends only on `target` (the HUD target view) and the
sampler/layout, all stable across frames - it can be cached the same way, or at
worst pooled. Small win, but it is exactly the kind of allocation the rest of
this codebase deliberately avoids.

### R4. `mesh_render` uniform packing reuses "padding until <date>" slots, and the dates have passed

**Impact**: Low - stale comments and a layout that is harder to audit than it
looks; the fields are load-bearing scalars riding in former padding.
**Priority**: P3. **Confidence**: 95. **Faithfulness**: n.a. **Effort**: S.

`crates/mesh/src/mesh_render/uniforms.rs` packs `sun_occlusion_layer`,
`model_clock`, `model_colour`, `hull_scale`, `env_layer`, `env_tint`,
`sky_grey` into slots the comments describe as "Padding until 2026-10-05",
"2026-10-07", "2026-10-17", "2026-10-30". All four dates are in the past
(today is 2026-11-23), so the comments now read as though the fields are still
reserved padding. They are not - each is a live per-draw scalar the shaders
read. The reuse itself is fine (it keeps the uniform block size stable for the
four `.wgsl` mirrors and the asset viewer), but the comments should be rewritten
to describe what the fields *are*, not when they stopped being padding. This
also brushes against `just check-comment-history`'s no-dates-in-comments rule;
the ratchet grandfathered these, but they are exactly the kind of dated comment
that rule exists to prevent accumulating.

### R5. `target_in` assumes the packed-565 card is sampled through an sRGB-typed view

**Impact**: Medium - a wrong view format silently breaks the behind-glass grab
(and anything else calling `target_in`) on packed targets.
**Priority**: P2. **Confidence**: 75. **Faithfulness**: n.a. **Effort**: S.

`crates/shader-check/shaders/target.wesl:128` - `target_in` decodes with
`srgb_decode` only when `encoded_out` is set; for `packed_out` targets it
returns `rgb` unchanged. That is correct *only* because the 5-6-5 card's texture
view is sRGB-typed so the hardware already decoded the stored bytes on fetch
(the comment at `pack_565` explains this). If any pipeline ever binds the same
card through an `Unorm` view - a debug view, a different crate, a future read
path - `target_in` hands back gamma-encoded values where linear was promised,
with no shader-side signal. The contract is real but lives entirely in view
format choice at bind time. A `debug_assert` or a named constant at the view
creation site ("this view MUST be `Rgba8UnormSrgb`") would pin it; today it is
convention.

### R6. Colour-space contract is per-pipeline overrides; mismatched combinations compile fine

**Impact**: Low-Medium - a pipeline built with `linear_out` but without the
matching `encoded_out`/`srgb_texels`/`square_decode`/`packed_*` produces
plausible-but-wrong colour, and nothing validates the combination against the
bound target.
**Priority**: P2. **Confidence**: 75. **Faithfulness**: n.a. **Effort**: M.

`target.wesl` exposes five interacting overrides (`linear_out`,
`square_decode`, `srgb_texels`, `encoded_out`, `packed_out`, `packed_gamma`).
`linear_out` is derived from the texture format
(`mesh_render::is_linear_target`), but the rest are per-title/per-model choices
set at pipeline build. Several combinations are meaningless together (e.g.
`encoded_out` without `linear_out`, `packed_out` on a float target) and the WGSL
compiler will happily build them - the `select`/`mix` structure means wrong
values, not errors. The host side presumably has a table mapping (title, model
kind, target format) -> override set; if that table is the only enforcement,
it is worth a `debug_assert` or a unit test enumerating the legal combinations,
because the failure is a subtle colour shift that survives review.

### R7. `mod_shader.wesl` engine-texture UV divides by `clip.w` with no near-plane guard

**Impact**: Low - mod-facing API; a backdrop/engine fill behind the engine
camera samples mirrored scene texels instead of returning transparent.
**Priority**: P3. **Confidence**: 85. **Faithfulness**: n.a. **Effort**: S.

`crates/fx/shaders/mod_shader.wesl` - `engine_uv` computes
`clip.xy / clip.w * vec2(0.5,-0.5) + 0.5` and `engine_texture` samples with it.
For world positions behind the engine camera `clip.w <= 0`, the projection
mirrors the UV into valid-looking texture space, so `engine_texture` returns
arbitrary scene content rather than the documented "transparent black with no
picture". `fs_main`'s own `screen_uv` has the same shape but for the *view*
camera where the rasterizer already clipped the triangle, so only the
engine-texture path (which projects arbitrary world points) is exposed. A
`clip.w > 1e-4` guard returning `vec4(0)` matches the module's own contract.

### R8. `mod_fragment` is compiled into three fragment entries; backdrop fills pay it twice

**Impact**: Low - up to ~3x fragment ALU for mod shaders on backdrop fills;
alpha is also passed through unclamped.
**Priority**: P3. **Confidence**: 85. **Faithfulness**: n.a. **Effort**: M.

`mod_shader.wesl` defines `fs_main`, `fs_backdrop` and `fs_fringe`, all of which
call the mod's `mod_fragment`. A `fill = "backdrop"` draw therefore runs the
mod's full shader in the backdrop pass *and again* in the fringe pass for the
sub-`BACKDROP_ALPHA` texels (and `fs_main` for ordinary fills). For a mod shader
that is expensive - raymarching, many taps - this is a real cost multiplier the
mod author cannot see. It may be inherent to the two-pass design (the fringe
needs the shaded colour to decide coverage), but it is worth documenting in the
mod-shader contract (`docs/architecture/wasm-extensions/mod-shaders.md`) so
authors know `mod_fragment` can run 2-3x per pixel. Separately, `fs_main`
returns `shaded.a` unclamped while `rgb` goes through `scene_rgb`'s clamp - an
out-of-range alpha from a mod reaches the target verbatim; probably intended,
but asymmetric.

### R9. `sun_delta`/`sun_known`/`shadow_exempt` are order-coupled private varyings

**Impact**: Low-Medium - the shadow-subtraction path depends on `lit_texel`
having run before `shadowed` in the same invocation; a refactor that reorders
or bypasses silently zeroes the correction.
**Priority**: P2. **Confidence**: 80. **Faithfulness**: yes (it exists to match
the original's shadowed redraw). **Effort**: S.

`crates/mesh/shaders/bindings.wesl` declares `var<private> sun_delta`,
`sun_known`, `shadow_exempt` - per-invocation state written by `shade::lit_texel`
and read by `shadow::shadowed` "within one fragment". The comments are clear
about the contract, but nothing enforces it: any code path that calls
`shadowed` without the matching `lit_texel` (a new material program, a future
refactor, a pipeline variant that splits the calls) reads zeros and falls back
to the multiply - which is the *wrong* answer the mechanism exists to fix, and
it fails silently. Returning the delta alongside the colour (or funnelling both
through one struct) would make the coupling structural instead of conventional.
Cheap insurance for a subtle correctness path.

### R10. `depth_layer_shadow` assumes a square, full-map depth texture; rim taps can read clamped border

**Impact**: Low - edge texels inside the `CASCADE_RIM` blend zone can compare
against the clamp-to-edge border depth.
**Priority**: P3. **Confidence**: 75. **Faithfulness**: n.a. **Effort**: S.

`crates/mesh/shaders/shadow.wesl:176` - `texel = 1.0 / textureDimensions(
shadow_depth_tex).x` uses only the width, and `coords.xy` is used directly as
the sample coordinate, implicitly assuming the depth map is square and spans
[0,1] in both axes. Both hold today; if the atlas ever becomes non-square or
the layer occupies a sub-rectangle, every PCF tap lands wrong with no
validation. Additionally the four taps offset by `±0.5` texel can cross the
layer edge while `near_weight`/`far_weight` is still > 0 inside the rim blend
zone, sampling the clamped border depth - the very thing `depth_layer_coords`'s
comment says must never happen ("Outside reads as uncovered, never as the
border"). The weight gate covers `coords.w <= 0` but not a sample that starts
inside and taps outside. A `textureLoad`-based tap or an explicit inside-check
per tap would close it; today it is a small, probably-invisible edge artifact.

### R11. `fs_stamp`'s loop bound is unclamped shader-side

**Impact**: Low - robustness only; the host clamps to `MAX = 20` today.
**Priority**: P3. **Confidence**: 95. **Faithfulness**: n.a. **Effort**: S.

`crates/post/shaders/bloom.wesl:113` - `for (var i = 0u; i < count; i++)` where
`count = u32(constants.stamp_meta.x)` indexes `constants.stamps[i]` (a
20-element array). `stamp_glow` clamps `stamps.len().min(MAX)` before writing,
so this is safe on every current path, but the shader trusts the uniform
absolutely: a `min(count, 20u)` costs nothing and removes the reliance on every
future writer of `Constants` remembering the bound.

### R12. `gpu::texture` upload path panics on formats with no copy size

**Impact**: Low - runtime panic for depth/astc-like formats if ever passed;
callers appear to guard today.
**Priority**: P3. **Confidence**: 90. **Faithfulness**: n.a. **Effort**: S.

`crates/gpu/src/texture.rs:39` - `format.block_copy_size(None).unwrap_or_else(
|| panic!(...))` after asserting only `block_dimensions() == (1,1)`. Any format
that is uncompressed but has no single-texel copy size (depth formats,
`Rgb10a2Unorm` variants without copy size on some backends) reaches the panic at
runtime rather than a typed error. Given this is a shared upload helper, a
`Result` or a documented "callers must pre-filter" contract would be safer than
a panic in a library crate. Same class: `writes.rs:168`'s
`.expect("a written aspect has one block size")` - internal invariant, lower
risk.

### R13. `present`/`render`/`fx`/`hud` host code: targeted review only - residual risk unquantified

**Impact**: Unknown. **Priority**: P2 (as a follow-up task, not a defect).
**Confidence**: n/a. **Faithfulness**: n.a. **Effort**: L.

The host-side Rust in `crates/mesh/src/mesh_render*` (~4.6k lines),
`crates/post/src` (~10k), `crates/render/src` (~5k), `crates/fx/src` (~4k),
`crates/hud/src` (~5k) and `crates/present/src` (~3k) was reviewed by targeted
inspection - bind-group caching, uniform `repr(C)`/`bytemuck` layout vs the WGSL
mirrors, panic/`expect` paths (78 hits, almost all test code or documented
invariants), and the shader/host contract boundaries above. That is enough to
surface R2, R3, R5, R6 and R12, but it is not a line-by-line review of ~30k
lines of pipeline construction, pass scheduling, atlas management and HUD
batching. Specific areas worth a dedicated pass next time: `mesh_render`'s
pipeline-variant cache key (the override set is large and a missing key
dimension silently reuses the wrong pipeline), `render/src/shadow.rs`'s atlas
allocation vs the shader's 8-map loop bound, `post/src/fsr3.rs`'s resource
lifetime across resizes, and `present/src/upscale.rs`'s dynamic-resolution
state machine.

## Summary table

| ID | Finding | Impact | Priority | Confidence | Effort |
| --- | --- | --- | --- | --- | --- |
| R1 | Implicit-LOD sampling in non-uniform flow, diagnostic suppressed | Medium | P2 | 85 | M |
| R2 | Emissive table OOB by construction (entry+1/+2 into 64 rows) | Medium | P2 | 80 | S |
| R3 | `stamp_glow` bind group allocated per call | Low-Med | P2 | 95 | S |
| R4 | "Padding until <date>" comments, dates elapsed | Low | P3 | 95 | S |
| R5 | `target_in` relies on sRGB-typed view for packed 565 | Medium | P2 | 75 | S |
| R6 | Colour-space override combinations unvalidated | Low-Med | P2 | 75 | M |
| R7 | `engine_uv` divides by `clip.w` unguarded | Low | P3 | 85 | S |
| R8 | `mod_fragment` runs up to 3x per pixel; alpha unclamped | Low | P3 | 85 | M |
| R9 | `sun_delta` private-varying call-order coupling | Low-Med | P2 | 80 | S |
| R10 | Depth-layer PCF: square-map assumption, rim border taps | Low | P3 | 75 | S |
| R11 | `fs_stamp` count unclamped shader-side | Low | P3 | 95 | S |
| R12 | `gpu::texture` panics on copy-size-less formats | Low | P3 | 90 | S |
| R13 | Host crates reviewed by targeted inspection only | Unknown | P2 | n/a | L |

## Suggested order

1. **R2** - smallest fix with the largest silent-failure surface: clamp or
   reserve the emissive lookahead rows.
2. **R3** - cache the stamp bind group; ten lines.
3. **R9** - make the sun-delta handoff structural before the next material
   program lands.
4. **R5/R6** - pin the view-format and override-combination contracts with
   assertions; both are convention-only today.
5. **R1** - the real work: convert the non-uniform `material_sample` call sites
   that survive pipeline overrides to explicit gradients.
6. **R13** - schedule the dedicated host-side pass; the shader layer is in good
   shape and the remaining risk is concentrated in pipeline/cache/lifetime
   code this review only sampled.

## Lead verification

Each finding was checked against the code on `main` at `0e8a38c80` before any fix lane was drawn.

| ID | Verdict | Evidence |
| --- | --- | --- |
| R1 | Confirmed as a risk, needs an audit | The `derivative_uniformity` suppressions and the implicit-LOD `material_sample` are as described. Whether any call site actually runs in non-uniform flow on a shipped pipeline variant is not yet shown. |
| R2 | Not confirmed | Every allocator that hands out an entry read with lookahead reserves the rows it reads. Water's `lay` reserves the whole block (`table.len() + block.len() < EMISSIVE_LIMIT`, `crates/mesh/src/mesh/rcs/water.rs`). `holo_billboard.rs` and `window_glass.rs` reserve `+ 2`, and `psp2/snow.rs` reserves `count`. `MaterialTables::of` writes table index `i` to row `i + 1`. A shader-side clamp would be insurance only. |
| R3 | Confirmed | `crates/post/src/bloom/stamps.rs` calls `create_bind_group` on every `stamp_glow`. The group depends only on `target`, the layout, the sampler and the lane's buffer. **Fixed in `ca2fa7cee`: `stamp_glow` keeps its bright-pass group per target view (`the_stamp_group_is_kept_until_the_target_view_changes`).** |
| R4 | Confirmed | Four `Padding until <date>` doc comments in `crates/mesh/src/mesh_render/uniforms.rs` describe live fields. **Fixed in `2127bff55`: each slot's comment says what it is; the `check-comment-history` ceiling for `mesh` lowered by four.** |
| R5, R6 | Design hardening, not defects | No mismatched view or override combination was found. Assertions would pin conventions that currently hold. |
| R7 | Confirmed | `engine_uv` in `crates/fx/shaders/mod_shader.wesl` divides by `clip.w` with no guard. **Fixed in `d649203b7`: `engine_uv` gives a point at or behind the camera's plane a coordinate far outside the picture and `engine_texture` reads it as transparent black (`a_point_behind_the_engine_camera_reads_transparent_black`).** |
| R8 | Confirmed as documented cost | The three entries call `mod_fragment`. The fix is a note in `docs/architecture/wasm-extensions/mod-shaders.md`. **Fixed in `d565c81a1`: documented in `mod-shaders.md` and the mod author skill. A backdrop fill runs `mod_fragment` twice per pixel (`fs_backdrop`, then `fs_fringe`), an ordinary draw once; the three entries are alternatives, so "up to three" was too many.** |
| R9 | Confirmed as coupling, not a current bug | All current `shadowed` callers run after `lit_texel`. |
| R10 | Plausible, unverified | The four `textureSampleLevel` taps at half a texel are as described. Whether a tap crosses the layer edge while the rim weight is non-zero needs a numeric check. |
| R11 | Confirmed | `bloom.wesl` loops to `u32(constants.stamp_meta.x)` unclamped. The host clamps today. **Fixed in `44d8c9e9e`: `count` is `min(.., STAMP_LIMIT)`, the shader's name for the host's `stamps::MAX`.** |
| R12 | Confirmed, low value | A `panic!` in `crates/gpu/src/texture.rs` on a format with no copy size. Every current caller passes a colour format. **Fixed in `8396b26d5`: a documented precondition and `debug_assert!` in place of the `panic!`; the callers are many and all pass colour formats, so a `Result` at each would be the larger change.** |
| R13 | Not a finding | A suggestion for a future host-side review pass. |
