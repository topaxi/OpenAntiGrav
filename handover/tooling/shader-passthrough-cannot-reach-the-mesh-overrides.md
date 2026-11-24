---
status: blocked-maintainer
titles: [all]
updated: 2026-11-24
summary: "Build-time SPIR-V reaches 1.6 %; resolve-once measured 6.2 % (safe, unlanded); a wgpu-hal stage cache (about 19 %) needs a decision"
size: L (40)
difficulty: hard (45)
---

# Shader passthrough cannot reach the mesh overrides

The approved design (SPIR-V written by naga at build time, handed to
`Device::create_shader_module_passthrough` on Vulkan) was measured and not built:
`docs/architecture/load-time.md`, "Fifth round". naga is about 31 % of a Pulse PSP
race launch's instructions, 29 of it the mesh pipelines, whose override constants
are run-time data (model textures, disc numbers, the player's anisotropy), and
naga 30's SPIR-V writer cannot emit specialization constants. Build-time blobs
reach 1.6 %.

## Open

- **Decision: is a run-time SPIR-V path acceptable?** Two designs, both outside
  the approved invariant ("the blob is naga's build-time output"):
  1. In-process memo: naga writes each distinct (module, entry point, constants,
     layout) stage once, later pipelines reuse it through passthrough. The race
     compiles 202 stages, 64 distinct; ceiling about 17 % of the launch on every
     launch.
  2. A disk cache of those blobs keyed on naga's version, shader text, entry
     point, constants, layout and the device's writer options. Ceiling about
     31 % on a warm launch.
  3. Measured, safe, not landed (`load-time.md`, "Sixth round"): resolve overrides once
     per distinct stage and hand `wgpu` the IR (`ShaderSource::Naga`, `naga-ir` on
     `oag-mesh`, no `unsafe`). Race walk 22.01 -> 20.63 G user instructions (-6.2 %),
     bare race launch 12.03 -> 11.00 G (-8.6 %), the one frame compared byte-identical.
     Under the 10 % bar, so it stayed a prototype (about 150 lines in `oag-mesh`'s
     `pipeline_cache`). It needs `module.overrides` cleared by hand after naga's
     `process_overrides`, which leaves it filled against its own documentation.
  4. A stage cache inside `wgpu-hal`'s `compile_stage` (vendored or upstream), keyed on
     module, entry point, constants and binding map: 44 compiles instead of 202, about
     19 % of a race launch, SPIR-V write included. Not built; vendoring `wgpu` needs the
     lead's word.
  Designs 1 and 2 must mirror `wgpu-hal`'s private, device-derived writer options and its
  per-layout binding renumbering, and must never pass a module with overrides
  through (constants are dropped silently). The traps, with `path:line`, are in
  the "Fifth round" section.
- **No native device-lost handler.** Only the web build registers
  `set_device_lost_callback` and `on_uncaptured_error` (`oag-game`'s `app/gpu.rs`,
  `mod web`), and the renderer setting cannot leave Vulkan on Linux, Android or the
  Deck. What a player sees when a native device is lost is not measured. This is a player-safety gap with or
  without passthrough; the brief's recovery step (kill switch, crash-loop marker,
  device-lost callback) was not built because its trigger, passthrough, was not.

## Next Steps

- Report upstream that naga's `process_overrides` writes the overrides arena back, against its
  doc comment ("empty `overrides` arena"); design 3 would then stop clearing it by hand.
- The prototype of design 3 is on branch `gate-walltime-7-design3` (not for merge: no wasm
  gate, no tests).
- Decide between landing design 3 as measured (6 to 9 %, safe, needs its tests and the
  HD and menu frame comparisons), the `wgpu-hal` stage cache (design 4), and designs 1 or 2.
- Ask the maintainer whether design 1 or 2 may go ahead (Opus, with the recovery
  step of the original brief as a precondition).
- Independently, a lane for the native device-lost callback: log at `error`,
  tell the player to relaunch, keep the process from dying silently.
- Measuring command for either design: the Fifth round's `perf stat -e
  instructions:u` on the Pulse PSP `--drive` walk, warm `MESA_SHADER_CACHE_DIR`,
  before and after, plus a stage census at `pipeline_cache::cached_pipeline`'s
  miss.
