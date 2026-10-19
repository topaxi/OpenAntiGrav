---
categories: [rendering]
---

# Render performance opportunities, 2026-10-19: a measurement gap on the Deck, and three small CPU items

2026-10-19, `render-perf-scout`, a read-only sweep of `crates/render`, `crates/mesh`, `crates/fx`,
`crates/post`, `crates/present`, the race's view side in `crates/raceplay` and the HUD and
UI passes in `crates/game`. **Nothing here was measured by this scout.** Every item is a
reading of the code, labelled as one. The win numbers are estimates with the reasoning shown.
The lane that takes an item measures it before and after.

Already written down, cited and not re-reported: the 4K and FSR cost, the Deck's 4K numbers
([high-resolution-cost.md](../../docs/rendering/high-resolution-cost.md), "Steam Deck, 4K"); HD's
bloom floor and half-resolution blur ([hd-gpu-cost.md](../../docs/rendering/hd-gpu-cost.md));
the split-screen encoder and uniform uploads
([split-screen-cpu-is-the-drivers-encoder-finish-now.md](split-screen-cpu-is-the-drivers-encoder-finish-now.md));
bindless on the Deck ([bindless-textures-could-take-a-fifth-of-a-views-cpu.md](bindless-textures-could-take-a-fifth-of-a-views-cpu.md));
the shadow map's GPU share, which another lane owns ([hd-gpu-cost.md](../../docs/rendering/hd-gpu-cost.md), "shadow passes").

## Open

Worth a lane now, in order:

1. ~~**The Deck's 28 fps stretch is unexplained by any measurement.**~~ Closed 2026-10-19
   (`deck-hd-28fps`). It was not a stretch: one 248 to 264 ms frame at race start, a
   synchronous soundtrack listing read on the first audio tick on `--race` and on a race
   resumed from the menus, held for two lines by the frame log's 120-frame mean. Fixed in
   `oag-sound` (`MusicDiscs` keeps listings). Numbers and evidence:
   [hd-gpu-cost.md](../../docs/rendering/hd-gpu-cost.md), last section. Items 2 and 3 could not
   be judged from that profile (64 of 1,613 steady samples unwound deeply); the panel is vsync
   bound at 90 fps with the main thread about 43 % busy, so neither limits it there.

2. **Default-path post passes create a bind group every frame.** The perf probe counts these
   (`crates/gpu/src/perfprobe.rs:153` `bind_group`), so the count is available with no new code.
   - `crates/post/src/bloom.rs:551` `prepare_in`: the bright group, built on every call.
   - `crates/post/src/hd_zoom.rs:665` (in `history_from`, line 655, called from `crates/post/src/hd_zoom/host.rs:87` each frame the zoom chain is built) calls `group_for` (line 677), which creates the group each call.
   - `crates/post/src/omega_tonemap.rs:667`: `distortion_group` on every frame that has a
     distortion source.
   - Opt-in only: `crates/post/src/fxaa.rs:227` and the source group of `crates/post/src/fsr1.rs`
     `render`, both per frame.
   - Frequency: once per post pass, per view. A split has one set per view.
   - Estimated win (estimate): creating a bind group costs about 5 to 30 us of CPU on Vulkan.
     With 3 to 6 per view a frame, that is tens to about 150 us of CPU a frame per view. At the
     Deck's 800p panel, where Pulse's CPU is 3.5 ms, that is a few percent.
   - Measure: `just bench-render pulse hd --size 1280x800` with `--features perf-probe` for the
     bind-group count per frame, and the CPU p50 before and after. Quote the load average.
   - Fix: cache each group on the source view and rebuild it on resize. In a split, key the
     cache per lane, because each view's source is its own target.
   - Risk: none to the picture or the simulation. The risk is a stale group after a resize or a
     source swap; a test should cover both.

