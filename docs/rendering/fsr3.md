# FSR 3.1, ported to WGSL

**What this is.** AMD FidelityFX Super Resolution 3.1's *upscaler* - the
temporal one - transliterated into WGSL compute shaders under
`crates/render/src/post/fsr3/`. Frame generation is deliberately not part of it
and never will be; [modern-features.md](../overview/modern-features.md) says why.

**Why a port and not the SDK.**
[ADR-0012](../architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md).
The short version is that driving the native Vulkan backend would cost lifting
`unsafe_code = "deny"`, a C++ toolchain, an `ash`/`wgpu-hal` version coupling
and Vulkan-only support, to inherit an FSR4 upgrade path that does not reach a
native Linux build.

**Status: all eight passes are ported, the chain produces a frame, and the
RECONSTRUCTION row selects it.** `upscale::Framebuffer::resolve_scene` reads
`oag_render::post::fsr3::Fsr3::output` on a race frame whose adapter has compute
shaders, and falls one rung to FSR 1 otherwise. It is off by default. What has
not happened is anybody looking at a *moving* frame it produced;
`HANDOVER.md`'s open-threads index points at the thread carrying that and the
rest of what is left.

## Provenance

| | |
| --- | --- |
| Upstream | [FidelityFX-SDK](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK) |
| Tag | `v1.1.4` |
| Commit | `c6efa6bf7f2027b3ec94f28578bb5965eabb9e55` |
| Licence | MIT - `licences/AMD-FidelityFX-MIT.txt` |
| Vendored? | **No.** Read from a scratch checkout; `just fsr-reference` fetches it |

The pin is load-bearing rather than bookkeeping. ADR-0012's only mitigation for
"a port is a fork" is that the WGSL stays *diffable* against a named upstream,
and a diff needs both sides named. `v1.1.4` specifically because 1.1.x is the
last line that is MIT source all the way down - SDK 2.x adds FSR4 as signed
binaries with no source, and nothing in this repository should point at a tree
containing them.

## What the renderer already supplies

Every input FSR 3.1 takes was built before the port opened, each for its own
reason, and each is documented where it landed:

| Input | Where it comes from |
| --- | --- |
| Colour, at render resolution | the offscreen scene target, allocated at the `render_scale` ceiling and drawn into a sub-rectangle ([ADR-0037](../architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md)) |
| Depth | stored and `TEXTURE_BINDING`-able since [ADR-0028](../architecture/adr/0028-camera-motion-blur-first.md) |
| Motion vectors | an always-on `Rg16Float` attachment every race draw writes ([ADR-0030](../architecture/adr/0030-velocity-buffer-motion-blur.md)) |
| Sub-pixel jitter | a Halton(2,3) offset post-multiplied onto the view-projection ([ADR-0039](../architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)) |
| A scene with no UI in it | [ADR-0036](../architecture/adr/0036-ui-composites-at-presentation-resolution.md) and [ADR-0038](../architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md) |

**The jitter turned out to already be upstream's own function.**
`ffxFsr3UpscalerGetJitterOffset` is `halton(index % phaseCount + 1, 2) - 0.5`
on x and base 3 on y - which is `oag_render::jitter::offset_pixels` line for
line, written before anybody had read the SDK. Only the phase *count* was ours;
`jitter::PHASES` documented itself as a plain sixteen standing in for a
ratio-derived count, and the port replaced it with
[`jitter::phases`](../../crates/render/src/jitter.rs), which is upstream's
`8 * (display_width / render_width)^2`.

## The passes

Upstream's dispatch order, which is also the order they must be ported in: a
pass reads what the ones before it wrote.

`Pass::ALL` in [`oag_render::post::fsr3`](../../crates/render/src/post/fsr3.rs)
is the same list and `the_pass_list_is_upstream_s_dispatch_order` asserts the
order, so this table and the code cannot drift apart silently.

| # | Upstream | Resolution | Ported |
| --- | --- | --- | --- |
| 1 | `prepare_inputs` | render | **yes** |
| 2 | `luma_pyramid` (SPD) | render/2 | **yes**, reduced to one dispatch - see below |
| 3 | `shading_change_pyramid` (SPD) | render/2, mips | **yes**, as a dispatch per level |
| 4 | `shading_change` | render/2 | **yes** |
| 5 | `prepare_reactivity` | render | **yes** |
| 6 | `luma_instability` | render | **yes** |
| 7 | `accumulate` | **presentation** | **yes** |
| 8 | `rcas` | presentation | **yes** |

