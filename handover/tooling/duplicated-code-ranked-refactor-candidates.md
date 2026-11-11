---
status: ready-to-wire
titles: [all]
updated: 2026-11-11
summary: "Seven duplicate-code refactors to wire, four later, ranked from jscpd and read on both sides; title-data policy call open."
---

# Duplicated code: ranked refactor candidates

A read-only scout sweep (jscpd, then reading both sides of the top groups).
Nothing here changes behaviour yet. Each candidate is a refactor: the
player sees no difference, so the win is one definition where there are
now copies, and fewer places for them to drift apart.

Regenerate the raw numbers (about 3 seconds, from the repository root):

```sh
npx -y jscpd crates --format rust --ignore '**/tests/**,**/target/**' \
  --min-lines 10 --reporters json --output data/scratch/jscpd --silent
jq '[.duplicates[].lines] | add' data/scratch/jscpd/jscpd-report.json
```

Paths in jscpd output are relative to `crates/`. Every path below carries
the `crates/` prefix. Line counts are jscpd's `lines` field for the clones
named, unless the text says "read". Confidence on each size estimate is a
scout estimate (0-100), capped at 60.

## Open

### Worth a lane now (ranked, highest first)

1. **`sprite_draw` is copied byte for byte between two ui-screens files.**
   - `crates/ui-screens/src/campaign/draw.rs:779-813` and
     `crates/ui-screens/src/endrace/draw.rs:642-676`: 35 duplicated lines,
     read on both sides, identical.
   - Both files import `Image`, `Placed` and `argb_to_rgba` from `oag_ui`,
     so the shared home is `oag-ui` (`crates/ui/src/frontend/` next to
     `Placed`, or the `Draw` vocabulary). Legal: ui-screens already depends
     on oag-ui, and oag-ui depends on nothing of ours that would cycle.
   - Frequency: called at each draw-list build (15 call sites in
     ui-screens). Not measured; this is layout code, not the per-frame
     draw loop.
   - Size: S, about 20 minutes. Confidence 55.
   - Measure: `just check-size`; `cargo nextest run -p oag-ui-screens`. No
     number should move. Risk: none to the simulation (UI only).

2. **Cutout and transparent draw loop is copied in four places.**
   - jscpd catches `crates/game/src/hud_countdown.rs:447-484` against
     `crates/game/src/preview.rs:1127-1164` (42 lines) and
     `crates/game/src/hud_countdown.rs:475` against
     `crates/mesh/src/capture.rs:382` (11 lines). Read, the same loop also
     sits in `crates/view/src/orbit.rs:585-625` and `capture.rs:350-390`.
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
   - This is per-frame draw code. Order must not change: draws stay in the
     `draws`, cutout, transparent sequence.
   - Size: M, half a day. Confidence 40.
   - Measure: `oag-game --screenshot` before and after on a race frame,
     with `--no-audio`, under the gate lock: the PNG must be byte-identical.
     Plus `cargo nextest run -p oag-mesh`.

3. **Post seat-lane code is copied between Omega and bloom pairs.**
   - `crates/post/src/omega_tonemap.rs:909-960` against
     `crates/post/src/wo_composite.rs:739-790` (52 lines: `retire_seat`,
     `seat_lane_builds`, `draw`); `omega_tonemap.rs:263` against
     `wo_composite.rs:196` (33 lines, `prepare_view` and `prepare_in`);
     `omega_tonemap.rs:458` against `wo_composite.rs:369` (32 lines).
   - `crates/post/src/bloom.rs:527-571` against
     `crates/post/src/ps2_bloom.rs:360-403` (45 lines, `prepare_view`);
     `bloom.rs:264` against `ps2_bloom.rs:154` (24); `bloom.rs:676` against
     `ps2_bloom.rs:519` (23). Total for the six named pairs: 209 lines.
   - `crates/post/src/hd_bloom/lanes.rs:159-295` already has a `SeatLanes`
     type. The two Omega files each keep a private `Seat` struct
     (`omega_tonemap.rs:213`, `wo_composite.rs:138`) and a private `draw`.
   - Shared home: `oag-post`, a root module (for example
     `crates/post/src/seat_lanes.rs` holding a generic `SeatLanes`, and
     `crates/post/src/pass.rs` for `draw`). The copies become callers. Legal:
     post reaches no title package (rule 3); `wo_composite` names none
     either (checked: no title import in post; one doc-comment mention in a test).
   - Frequency: per split-screen frame for `prepare_view`; `draw` is a
     per-frame pass. Not measured.
   - Size: M, about a day with the post tests. Confidence 35.
   - Measure: `cargo nextest run -p oag-post` (includes `split_tests`,
     `bloom_tests`, `seat_lane_builds` counters), and a split-screen
     screenshot before and after under the gate lock.

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
     `rg create_texture crates/gpu/src` finds only tests, so no helper
     exists there yet. Legal: fx, post, mesh, render and game all depend on
     oag-gpu.
   - Size: M, about half a day, 15 call sites. Confidence 35.
   - Measure: `cargo nextest run -p oag-fx -p oag-render -p oag-mesh`; the
     textures must be byte-identical (dump one per crate before and after).
     No frame-time number should move.

