# The GL backend (a trial)

wgpu's second tier: desktop OpenGL through EGL, and WebGL2 in a browser. The
game does not offer it. `oag_game::adapter::BACKENDS` is still
`Backends::PRIMARY`, there is no `gl:` row on the RENDERER menu and the web
build still requires WebGPU. This page is what a trial on 2026-10-15 found,
what was made to work, and what stands between GL and a row a player can pick.

**Verdict: desktop GL works on Mesa; WebGL2 and NVIDIA's GL do not draw a race.**
On Mesa (the Intel iGPU, and llvmpipe) every title boots to its front end and
races, and the stills match Vulkan's (table below). On WebGL2 the front end and
the menus draw and a race does not load. NVIDIA's proprietary GL cannot draw
one either. The recommendation is at the end.

## Turning it on

| Where | How |
| --- | --- |
| Desktop | `OAG_GL=1` adds `Backends::GL` to the instance (`adapter::GL_ENV`). Name the adapter in `graphics.renderer`, e.g. `gl: Mesa Intel(R) Graphics (ARL)`; the row never lists it, and an unknown name falls back to the default with the list of what the machine has in the log. |
| Desktop, window | The window's instance is made with the display handle (`adapter::instance_for_window`): an EGL display is built from the X11 or Wayland display, and without it a GL adapter enumerates but reports itself unable to present, so the window quietly stayed on Vulkan. |
| Browser | `OAG_WEB_FEATURES=web,webgl just web` adds wgpu's `webgl` feature; the page falls to WebGL2 when WebGPU is absent (`adapter::instance_with_webgpu_detection`). `scripts/web-screenshot.py --software webgl` runs headless Chromium without WebGPU, so it serves WebGL2 from swiftshader. |

On a machine whose EGL is NVIDIA's, force Mesa for the trial:
`__EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json`.
The loader report names the adapter's capabilities (block-compressed textures,
timestamp queries, compute, independent blend) and what GL costs the player:

```
GL backend, reduced: msaa: off (...); mapped shadows: no shadow (...); HD hull self-shadow: ...
GL backend: block-compressed textures yes, timestamp queries yes, compute yes, independent blend yes
```

## What works, per backend

Stills at 1440x816 (`--race --ticks 60`, `--presented` where named), one frame
per title, Vulkan and GL on the same Intel iGPU and the same binary. "Mean" is
the mean absolute per-channel difference out of 255.

| | Mesa GL 4.6 core (iGPU, window on llvmpipe) | NVIDIA GL (EGL, 3.3) | WebGL2 (Chromium, swiftshader) |
| --- | --- | --- | --- |
| Device | yes | yes once indirect validation is off (`adapter::descriptor`); not before | yes once the limits are lowered (`required_limits`); needs a canvas surface to enumerate |
| Front end, all five titles (`--menu-page main`, read as images) | Pulse, Pure, 2048 pixel-identical (max diff 0); HD max diff 4 and Omega 98, mean 0.001 or less (the HD and Omega backdrops, which `crates/game/shaders` draws, are among them) | not reached | Pulse: the intro movie, the menus, the backdrop and the ticker draw |
| Race, Pulse, Pure, HD, 2048, Omega | draws; mean diff 0.000 to 0.004 | the first race pipeline fails (`INDEPENDENT_BLEND`) | load fails: 77 pipelines refused (`INDEPENDENT_BLEND`), then `sampler2DMS` |
| HD Zone, Omega Zone (`--mode zone --zone-stage 3`) | mean 0.019 and 0.005 (0.05% and 0.01% of pixels differ by more than 8) | no | no |
| Bloom, FXAA, SMAA, FSR 1 | yes, mean 0.001 to 0.008 | no | not reached |
| FSR 3 (compute) | yes, mean 0.003 | reports the compute flag, so it would claim support; untested past #1 | no compute: runs as FSR 1, named in the log |
| Motion blur at speed, rocket pickup, opponents, FSR 3 (900 ticks, autopilot) | yes, mean 0.008 | no | no |
| Block-compressed textures | yes: HD keeps its BC blocks, no RGBA8 decode | yes | yes |
| Timestamp queries | yes | not reached | no (no dynamic resolution signal) |
| MSAA | off (see below) | - | off |
| Shadows | blob and `original` work; `mapped` draws none; HD hull keeps coverage, loses its own shadow | - | - |

Mesa GL is the only backend that races, and it is as good as Vulkan except for
four things, all named in the loader report.

