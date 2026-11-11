---
status: ready-to-wire
titles: [all]
updated: 2026-11-11
summary: "Duplicate-code refactors ranked from jscpd; candidates 1 to 6 landed (4, 5 with a remainder), five later; Omega takes HD's ALWAYS_ON (lane running)."
---

# Duplicated code: ranked refactor candidates

A read-only scout sweep (jscpd, then reading both sides of the top groups).
Nothing here changes behaviour yet. Each candidate is a refactor: the
player sees no difference, so the win is one definition where there are
now copies, and fewer places for them to drift apart.

Regenerate the raw numbers (about 3 seconds, from the repository root):

```sh
npx -y jscpd crates --format rust --ignore '**/tests/**,**/target/**' \
  --min-lines 10 --reporters json --output target/jscpd --silent
jq '[.duplicates[].lines] | add' target/jscpd/jscpd-report.json
```

Paths in jscpd output are relative to `crates/`. Every path below carries
the `crates/` prefix. Line counts are jscpd's `lines` field for the clones
named, unless the text says "read". Confidence on each size estimate is a
scout estimate (0-100), capped at 60.

**Ranking rule.** No candidate has a measured player-visible win. Each is
ranked by copies removed times drift risk, divided by effort. Drift is
real: the HD and Omega `ALWAYS_ON` lists already differ (see the title
section below).

## Open

### Worth a lane now (ranked, highest first)

1. ~~**`sprite_draw` is copied byte for byte between two ui-screens files.**~~
   **Landed (dedupe-ui-screens).** One definition, `oag_ui::screen::Image::sprite_draw`
   (`crates/ui/src/screen/widgets.rs`, next to `Fill::draw`); the campaign and
   endrace copies are gone and every call site (campaign, endrace, track
   panel, HD medal crop) calls the method. No re-export. Covered by a new
   `oag-ui` test (`screen::tests::sprite_draw_samples_a_sub_rect_and_tiles_past_the_placed_rect`)
   plus the screens' existing tests (campaign draw/HD medal, endrace draw,
   track panel `TiledSprite`) and the disc-backed `track_panel`,
   `hd_endrace` and `selection_screens` ground truths. jscpd no longer
   reports the 35-line pair.

2. ~~**Cutout and transparent draw loop is copied in four places.**~~
   **Landed** (branch `dedupe-gpu-mesh`). One definition,
   `oag_mesh::mesh_render::draw_cutout_then_transparent`
   (`crates/mesh/src/mesh_render/draw_groups.rs`): takes the pass, the model,
   a `CutoutPipelines`, a `TransparentPipelines` and a `bind` closure, so each
   caller keeps its own out-of-range texture-slot rule (`capture.rs` and the two
   game files fall back to slot 0, `orbit.rs` clamps to the last bind). `Built::cutout_set`
   and `Built::transparent_set` build the two tables for the game callers. The
   four copies are gone from `crates/mesh/src/capture.rs`, `crates/view/src/orbit.rs`,
   `crates/game/src/hud_countdown.rs` and `crates/game/src/preview.rs`; the
   opaque `Model::draws` loop stays at each caller, which sets the opaque
   pipeline first.
   - Pinned by a new `oag-mesh` test, `cutout_then_transparent::each_of_the_cutout_and_transparent_lists_paints_its_own_half_of_the_frame`
     (a quad in each list through `capture_pixels_from`; it fails if either
     loop is dropped), and a before/after `oag-game --race` capture of Pulse's
     countdown at four ticks, byte-identical (the `hud_countdown` path).
     `orbit.rs` is windowed-only with no test and was not measured.
   - jscpd: the 42-line `hud_countdown`/`preview` pair and the 11-line
     `hud_countdown`/`capture.rs` pair are gone. What it still pairs between
     those four files (9 pairs, 11 to 21 lines) is the surrounding setup:
     pass state, the `bind` closure and the opaque loop. Not folded.

