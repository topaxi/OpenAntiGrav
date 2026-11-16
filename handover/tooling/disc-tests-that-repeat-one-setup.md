---
status: ready-to-wire
titles: [all]
updated: 2026-11-16
summary: "Disc tests repeating one walk, load or sim: PSP2 model walks 90-130 CPU-s (needs maintainer yes); magstrip, omega_held_loops, split_hud next"
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

### 1. The PSP2 model walks (needs the maintainer's yes, redundancy thread item 1): `psp2_rcsmodel_ground_truth.rs` (6 tests) and `psp2_rcsmodel_material_ground_truth.rs` (4 tests)

- **Setup:** each test walks every `.rcsmodel` of the three 2048 packages
  (`PACKAGES`, both files) and decodes each with `psp2::parse`. The first file's
  `survey(check)` (line 65) already takes a check closure, so its tests are
  written as one walk each. The second file's `every_model(check)` (line 42)
  is the same walk, copied.
- **Log (per test, all about 15-16 s):** `every_shipped_model_decodes`,
  `every_submesh_closes_on_its_own_counts`, `positions_decode...`,
  `normals_decode...`, `tangents_are_orthogonal...` (the first file), and
  `texcoords_decode...`, `every_material_resolves...`,
  `every_submesh_names_a_material...` (the second). About 128 s summed (about 81 s and 47 s).
  `a_circuit_is_world_space_and_a_craft_is_not` (line 247) walks the same set
  and was not in the log.
- **Saving:** one walk per file instead of one per test. About 8-9 extra walks
  at 15 s each. Estimate **90-130 CPU-s** (reading: a walk is most of each
  test's time, because the checks are arithmetic on the decoded values).
- **Recipe:**
  1. Nextest runs each `#[test]` in its own process, so a walk is saved only
     when one test calls `survey` once for all its claims. A `mod` shared by
     two test functions does not help. Put the second file's checks into
     `psp2_rcsmodel_ground_truth.rs` (one `PACKAGES`, one `survey`) and delete
     the copy in the material file.
  2. Make one `#[test]` per file that calls `survey` once with a closure that
     runs every claim, or one `#[test]` for both files. Each claim keeps its
     own `assert!` text. The count of walks falls from 10 to 1 or 2; a
     per-claim test would keep the walks and so would not save anything.
  3. Keep the census numbers (`Survey`) asserted in the one walk, as now.
- **Tail:** a merged walk is one 15 s pass plus the checks. Under the 43 s
  slowest test in the log.
- **Risk:** the simulation is not touched. Decode output is read, not changed.
  The risk is an assertion that depended on the test's own walk order (it
  asserts counts that do not depend on order, so check each one). Check that
  `a_circuit_is_world_space_and_a_craft_is_not` needs only the positions of
  the models it reads.

### 2. `magstrip_wire_hd_ground_truth.rs` (6 tests, 70 s summed)

Read the code this time. Four facts:

- **A double load in one test.** `every_craft_visits_a_strip...` (line 120)
  starts with `if load().is_none() { return; }` (line 121). That loads the full
  HD race just to skip. Then `fly(false, None)` (line 124) calls `load()` again
  (line 68). `magstrip_wire_2048_ground_truth.rs` line 135 has the same guard
  over a 2048 load, with `fly` loading again. A `rg` for any loader called as a
  `.is_none()` guard finds only these two (the `trackstartup` `image()` hits are
  path probes, not loads). **Fix:** probe the image path
  (`oag_testdata::image(...)`) instead. One-line change in each file, saving one
  race load per test.
- **`every_craft` could fold into the hum test's `render(false)` flight.** The
  hum test's `render` closure (near line 178) calls `fly(drop_edges, Some(&mut audio))`,
  with `drop_edges` false and then true. `fly` builds `log.starts`/`stops` from
  `race.pending_cues()` before any drain, on both the audio and no-audio paths
  (lines 75-84). `race_tick_keeping` (`crates/game/src/sound.rs`, line 188) drops
  cues only from the audio input (`keep`), and `note_song` writes the readout,
  not the sim. So the `Log` should match. **Not confirmed**: the redundancy
  thread's step, one run that prints both logs and compares them, is still the
  proof. If they match, fold the strip-visit asserts into the hum test's
  `render(false)` log and delete `every_craft`. Saves one load and one
  2,400-tick flight, about **11 s** (its time in the log).
