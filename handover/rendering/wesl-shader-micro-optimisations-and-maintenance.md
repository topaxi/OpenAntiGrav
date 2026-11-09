---
status: ready-to-wire
titles: [all]
updated: 2026-11-09
summary: "WESL sweep: present-composite pow, Zone fetches 9 to 3, HUD fetches 4 to 1, bloom blur taps, fog branch; duplicate sRGB helpers, orphaned comments"
categories: [rendering]
---

# WESL shaders: micro-optimisations and maintenance, ranked

2026-11-09, `wesl-scout`, a read-only sweep of the 72 shader files under `crates/` (`mesh`, `post`, `fx`,
`render`, `present`, `game`, `view`, `shader-check`). **Every item is a reading of the source. Nothing was
built, run or timed**: the scout may not run cargo, and `deck-gpu` owns the Deck. Wins are estimates with the
reasoning shown. A lane that takes an item measures before and after with the command given.

Already written down and not re-reported: the track-solid folds (`slots_present`, the plain-draw pipeline, the
emissive, light-rig and Zone folds, byte-identical, Deck 8.2 to 7.65 ms HD) in
`docs/rendering/hires-gpu-levers.md`; HD's bloom floor in `docs/rendering/hd-gpu-cost.md`; the half-res blur rung
in `docs/rendering/motion-blur-half-rung.md`; the `lit_texel` split and the FSR 3 helper fold in
[wesl-shaders-semantic-errors-point-at-generated-text.md](wesl-shaders-semantic-errors-point-at-generated-text.md)
(its "828-line" figure is stale: `lit_texel` runs from `shade.wesl:68` to the end of the 1,581-line file).
Untimed HUD passes and bind-group churn: [render-perf-opportunities-2026-10-19.md](render-perf-opportunities-2026-10-19.md).

**The ranking is by expected Deck milliseconds at 4K, for the race type where the pass runs.** At 1080p and 4K the
Deck is GPU-bound (`docs/rendering/frame-thread-cpu.md`, "The ceiling on the Deck"), so a fragment-side saving
shows as frame time; at 800p the frame thread is the limit and these do not move the frame.

Measuring shaders: `OAG_RENDER_GPU_BENCH=100 OAG_RENDER_PERF=1 oag-game <image> --race --mode single_race --seed 1
--autopilot --no-audio --size WxH --ticks 2500 --screenshot out.png` on a `perf-probe` build, median of 3
processes (recipe: `docs/rendering/hires-gpu-levers.md`, "Method"). `RADV_DEBUG=shaderstats` prints VGPRs and
instruction counts per shader; `RADV_DEBUG=shaders` dumps the ISA, where `image_sample` lines count the fetches.
The picture check is `cmp` of two `--screenshot` runs, as the folds above used.

## Open

### Worth a lane now

1. **The present composite pays three `pow` it does not need on every output pixel.**
   `crates/present/shaders/upscale.wesl:65` and `:73` (`fs_main`).
   - What: `select(frame_color.rgb, srgb_decode(frame_color.rgb), grade.decode > 0.5)` evaluates `srgb_decode`
     (a `pow(_, 2.4)` on three channels) for every pixel and then discards it, because `select` is not a branch.
     `decode` is 0 whenever the sampler already decoded (the offscreen sRGB view; `upscale.rs:686` to `:701`).
     Then `pow(clamp(linear), grade.exponent)` runs on three channels, and the exponent is exactly 1.0 at the
     default gamma (`Gamma::NEUTRAL` is 100, `exponent()` is `100.0 / 100`, `display.rs:833`). Two uniform
     `if`s remove both. A neutral grade (exponent 1, brightness 1, decode 0) can return the tap untouched.
   - How often: per output pixel, every presented frame. 8.3 M pixels at a 4K output, **including the default
     docked Deck**, which draws 1906x1080 and upscales to 4K.
   - Picture: unchanged in intent. `pow(x, 1.0)` is `exp2(log2(x))` on a GPU, which is not exactly `x`, so up
     to 1 level of the 8-bit output could move; `cmp` will say how many pixels.
   - Win (estimate): 6 to 12 transcendental ops a pixel on 8.3 M pixels; at quarter rate on 512 lanes that is
     0.2 to 0.5 ms. The measured cost of our own present at a 4K output is +1.7 ms HD, +3.0 ms Pulse
     (`docs/rendering/hires-gpu-levers.md`, "The presented 4K output on the Deck"), so this is a tenth to a
     quarter of it.
   - Measure: `--presented` with `OAG_RESOLVE_BENCH=60` at `--size 3840x2160` (the "present composite" row of
     the span tables); the number to move is the composite's serialised CPU plus GPU time.
   - Risk: none to the simulation. Keep the clamp for a source that holds values above 1 (an extended-range
     surface) when the grade is not neutral. Effort: 30 minutes.

