# Dynamic resolution

**Status: the structure exists, the controller does not.** `Framebuffer` carries
an allocation and a render extent separately, per
[ADR-0037](../architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md),
and every consumer of the old single `size` has been resolved to one or the
other. Nothing moves the extent yet: there is no cost signal worth controlling
on and no controller, so the extent equals the allocation on every frame the
game draws, and a build with this structure in it produces byte-identical frames
to one without.

Dynamic resolution scaling is its own feature and not an upscaling one. It is a
closed loop: measure what a frame cost, resize the render target for the next
one, hold a frame budget. Upscaling is its mandatory partner - something has to
carry a smaller frame onto the surface - but the resampler is a separate,
swappable piece and this project already ships one
([ADR-0012](../architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)).

## The two sizes

| | **Allocation** | **Render extent** |
| --- | --- | --- |
| What it is | The texture's real dimensions | The sub-rectangle drawn into this frame |
| Where it comes from | `upscale::target_size(rect, render_scale, limit)` - the `render_scale` **ceiling** | A controller's runtime scale, `<=` the allocation on both axes |
| Moves when | The window, the aspect or the `render_scale` row moves | As often as a policy says, several times a second |
| Costs | Six texture creations and a bind group | A uniform write and two viewport calls |
| Read by | Anything that *builds an attachment* | Anything that *is a viewport* |
| Accessor | `Framebuffer::allocation()` | `Framebuffer::extent()` |

`Framebuffer::resize` moves the allocation and returns whether it reallocated;
`Framebuffer::set_extent` moves the extent and returns nothing, because there is
nothing for a caller to follow up on. **Any reallocation resets the extent to
the new allocation.** That is not a convenience: a window resize that shrinks
the allocation while a stale larger extent survived would set a viewport past
the attachment, which is a validation error rather than a bad picture.

A third size sits beside both and is unrelated to either: the **presentation
target**, `Framebuffer::resize_output`, which follows the surface. The UI
composites there after the scene resolves into it
([ADR-0036](../architecture/adr/0036-ui-composites-at-presentation-resolution.md)),
and a stage with no 3D scene draws straight into it and never touches the scaled
target at all
([ADR-0038](../architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)).

## The audit

Every reader of the old `Framebuffer::size()`, and which of the two it meant.
This table is the record of that reading rather than a summary of it - a new
consumer of either size has to answer the same question, and the type cannot
answer it for them.

