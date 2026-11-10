---
status: parked
titles: [all]
updated: 2026-11-09
summary: "Dependency benefits too big for the upgrade lane: wgpu HDR surface colour space, winit 0.31 and wasmtime 51 once stable, a headphone mixer bench rerun"
categories: [rendering, audio]
size: L (40)
difficulty: medium (40)
---

# Dependency upgrade benefits not yet applied

The upgrade lane ([`docs/architecture/dependency-upgrades.md`](../../docs/architecture/dependency-upgrades.md))
took every dependency to its newest release or named why not, read the
changelog of each, and applied what was small: PS3 sector decryption as one
batch (aes 0.9), the HRTF block at 513 frames (hrtf 0.10). What follows is what
was too big or is waiting on an upstream release. Impact is what a player or a
build would gain; size and confidence follow `HANDOVER.md`'s scale. None of
the numbers below is measured unless it says so.

## Open

- **wgpu 30's surface colour space** (`SurfaceConfiguration::color_space`,
  `SurfaceColorSpace::{ExtendedSrgbLinear, Bt2100Pq, DisplayP3, ...}`,
  `Surface::display_hdr_info()` for luminance and headroom). `oag-game`'s
  surface setup (`crates/game/src/main/gpu.rs`) takes `Auto`, which is the old
  behaviour. A real HDR or wide-gamut output would follow the HDR target and
  tone-map work that is already a thread
  (`handover/rendering/hdr-target-and-tone-map-per-title.md`). Impact: HDR
  displays only (none of the maintainer's test displays is known to be HDR),
  moderate. Size L (40), difficulty medium (50). Vulkan's extended colour
  spaces depend on the driver, so a RADV and a Deck check come first.
- **Headphone mixer cost, rerun.** hrtf 0.10 changed the convolver (one
  crossfaded block, per-ear tails merged) and `binaural::BLOCK` is now 513
  frames. One voice costs 1.29 ms of CPU per second of audio at 513, against
  3.35 ms at 512 and 3.04 ms at 256 (measured, release-shaped test build on the
  workstation, not through the mixer). The 128-voice and Deck figures in
  `docs/ghidra/functions/psp-pulse-usa/positional-audio.md` (the mixer table and
  "Cost on a Steam Deck") predate it. Re-run the same bench on the Deck and put the
  new table beside the old. Impact: confirms (or refutes) a headphones-mode CPU
  saving of about 2x on weak CPUs; the Deck and S24 are the machines it matters
  on. Size S (70), difficulty easy (80), needs the Deck (`blocked-hardware` when
  it is offered).
- **hrtf 0.10's per-block gain** (`HrtfContext::new_gain` / `prev_gain`,
  interpolated across the block with the direction) is passed as 1.0 in
  `binaural.rs`; the mixer applies voice gain afterwards. Moving distance gain
  into it would remove any step at block edges when a placed voice moves
  quickly. Impact: small and unmeasured (an audible zipper has not been
  reported). Size XS (60), difficulty easy (70). Only worth doing if a listening
  check hears a step.
- **winit 0.31** is at `0.31.0-beta.4` (split into `winit-x11`, `winit-wayland`
  and the like). Moving to it may drop the second copies 0.30 forces: `x11rb`
  0.13 (ours is held there for it) and `tiny-skia` 0.11 through
  `sctk-adwaita`; the beta's own dependency lists were not read. Impact: a
  smaller and faster build and one `x11rb`; no player-visible change expected.
  Size XL (30), difficulty hard (40): the event-loop and window API changed.
  Wait for the stable release; reopen as an `oag-game` and `oag-view` migration
  lane then.
- **wasmtime 51** is `51.0.0-rc.1`; ours is the exact pin `=50.0.1` on purpose
  (determinism: fuel, NaN canonicalisation, ABI word). Its `cranelift-codegen`
  decides whether `sha2` 0.10 (hence `generic-array` 0.14.7 and
  `crypto-common` 0.1, a second copy beside aes 0.9's `crypto-common` 0.2)
  leaves the tree. Re-check `cargo tree -d` when 51 is stable; the mod author
  skill needs a look only if `oag-ext-abi` moves. Size M (45), difficulty
  medium (50).
- **A `wgpu::PipelineCache` on disk** (Vulkan pipeline cache, feature
  `PIPELINE_CACHE`) would cut first-launch pipeline compiles on the Deck and
  Android. It is **not applicable as it stands**: `Device::create_pipeline_cache`
  is `unsafe` (the cache bytes are trusted), which the maintainer's rule
  against wgpu's unsafe escape hatches and the workspace's
  `unsafe_code = "deny"` both refuse. Listed so nobody re-derives it; it needs
  a maintainer decision (`blocked-maintainer`) before any size is worth
  estimating. Desktop race build is already 195-372 ms
  (`docs/architecture/race-load-transition.md`), so the gain would be on
  slower GPUs.

## Next Steps

1. When the maintainer next has a Deck on the desk, run the headphone mixer
   bench (S) and close that item.
2. When winit 0.31 and wasmtime 51 reach stable, open the two migration lanes
   above and drop `num-bigint` and `x11rb` holds where the second copy goes
   away (`docs/architecture/dependency-upgrades.md`, "Held back").
3. Decide the HDR direction with the existing HDR target thread before anything
   in `gpu.rs` changes.
