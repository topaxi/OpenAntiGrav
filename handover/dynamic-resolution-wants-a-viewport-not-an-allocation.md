# Dynamic resolution wants a viewport, not an allocation

Dynamic resolution scaling (DRS) is its own feature, not an upscaling one: a
closed loop that measures how expensive a frame was and resizes the render
target for the next one. Upscaling is its mandatory partner - something has to
carry a smaller frame onto the surface - but the resampler is a separate,
swappable piece, and this repository already ships one.

**Both halves DRS sits between exist.** `[graphics] render_scale`
(`display::Scale`, 25-200 % of the aspect rectangle) already decouples render
resolution from presentation, and `upscale::target_size` +
`Framebuffer::resize` already act on it every frame at
`crates/game/src/main/session/frame.rs:439`. `[graphics] upscaler` already
resolves that frame with FSR 1 (`oag_render::post::fsr1`, per
[ADR-0012](../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)).
What is missing is the controller, a frame-cost signal it can trust, and two
structural facts without which DRS is *visibly worse* than a fixed scale.

## What is verified about the current shape

Read out of the tree on 2026-09-02, not assumed:

- **The scale is a per-frame quantity already.** `frame.rs:439` recomputes
  `target_size(rect, settings.graphics.render_scale, max_texture_dimension_2d)`
  every frame and calls `Framebuffer::resize`, which returns `true` when it
  rebuilt so the race's depth attachment, a race parked in
  `Session::suspended_race` and a scene still building in
  `LoadingStage::built_race` can all follow. A controller has a place to write
  into; it does not have to invent the plumbing.
- **`Framebuffer::resize` reallocates.** `crates/game/src/upscale.rs:419`
  early-returns only when the size is unchanged; otherwise it rebuilds the
  texture, both views and the bind group. It was written for a menu row moved
  once, not for a value moving several times a second.
- **The whole frame is one extent.** `frame.rs:490` hands each stage
  `inside = (0.0, 0.0, size.0, size.1)` off `Framebuffer::size()`, and
  `Framebuffer::resolve` reads `self.size` for both `magnifies` and FSR 1's
  `Frame { input }`. Nothing in the type distinguishes "how big the texture is"
  from "how much of it was drawn this frame".
- **FSR 1 was deliberately told they are the same rectangle.**
  `post/fsr1.rs:46`: *"The input viewport and the input resource are the same
  rectangle here - so the viewport and size arguments collapse into one."*
  Upstream's `FsrEasuCon` takes them separately; this port folded them.
- **No UI is drawn at the render scale any more, and that removes a real DRS
  hazard.** Since ADR-0036 and ADR-0038 the HUD and scoreboard composite into
  the presentation target after the resolve, the performance overlay goes onto
  the surface after the grade, and the menus, front end, launcher and loading
  screen skip the scaled target entirely. Under the old shape a controller
  moving the scale several times a second would have made HUD glyphs and menu
  text shimmer as they were re-rasterised at a new size every few frames -
  the most visible artefact DRS could have produced, on the element a player
  reads rather than looks at. It structurally cannot happen now. `perf.rs` used
  to argue "the overlay should cost what the game costs"; its own module doc
  records why that is right for a scale set once and wrong for one that moves.
- **The frame-cost signal that exists is a wall-clock interval, not a GPU
  cost.** `frame.rs:118` feeds `perf::Meter::record(elapsed)` the duration
  between loop iterations. The device is created with `Features::empty()`
  (ADR-0012 records this and makes any addition an adapter probe with a
  fallback chain).

## The three things that block it, in order of expense

**1. The UI lived inside the scaled target - and this was a blocker, not a
caveat. It is done.** A *static* 50 % is a choice the player sees once. A scale
changing every few frames would have made HUD glyphs off a coverage atlas crawl
continuously, on the one element being read mid-race. Phase 0 below removed it:
no UI is drawn at the render scale any more, and
[modern-features.md](../docs/overview/modern-features.md)'s *a scene without UI
in it* row is **Done**.

**Decided in
[ADR-0036](../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md)
and part-built** - see Phase 0 below. The performance overlay has moved out of
the scaled target, which matters here for its own sake: it is the row a player
reads to tell DRS from a stutter, and it can now be read while the scale moves.
The HUD and the menus have not moved, so this is still a blocker.

**2. Every scale change is a reallocation.** A controller that steps the scale
2 % per frame would rebuild a texture, two views, a bind group, a depth
attachment and (under MSAA) the multisampled attachments, several times a
second - paying a cost to save a cost. The standard answer, and the one this
plan proposes: **allocate once at the ceiling and vary the viewport.**

**3. The wall-clock interval carries no headroom.** Under `Vsync::On` it is
pinned to the refresh; under any `FrameLimit` it is pinned to the limit,
because `schedule_next_frame` sleeps to it. It only tracks work with vsync
`off` *and* no limit - a diagnostic configuration, not a shipping one. So a
GPU timestamp is the primary path, not a refinement.

## Phase 0 landed 2026-09-02: the UI composites at presentation resolution