| Site | Means | Why |
| --- | --- | --- |
| `main/app.rs`, building the first `Framebuffer` | Allocation | `target_size` off the ceiling, before any frame has been drawn |
| `main/app.rs`, `Stage::race(..., framebuffer.allocation(), ...)` | Allocation | Builds the scene's depth and MSAA attachments, which must match the colour texture |
| `main/session/frame.rs`, `framebuffer.resize(wanted)` | Allocation | `wanted` is `target_size` off the `render_scale` ceiling |
| `main/session/frame.rs`, the three `scene.resize(...)` arms and the suspended race | Allocation | Same validation rule: a depth attachment sized for the colour one |
| `main/session/frame.rs`, `inside` handed to every stage | **Extent** | It is a viewport. Everything downstream that normalises against it - the projection, and [ADR-0039](../architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)'s jitter via `jitter::matrix(frame, (viewport.2, viewport.3))` - inherits the extent for free |
| `main/session/load.rs`, `build_race_stage(..., allocation, ...)` | Allocation | Builds attachments, as `app.rs` does |
| `main/session/load.rs`, `warm_up`'s rectangle | **Extent** | It is a viewport. Warming pipelines is keyed on format, not on the size of the attachment they first meet - see ADR-0038 |
| `upscale.rs`, `resolve_scene`'s `magnifies` test | **Extent** | Whether the *drawn pixels* are fewer than the destination's is what decides whether a magnifier has anything to do |
| `upscale.rs`, FXAA/SMAA `Frame::size` and FSR 1 `Frame::input` | **Extent**, and see the caveat below | The source rectangle those passes reconstruct |
| `upscale.rs`, `present`'s UV scale and clamp | **Both** | The ratio `extent / allocation` is the whole point of the split |
| `upscale.rs`, `composite` | Neither | It reads the presentation target, whose size is `Output::size` |
| `race/capture.rs`, `PresentedState::scene_size` | Allocation | `target_size` off the capture's own `--render-scale`; a capture never moves it |
| `race/scene/frame.rs`, `self.depth.size()` into `motion_blur::Frame::size` | Allocation | `motion_blur::Frame` already separates `size` (the attachments' dimensions) from `viewport` (the rectangle drawn), and takes the correct one for each. No change was needed here |

Two sites deliberately unaffected:

- **`capture.rs`'s front-end path has no `Framebuffer` at all**, and since
  ADR-0038 that is correct rather than a gap: the front end has no scene to
  scale. What it still lacks is the grade and the aspect bars, which is a
  different want.
- **`race/capture.rs`'s ordinary (non-`--presented`) path builds no
  `Framebuffer` either** and draws the scene straight into the capture texture.
  A `--screenshot` is unaffected by anything on this page.

## The caveat that bounds what may set an extent

**No per-frame scene post-process is viewport-aware yet, and there is no safe
configuration of the settings that makes one.** Two different shapes of not
knowing:

- **FXAA, SMAA and FSR 1** are handed the **extent** as their input size while
  reading a view of the **allocation**-sized texture. Correct exactly while the
  two are equal, and wrong the moment they are not: each derives a texel size
  from the size it was given, so an extent below the allocation has them
  stepping the wrong distance across a texture larger than they think it is -
  and FXAA's and SMAA's own targets would be rebuilt on every extent change,
  which is the cost this whole structure exists to avoid.
- **`bloom` and `hd_bloom` take no size and no viewport at all.** They are
  handed the scene view and nothing else, so a short extent would have them
  blurring the undrawn region inward as a dark edge along the extent's own
  boundary. Bloom is its own setting and is on in an ordinary race - which is
  why "turn the upscaler and the anti-aliasing off" is **not** a safe
  configuration, and why the rule below is unconditional rather than a list of
  rows to set to `off`.

So, until those passes take a viewport and a resource size separately:
**nothing outside a test may set an extent below the allocation.** The two
tests that do build a bare `Framebuffer` with no `Scene` behind it, so there is
no post chain for them to reach. `Framebuffer::set_extent` says so on itself.

`motion_blur` is the one pass already shaped right, and is the shape the others
need: `motion_blur::Frame` carries the attachments' `size` and the drawn
`viewport` as separate fields and takes the correct one for each. The
restoration that starts on the rest is un-folding `fsr1::Constants::new`'s
viewport and size arguments back to the two `ffx_fsr1.h` has, which moves the
port *closer* to upstream and so keeps ADR-0012's transliteration property
intact.

## There is no scissor, and that is not an omission

The extent is applied with `set_viewport` alone. A scissor would add nothing:
NDC clipping already bounds every rasterized fragment to the viewport
rectangle, so no draw can reach outside the extent without one. The one thing
neither a viewport nor a scissor bounds is `LoadOp::Clear`, which clears the
whole attachment either way - see ADR-0037's consequences.

## What is still missing, in order of expense

**A cost signal.** The only frame measurement that exists is a wall-clock
interval between loop iterations, fed to `perf::Meter`. Under `Vsync::On` it is
pinned to the refresh and under any `FrameLimit` it is pinned to the limit,
because the frame loop sleeps to it - so it tracks real work only with vsync off
*and* no limit, which is a diagnostic configuration rather than a shipping one.
A GPU timestamp (`wgpu::Features::TIMESTAMP_QUERY`, behind the adapter probe and
fallback chain ADR-0012 already mandates) written either side of the scene
passes is the primary path, not a refinement. Two things it will need said out
loud: the reading resolves a frame or more late, so a controller acts on stale
cost and its cooldown must cover the frames in flight; and where there is no
timestamp support dynamic resolution is off, and the menu row says so through
the existing `disabled_by` mechanism rather than advertising a controller driven
by a signal pinned to the refresh.

**A controller.** A pure function of fed measurements, the way `perf::Meter`
already is - `record(cost)` in, a scale out, never reading a clock or a GPU
itself, which is what makes it testable without one. The conventional policy,
and it should stay conventional: correct by `sqrt(budget / measured)` because
cost scales with pixel count; a deadband so a frame sitting comfortably inside
budget does not twitch; a clamped per-frame delta that is asymmetric, falling
fast because a dropped frame is already visible and rising slowly because a rise
that overshoots costs a drop; quantized steps on a grid; and a cooldown after
any step.

**Settings and a menu row.** `[graphics] dynamic_resolution`, defaulting off -
the footing every enhancement here starts on - and a floor the controller may
not go below. `render_scale` keeps its meaning and becomes the ceiling, which is
what makes a row that disagrees with it worth warning about.

## Three properties a controller must not break

- **It never writes `settings.graphics.render_scale`.** It reads it as the
  ceiling and emits a separate runtime value. Persisting a controller's output
  into the player's settings file would make their chosen quality drift
  downward with every session on a loaded machine.
- **It never reaches the simulation.** Dynamic resolution is presentation-side,
  like `perf::Meter` and `FrameLimit`; the timestep stays fixed at 60 Hz per
  [ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md) whatever
  the resolution does.
- **Every capture path forces it off.** This project's comparisons are
  byte-identical screenshot diffs, and a controller driven by how busy the
  machine is makes every one of them irreproducible. `crate::perf` already makes
  exactly this argument for why the performance overlay is window-only; it
  applies verbatim. A load is not a frame either - `Session::stalled` exists for
  that and clears the meter, and a controller needs the same guard or a track
  load drives the scale to the floor and takes seconds to climb back.
