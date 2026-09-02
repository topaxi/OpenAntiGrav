# Modern platform features: upscaling, Steam Input, FMV enhancement

Desired long-term features recorded ahead of time, because each one has a
licensing question ("this must remain an OSS project") and one or two cheap
architectural decisions that are much easier to make early than to retrofit.
Upscaling is no longer only planning: FSR 1 is built, and the route for FSR 3.1
is decided in
[ADR-0012](../architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md).
The rest is M7-scoped ("modern features") or later and blocks nothing. Licence
statements were re-verified 2026-07-31; re-verify them again before acting on
them, because FSR4's changed once already and the change mattered.

## Upscaling: FSR 3.1, ported to WGSL

**Licensing, re-verified 2026-07-31.** AMD's FidelityFX SDK - the FSR 2/3.1
upscalers and FSR3 frame generation - is MIT-licensed C++/HLSL, with a Vulkan
backend. Fully compatible with this project; the shader source may legally be
vendored or ported, and FSR 1's has been.

**The FSR4 correction.** This section used to say FSR4 was a closed binary
distributed *through AMD's driver*, upgrading any game that exposed the FSR 3.1
API surface, and built its whole strategy on inheriting that upgrade for free.
That is no longer the shape of it, and on this project's first-tier platforms it
never pays out:

- FSR4 ships as **prebuilt, signed DLLs in FidelityFX SDK 2.0** (August 2025),
  with no source. A brief accidental source publication was withdrawn. SDK 2.2
  ("Redstone") adds FSR Upscaling 4.1 and Ray Regeneration on the same terms.
- The driver-side upgrade path is a **Windows/Adrenalin** mechanism. For a
  native Linux ELF it does not exist; on Linux it reaches only *Windows* games,
  through Proton's DLL substitution.

So **FSR 3.1 has to be worth having on its own merits** - it is - and no design
decision should be shaped around inheriting FSR4. Still hard-code nothing
FSR4-specific: if that path ever does reach a native build, exposing the 3.1 API
surface remains the way to inherit it.

**The route is decided: port to WGSL.**
[ADR-0012](../architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md) has
the reasoning. In short, driving the SDK's Vulkan backend through `wgpu-hal`'s
`as_hal` would cost lifting `unsafe_code = "deny"` for the first time, a C++
toolchain against ADR-0008's precedent, an `ash`/`wgpu-hal` version coupling,
and Vulkan-only support - to inherit an upgrade that does not reach here.

**Frame generation is deliberately out of scope.** FSR3 FG wants to interpose on
presentation, which wgpu does not expose - and the simulation is fixed 60 Hz
with unlocked-frame-rate presentation planned via state interpolation, so the
renderer can produce real frames at any rate. Interpolated fake frames plus
their latency buy little for a racing game.

**What is built.** FSR 1 (EASU then RCAS) is ported and shipped behind
`[graphics] upscaler`, defaulting off, and only runs where it is actually
magnifying - it is a magnifier, and asked to minify it undoes the supersampling
it was handed. It is also the middle rung of FSR 3.1's fallback chain, for
adapters that cannot supply the storage-texture features the temporal path
needs. See [`oag_render::post`](../../crates/render/src/post/mod.rs) for the
colour-space arrangement, which is the part most easily got wrong.

**What FSR 3.1 still needs**, none of which exists yet - the prerequisites this
section has always listed, now with their status:

