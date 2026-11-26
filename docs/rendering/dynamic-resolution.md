# Dynamic resolution

**Status: it works, and it is off by default.** `target_fps` names a
target frame rate, `minimum_resolution` bounds how far the picture may
shrink, and `render_scale` becomes the ceiling. On Wipeout HD it also takes
`original`, the console's own width throttle
(["A title's own width throttle"](#a-titles-own-width-throttle)). All three live in
`[render_profiles.<title> (<platform>)]` and so are **per (title, original
platform)**, which is the right table for them: a target Pulse holds
comfortably is not one HD/Fury holds, and a PS2-sourced Pulse race is not the
same scene as a PSP-sourced one either - the whole point of the feature is
that the answer depends on how expensive the scene is - the controller reads
it and never writes it. Every scene-resolution pass takes a resource size and a
viewport separately, the scene pass is timed on the GPU every frame, and
`crates/present/src/drs.rs` turns the second into the first.

A build with the whole feature in it produces **byte-identical frames** to one
without, at the default. That is measured rather than argued: six `--presented`
captures either side of each commit, covering FSR 1, FXAA, SMAA, the PSP bloom
with the effect actually contributing, and an HD/Fury circuit.

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
| `app/handler.rs`, building the first `Framebuffer` | Allocation | `target_size` off the ceiling, before any frame has been drawn |
| `app/handler.rs`, `Stage::race(..., framebuffer.allocation(), ...)` | Allocation | Builds the scene's depth and MSAA attachments, which must match the colour texture |
| `app/session/frame.rs`, `framebuffer.resize(wanted)` | Allocation | `wanted` is `target_size` off the `render_scale` ceiling |
| `app/session/frame.rs`, the three `scene.resize(...)` arms and the suspended race | Allocation | Same validation rule: a depth attachment sized for the colour one |
| `app/session/frame.rs`, `inside` handed to every stage | **Extent** | It is a viewport. Everything downstream that normalises against it - the projection, and [ADR-0039](../architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)'s jitter via `jitter::matrix(frame, (viewport.2, viewport.3))` - inherits the extent for free |
| `app/session/load.rs`, `build_race_stage(..., allocation, ...)` | Allocation | Builds attachments, as `app.rs` does |
| `app/session/load.rs`, `warm_up`'s rectangle | **Extent** | It is a viewport. Warming pipelines is keyed on format, not on the size of the attachment they first meet - see ADR-0038 |
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

## Every scene pass carries a resource size and a viewport

`motion_blur` was the only pass already shaped right and is the shape the rest
took: `Frame` carries the attachments' `size` and the drawn `viewport`
separately and each pass takes the correct one for each. `post::sub_rectangle`
is the shared scale-and-clamp, and it returns **exactly `1.0` on both when the
two sizes are equal, by construction** - the same argument
`upscale::blit::Source::WHOLE` already made for the final blit, and the reason
none of this moves a byte at the ceiling.

| Pass | What it takes now | Note |
| --- | --- | --- |
| `fsr1` | `viewport` and `input` separately | A **restoration**: `FsrEasuCon` takes `inputViewportInPixels` and `inputSizeInPixels`, `con0` off the first and `con1`..`con3` off the second. This port had folded them into one argument, so un-folding moves the diff against `ffx_fsr1.h` closer and keeps [ADR-0012](../architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)'s transliteration property |
| `fxaa`, `smaa` | `size` (the allocation) and `viewport` | **Their targets size off the allocation**, which is the point: an extent moving several times a second must not rebuild them. Taps step texels of the resource and clamp half a texel inside the drawn edge |
| `bloom` | A scene `size` and `viewport` | The scratch buffers stay a fixed 240x136, so the bright pass maps the extent onto the whole buffer and the blur radius stays about 2 % of the drawn frame at any scale. The composite writes through a viewport |
| `hd_bloom` | The same, applied down the ladder | Each level's viewport is its own size times the one fraction, so a single UV scale covers scene, half, quarter and every luminance halving |
| `motion_blur` | Unchanged | It was already right |

Two things worth carrying forward from doing it:

- **SMAA's search loops need no clamp, and that is a property of the clear
  rather than luck.** `LoadOp::Clear` is not viewport-restricted, so the edges
  texture outside the extent is zero - and a search terminates on zero edges
  exactly as it does at the frame border. Only the colour taps are clamped,
  which is where a dark fringe would otherwise show. It is said in the shader so
  nobody "fixes" it.
- **HD's blur tap spacing follows the viewport, not the resource.** The
  executable measures `Bloom vertical size` against its quarter *frame* - its
  hard-coded `1/480` and `1/270` are a 1080p frame's quarters - and under a
  short extent the frame is the extent. Left on the resource the glow would
  widen as a fraction of the picture every time the scale fell. No recovered
  constant moves; the divisor is the same formula read at the right size.

### The trap in the resolve, which no capture can catch

`resolve_scene` chose its source rectangle with `match source { Some(_) =>
Source::WHOLE, None => Source::of(..) }` - that is, "did a pass run". **The
question is which size the bound view is.** FSR 1 resolves to the presentation
rectangle and hands back a texture exactly that size; FXAA and SMAA hand back
an allocation-sized one with the extent in its corner. Both arms agree while
the extent is the allocation, so every `--presented` capture passes either way
and the bug would have surfaced the first time a controller stepped the scale -
as a picture stretched by the allocation-to-extent ratio, long after the change
that caused it. `upscale::resolved_source` is a named function for exactly that
reason, and `only_an_upscaled_frame_is_read_whole` is the only guard there is.

### MSAA resolves the whole attachment, and that is the safe answer

Measured, not reasoned: `crates/mesh/tests/msaa_resolve_viewport.rs` primes a
resolve target blue, clears a 4x multisampled attachment red, draws green into
a quarter-sized viewport of it and reads the resolve back. Inside the viewport
it is green; **outside it, red** - the pass's own clear, not the blue that was
there before.

So under `msaa = "4x"` a short render extent leaves the region
outside it holding the multisampled attachment's `LoadOp::Clear`, which in a
race is black with alpha zero - exactly what the non-MSAA path's clear leaves
there, so every sampler's half-texel clamp covers both identically. The answer
that would have been a problem is the other one: a viewport-restricted resolve
would leave the *previous, larger* frame outside the extent, which is the
classic stale fringe and a materially worse thing to defend against.

No `--presented` capture can distinguish the two, because a capture never moves
the extent. That is why it is a test.

What it does cost is real: the resolve does full-allocation work whatever the
extent is, the same way `LoadOp::Clear` is not viewport-restricted. Both bound
what dynamic resolution can save at a low scale, and both belong in the budget
conversation rather than in a surprise.

### One residual, stated rather than hidden

EASU's twelve-tap kernel reaches about two texels past its sample point, so at
the right and bottom edge of a short extent it reads the cleared region.
**Upstream has the same property** and answers it with `inputSizeInPixels`
alone; adding a clamp `ffx_fsr1.h` does not have would spend ADR-0012's
transliteration property to fix an artefact nobody has yet reported seeing.

## There is no scissor, and that is not an omission

The extent is applied with `set_viewport` alone. A scissor would add nothing:
NDC clipping already bounds every rasterized fragment to the viewport
rectangle, so no draw can reach outside the extent without one. The one thing
neither a viewport nor a scissor bounds is `LoadOp::Clear`, which clears the
whole attachment either way - see ADR-0037's consequences.

## The cost signal, which exists now

**The scene pass is timed on the GPU, every frame, in the window.**
`oag_gpu::timing::PassTimer` is a ring of four timestamp pairs;
`Session::frame` claims one before the race stage encodes, hands it to the
scene pass through the pass descriptor's own `timestamp_writes`, resolves it
into the same encoder and reads it back after the submit without ever blocking.
What comes back is fed to `Session::scene_cost`, a second `perf::Meter` beside
the frame-interval one, and the same reading goes to `Session::drs`.

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

### What is timed, what is measured-and-fixed, and what is left to the residual

Three categories since [ADR-0042](../architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md)
and [ADR-0043](../architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md),
replacing the two-way "in the budget or not" split this section used to draw.
The third category is no longer a constant: since
[ADR-0044](../architecture/adr/0044-the-residual-is-a-learned-upper-bound.md)
`RESIDUAL_SHARE` is where the reserve *starts* and the most it may ever be,
and the reserve itself is a `drs::Residual` learned per session - see "The
residual learns" below:

| Pass | Category | Why |
| --- | --- | --- |
| The `race` pass (`race/scene/frame.rs`) | `drs::Cost::scalable` | Falls with the render extent, timed every frame |
| `motion_blur` | `drs::Cost::scalable` | Also falls with the extent; timed as one chain across six render passes since ADR-0042, via `PassTimer::half_writes`. Since 2026-10-25 each frame records whether the chain encoded (`session::timing::Drawn`), so a reading that did not come back from a frame that drew the blur is a skipped frame, and only a frame that drew none reads the blur as free |
| `hd_bloom` | `drs::Cost::scalable` | Also falls with the extent; timed as one chain the same way since ADR-0043. `Scene::has_hd_bloom` tells "ring full" from "no chain to measure" - Pulse, Pure, and an HD circuit with no `HDR and Bloom` block all have none |
| The FSR 3.1 chain's six render-resolution dispatches | `drs::Cost::scalable` | Also fall with the extent; timed as their own compute pass (`Session::upscale_cost`) since [ADR-0045](../architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md) |
| The FSR 3.1 chain's `accumulate` and `rcas` | `drs::Cost::fixed` | Timed (`Session::upscale_presented_cost`), and the one part of the frame that genuinely does not fall with the extent: both run at presentation size. Until ADR-0045 the *whole* chain was counted here, which was deliberately conservative and, on a Steam Deck where the chain costs 4-5 ms, no longer a small thumb on the scale |
| `bloom` (Pulse's, PS2's) | The residual | Timed for the overlay and the frame log only (`PSP BLOOM`, `PS2 BLOOM`, see "Every other pass has a row of its own"), never in `drs::Cost`. Draws at a fixed 240x136 regardless of scale, so - unlike `hd_bloom` - it does not belong in `scalable` even once it is timed |
| FXAA, SMAA, FSR 1, the blit, the HUD, the composite, the glass and shadow passes, the probe blend, the Omega tonemap, the HD zoom, photo fx | The residual | Timed for the overlay and the frame log only; **not in `drs::Cost`** (they are presentation-resolution, fixed-size or off the extent path, or - the glass and shadow passes - not yet shown to scale with it). Their time is in the controller's residual reading all the same, which is why the overlay's `OTHER` shows less than it |
| The perf overlay, AI/physics for opponents beyond the player | The residual | Driver overhead, or CPU-side work off the render-extent path entirely - permanently outside the extent's reach |

Each pass is timed and summed rather than the encoder being bracketed
first-to-last: bracketing would need `TIMESTAMP_QUERY_INSIDE_ENCODERS`, which is
not WebGPU-portable, and would have to know which pass is last - which varies
with the bloom, motion-blur and HD-chain settings.

The `dev` overlay's own GPU-cost panel - top left, separate from the
frame-time panel at top right - prints one row per timed reading, `SCENE`,
`BLOOM`, `BLUR`, `FSR3`, each only once it has a reading, then **one row per
separately-encoded pass** (below), followed by an `OTHER` row:
`perf::Meter`'s own wall-clock frame time minus whatever the rows above add up
to. That is the residual made visible rather than assumed - see
`perf::GpuCost::rows` and `residual_ms`, and, for the same subtraction fed back
into the budget instead of only printed, "The residual learns" below. Its own panel rather
than a block inside the frame-time one, so a reader is not holding five
numbers on one crammed line.

**Every other pass has a row of its own** (`perf::PassRows`,
`oag_gpu::timing::PassTimer::with_passes`): each render or compute pass that is
not inside `SCENE`'s pass or one of the chains above takes a pass-boundary
timestamp pair of its own (`RenderPassTimestampWrites`, valid on tile-based
GPUs, unlike a timestamp between draws), and its label names the row. The rows,
in the overlay's order: `GLASS` (the behind-the-glass and sea reflection
target), `ADVERT CARDS`, `DISTORT`, `SHADOW MAP`, `SELF SHADOW`, `OCCLUSION`,
`PROBE BLEND`, `LENS FLARE`, `PS2 BLOOM`, `PSP BLOOM`, `SMAA`, `FXAA`, `FSR1`,
`FSR3 CLEAR`, `HD ZOOM`, `OMEGA TONEMAP` (its bloom too), `WO COMPOSITE`,
`PHOTO FX`, `COMPOSITE`, `BLIT` (the plain upscale pass), `BACKDROP` and
`MENU SCENE` (the front end's) and `HUD AND UI` (the HUD, its countdown and
glow mask, the menus, this overlay). **On HD the grade and encode are the last
pass of `BLOOM`'s chain**, so they are in `BLOOM` and have no row of their own
(one attribution per pass). Passes that share a label (the shadow map is one pass per layer)
are summed. Each row has two means over the last 120 frames. **The overlay shows the mean
over only the frames the pass ran in**, so a pass that runs a second in every few (the HD zoom
ring, the photo effects, the glass outside the tunnel) reads its cost and not a fraction of it;
a row appears while the pass ran in the window and that mean is at least 0.1 ms, and goes
when no frame of the window ran it. `OTHER` and the frame log's `pass_ms` use the **per-frame**
mean, a pass that did not run in a frame costing that frame zero, so `frame - timed` stays
honest (`perf::PassRows::total` against `ordered_when_run`). A row under 0.1 ms is left off the
overlay and kept in the frame log. A pass
recorded on another thread (each view of a split screen) is not timed, so a
**split screen shows no per-pass rows**, and neither does a race with further
windows. `OTHER` subtracts all of them, hidden rows included.

**`OTHER` is not exactly the controller's reading any more.** The controller
learns from `frame - drs::Cost::total()`, and `drs::Cost` stays the five
readings it was fed before these rows (`SCENE`, `BLUR`, `BLOOM` and the two
`FSR3` halves): the labelled passes never reach `session::timing::assemble`.
The overlay's `OTHER` is that reading less the labelled passes' sum
(`the_overlays_other_is_the_controllers_reading_less_the_labelled_passes`).
What they do not time is the parts **inside** `SCENE`'s pass (track, craft,
particles, sky, fog), which a tile-based GPU cannot split.