**Done, and it is no longer a prerequisite - it is a property to rely on.**
[ADR-0036](../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md)
settles what [ADR-0013](../docs/architecture/adr/0013-anti-aliasing-architecture.md)
explicitly declined to decide, and
[ADR-0038](../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)
supersedes its seam item for stages with no 3D scene. Read both before Phase 1;
they are what the rest of this plan now assumes.

The shape DRS inherits:

- `resolve_scene` - FXAA/SMAA, FSR 1 and the blit, from the scaled target into
  a presentation-sized one, **ungraded**.
- The UI draws into that presentation target.
- `composite` - the presentation target onto the surface, **graded**, last.
- A stage with no 3D scene (launcher, loading, front end, menus) never calls
  `resolve_scene` at all and draws straight into the presentation target; its
  own clear draws the aspect bars.

**Two consequences for this thread specifically.** (1) The scaled target is now
touched by nothing but the race scene, so a controller writing to it several
times a second cannot make text shimmer - see the "What is verified" bullet
above. (2) `Framebuffer` already separates the two sizes: `resize` moves the
scene target on `render_scale`, `resize_output` moves the presentation target on
the surface. Phase 1's "fixed allocation, moving viewport" now only has to
change the first of those.

## The plan

Estimates assume the phase is picked up cold, and assume no ground-truth data
is needed - none of this is reverse-engineering, all of it is this project's
own enhancement, on the same footing as FSR 1 and SMAA. Phase 0 above is
already done; these five are what is left.

### Phase 1 - a fixed allocation and a moving viewport

**Roughly a day and a half.** Split `Framebuffer`'s one `size` into an
**allocation** (the ceiling, resized only when the window or `render_scale`
ceiling changes) and a **render extent** (this frame's pixels, a sub-rect
anchored at the origin). Then:

- `view()`'s consumers set viewport and scissor to the render extent; the
  stages' `inside` at `frame.rs:490` becomes the extent, not the allocation.
- `present`'s fullscreen triangle scales its UVs by `extent / allocation`, and
  the sampler must **clamp at the extent's edge**, or the blit pulls in stale
  pixels from outside the rect - the classic DRS artefact, a bright fringe on
  the right and bottom.
- The race's depth attachment, and MSAA's attachments, are allocated at the
  ceiling and viewport-restricted like the colour one. `Framebuffer::resize`'s
  `true` return then fires far less often, which is the point.
- `race/capture.rs:317` calls `target_size` itself and needs the same split, or
  it silently keeps the old meaning.

This is what a new ADR-0037 should record - the number is free as of
2026-09-02, see the [ADR index](../docs/architecture/adr/README.md), 0036
having been taken by Phase 0's own decision: *dynamic
resolution varies a viewport, not an allocation* -
with the memory cost stated honestly, since the target is always the ceiling's
size even when rendering below it.

### Phase 2 - a cost signal worth controlling on

**Roughly a day.** `wgpu::Features::TIMESTAMP_QUERY`, behind an adapter probe
with the fallback chain ADR-0012 already mandates for FSR 3.1: a missing
feature degrades, never fails to boot. Write a timestamp either side of the
scene passes so the measurement is *the part that scales* - the UI composite
after Phase 0 does not scale and must not be in the budget.

Two things to say out loud in the doc page:

- **The reading is late.** Timestamps resolve a frame or more behind, so the
  controller is acting on stale cost. The cooldown after a step has to cover
  the frames in flight or the loop will chase itself.
- **Where there is no timestamp support, DRS is off**, and the menu row says
  so via the existing `disabled_by` mechanism (`menu.rs:335`), the way the
  frame-limit row is already greyed under `display.vsync = on`. Advertising a
  controller driven by a signal that is pinned to the refresh is worse than
  not offering it.

### Phase 3 - the controller

**Roughly a day, most of it tests.** A new module - `crates/game/src/drs.rs` -
that is a **pure function of fed measurements**, exactly as `perf::Meter` is:
`record(cost_seconds)` in, a scale out, never reading a clock or a GPU itself.
That is what makes it testable without a GPU, and it is the same argument
`perf.rs` already makes for why the module is not a determinism problem -
nothing here is read by the simulation.

The policy, which is conventional and should stay so:

- Cost scales with pixel count, so the correction is
  `next = current * sqrt(budget / measured)`.
- A **deadband**: act only outside roughly 85-95 % of budget, so a frame
  sitting comfortably does not twitch.
- A **clamped per-frame delta**, asymmetric: fall fast (a dropped frame is
  already visible), rise slowly (a rise that overshoots costs a drop).
- **Quantized steps** on a grid, so the extent lands on a small set of values -
  cheaper to reason about, and it keeps a hysteresis band meaningful.
- A **cooldown** after any step, covering the frames in flight from Phase 2.

**Invariant, and the kind of bug that survives review: DRS never writes
`settings.graphics.render_scale`.** It *reads* it as the ceiling and emits a
separate runtime scale. Persisting a controller output into the player's
settings file would make their chosen quality silently drift downward with
every session on a loaded machine.

### Phase 4 - the settings and the row