| Prerequisite | State |
| --- | --- |
| Render resolution decoupled from presentation | **Done** - the render scale, and the offscreen target it draws into. Since [ADR-0037](../architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md) that target is allocated at the ceiling and drawn into a sub-rectangle, so a *per-frame* render size - which is what a temporal upscaler takes as a first-class input - costs a uniform write rather than a reallocation. Something moves it now: `[render_profiles.<title>] dynamic_resolution` names a target rate and a controller holds it, per title and off by default. See [dynamic resolution](../rendering/dynamic-resolution.md) |
| A depth buffer the upscaler can consume | **Done** - `StoreOp::Store` and `TEXTURE_BINDING`, read every frame by the motion blur pass ([ADR-0028](../architecture/adr/0028-camera-motion-blur-first.md)) |
| Per-pixel motion vectors from every draw | **Done** - the race writes an always-on `Rg16Float` velocity attachment from every draw's premultiplied previous-tick matrices ([ADR-0030](../architecture/adr/0030-velocity-buffer-motion-blur.md)); motion blur is its first consumer |
| Camera jitter, sub-pixel per frame | **Done** - a 16-phase Halton(2,3) offset post-multiplied onto the view-projection *after* the frustum and the motion snapshot, so neither culling nor the velocity buffer sees it ([ADR-0039](../architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)). Behind `--camera-jitter`, off by default: nothing reconstructs from it yet, so on its own it is a shimmer and a worse picture |
| A scene without UI in it | **Done** - the UI composites at presentation resolution ([ADR-0036](../architecture/adr/0036-ui-composites-at-presentation-resolution.md)) and a stage with no scene never enters the offscreen target at all ([ADR-0038](../architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)). The HUD and the scoreboard draw into the presentation target after the resolve, the performance overlay onto the surface after the grade, and the menus, front end, launcher and loading screen bypass the offscreen target entirely. What FXAA, SMAA and an upscaler now see is the race's 3D scene and nothing else |

That last row was the expensive one, and it is not on the original list because
it only became visible once something downstream needed a scene it could reason
about. FSR must consume a frame with no UI in it, and the UI must then be
composited at presentation resolution. **It is now done**, and it cost a
presentation-sized colour target plus one more fullscreen pass a frame - stated
in ADR-0036 rather than hidden.

**Every row is now filled.** Camera jitter, the last of them, landed on
2026-09-02 - see [ADR-0039](../architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md).
What FSR 3.1 needs from the renderer, the renderer now has; what remains is the
port itself.

It paid for itself twice over on the way. The HUD and the menus are no longer
resampled at all, which was a picture-quality complaint in its own right; and it
answered half of an open question without a measurement - the doubt over
defaulting `[graphics] upscaler` to `fsr1` was that a sharpener rings on
480x272-era paletted raster and coverage-atlas glyphs, and no sharpener reaches
either any more.

**Rows two and three are cleared, both by
[motion blur](../rendering/motion-blur.md)** - its camera-reprojection
stepping stone cleared the depth row
([ADR-0028](../architecture/adr/0028-camera-motion-blur-first.md)), and its
per-object velocity tier, now built as designed
([ADR-0030](../architecture/adr/0030-velocity-buffer-motion-blur.md)),
cleared the motion-vector row: every race draw writes an always-on
`Rg16Float` velocity attachment, deliberately independent of the blur
setting so FSR 3.1 can rely on it. The blur touches the last row only
partly: its chain runs before the HUD is drawn, so a UI-free scene
demonstrably exists at that point in the frame without being handed
downstream. The UI-free scene has since been handed downstream for real -
ADR-0036 and ADR-0038 - and sub-pixel camera jitter followed it on the same day
([ADR-0039](../architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)),
so **FSR 3.1 lacks no renderer-side prerequisite at all now.**

## Steam Input

**Licensing.** Steam Input is part of Steamworks, which is proprietary: the
SDK comes under Valve's licence and `steam_api` is a closed shared library.
Plenty of OSS games integrate it anyway; the repository simply must not
contain it.

**The pattern:**

- **Never vendor the SDK.** An off-by-default cargo feature (the `steamworks`
  Rust crate wraps the SDK; the bindings themselves are MIT) that
  dynamically links a user- or Steam-supplied `libsteam_api`. The repository
  stays 100% open; the feature only does anything in a Steam environment.