**A `FRAME` row and three indented rows under it sit below `OTHER` since
2026-09-07, and none of them is a GPU timestamp**: `CPU`, `PRESENT` and
`OUTSIDE`, from two `Instant`s in the frame loop rather than from any adapter
feature, so a machine with no `TIMESTAMP_QUERY` at all still gets a breakdown
of its frame. They partition the wall-clock frame time exactly - the frame
body, the two swapchain calls inside it that may block
(`get_current_texture` and `present`, which is where a `Vsync::On` wait
lands), and everything outside it, the frame limiter's own
`ControlFlow::WaitUntil` above all. That is what says which of the three
possible causes a large `OTHER` actually is: idle time, CPU work, or GPU
passes nothing brackets.

Two things about that layout are deliberate. The indent hangs off `FRAME` and
not off `OTHER`, because the three sum to the wall-clock frame time and not to
the residual, which has the timed passes subtracted out of it and overlaps all
three; nesting them under `OTHER` would put a visibly wrong sum on an
instrument. And the third row is spelled `OUTSIDE` rather than `SLEEP`:
`App::about_to_wait` only waits while the deadline is still ahead, so a loop
already at or behind its limit takes the `Poll` branch and the same gap is
event-loop dispatch rather than sleep. See `perf::CpuCost`.

They are **display-only** and deliberately not fed to `drs::Cost`: that type
splits GPU cost by whether the render extent moves it, and a wall-clock
reading answers a different question. What they make possible is recorded
under "The residual learns" - the sleep term ADR-0044 can only infer is now
measured directly.

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

### Four properties the timer holds, and why each is load-bearing

- **A reading names its own frame**, rather than being assumed to describe the
  last one. That is what makes the load guard possible at all: `meter.clear()`
  cannot reach a measurement still somewhere between the queue and the map, so
  `Session::stall_frame` drops every reading up to and including the frame that
  carried the load. A controller reading only "the newest value" could not.
- **Nothing blocks.** Waiting on the map would drain the pipeline being
  measured. A frame with no free slot is simply not measured - a gap in the
  samples, where a reused slot would be a wrong number in them.
- **A ring is drained, never read one reading at a time.** Since ADR-0042 the
  frame loop reads four rings and trusts only readings that name the same
  frame, and that pairing has a failure mode a single ring never had. The map
  callbacks for one submission fire in whichever `device.poll` runs after the
  GPU finishes it, and the loop polls once per ring in turn - so a completion
  landing between two of those polls reaches the later ring on this frame and
  the earlier ring on the next. If the earlier ring then hands back one
  reading per frame, it carries a backlog of one from that moment on: it
  returns the older reading on every call, its reading is a frame behind the
  other three's on every call, and nothing short of a frame with no arrival
  at all - which a vsync-paced loop at a steady rate never has - resyncs
  them. Every frame after that is a mismatch, `Cost::fixed` reads as
  "not measured yet", `Controller::record` is never called, and the render
  scale freezes wherever it was. `PassTimer::drain` takes everything that is
  ready in one call, so a backlog cannot form and two rings can disagree by at
  most the one frame the race itself costs, and are back in step on the next.
  `Session::read_timing_and_feed_drs` feeds every drained scene reading in
  order, oldest first, since a backlog of readings is a backlog of frames and
  the controller's cooldown counts readings as frames.
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

## The controller

`crates/present/src/drs.rs`, and it reads nothing: fed one `drs::Cost` a frame - the
scalable readings summed, the fixed reading carried separately - it emits a
rectangle. Never a clock, never a GPU, never a settings file - the argument
`perf.rs` already makes for why a presentation-side module is not a
determinism problem, and the same shape, so every number in it is testable
against a sequence somebody chose.

### The budget subtracts what it can measure

Not derived from the wall-clock frame interval, and that part of the original
design is unchanged: a controller fed headroom from `interval - scene` is inert
under vsync or any frame limit, because the loop *sleeps* to the target, so the
ratio is 1.0 at every render scale and nothing ever moves - in precisely the
configuration a player turns the feature on for. It would also put a clock
inside the one module whose selling point is that it has none. See
[ADR-0040](../architecture/adr/0040-the-dynamic-resolution-budget-is-a-share-of-a-frame.md).

What changed, in [ADR-0042](../architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md):
`drs::SCENE_SHARE`, a single constant standing in for everything the scene pass
was not, is gone. The budget is now `period - fixed - residual`, where `fixed`
is `drs::Cost::fixed` - FSR 3.1's own GPU timing, already measured and
previously discarded, and since
[ADR-0045](../architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md)
only the presentation-resolution half of it - and the residual opens each
session at
`RESIDUAL_SHARE = 0.20` of the period and covers what genuinely cannot be reached by a render-extent change: the HUD,
the composite, the blit, the perf overlay, the MSAA resolve, and, per
[ADR-0043](../architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md), the
AI and physics cost of every opponent beyond the player - `hd_bloom` itself
joined `Cost::scalable` in that same ADR rather than staying here. A single
constant could not serve both a profile with FSR 3.1 and MSAA and one with
neither - measured, the scene pass's own share of the frame ranged from 23 % to
39 % across four render
profiles on one circuit, which is not a range one constant can be.

### The residual learns, and can only ever tighten

`RESIDUAL_SHARE` was measured on one circuit, on one adapter, with one grid
size, and both ADRs that set it said so. Reported from play on a second
adapter - a Steam Deck (AMD, RADV) at a 90 Hz target - it is wrong by enough to
matter: the machine holds 90 FPS at the full render scale, and the controller
shrinks it to 60-65 % and settles there. Over-reserving does not drop a frame;
it takes pixels away quietly, which is why nothing caught it earlier.

[ADR-0044](../architecture/adr/0044-the-residual-is-a-learned-upper-bound.md)
makes the reserve a `drs::Residual`: an estimate in seconds that starts at
`RESIDUAL_SHARE * period` and is tightened by frames that can prove a smaller
one. The composition root hands `Controller::record` the frame's own wall-clock
duration - the same `elapsed` `perf::Meter` is fed, and `None` for a frame that
carried a load - so the controller still never reads a clock.

Two facts make that safe, and both are properties rather than choices:

