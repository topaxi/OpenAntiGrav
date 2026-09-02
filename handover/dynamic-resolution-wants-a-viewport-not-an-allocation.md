# Dynamic resolution wants a viewport, not an allocation

Dynamic resolution scaling (DRS) is its own feature, not an upscaling one: a
closed loop that measures how expensive a frame was and resizes the render
target for the next one. Upscaling is its mandatory partner - something has to
carry a smaller frame onto the surface - but the resampler is a separate,
swappable piece, and this repository already ships one.

**Both halves DRS sits between exist.** `[graphics] render_scale`
(`display::Scale`, 25-200 % of the aspect rectangle) already decouples render
resolution from presentation, and `upscale::target_size` +
`Framebuffer::resize` already act on it every frame in
`crates/game/src/main/session/frame.rs`. `[graphics] upscaler` already
resolves that frame with FSR 1 (`oag_render::post::fsr1`, per
[ADR-0012](../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)).
What is left is the controller and the wiring that feeds it.

## What is true of the tree, as of 2026-09-02

Rewritten after Phase 1 and Phase 2's probe landed. **The three structural
blockers this file was written around are cleared**; what is left is not
structure. Deliberately no line numbers - the version of this section written a
day earlier cited five, and every one of them has moved since.

- **Two sizes, not one.** `Framebuffer` carries an `allocation()` - the
  `render_scale` ceiling, what the texture actually is, what every size-matched
  attachment follows - and an `extent()`, the sub-rectangle drawn into this
  frame. `set_extent` moves the second for a uniform write, allocating nothing.
  There is deliberately **no `size()`**, so a new caller has to say which it
  means. [ADR-0037](../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md);
  the per-consumer audit is on
  [dynamic-resolution.md](../docs/rendering/dynamic-resolution.md).
- **Nothing moves the extent.** Every frame the game draws has extent equal to
  allocation, so no behaviour changed and no capture's bytes moved. A controller
  is the thing that would move it, and there is no controller.
- **No per-frame scene post-process is viewport-aware, and that bounds who may
  move it.** FXAA, SMAA and FSR 1 take the extent as an input size while reading
  a view of the allocation; `bloom` and `hd_bloom` take neither a size nor a
  viewport at all. Bloom is its own setting and is on in an ordinary race, so
  there is **no combination of menu rows that makes a sub-extent safe** - see
  Phase 5, which owns both halves. `motion_blur` is the one pass already shaped
  right and is the shape the others need.
- **FSR 1 was deliberately told the viewport and the resource are the same
  rectangle.** `post/fsr1.rs`: *"The input viewport and the input resource are
  the same rectangle here - so the viewport and size arguments collapse into
  one."* Upstream's `FsrEasuCon` takes them separately; this port folded them,
  and Phase 5 un-folds them - which moves the port *closer* to upstream.
- **No UI is drawn at the render scale, and that removes a real DRS hazard.**
  Since ADR-0036 and ADR-0038 the HUD and scoreboard composite into the
  presentation target after the resolve, the performance overlay goes onto the
  surface after the grade, and the menus, front end, launcher and loading screen
  skip the scaled target entirely. Under the old shape a controller moving the
  scale several times a second would have made HUD glyphs and menu text shimmer
  as they were re-rasterised at a new size every few frames - the most visible
  artefact DRS could have produced, on the element a player reads rather than
  looks at. It structurally cannot happen now.
- **GPU timestamps are probed but not enabled.** `oag_render::timing::Timing`
  reports what an adapter offers without requesting a device; nothing in the
  workspace asks for the feature, and `mesh_render::optional_features` is the
  one line that changes when a consumer exists. The signal actually in use is
  still `perf::Meter` over a wall-clock interval, which is pinned to the refresh
  under `Vsync::On` and to the limit under any `FrameLimit` - it tracks real
  work only with vsync `off` *and* no limit, a diagnostic configuration rather
  than a shipping one. That is why a GPU timestamp is the primary path and not a
  refinement.

## The three things that blocked it - all three are cleared

Kept as a record of what the work was, not as a list of open items.

**1. The UI lived inside the scaled target.** Cleared by Phase 0. A *static*
50 % is a choice the player sees once; a scale changing every few frames would
have made HUD glyphs off a coverage atlas crawl continuously, on the one element
being read mid-race. Decided in
[ADR-0036](../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md)
and [ADR-0038](../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md);
[modern-features.md](../docs/overview/modern-features.md)'s *a scene without UI
in it* row is **Done**. The performance overlay moving out matters here for its
own sake: it is the row a player reads to tell DRS from a stutter, and it can
now be read while the scale moves.