3. ~~**Post seat-lane code is copied between the Omega files and the bloom pair.**~~
   **Landed** (branch `dedupe-post`). What is one definition now, in `oag-post`:
   - `seat_lanes.rs`: a generic `SeatLanes<L>` (the per-seat lane store and its
     build counter) behind `hd_bloom`, `omega_tonemap` and `wo_composite`. Their
     public `seat_lanes`, `retire_seat(s)` and `seat_lane_builds` delegate to it.
   - `pass.rs`: `fullscreen_pipeline` (the `render` closure of Omega, WO, Omega's
     bloom levels and the PSP bloom), `draw` and `draw_with` (the loading
     full-screen pass, and the same with a chosen load op), `clear`, and the
     `Target` that Omega and WO each carried.
   - `bloom.rs`: `ensure_view_lane`, `lane_group`, `scene_group`,
     `scratch_pass` and `Frame::uv`, which `Bloom` and `Ps2Bloom` share for
     `prepare_view`, their bind groups and their three scratch passes. Both
     composites go through `pass::draw` with the viewport clamped by the caller.
   - jscpd on `crates/post`: 182 clones, 3,001 duplicated lines before; 165, 2,642
     after. Gone: the 52-line `retire_seat`/`draw` pair, the 33-line `Target`
     pair, the 32-line `render` closure pair, the 24-line bind-group pair.
   - Tests that pin it: `wo_composite::tests::split::*` and
     `ps2_bloom::tests::a_view_lane_draws_what_the_whole_lane_draws_for_its_rectangle`
     (both added first; neither path had a test through its view API), plus the
     existing `omega_tonemap::split_tests`, `bloom::tests` and `hd_bloom` split tests.
   - **Left as they were, on purpose.** `bloom::Target` and `motion_blur::Target`
     differ from `pass::Target` (no `max(1)`, no `COPY_DST`; the first returns a
     `Result`), so merging them would change texture usage or the zero-size case.
     `Bloom` and `Ps2Bloom` keep eager `Vec<Lane>` view lanes (a lane below `view`
     is built too, so `composite_view` finds it), which `SeatLanes` (builds only the
     asked seat) would change; the PS2 pipeline closure takes shader constants.
     What jscpd still pairs there is the public method signatures
     (`prepare_view`, `prepare_in`, `composite_view`, `composite_in`: 31 and 23
     lines) and `Chain::new`'s bind-group layout entries, `run_placed` skeletons
     between `omega_tonemap.rs` and `wo_composite.rs` (14-20 lines each).
     Folding those would take a macro or a trait for no further saving.

