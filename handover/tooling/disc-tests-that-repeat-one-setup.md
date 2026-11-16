---
status: ready-to-wire
titles: [all]
updated: 2026-11-16
summary: "Disc tests repeating one setup: groups 1-4 merged (test-merge); music_resume, omega_event_xp, ps2_source, now_playing, psarc open"
size: S (30)
difficulty: medium (30)
---

# Disc-backed tests that repeat one setup

Question (maintainer, 2026-11-16): which `*ground_truth*` tests repeat the same
setup (an archive walk, a circuit load, a race run to the same tick) with only
the assertions changing? Nextest shares nothing between tests, so each pays
the setup again. The pattern to apply is
[`crates/rcs/tests/gxp_ground_truth.rs`](../../crates/rcs/tests/gxp_ground_truth.rs):
one pass, each claim a named check function.

**Every number here is a reading, not a measurement.** Times come from the
lead's `test-data.log` in `lead-gate` (load about 19). That log holds 928 PASS
lines of the 12,692 tests the run counted, so a test missing from it has no
time here. No build or run was made for this scout. Measure each group before
merging. Estimates carry confidence 30 on the rubric.

## Overlap with other threads

- [`test-suite-redundancy-next-targets.md`](test-suite-redundancy-next-targets.md)
  already lists the PSP2 model walks as its open item 1 ("the maintainer's
  call") and recommends this same `gxp` shape. Group 1 here is that item with
  the code read and the cost counted. It also names `magstrip_wire_hd` as its
  item 3, unverified. Group 2 here is that item with the code read.
- Its item 4 (the `oag-render` repeats) is not re-counted here.
- [`test-data-tail-is-now-decode-and-sim-sweeps.md`](test-data-tail-is-now-decode-and-sim-sweeps.md)
  owns decode and sim cost. `omega_transparency` and `omega_normal_map` are
  ruled out there ("share no work worth merging"); this scout agrees.
- The other scout's `tests-that-could-move-down-the-dependency-tree.md` is not
  in this worktree or on `main` at `6c39eed76`, so no overlap could be checked.
  If it lands and names any group here, drop that group from here.

## Ranked groups

### Done (test-merge, 2026-11-16), measured in `docs/architecture/workspace-layout.md` ("What the `test-merge` pass measured")

- **1. PSP2 model walks** (maintainer yes 2026-11-16): ten tests to three, 94 to 13 s user CPU.
- **2. `magstrip_wire_hd`**: the `is_none()` guard fixed in it and in `magstrip_wire_2048` (1.8 s each); `every_craft` folded into the hum flight; the three rumblers share one flight. Six tests to four, 24.7 to 16.8 s summed. Open: the 2048 file's tests share nothing (3 to 5 s each); the wall test and the autopilot flight stay separate on purpose (the autopilot changes the simulation).
- **3. `omega_held_loops`**: five tests to one, 14.6 to 3.0 s.
- **4. `split_hud`**: the six HD two-view tests to one over three loads, 15.5 to 6.9 s. Open: the two four-player tests (0.4 s each) still load separately.

The estimates above this section's first draft were 3 to 5 times too high for groups 2 and 3 (they read the log at load 19): a load is 1.8 s of CPU, and that, not the holds, was the cost. Re-measure the open groups below alone before merging them.

### 5. `music_resume_ground_truth.rs` (12 tests, 57 s summed): a smaller saving than first thought

- **Triads, corrected:** `pulse_psp` has `resumes`, `second_launch_decodes_nothing`
  and `run_restores_saves_and_quits`. `pulse_ps2` has `resumes` and
  `run_restores`. `hd` has `resumes` and `second_launch`. `pure`, `wipeout_2048`
  and `omega` have `resumes`. `omega_reads_its_music_bank_once_across_tracks`
  is a different claim.
- **What repeats:** the `MusicDiscs::survey` of the disc and the `open_audio`.
  The second launch needs its own first launch in its own cache: `scratch(label)`
  (line 98) gives each test a directory. So the decode is not shared, only the
  survey. Estimate **6-10 CPU-s** (reading; the survey's time is unmeasured).
- **Do not merge `run_restores_saves_and_quits` into the others.** It simulates
  25 s of racing (1,500 ticks, line 486) and is the group's long test.
- **Risk:** a shared cache would make the second launch's "decodes nothing"
  check pass for the wrong reason (the first launch warmed it). Keep the scratch
  per claim.

### 6. `omega_event_xp_ground_truth.rs` (2 tests, 43 s summed): a clean merge after all

- Both tests call `race::load_event(&options(&source), EVENT)`. The first
  (`an_omega_event_launch_carries_omegas_award_table`, about 19 s) then calls
  `oag_omega::open` and `xp_tables`, and asserts the table and column. The second
  (about 23 s) runs the race to its end. **Recipe:** load once; run the first
  test's asserts on `loaded.campaign_2048_event` before `Race::start(loaded.setup)`;
  then run the second test's race. The merged cost is about the second test
  plus the first's `oag_omega::open` and table read, not the sum of the two.
- **Saving:** one event load, estimate **5-19 CPU-s**. The share of the first
  test's 19 s that is the load is not measured.
- **Tail:** estimate 25-37 s (to measure), under 43 s if it holds. Merging would make
  a new slowest test; measure it.
- **Risk:** low. The first test's asserts read only the `progress` and the
  Omega table. Keep the table check before the race runs.

### 7. `ps2_source_ground_truth.rs` (10 tests, 50 s summed)

- `the_ps2_disc_loads_a_driveable_race` (line 137) and
  `a_ship_spawns_and_steps_on_the_ps2_disc` (line 184) each call `load(PS2_IMAGE)`.
  The second adds `Race::start` and 60 ticks. **Fold** the spawn and step
  claims after the static asserts, one `Loaded`.
- **Saving:** one PS2 race load. Estimate **3-6 CPU-s** (reading).
- `every_ps2_ship_s_texture_set_is_found_by_directory_position` (line 375) loads
  the PS2 race with `race::load` for its twelve teams. A different question, so
  keep it.

### 8. `now_playing_ground_truth.rs` (17 tests, 67 s summed)

- `a_no_audio_run_shows_the_song_it_would_play` (line 159) calls
  `timeline("data/images/pulse-psp-eu.chd", TimeTrial, "VENOM")`, the same call
  that `pulse_starts_the_song_and_its_display_together_on_go` (line 262) makes
  for its first mode. **Fold** the no-audio asserts into the Pulse start test's
  time-trial pass on the same beats.
- **Saving:** one Pulse time-trial timeline. Estimate **3-5 CPU-s**.
- The two Pulse races at lines 455 and 505 differ in `opponents`, so they are not
  one setup.

### 9. `psarc_script_ground_truth.rs` and `omega_psarc_ground_truth.rs` (partial)

- `the_script_reads_omegas_nine_archives_as_the_reader_does` (9.1 s in the log)
  opens the same Omega archives that `omega_psarc`'s per-archive checks walk
  (`open(name)` at line 51 of that file, `data00` alone at 8.2 s). Its
  reader side does `Archive::open` and a sample of paths. **Partial overlap:** the
  Python script's time is its own and cannot be shared, so only the reader-side
  open could fold into one `omega_psarc` check. Estimate **3-5 CPU-s**, low
  confidence (20).

### Checked, not a merge: already solved or a different setup

- **`hd_water_ground_truth.rs` (17 tests, 32 s) and `hd_lightmapped_albedo_ground_truth.rs`
  (10 tests, 15 s): open, not solved.** They read through `archive_cache`
  (`crates/mesh/tests/archive_cache/mod.rs`, line 67), a `thread_local!` cache.
  Nextest runs each test in its own process, so the cache only removes repeats
  inside one test. Each test still re-reads its archives, and the same Vineta
  and Sebenco `.vex` files are read by several tests. Each test is 1-2 s, so the
  saving is small per test. The circuit builds are the redundancy thread's
  item 4.
- **`trackfile_ground_truth.rs` (22 `race::load` calls):** each per-image test
  exports and reads back its own circuits, so the loads are not one setup
  across tests. `a_lone_craft_races_a_disc_circuit_and_its_exported_file_identically`
  (7.5 s) repeats the export of its first image, a small overlap. Not checked
  deeper.
- **`zone_ground_truth.rs` and `livery_ground_truth.rs`:** many loads, each for a
  different circuit or livery. Not checked deeper.
- **`demo_ground_truth.rs`:** `the_headless_demo_run_chains_demos_with_nobody_in_them`
  (line 148) runs the game binary as a process (`--demo --dry-run --ticks 700`).
  The Pulse demo tests run `chain::run` in process. Different path, different
  setup. Keep.
- **`spawn_heading_ground_truth.rs` (11 tests, 127 s):** already one load per
  circuit. The eight slice tests exist for the parallel tail (its doc comment,
  lines 312-340: 147 s on one core). Folding them makes one test the new
  slowest.
- **`split_grid_ground_truth.rs` (2 tests, 55 s):** `grid_of` is called with
  `views` 1 and 2, so the races differ.
- **`drive_ground_truth.rs` (16 tests, 161 s) and the other `--drive` families**
  (`campaign_cursor`, `selection_screens`, `pause_menu`): each test is its own
  `oag-game` process with its own script and flags, so the shared part (the
  boot, the front-end walk) cannot be shared by nextest. A merge would run every
  case in one process and lengthen the tail. A real saving needs a saved boot
  that every test restores: a design task, not a merge. Not measured here.
- **`omega_transparency` (the log's seven tests, about 43 s):** ruled out in
  `test-data-tail-is-now-decode-and-sim-sweeps.md` ("share no work worth merging").

## Merge recipe, every group

1. Measure first: `systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0 cargo nextest run -p <crate> --run-ignored all -E 'test(/^<module>::/)'` with `OAG_REQUIRE_GAME_DATA=1`. Time the target tests alone.
2. Each old assertion becomes a named check function over one load
   (`gxp_ground_truth.rs` is the shape). Keep every assertion message, so a
   failure still names its claim.
3. Compare the merged test's time with the old tests' sum and with the 43 s
   slowest test.
4. `just check-test-budget` and `just check-test-groups` after any change that adds a
   test module or touches a GPU-spawning test.

## Next Steps

1. Group 6: read `an_omega_event_launch_carries_omegas_award_table` against its
   merge partner and measure both alone first (the group 3 estimate was 5 times
   high). 30 minutes.
2. Groups 5, 7 and 8 are 3 to 10 s each by reading; measure alone and merge only
   what is over 3 s. An hour for the three.