**2. Every scale change was a reallocation.** Cleared by Phase 1. A controller
stepping the scale 2 % per frame would have rebuilt a texture, two views, a bind
group, a depth attachment and (under MSAA) the multisampled attachments, several
times a second - paying a cost to save a cost. Phase 1 took the standard answer:
**allocate once at the ceiling and vary the viewport.** Moving the extent is now
a uniform write and two `set_viewport` calls.

**3. The wall-clock interval carries no headroom.** Cleared *as an unknown* by
Phase 2's probe rather than solved: both development adapters support GPU
timestamps end to end, so the signal is reachable and the shape of the fallback
chain is known. Wiring one is the work that is left.

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
is needed - none of this is reverse-engineering, all of it is this project's own
enhancement, on the same footing as FSR 1 and SMAA. **Phases 0 and 1 are done
and Phase 2's probe is done**; the rest of Phase 2, then 3, 4 and 5, are what is
left. The phases are kept in place rather than deleted as they land, because
each one's reasoning is what the next is built on - a `## Open` question below
is usually a question about a phase that has already shipped.

### Phase 1 landed 2026-09-02: a fixed allocation and a moving viewport

**Done.** [ADR-0037](../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md)
records the decision and
[dynamic-resolution.md](../docs/rendering/dynamic-resolution.md) carries the
audit of every consumer. What is now true:

- `Framebuffer` has `allocation()` and `extent()` and **no `size()`** - the
  removal is the point, so a new caller has to say which it means. `resize`
  moves the allocation off the `render_scale` ceiling and still returns whether
  it reallocated; `set_extent` moves the extent, clamps into `1..=allocation`
  and allocates nothing. **Any reallocation resets the extent**, or a window
  resize under a stale larger extent sets a viewport past the attachment.
- `frame.rs`'s `inside` and `warm_up`'s rectangle take the extent, so the
  projection and ADR-0039's jitter follow it with no change of their own. The
  three `scene.resize` arms, the suspended race, `app.rs` and `load.rs`'s
  `build_race_stage` take the allocation.
- The blit reads the extent: `Source` rides the grade uniform as a UV scale
  plus a clamp half a texel inside the drawn edge. `Source::WHOLE` is exactly
  `1.0` on both **by construction**, which is what keeps the change
  structurally inert - measured, not asserted: `--presented` captures at
  `--render-scale 50 --upscaler fsr1` and at `--render-scale 100 --upscaler
  off` are byte-identical across the commit (`sha256`, 1440x816).
- `race/capture.rs` needed no change after all. It never called
  `Framebuffer::size()`; it computes `target_size` into its own
  `PresentedState::scene_size` and hands that to `Framebuffer::new`, which sets
  both sizes equal. Likewise `race/scene/frame.rs:874`'s `self.depth.size()`:
  `motion_blur::Frame` already separates `size` from `viewport` and was already
  taking the right one for each.

**What Phase 1 did *not* do, and it bounds what may set an extent.** No
per-frame scene post-process is viewport-aware, and the list is longer than the
upscaler. FXAA, SMAA and FSR 1 are handed the extent as their input size while
reading a view of the allocation-sized texture - correct only while the two are
equal, and their own targets would be rebuilt on every extent change besides.
**And `bloom` and `hd_bloom` take no size and no viewport at all**, so a short
extent would have them blurring the undrawn region inward as a dark edge; bloom
is its own setting and is on in an ordinary race, so there is no combination of
menu rows that makes a sub-extent safe. Nothing outside a test may set one, full
stop; the two tests that do build a bare `Framebuffer` with no `Scene` behind
it. `Framebuffer::set_extent` says so on itself.

`motion_blur` is the one pass already shaped right and is the shape the others
need - `motion_blur::Frame` carries `size` and `viewport` separately and takes
the correct one for each.

Two things worth carrying forward that were not in the plan: the memory cost is
against a *hypothetical* reallocating DRS and **not** against today (the
allocation is exactly what `target_size` already allocated), and `LoadOp::Clear`
is not viewport-restricted in wgpu, so a scaled frame still pays a ceiling-sized
clear - which bounds what DRS can save at low scales and belongs in Phase 2's
budget rather than in a surprise.

### Phase 2 - a cost signal worth controlling on

