# Modern platform features: upscaling, Steam Input, FMV enhancement

Desired long-term features recorded ahead of time, because each one has a
licensing question ("this must remain an OSS project") and one or two cheap
architectural decisions that are much easier to make early than to retrofit.
All of this is M6-scoped ("modern features") or later; nothing here blocks
current work. Licence statements are as of 2026-07 - re-verify them when M6
actually opens, especially FSR4's.

## Upscaling: FSR 3.1, with FSR4 arriving via the driver

**Licensing.** AMD's FidelityFX SDK - the FSR 2/3.1 upscalers and FSR3 frame
generation - is MIT-licensed C++/HLSL. Fully compatible with this project;
the shader source may legally be vendored or ported. **FSR4 is not**: it is
the ML-based generation, shipped at launch as a closed binary distributed
through AMD's driver, upgrading games that expose the FSR 3.1 API surface
(3.1 deliberately decoupled its interface from the implementation for
exactly this). AMD has signalled open-sourcing intent, but plan as if it
stays closed.

**Strategy: implement FSR 3.1 properly and never touch FSR4 code.** On
capable hardware and drivers the user gets FSR4 through the driver-side
upgrade path without this project shipping or linking anything proprietary.
Do not hard-code anything FSR4-specific.

**Integration cost is technical, not legal.** The SDK targets Vulkan and
DX12 natively; this project renders through wgpu. Two routes, undecided:

1. Drive the SDK's Vulkan backend through wgpu-hal's `as_hal` escape hatches
   (unsafe interop, keeps upstream's tested implementation), or
2. port the upscaler shaders to WGSL (legal under MIT; a large but
   self-contained effort - community WGSL ports of FSR1's two spatial passes
   exist and are a sensible first rung).

**Frame generation is deliberately out of scope.** FSR3 FG wants to
interpose on presentation, which wgpu does not expose - and the simulation
is fixed 60 Hz with unlocked-frame-rate presentation planned via state
interpolation, so the renderer can produce real frames at any rate.
Interpolated fake frames plus their latency buy little for a racing game.

**What to decide early, in `oag-render`, when its pipelines grow beyond the
current mesh/ribbon stage** - temporal upscalers have prerequisites that are
nearly free to design in and expensive to retrofit:

- every draw must be able to emit per-pixel **motion vectors**;
- a real **depth buffer** the upscaler can consume;
- **camera jitter** support (sub-pixel projection offsets per frame);
- **render resolution decoupled from presentation resolution** (dynamic
  resolution is already on the M6 list, so this aligns with existing plans).

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
| FSR 3.1 upscaling | MIT, open | Design `oag-render` for motion vectors, depth, jitter, decoupled render resolution |
| FSR3 frame generation | MIT, open | Skip - interpolated presentation from the 60 Hz simulation is the better fit |
| FSR4 | Closed binary via driver (re-check at M6) | Expose the FSR 3.1 API surface; let drivers upgrade it; ship nothing proprietary |
| Steam Input | Proprietary SDK | `gilrs`/SDL baseline in `oag-input`; optional non-vendored `steamworks` feature; keep the input layer action-shaped |
| FMV upscaling | Open models exist (mind the weights licences) | None - ADR-0008's cache design already leaves the door open; output stays in `data/`, never committed |
