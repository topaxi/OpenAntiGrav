---
status: measure
titles: [all]
updated: 2026-11-24
summary: "Design 4 landed (vendored wgpu-hal stage cache, upstream #7729 shape, launch -21 %); left: device-lost handler, upstream PR, GPU wall clock"
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

- **Landed: design 4**, a per-device SPIR-V cache (shaped like upstream's D3D12 cache, gfx-rs/wgpu#7729) in a vendored `wgpu-hal`'s Vulkan
  `compile_stage` (`docs/architecture/vendored-wgpu-hal.md`; `load-time.md`, "Seventh
  round"): race walk -14.7 %, bare race launch -21.2 %, `bomb_orbit_ground_truth` -37.4 %
  user instructions, frames pixel-identical. Designs 1 to 3 are no longer needed for the
  launch; design 2 (a disk cache of the words) is the only one that would still take the
  remaining 11 % of `compile_stage` (first writes), and it breaks the build-time invariant.
- **Upstream PR not opened.** Its text is in `vendored-wgpu-hal.md`, "Upstream pull request
  text"; sending it is the maintainer's call. Until a `wgpu` release carries it, every `wgpu`
  upgrade re-vendors (`vendored-wgpu-hal.md`, "When to drop it").
- **Player's load as wall clock on a presenting GPU is unmeasured.** The only wall clock
  taken was lavapipe on Xvfb under load 48 to 63 (medians 10.4 s -> 6.8 s, best 6.40 ->
  6.23 s); measure with `--measure-race-load 2` on the desktop or the Deck.
- **No native device-lost handler.** Only the web build registers
  `set_device_lost_callback` and `on_uncaptured_error` (`oag-game`'s `app/gpu.rs`,
  `mod web`), and the renderer setting cannot leave Vulkan on Linux, Android or the
  Deck. What a player sees when a native device is lost is not measured. This is a player-safety gap with or
  without passthrough; the brief's recovery step (kill switch, crash-loop marker,
  device-lost callback) was not built because its trigger, passthrough, was not.

## Next Steps

- Maintainer: send the upstream PR (text in `vendored-wgpu-hal.md`) and the naga
  `process_overrides` overrides-arena report, or decide not to.
- Measure the player's load on a presenting GPU: release build,
  `oag-game data/images/pulse-psp-eu.chd --no-audio --measure-race-load 2`, base
  (`[patch.crates-io]` line for `wgpu-hal` removed) against the cache, interleaved.
- Independently, a lane for the native device-lost callback: log at `error`,
  tell the player to relaunch, keep the process from dying silently.
