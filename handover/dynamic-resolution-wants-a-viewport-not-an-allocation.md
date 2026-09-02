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
- **The menus, the front end and the loading screen are still drawn at the
  render scale**, into the offscreen target, in the same 480x272 space. **The
  HUD and the performance overlay are not, since ADR-0036** - the HUD into the
  presentation target before the grade, the overlay onto the surface after it.
  `perf.rs` used to argue "the overlay should cost what the game costs" for all
  of them, and its own module doc now records why that is right for a scale set
  once and wrong for one that moves. So this bullet is mostly cleared, and the
  part still standing is the part a *player* looks at least while racing.
- **The frame-cost signal that exists is a wall-clock interval, not a GPU
  cost.** `frame.rs:118` feeds `perf::Meter::record(elapsed)` the duration
  between loop iterations. The device is created with `Features::empty()`
  (ADR-0012 records this and makes any addition an adapter probe with a
  fallback chain).

## The three things that block it, in order of expense

**1. The UI lives inside the scaled target - and this is a blocker, not a
caveat.** A *static* 50 % is a choice the player sees once. A scale changing
every few frames makes HUD glyphs off a coverage atlas crawl continuously, and
that lands on the one element being read mid-race. This is the same restructure
[fsr-1s-default-is-open.md](fsr-1s-default-is-open.md) is blocked on, and the
same "Absent" row - *a scene without UI in it* - in
[modern-features.md](../docs/overview/modern-features.md). Doing it once
unblocks DRS, FSR 1 on the front end, and FSR 3.1.

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

## Phase 0, shared: the UI composites at presentation resolution

**This section is duplicated, on purpose, in
[FSR 1's default is open](fsr-1s-default-is-open.md) - whoever picks up either thread does this
first, and it is the same piece of work both times.** Duplicated rather than
linked because a thread file is deleted the moment its work lands: if the other
file is gone, this work is very likely already done, and
[modern-features.md](../docs/overview/modern-features.md)'s prerequisite table
is the thing to check before starting rather than the missing file.
When it lands, delete this section from whichever thread is still open.

**Decided, and most of it built.**
[ADR-0036](../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md)
settles what [ADR-0013](../docs/architecture/adr/0013-anti-aliasing-architecture.md)
explicitly declined to decide: the scene resolves first, the UI draws after it,
at presentation size. Read it before touching the rest.

**What is built.** `Framebuffer` now holds a presentation-sized target between
the upscale and the surface, and `Framebuffer::resolve` is gone, split in two:

- `resolve_scene` - FXAA/SMAA, FSR 1 and the blit, into that target,
  **ungraded**.
- `composite` - the target onto the surface, **graded**, last.

The UI draws between them. Two elements have moved across:

- **The performance overlay**, onto the surface *after* `composite`, so it is
  deliberately outside the grade - an instrument, not picture content. ADR-0036's
  one standing exception.
- **The HUD and the scoreboard**, into the presentation target *before*
  `composite`, so they are sharp at any render scale, no upscaler ever sees
  them, and they are still inside brightness and gamma. `RaceStage::render` is
  the scene alone now; `RaceStage::draw_hud` is the other half, called from
  `Session::frame` and from `race/capture.rs`'s `--presented` path.

Two tests in `crates/game/src/upscale/tests.rs` pin it:
`the_ui_composites_over_the_blit_at_presentation_resolution` for the geometry
and `the_hud_is_graded_with_the_scene_and_the_performance_overlay_is_not` for
the grade, the exception, and double-grading. Both were checked against the old
arrangement rather than assumed - drawn into the scaled target, a
one-surface-pixel fill covers no sample centre and vanishes from the frame
completely.

**Correcting ADR-0036 while it is fresh:** its consequence "every race
`--screenshot` changes bytes when the HUD moves" is **wrong**. Only
`--presented` does. The plain capture path builds no `Framebuffer` and runs no
resolve, so its HUD was already at native size and is untouched. The ADR is
immutable and stays as written; this is the correction of record.

**Roughly half a day left**, and it is one slice:

- **The menus, the front end and the loading screen.** They still draw into the
  offscreen target from their stages, so they are still rasterised at the render
  scale. The front end is the awkward one: it draws its movie and its UI through
  one `Renderer::render` over one draw list, with the movie a `Draw::Video`
  entry in it, and ADR-0036 puts a movie on the *scene* side of the seam - so
  this means splitting that call.
- **`crates/game/src/capture.rs`'s front-end path still has no `Framebuffer`
  at all**, which is the whole reason FSR 1 has never reached the front end.
  `race/capture.rs` is done and is not a template for it: that one had a
  `Framebuffer` already.

What it delivers, in the words `docs/overview/modern-features.md` uses: the
prerequisite table's last row, *a scene without UI in it*, stops being
**Absent**. It has **not** stopped yet - the HUD is out of the scaled target
but the menus and the front end are not, and FSR 3.1 needs all of it. The row
moves in the change that moves them. Update it there.

## The plan

Estimates assume the phase is picked up cold, and assume no ground-truth data
is needed - none of this is reverse-engineering, all of it is this project's
own enhancement, on the same footing as FSR 1 and SMAA. Phase 0 above comes
first; these five follow it.

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
  `Framebuffer` at all (line 28), which is the whole of Phase 0's difficulty.
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

1. Finish Phase 0 above - decided in ADR-0036 and part-built, with the
   presentation-sized target and the grade pass as the next piece. Everything
   below queues behind it, and so does
   [fsr-1s-default-is-open.md](fsr-1s-default-is-open.md).
2. Read `Framebuffer`'s five consumers of `size()` end to end (`resolve`,
   `present`, `frame.rs:490`, `session/load.rs:486`, `race/capture.rs:317`) and
   write down which of them means "the allocation" and which means "what was
   drawn". That list *is* the Phase 1 diff.
3. Probe `Features::TIMESTAMP_QUERY` on the development machines and record
   what is actually available - the fallback chain's shape depends on the
   answer, and a probe is twenty lines.
4. Write ADR-0037 for the viewport-not-allocation decision before the code, and
   `docs/rendering/dynamic-resolution.md` alongside it. Neither may link back
   into this file: a `docs/` page must never link into `handover/`.
5. Update [modern-features.md](../docs/overview/modern-features.md)'s
   prerequisite table and the M7 "Modern features" row in
   [roadmap.md](../docs/overview/roadmap.md) when the first phase lands.