2. **A Zone race samples nine textures where three are needed.** `crates/mesh/shaders/zone.wesl:110`
   (`zone_sample`, four colour fetches) and `:206` (`zone_glow`, four band fetches), plus the visualiser lookup.
   - What: both functions fetch the track and scene textures at both layers and then `select` one, on the
     comment's argument that a branch would break uniform control flow. Only `zone_is_track(slots)` (a flat
     varying, so uniform over a 2x2 quad, the argument `lit_texel`'s own attribute makes) and `zone_inside(world)`
     (per fragment) choose. Branch on the flat bit to pick `zone_tex` or `zone_scene_tex`, and pass the layer as
     `select(1, 0, inside)`: a per-lane layer index is legal for `textureSample` and does not change the level
     selection. Four colour fetches become one, four band fetches become one. `zone_inside` is also evaluated
     three times per fragment (`zone_sample`, `zone_glow`, `zone_set`) and `zone_set` twice; compute them once.
   - How often: per fragment of every draw in a Zone race while `scene.zone.enabled`; never outside one. The
     comment above `zone_to_eye` in `shade.wesl:860` already measures the nine fetches at "about a fifth of the
     race pass on an integrated GPU".
   - Picture: byte-identical (the same texel is read, only the redundant ones are dropped).
   - Win (estimate): six of nine fetches gone, so about 13 % of the race pass in a Zone race, 1 to 3 ms at
     native 4K on the Deck HD, 0.3 to 0.8 ms at 1080p. Zero in Time Trial, which is what `bench-render` runs.
   - Measure: the two `--mode zone` races the fold lane used (HD and Pulse, tick 2,400), same span command as
     above; `RADV_DEBUG=shaderstats` for `image_sample` count and VGPRs; `cmp` against the binary before.
   - Risk: naga's and Tint's uniformity analysis on the new branch (use the same `@diagnostic(off,
     derivative_uniformity)` as `zone_glow`); check the web path validates. Effort: 1 to 2 hours.

3. **Every HUD and menu fragment fetches four textures and keeps one.** `crates/game/shaders/ui.wesl:224`,
   `:234`, `:235`, `:238` (`fs_main`).
   - What: the glyph atlas, sprite sheet, face atlas and buttons atlas are all sampled for every fragment, then
     `select`ed by `in.mode`, which is `@interpolate(flat)`. A text quad needs the atlas, a sprite quad the
     sheet, a face or button quad its own atlas. Branch on `in.mode` (quad-uniform) around each fetch, with
     `@diagnostic(off, derivative_uniformity)`.
   - How often: per fragment of the HUD and every menu draw. The HUD is drawn at presentation resolution
     (ADR-0036), so at a 4K output that is every HUD pixel at 4K, once a frame, one view or split.
   - Picture: byte-identical.
   - Win (estimate): three wasted fetches a fragment on a pass with tiny ALU. The textures are small and cached,
     so the saving is fetch issue and cache traffic: 0.05 to 0.3 ms at 4K with a full HUD. The HUD has no GPU
     span today, so the estimate has no measurement behind it.
   - Measure: first add a span (item 4 of render-perf-opportunities-2026-10-19.md), or compare `--presented` runs with and without the
     HUD; `RADV_DEBUG=shaderstats` on the `fs_main` of `ui.wesl` should drop from four `image_sample` to one to two.
   - Risk: none to the simulation. Effort: 1 hour.