## Failures, classified

The order is how much of the game each one costs, worst first. (a) is a feature
we switch off on GL and name; (b) is a limit we lower; (c) is a blocker.

| # | Failure | What it hits | Class | Status |
| --- | --- | --- | --- | --- |
| 1 | `Downlevel flags INDEPENDENT_BLEND are required` on every pipeline that has a second colour target: `mesh`, every `mesh blend`, glow stamps, alpha test, `psys`, exhaust, beam, shadow, flash, weapon quads | **WebGL2 and NVIDIA GL: every race.** The scene pass writes colour and a velocity target whose write mask differs (`Velocity::target`), which GL without `ARB_draw_buffers_blend` (WebGL2, a 3.3 core context) cannot express | (c) for now; (a) if the scene can be built with no velocity target there | Not wired. Mesa has it. See "What is left" |
| 2 | `A image was used with multiple samplers` in the mesh shader | **Every title on every GL**: GL has one sampler per texture unit, and `zone_tex` and the three other Zone stage textures were read through `zone_sampler` and `zone_nearest_sampler` | (a), no player loses anything | **Wired**: `zone.wesl` `zone_band` fetches the texel itself under the `gl` feature. HD and Omega Zone stills match Vulkan |
| 3 | `no matching function for call to textureLod(sampler2DShadow ...)` and `textureLoad from depth textures is not supported in GLSL` | **Every title on every GL**, in the one shared mesh shader: a depth texture can be compared through a comparison sampler on GL and not sampled or fetched | (a) | **Wired**: the `gl` variant of `depth_taps_lit` and `self_shadow_taps_lit` returns "every tap lit". Costs the `mapped` shadow tier and HD's hull self-shadow term |
| 4 | `glTexStorage2DMultisample(target=GL_TEXTURE_2D)`, then incomplete framebuffers and a black scene | MSAA, any title. wgpu-hal gives a multisample texture that is also sampled the wrong target (renderbuffers are fine; the scene's MSAA targets are read by the post chain) | (a); an upstream wgpu fix would restore it | **Wired**: `Scene::new` clamps MSAA to off on GL and warns once. WebGL2 would also fail at `sampler2DMS` (GLSL ES 3.0 has no multisample sampler) |
| 5 | `wgpu-hal heuristics assumed that the view dimension will be equal to D2 rather than D2Array`, eight times a run | A one-layer texture viewed as an array: the two shadow placeholders | (b), cosmetic | **Fixed everywhere**: the placeholders have two layers; nothing samples them |
| 6 | `Limit 'max_compute_workgroups_per_dimension' value 65535 is better than allowed 0` | WebGL2 device creation | (b) | **Wired**: `required_limits` is the worse of wgpu's default and the adapter's. Vulkan, Metal and Dx12 exceed the default, so for them it is the default exactly |
| 7 | `indirect-validation error: ComputePipeline ... doesn't support Features(BUFFER_STORAGE | COMPUTE_SHADER ...)` then a lost device | NVIDIA GL: wgpu-hal asks EGL for exactly a 3.3 context (`gl_context_attributes`), NVIDIA returns exactly that, and wgpu builds a compute pipeline for indirect-call validation with the device. Mesa returns 4.6 for the same request | (b) | **Wired**: the GL instance drops `VALIDATION_INDIRECT_CALL`; this engine issues no indirect draw or dispatch. The device then creates, and #1 is the next wall |
| 8 | WebGL2 has no adapter without a canvas | Browser fallback | (b) | **Wired** for the trial: `web::prepare` makes a surface on the page's canvas first when WebGPU is absent |

**2048 and Omega check (the Omega and 2048 rule): `ported`.** Both race on GL
through the one shared mesh shader and race path, and their stills are in the
table; nothing here is title-specific, so there is no second wiring to do.

**Found on the way, not in this lane:** the local `/usr/bin/chromium` (154)
refuses the mesh shader on WebGPU too, on `main`'s shader and on this branch's:
`textureSample must only be called from uniform control flow` at the four
`zone_nearest_sampler` reads in `zone_glow`, called from `lit_texel`'s
`slots` branch (the `@diagnostic(off, derivative_uniformity)` on `lit_texel`
does not reach the callee in that Tint). So a WebGPU race walk could not be
re-run here to compare. The default web build was compiled and its
front end ran to the same point as `main`'s; this branch's `zone.wesl` keeps
those four calls inline in the default variant so that it is the same text
as before.

Not hit, and worth knowing: compute passes (FSR 3, Omega's tonemap) work on
Mesa GL 4.6, and FSR 3 on WebGL2 already degrades to FSR 1 through the existing
`oag_post::fsr3::supported` check. Timestamp queries are present on Mesa and
absent on WebGL2, and the pass timer already checks the device. The loader
report said for WebGL2: block-compressed textures yes, timestamp queries no,
compute no, independent blend no.

## What is left

1. **A scene with no velocity target.** #1 is the whole of WebGL2 and NVIDIA GL.
   `Velocity::None` pipelines exist; the work is a scene whose render pass has
   one colour attachment, about twenty `Velocity::Write` call sites in
   `oag-raceplay`, the pass setup in `scene/motion.rs`, and the effects in `oag-fx`
   and `oag-render` that build their own target lists. The player loses motion
   blur and FSR 3's motion vectors, and WebGL2 had neither. **Measure what is
   behind it before promising a WebGL2 race**: a WebGL2 context refused 77
   pipelines for one reason, and a second one (`sampler2DMS`, behind MSAA) is
   known; nothing past those was reachable.
2. **A comparison sampler for the depth reads**, which would bring the `mapped`
   tier and the HD hull self-shadow back on GL. It is a different sampler
   binding type on every backend, so it is a layout change, not a GL branch.
3. **MSAA on GL** waits on wgpu-hal giving a sampled multisample texture the
   `TEXTURE_2D_MULTISAMPLE` target, or on resolving the scene's MSAA targets
   through a pass of ours.
4. **NVIDIA's EGL** gives a 3.3 context where Mesa gives 4.6. wgpu-hal does not
   offer a way to ask for more; NVIDIA has Vulkan everywhere it has GL, so this
   is only a reason not to offer a `gl:` row on a machine that has both.

## Recommendation

- **Desktop RENDERER row: not yet, and if built, an explicit `gl: <adapter>`
  entry on Mesa only.** A Mesa GL adapter races every title with three named
  reductions (MSAA, `mapped` shadows, HD hull self-shadow). Most machines
  that have Mesa GL have Vulkan beside it, and Vulkan loses none of the three
  (frame times were not compared), so for them the row is a worse choice a player
  could make. The case for it is a machine whose only driver is GL (an older GPU
  with no Vulkan ICD). If the maintainer wants that, the work is small: add the
  row, filter out a GL adapter that lacks independent blend or compute 4.3 (so
  NVIDIA's EGL is not offered), and keep the loader line.
- **Web fallback for a browser with no WebGPU: not yet.** The front end draws
  and the menus work, but a race does not load, and a page that opens to a
  menu and then stops is worse than the clear "WebGPU is required" message it
  has today. Revisit after "What is left" 1.
- Whichever is built, the `gl` shader variant, the capability line and the
  named reductions are already in, and `the_mesh_pipelines_build_on_gl`
  (`oag-mesh`) builds the real pipelines on a real GL device wherever the
  machine has one with independent blend (it skips and says so elsewhere; run it
  with the Mesa EGL variable above).

## Reproducing

Setup used for the web half: `rustup toolchain install nightly-2026-10-08
--profile minimal --component rust-src --target wasm32-unknown-unknown`
(added to the machine's `~/.rustup`) and `cargo install wasm-bindgen-cli
--version 0.2.129 --root <a scratch dir>` (never in `~/.cargo/bin`).

```sh
# Vulkan and GL stills, same machine
OAG_GL=1 __EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json \
  oag-game --no-audio --race --ticks 60 --screenshot gl.png data/images/pulse-psp-eu.chd
# with graphics.renderer = "gl: Mesa Intel(R) Graphics (ARL)" in settings.toml
# (the settings file names the adapter; the log lists what the machine has)

# WebGL2
OAG_WEB_FEATURES=web,webgl scripts/build-web.sh --dev
uv run --with playwright python3 scripts/web-screenshot.py --dist target/web/dist \
  --image data/images/pulse-psp-eu.chd --out data/web-shots --software webgl \
  --at 14,22,28 --press Enter@8 --press Enter@11 --press Space@14 \
  --press Enter@18 --press Enter@22
```

Vulkan stills were compared before and after this change with `cmp`: Pulse and
Pure and HD (default, `--shadows mapped`, `--shadows original`, and an HD Zone
race) are byte-identical, so nothing changed on a backend that was already
offered.