- **Every reading is an upper bound.** `reading = frame - timed = residual +
  sleep`, and sleep is never negative, so an observation cannot claim the
  machine spends less outside its timers than it does - only less than the last
  guess did.
- **The gate is the no-slack condition solved for the reading.** Under pacing,
  `reading <= reserve` is the same statement as `scalable >= budget`: a frame
  whose scalable cost had already filled its budget, with no slack for the loop
  to have slept away. So a reading below the reserve is always safe to fold in,
  and one above it is only trusted when the frame ran past the target period by
  `OVERRUN` (5 %, jitter's width) - which is a frame the loop demonstrably did
  not sleep through.

`RESIDUAL_SHARE` caps whatever comes out, so the reserve can only ever shrink
relative to what shipped before: it is a floor under the *budget*, never a
floor under the reserve. `a_learned_residual_never_reserves_more_than_the_constant`
asserts that over a grid of targets, fixed costs and learned values.

**What it buys, and where it stops.** `drs::tests::machines` simulates the
reported machine frame by frame against the real controller - a model of a
report, labelled as one, with the split inside the scene pass chosen to put
the constant's settling point where the report puts it:

| Pacing | Constant | Learned | Reserve reached |
| --- | --- | --- | --- |
| Vsync at the target rate | 0.80 scale | 0.90 | 1.61 ms of 2.22 |
| Frame limiter at 240, target 90 | 0.80 scale | 0.95 | 0.50 ms - the truth |

A loop sleeping to the very rate the controller aims at destroys the evidence
before the controller sees it: once it has bought back enough slack to be
comfortable, every reading is inflated by the sleep it just created and the
gate refuses all of them. Under a limiter above the target - the shipped
default is 240 - the work is what is left on the clock and the estimate
converges exactly. Closing the vsync gap needs the untimed work *timed*, which
is a fifth ring and a CPU-side one.

The estimate is in memory for the session and carries no `Serialize`: it is
what a machine is doing this run, not a preference, and `drs` cannot reach
`crate::settings` to write one anyway. It survives `Controller::reset` for the
same reason the scale does - a load is the one moment its evidence is
worthless, not a reason to unlearn a race's worth of it.

Because the budget now moves within a session, the `dynamic resolution:` trace
line and the `out of reach` warning both print the reserve in force and whether
it is learned or assumed. Two lines a minute apart can otherwise divide the
same scalable cost by two different budgets with nothing saying so.

### The policy

Conventional, and it should stay so: correct by `sqrt(budget / measured)`,
because cost is per-pixel and the scale is per-axis; a deadband so a
comfortable frame does not twitch; an asymmetric clamp; a grid; a cooldown
covering the frames in flight.

Three things that are not obvious, all three found by running it rather than by
reading it:

- **Both delta clamps are counted in grid steps.** A 4 % rise ceiling against a
  5 % grid is a controller that can never rise at all - from any grid point,
  `scale * 1.04` floors straight back onto the point it started from, forever.
  Counting in the grid's own units makes that unrepresentable rather than a
  pair of numbers to keep consistent by hand.
- **The scale is an integer count of grid points.** `0.5 / 0.05` is not exactly
  `10.0` in `f32`, and deriving the current point by dividing floored a
  legitimate one-step rise back onto its own point at some scales and not
  others.
- **A rise has to be earned by a run of frames, not just survive a cooldown.**
  Measured: with a cooldown alone, a 4K race aiming at 144 changed resolution
  **143 times in a minute**, flipping between 100 % and 95 %. Every individual
  decision was right; the scene pass genuinely costs 0.75 to 1.07 of its budget
  at one fixed scale as the camera moves. `RISE_PATIENCE` consecutive
  under-budget frames, with anything comfortable or over budget breaking the
  run, took the same scene to **49** changes and a trajectory that falls to
  70 % and climbs back through 75, 80, 85, 90. A fall still answers a single
  frame: it has already been seen.

At a target the machine can only just hold, the scale settles between two
adjacent grid points and keeps stepping between them. That is what a closed
loop on a marginal load does, and the honest answer is a lower target rather
than a cleverer policy.

**Two more things, both found once [ADR-0042](../architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md)
made the true scalable cost visible for the first time:**

- **A rise has to survive its own overshoot, not just clear the deadband.** One
  grid step is a fixed fraction of the *ceiling*, so it is a growing fraction of
  *cost* as the scale falls - about 10 % at the top of the range, about 21 % at
  half scale, against a 15 %-wide deadband. Below roughly 70 % scale a rise the
  deadband permits can land outside it on the other side and get corrected back
  next frame: the same two-point ping-pong `RISE_PATIENCE` exists to prevent,
  reappearing lower on the grid. A candidate rise is now refused unless
  `ratio * (next / here)^2` still clears the deadband's own floor - checked
  against the floor rather than the middle, because the quadratic cost model is
  itself optimistic (real frames carry per-frame cost that does not scale with
  pixels). Measured on the report this ADR was written from: 176 changes in 41
  seconds before this guard, 83 after.
- **An unreachable target is earned by a run, not declared on one frame.** When
  the measured fixed cost alone fills more of the frame period than the
  residual leaves room for, or the scale is parked at the floor and
  still over budget, no render scale can deliver the target - and the
  controller must not walk toward the floor chasing a rate that will never
  arrive, the same failure `Target::at_most` already guards from the clamping
  side. **Superseded for the fixed-cost half (2026-10-19, ADR-0073):** holding
  there left a 4K FSR 3.1 profile at full resolution whatever its minimum said;
  the controller now falls to the floor and still reports it (next section). `Controller::unreachable()` requires `RISE_PATIENCE` consecutive frames
  of "nothing left to decide" before it reports true, for the same reason a
  rise needs earning: at a target the machine can only just miss, the
  underlying condition flickers between frames, and a caller that logged every
  flicker would print continuously about a situation that had not changed.
  Measured: 68 raw per-frame occurrences in 41 seconds, 7 after requiring a run.

### The rows

`[render_profiles.<title> (<platform>)] target_fps` names the rate and `off`
is one of its values -
one row, one answer, and no second key that can disagree with it.
`minimum_resolution` is a `display::Scale` off the same list
`render_scale` offers, so the two read against each other.

The floor row warns when it is at or above the render scale, and the pairs are
**enumerated** in `menu.toml`: `Condition` compares values and has no ordering,
deliberately, because that is what keeps the menu module ignorant of what any
setting means. Thirty-six hand-written comparisons is the kind of list that
drifts, so `the_floor_warns_at_exactly_the_scales_it_cannot_fall_below`
generates the same set from `Scale::OFFERED` and demands they match.
`drs::Limits::new` is the other half: it brings the floor under the ceiling
where the two first meet, because the warning tells a player rather than
stopping them and `Ord::clamp` panics when `min > max`.

**RENDER SCALE gets no warning**, decided 2026-09-02 and still true.

### A ceiling is not a drawn size, and three warnings had to learn that

`render_scale` used to be the size the frame was drawn at, and three warnings
on the graphics page were written against that reading. Once a controller can
go below it they are wrong, and one of them wrong in the worst direction -
`fsr1` at a 100 % ceiling with a 50 % minimum says *"NO EFFECT AT RENDER
SCALE 100 OR ABOVE"* while FSR 1 magnifies every frame. A working setting
reported as dead.

What each really wants is *the lowest size the frame may be drawn at*, which is
`render_scale` when the controller is off and the floor when it is not.
`Condition.all` is an AND and that is an OR, so each is **two entries sharing
one message**: one requiring `target_fps = off`, one requiring the floor high
enough that the statement holds anyway.

All three were on two rows when this was written. Since
[ADR-0041](../architecture/adr/0041-one-row-for-what-resolves-the-frame.md) the
middle one is **gone** rather than fixed - FXAA and FSR 1 are values on one
axis now, so "fights the upscaler" is a combination nobody can select - and the
other two share the RECONSTRUCTION row.

| Warning | Was wrong when | Now |
| --- | --- | --- |
| `fsr1`, *"no effect at 100 or above"* | The controller drops below 100 from a 100+ ceiling | Fires only with the controller off, or a floor at 100+ |
| `fxaa`/`smaa`, *"fights the upscaler below 100"* | The ceiling is 100+ but the floor is not - a **missing** warning rather than a wrong one | Deleted with ADR-0041: unrepresentable |
| `fxaa`/`smaa`, *"redundant on top of 200 % supersampling"* | The controller stops supersampling | Fires only with the controller off, or a floor at 200 |

`a_warning_about_the_drawn_size_reads_the_floor_and_not_the_ceiling` pins the
floor halves against `upscale::magnifies`, the same guard the `off` halves were
already pinned to.

### `default` follows the output

The TARGET FPS row offers `default` after the fixed rates, stored as the word
`"default"` in `[render_profiles.<title>]`. It is resolved each frame at
`Session::render_limits` (`crates/game/src/app/session/draw.rs`) with
`drs::Target::resolve(display.hz())`, so the controller only ever sees a
rate: the refresh the window's output reports (re-polled about once a second,
so moving the window to another monitor retargets), or 60 when the platform
reports none (logged at `debug`). The limiter and display clamp below still
apply on top. The row draws as `DEFAULT (144)` with the rate in force, set
when the menus open and again when the rate changes. No `warn_when` pair is
declared for it: its rate is the output's own.

The FRAME LIMIT row ends with the same idea, `refresh_rate` (drawn
`REFRESH RATE (144)`): `perf::FrameLimit::resolve` in
`crates/game/src/app/session/pacing.rs` limits the loop to the output's
rate, or 60 when none is known. The audio buffer sizing at startup reads the
configured limit before a window exists, so it treats it as unlimited.

### The target is held to the frame limiter and to the display

Aiming above the limiter asks for frames the loop is not allowed to produce. At
`target_fps = 144` behind a 60 limit the budget is 2.4x too tight, so the
controller drops the resolution permanently to hold a rate the limiter forbids
whatever it does - every pixel given up buys nothing. The display is a bound of
the same kind: a Steam Deck docked to a 60 Hz TV under gamescope holds the output
to 60 whatever the limit says.

`drs::Target::at_most` is the clamp, applied through `refresh::held_target` to
`Session::limiter_hz` and to the display rate. The menu row's own `warn_when` is the visible half for the
limiter pairing only: it reads settings, so it cannot see a runtime refresh. The
display clamp is logged once per change at `debug` (`display refresh in force`).