4. **`lit_texel` fetches and computes things the plain track draw may not use; read the ISA first.**
   `crates/mesh/shaders/shade.wesl:503`, `:665` to `:716`, `:828`, `:452`, `:878`, `:904`.
   - What, four parts, each possibly already removed by the driver:
     (a) `second = material_sample(MATERIAL_LIGHTMAP, in.texcoord)` at line 503 is an anisotropic fetch of the
     lightmap *atlas* at the diffuse coordinate, read only under `slots` bits 1, 2, 4 and 1024 (and by
     `glow_second` when `glow_program` is on). `slots_present` removes it only if no vertex of the pipeline's
     draws carries those bits. The plain-draw split keeps bits 2 and 4 if any plain draw has them. A flat
     `if ((in.slots & (2u | 4u | 1024u)) != 0u)` makes it per draw.
     (b) The glass sheen block (`sheen_prelit` has a vec3 `pow` at 665, `grid_linear` a `pow` at 698, `sheen_rgb`
     a third at 709) runs for every fragment and is chosen by `select(shaded, sheen, ramp_sheen)` at 1235. It
     folds only when `slots_present` lacks bit 1024; sheen draws carry a row, so the plain pipeline lacks it, but
     the rich pipeline (hull, emissive rows) is told the model's whole OR.
     (c) `mag_glow_gamma = pow(mag_glow, 1/2.2)` (828) and `mag_glow = decode(cell_picture)` run on a zero vector
     in every pipeline that keeps bit 131072: six `pow` for a constant 0.
     (d) `normalize(scene.fog.camera - in.world)` is written three times (452, 878, 904) and fog takes
     `length(world - camera)` again.
   - Win (estimate): 0 to 3 % of the track pass, 0 to 0.8 ms at 4K HD on the Deck; zero if the compiler already
     folds them. The previous folds read as -20 to -50 %, so some of this was never folded by hand-written
     `select`s; that is why it is worth ten minutes of ISA before any edit.
   - Measure: `RADV_DEBUG=shaders` on the HD plain pipeline, count `image_sample` (expect albedo, lightmap atlas,
     and the shadow-free path only) and `v_log_f32` (each `pow` is one). Then edit and re-count.
   - Picture: byte-identical for (a), (c), (d); (b) is a pure guard. Risk: derivative uniformity on the new
     guards, as item 2. Effort: 1 to 3 hours including the ISA.

5. **HD's bloom blur takes nine taps where five bilinear taps give the same filter.**
   `crates/post/shaders/hd_bloom.wesl:265` to `:285` (`fs_blur`, weights `0.1 0.2 0.5 0.8 1.0 0.8 0.5 0.2 0.1`
   over 4.2).
   - What: merge each adjacent pair of taps into one bilinear fetch at the weighted centre: offsets +-3.333 and
     +-1.385 texels with weights 0.3 and 1.3, plus the centre with 1.0. `omega_bloom.wesl:34` to `:48` already
     does exactly this (offsets `1.3846154 3.2307692 5.0769229`). Nine fetches become five per pass, two passes
     per bloom.
   - How often: per pixel of the quarter-resolution ladder, two passes a frame, HD and Omega-style HD chain only.
   - **The picture changes.** Interior pixels agree to the sampler's weight precision (8 bits of the fractional
     weight, about 1/256 of a tap), the drawn-rectangle clamp acts on a merged coordinate instead of on each
     texel, and the 8-bit source rounds differently. `hires-gpu-levers.md` holds HD's bloom "to exactness", so
     this needs the maintainer's ACK and must be reported as a change, not an optimisation.
   - Win (estimate): HD bloom is 0.94 ms at 1080p and 2.98 ms at 4K on the Deck, the blur a part of it that has no
     span of its own; 9 to 5 taps on two passes saves 0.1 to 0.3 ms at 4K.
   - Measure: HD `OAG_RENDER_GPU_BENCH` bloom span at 4K, then `cmp` and an rmse of the two frames.
   - Effort: 1 hour plus the ACK.

6. **Fog computes both curves for every fragment.** `crates/mesh/shaders/fog.wesl:6` and `:13` to `:14`
   (`fogged`).
   - What: `length(world - camera)` (a square root) and `exp(-s * s)` both run, then `mix(linear, authored,
     scene.fog.curve)` keeps one. `curve` is a uniform that `uniforms.rs:305` documents as 1.0 for HD's curve and
     0.0 otherwise. Check it is never between; if so, `if scene.fog.curve > 0.5 { exp } else { linear }`. Same for
     `scene.fog.enabled`.
   - How often: per fragment of every opaque, blended and cutout draw, on every title.
   - Picture: byte-identical when `curve` is 0 or 1 (`mix` with an exact 0 or 1 returns an operand).
   - Win (estimate): one sqrt (HD) or one exp (the rest), about 6 to 10 of a 1,400 to 3,900 instruction shader:
     0.2 to 0.5 %, under 0.15 ms at 4K. Small and safe; do it with item 4.
   - Measure: the track solid span, 3 processes interleaved. It may not resolve; the `shaderstats` count will.
   - Effort: 30 minutes.