**The chain is complete but unlooked-at.** Every pass is checked against
arithmetic worked out on the CPU, and no human has seen a frame it produced -
see [what is checked](#what-is-checked-and-how) for exactly how far that goes.

Three upstream passes are deliberately out of scope. `autogen_reactive` and the
TCR autogenerator exist to guess a reactive mask for engines that cannot supply
one; this renderer draws its own frame and can author the mask directly if it
ever needs to, so guessing it from two colour buffers is machinery with no
customer here. `debug_view` is a development overlay for the SDK's own sample.

## Deviations from upstream

ADR-0012 accepted that "some things cannot be transliterated at all" and named
SPD in advance. That prediction held, and the survey found one more.

### The SPD pyramids become a conventional reduction

Upstream's two pyramid passes use FidelityFX's Single Pass Downsampler: one
dispatch writes six mip levels, coordinating through a `globallycoherent`
read-write texture and a global atomic counter so the last workgroup to finish
can reduce the rest. **WGSL has no `globallycoherent` and no equivalent
memory-coherency guarantee across workgroups**, so the single dispatch cannot
be expressed. Each pyramid is built as a chain of ordinary reduction dispatches
instead, one per level.

This is a structural change, not an arithmetic one: the reduction each level
performs is upstream's, and the levels' contents are the same. What changes is
the number of dispatches and the loss of SPD's one-pass property.

**Only these two passes use `groupshared` at all.** Everything else under
`gpu/fsr3upscaler/` is per-pixel, which is what keeps the substitution isolated
to the two passes rather than infecting the port.

**And the luma pyramid turned out not to need a pyramid.** Its three products
are `farthest_depth_mip1` at mip 1, the pyramid itself at mip 6, and an
auto-exposure estimate at the 1x1 level. The second is SPD's own handoff between
workgroups and is overwritten by the shading-change pyramid before anything
reads it. The third has no reader here, for the reason below. So what is left is
`farthest_depth_mip1` - a single 2x2 box average - and the pass is one dispatch
rather than a chain.

### Every dispatch walks a texture extent, not a texel count

wgpu caps a dispatch at **65535 workgroups per dimension**, and the one pass
here that is not naturally two-dimensional walked straight into it: the clear
over `reconstructed_previous_nearest_depth` is a *buffer*, so it was dispatched
flat over `width * height / 64` - which a 2880x1800 render extent turns into
81000 and a panic in the frame loop.

It now walks the grid the buffer represents, at `extent / 8` per dimension like
every other pass. That makes the bound a property of what this port can
**allocate** rather than of the arithmetic at one call site: no texture it can
create comes within an order of magnitude of the cap, and a test says so at
sizes past any adapter's `max_texture_dimension_2d`.

The captures that were taken before this all ran at 720x408, where the flat
shape came to 4590 - which is the shape of the whole class: a fixture small
enough to be quick is a fixture too small to reach a limit.

### MSAA makes two of the inputs a different type

`msaa = "4x"` makes the scene's depth and
velocity attachments multisampled, and a multisampled binding is a different
WGSL type - so `prepare_inputs` is built **twice**, against a single-sampled and
a multisampled declaration of those two, and the dispatch picks by the sample
count the scene was actually built with.

Only that pass needs it: everything after it reads a target this port wrote
itself, and none of those is multisampled. The two builds share a byte-identical
body, because `textureLoad(t, p, 0)` is spelled the same either way - the `0`
being a mip level on one and a **sample index** on the other. Reading sample 0
rather than resolving is the decision [motion blur](motion-blur.md) already
made, for the same reason: a resolve averages velocity across a silhouette edge,
which is where the two surfaces have least in common.

**The sample count travels with the views, not with the setting.** MSAA's count
is baked into every scene pipeline when the scene is built, so the row and the
scene disagree for a whole race after a player moves it - which is what that
row's `restart_required` note is about, and which would otherwise hand the
upscaler a bind group of the wrong shape. Found by playing it; the compile-only
tests could not see it, and there is now one that can.

### RCAS is FSR 1's, with the denoise branch FSR 1 did not need

`ffx_fsr3upscaler_rcas.h` is a thin wrapper around `FsrRcasF` from
`fsr1/ffx_fsr1.h` - the same routine
[`oag_render::post::fsr1`](../../crates/render/src/post/fsr1.rs) already ports.
It is ported a second time rather than shared, because the two sit in different
scaffolding entirely (a fragment pass over a full-screen triangle against a
compute dispatch over a storage texture) and folding them together would mean
neither could be diffed against its own upstream file.

The one arithmetic difference is worth stating: FSR 3.1 sets
**`FSR_RCAS_DENOISE 1`** and FSR 1's use here does not. That enables a five-tap
luma term that scales the sharpening lobe down where the neighbourhood looks
like noise rather than like an edge. Upstream's own comment on the define
recommends applying film grain after RCAS instead of turning it on - but a
temporally accumulated frame carries reconstruction noise a single-frame sharpen
never sees, and sharpening that is exactly wrong.

### Auto-exposure is off, and `frame_info` is therefore not allocated

`Exposure()` comes from the frame-info target only when
`FFX_FSR3UPSCALER_ENABLE_AUTO_EXPOSURE` is set; otherwise it is the
application's, and an application that supplies none gets upstream's own 1x1
default, which holds zero and which `Exposure()` maps to exactly `1.0`. This
renderer draws into a low-dynamic-range target, has no exposure anywhere in its
pipeline, and gives a player a brightness *grade* applied after all of this
instead. Adding an automatic global brightness adaptation the original game does
not have would be inventing a feature rather than porting one, so the port takes
upstream's no-auto-exposure path and `exposure()` is the constant it produces.

The frame-info target's other product, `SceneAverageLuma()`, is **dead in
v1.1.4**: declared in `ffx_fsr3upscaler_common.h` and called from no pass header
at all. Checked across every one rather than assumed. With neither product read,
the target is not allocated.

If an HDR path ever lands and wants auto-exposure, the rest of the reduction -
a mip chain over `(log(luma), luma)` down to 1x1 and the
`ComputeAutoExposureFromLavg` smoothing - goes back into `luma_pyramid.wgsl`,
which says so. It is left out rather than left half-built.

### The game decides when a reset happens, not the port

Nothing here schedules a reset on its own initiative - `Fsr3::render` only
ever does what `Temporal::reset` tells it to, and that flag is produced
entirely on the `oag-game` side, by `race::scene::motion::CutWatch`. It
observes `Race::camera_view()` and the player's own `Race::respawns()` once a
frame and reports a cut whenever either has moved since the last observation:
a `SELECT`-cycled view change (`Race::set_camera_view`), or the player's craft
being teleported back onto the track after a `Reset` contact. Both hand the
next frame's resolve a history of a different shot than the one it is about
to draw, the same way the sequence's opening frame does, and
`scene::motion::record_frame` throws the history away for exactly the same
reason in both cases: `reset: phase == 0 || cut`.

Deliberately the *player's* respawn count and not the field's: the camera
being watched is the player's, and an opponent recovering elsewhere on the
circuit is not a cut in this shot. Pinned by `CutWatch`'s own unit tests
beside `record_frame`, which need no adapter - this is a pure book-keeping
question, not a pixel one. What a mid-race view switch or respawn actually
looks like through FSR 3.1 is unread; see the handover thread's open items
for playing it.

### A reset clears the accumulation it reads, as a render pass

Not a deviation in *what* happens but in *how*, and recorded because the first
version of this port did not do it at all. Upstream's `resetAccumulation` path
in `ffx_fsr3upscaler.cpp` schedules a backend clear of the accumulation SRV -
the half of `ACCUMULATION_1`/`_2` this frame will read - and of `SPD_MIPS`,
before the passes run. Setting `frameIndex` to zero is only the other half of a
reset: it makes every presentation pixel a *new sample*, so `accumulate` never
reprojects a history colour, but the history **weight** it blends that zeroed
colour in at still comes from `prepare_reactivity`, which reads the previous
frame's accumulation texture. Into targets that already ran a sequence - a race
restart, which is the common case, since `Fsr3` and its `Targets` outlive a
`Scene` - that texture says "fully accumulated" at every texel, and the first
frames of the new race come out darkened rather than being the plain
initial-sample upsample they are on a fresh allocation.

Here the clear is an empty render pass with `LoadOp::Clear`, the same wipe
`new_locks` gets every frame, so `accumulation` carries `RENDER_ATTACHMENT`
alongside its storage usage. `spd_mips` is **not** cleared: every level of that
pyramid is rewritten by the dispatch chain each frame, where upstream's
single-pass downsampler writes only the levels it reaches. Pinned by
`a_reset_reads_no_history_even_when_the_targets_still_hold_one`, which runs
three still frames into one set of targets with the third a reset, and reads
two thirds of a frame of history on that frame without the clear and none with
it.

## What is checked, and how

Nothing downstream consumes these passes yet, so a wrong reduction has no
visible symptom until `accumulate` lands. Each pass is therefore checked by
**reading its output back off a real device and comparing it against arithmetic
worked out on the CPU** - `post::fsr3::readback` runs the chain over a hand-built
8x4 depth buffer, filled by a fullscreen pass writing `frag_depth` because
WebGPU has no buffer-to-texture copy into a depth format.

Two properties are pinned that way, and both are the kind a comment cannot
defend:

- **`FindDepthExtents`' `fFarthest` is exactly the centre texel's own depth**,
  never the neighbourhood's maximum. It is initialised to the centre's depth and
  only ever `max`ed with a sample that was *nearer* than the running nearest -
  and every such sample is nearer than the centre was, so the value cannot move.
  This reads as a bug, it is upstream's, and it is what a future reader would
  "fix" into a real maximum.
- **The reduction is a plain `(v0+v1+v2+v3)*0.25` box average**, checked against
  the CPU-side view-space depth transform rather than against a second run of
  the same shader, so a wrong `deviceToViewDepth` cannot cancel itself out.

The fixture runs **more than one frame**, because half of what this port does is
temporal: a first frame has no history, so a pass that compares against the
previous frame reads the ping-pong's unwritten half and nothing about its output
is attributable. Two frames of a *still* scene is the sharpest instrument
available before `accumulate` lands, and it pins a third trap:

- **A still scene must report zero shading change**, and upstream reaches that
  answer through a value that looks like its opposite. Two neighbourhoods
  matching to within `FP16_MIN` set `fMinDiff` to `FP16_MAX`, and the *final
  multiply* is what turns that back into a zero. Drop that multiply - it reads
  like a redundant guard - and a perfectly still frame reports the largest
  shading change representable, at every level of the pyramid.

Its control is a scene that really did change, checked for both magnitude and
sign. That one has to use a **flat** colour where the others use a gradient:
`ComputeMinimumDifference` walks two sorted five-tap sets merge-style and keeps
the smallest relative difference between any pair, so across a gradient some
dark tap is nearer to some bright tap than the ratio between the frames, and the
answer is a fact about the neighbourhood rather than about the formula.

**What it does not check is whether a texel matches what `ffx_fsr3upscaler`
would have produced.** There is no reference implementation to diff against
without building the SDK, which ADR-0012 declined. What the readbacks establish
is that each pass computes what upstream's *source* says it should, worked out
independently; what remains unestablished is that the source was read correctly.
A `--presented` capture beside the `fsr1` and `off` ones is the instrument for
that, and it has not been taken.

### What it costs is measured, not argued

The chain is **two** `begin_compute_pass` calls and two `wgpu` timestamp
pairs, split where the resolution changes: six dispatches at the render extent
or half of it, then `accumulate` and `rcas` at presentation size. Bracketing a
single dispatch would need `TIMESTAMP_QUERY_INSIDE_PASSES` and that is not
WebGPU-portable, so a pass boundary is the finest split available - and this
is the one both readers want, per
[ADR-0045](../architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md):
a person deciding whether the widened formats or the missing FP16 path is
worth attacking, and a dynamic-resolution controller that has to know which
half of the cost it can lower. `oag_render::timing::PassTimer` already existed
for the scene pass ([dynamic resolution](dynamic-resolution.md)); FSR 3.1 gets
two rings of its own rather than a share of that one, because a slot spent
here is a frame the resolution controller does not get a scene reading for.
The two are claimed, abandoned and resolved together - `Fsr3::render` encodes
both passes or neither and says which.

The readings land on the `dev` performance overlay's dedicated GPU-cost panel,
top left, beside the scene pass's - `SCENE x.xx MS`, then `FSR3 REN x.xx MS`
for the half that scales and `FSR3 OUT x.xx MS` for the half that does not.
A render scale moves them in opposite directions: lowering it
makes the scene pass cheaper and gives the temporal resolve more to
reconstruct, and this runs *after* the scene pass rather than inside it - so
choosing a render scale on the scene reading alone is choosing on part of the
cost. See [dynamic-resolution.md](dynamic-resolution.md) for the other rows
that panel carries and the `OTHER` row that says what none of them measured.

`Fsr3::sizes` is reported the same way, once per allocation, as a log line at
`info`. It is the number [goals.md](../overview/goals.md)'s first-tier Steam
Deck question needs, and the table below is an argument until something prints
it.

The resolve has one property the others do not, and it needs jitter to see:
**a still scene must converge**. Each frame lands its samples somewhere new
inside each pixel, the history absorbs them, and after a full sequence the
answer stops moving - to within a residual that scales with the neighbourhood's
own contrast, because a converged history is still snapped to the rectification
box each frame. It is checked at 32 phases with the real Halton offsets, paired
with the direct statement that the chain believes what it has: a full
accumulation channel and no false disocclusion or shading change.

It caught the port's first substantive bug at pass 5, and one that would never
have produced an error message: **`accumulation` and `luma_history` are
render-sized upstream, not presentation-sized**, despite their names. Both are
properties of the *sample* being reprojected rather than of the pixel it lands
in. Allocated presentation-wide, only their top-left corner is ever written and
every read past it comes back zero - which reads as "this pixel has no history"
everywhere, and would have shown up as a permanently blurry picture with nothing
to point at.

Running the fixture at 8x4 rather than at a realistic size is deliberate, and
it paid for itself immediately: a 4x2 half-resolution target cannot hold six mip
levels, which wgpu rejects and which no test at 960x540 would ever have reached.
It caught a second one on the next pass - a dispatch cannot bind one texture as
both a storage write and a sampled resource, even when the shader ignores the
read, so the pyramid's level-0 dispatch had to be pointed at something other
than the level it writes.

### `HistorySample` shares its weights and keeps its taps

Two changes inside `accumulate.wgsl`'s 4x4 history fetch, both of which leave
the arithmetic alone and neither of which is upstream's shape. They are listed
here rather than left silent because this page's whole premise is that the WGSL
stays diffable against `ffx_fsr3upscaler_accumulate.h`, and a reader diffing
these two functions will find them rearranged.

**The Lanczos weights are computed once per axis, not once per row.** Upstream's
`Lanczos2` interpolation computes `w0..w3` from the fractional offset inside the
same function that blends four taps with them - which is fine where it is
written once, and the fetch calls it four times with the *same* horizontal
fraction. With `FFX_FSR3UPSCALER_OPTION_REPROJECT_USE_LANCZOS_TYPE` at
upstream's default of 0, every weight is the reference sinc and every `lanczos2`
is two `sin`: 40 of them for one presentation pixel, where 16 suffice.
`lanczos2_weights` is split out and the row loop is handed one `vec4<f32>`.
Same expression, same input, same result.

**The deringing clamp reuses the taps the fetch already loaded.** Upstream's
`DeclareCustomFetchBicubicSamples` loads the inner 2x2 a second time after the
separable pass, which is four extra `textureLoad`s out of twenty from the
largest texture this pass reads. Those four are exactly rows 1 and 2's two
middle taps, so the min and max are folded in as the loop passes them. `min`
and `max` are associative and exact on floats, so the four-way reduction is the
same one.

Both are per-presentation-pixel costs in the one pass that runs at presentation
resolution, which is why they were worth taking at all.

### The bind groups are two sets, not one a frame

Not a deviation from upstream at all - upstream's HLSL says nothing about
descriptor lifetime - but the shape a reader of `groups.rs` should expect.
Every resource FSR 3.1 binds is either a `Targets` member, which moves only when
an allocation does, or one of the three scene views, which move only on a
resize. What is left is the ping-pong parity, and a parity has two values. So
`groups::Cache` holds both sets and hands back the one this frame wants,
rebuilding when the scene views change identity, when MSAA changes which build
of `prepare_inputs` runs, or when `Fsr3::resize` replaces the targets.

`Fsr3::group_rebuilds` counts the rebuilds, because a cache hit and a rebuild
produce identical pixels and nothing else can tell them apart.

### The scattered depth store becomes a storage buffer

`ReconstructPrevDepth` projects each pixel's depth into the *previous* frame's
grid and writes up to four texels around the reprojected position, resolving
collisions with `InterlockedMin` on an `R32_UINT` UAV. Two properties follow
from that and neither is negotiable: the pass scatters, so it cannot be a
fragment pass; and it needs an atomic, so the target cannot be an ordinary
attachment.

Texture atomics are behind `wgpu::Features::TEXTURE_ATOMIC`. **A storage buffer
of `atomic<u32>` needs no feature at all** - buffer atomics are WebGPU core -
so the target is a buffer indexed `y * width + x`. The arithmetic is upstream's
`InterlockedMin` on the same bit-packed depth; only the resource kind differs.

**It is cleared by a compute dispatch, not by `clear_buffer`**, and that is
forced rather than chosen: `wgpu::CommandEncoder::clear_buffer` writes zero and
only zero, and zero is the *nearest* possible depth - `atomicMin` would keep it,
so every texel would read back as touching the near plane. The clear writes
upstream's own value for a non-inverted projection, `1.0` as its bit pattern.
`clear.wgsl` carries the argument, and it is also why that dispatch walks a
texture extent rather than a texel count.

### The intermediates are widened to baseline-storable formats

WebGPU guarantees storage-texture support for a fixed list of formats, and
`R16_FLOAT`, `RG16_FLOAT` and `R8_UNORM` - which is most of what upstream holds
its intermediates in - are **not** on it. In wgpu 30 they are render-attachment
formats only; a storage binding on them needs adapter-specific format features.

Rather than probe for those and carry two layouts, each intermediate is held in
the narrowest format the *baseline* guarantees:

**Storable is not the only property that matters, and the second one is easy to
miss.** `R32Float` is baseline-*storable* but not baseline-*filterable* -
`float32-filterable` is itself a WebGPU feature - so any intermediate some pass
reads through the linear sampler needs a format that is both. Which ones those
are was settled by listing every `Sample*` callback the remaining passes
actually **call**, rather than the ones that merely exist: `SampleAccumulation`,
`SampleCurrentLuma`, `SampleDilatedReactiveMasks`, `SampleLumaHistory`,
`SampleLumaInstability`, `SampleShadingChange` and
`SampleTransparencyAndCompositionMask`, and no others. `SampleInputDepth` is
declared and never called, which is why the depth attachment's unfilterable
binding never becomes a problem.

| Upstream | Here | Why |
| --- | --- | --- |
| `R16_FLOAT`, loaded only | `R32Float` | storable in the baseline, where `R16Float` is not. Twice the bytes, no loss |
| `R16_FLOAT`, sampled | `Rgba16Float` | storable *and* filterable, which `R16Float` is neither of and `R32Float` is only the first of |
| `R8_UNORM` | `Rgba8Unorm` | **not a widening at all**: storable, filterable, and the same 256 levels over the same `0..1` range |
| `RG16_FLOAT` | `Rgba16Float` | `Rg16Float` carries no storage guarantee. Two channels written, two wasted |
| `R8G8B8A8_UNORM` | `Rgba8Unorm` | already baseline |
| `R16G16B16A16_FLOAT` | `Rgba16Float` | already baseline |
| `R32_UINT` (atomic) | storage buffer | see above |

**This costs memory and buys portability**, and the trade is deliberate: a
widened intermediate is a number in a texture descriptor, while a feature probe
is a second code path that every future change has to keep working and that
this machine cannot exercise. The cost is real - a sampled scalar is four times
upstream's bytes - and it is unmeasured on the Steam Deck.

The consequence worth stating plainly: **FSR 3.1 here requests no wgpu feature
at all.** ADR-0012 expected an adapter probe with a fallback chain because the
storage formats were thought to be outside the baseline. The probe still exists,
but what it probes for turned out to be one thing rather than several - see
below.

### The precision loss that follows, and does not

Widening never loses precision; it is the reverse of what upstream does. Nor is
upstream's deliberate *quantisation* lost, which it would have been under an
earlier version of the table above: the `R8_UNORM` targets - `accumulation`,
`shading_change`, `new_locks` - are held at `Rgba8Unorm` and so round to the
same 256 levels over the same range that upstream's do. That matters because
accumulation is a feedback loop, and a loop that quantises differently drifts
differently.

What does remain divergent is the FP16 path upstream ships and this port does
not take: every intermediate here is at least as wide as upstream's, so any
arithmetic upstream performs in halves is performed here in floats. That is not
correctable by matching formats.

## The fallback ladder

ADR-0012's requirement is that a missing capability **degrade, never fail to
boot**. With no features requested, the ladder has one real rung to check:

1. **FSR 3.1**, when the adapter reports `DownlevelFlags::COMPUTE_SHADERS`.
2. **FSR 1**, when it does not - EASU and RCAS are fragment passes and run on
   anything that can draw a triangle. This is what ADR-0012 meant by FSR 1 being
   "load-bearing rather than a stepping stone".
3. **The blit's own bilinear tap**, if FSR 1's shaders will not compile either.

The decision is a pure function of the adapter's reported capabilities -
`oag_render::post::fsr3::supported` - and a player who selected `fsr3` on a
machine that cannot run it gets FSR 1 and a log line, with the setting still
saying `fsr3` so that the same settings file does the right thing on a machine
that can.

**The lowest rung cannot be exercised here.** Every adapter on the development
machine reports `COMPUTE_SHADERS`, so a green test run is not evidence that a
compute-less adapter degrades rather than failing to boot. Same shape of hole as
the missing-`TIMESTAMP_QUERY` case in
[dynamic resolution](dynamic-resolution.md).

## Colour space: gamma, and upstream would want otherwise

**FSR 3.1 accumulates linear light upstream, and cannot here.** The reasoning
for upstream's choice is sound - averaging several frames of one surface in an
encoded space weights a dark sample as brighter than it is, and the
reconstruction is doing arithmetic on light. [`post`](../../crates/render/src/post/mod.rs)'s
own table anticipated it and listed FSR 3.1 as wanting "a fourth thing", linear
light with its own tonemapping either side of accumulation.

That note predates the port and does not survive it.
[ADR-0020](../architecture/adr/0020-gamma-authoritative-colour-space.md) makes
gamma this renderer's authoritative colour space and says plainly that nothing
linearises: the PSP's blend equations are defined on stored framebuffer bytes,
the art was authored against that arithmetic, and the scene target holds gamma
values in a deliberately *non*-sRGB format. **There is no linear light in this
pipeline to hand FSR 3.1**, and manufacturing some for one pass would
reintroduce exactly the two-colour-spaces-at-once inconsistency that ADR
removed - the one that cost up to 73/255 on the boost plume and made every
measurement of one thing against another measure two spaces.

So the port reads the same view FSR 1 reads and accumulates encoded values.
That is a real divergence from upstream, in the same family as the FP16 path,
and it is the *consistent* choice rather than the accurate one - which is the
trade ADR-0020 already made on this renderer's behalf everywhere else.

Whether it is visible is unmeasured, and it is the first thing a comparison
capture should be looked at for: encoded accumulation biases a converging
history *bright* in the shadows, so a ghost trail behind a dark object is where
it would show.

## Jitter is not optional here

`--camera-jitter` is off by default because
[ADR-0039](../architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)
observed that jitter with nothing reconstructing from it makes the picture
strictly worse - a shimmer bought for nothing. The inverse is just as true: FSR
3.1 with no jitter has one sample per pixel per frame to reconstruct from and
degrades to a blurry reprojection.

So jitter is **derived from the active upscaler** rather than being an
independent setting: on when FSR 3.1 is the one actually running, off when the
ladder fell through to FSR 1 or the blit. The flag remains, as an override for
looking at jitter on its own.
