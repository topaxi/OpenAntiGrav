# Dynamic resolution

**Status: it works, and it is off by default.** `target_fps` names a
target frame rate, `minimum_resolution` bounds how far the picture may
shrink, and `render_scale` becomes the ceiling. All three live in
`[render_profiles.<title>]` and so are **per title**, which is the right table
for them: a target Pulse holds comfortably is not one HD/Fury holds, and the
whole point of the feature is that the answer depends on how expensive the
scene is - the controller reads it and
never writes it. Every scene-resolution pass takes a resource size and a
viewport separately, the scene pass is timed on the GPU every frame, and
`crates/game/src/drs.rs` turns the second into the first.

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

Measured, not reasoned: `crates/render/tests/msaa_resolve_viewport.rs` primes a
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
`oag_render::timing::PassTimer` is a ring of four timestamp pairs;
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
| `motion_blur` | `drs::Cost::scalable` | Also falls with the extent; timed as one chain across six render passes since ADR-0042, via `PassTimer::half_writes` |
| `hd_bloom` | `drs::Cost::scalable` | Also falls with the extent; timed as one chain the same way since ADR-0043. `Scene::has_hd_bloom` tells "ring full" from "no chain to measure" - Pulse, Pure, and an HD circuit with no `HDR and Bloom` block all have none |
| The FSR 3.1 chain | `drs::Cost::fixed` | Timed (`Session::upscale_cost`), but does not fall with the extent - its temporal accumulate and RCAS passes run at presentation size. Counted in full rather than split, which is conservative: some of the chain does shrink with the extent, and treating it all as fixed under-states the true budget |
| `bloom` | The residual | Not timed. Draws at a fixed 240x136 regardless of scale, so - unlike `hd_bloom` - it does not belong in `scalable` even once it is timed |
| FXAA, SMAA, FSR 1, the blit, the HUD, the composite, the perf overlay, AI/physics for opponents beyond the player | The residual | Presentation-resolution, driver overhead, or CPU-side work off the render-extent path entirely - permanently outside the extent's reach |

Each pass is timed and summed rather than the encoder being bracketed
first-to-last: bracketing would need `TIMESTAMP_QUERY_INSIDE_ENCODERS`, which is
not WebGPU-portable, and would have to know which pass is last - which varies
with the bloom, motion-blur and HD-chain settings.

The `dev` overlay's own GPU-cost panel - top left, separate from the
frame-time panel at top right - prints one row per timed reading, `SCENE`,
`BLOOM`, `BLUR`, `FSR3`, each only once it has a reading, followed by an
`OTHER` row: `perf::Meter`'s own wall-clock frame time minus whatever the rows
above add up to. That is the residual made visible rather than assumed - see
`perf::GpuCost::rows` and `residual_ms`, and, for the same subtraction fed back
into the budget instead of only printed, "The residual learns" below. Its own panel rather
than a block inside the frame-time one, so a reader is not holding five
numbers on one crammed line.

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

`crates/game/src/drs.rs`, and it reads nothing: fed one `drs::Cost` a frame - the
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
is `drs::Cost::fixed` - the FSR 3.1 chain's own GPU timing, already measured
and previously discarded - and the residual opens each session at
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
  side. `Controller::unreachable()` requires `RISE_PATIENCE` consecutive frames
  of "nothing left to decide" before it reports true, for the same reason a
  rise needs earning: at a target the machine can only just miss, the
  underlying condition flickers between frames, and a caller that logged every
  flicker would print continuously about a situation that had not changed.
  Measured: 68 raw per-frame occurrences in 41 seconds, 7 after requiring a run.

### The rows

`[render_profiles.<title>] target_fps` names the rate and `off` is one
of its values -
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

### The target is held to the frame limiter

Aiming above the limiter asks for frames the loop is not allowed to produce. At
`target_fps = 144` behind a 60 limit the budget is 2.4x too tight, so the
controller drops the resolution permanently to hold a rate the limiter forbids
whatever it does - every pixel given up buys nothing.

`drs::Target::at_most` is the clamp and the menu row's own `warn_when` is the
visible half: the row says what was asked for, the clamp holds the controller
to what will actually be produced. Neither would be honest alone.

**Both stop at `Vsync::On`.** There the *display* is the bound and this build
cannot ask a surface what its refresh is. The only fallback available is the
simulation's 60 - which `Session::presentation_hz` does take for the overlay's
graph scale, and says so - and taking it here would silently cap a 144 Hz
panel's target at 60 while the row still read 144. A wrong clamp is worse than
none.

**On an adapter with no `TIMESTAMP_QUERY` there is no signal and the extent
never moves, and the row cannot say so by greying out.** `disabled_by` names a
*setting*, and `Definition::check_condition` requires a row that edits it - an
adapter capability is neither. An earlier version of this page claimed the
`disabled_by` mechanism covered this; it was written before anyone read
`check_condition`, and this is the correction. The row is stored and inert,
which is the same thing `restart_required` already means.

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
  machine is makes every one of them irreproducible. `crate::perf` already makes
  exactly this argument for why the performance overlay is window-only; it
  applies verbatim. A load is not a frame either - `Session::stalled` exists for
  that and clears the meter, and a controller needs the same guard or a track
  load drives the scale to the floor and takes seconds to climb back.
