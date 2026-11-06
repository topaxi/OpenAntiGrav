---
status: ready-to-wire
titles: [all]
updated: 2026-11-06
summary: "test-data: om 8,349 s summed before speed-plan, notebook 10,753 s after; single-ray broadphase left; lead's gate may drop its Intel ICD"
size: M (50)
difficulty: medium (50)
---

# `test-data` is throughput-bound, and GPU tests exhaust VRAM

The maintainer asked whether [nextest test groups](https://nexte.st/docs/configuration/test-groups/)
help with memory and VRAM pressure. They did for the out-of-memory failures (landed, below); the
suite has a second problem they do not touch.

## Landed

Lane A: `.config/nextest.toml` has a `gpu-vram` group of 4 threads over the 50 test modules that
spawn `oag-game` or call `capture_pixels_from`/`capture_from`; `just check-test-groups` (in the gate)
keeps that list exact. Same tree, full run on the 8 GiB NVIDIA GPU, no `VK_ICD_FILENAMES`: 981 s with
5 out-of-memory failures and a 7,782 MiB peak before; 959 s, all 10,909 passed, 4,005 MiB peak after
(load means 26.4 and 25.5). No RAM group was needed (35 GiB of 62 stayed free). Numbers, per-process
table and method: `docs/architecture/workspace-layout.md`, "Test groups".

## Open

- **The race load does not stop early on exit**: closing the window on the loading screen waits out
  the rest of the load (at most 0.5 s Pulse, 3.7 s 2048, 3.9 s HD in a debug build). A check of
  `gpu_workers::stopping()` at the load's stages would cut it. Only worth it if a player notices.
- **A window close sometimes does not land**: about one run in eight of a `WM_DELETE_WINDOW` sent to
  a window on Xvfb during a load was ignored for 30 s, with or without the fix. Unexplained; may be
  the harness (python-xlib, no window manager).
- **Lane B landed 2026-11-06** (numbers and per-family table in `docs/architecture/workspace-layout.md`, "Test-data suite measurements"): summed time 14,018 s to 12,321 s, wall 896 s to 787 s. The 9,000 s target is not met. What is left, with where to measure it:
  - **Speed-plan learning landed 2026-11-06** (`docs/architecture/workspace-layout.md`, "What the `speed-plan` pass measured"): suite 12,611 s to 10,753 s, `ai_clean_lap_gate` 447 to 239, `speed_plan` 280 to 142, bit-identical on 240 runs. Left on a gate row, measured with the recipe below on `ai_clean_lap_gate::track_07_flash`: `Indices::overlapping` under the single rays (`hover`, `maglock`, `clip`, `sweep`) 14 %, `PadVolume::contains` 5 %, `hull_probes` 5 %, `CageTrack::locate` 3 %.
  - ~~PFSC inflate and XTS~~ landed (lane suite-decoders, Ryzen 9 7900): `oag-disc` reads a 685 MB Omega archive out of the patch package at 365 to 552 MB/s (output hash identical), and `autopilot_original_ground_truth::omega_flies_the_original_autopilot` fell from 9.25 s to 6.86 s (three runs each, load 5). Almost no other test opens the `.pkg`; most read `data/extracted`. Open: `vpk` (2048) still inflates with `miniz_oxide`.
  - `.gnf` duplicate memoisation, measured and not built (lane suite-decoders): by texel count 14 % (data00), 18 % (data01), 27 % (data02), 10 % (data03), 18 % (data04), 2 % (data08) of an archive's `.gnf` are byte-duplicates, but only 6.5, 8, 17, 2, 7 and 1 % are duplicates within one `i % 3` slice. Slicing by content hash would reach the whole-archive figure (about 17 % weighted) at the cost of every slice reading and hashing every blob. A slice is 60 % BC7 (`bc7`, `read_indices`), so the cut is about 40 to 60 s of the 504 s, under 1 % of the suite. Reopen only if the BC7 block decode itself cannot get faster.
  - `omega_wem` (370 s) is `atrac9dec` at 15 ns per sample; `extract_ground_truth` (350 s) is the Omega load plus the flight.
  - Other sim-driving tests still tick drawn: `absorb`, `replay`, `projectile_tilt`, `hit_effects_inherit`, `launch_boost`, `zone`, `lock_sight`, `checkpoint_circuit` (they read effects or cues, so each needs a look).
  - **Profiling recipe.** `perf` is installed; user mode only (`perf_event_paranoid` 2). `--call-graph dwarf` unwinds nothing here; build with frame pointers into a separate target and pass `--user-callchains` or the event does not open: `RUSTFLAGS=-Cforce-frame-pointers=yes CARGO_TARGET_DIR=<scratch> CARGO_BUILD_BUILD_DIR=<scratch> RUSTC_WRAPPER= cargo test -p oag-raceplay --test it_a_g --no-run`, then from `crates/raceplay` with `LD_LIBRARY_PATH=$(rustc --print sysroot)/lib`: `perf record -e cpu-clock:u -F 999 --user-callchains --call-graph fp <bin> --ignored --exact <test>`. To compare two builds, `taskset -c 0-5 perf stat -e cpu_core/cycles/u,cpu_core/instructions/u` repeats to 0.5 % where task-clock wanders 10 %; a test-only `cargo test -p` binary and a `--workspace` one differ, so compare like with like.

## Next Steps

**Lane B, the rest** is the list above; 10,753 s is the suite after speed-plan learning, 1,750 s short of 9,000. Run on an idle machine, since the ceiling is judged there.
