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

**Status: in flight.** The [pass table](#the-passes) below is the port's spine
and says which passes exist; `HANDOVER.md`'s open-threads index points at the
thread carrying what is not done yet and the traps found on the way.

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
| 5 | `prepare_reactivity` | render | no |
| 6 | `luma_instability` | render | no |
| 7 | `accumulate` | **presentation** | no |
| 8 | `rcas` | presentation | no |

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

Running the fixture at 8x4 rather than at a realistic size is deliberate, and
it paid for itself immediately: a 4x2 half-resolution target cannot hold six mip
levels, which wgpu rejects and which no test at 960x540 would ever have reached.
It caught a second one on the next pass - a dispatch cannot bind one texture as
both a storage write and a sampled resource, even when the shader ignores the
read, so the pyramid's level-0 dispatch had to be pointed at something other
than the level it writes.

### The scattered depth store becomes a storage buffer

`ReconstructPrevDepth` projects each pixel's depth into the *previous* frame's
grid and writes up to four texels around the reprojected position, resolving
collisions with `InterlockedMin` on an `R32_UINT` UAV. Two properties follow
from that and neither is negotiable: the pass scatters, so it cannot be a
fragment pass; and it needs an atomic, so the target cannot be an ordinary
attachment.

Texture atomics are behind `wgpu::Features::TEXTURE_ATOMIC`. **A storage buffer
of `atomic<u32>` needs no feature at all** - buffer atomics are WebGPU core -
so the target is a buffer indexed `y * width + x`, cleared per frame with
`clear_buffer`. The arithmetic is upstream's `InterlockedMin` on the same
bit-packed depth; only the resource kind differs, and the clear is cheaper than
the texture clear it replaces.

### The intermediates are widened to baseline-storable formats

WebGPU guarantees storage-texture support for a fixed list of formats, and
`R16_FLOAT`, `RG16_FLOAT` and `R8_UNORM` - which is most of what upstream holds
its intermediates in - are **not** on it. In wgpu 30 they are render-attachment
formats only; a storage binding on them needs adapter-specific format features.

Rather than probe for those and carry two layouts, each intermediate is held in
the narrowest format the *baseline* guarantees:

| Upstream | Here | Why |
| --- | --- | --- |
| `R16_FLOAT` | `R32Float` | `R32Float` is baseline-storable; `R16Float` is not |
| `RG16_FLOAT` | `Rgba16Float` | `Rgba16Float` is baseline-storable; `Rg16Float` is not. Two channels are written, two are wasted |
| `R8_UNORM` | `R32Float` | same reason as `R16_FLOAT`; the value is a `0..1` scalar either way |
| `R8G8B8A8_UNORM` | `Rgba8Unorm` | already baseline |
| `R16G16B16A16_FLOAT` | `Rgba16Float` | already baseline |
| `R32_UINT` (atomic) | storage buffer | see above |

**This costs memory and buys portability**, and the trade is deliberate: a
widened intermediate is a number in a texture descriptor, while a feature probe
is a second code path that every future change has to keep working and that
this machine cannot exercise. The cost is real - the render-resolution scalar
targets are four times the bytes upstream uses, not two, because the alternative
to `R8_UNORM` is `R32Float` and not `R16Float` - and it is unmeasured on the
Steam Deck.

The consequence worth stating plainly: **FSR 3.1 here requests no wgpu feature
at all.** ADR-0012 expected an adapter probe with a fallback chain because the
storage formats were thought to be outside the baseline. The probe still exists,
but what it probes for turned out to be one thing rather than several - see
below.

### The precision loss that follows, and does not

Widening never loses precision; it is the reverse of what upstream does. What
*is* lost is upstream's deliberate quantisation: `R8_UNORM` accumulation and
reactive masks quantise to 256 levels, and holding them at full float means this
port's history behaves marginally differently from a reference run. That is a
divergence in the same family as the FP16 path upstream ships and this port does
not take, and it is not correctable by matching formats alone.

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

## Colour space

**FSR 3.1 wants linear light**, which is a different answer from FSR 1's and is
why [`post`](../../crates/render/src/post/mod.rs)'s own table always listed it
as a fourth consumer rather than folding it in with the others. Accumulation
averages several frames of the same surface, and averaging sRGB-encoded values
weights a dark sample as though it were brighter than it is; the reconstruction
is being asked to do arithmetic on light, so it must be handed light.

Concretely: FSR 3.1 reads the scene target through its **sRGB** view - the one
that decodes on read - where FSR 1 reads the non-sRGB one. Its output is linear
too, and the blit's grade therefore must not decode it a second time.

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
