# Dynamic resolution

**Status: the structure exists, the controller does not.** `Framebuffer` carries
an allocation and a render extent separately, per
[ADR-0037](../architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md),
and every consumer of the old single `size` has been resolved to one or the
other. The cost signal exists too: the scene pass is timed on the GPU every frame in
the window, and the reading names the frame it was taken on. Nothing moves the
extent yet - there is no controller, and the scene-resolution post-processes
still take no viewport - so the extent equals the allocation on every frame the
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

## The cost signal, which exists now

**The scene pass is timed on the GPU, every frame, in the window.**
`oag_render::timing::PassTimer` is a ring of four timestamp pairs;
`Session::frame` claims one before the race stage encodes, hands it to the
scene pass through the pass descriptor's own `timestamp_writes`, resolves it
into the same encoder and reads it back after the submit without ever blocking.
What comes back is fed to `Session::scene_cost`, a second `perf::Meter` beside
the frame-interval one. **Nothing reads it yet** - there is no controller and
no overlay row - and it is fed anyway, so the signal a controller will be built
on is a measured thing rather than a planned one.

`mesh_render::optional_features` asks for `TIMESTAMP_QUERY` because of it,
intersected with the adapter's own and never demanded, exactly as
`TEXTURE_COMPRESSION_BC` already was. **Only the portable bit**: the two native
ones are a driver behaviour change nothing here needs, and bracketing a pass
through its own descriptor uses neither. That line reaches every
`request_device` in the workspace, so it was checked against the instrument
this project judges a renderer change with - the two `--presented` captures
(`--render-scale 50 --upscaler fsr1`, `--render-scale 100 --upscaler off`,
pulse-psp-usa, 300 ticks, 1440x816) are byte-identical either side of it.

The wall-clock interval `perf::Meter` still holds is what this replaces for
control purposes: under `Vsync::On` it is pinned to the refresh and under any
`FrameLimit` to the limit, because the frame loop sleeps to it, so it tracks
real work only with vsync off *and* no limit - a diagnostic configuration
rather than a shipping one.

### What is in the budget, and what is not

**The scene pass and nothing else**, and this is an under-measurement that is
recorded rather than hidden:

| Pass | In the budget | Why |
| --- | --- | --- |
| The `race` pass (`race/scene/frame.rs`) | **Yes** | The one pass whose cost falls with the render extent |
| `bloom`, `hd_bloom`, `motion_blur` | Not yet | They draw at scene resolution and belong in it. They take no viewport, which is the same thing that stops the extent moving at all - so they join the budget in Phase 5, when they gain one |
| FXAA, SMAA, FSR 1, the blit | **No, permanently** | The resolve draws at presentation size whatever the scale is |
| The HUD, the composite, the perf overlay | **No, permanently** | Presentation resolution since ADR-0036 and ADR-0038 |

Folding a fixed cost into a budget that exists to be divided by a moving one is
the error the split guards against, which is why the last two rows are
permanent rather than pending. Each pass is timed and summed rather than the
encoder being bracketed first-to-last: bracketing would need
`TIMESTAMP_QUERY_INSIDE_ENCODERS`, which is not WebGPU-portable, and would have
to know which pass is last - which varies with the bloom, motion-blur and
HD-chain settings.

### What the signal does in a running race

Measured 2026-09-02 in a window under Xvfb, NVIDIA RTX PRO 2000, `pulse-psp-usa
--race`, `RUST_LOG=oag_game=debug`: 5,306 consecutive readings, every one
resolving **exactly one frame late**. The mean scene pass over frames 100..400:

| `--render-scale` | Mean scene pass |
| --- | --- |
| 50 | 0.226 ms |
| 100 | 1.556 ms |
| 200 | 2.893 ms |

**The claim is the trend and not the exponent.** These are three separate runs,
and a faster run is further along the track at the same frame number, so the
world state differs between them - the three points do not admit one
fixed-plus-per-pixel fit, which is evidence that the runs are not comparable
rather than evidence about the renderer. What they do establish is that the
timer measures the part that scales, which is what it was wired up to do. A
controlled measurement needs the world pinned, and the capture path - which is
deterministic per tick - deliberately gets no timer.

Two things a controller built on this will need said out loud: the reading
resolves a frame or more late, so it acts on stale cost and its cooldown must
cover the frames in flight; and where there is no timestamp support dynamic
resolution is off, and the menu row says so through the existing `disabled_by`
mechanism rather than advertising a controller driven by a signal pinned to the
refresh.

