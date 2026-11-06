---
status: ready-to-wire
titles: [all]
updated: 2026-11-06
summary: "test-data: om 8,349 s summed, notebook 12,321 s; speed-plan learning left; .gnf dedup not worth it; NVIDIA segfault in one 2048 test"
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

- **`campaign_cursor_ground_truth::the_2048_map_reopens_on_the_event_last_started` segfaults on
  NVIDIA, and it is not VRAM**: 3 of 8 runs alone on the idle GPU (787 MiB), 7 of 7 pass on the
  Intel ICD. `coredumpctl` shows `SIGSEGV` (`SEGV_ACCERR`) on the game's `race-load` thread at an
  instruction pointer outside every mapping, while the main thread is inside an `ioctl` in
  `libnvidia-eglcore`. A thread loading the race while the main thread talks to the driver is the
  suspect. Reproduce: `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all -E
  'test(the_2048_map_reopens_on_the_event_last_started)'` on the NVIDIA GPU, a few times.
- **The lead's gate can stop forcing the Intel ICD for memory's sake**: the group makes the NVIDIA
  run pass. It still hides this segfault, so drop `VK_ICD_FILENAMES` when that test is fixed or
  accepted as flaky.
- **Lane B landed 2026-11-06** (numbers and per-family table in `docs/architecture/workspace-layout.md`, "Test-data suite measurements"): summed time 14,018 s to 12,321 s, wall 896 s to 787 s. The 9,000 s target is not met. What is left, with where to measure it:
  - Speed-plan learning is about two thirds of a fork circuit's AI gate row (`ai_clean_lap_gate::track_07_*`: 1.2 to 1.7 s per `Race::start`, five starts per row). The perturbation reaches the plans, so they cannot be shared across the five runs; making `with_plan_course_on` cheaper is a bit-identical `oag-physics`/`oag-ai` change (collision `Aabb::overlaps`, the BVH walk, `nearest_reset`, `Line::walk` are the top frames). Measure: an in-process `SIGPROF` sampler (below) on `ai_clean_lap_gate::track_07_flash`, then `just check-determinism` and the committed hashes.
  - ~~PFSC inflate and XTS~~ landed (lane suite-decoders, Ryzen 9 7900): `oag-disc` reads a 685 MB Omega archive out of the patch package at 365 to 552 MB/s (output hash identical), and `autopilot_original_ground_truth::omega_flies_the_original_autopilot` fell from 9.25 s to 6.86 s (three runs each, load 5). Almost no other test opens the `.pkg`; most read `data/extracted`. Open: `vpk` (2048) still inflates with `miniz_oxide`.
  - `.gnf` duplicate memoisation, measured and not built (lane suite-decoders): by texel count 14 % (data00), 18 % (data01), 27 % (data02), 10 % (data03), 18 % (data04), 2 % (data08) of an archive's `.gnf` are byte-duplicates, but only 6.5, 8, 17, 2, 7 and 1 % are duplicates within one `i % 3` slice. Slicing by content hash would reach the whole-archive figure (about 17 % weighted) at the cost of every slice reading and hashing every blob. A slice is 60 % BC7 (`bc7`, `read_indices`), so the cut is about 40 to 60 s of the 504 s, under 1 % of the suite. Reopen only if the BC7 block decode itself cannot get faster.
  - `omega_wem` (370 s) is `atrac9dec` at 15 ns per sample; `extract_ground_truth` (350 s) is the Omega load plus the flight; `speed_plan_ground_truth` (250 s) learns plans by design.
  - Other sim-driving tests still tick drawn: `absorb`, `replay`, `projectile_tilt`, `hit_effects_inherit`, `launch_boost`, `zone`, `lock_sight`, `checkpoint_circuit` (they read effects or cues, so each needs a look).
  - **No profiler is installed** (`perf`, `samply`, `valgrind` absent; `perf_event_paranoid` 2 and `ptrace_scope` 1 rule out `samply` and an external `gdb -p`). What worked: an `ITIMER_PROF` handler calling `libc::backtrace`, dumped at exit and resolved with `addr2line`. It needs `unsafe` (the workspace forbids it), so it lived as an uncommitted test module with a local `allow(unsafe_code)`; the maintainer said the lead may argue for it. A `scripts/` tool or an `unsafe`-allowed dev-only crate would keep it.

## Next Steps

**Lane B, the rest** is the list above; take speed-plan learning first (largest, and it helps every race load). Run on an idle machine, since the ceiling is judged there.

**The NVIDIA segfault**: run the test under `gdb` with the core from `coredumpctl debug`; look at what
`race-load` runs at that moment (the load runs wgpu calls off the main thread?) and whether the
Intel and AMD drivers merely tolerate it.
