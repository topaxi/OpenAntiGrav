---
status: parked
titles: [all]
updated: 2026-11-26
summary: "Dependency benefits left for lanes: wasmtime 51, quinn 0.14 follow-ups, zip zstd-rs, cpal 0.19, wgpu to crates.io at 31, winit 0.31, HDR, mixer bench"
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
  surface setup (`crates/game/src/app/gpu.rs`) takes `Auto`, which is the old
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
  lane then. It also turns the startup panic on an X server without RandR into an
  error (`docs/rendering/dynamic-resolution.md`); it does not make one work.
- **wasmtime 51.0.0 is stable (2026-11-20) and ready to open as a lane.** Ours
  is the exact pin `=50.0.3` on purpose (determinism: fuel, NaN
  canonicalisation, ABI word). Gains: faster compile of host-defined functions
  ([#14517](https://github.com/bytecodealliance/wasmtime/pull/14517)) and the
  advisories in its notes (none reachable from `oag-modhost`). It does **not**
  remove the second `sha2`/`generic-array` copy: `cranelift-codegen` 0.138.0
  still requires `sha2 ^0.10.2`. `MemoryType::new` and `TableType::new` now
  return `Result`; `crates/modhost/src` calls neither. Measuring command: the
  extension ground truths and the mod hash tests of `oag-modhost`, plus
  `cargo tree -d`; the mod author skill needs a look only if `oag-ext-abi`
  moves. Size M (45), difficulty medium (50).
- **web-transport-quinn 0.14 follow-ups** (the bump itself landed with `rind`
  as the provider; `docs/architecture/dependency-upgrades.md`, ranked row 6).
  Unadopted: `with_system_roots` for a CA-signed public server,
  `with_congestion_control(LowLatency)` (BBR) and `with_transport_config` for
  the link, response headers on `reject`, and the `qlog` feature. Measuring
  command for the congestion choice: the `oag-net` loopback tests with a
  lossy path. Size S (60), difficulty easy (60).
- **zip's `zstd-rs` feature** (zip 9.8.0,
  [#1329](https://github.com/zip-rs/zip2/pull/1329); `zstd` 0.14.4's
  `rust-backend`) would let the web build read zstd mod entries and drop the C
  `zstd-sys` build, which `crates/mod/Cargo.toml` names as the reason zstd is
  native-only. Check first:
  `cargo check -p oag-mod --target wasm32-unknown-unknown` with the feature on.
  Size S (60), difficulty easy (60).
- **cpal 0.19 xruns** (`CallbackInfo::xrun()`, unreleased) replace the timed
  late-callback count in `crates/audio/src/output.rs`; 0.18.2 reports
  PipeWire, CoreAudio and AAudio xruns as `ErrorKind::Xrun` but 0.19 removes
  that kind again, so wait for 0.19. Size S (60), difficulty easy (60).
- **wgpu: back to crates.io at 31, then `UNRESTRICTED_BYTES_PER_ROW`.** The
  tree is on a git pin of trunk (`docs/architecture/dependency-upgrades.md`,
  "Git pins"), so the API breaks (`submit` returning an `Option`, the new
  descriptor fields, `has_srgb_suffix`) are already done. When wgpu 31 ships:
  `wgpu = "31"` and `naga` the same, re-run the six-frame picture check and one
  S21 round from that page. The pin already carries
  `Features::UNRESTRICTED_BYTES_PER_ROW` ([#10619](https://github.com/gfx-rs/wgpu/pull/10619)),
  which retires the 256-byte row padding in `crates/view/src/offscreen.rs`,
  `crates/mesh/src/capture.rs` and `crates/gpu/src/writes.rs`; not wired. Size
  S (50), difficulty easy (50).
- **A toml regression test.** `toml_parser` 1.1.6 fixes a stack overflow on
  invalid TOML ([toml-rs/toml#1265](https://github.com/toml-rs/toml/pull/1265));
  the lock has it. A test feeding a mod manifest thousands of unbalanced `[` and
  `]` would pin it for the mod loader (`crates/mod/src/manifest.rs`). Size XS
  (70), difficulty easy (80).
- **A `wgpu::PipelineCache` on disk** (Vulkan pipeline cache, feature
  `PIPELINE_CACHE`; `Device::create_pipeline_cache` is `unsafe`): measured on
  desktop RADV and not built. Mesa's own shader disk cache already takes a Pulse
  PSP race launch from 65.7 G to 24.0 G instructions on the second launch, and a
  pipeline cache adds about 1 % over that; naga's compile runs before it
  (`docs/architecture/load-time.md`, "Third round"). Open only for a driver with
  no disk cache of its own, unmeasured: the S24's Xclipse driver on Android.
  Measuring command:
  `perf stat -e instructions:u` around a race launch on that device with and
  without the driver's cache warm; under about 10 % keeps it unbuilt.

## Next Steps

1. When the maintainer next has a Deck on the desk, run the headphone mixer
   bench (S) and close that item.
2. Open the wasmtime 51 lane now, and the web-transport-quinn 0.14 follow-ups when a
   lossy-path measurement is wanted; open the winit
   0.31 lane when it is stable. The `x11rb` 0.13 copy goes with winit and
   `arboard` (`docs/architecture/dependency-upgrades.md`, "Held back" and
   "Benefits ranked").
3. Decide the HDR direction with the existing HDR target thread before anything
   in `gpu.rs` changes.