- **The open baseline is ordinary gamepad support** - `gilrs` (or SDL)
  feeding `oag-input`. Note that Steam Input mostly works with *no* API
  integration at all: Steam translates the user's controller configuration
  into synthetic gamepad/keyboard input. The explicit API buys action sets,
  per-context rebinding and button glyphs.
  **That baseline now exists**: `gilrs` feeds a hardcoded mapping in
  `oag-input`, alongside the keyboard's, and both merge into one abstract
  button state. See [packaging](../tools/packaging.md#gamepad) for the table.
  Nothing about Steam Input itself has moved out of planning.
- **Keep `oag-input`'s abstraction action-shaped**, which it already is: the
  crate exists to map devices onto an abstract button layer producing
  `InputSnapshot`. Steam Input's model is named actions in named action sets
  (menu vs race); as long as the boundary stays shaped that way rather than
  hardware-button-shaped, a Steam Input backend later is just another device
  mapper behind the same boundary, and the simulation never knows.

## Ahead-of-time upscaling of the prerendered movies

The PMF movies decode already, and the movie cache stores decoded video as
lossless AV1 keyed to its source ([ADR-0008](../architecture/adr/0008-av1-movie-cache.md)).
Neural upscaling of that video has a healthy open ecosystem - the binding
constraint is this project's own leakage rule, not the models:

**An upscaled FMV is derived game content and can never ship with the
project.** The pipeline must be user-side: read the user's own disc, write
into the gitignored `data/cache/`, exactly like the existing cache.

**Open models and tools** (code licences; see the trap below):

| Model / tool | Licence | Notes |
| --- | --- | --- |
| Real-ESRGAN | BSD-3 (code and official weights) | Handles compression artefacts well; PMF video is heavily compressed low-resolution H.264 |
| BasicVSR++ (MMagic) | Apache-2.0 | A true *video* model with temporal propagation - much less frame-to-frame flicker than a per-frame image model |
| SwinIR | Apache-2.0 | Strong image-restoration baseline |
| Anime4K | MIT | Real-time GLSL shaders, portable to WGSL - a playback-time alternative needing no cache or ML runtime |
| ncnn / ONNX Runtime / tract | BSD / MIT / Apache-2.0 | Inference runtimes; `realesrgan-ncnn-vulkan` is a self-contained Vulkan binary, no Python, the friendliest thing to shell out to |

**Two licensing traps:**

- **Code licence is not weights licence.** The community model zoo is full
  of fine-tuned ESRGAN-family weights under non-commercial or unspecified
  terms, and some research repos (VRT/RVRT) are CC-BY-NC. Recommend or
  automate only models whose *weights* are explicitly permissive -
  Real-ESRGAN and BasicVSR++ official releases qualify.
- **Bundle no weights regardless.** Even permissive weights are large binary
  blobs; the right shape is a tool or `just` recipe that downloads (or takes
  a path to) the model and runs the upscale locally, the way `data/` already
  works for everything else.

**Pipeline shape, when built:** decode PMF, run frames through the model,
encode into the movie cache as a *separate, version-stamped* extent - never
replacing the bit-identical lossless baseline, so the upscale stays
reproducible and disposable. Prefer a temporal model, or add temporal
stabilisation to a per-frame one; the intro reels are mostly motion, and
single-image upscalers shimmer on it.

**Evaluate the cheap alternative first:** Anime4K-class WGSL shaders at
playback time. Zero cache growth, zero Python/ONNX dependency, applies to
every video automatically. Lower quality ceiling than offline neural
upscaling, but for 480x272 stylised FMV upscaled to a modern display it may
be most of the win for a fraction of the machinery.

## Summary

| Feature | Licence status | Early decision |
| --- | --- | --- |
| FSR 1 upscaling | MIT, ported | **Built.** `oag_render::post::fsr1`, off by default, magnification only |
| FSR 3.1 upscaling | MIT, open | Port to WGSL ([ADR-0012](../architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)); every renderer-side prerequisite - motion vectors, readable depth, jitter, a UI-free scene - is now in place |
| FSR3 frame generation | MIT, open | Skip - interpolated presentation from the 60 Hz simulation is the better fit |
| FSR4 | Signed DLLs, no source; driver upgrade is Windows-only | Ship nothing proprietary and hard-code nothing FSR4-specific. Do not plan around inheriting it |
| Steam Input | Proprietary SDK | `gilrs`/SDL baseline in `oag-input`; optional non-vendored `steamworks` feature; keep the input layer action-shaped |
| FMV upscaling | Open models exist (mind the weights licences) | None - ADR-0008's cache design already leaves the door open; output stays in `data/`, never committed |