**Half a day.** `[graphics] dynamic_resolution`, defaulting off (the footing
every enhancement here starts on), naming a target rate; and
`[graphics] dynamic_resolution_floor`, a `display::Scale` the controller may
not go below. `render_scale` keeps its meaning and becomes the ceiling. Follow
`display::Vsync`'s and `Sharpness`'s `serde(untagged)` precedent if the shape
of the value is likely to change - both files record why that twenty lines was
worth it.

The menu page needs the pairing spelled out the way the upscaler row already
warns at scales where it does nothing (`menu/tests/definition.rs:553` asserts
the warning names exactly those scales) - a DRS row and a `render_scale` row
that silently disagree is the row a player will file a bug about.

**Decided 2026-09-02: RENDER SCALE itself gets no warning, deliberately.**
Since ADR-0038 the row changes nothing on the screen a player moves it from -
the menus draw at presentation resolution, so the setting is invisible until a
race starts. Asked directly, the maintainer chose silence. The reasoning to
keep: the existing mechanism means *"this row is redundant given another row's
value"*, and *"this row's effect is not visible on this screen"* is a different
kind of statement; overloading one warning with both was judged worse than no
warning at all. Do not re-open this as an oversight - it is a decision. The DRS
pairing above is the first kind of statement and does still want a warning.

### Phase 5 - tell the upscaler the size changed

**Half a day for FSR 1.** Un-collapse `fsr1::Constants::new`'s viewport and
size arguments back to upstream's two, which is a *restoration* of
`ffx_fsr1.h`'s own parameters and so keeps ADR-0012's transliteration property
intact - the diff against upstream gets closer, not further. FSR 1 is spatial,
holds no history, and so absorbs a resolution change on the frame it happens.

**FSR 3.1, when it lands, is where DRS pays properly**: a temporal upscaler
takes a per-frame render size as a first-class input. Note the coupling now
rather than discovering it then - the jitter sequence's phase count is
conventionally a function of the scale factor, so *camera jitter*, the other
"Absent" row in modern-features.md, has to be told the scale each frame too.

## Traps

- **Every capture path must force DRS off, and there are two of them with
  different shapes.** `crates/game/src/race/capture.rs` builds its own
  `Framebuffer` (line 380, conditional on `presented`) and calls `target_size`
  itself (line 317); `crates/game/src/capture.rs`'s front-end path has no
  `Framebuffer` at all (line 28) - which since ADR-0038 is correct rather than
  a gap, because the front end has no scene to scale; what it still lacks is
  the grade and the aspect bars, not a render target.
  Fixing only the first leaves the second reacting to machine load. This
  project's comparisons are byte-identical screenshot diffs - thirty captures
  came out byte-identical when the PVS tiers were validated - and a controller
  driven by how busy the machine is makes every one of them irreproducible.
  `perf.rs` already makes exactly this argument for why the overlay is
  window-only; it applies verbatim. (`oag-trace` has no screenshot route of its
  own - checked - so it is not a third path.)
- **DRS must not reach the simulation.** It is presentation-side, like
  `perf::Meter` and `FrameLimit`; the timestep stays fixed at 60 Hz per
  ADR-0007 whatever the resolution does.
- **Do not measure a load as a frame.** `Session::stalled` already exists for
  this and clears the meter; the DRS controller needs the same guard, or a
  track load will drive the scale to the floor and take seconds to climb back.
- **Do not read a menu capture taken with DRS on as evidence of anything**, for
  the same reason the FSR 1 thread records about `--upscaler`: the setting
  changes the overlay's own text, so two images differ for an unrelated reason.

## Open

- What the target rate should be *offered* as. The simulation is fixed 60 Hz
  with unlocked presentation, so a target above 60 is meaningful, but the
  existing `FrameLimit` type already covers "how many frames a second" and
  reusing it against a different meaning may be worse than a new type.
- Whether the floor should be per-title. HD/Fury's scenes and Pulse's are not
  the same cost, and a floor that is right for one may be visibly poor on the
  other. No measurement has been taken either way.
- Whether the memory cost of allocating at the ceiling is acceptable on the
  Steam Deck, which [goals.md](../docs/overview/goals.md) names in the first
  tier. Unmeasured.
- Whether the perf overlay should show the current scale. It is the only way a
  player can tell DRS from a stutter, which argues yes, but it is a fourth line
  on a `dev`-tier readout that is already dense.

## Next Steps

1. Read `Framebuffer`'s five consumers of `size()` end to end (`resolve`,
   `present`, `frame.rs:490`, `session/load.rs:486`, `race/capture.rs:317`) and
   write down which of them means "the allocation" and which means "what was
   drawn". That list *is* the Phase 1 diff.
2. Probe `Features::TIMESTAMP_QUERY` on the development machines and record
   what is actually available - the fallback chain's shape depends on the
   answer, and a probe is twenty lines.
3. Write ADR-0037 for the viewport-not-allocation decision before the code, and
   `docs/rendering/dynamic-resolution.md` alongside it. Neither may link back
   into this file: a `docs/` page must never link into `handover/`.
4. Update [modern-features.md](../docs/overview/modern-features.md)'s
   prerequisite table and the M7 "Modern features" row in
   [roadmap.md](../docs/overview/roadmap.md) when the first phase lands.