**Where the display rate comes from** ([ADR-0084](../architecture/adr/0084-the-target-is-held-to-what-the-display-shows.md),
`oag_present::refresh`):

- *The report*: the output's refresh, asked again once a second. On X11 (Game Mode
  included) that is RandR, read on a thread of its own, because winit's monitor report
  freezes after window creation there; on Android it is the activity's `Display` read over JNI,
  because winit reports none ([android.md](../tools/android.md), "The display's refresh rate");
  elsewhere the current monitor's `refresh_rate_millihertz`. A mode change follows within about two seconds on Xvfb; a real
  dock and undock under gamescope has not been run. Believed, gamescope's included; below
  20 Hz after rounding it is no report. Measured here: Xvfb reports none; a nested headless
  `gamescope -r 60` reports 59855 mHz and names its monitor `gamescope`; a Steam Deck
  docked to a 4K TV in Game Mode reports 59980 mHz, and DRM, gamescope's own mode
  (`3840x2160@60`) and `xrandr` (59.98 Hz) agree. The Deck's own panel and a leftover docked mode are in
  ["On the Steam Deck: the refresh the output reports"](#on-the-steam-deck-the-refresh-the-output-reports).
- *The measurement*, which may raise the report but never lower it (`max`). An interval is
  evidence of the output only under `Fifo` and only if the busy work (`interval - wait +
  gpu`) is at most 40 % of it, so a loop that misses every second vblank cannot read as a
  display at half the rate, and a `Mailbox` or `Immediate` pacing is no refresh. 90 such
  intervals within 8 % spread (`(p90 - p10) / median`) give the rate; wider spread
  (variable refresh) gives none. The last rate is kept when evidence stops (the clamp fills
  the GPU and removes its own evidence) and dropped by a report change, a surface
  reconfigure, a vsync or present-mode change and every race start.
- *Neither*: no clamp. A 144 Hz panel keeps 144; an unknown refresh never caps at the
  simulation's 60. `Session::presentation_hz` (the overlay graph's scale) takes the
  display rate before that fallback.

The CPU governor's budget is the lower of the limiter and the display
(`refresh::held_hz`). Under `Vsync::On` that makes the governor live wherever a report exists.

**On an adapter with no `TIMESTAMP_QUERY` there is no signal and the extent
never moves, and the row cannot say so by greying out.** `disabled_by` names a
*setting*, and `Definition::check_condition` requires a row that edits it - an
adapter capability is neither. An earlier version of this page claimed the
`disabled_by` mechanism covered this; it was written before anyone read
`check_condition`, and this is the correction. The row is stored and inert,
which is the same thing `restart_required` already means.

## A second output: the blur gather (2026-10-25)

The controller also chooses the motion blur's gather resolution: it sheds the full
gather on the overrun the extent would have fallen on, and restores it only at the
extent's ceiling, after three seconds of frames predicted to land under the deadband
([ADR-0075](../architecture/adr/0075-dynamic-resolution-sheds-the-blur-gather-before-pixels.md),
`drs::blur`). The player's `motion_blur_resolution` is its ceiling, carried in
`drs::Limits`, and it writes no setting either. Measurements and the running-race check
are in [the half-gather blur rung](motion-blur-half-rung.md).

## A third output: the reflection step (2026-11-25)

REFLECTION RESOLUTION `auto` (the default) is a rung of the same controller, `drs::reflection`:
the glass and sea target is drawn at `original`, `half` or `quarter` of its fixed size by the
target's own timed pass (the `GLASS` row). It runs **whether or not a resolution target is on**,
because its budget is the target's period when one runs and otherwise the rate the limiter or the
output holds the loop to (the CPU governor's `held_hz`); with neither it is inert. While it still
has a step to give up and wants to, an overrun **holds the extent** instead of falling, so
reflections reach `quarter` before the render scale moves; with the glass a small share it never
holds and the extent loop is the one it was (`a_small_share_leaves_the_extent_loop_exactly_as_it_was`).
It restores only with the extent at its ceiling. The policy, thresholds and the phone runs are in
[water.md](water.md), "REFLECTION RESOLUTION `auto`"; the restore patience and relapse backoff are
the blur rung's own (`drs::patience`, shared).

## A title's own width throttle

2026-11-26. TARGET FPS `original` runs Wipeout HD's own controller instead of the budget
loop above. The law is read off the PS3 executable
([scene-width-throttle.md](../ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md),
confidence 88 to 92): the scene's width is `1 - level / 96` of the full width (1920 to 1300 of
1920), the level moves one notch a frame, narrower when the GPU's frame takes more than
16111 us or the flip was late, wider when it takes 15555 us or less, and the height never moves.
The numbers are title data (`oag_title::width_throttle::Law`, set on HD alone), the controller is
`oag_present::drs::width`, and none of the budget loop's budget, residual, deadband, cooldown or
patience applies.

| | HD's original | Ours |
| --- | --- | --- |
| Height | 1080, fixed | the render-scale ceiling's, fixed |
| Width | `1920 - 20 * level` | the ceiling's `(96 - level) / 96`, rounded; level 0 to 31 |
| Rate it aims at | 60 Hz | **ours:** the rate the limiter or the output holds the loop to; at 60 Hz and below the console's two durations, above it each scaled by `60 / rate` |
| Narrow | GPU span over 16111 us, or the flip not on time | the same durations; **ours:** "not on time" is a frame interval over 1.5 of the paced period |
| Widen | GPU span of 15555 us or less, on time | the same |
| GPU span | first to last command of a frame | **ours:** the sum of every timed pass of one frame: the budget loop's `Cost` plus the labelled passes (shadows, glass, HUD, composite, blit), less the zoom ring, which the bloom chain's reading already holds |
| Reading's age | two frames old | the newest drained, usually one frame old; one step a frame however many drained |
| Off | Photo Mode and menus | Photo Mode (the render-scale size), and every title without a law |

What follows from it:

- **The camera keeps the full picture's shape.** The extent is narrower than the
  ceiling, so the race scene builds its projection for the ceiling's aspect
  (`Scene::set_squeeze`) and the presentation stretches the columns back out, as the console's
  final stage does. Framing and FOV do not move; the picture only loses horizontal resolution.
- **MINIMUM RESOLUTION is greyed.** The floor is the law's own last notch.
- **The blur rung stays at the player's gather** and the budget loop never runs. The reflection
  rung still does; it counts the extent as at its ceiling at level 0.
- **A load, a surface change or a changed row puts the width back to full**
  (`Controller::reset`). Ours; the console's level was read at 31 during a load.
- **It throttles at every output size.** Ours: the console resets the level to 0 on any
  target under 1080 lines (confidence 80), so its 720p path never narrows.
- **A late frame narrows even when the GPU was quick**, which on a loaded host is the whole
  of its movement: a CPU stall counts, as it does on the console.
- **With no `TIMESTAMP_QUERY` it does nothing.** The late-frame input alone would narrow and
  nothing would ever widen.
- **FSR 3.1 reads a narrower camera than was drawn.** It derives the aspect from the render
  size, as upstream does, so under a squeeze its view-space reconstruction is short on x by the
  squeeze. Not measured; the default reconstruction on HD is off.

Omega's executable has none of it and 2048's was not read
([scene-width-throttle.md](../ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md),
"Omega and 2048"), so only HD offers the value; on another title the row drops it and a
hand-written `original` is no controller.

### Measured: the Rocket hit at 3840x2160

2026-11-26, a release build in a borderless 3840x2160 window on a headless weston (RADV, 16:9,
vsync off, the frame limit equal to the refresh), the
[particle-fill reproduction](particle-fill.md#how-to-reproduce-it) (HD, seed 1, autopilot,
Rocket fired once a second from tick 600, hits at ticks 761 and 762). Neither the input script
nor the per-frame log exists in a windowed run, so both came from a throwaway patch, not kept.
Other work held the one-minute load between 21 and 87. GPU is the span in the table above;
interval is the wall-clock frame. "Existing" is TARGET FPS at the same rate with the default
50 % floor. The window is ticks 760 to 789.

| Card, rate | Arm | GPU mean / worst | Interval mean / worst | Size drawn |
| --- | --- | --- | --- | --- |
| RX 7800 XT, 60 | existing a, b | 3.7 / 5.4 to 5.6 ms | 16.6 to 16.7 / 17.3 ms | 3840x2160 throughout |
| RX 7800 XT, 60 | original b | 3.7 / 5.4 ms | 16.7 / 19.1 ms | 3840x2160 throughout |
| RX 7800 XT, 60 | original a (load rose to 87 in it) | 3.8 / 5.2 ms | 26.9 / 49.5 ms | 3840 to 3680 wide, 2160 high |
| iGPU (Raphael), 60 | existing a, b | 36 to 47 / 68 to 88 ms | 37 to 50 / 70 to 95 ms | 1920x1080, the floor |
| iGPU (Raphael), 60 | original a, b | 79 to 81 / 112 to 134 ms | 82 / 114 to 136 ms | 2600x2160, the floor |

**At 60 Hz the discrete card never leaves 3840:** its worst hit frame is a third of 16.1 ms.
The one narrowing there, four notches, came from frames the loaded host delivered late.
**On the iGPU `original` cannot save the frame:** it can give up 32 % of the pixels and no
height, and the frame needs far more; the existing loop gives up 75 %.

Width traces at the turning points, `tick:width`:

| Run | Trace over ticks 740 to 800 |
| --- | --- |
| 7800 XT 60, existing b and original b | 3840 at every tick |
| 7800 XT 60, original a | 3840 to 761, 766:3800, 771:3680, 776:3840, 789:3760, 792:3800, 799:3680 |
| iGPU 60, existing a | 1920 at every tick (1080 high) |
| iGPU 60, original a | 2600 at every tick (2160 high) |
| 7800 XT 240, original b | 2600 to 748, up to 3000 at 766, back to 2600 at 774, then 2600 to 2680 |

At 240 Hz (the durations become 4028 and 3889 us) four runs of each arm, ticks 600 to 799:

