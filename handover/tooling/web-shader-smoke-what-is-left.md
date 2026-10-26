---
status: blocked-maintainer
titles: [all]
updated: 2026-10-15
summary: "web-shaders CI job unrun on GitHub; the maintainer pushes the branch and fixes runner assumptions"
---

# The web shader smoke test: what is left

2026-10-15, lane `webgpu-smoke`. `just check-web-shaders` creates all 44 WGSL
modules the web build emits in headless Chromium 154 on its CPU adapter and
fails on any compile error; the Chromium 154 rejection of the mesh shader is
fixed (`@diagnostic(off, derivative_uniformity)` on `zone_glow`, `zone_sample`
and `sample_at_slope_level`). The record is [web.md](../../docs/tools/web.md),
"The shader smoke test" and "Shaders".

## Open

- **The CI job is unrun.** `web-shaders` in `.github/workflows/ci.yml` (Chrome
  stable and beta through `browser-actions/setup-chrome@v1`, `pip install
  playwright`, `OAG_REQUIRE_WEB_SHADERS=1`) was written without GitHub Actions
  to run it on. Unverified: that `setup-chrome`'s `chrome-path` output is what
  Playwright's `executable_path` wants, that Chrome on the runner reports a
  `swiftshader` adapter headless (the local run was Arch's Chromium 154), that
  the `cargo check` of six crates fits the runner's disk and time (the main
  `check` job needed its disk freed), and that `@v1` is the right major. The
  first red run names which.
- **Firefox is local-only.** It accepts all 44 modules (so its naga and Tint
  differ) but gave no adapter under lavapipe on this machine, so a GPU-less
  runner cannot run it. `just check-web-shaders --browser all` on a machine
  with a GPU. A runner with a GPU, or a Firefox that accepts lavapipe, would
  close it.
- **Safari / WebKit's compiler is unchecked.** Nothing here shows a WebKit
  with WebGPU headless on Linux; a macOS runner with Safari 26 would.
- **Chromium 153 is not bisected.** The pacman cache holds no 153 package, so
  whether 153 accepted today's mesh shader (and which version began rejecting
  it) is unconfirmed.
- **Module creation is not pipeline creation.** A bind group layout that
  disagrees with a shader, an `override` given a bad value or a missing limit
  still needs a race. Creating each pipeline against its layout in the browser
  would need the engine's own pipeline descriptors, i.e. the web build itself;
  a headless race walk is the existing path (`scripts/web-screenshot.py`).
- **Tint stops at the first uniformity error per module.** A red run can hide
  a second; the three fixed here showed one at a time.
- **Default adapter draws white on this notebook.** Headless Chromium on the
  NVIDIA card gives blank screenshots from the front end on; the Intel iGPU
  draws (`VK_ICD_FILENAMES=.../intel_icd.json`). No page error; not
  investigated.

## Next Steps

1. Push the branch and read the first `web-shaders` run; fix the runner
   assumptions above (about 20 minutes, one or two pushes).
2. Add `check-web-shaders` to the CLAUDE.md command list (the maintainer's
   file): `just check-web-shaders # every WGSL module the web build emits, created by a real browser's compiler on a CPU adapter; skips loudly with no Chromium (scripts/check-web-shaders.py)`.
3. Decide whether a WebKit runner is worth it (a macOS job, Safari 26).