### Three properties the timer holds, and why each is load-bearing

- **A reading names its own frame**, rather than being assumed to describe the
  last one. That is what makes the load guard possible at all: `meter.clear()`
  cannot reach a measurement still somewhere between the queue and the map, so
  `Session::stall_frame` drops every reading up to and including the frame that
  carried the load. A controller reading only "the newest value" could not.
- **Nothing blocks.** Waiting on the map would drain the pipeline being
  measured. A frame with no free slot is simply not measured - a gap in the
  samples, where a reused slot would be a wrong number in them.
- **No capture path is timed.** A capture has to be reproducible rather than
  fast; see the trap on the thread. `OAG_RENDER_BENCH`'s figure is the CPU-side
  encode cost, which is a different number about a different thing.

### The row that tells this from a stutter

The `dev` performance overlay names the size the scene was drawn at, under the
draw counts: `RENDER 1440x816` while the extent is the whole allocation - every
frame today - and `RENDER 1216x688 OF 1440x816  84%` once something moves it.
`perf::RenderSize`, and `None` on a stage with no scene, so the menus never
name a texture nothing on screen came from.

**Without it a controller is invisible in the one way that matters.** A frame
rate that recovers because the resolution dropped and one that recovers because
the load passed are the same graph; the row is the difference. It is readable
*while* the scale moves, which is exactly what
[ADR-0036](../architecture/adr/0036-ui-composites-at-presentation-resolution.md)
bought by taking this overlay out of the scaled target - under the old shape it
would have been re-rasterised at a new size every few frames, which is the one
element a player is reading rather than looking at.

### What the probe actually found

Measured 2026-09-02 on this project's development machine, both adapters, by
`cargo nextest run -p oag-render -E 'test(/timing::/)' --no-capture` - which is
the command that reproduces the table rather than a transcription of one run.
Every number below came from resolving a real query pair around a cleared
256x256 pass, not from reading a feature bit:

| Adapter | Backend | Features | Tick period | A cleared 256x256 pass |
| --- | --- | --- | --- | --- |
| Intel(R) Graphics (ARL), integrated | Vulkan | all three | 52.083332 ns | 12,708 ns |
| NVIDIA RTX PRO 2000 Blackwell Laptop GPU, discrete | Vulkan | all three | 1 ns | 352 ns |

"All three" is `TIMESTAMP_QUERY`, `TIMESTAMP_QUERY_INSIDE_ENCODERS` and
`TIMESTAMP_QUERY_INSIDE_PASSES`. Only the first is needed to bracket a pass, and
only the first is WebGPU-portable; a controller should ask for no more.

Three things this settles:

- **The fallback chain has a real branch and it is not hypothetical.** Both
  adapters here support it, so the *adapter*-without-the-feature branch cannot
  be exercised on this machine - it needs a deliberately downgraded adapter or
  another machine, and a green run here is not evidence that it works. The
  **device**-without-the-feature branch *is* covered, and by the simplest
  possible means: `a_device_without_the_feature_gets_no_timer` requests a device
  asking for nothing and asserts `PassTimer::new` hands back `None` rather than
  a timer that would fail validation on its first pass. That is the distinction
  `PassTimer::new` was built around - it checks the device it was given rather
  than trusting what the adapter advertised. The greyed-out menu row is still
  untested, because there is still no row.
- **The tick period differs by 52x between two adapters in the same laptop**, so
  a cost budget must be computed from `Queue::get_timestamp_period` every run
  and never from a constant. It also means a probe on one adapter says nothing
  about the other, which is why the test loops over all of them rather than
  taking whichever `request_adapter` picks.
- **Granularity is not the limit.** The coarser of the two is 52 ns, against a
  16.67 ms frame budget - about 320,000 ticks. What bounds a controller is the
  latency of the reading, not its resolution.

## What is still missing, in order of expense

**A controller.** A pure function of fed measurements, the way `perf::Meter`
already is - `record(cost)` in, a scale out, never reading a clock or a GPU
itself, which is what makes it testable without one. The conventional policy,
and it should stay conventional: correct by `sqrt(budget / measured)` because
cost scales with pixel count - **which is assumed and not yet measured here**,
see the caveat under the running-race table above; a deadband so a frame
sitting comfortably inside
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
