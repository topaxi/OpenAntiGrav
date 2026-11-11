---
status: ready-to-wire
titles: [all]
updated: 2026-11-11
summary: "Duplicate-code refactors ranked from jscpd; candidates 1, 3 and 6 landed, 2, 4 and 5 to wire, five later; Omega takes HD's ALWAYS_ON (lane running)."
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

2. **Cutout and transparent draw loop is copied in four places.**
   - jscpd catches `crates/game/src/hud_countdown.rs:447-484` against
     `crates/game/src/preview.rs:1127-1164` (42 lines) and
     `crates/game/src/hud_countdown.rs:475` against
     `crates/mesh/src/capture.rs:382` (11 lines). Read, the same loop also
     sits in `crates/view/src/orbit.rs:585-625` and `crates/mesh/src/capture.rs:350-390`.
     jscpd misses the last two because a comment or the order differs.
     Real copies: four, about 35 lines each.
   - Each copy walks `model.draws`, then `alpha_tested_draws` through
     `CutoutPipelines::select`, then `transparent_draws` through
     `TransparentPipelines::select`, switching `set_pipeline` only when the
     pipeline changes. `Built` is `oag_mesh::mesh_render::Built` in both
     game files, so the shape is the same.
   - Shared home: `oag-mesh`, `crates/mesh/src/mesh_render/` next to
     `cutout.rs` and `blend.rs`, which define both pipeline sets. Legal:
     game, view and mesh all depend on oag-mesh.
   - Helper takes the pipeline sets and a `bind` closure, not `&Built`,
     so `raceplay/src/drawable/draw.rs` (its own culled variant, not a
     copy) can stay as it is.
   - This is per-frame draw code (read: inside `draw`). Order must not
     change: draws stay in the `draws`, cutout, transparent sequence.
   - Size: M, half a day. Confidence 40.
   - Measure: `oag-game --screenshot` before and after on a race frame,
     with `--no-audio`, under the gate lock: the PNG must be byte-identical.
     Plus `cargo nextest run -p oag-mesh`.

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

4. **The single-level RGBA texture upload and clamp sampler are copied in
   about 15 places across fx, post, render, mesh and game.**
   - jscpd clones whose text contains `write_texture`: 266 lines over 15
     pairs. Largest: `crates/fx/src/exhaust/texture.rs:73-106` against
     `:158-191` (34 lines, one file), `crates/fx/src/weapon_quads.rs:93-116`
     against `crates/render/src/shadow.rs:656-679` (24 lines, the same
     clamp sampler plus bind group).
   - `crates/mesh/src/mesh_render/shadow_map.rs:269-298` against `:412-441`
     (30 lines): a 1x1 placeholder texture, written by hand.
   - Two helpers already exist and must stay: `crates/game/src/render/resources.rs:175`
     `upload_rgba` (single level, takes a format) and
     `crates/mesh/src/mesh_render/texture.rs:70` `upload_rgba` (mip chain,
     drops levels above the device limit). The single-level one is the
     candidate's base.
   - Shared home: `oag-gpu` (`crates/gpu/src/`, "shared GPU vocabulary").
     `rg create_texture crates/gpu/src` finds only test code, so no helper
     exists there yet. Legal: fx, post, mesh, render and game all depend on
     oag-gpu.
   - Size: M, about half a day, 15 call sites. Confidence 35.
   - Measure: `cargo nextest run -p oag-fx -p oag-render -p oag-mesh`; the
     textures must be byte-identical (dump one per crate before and after).
     No frame-time number should move.

5. **`Model` is built by hand as a literal in several places.**
   - Full literals, all with empty defaults: `crates/render/src/track.rs:137`
     (inside `build_model`, `track.rs:50`), `crates/render/src/collision.rs:278`
     (inside `build_model`, `collision.rs:211`), `crates/mesh/src/mesh/merge.rs:25`
     (`merge`), `merge.rs:207` (a test helper), `crates/mesh/src/mesh/sky_cube.rs:372`,
     and `crates/mesh/src/mesh.rs:1005`. `crates/mesh/src/mesh.rs:305` already
     has `Model::none(label)`. jscpd's `airbrakes: [None, None]` clones: 48
     lines, one clone. About 25 test files repeat the literal too (see the
     test-boilerplate section).
   - `Model` derives `Debug` and `Clone` only, with no `PartialEq`. An
     equality check in a test must compare `vertices`, `indices` and `draws`
     by hand.
   - Shared home: `oag-mesh`, a `Model::from_draws(label, vertices,
     indices, draws, centre, radius)` (or the two build functions call
     `Model::none` and set their fields). Legal: render depends on mesh.
   - Risk, read it before wiring: a full literal fails to compile when someone
     adds a field. That is currently a feature: `Model::none`'s doc says a
     `Default` would be "constructible by accident, which this must not be".
     A constructor with explicit arguments keeps that; a struct-update
     `..Model::none(label)` in tests would silently inherit a new field's
     default. Choose the constructor, and leave test literals alone unless
     the lead wants the churn.
   - Size: S for the two `build_model` functions, about 1 hour. Confidence 35.
   - Measure: `cargo nextest run -p oag-render -p oag-mesh`; the built models
     match the old ones on `vertices`, `indices` and `draws` (a test that
     compares them).

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
3. Then candidate 2 (draw loop into oag-mesh), with a before-and-after
   screenshot under the gate lock.