- **The hum test also loads the race a third time** (line 153, `load()` only to
  read the voices). Fold that voice check into the `render` closure's first
  race if `fly` can hand it back, or accept it as one load.
- **The rumble runs share one simulation.** `rumble_run` (line 252) calls
  `race.tick` every tick, whatever the token. `paused_from` only passes a flag
  to `Rumbler::tick` (line 288), so the sim keeps running. `Rumbler::tick` takes
  `race: &Race` (`crates/game/src/rumble.rs`, line 224), so it cannot write to
  the race. The `none` run (line 328), the `ps3 paused` run (line 333) and the
  pad run (line 303) can share one 2,400-tick race with one `Rumbler` each. The
  autopilot run (line 353) sets `autopilot_timer` before its loop, so it changes
  the sim and stays separate. Estimate **15-25 CPU-s** (reading: two flights
  and two loads saved).
- **The wall test** (`flown_into_the_wall`, line 370) loads the same HD race
  without the magstrip assert. Its race can join the shared one only if its
  stick input is off for the others. Not checked; leave it.
- **Risk:** a `Rumbler` that feeds back through something the race reads (not
  found, but the `Rumbler` holds a `Pad` and a `Library` only). Compare the pad
  logs before and after on the same seed. No arithmetic change in the sim.

### 3. `omega_held_loops_ground_truth.rs` (5 tests, 72 s summed)

- **Setup:** `omega()` (line 29) loads the Omega single race off
  `data/extracted/ps4`, starts it, opens audio, runs 60 race ticks, 300 frozen
  audio steps and 30 more to settle the pool. Each of the five tests pays it
  before `holds()` (line 86) runs 900 + 60 + 1 audio steps. Per test the log
  reads about 14.2-14.9 s.
- **Saving:** four of the five setups. Estimate **25-35 CPU-s** (reading:
  the setup is most of each test; the holds are a few hundred steps).
- **Recipe:** one `#[test]` that calls `omega()` once and runs the five cases in
  order. Re-read `idle` before each `open`. Keep each case's `name` in its
  messages. The `close` closures already reset their field.
- **Tail:** about 14 s + 5 x 2 s, under 43 s.
- **Risk:** the cases share one race and one `Audio`. A leftover (the leach beam
  not cleared, a voice still sounding) would fail a later case for an earlier
  case's reason. Keep the per-case `idle` re-read. No sim change.

### 4. `split_hud_ground_truth.rs` (13 tests, 63 s summed)

- **`split_race(4)`** (line 33) is called by `every_view_draws_its_own_slots...`
  (line 101) and `a_later_views_reticle_follows_its_own_slots_lock` (line 177).
  Same four-player race, one load each. **Fold** the two: the draw checks, then
  the reticle check on the same `race`. Saves one load.
- **`hd_loaded(2)`** (line 200) is loaded in six tests: `hd_draws_its_own_authored_split_layouts`
  (line 219, with `hd_loaded(1)`), `hd_draws_a_reticle_in_its_authored_split_view`
  (line 422) and `..._vertical_split_view` (line 431), both through
  `hd_reticle_in_split_view` (line 436), `hd_authors_the_bar_both_places...`
  (line 934) and `hd_a_two_h_two_v_race_draws_each_window...` (line 959).
  `hd_draws_its_own_vert_layouts...` (line 304) loads `hd_loaded(2)` and
  `hd_loaded(3)`. Per two-view HD load, one shared load serves the
  horizontal-cut reticle, the bar, and the two-by-two checks. The vertical
  reticle needs its own cut on the same load (its `cut` argument), so it can
  join the shared test too.
- **Saving:** five HD two-view loads, one four-player load. Estimate
  **12-18 CPU-s** (reading: six calls become one load, five saved).
- **Tail:** the merged HD test runs several check functions on one load. Its
  time is the load plus those draws; measure it against 43 s.
- **Risk:** the reticle check mutates the race's lock state. It must run last
  on its `Loaded`, or start a new `Race` from a clone of `setup`. Check whether
  `Setup` clones before merging.

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

1. Group 1: measure one `survey` walk on `psp2_rcsmodel_ground_truth` alone
   (`time` the test, 2048 disc present). This sets the real saving. 15 minutes.
2. Group 2, the one-line guard: change `if load().is_none()` at
   `magstrip_wire_hd_ground_truth.rs:121` to a probe of the image. 5 minutes.
3. Group 6: read `an_omega_event_launch_carries_omegas_award_table` against its
   merge partner. Already read here: go straight to the merge. 30 minutes.
