---
status: ready-to-wire
titles: [all]
updated: 2026-11-05
summary: "test-data is throughput-bound: cut the top families; open too: an intermittent NVIDIA segfault in one 2048 test (gpu-vram group landed)"
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
- 389 tests over 10 s hold 71 % of the time (2026-11-05). Summed test time is 14,394 s over 16
  threads, 900 s, against 925 s of wall clock; the slowest test is 124 s. The suite is
  throughput-bound, so priority cannot help and only less work can. Families by summed seconds:
  `omega_gnf` 830, `autopilot_original` 617, `omega_transparency` 591, `ai_clean_lap_gate` 541,
  `omega_wem` 423, `extract` 363, `vita_2048_original_ai` 347, `trackfile` 329.

## Next Steps

**Lane B, less work in the top families.** Follow CLAUDE.md's "Before adding a disc-backed test"
rules on what exists: one pass per archive, sniff before reading, stop at the tick a claim is
decided. Target: summed time under 9,000 s without dropping an assertion, each family's before and
after reported. Run on an idle machine, since the ceiling is judged there.

**The NVIDIA segfault**: run the test under `gdb` with the core from `coredumpctl debug`; look at what
`race-load` runs at that moment (the load runs wgpu calls off the main thread?) and whether the
Intel and AMD drivers merely tolerate it.