3. **Two empty shadow clears every frame on the hull and blob tiers.** When the tier is not
   `Original` with casters, or `Mapped`, `render_shadow_map` still runs
   `map.render(queue, encoder, &[])` (`crates/raceplay/src/scene/frame/shadow.rs:252`) and
   `map.render_depth(queue, encoder, &default_fit(), &[])`. Both begin a pass that clears the
   1024x768 atlas and the 2048x2048 depth map (`crates/render/src/shadow/map.rs:623` (`render_depth`) and `:700` (`render`)).
   The frame runs it on the first view only (`does_frame_work`).
   - **Keep a clear on a tier change.** `shadow.rs` around line 596 says why the coverage map is
     cleared while its tier is on: two maps are bound at once, and a switch would show the other
     one's leftovers. A skip must clear once on the frame the tier changes, or the map must be
     unread on the tiers that skip it. Check which of the two holds before the fix.
   - Estimated win (estimate): two pass begins and two clears of CPU, tens of microseconds. The GPU
     cost is a 16 MB depth clear and a 0.8 MB atlas clear. On RDNA a depth clear is usually a fast
     clear with metadata only, so the GPU share is probably small. `shadows.md` (around line 919)
     records that the map pass sits in no GPU column, so this is unmeasured.
   - Measure: a `PassTimer` around the two clears, or the CPU p50 from `bench-render pulse` at
     `--size 1280x800`, before and after, on the Deck's CPU-bound panel.
   - Risk: a stale shadow after a tier switch, which is visible. Not a simulation change.
     `crates/render/tests/shadow_map_coverage.rs` already covers the "no casters clears" case.

Later:

4. **The HUD overlay runs up to four full-target passes a frame, each untimed.**
   `crates/game/src/hud_overlay.rs:386` to `:410` (`paint`) calls `Overlay::overlay` for values,
   captions, rows and skin. Each call is `render_with` (`crates/game/src/render.rs:619`), which
   begins a pass with the `load` op (`LoadOp::Load`) on the whole target (`render.rs:908` to `:921`).
   - Reading: the target is a full-size attachment, but on an immediate-mode GPU the load only
     reads the fragments the draws touch, so the GPU cost is small. The CPU cost is four pass
     begins, four `write_buffer`s and the per-call `HudStretch::measure`. Tens of microseconds.
   - Frequency: once a frame, for the whole screen; a split is one list (`draw_views`), so not per view.
   - Estimated win (estimate): tens of microseconds of CPU a frame. Merging the four into one pass
     saves three begins and three writes. It is not the Deck's 28 fps (item 1).
   - Measure: the item 1 timing of `paint`, and a presented run with the HUD on and off.
   - Risk: layering. The four overlays must draw in the same order over the same pixels. The
     check is a byte-for-byte `--screenshot` of the HUD, before and after, on Pulse and HD.

## Checked, no finding

These were read because a plausible concern would have been one. None is a finding.

- **No blocking readback on the runtime path.** Every `wait_indefinitely` and `map_async` in the
  render crates is in a test, in a `read_back` helper that only a capture calls, or in the opt-in
  GPU timer (`crates/gpu/src/timing/timer.rs:461`).
- **The shadow sample is gated per draw.** `crates/mesh/shaders/shadow.wesl:122` returns on
  `scene.shadow.strength <= 0.0`, and `crates/mesh/shaders/shade.wesl:246` skips the occlusion
  for `receives_shadow == 2`, the track's pipelines. `shade.wesl`'s per-slot branches read
  `in.slots`, which is constant per draw, so they do not diverge within a draw.
- **Shadows are drawn once per frame in a split.** `shadow.rs:72` `render_shadow_passes` returns
  before the map passes on a later view. Only the `Mapped` depth map is redrawn per view, and only
  for the craft that view follows.
- **The behind-the-glass target draws per view,** because its projection comes from the view's own
  camera (`crates/raceplay/src/scene/frame.rs:519`). It cannot be shared.

## Next Steps

1. Item 1 first: the Deck's HD worst stretch, read per frame, with the maintainer's go-ahead.
   Until it is read, no fix is ranked above the others.
2. Items 2 and 3 as one CPU lane on the notebook with `--features perf-probe`, quoting the bind-group
   count and the CPU p50 before and after. Start from `just bench-render pulse hd --size 1280x800`.
3. Item 4 only if item 1 points at the HUD.
