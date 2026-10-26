---
status: parked
titles: [all]
updated: 2026-10-15
summary: "GL backend trial parked on branch gl-backend, unmerged; revisit only on demand"
categories: [rendering, tooling]
---

# The GL backend trial is parked on a branch, unmerged

2026-10-15, drive lead. Maintainer decision: no GL and no WebGL until there is actual demand
and use, and the GL backend is best effort - it never blocks or reshapes the default backends
(Vulkan, Metal, Dx12, WebGPU). The trial (`gl-backend`) is pushed as branch `gl-backend`
(`9ebcb262c`, main `f61e42ac8` merged in) and deliberately **not merged**. Its own docs
(`docs/tools/gl-backend.md`, the failure table) and its handover thread live on that branch.

## What the trial found

- **Mesa GL 4.6** (Intel iGPU, llvmpipe): every title boots and races, FSR 3 and motion blur
  included, close to Vulkan, with three things off and named in the loader report: MSAA
  (wgpu-hal 30 allocates sampled multisample textures with the wrong target), `mapped`
  shadows, and HD's hull self-shadow (GL compares depth textures but cannot sample them).
- **WebGL2** (headless Chromium) and **NVIDIA's GL 3.3**: menus draw, race load fails - 77
  pipelines refused for lack of independent blend (the velocity target has its own write
  mask). A scene with no velocity target is the fix, about 20 call sites, not built.
- The default build would carry a second compiled mesh shader (`mesh_gl.wgsl`); taking the
  branch further means putting all of GL behind a cargo feature that is off by default first.
  That gating was in progress, uncommitted, when the lane was parked and is lost.

## Open

- Revisit only on demand. Start from the branch, gate GL behind an off-by-default feature,
  then the velocity-less scene for independent blend.