4. ~~**The single-level RGBA texture upload and clamp sampler are copied in
   about 15 places across fx, post, render, mesh and game.**~~
   **Landed, with a remainder** (branch `dedupe-gpu-mesh`). New in `oag-gpu`
   (`crates/gpu/src/texture.rs`): `upload_2d` (any uncompressed colour format,
   `TEXTURE_BINDING | COPY_DST`, one mip, a zero extent becomes one texel in the
   texture, the write and the row stride), `upload_rgba8` (`Rgba8Unorm`) and
   `clamp_linear_sampler` (clamp on every axis, `Linear` mag and min).
   `FlareTexture::upload` (`oag-fx`, now `pub`) is the one call every fx and render
   consumer of a `FlareTexture` makes. Callers moved: `exhaust::FlareTexture`
   (both copies in one file), cloud, beam, mist, weapon quads, ghost static,
   the loading glow strip, the render shadow blob, and `mesh_render::shadow_map`
   (the sky probe upload and four 1x1 placeholders). The clamp sampler replaces the
   identical one in cloud, weapon quads, loading, the shadow blob and `shadow_map`.
   - Pinned by `oag-gpu` `texture::tests` (a 3x5 image read back row for row by
     a compute shader, the descriptor's size, format, usage, mips, samples and
     dimension, 0x0 to 1x1, a one-channel format's stride) and a before/after
     `oag-game` capture: Pulse race at three ticks, the Pulse and HD loading
     screens, all byte-identical.
   - **Left, differs:** `crates/game/src/render/resources.rs` `upload_rgba` creates
     the texture clamped but writes the unclamped extent (a 0-wide image is a
     zero-extent write there, an error here); `crates/fx/src/mod_shader/pipeline.rs`
     `texture` has no clamp and `rows_per_image: None`; `crates/mesh/src/mesh_render/texture.rs`
     `upload_rgba` is the mip chain. The depth placeholders in `shadow_map`
     (`RENDER_ATTACHMENT`, nothing written) are not uploads.
   - **Left, `crates/post` was out of the lane:** `smaa::upload` (it also
     returns a view) and six identical clamp-linear samplers (`bloom`,
     `hd_bloom`, `hd_zoom`, `omega_tonemap`, `ps2_bloom`, `wo_composite`), plus
     `fxaa`, `motion_blur` and `smaa`'s. They can call `oag_gpu::texture` now.
     The samplers in `fx::psys`, `game` (render, face, backdrop, scene, video)
     and `present::upscale` set clamp and `Linear` through `..Default::default()`
     (clamp is the default) and are the same state; they are under jscpd's ten
     lines and were not touched.
   - jscpd: 15 non-test `write_texture` clones to 2, both against the new helper
     (`resources.rs` and `smaa.rs`, the two left above).

5. ~~**`Model` is built by hand as a literal in several places.**~~
   **Landed, with a remainder** (branch `dedupe-gpu-mesh`).
   `Model::from_draws(label, vertices, indices, draws, centre, radius, mesh_count)`
   (`crates/mesh/src/mesh.rs`) is the one literal of a synthetic model; `Model::none`
   is `from_draws` with nothing, and `render::track::build_model`,
   `render::collision::build_model`, `mesh::merge` and `mesh::sky_cube::build`
   call it (the sky cube then sets `textures`, `lightmaps` and `glow_from_program`).
   It is a full literal, not a struct update over `none`, so a field added to
   `Model` is a compile error in exactly one place for these. Checked field by
   field that each old literal equalled `none`'s defaults apart from the
   arguments (`merge` has `radius` 1.0).
   - Pinned by `mesh::tests::from_draws_places_each_argument_and_leaves_the_rest_empty`
     (every argument in its own field, the rest equal to `none`'s) and the existing
     `track`, `collision`, `merge` and `sky_cube` tests.
   - **Left on purpose:** the parsed model's literal in `mesh::build` (`mesh.rs`,
     it sets most fields from the file, so it is the other place a new field must
     be chosen), the `merge_tests` helper and the roughly 25 test literals. jscpd
     still pairs `from_draws` with that parsed literal (19 lines).

6. ~~**Smaller ui-screens copies: endrace, campaign and seat assign.**~~
   **Landed (dedupe-ui-screens), with a remainder.** The real clone family
   was the screen prologue, copied in fourteen draw lists (campaign x6,
   endrace x5, picker, join, pure_chain, seat_assign, window_layout,
   `oag_ui::menu::draw_list`), not only the named pairs:
   - `oag_ui::menu::Layers::behind(skin, frame, backdrop, race_behind)` and
     `Layers::push_title(skin, frame, title)` (`crates/ui/src/menu/layers.rs`):
     the backdrop layer and the `Draw::title` push, in the old order
     (backdrop, title, then the caller's footer overlay).
   - `Draw::plain` (`Draw::in_role` with no role) replaces the identical
     `text` closure in join, pure_chain, seat_assign and window_layout.
   - `Screen::plain_draws(sprites)` replaces the fills-then-images opening of
     `endrace_menu_draw_list`, `hd_menu_draw_list` and `cell_help_draw`.
   - `Geometry::row_at` (`window_layout.rs`) replaces the `row_at` call in the
     seat_assign and window_layout pointer handlers.
   - Whole-crate jscpd on `crates/ui-screens`: 1,617 duplicated lines
     (4.52 %) to 1,233 (3.48 %).
   **Left, differs on purpose:** (a) the nine-argument parameter lists of
   `grid_draw_list`, `cell_draw_list`, `hd_grid_draw_list`,
   `endrace_menu_draw_list`, `hd_menu_draw_list` and `rewards_layers` still
   show as 11-17 line clones: they are the public draw-list signatures
   `oag-game` calls; folding them into a struct is an API change for a later
   lane. (b) The seat_assign and window_layout pointer handlers share an
   idle/back/`at`/rows prologue (13 lines) but push different `Event` enums
   and track `hover` against `cursor` plus scroll. (c) `text_draw` in
   campaign (has the `FacedText` role branch) and endrace (plain `Text`)
   differ; picker's `text_draw` is a third form. (d) The per-screen
   `fill_draw`/`image_draw` wrappers are one-liners over `Fill::draw` and
   `Image::sprite_draw`.

### Later (real duplication, smaller win, or a sim crate)

7. **AI driver output tail is copied between two `ai/src/original` files.**
   - `crates/ai/src/original/field.rs:409-440` against
     `crates/ai/src/original/sibling.rs:313-344` (32 lines, read): the
     `driver.rail.snap`, `steer::pitch` and `Output { controls: ShipControls {
     ... } }` assembly. The head differs (`own_lane` against `lane`).
   - **Simulation crate (`oag-ai`), bound by
     `docs/architecture/determinism.md`.** The move must keep the expression
     and its operation order exactly, or it is an arithmetic change. No
     reordering, no `mul_add`, no SIMD.
   - Measure: `just check-determinism`; `cargo nextest run -p oag-ai --test
     determinism`; both committed hashes must stay unchanged.
   - Size: S, 1-2 hours. Confidence 30.

8. **Movie decoders copy the same plumbing.**
   - `crates/game/src/movie/bink.rs:126-152` against `movie/mp4.rs:118-144`
     (27 lines: the `movie(...)` tail and the `Conversion`/`transcode` call),
     and `bink.rs:163` against `mp4.rs:147` (12 lines, `ensure_source`).
   - Shared home: `crates/game/src/movie/` as a `common.rs`. Legal.
   - Size: S, 1-2 hours. Confidence 30.

9. **LAN and online menu ticks copy the button and pointer handling.**
   - `crates/game/src/main/session/lan.rs:342-364` against
     `session/online.rs:596-618` (23 lines: the fold of presses and the
     stage borrow), and `lan.rs:366-381` against `online.rs:625-640` (16
     lines: Start, Circle and pointer events).
   - Shared home: a helper in `crates/game/src/main/session/` that both call.
     Legal.
   - Size: S, 2-3 hours. Confidence 25. Measure: `cargo nextest run -p
     oag-game` for the session tests, under the gpu-vram cap if they spawn
     the game.

10. **`race::Loaded` is destructured field by field in two places.**
    - `crates/game/src/main/stage.rs:465-504` against
      `crates/game/src/race_capture.rs:413-452` (36 lines): a 45-field
      `let race::Loaded { ... } = ...` pattern repeated. Adding a field
      means editing both.
    - Shared home: pass `Loaded` whole, or group it. Legal (game-internal).
    - Size: S, under an hour. Confidence 25. Low win.

11. **Headless readback copies in the shadow code.**
    - `crates/render/src/shadow/occlusion.rs:465` `read_back` (`Vec<u8>`) and
      `crates/render/src/shadow/self_shadow.rs:334` `read_back` (`Vec<f32>`)
      share their copy and map code (jscpd: 22 and 24 lines). They differ in
      element type, row stride (`SIZE` against `SIZE * 4`) and aspect (`All`
      against `DepthOnly`). Not a plain copy; a generic helper would need a
      texel type parameter.
    - Both are live but headless-only: their docs say "the frame never
      calls this". Callers are `crates/raceplay/src/scene/frame/shadow.rs:631-648`
      (inside `dump_offscreen_if_asked`) and headless checks. This is debug
      code, not frame code.
    - Size: S, about 2 hours, low value. Confidence 20.
    - Measure: `cargo nextest run -p oag-render`; readbacks return the same
      values.

Also small, same pattern: `word()` helper copies in
`game/src/main/session/photo.rs:66` and `replays.rs:175` (11 lines each).

### Title data (not dedupe candidates, a policy call for the lead)

These are real copies across title crates. CLAUDE.md says the title crates
own their tables, but Omega's front end is HD's by lineage, and Omega already
depends on `oag-hd` (`crates/omega/Cargo.toml` line 23). So referencing HD's
constants is legal for Omega. Decide whether to share or keep copies.
Group total: 585 lines over 35 clones between `hd/`, `omega/` and `2048/`.

- `crates/hd/src/frontend.rs:250-296` against `crates/omega/src/frontend.rs:423-469`
  (44 lines): `MENU_BLOCKS` with the same measured values (HD uses `names::`
  constants, Omega literal paths).
- `crates/hd/src/pause_menu.rs:108-150` against `omega/src/pause_menu.rs:101-143`
  (43 lines).
- `crates/hd/src/hud.rs:565-605` against `omega/src/hud.rs:392-432`
  (41 lines, the `ALWAYS_ON` list). **Drift already**: HD's list has
  `"LapPanel2"` with an Eliminator comment and Omega's does not. Omega's
  comment says "HD's list". Decide if Omega should carry the Eliminator
  widget, and then share.
- `crates/2048/src/lib.rs:280-307` against `crates/omega/src/lib.rs:268-295`
  (28 lines): title tables. `crates/2048/Cargo.toml` has no `oag-hd` or
  `oag-omega` edge, so sharing from 2048 needs a new title-to-title
  dependency. The lineage in `docs/formats/omega-status.md` is the reason
  to ask; it is not a refactor by itself.

### Test boilerplate (c): leave unless the lead asks

- 340 clones touch a test file on both sides: 5,328 lines, 37% of all
  duplicated lines. Examples: `post/src/fxaa/tests.rs` against
  `smaa/tests.rs` (60 and 54 lines), `post/src/motion_blur/tests.rs` against
  itself, `post/src/hd_bloom/tests.rs`, `sound/src/tests.rs` (402 lines over
  21 clones), `ui-screens/src/campaign/draw/tests.rs`, and about 25 mesh and
  render tests that repeat the `Model` literal from candidate 5.
- Test-only cleanup is cheap but a test-file change lands in the affected
  gate; it does not justify a lane by itself.

### Noise (d): not worth a lane

- `rcs/src/gcn/opcodes.rs:570-613` against `:571-614` (44 lines): match
  arms in one file.
- `texture/src/bcn/bc7.rs:279-338` against `bc7/tests.rs:186-245` (60 lines,
  test data).
- `ai/src/plan/tests.rs` against `ai/src/probe.rs` (89 lines, test).

### Not classified (read the jscpd fragment, not the code)

These are real clones I did not open. Check them before a lane takes them:

- `crates/fx/src/psys/pipeline.rs:179-207` against `crates/view/src/main.rs:921-949` (29 lines).
- `crates/game/src/capture/seat_assign_page.rs:78` against `window_layout_page.rs:72` (21 lines).
- `crates/game/src/preview.rs:504` against `crates/ui-screens/src/picker/body.rs:336` (21 lines).
- `crates/mesh/src/mesh/shine_pass/tests.rs` against `render/src/shine/tests.rs` (37, test).

### Limits of this measure

- jscpd finds token-level copies. A copy with one changed line is
  partly found (candidate 2's second and third copies were missed). The
  real duplication is larger than the 2.1% it reports.
- It counts test files too; the non-test totals are in candidate lines
  above, not the headline.
- No candidate has a measured player-visible number. These are
  maintainability refactors; the test is that output bytes do not change.

## Next Steps

1. ~~Wire candidate 1 (`sprite_draw` into oag-ui).~~ Landed with candidate 6.
2. Decide the title-data policy question (HD and Omega front ends, and the
   `ALWAYS_ON` drift) before anyone touches `crates/omega/`.
3. Candidates 2, 4 and 5 landed. Left from 4: `oag-post` can call
   `oag_gpu::texture` (`smaa::upload` and its clamp-linear samplers), once its
   lane is free.