| Arm | GPU worst | Size drawn | Late frames |
| --- | --- | --- | --- |
| existing | 3.5 to 4.8 ms | 3840x2160 in one run; down to 2880x1620 in three | 0 to 7 over 25 ms |
| original | 3.4 to 4.5 ms | width down to 3200, 2600, 2600, 3800; always 2160 high | 0 to 3 over 25 ms |

The one `original` run that held its pace (interval mean 4.23 ms against the 4.17 ms period,
GPU worst 3.4 ms) moved one notch. The other three had interval means of 4.4 to 9.4 ms and
two of them GPU frames past 4.0 ms, so these runs do not separate the GPU durations from the
late-frame input at that rate. The Steam Deck is measured in
["On the Steam Deck: TARGET FPS `original` against the budget loop"](#on-the-steam-deck-target-fps-original-against-the-budget-loop).

Frames at level 0 and level 31, the race held on one tick (the grid at tick 200): the same
framing, craft and HUD, softer horizontally, no stale columns at the right edge.

## Three properties the controller does not break

- **It never writes `settings.graphics.render_scale`**, and cannot: `drs.rs`
  does not import `settings` and is handed `drs::Limits` by value. It reads the
  ceiling and emits a separate runtime rectangle. Persisting a controller's output
  into the player's settings file would make their chosen quality drift
  downward with every session on a loaded machine.
- **It never reaches the simulation.** Dynamic resolution is presentation-side,
  like `perf::Meter` and `FrameLimit`; the timestep stays fixed at 60 Hz per
  [ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md) whatever
  the resolution does.
- **Every capture path is outside it.** `race/capture.rs` builds its own
  `Framebuffer` at its own `--render-scale`, never calls `set_extent` and never
  constructs a controller; the front-end capture has no `Framebuffer` at all,
  which since ADR-0038 is correct rather than a gap. This project's comparisons are
  byte-identical screenshot diffs, and a controller driven by how busy the
  machine is makes every one of them irreproducible. `oag_present::perf` already makes
  exactly this argument for why the performance overlay is window-only; it
  applies verbatim. A load is not a frame either - `Session::stalled` exists for
  that and clears the meter, and a controller needs the same guard or a track
  load drives the scale to the floor and takes seconds to climb back.

## The fixed cost over the period (2026-10-19)

Report: Pulse (PS2) "always renders full resolution, no matter the value" on the Steam
Deck, Pulse (PSP) scaling as expected. Not the platform and not `bb542a73a` (only menu
values and the default floor): the maintainer's PS2 row is the one with
`reconstruction = "fsr3"`, `target_fps = "90"`, and the Deck's gamescope was on a
3840x2160 output. FSR 3.1's presented half measured 11.99 ms there against an 11.11 ms
period, so the budget was negative and the controller held at the ceiling.

Deck, release build, 50 s races on the maintainer's settings file copied read-only into
a private XDG dir (`render=` is the scene target):

| profile | scene target | fps |
| --- | --- | --- |
| PS2, fsr3, target 90, min 50 (as reported), before | 3840x2160 all run | 15 |
| PS2, smaa, target 90, min 50 | 2112x1188 (55 %) | 49 |
| PSP, fsr3, target 60, min 25 | 960x540 (25 %) | 41 |

Static `--render-scale 50` already gave 720x408 on PS2 and PSP alike (Xvfb, lavapipe), so
the fixed-scale path was never broken; only the controller held. ADR-0073 makes it fall
to the floor. Pins: `oag_present::drs::tests::an_unreachable_target_falls_to_the_floor_and_says_so`,
`a_fixed_cost_over_the_period_still_honours_the_minimum_resolution` (floors 25, 33 and 50),
and `oag_game::settings::tests::a_render_scale_below_100_shrinks_every_title_and_platforms_scene_target`
(every `(title, platform)` row, and the reported file's shape).

## On the Steam Deck: HD Zone against a time trial

2026-11-26, at `790ad033b` (before TARGET FPS `original` existed). The report: on a Deck, HD's
Zone race on Chenghou Project settles at about 75 % under dynamic resolution where a time trial
on the same circuit holds 100 %.

**Verdict: true in direction at 3840x2160, not seen at 1280x800 or 1920x1080.** At 4K, with the
reporter's own settings, Zone settles at about 0.80 (stepping to 0.75) and a time trial at about
0.93, not 1.00. The pass that grows is the race pass, which is mostly faithful. The settle is so
much deeper than that cost because the Deck's GPU clock falls with the scale: with the clock pinned
at its maximum, Zone settles at 0.96 and the time trial at 0.99
([below](#why-the-4k-settle-is-so-deep-the-gpu-clock-falls-with-the-scale)).

Checked again at `d66a52ae6`, two 4K paced runs each: Zone 0.83 and 0.84, time trial 0.93 and
0.95 (second half of each run).

### How it was run

A Steam Deck OLED on AC power, RADV VANGOGH, the portable container build, autopilot, sound on,
70 s windowed runs inside a nested headless gamescope (the panel itself is 1280x800, so the
larger sizes cannot be shown on it without a dock; the real Game Mode output was not measured):

```sh
gamescope --backend headless -W <w> -H <h> -w <w> -h <h> -r <hz> -- \
  oag-game <hd iso> --race --no-intro --autopilot --frame-log \
  --track /data/environments/04_chenghou_project/track.vex --mode <zone|time_trial> --script 70:quit
```

The settings file is a copy of the reporter's, with `pause_on_focus_loss = false`. Its HD profile
is `scene_precision = "float"`, `circuit_lighting = "simplified"`, `shadow_resolution = "ultra"`,
`reconstruction = "fxaa"`, `motion_blur = "off"`, `minimum_resolution = 25`,
`target_fps = "default"`, under `vsync = "smooth"` and `frame_limit = "refresh_rate"`. The arms:

- **paced**: as above.
- **DRS off, paced**: `target_fps = "off"`.
- **unpaced**: `vsync = "off"`, `frame_limit = "unlimited"`, with `target_fps` as above or `"off"`.

The refresh was chosen: `-r 90` at 1280x800 (the panel), `-r 60` at the two TV sizes. Scale is
`render=` over `surface=` in the frame log, per axis; the first 8 lines of each run are left out.

### Settled scale

| Size, target | Time trial, paced | Zone, paced | Time trial, unpaced | Zone, unpaced |
| --- | ---: | ---: | ---: | ---: |
| 1280x800, 90 | 1.00 | 1.00 | 1.00 | 1.00 |
| 1920x1080, 60 | 1.00 | 1.00 | 1.00 | 1.00 |
| 3840x2160, 60, second half of each run | 0.88 to 0.95 (4 runs, median 0.93) | 0.79 to 0.89 (6 runs, median 0.80) | 0.98 | 0.94 |

The six Zone runs include one 160 s long (0.79, so it does not drift down further with the
stages) and one with `--no-audio` (0.80): **sound changes nothing**. Frame rate is held at 60 in
every paced DRS run.

### Per-pass GPU time

DRS off, unpaced, one run each, so the GPU sat at its top clock (about 1600 MHz at the two
smaller sizes, 1530 MHz at 4K). Milliseconds, time trial then Zone:

| Pass | 1280x800 | 1920x1080 | 3840x2160 |
| --- | ---: | ---: | ---: |
| race | 1.65 / 1.87 | 2.46 / 2.86 | 7.15 / 8.75 |
| HD bloom | 0.68 / 0.67 | 1.22 / 1.14 | 4.30 / 3.92 |
| glass | 0.18 / 0.22 | 0.18 / 0.21 | 0.18 / 0.22 |
| advert cards | 0.18 / 0.01 | 0.18 / 0.01 | 0.18 / 0.01 |
| shadow map and self shadow | 0.17 / 0.20 | 0.17 / 0.20 | 0.17 / 0.20 |
| FXAA | 0.12 / 0.12 | 0.23 / 0.22 | 0.91 / 0.89 |
| blit | 0.15 / 0.13 | 0.28 / 0.24 | 1.01 / 0.86 |
| HUD and UI | 0.09 / 0.06 | 0.17 / 0.10 | 0.66 / 0.37 |
| occlusion, zoom, probe blend | 0.05 / 0.05 | 0.08 / 0.07 | 0.26 / 0.20 |
| **all timed** | **3.27 / 3.32** | **4.97 / 5.05** | **14.83 / 15.41** |

A second 4K pair agreed within 0.02 ms on the race pass.

**The race pass is the one that grows**: +1.60 ms at 4K, +0.40 at 1080p, +0.22 at 800p. It is the
Zone surface, which is mostly faithful ([hd-zone-recolour.md](hd-zone-recolour.md), "What a Zone
race costs"). What the controller manages (race pass plus bloom) is 12.67 ms against 11.45, 11 %
more. The budget loop's other costs are smaller in Zone.

### Why the 4K settle is so deep: the GPU clock falls with the scale

These runs were made with the Deck undocked; docked, the clock does not park
([below](#on-the-steam-deck-what-holds-the-clock-and-the-time-trials-late-seconds)).

A cost 11 % higher moves a controller's linear scale by about 5 %. The rest is the Deck's clock:
with DRS on, the controller lowers the scale, the load falls, the Deck's governor lowers the
clock, the GPU's timestamps (milliseconds, not cycles) grow, and the controller reads them as no
room to rise. Confidence **90**.

The clock before the pin, under the same pacing:

| 3840x2160, paced, `790ad033b` | fps | mean frame | p99 | median GPU clock |
| --- | ---: | ---: | ---: | ---: |
| Time trial, DRS off | 59.8 | 16.73 ms | 17.23 ms | 1536 MHz |
| Zone, DRS off | 58.8 | 17.03 ms | 18.58 ms | 1574 MHz |
| Time trial, DRS on | 60.0 | 16.67 ms | 16.85 ms | 1305 MHz |
| Zone, DRS on | 59.9 | 16.71 ms | 16.87 ms | 1104 MHz |

#### With the clock pinned

2026-11-26, at `7654d470a`, same Deck, same command and settings file as above at 3840x2160,
`-r 60`, 70 s, sound on, AC power. The GPU clock was pinned at its maximum in Steam's performance
settings (`power_dpm_force_performance_level` read `manual`, `pp_od_clk_voltage` SCLK 1600 to
1600). `hwmon`'s `freq1_input` read 1600 MHz in every sample of every run (two samples a second).
The memory clock is not part of the pin: it read 800 MHz in most samples, dipping to 600.

| 3840x2160, paced, clock pinned | settled scale, per run | median | frame ms, median of per-second means | per-second p99 ms, mean (worst second) | GPU power, median |
| --- | --- | ---: | ---: | --- | --- |
| Time trial, DRS on, 3 runs | 0.990, 0.985, 0.987 | **0.99** | 16.67 | 16.87 to 17.02 (19.60) | 15.6 to 16.0 W |
| Zone, DRS on, 3 runs | 0.974, 0.961, 0.958 | **0.96** | 16.67 | 16.90 to 16.94 (18.78) | 15.6 to 16.1 W |
| Time trial, DRS off, 1 run | 1.00 | | 16.67 | 17.53 (21.89) | 15.5 W |
| Zone, DRS off, 1 run | 1.00 | | 16.67 | 18.17 (22.38) | 15.5 W |

The frame columns are statistics of the frame log's one-second lines, not per-frame percentiles.
The runs alternated between the two modes. Zone still steps to 0.85 or 0.90 over one repeating
stretch of the autopilot lap and comes back to 1.00, so it follows the load instead of staying
down.

The build is newer than the unpinned runs', so the DRS-off runs are the bridge: race pass and HD
bloom cost 8.67 and 3.91 ms in Zone, 7.04 and 4.33 ms in the time trial, against 8.76 and 3.94,
7.13 and 4.35 at `790ad033b`. The cost at full size did not move. An unpinned run of the same
binary was not made (the pin was set by hand in Steam and left in place).

**The test that does not depend on any clock reading.** A pass whose cost is per pixel, on a GPU
at a steady clock, takes its full-size time multiplied by the mean of the scale squared:

| Measured race pass ms over that prediction | Time trial | Zone |
| --- | --- | --- |
| Pinned, DRS on (3 runs each; HD bloom the same) | 0.99 | 0.99 to 1.00 |
| Not pinned, DRS on (4 runs each, `790ad033b` and `d66a52ae6`) | 1.11 to 1.14 | 1.29 to 1.38 |

Unpinned, the GPU took a third longer per pixel in Zone than at full size. Pinned, it did not.
The unpinned clocks (about 1100 to 1200 MHz in Zone, 1300 in the time trial, against 1540 to
1575 MHz at full size) give ratios of that size.

**The price of the pin is power.** Unpinned with DRS on, the same `hwmon` reading was 9.6 to
10.5 W in Zone and 11.5 to 12.6 W in the time trial, so the deeper settle saved 3 to 6 W. On
battery that may be the better trade for a player.

#### What a rise rule that survives this needs

Written before the probe rise was built; what was built is in
[The probe rise](#the-probe-rise) below:

- **The rise is the stuck half, not the budget.** In two traced unpinned Zone runs the residual
  stayed between 2.3 and its opening 3.33 ms of the 16.67 ms period, so it had not tightened the
  budget. At the opening residual the budget is 13.3 ms and the deadband 10.7 to 12.7 ms; Zone's
  median reading in a traced run was 11.1 ms, so it held. A rise needs `RISE_PATIENCE` (8)
  readings under the deadband.
- **GPU milliseconds cannot say whether there is room.** `drs.rs` reads no clock by design
  ("The controller", above), and a clock source is Linux and driver specific. The portable shape is
  a probe: after a long hold with no late frame, step up one grid point and keep it only if no
  frame is late. `drs::blur` already restores with a backoff, and `drs::width` already counts a
  late frame (1.5 periods).
- **A probe has to outlast the governor's ramp.** The first readings after a step up are taken at
  the old, low clock, and a fall answers a single reading over budget, so it would undo the probe
  at once. How long the Deck's governor takes to raise the clock is unmeasured and is the first
  measurement to take.
- **Both existing guards stay.** `RISE_PATIENCE` and the overshoot guard stopped a size change
  loop ("The policy", above); a probe needs its own backoff on top, and a change in the policy
  needs an ADR (the budget is ADR-0042's).
- **Acceptance:** unpinned Zone at 3840x2160 settling near 0.96, no more late frames than now, no
  more size changes per minute, and no change on a GPU that holds its clock.
- **TARGET FPS `original`** also steps on a GPU duration
  ("A title's own width throttle"). Whether a falling clock holds it down too is unmeasured.

### The probe rise

[ADR-0096](../architecture/adr/0096-dynamic-resolution-probes-a-rise-a-falling-clock-hides.md),
`oag_present::drs::probe`. After `HOLD_SECONDS` (4 s) below the ceiling with every reading quiet
(at or under the deadband's top, a wall-clock frame no longer than `LATE_PERIODS` (1.25) periods,
no step), the controller steps up one grid point on trial. For `RAMP_SECONDS` only a late frame
(1.5 s) moves the scale; then it samples the scalable cost for `SAMPLE_SECONDS` (0.5 s) and keeps the step only if
the mean is at or under `KEEP_SHARE` (0.92) of the pre-step cost times `(next / here)^2` and at or
under the deadband's top. A reverted probe doubles the next hold, up to 128 s; a kept one clears
the backoff. `RISE_PATIENCE` and the overshoot guard are unchanged on the normal rise.

Why the keep test is a ratio and not "no late frame": under smooth vsync and a limiter a GPU-bound
frame shows as a slightly long interval, not a doubled one, so a late-frame test alone keeps
almost every probe. A GPU at a steady clock pays the full `(next / here)^2` for a step (the Deck's
race pass pinned: 0.99 to 1.00 of its full-size cost times the scale squared), a governor that
raises its clock pays about `1 / (next / here)^2` of it (0.886 to 0.905).

How these were run: the command in "How it was run" above, with
`RUST_LOG=info,oag_game::app::session::timing=trace` (every scale change and every GPU reading,
plus a `probe began` / `probe ended, <verdict>` line), every stderr line stamped with bash's
`$EPOCHREALTIME`, and beside it a loop reading `hwmon/freq1_input`, `gpu_busy_percent` and
`power1_average` and stamping them the same way, with `sleep 0.02` between reads.

**The governor's ramp.** 2026-11-26, the Deck unpinned (`power_dpm_force_performance_level`
`auto`), 3840x2160 Zone as above, `hwmon` `freq1_input` sampled about every 22 ms beside a log
whose every line carries the same clock. For each step up of the scale (normal rise or probe), the
clock before (mean of the second before) and after (mean of 1 to 2 s after), and the time to 90 %
of the rise, over six runs (three before, three with the first probe build):

- **71 steps up; in 38 the clock did not rise at all** (under 20 MHz), most of them sitting at
  1040 MHz with the GPU 86 to 100 % busy. 1040 MHz is where the Deck's governor parks this load:
  the median clock of a whole unpinned 4K Zone run with no probe was 1040 to 1073 MHz.
- **Where it rose, 90 % of the rise took 0.01 to 1.6 s** (33 steps, median 0.67 s, 90th
  percentile 1.45 s; 20 over 0.5 s).

So `RAMP_SECONDS` is 1.5 s. The first build judged at 0.5 s and reverted trials half way up the
ramp, at 0.94 to 0.97 of the steady-clock prediction.

**What a kept and a reverted probe read.** Over the unpinned 4K Zone runs, kept probes read 0.82
to 0.92 of the steady-clock prediction (16 probes; the model said 0.886 to 0.905 for a governor
that carries the whole step); reverted ones 0.93 to 0.97 (the clock rose part way, or not) and
1.10 to 1.13 (the scene got heavier during the sample).

**A GPU at a steady clock does not move.** The Deck with its clock pinned at 1600 MHz is one, so
the same 4K Zone race as above ran with and without the probe (`432db773b` against the probe's
builds, 70 s, sound on, Deck load 1.0 to 2.3):

| Pinned, Zone 3840x2160 | settled scale (second half), per run | size changes a minute | probes |
| --- | --- | --- | --- |
| Before | 0.966, 0.963 | 32, 34 | |
| After (`a051cfef3`) | 0.963, 0.965, 0.960 | 32, 28, 32 | one a run (0.90 to 0.95), all three reverted, 0.09, 0.6 and 1.0 s after the step |
| After (`8dc1a3d29`) | 0.968, 0.963 | 32, 32 | one a run, both reverted on a late frame 0.12 and 0.15 s after the step |

The first three reverts include the long frame the step itself costs: 29 to 36 ms between frames
straight after it, at a pinned clock, whatever the scale does next. A late frame inside the step's
own `COOLDOWN` is therefore not judged (`8dc1a3d29`). The two reverts after that were real: at 0.95
pinned, the GPU's frame is about the period, and the normal loop itself flips between 0.90 and
1.00 there.

**The workstation never reaches the probe.** HD Zone at 3840x2160 under the Deck's settings on
the RX 7800 XT, in a headless gamescope at 60 Hz: the race pass is 1.3 ms, so the scale holds 1.00
with and without the probe (one run each, workstation load 47 to 64, which made the loop CPU
bound); at the ceiling there is nothing to probe.

**On the Deck, unpinned.** The same commands at `432db773b` (before) and `8a7c3a5cd` (after,
the 1.5 s ramp), 70 s each, runs interleaved, Deck load 0.4 to 2.6. Settled scale is the mean over
the second half of the frame log's one-second lines; size changes and lost frames (the sum of
`60 - fps` over seconds under 60) per minute over the race after its first 8 lines; GPU clock and
power the medians over the race.

| 3840x2160, 60 | settled scale, per run | size changes a minute | lost frames a minute | seconds with p99 over 20.8 ms | GPU clock | GPU power |
| --- | --- | --- | --- | --- | --- | --- |
| Zone, before | 0.837, 0.831, 0.863 | 15, 19, 18 | 0, 8, 0 | 0, 0, 0 | 1040 to 1073 MHz | 8.1 to 8.3 W |
| Zone, after | 0.895, 0.906, 0.903 | 17, 11, 8 | 1, 0, 0 | 0, 0, 0 | 1318 to 1409 MHz | 10.7 to 11.9 W |
| Time trial, before | 0.982, 0.982, 0.981 | 13, 14, 11 | 58, 148, 72 | 9, 14, 13 | 1569 to 1574 MHz | 15.0 W |
| Time trial, after | 0.984, 0.982, 0.976 | 16, 13, 20 | 129, 191, 88 | 15, 22, 21 | 1569 to 1577 MHz | 14.9 to 15.0 W |

- **Zone rises from 0.84 to 0.90**, with no more size changes and no late seconds, at 3 to 4 W
  more. Not the 0.96 the clock pinned gives: 6 of 13 probes in those three runs reverted, and the
  governor often holds 1040 MHz through a step.
- **The time trial does not move.** Its clock is already at the top (1570 MHz), so 2 probes fired
  in three runs, both reverted on a late frame. Its lost frames are high on both binaries, and
  higher than the same time trial on an earlier day (0.93 then, 0.98 now): the time trial at 4K is
  GPU bound at the top clock. Lost frames overlap between the arms (58 to 148 before, 88 to 191
  after), but seconds with a p99 over 20.8 ms do not (9, 14, 13 before; 15, 22, 21 after). The
  probe is an unlikely cause: the worst run (22 such seconds) fired no probe, and with no trial
  running the controller does what it did before. Run order or heat is not ruled out (the arms
  alternated, before first); unresolved.
- **1920x1080 and 1280x800** (Zone, one run each, 60 and 90 Hz): 1.00 before and after, no probe,
  no size change, no lost frame.
- The first probe build (0.5 s ramp, `8dc1a3d29`), three Zone runs: 0.826, 0.916, 0.865.

## On the Steam Deck: what holds the clock, and the time trial's late seconds

2026-11-26. Two questions left by a trial build of a "probe rise" (a rise that GPU milliseconds
cannot veto, kept only if no frame is late; the shape under
["What a rise rule that survives this needs"](#what-a-rise-rule-that-survives-this-needs), on the
unmerged branch `drs-probe-parked`): what parks the unpinned Deck at 1040 MHz in a 4K Zone race,
and whether the probe causes the extra late seconds it showed in a 4K time trial.

### The Deck was docked, and that changes the answer

The journal shows the Deck undocked from 07:30:47, then docked to a 4K TV at 18:29:17 (gamescope's
session selected 3840x2160@60 on `DP-3`). It was still docked during the runs below.

- **Undocked** (07:30 to 18:29): the runs of
  ["On the Steam Deck: HD Zone against a time trial"](#on-the-steam-deck-hd-zone-against-a-time-trial),
  its pinned-clock runs included, and the trial build's own Zone and pinned-clock runs.
- **Docked**: the trial build's six time trials (the first began 6 s before the dock event, still
  loading) and everything measured below.

Docked, the 1040 MHz park does not happen:

| 3840x2160, 60, `auto` clock, docked | runs | settled scale | GPU clock, median | socket power, median | frame-log lines with p99 over 20.8 ms |
| --- | ---: | --- | ---: | ---: | --- |
| Zone, `432db773b` (the build that parked undocked in 3 of 3 runs) | 2 | 0.93, 0.93 | 1600 MHz | 14.9 W | 12, 5 |
| Zone, main `da0a4e53e` (full sampler) | 2 | 0.93, 0.93 | 1600 MHz | 14.9, 15.0 W | 20, 15 |
| Zone, main with the probe merged (`e8e7665ff`) | 2 | 0.93, 0.93 | 1600 MHz | 14.9 W | 10, 16 |

Two more Zone runs of main with the older light sampler also read 1600 MHz and 15.0 W (12 and 13
lines), so the heavier sampler is not what raises the clock.

- **The code is not what changed** (confidence **85**): the same binary parked before and runs at
  1600 MHz now.
- **The dock is the likely cause** (confidence **60**): it is the one change found in the journal,
  but the Deck was not undocked again to reverse it. Why a docked Deck raises the clock is not
  measured.
- **Per-pixel GPU cost did not change.** Pass time times the clock, over the share of full size
  drawn, for `432db773b`: race pass 13.2 to 13.4 kilocycles parked and docked alike, HD bloom 6.0
  against 6.2. Present-size passes (FXAA, blit, HUD) cost 2.6 against 3.2.

### Method

The command, settings file and build recipe of ["How it was run"](#how-it-was-run), `--mode zone`
or `time_trial`, 70 s, `vsync = "smooth"`, `frame_limit = "refresh_rate"`, sound on,
`RUST_LOG=info,oag_game::app::session::timing=trace`. The first 8 frame-log lines are left out. A
loop beside the game read, at 20 Hz: `gpu_metrics` (v2.4, 168 bytes; current GPU clock at offset
`0x4c`, socket, CPU, SoC and GPU power at `0x28` to `0x2e`, ASIC throttle bits at `0x6c`),
`hwmon`'s `freq1_input` and `power1_average`, `gpu_busy_percent`, `pp_dpm_sclk`,
`pp_power_profile_mode`, `power1_cap`, `power2_cap` and each core's `scaling_cur_freq`. Nothing was
written to sysfs. Throttle bit names are `smu11_driver_if_vangogh.h`'s.

`power1_average` equalled the socket power field within 1 mW in 16,767 of 16,771 samples, so the
"GPU power" and "W" readings on this page are **socket power** (the whole APU), not the GPU alone.

The frame log's `p99_ms` covers the last 120 frames, about two seconds, so one late stretch
usually shows on two lines. Counts of "late seconds" are counts of lines.

### Docked: at or near the 1600 MHz ceiling, with the 15 W package limit trimming the dips

Confidence **90**, from the Zone runs of main and of the probe build with the full sampler, and the
four time trials below:

- Throttle bit `FPPT` was set in 91 to 100 % of samples and `SPPT` in 72 to 94 %. No thermal or
  current bit was ever set. `power1_cap` and `power2_cap` both read 15 W.
- Socket power stayed at about 14.9 W. The GPU clock's median was 1600 MHz in Zone and about 1576
  in the time trial, with dips to about 1480. Nothing holds it below its ceiling but those dips.
- The fastest CPU core read 1400 MHz in the median sample, and the frame log's CPU time was 7 to
  14 ms in the median line.
- `gpu_busy_percent` read 99 or 100 % in the median sample, while the timed passes summed to about 13.4 ms
  of 16.7 ms in Zone. What fills the rest is not measured, and the budget loop cannot see it.
- `pp_power_profile_mode` marks **no** row as active in any sample. In the upstream driver
  (`vangogh_ppt.c`, v6.16) that is what the boot default, index 0, looks like, which Van Gogh does
  not list. SteamOS's kernel source was not read.

### The undocked park at 1040 MHz: what it is not

From the trial build's raw Zone samples (clock, busy and power only; no throttle bits):

- **Not a power limit** (confidence **85**): parked samples drew a median 7.6 to 8.5 W, and the same
  Deck minutes apart ran at up to about 1560 MHz and 13.6 W in a run whose probe held a larger size.
- **Not CPU bound** (confidence **80**): CPU time was about 5 ms of 16.7, while the timed GPU passes
  filled 14.8 ms of the frame at 1040 MHz.
- **Thermal limits and the power profile: not sampled** undocked.
- What is left is the firmware's own demand governor not raising a GPU whose frames fit.
  **Hypothesis, confidence 45**: reached by elimination. In support, an idle docked Deck reads a
  current clock of only 200 or 1040 MHz, so 1040 looks like the level an awake GPU takes before
  anything asks it to rise.

### The time trial's late seconds are not the probe

Order after, before, before, after (the earlier set ran before first). "Before" is main
`da0a4e53e`, "after" is that with the probe merged.

| 3840x2160 time trial, docked | build | settled scale | lines with p99 over 20.8 ms | of those near a size change / near nothing | probes inside the window |
| --- | --- | ---: | ---: | --- | ---: |
| 1 | after | 0.977 | 8 | 4 / 4 | 0 |
| 2 | before | 0.982 | 23 | 11 / 12 | 0 |
| 3 | before | 0.979 | 13 | 7 / 6 | 0 |
| 4 | after | 0.982 | 15 | 10 / 5 | 0 |

"Near" is within about a second of the line's window. All four sat at 15 W with `FPPT` set.

**The dropped first 8 lines are where the probe does fire**: in 3 of the 5 "after" runs (run 4 and
two earlier ones), begun about 5 s in and reverted within 0.3 s. The lines it fired on and after
read over 20.8 ms twice in one run (21.36), once in another (21.08) and not at all in the third,
and four runs with no probe had a pair of 22.1 to 25.0 ms lines at the same point. Over the 8
dropped lines the late counts are 2 to 5 for "after" and 2 to 4 for "before".

**Verdict: the probe does not cause the time trial's late seconds** (confidence **85**). The ranges
overlap (after 8 and 15, before 23 and 13), and across all five "after" runs, the three earlier ones
included (15, 22 and 21 lines against 9, 14 and 13), no probe began after the 8 dropped lines. A
loop at the power limit seldom holds the 4 s of quiet a probe waits for.

Not covered: the same time trial undocked.

### Open: the undocked clock park

**What parks an undocked Deck at 1040 MHz is not answered.** The question above is answered for a
docked Deck only; the maintainer kept it docked. Recipe for a later undocked pass, about 8 minutes:

1. Deck undocked and on AC: `/sys/class/drm/card0-DP-3/status` reads `disconnected` and
   `power_dpm_force_performance_level` reads `auto`. Check both before each run and stop if not.
2. Four 70 s runs of the command in ["How it was run"](#how-it-was-run) at 3840x2160, `-r 60`,
   `--mode zone`, on a copy of the player's settings, in the order probe, no probe, no probe, probe.
3. Beside each run, sample at 20 Hz everything listed under ["Method"](#method).
4. In the samples whose current GPU clock is 600 to 1100 MHz, read: the `FPPT`/`SPPT`/thermal
   throttle bits (any set would name a limit and end the demand-governor hypothesis), socket power
   against `power1_cap`, the starred row of `pp_power_profile_mode` if any, the temperature, and the
   share of the frame period the timed passes fill.

## On the Steam Deck: TARGET FPS `original` against the budget loop

2026-11-26, at `d18b06815`. The question: on a Deck, does TARGET FPS `original` (HD's width
throttle) hold the Rocket barrage closer to its frame target than TARGET FPS `default`?

**No, at all three sizes.** At 1280x800 and 1920x1080 it is barely different from no dynamic
resolution at all (means within 0.15 ms, p99 within 1.5 ms). At 3840x2160 it helps, but far less
than `default` does.

### How it was run

A Steam Deck OLED on AC power at default GPU clocks (1460 to 1515 MHz mean while busy), RADV,
the portable container build, sound on, one-minute load 0.8 to 3.0. Windowed in a nested
headless gamescope: `-r 90` at 1280x800 (the panel), `-r 60` at the two TV sizes, which the
panel cannot show. The controller only runs windowed, and a windowed run has neither an input
script nor a per-frame record, so both came from a throwaway patch, not kept: it fed the script to
the race's ticks, wrote one row per presented race frame (tick, wall interval, size drawn, width
level), saved the surface image at asked ticks and quit at tick 900.

```sh
gamescope --backend headless -W <w> -H <h> -w <w> -h <h> -r <hz> -- \
  oag-game <hd iso> --race --no-intro --autopilot --frame-log --mode single_race --seed 1 \
  --give rocket --log-every 0
```

The race is the [particle-fill reproduction](particle-fill.md#how-to-reproduce-it) on Talon's
Junction. Settings: `vsync = "off"`, `frame_limit = "unlimited"`, `aspect = "free"`, and the HD
render profile of "On the Steam Deck: HD Zone against a time trial" (so `minimum_resolution =
25`). Three arms, `target_fps` = `"off"`, `"default"`, `"original"`, four rounds each with the
order rotated. The state hash at tick 800 is `62b06c2ee353b11d` in all 36 runs.

The target period is 11.11 ms at 90 Hz and 16.67 ms at 60 Hz. At 90 Hz the throttle's durations
scale to about 10.7 and 10.4 ms.

### Frame times

Milliseconds per presented frame, medians over the four rounds. "Worst" is the range of each
round's slowest frame. "Over" is the share of frames longer than 105 % of the period. "Pixels"
is the mean share of the full size drawn. A slow arm presents fewer frames over the same ticks
(at 3840x2160 `off` has 71 in the barrage and 15 in the hit, so its p99 is nearly its worst).

The barrage, ticks 700 to 829:

| Size, `target_fps` | mean | p99 | worst | over | pixels |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1280x800 `off` | 8.99 | 18.2 | 21.9 to 23.7 | 18 % | 100 % |
| 1280x800 `default` | 8.41 | 16.6 | 19.3 to 19.6 | 13 % | 73 % |
| 1280x800 `original` | 8.97 | 18.1 | 20.9 to 23.7 | 18 % | 98 % |
| 1920x1080 `off` | 11.96 | 30.0 | 32.5 to 38.5 | 16 % | 100 % |
| 1920x1080 `default` | 10.48 | 26.4 | 28.3 to 37.2 | 12 % | 77 % |
| 1920x1080 `original` | 11.81 | 28.5 | 32.7 to 37.4 | 16 % | 99 % |
| 3840x2160 `off` | 30.37 | 63.3 | 62.6 to 73.4 | 95 % | 100 % |
| 3840x2160 `default` | 11.64 | 29.2 | 30.3 to 43.6 | 14 % | 20 % |
| 3840x2160 `original` | 24.22 | 55.9 | 56.5 to 73.5 | 66 % | 71 % |

The craft hit, ticks 760 to 789:

| Size, `target_fps` | mean | p99 | worst | over |
| --- | ---: | ---: | ---: | ---: |
| 1280x800 `off` | 8.65 | 16.1 | 16.3 to 18.0 | 10 % |
| 1280x800 `default` | 8.09 | 14.2 | 14.4 to 16.6 | 8 % |
| 1280x800 `original` | 8.79 | 16.1 | 15.7 to 18.2 | 13 % |
| 1920x1080 `off` | 12.20 | 26.1 | 27.6 to 29.9 | 10 % |
| 1920x1080 `default` | 9.80 | 20.8 | 20.5 to 26.0 | 6 % |
| 1920x1080 `original` | 12.12 | 24.7 | 25.5 to 27.6 | 11 % |
| 3840x2160 `off` | 33.71 | 61.4 | 57.8 to 66.1 | 100 % |
| 3840x2160 `default` | 10.68 | 23.5 | 20.3 to 34.8 | 9 % |
| 3840x2160 `original` | 25.25 | 45.9 | 42.1 to 51.6 | 90 % |

### The size over time

Round 1, the size drawn at the first frame at or past each tick. `original` keeps the full height,
so only its width is listed.

| Tick | 400 | 700 | 728 | 762 | 776 | 800 | 830 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 800p `default` | 1280x800 | 1152x720 | 896x560 | 832x520 | 1024x640 | 1152x720 | 1088x680 |
| 800p `original`, width | 1280 | 1200 | 1200 | 1200 | 1280 | 1280 | 1173 |
| 1080p `default` | 1920x1080 | 1536x864 | 1632x918 | 1344x756 | 1536x864 | 1728x972 | 1440x810 |
| 1080p `original`, width | 1920 | 1840 | 1840 | 1800 | 1920 | 1920 | 1760 |
| 4K `default` | 3264x1836 | 1728x972 | 1536x864 | 1152x648 | 1536x864 | 1728x972 | 1728x972 |
| 4K `original`, width | 3200 | 2840 | 2720 | 2600 | 2600 | 2600 | 2680 |

Why it cannot keep up:

- **It moves one notch a frame and widens as soon as a frame is cheap.** The barrage's long frames
  come in short runs. At 1280x800 it reaches level 14 at most, at 1920x1080 level 10, and it is
  back at level 0 between them. It changes width 79 to 80 times (800p) and 55 to 59 times (1080p)
  in ticks 700 to 829, where `default` changes size 18 to 22 times.
- **It can give up only 32 % of the pixels.** At 3840x2160 the race does not fit before any
  Rocket: it is at level 31 by tick 599, so the barrage finds nothing left to take. `default`
  goes down to 1152x648.

### The picture

One surface image a run, in runs of their own (the readback stalls the frame), at tick 728
(3840x2160: 730) and 775. A capture lands on the first frame at or past the tick, so some arms
show a tick one or two later. Compared at 1:1:

- **`original` at 1280x800 (1200x800) and 1920x1080 (1840x1080)** looks the same as full size.
- **`original` at 3840x2160 (2720x2160)** is slightly softer horizontally only; framing and FOV
  do not move and no stale column shows.
- **`default`** is softer on both axes at every size (896x560, 1728x972, 1536x864): slightly at
  800p and 1080p, plainly at 3840x2160, where diagonal edges stair-step.
- The HUD is sharp in every arm. The blast cloud differs between images, but the captures differ
  in tick and in wall-clock phase, so that is not put down to an arm.

Not measured: `vsync` on or a frame limit, the real Game Mode output at the TV sizes, and a
console.

## On the Steam Deck: the refresh the output reports

2026-11-26, the same Deck and build, after the X11 reader moved onto `oag_thread::Poller`. The
reading is the `X server reports N mHz` debug line.

| Where | Reading | Rate in force |
| --- | --- | --- |
| Live Game Mode, `DISPLAY=:0` (the Steam client's display) | 89887 mHz | 90 Hz, the OLED panel |
| Live Game Mode, `DISPLAY=:1` (gamescope's second Xwayland) | 59981 mHz at 3840x2160 | 60 Hz |
| Nested headless gamescope `-r 90`, 12 runs | 89887 mHz | 90 Hz |
| Nested headless gamescope `-r 60`, 24 runs | 59963 mHz (1920x1080), 59981 mHz (3840x2160) | 60 Hz |

- Each run logged the line once. On the two live displays the `x11-refresh` thread used no
  measurable CPU time (0 clock ticks over 18 s).
- **`:1` reported the TV the Deck had been docked to**, although the panel was running at 90 Hz.
  gamescope's journal shows server #1 last set to 3840x2160@60 while docked, and the undock that
  morning updated server #0 only. The same journal shows server #1 set to 1280x800@90 on an
  earlier undocked day. So this reads as a mode gamescope had not refreshed since the undock, and
  the reader reports what the server says. That Steam games are started on `:1`, and that a launch
  refreshes the mode, are inferred. **What a game started from Steam reads on an undocked Deck is
  not measured.**
- A 60 Hz LCD Deck was not available. Desktop mode needs a session switch and was not tried.

The failure paths, on a workstation Xvfb (lavapipe):

- **No RandR extension**: the game panics inside winit 0.30.13 before the reader starts. No such
  session reaches the reader. From the code, a refused request would make a round answer nothing,
  with no log line, and the thread sleeps its period whatever the answer; that was not run. Not
  supported, and not planned:
  - *Reproduce*: `Xvfb :N -screen 0 1280x720x24 -extension RANDR`, then `oag-game --no-audio`
    with `WINIT_UNIX_BACKEND=x11 DISPLAY=:N`. Exit status 101, `failed to request XRandR
    version: UnsupportedExtension` at `platform_impl/linux/x11/xdisplay.rs:112` (run 2026-11-26).
  - *Why nothing of ours avoids it* (winit source, read): the panic is in `XConnection::new`,
    run by the lazy `X11_BACKEND` that `EventLoop::new()` reads, before any window attribute or
    monitor query exists. `EventLoop::new` has a second `expect` on the same extension
    (`select_xrandr_input`, `x11/mod.rs`).
  - *Upstream* (read 2026-11-26): 0.30.13 is the last 0.30 release and the head of `v0.30.x`.
    `master` and `v0.31.0-beta.4` still require RandR 1.2 and return `ExtensionNotSupported("the
    X11 backend requires XRandR 1.2 or newer")` instead of panicking. In winit
    [#1781](https://github.com/rust-windowing/winit/issues/1781), the same defect with an older
    message, a maintainer answered "winit doesn't support systems without xrandr"
    ([#576](https://github.com/rust-windowing/winit/issues/576) is the other report).
  - *On a 0.31 upgrade* (read, not run): `new_event_loop` in `crates/game/src/app.rs` passes the
    error up, so the game would exit with a logged error in place of the panic.
- **RandR with a mode that has no timing** (a default Xvfb): no reader log line, no measurable CPU
  time over 59 s, rate unknown. Switching the output to a 90 Hz mode logged 90013 mHz and put
  90 Hz in force; switching back logged nothing and kept 90 Hz. This is the "no answer" path. A
  mode that computes to 0 mHz is pinned by `a_mode_is_its_dot_clock_over_its_scan` and
  `a_round_with_no_answer_keeps_the_last_one`.