5. **Shadow occlusion and self-shadow read the same texture back.**
   - `crates/render/src/shadow/occlusion.rs:261-282` against
     `crates/render/src/shadow/self_shadow.rs:202-223` (22 lines), and
     `occlusion.rs:483-506` against `self_shadow.rs:352-375` (24 lines). The
     only difference is `TextureAspect::All` against `DepthOnly` (read).
   - Shared home: `oag-render` (`crates/render/src/shadow/` gets a
     `readback.rs`), taking the aspect as a parameter. Legal.
   - Size: S, about 2 hours. Confidence 30.
   - Measure: `cargo nextest run -p oag-render`; both readbacks return the
     same values.

6. **`Model` is built by hand as a 40-field literal in four places.**
   - `crates/render/src/collision.rs:206` `build_model` and
     `crates/render/src/track.rs:50` `build_model` each write a full `Model {
     ... }` literal with the empty defaults (about 48 lines, jscpd
     `airbrakes: [None, None]`). `crates/mesh/src/mesh.rs:305` already has
     `Model::none(label)` and `crates/mesh/src/mesh/merge.rs:31` a third
     literal. About 25 test files repeat the literal too (31 files in all) (see candidate
     test-boilerplate below).
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
     equal the old ones field for field (a test that compares them).

7. **Smaller ui-screens copies: endrace, campaign and seat assign.**
   - `crates/ui-screens/src/endrace/draw.rs:383-412` against
     `endrace/hd.rs:776-805` (30 lines, `endrace_menu_draw_list`); the
     file pair totals 112 lines over 6 clones.
   - `crates/ui-screens/src/campaign/draw.rs:34-66` against `:209-241`
     (33 lines, `grid_draw_list`) and `campaign/hd.rs:203-223` (21 lines).
   - `crates/ui-screens/src/seat_assign.rs:311-330` against
     `window_layout.rs:530-549` (20 lines, the pointer handler), and
     `seat_assign.rs:439-458` against `window_layout.rs:659-678` (20 lines,
     the hint and rows body); the pair totals 115 lines over 6 clones.
   - Shared home: `oag-ui-screens`, a module both screens import, for the
     pointer and body helpers. Legal (ui-screens is the owner).
   - Size: M, about a day for all three. Confidence 25.
   - Measure: `cargo nextest run -p oag-ui-screens`, with the pointer
     tests the screens already have. Per the pointer rule, behaviour does not
     change.

### Later (real duplication, smaller win, or a sim crate)

8. **AI driver output tail is copied between two `ai/src/original` files.**
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

9. **Movie decoders copy the same plumbing.**
   - `crates/game/src/movie/bink.rs:126-152` against `movie/mp4.rs:118-144`
     (27 lines: the `movie(...)` tail and the `Conversion`/`transcode` call),
     and `bink.rs:163` against `mp4.rs:147` (12 lines, `ensure_source`).
   - Shared home: `crates/game/src/movie/` as a `common.rs`. Legal.
   - Size: S, 1-2 hours. Confidence 30.

10. **LAN and online menu ticks copy the button and pointer handling.**
    - `crates/game/src/main/session/lan.rs:342-364` against
      `session/online.rs:596-618` (23 lines: the fold of presses and the
      stage borrow), and `lan.rs:366-381` against `online.rs:625-640` (16
      lines: Start, Circle and pointer events).
    - Shared home: a helper in `crates/game/src/main/session/` (a menu tick
      helper) that both call. Legal.
    - Size: S. Confidence 25. Measure: `cargo nextest run -p oag-game` for
      the session tests, under the gpu-vram cap if they spawn the game.

11. **`race::Loaded` is destructured field by field in two places.**
    - `crates/game/src/main/stage.rs:465-504` against
      `crates/game/src/race_capture.rs:413-452` (36 lines): a 45-field
      `let race::Loaded { ... } = ...` pattern repeated. Adding a field
      means editing both.
    - Shared home: pass `Loaded` whole, or group it. Confidence 25. Low win.

Also small, same pattern: `word()` helper copies in
`game/src/main/session/photo.rs:66` and `replays.rs:175` (11 lines each).

### Title data (not dedupe candidates, a policy call for the lead)

These are real copies across title crates. CLAUDE.md says the title crates
own their tables, but Omega's front end is HD's by lineage, and Omega already
depends on `oag-hd` (`crates/omega/Cargo.toml` line 23). So referencing HD's
constants is legal. Decide whether to share or keep copies.

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
  (28 lines): title tables. 2048 inherits HD by lineage too (see
  `docs/formats/omega-status.md`).

### Test boilerplate (c): leave unless the lead asks

- 340 clones touch a test file on both sides: 5,328 lines, 37% of all
  duplicated lines. Examples: `post/src/fxaa/tests.rs` against
  `smaa/tests.rs` (60 and 54 lines), `post/src/motion_blur/tests.rs` against
  itself, `post/src/hd_bloom/tests.rs`, `sound/src/tests.rs` (402 lines over
  21 clones), `ui-screens/src/campaign/draw/tests.rs`, and about 25 mesh and
  render tests that repeat the `Model` literal from candidate 6.
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

1. Wire candidate 1 (`sprite_draw` into oag-ui). Smallest, identical copies,
   easiest to check: `just gate-affected`.
2. Decide the title-data policy question (HD and Omega front ends, and the
   `ALWAYS_ON` drift) before anyone touches `crates/omega/`.
3. Then candidate 2 (draw loop into oag-mesh), with a before-and-after
   screenshot under the gate lock.