### Later

7. **Particle sprites compute the procedural blob too.** `crates/fx/shaders/psys.wesl:126` to `:138` (`fs_main`).
   `procedural` (a `length`, a `pow(shape, 1.2)`) and the sheet fetch both run, then `select(procedural, sprite,
   in.sampled > 0.5)`. `sampled` (`location(3)`, line 43) is a plain interpolated float, so a branch needs
   `@interpolate(flat)` on it (and on `cap`, `cell_v`). Picture byte-identical for a constant-per-quad value.
   Particles are additive overdraw, so this scales with the effect count: estimate 0.05 to 0.3 ms in an
   effect-heavy 4K frame. Measure with the `fx` span in a weapon-heavy race. Effort: 1 hour.

8. **Fetches kept alive by `mix` with a constant.** `crates/fx/shaders/exhaust.wesl:111` and `:117` fetch the HD
   trail-shape texture and a second flare tap for every exhaust fragment, then `mix(..., trail_shape)` with an
   `override` that is 0 on every title but HD. `if trail_shape != 0.0` guarantees the fetches are gone.
   `crates/post/shaders/omega_tonemap.wesl:205` fetches the distortion texture on every Omega frame even when
   `params.distortion` is zero (`uv = in.uv + distortion * d` is then `in.uv`). Both byte-identical. Estimate: one
   fetch of two to three on small passes, under 0.1 ms. Effort: 30 minutes for both.

