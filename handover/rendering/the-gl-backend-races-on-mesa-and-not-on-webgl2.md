---
categories: [rendering, tooling]
---

# The GL backend races on Mesa and not on WebGL2

2026-10-15, queued by the maintainer: can wgpu's GL tier be a RENDERER row on
the desktop, and a fallback for a browser with no WebGPU? The trial and its
failure table are `docs/tools/gl-backend.md`. Short answer: Mesa GL 4.6 races
all five titles within noise of Vulkan (three named reductions); WebGL2 boots to
the menus and refuses a race load; NVIDIA's EGL gives a 3.3 context and refuses
the first race pipeline.

Landed: `OAG_GL=1` (a dev switch, no menu row), the window's display handle for
GL, a `gl` variant of the mesh shader chosen by `oag_gpu::backend::is_gl`, MSAA
clamped off on GL, the GL gaps and the adapter's capabilities in the loader
report, `required_limits` lowered to the adapter's, `webgl` as a build feature
of the web build, and `the_mesh_pipelines_build_on_gl`.

## Open

- **WebGL2 and NVIDIA GL: `INDEPENDENT_BLEND`.** The scene's second colour
  target (velocity, write-masked on blended draws) differs in write mask from
  the first, which these contexts cannot do. 77 pipelines refused on WebGL2;
  `sampler2DMS` (MSAA's prepare shader) is the next error and nothing behind
  those was reachable. Unknown: how many more walls follow.
- **`mapped` shadows and HD's hull self-shadow are off on GL.** A depth texture
  cannot be sampled or fetched in GLSL, only compared; the compare wants a
  comparison sampler, a different binding type on every backend.
- **MSAA is off on GL.** wgpu-hal 30 allocates a sampled multisample texture
  with `GL_TEXTURE_2D` as its target. Not reported upstream.
- **A desktop `gl:` row is not built**: most machines with Mesa GL have Vulkan,
  and Vulkan keeps all three. Maintainer's call (see the page's
  recommendation).
- **Frame times were not compared** between Vulkan and Mesa GL.
- 2048 and Omega check: both raced on GL over the one shared
  shader and path (`ported`); nothing title-specific is wired.

- **Unrelated, found here: the web build's mesh shader is refused by Chromium 154's
  Tint** (`textureSample must only be called from uniform control flow`, in
  `zone_glow`, on `main` too). The web race walk cannot run on this machine's
  Chromium until that is fixed; check whether CI's browser is older.

## Next Steps

1. Build the scene without a velocity target when the device lacks independent
   blend (`Velocity::None` pipelines exist; about twenty `Velocity::Write` call
   sites, `scene/motion.rs`, the effects in `oag-fx` and `oag-render`).
   Re-run the WebGL2 walk (`scripts/web-screenshot.py --software webgl`) to see
   what is behind it. An afternoon for the call sites, an unknown after.
2. If the maintainer wants a desktop row: offer a GL adapter only when it has
   independent blend and 4.3 compute, label it, and keep the loader line.
3. Try a comparison sampler for the depth reads on every backend and `cmp` the
   Vulkan stills; it would bring the two shadow terms back on GL.