**This is the next one to pick up, and its probe is done.**
`oag_render::timing::Timing` reports what an adapter offers without requesting
a device, and `Timing::features` turns that back into what a `request_device`
may safely ask for. **Nothing enables it**:
`mesh_render::optional_features` is the one line that changes when a consumer
exists, and turning a feature on with nothing reading it buys a driver
behaviour change for nothing.

Measured 2026-09-02 on the development machine, both adapters, end to end - a
real query pair resolved around a cleared pass, not a feature bit read. Both
support all three timestamp features over Vulkan; the Intel iGPU's tick is
**52.083332 ns** and the NVIDIA dGPU's is **1 ns**. The table and what it
settles are on
[dynamic-resolution.md](../docs/rendering/dynamic-resolution.md#what-the-probe-actually-found);
the two consequences to carry into the rest of Phase 2 are that a budget must
be computed from `Queue::get_timestamp_period` every run and never from a
constant, and that **the no-timestamp fallback path cannot be exercised on this
machine** - both adapters have the feature, so a green run here is not evidence
that the degraded path works.

**Roughly a day for the rest.** Write a timestamp either side of the scene
passes so the measurement is *the part that scales* - the UI composite after
Phase 0 does not scale and must not be in the budget.

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

### Phase 5 - tell the upscaler, and the bloom, the size changed

**Half a day for FSR 1, and bloom is a second job beside it** - `bloom::render`
and `hd_bloom::run` take neither a size nor a viewport today, so they need one
added rather than un-folded. Both have to land before a controller may move the
extent at all; see the caveat under Phase 1.

**Half a day for FSR 1.** Un-collapse `fsr1::Constants::new`'s viewport and
size arguments back to upstream's two, which is a *restoration* of
`ffx_fsr1.h`'s own parameters and so keeps ADR-0012's transliteration property
intact - the diff against upstream gets closer, not further. FSR 1 is spatial,
holds no history, and so absorbs a resolution change on the frame it happens.

**FSR 3.1, when it lands, is where DRS pays properly**: a temporal upscaler
takes a per-frame render size as a first-class input.

**Half of the coupling this used to warn about is already satisfied.** Camera
jitter landed 2026-09-02
([ADR-0039](../docs/architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md))
and it is handed the render target's extent every frame -
`oag_render::jitter::matrix(frame, (viewport.2, viewport.3))`, off
`Framebuffer::size()` - so a scale that moves several times a second already
reaches it with no change here. What is *not* satisfied is the second half: the
**phase count** is a fixed sixteen and deliberately so, because deriving it
from the presentation-to-render ratio is the FSR 3.1 port's decision to make,
not this thread's and not `jitter`'s. When that port lands, a moving scale means
a moving phase count, and `oag_render::jitter::PHASES` is the one place it
changes.

## Traps

- **Every capture path must force DRS off, and there are two of them with
  different shapes.** `crates/game/src/race/capture.rs` builds its own
  `Framebuffer` (conditional on `presented`) and calls `target_size` itself;
  `crates/game/src/capture.rs`'s front-end path has no `Framebuffer` at all -
  which since ADR-0038 is correct rather than a gap, because the front end has
  no scene to scale; what it still lacks is the grade and the aspect bars, not
  a render target. **Phase 1 makes the first of the two safe by construction**:
  the capture builds its `Framebuffer` at `scene_size` and never calls
  `set_extent`, so its extent is its allocation for the life of the run. That
  holds only while nothing wires a controller into the capture path, which is
  what this trap is now guarding.
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

Steps 1, 3 and 4 of the original list are done: the audit is written up in
[dynamic-resolution.md](../docs/rendering/dynamic-resolution.md), ADR-0037 is
written, and `modern-features.md`'s first prerequisite row plus the M7 rows in
[roadmap.md](../docs/overview/roadmap.md) name the split.

1. Bracket the scene passes with a timestamp pair and feed the resolved
   duration to a meter, off `Timing` (already built, see Phase 2). The reading
   resolves a frame or more late, so whatever holds it has to say which frame it
   belongs to rather than assume the last one.
2. Measure the ceiling-sized clear before designing a policy around a budget.
   `LoadOp::Clear` is not viewport-restricted, so the scene pass's fixed cost
   does not fall with the extent; how much of a frame that is decides whether a
   floor below (say) 60 % buys anything at all.
3. Measure the ceiling allocation's memory on a Steam-Deck-class target. It is
   the one number ADR-0037 states as unmeasured, and the answer could argue for
   a per-title ceiling rather than a per-title floor.