9. **The vertex stage does fixed work for every vertex.** `crates/mesh/shaders/mesh.wesl:55` to `:60` builds a
   cofactor matrix (three cross products and a dot) and `spu_light.wesl:26` loops over every scene light, up to
   128, with a `length` and a divide each, for every vertex of every draw; the comment at `spu_light.wesl:16` says
   the per-chunk cull is not done. On HD at 800p the Deck's track solid span is 4.62 ms against 8.22 at 1080p and
   26.3 at 4K: a straight line through the three points leaves about 1.6 ms that does not scale with pixels, which
   is vertex, setup and draw cost. The light loop's share of it is **unknown**. Two options: a pipeline `override`
   `moving` that skips the cofactor for static geometry (node slot 0), and a per-draw light list on the CPU (the
   original's `LightCulling`; the sum is the same, as the comment states). Measure first: set
   `scene.spu_lights.count` to 0 in a diagnostic run and read the 800p track span on the Deck. Estimate 0 to 0.8
   ms flat across sizes, so it helps 800p and 1080p more than 4K. Effort: the override 1 hour, the CPU list 4
   hours and Rust.

10. **Opt-in passes with cheap tap or trig savings.** `omega_bloom.wesl:51` to `:59` (`fs_down`: eleven taps from
    six weights, mergeable to six bilinear taps; picture changes at rounding, same ACK as item 5).
    `photo_fx.wesl:80` to `:81` computes `cos(a)`, `sin(a)` and `sqrt(t)` per tap per pixel from the loop index
    alone; a `const` table of the unit-disc offsets removes them (the `round()` of the offset can differ in
    a rare tie, so `cmp`). `motion_blur.wesl` (`gather`, line 308 onward) divides by `constants.inv_size` and
    `rect_pixels` per tap; two uniforms (`size`, `rect_size / rect_pixels`) remove the divides but change the
    rounding order. All three are off by default, so the Deck's default profile does not pay them.

## Maintenance

11. **Five copies of the sRGB curve.** Decode: `upscale.wesl:44`, `photo_fx.wesl:101` and `beam.wesl:72`
    (`srgb_to_linear`); `target.wesl` has no copy (its `decode` is the 2.2 power, a different curve on purpose).
    Encode: `omega_tonemap.wesl:228` (`scanout_encode`) and `occlusion.wesl:69` repeat
    `colour.wesl:11` (`srgb_encode`). Add `srgb_decode` to `crates/shader-check/shaders/colour.wesl` and import it.
    Check: `omega_tonemap` and `occlusion` omit the `max(c, 1e-6)` guard that `srgb_encode` has; the lower branch
    is taken first, so the result is the same, but `cmp` it. Gain: one place to fix a curve bug. Measure: the
    build's `oag-shader-check` validation, then `cmp` on an Omega frame and an occlusion capture. 1 hour.

12. **Misplaced and dangling comments.** `shadow.wesl:5` to `:20` documents HD's engine-flare overrides, which
    live in `flame.wesl` (imported at `shade.wesl:7`); `shadow.wesl:23` to `:33` is the GE fog ramp, which belongs
    in `fog.wesl`, whose own `:10` points at "`Fog::curve` above" that is not there (nor at `types.wesl:342`);
    `shade.wesl:135` says "See this file's header" for the stand-in light and the header (line 1) says nothing
    of it. Move the first two, point the last three at `docs/rendering/` or `uniforms.rs:305`. Gain: a reader of
    the shadow module is not told about flames. No picture change; `oag-shader-check` and `just check-docs`. 30
    minutes.

13. **History in `.wesl` comments, under the ratchet.** `python3 -I scripts/check-comment-history.py --table` counts
    them per crate; `shade.wesl` has six dated lines (`2026-10-05`, `2026-08-20`, `2026-08-24`, `2026-10-21`),
    `types.wesl` four, `hd_bloom.wesl` and `bloom.wesl` two each, one each in `zone`, `psys`, `exhaust`, `beam`;
    "used to" / "prior version" narration at `shade.wesl:319`, `:340`, `:1054`, `:1123`; `psys.wesl:107`, `ui.wesl:84`, `:285`,
    `motion_blur.wesl:349`. Rewrite each as what and why with a `docs/` page citation, then lower the `oag-mesh`,
    `oag-post`, `oag-fx`, `oag-game` rows. The ratchet fails if a row is left above the count. 1 to 2 hours.

14. **Magic numbers with no page.** `shade.wesl:1552` (`0.45` and `(0.16, 0.18, 0.21)` for the `grade-sky`
    overcast: ours, unlabelled and uncited; cite the track-module doc that adds `sky_grey`); `65504.0` appears in
    four weapon branches of `shade.wesl` (a half-float max, labelled "chosen, not measured" once, at the first use,
    not at the others); `0.099975586` at the bomb halo; `0.340088` in `game/shaders/scene.wesl:46`. Add a named `const` and
    the citation. 30 minutes.

## Checked, no finding

- **Every entry point is loaded.** All 146 `@vertex`/`@fragment`/`@compute` names (minus the 32 `vs_main` and 23
  `fs_main`, which share a name by convention) appear as a string literal in Rust. No dead entry point.
- **The depth prepass pays no fragment cost.** `fs_depth_only` writes a constant; its unused varyings are linked
  out.
- **`shadowed` is skipped where it should be**: `receives_shadow` is an `override`, and the track's pipelines
  never call `sun_occlusion` (`shade.wesl:351`).
- **`hd_bloom.wesl` `reduce_sum` and the motion blur's tile passes** are already unrolled and bounded.
- **Duplicated `fs_blur`, `fs_copy`, `fs_composite` names** are separate passes with separate bindings, not
  copies. `drawn()` (`hd_bloom`, `hd_zoom`, `omega_tonemap`) is one line over three different `Constants`
  structs; sharing it needs the fields passed as arguments and gains nothing.
- **f16.** No shader here would gain from f16 that the Deck's RADV does not already narrow, and the web path
  (Tint) needs `shader-f16` per device. Not recommended.

## Next Steps

1. Item 1 and item 6 as one 1-hour change in `present` and `mesh`, `cmp` before and after, quote the 4K
   composite span (`OAG_RESOLVE_BENCH=60 --presented`) on the host and, when the Deck is free, on it.
2. Item 2 (Zone fetches) with the two Zone races, then item 3 (HUD fetches) after a span for the HUD exists.
3. Item 4: dump the ISA of the HD plain pipeline with `RADV_DEBUG=shaders` first; edit only what it shows.
4. Ask the maintainer about item 5 (bloom blur taps change the picture at rounding) before any edit.
5. Maintenance 11 to 14 as one docs-and-comments lane; run `oag-shader-check` (the build), `just check-docs` and
   the comment-history ratchet.
