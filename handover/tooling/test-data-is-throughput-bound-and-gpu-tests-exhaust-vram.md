---
status: ready-to-wire
titles: [all]
updated: 2026-11-05
summary: "test-data: GPU tests exhaust the notebook's 8 GiB VRAM, so nextest test groups; and the suite is now throughput-bound, so cut the top families"
size: M (50)
difficulty: medium (50)
---

# `test-data` is throughput-bound, and GPU tests exhaust VRAM

The maintainer asked whether [nextest test groups](https://nexte.st/docs/configuration/test-groups/)
help with memory and VRAM pressure. They do, for one failure seen, and the suite has a second
problem they do not touch.

## Open

Read off one full run (2026-11-05, 10,889 tests, Intel ICD, load from two members alongside):

- **VRAM.** A member's `test-data-affected` on the NVIDIA GPU (8 GiB) failed `hd_weapon_scene_ground_truth`
  (three tests), `hd_leach_ball_ground_truth` and `hd_plasma_ball_ground_truth` with Out of Memory;
  all pass when run alone. `campaign_cursor_ground_truth::the_2048_map_reopens_on_the_event_last_started`
  segfaults on NVIDIA and passes on Intel, also on `main`. That one being VRAM is a guess.
- **The lead's gate hides this**: it sets `VK_ICD_FILENAMES` to Intel's, whose memory is system RAM.
  Members do not, so they see failures the lead's gate never does.
- **RAM is unmeasured.** No RAM failure has been recorded here; nothing says which tests are heavy.
- **`.config/nextest.toml`'s header is out of date.** It calls the suite tail-bound. Summed test
  time is 14,394 s; over 16 threads that is 900 s, against 925 s of wall clock. The slowest test
  is 124 s. The suite is throughput-bound now, so priority cannot help and only less work can.
- 389 tests over 10 s hold 71 % of the time. Families by summed seconds: `omega_gnf` 830,
  `autopilot_original` 617, `omega_transparency` 591, `ai_clean_lap_gate` 541, `omega_wem` 423,
  `extract` 363, `vita_2048_original_ai` 347, `trackfile` 329.

Limits of a test group: it caps concurrency inside one nextest run. It does not see a game on the
desktop or a second run; the `gate.lock` covers the second. Capping GPU tests costs wall clock in a
throughput-bound suite, so its value has to be set from a measurement, not picked.

## Next Steps

**Lane A, test groups (a measure, then a config change).**

1. Measure each test's peak RSS and GPU memory on one full run: a nextest wrapper script around
   `/usr/bin/time -v` for RSS, `nvidia-smi` sampling for VRAM. Scratch only; not shipped in the config.
2. List the tests that open a GPU. About 125 test files name an adapter or a device, 40 spawn
   `oag-game`; these sit inside shared `it` binaries, so filter by test module, not binary.
3. Add `[test-groups]` with a `gpu` group, `max-threads` set from step 1, and put the top RAM tests
   on `threads-required` or a second group if step 1 shows they need it.
4. Add a check that fails when a test opens a GPU and is not in the group, or the filter rots.
5. Prove it: the six failures above pass in a loaded NVIDIA run, and report wall clock before and
   after. Correct the config's header. Then decide whether the lead's Intel ICD stays.

**Lane B, less work in the top families.** Follow CLAUDE.md's "Before adding a disc-backed test"
rules on what exists: one pass per archive, sniff before reading, stop at the tick a claim is
decided. Target: summed time under 9,000 s without dropping an assertion, each family's before and
after reported. Run after lane A, on an idle machine, since the ceiling is judged there.
