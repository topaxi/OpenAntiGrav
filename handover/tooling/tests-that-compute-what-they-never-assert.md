---
status: ready-to-wire
titles: [all]
updated: 2026-11-22
summary: "Slow test-data tests that load art or run ticks their assertions never read; ribbon batch on HD, Omega, 2048 first"
categories: [tooling]
size: S (30)
difficulty: easy (30)
---

# Tests that compute what they never assert

Opened 2026-11-16 by the `scout-test-work` scout (maintainer's question: which
tests do work their assertion never reads). Method: the slowest tests of the
2026-11-13 full `test-data` gate (12,519 tests, the lead's log), the bodies of
about 40 distinct functions read in full, the rest from their headers and call
counts.

Times are the gate's wall seconds from 2026-11-13, not CPU. Savings are
**estimates** (confidence 20 to 40), not measured. Re-time a row before its lane
starts. Commit `6c39eed76` (test-tail-4) touched none of the files named in
rows 1 to 6, but later commits touched some of them.

Not repeated: the tests named as done (`hd_eliminator_hud_ground_truth`,
`pilot_assist_levels`, `omega_original_ai`, `vita_2048_original_ai`,
`hd_ai_craft_laws`, `spawn_heading` HD slice, Omega replay tick-zero, `speed_plan`).
Repeated disc setups belong to `disc-tests-that-repeat-one-setup.md`; tests that
could move down the dependency tree to `tests-that-could-move-down-the-dependency-tree.md`
(other scouts' threads, if they have landed). Not repeated from
`docs/architecture/workspace-layout.md`, "Measured and not done": the probe test
(`hd_env_probes_ground_truth`), the PS2 track export, and the Omega normal-map
census's parallel loop.

**Which titles the ribbon is neutral on.** Per the `test-tail-4` section, a
`Setup` built with `ribbon: true` differs from the disc load only in view fields
on HD, Omega and 2048. On Pulse PSP and PS2 it drops `finished_thrust.skill`
(4.0 becomes 2.0). Pure was never compared. Every ribbon row below names only
HD, Omega and 2048, and leaves Pulse and Pure on the disc load.

## Worth a lane now: one ribbon batch

All four rows need one `Setup` probe first: `{:?}` of each field with and without
the ribbon, on the titles named in each row, including Altima and Tech De Ra
on Omega and `views` 1 and 2 (the doc's compare did not cover these).

1. **`crates/raceplay/tests/omega_hud_ground_truth.rs::a_2048_era_circuit_runs_the_original_s_laps_per_class`** (24.7 s)
   - Unread work: four full Omega loads through `load_on` (line 601), with no
     `ribbon` set. The test reads only `Race::start(..).readout().laps`.
   - Change: `ribbon: true` in `load_on`.
   - Saving: estimate 10 to 15 CPU-s (four Omega loads, the heavy title).
   - Check: the four lap counts are unchanged, and the probe is clean on Altima
     and Tech De Ra.

2. **`crates/raceplay/tests/slipstream_ground_truth.rs::its_share_of_the_pad_draws_is_a_fraction_of_a_turbo_on_every_titles_real_table`** (20.7 s)
   - Unread work: five full loads (line 417). The test reads only
     `setup.weapons`, the authored pickup table.
   - Change: `ribbon: true` for HD, Omega and 2048. Pulse PSP EU and Pure stay on
     the disc load.
   - Saving: estimate 6 to 9 CPU-s (three of five loads, the heavy ones).
   - Check: `setup.weapons` equal with and without the ribbon on the three titles.

3. **`crates/game/tests/split_grid_ground_truth.rs::one_person_starts_last_on_every_title` and `two_people_start_on_the_last_two_rows_on_every_title`** (20.6 s and 20.4 s)
   - Unread work: each `grid_of` (line 24) loads the full art of every title and
     reads only ship positions after `Race::start`. HD and Omega are the heavy
     loads, and both are in the neutral set.
   - Change: `ribbon: true` for HD and Omega. Pulse PSP and PS2 keep the disc load;
     Pure waits for its measurement.
   - Saving: estimate 5 to 8 CPU-s over the two tests (four loads).
   - Check: `Setup` equal at `views` 1 and 2 on HD and Omega. The grid order arrays
     stay as written.

4. **`crates/raceplay/tests/launch_boost_ground_truth.rs::every_title_flies_a_launch_boost_and_names_its_source`** (27.6 s)
   - Unread work, two parts:
     - Six full loads through `authored` (line 262). The test reads only
       `setup.start_boost` and the `<StartBoost>` report lines.
     - The loop loads `pulse-psp-usa.chd` a second time (the first load is `psp`,
       before the loop). That repeat is a full Pulse load whose result is already
       in `psp`.
   - Change: reuse `psp` for the Pulse USA iteration (zero risk, no ribbon). Also
     `ribbon: true` for HD, Omega and 2048; Pulse PSP, Pulse PS2 and Pure keep the
     disc load.
   - Saving: the reuse is one Pulse PSP load, estimate 2 to 4 CPU-s. The ribbon
     part is estimate 6 to 9 CPU-s (three loads).
   - Check: the `<StartBoost>` report line is still written, and `start_boost`
     equal with and without the ribbon on the three titles.

## Worth a look, not in the batch

5. **`crates/game/tests/hillside_rain_frame_ground_truth.rs::the_final_lap_frame_is_greyer_darker_and_flareless_and_an_earlier_one_is_untouched`** (19.0 s)
   - Unread work: `frame` (line 107) runs one `oag-game` process per call. Lines
     160 and 165 call it at 6,000 and 9,000 ticks for each of two mods. The
     6,000-tick run is a prefix of the 9,000-tick run (same mod, deterministic),
     so 12,000 of the 30,000 drawn ticks repeat work.
   - Change: one process per mod that writes both screenshots. The
     `replay_viewer_ground_truth` test already takes several screenshots in one
     `--drive` script (`screenshot DIR/...`), so this may be test-side only. It is
     unverified whether a `--drive` script can reach the hillside track, which the
     test sets with `--track-file` and `--race`.
   - Saving: estimate 6 to 8 CPU-s (40 % of the drawn ticks).
   - Check: `early_on == early_off` and the three sky and flare assertions read the
     same bytes. Print both frame hashes before and after.

6. **`crates/mesh/tests/omega_normal_map_ground_truth.rs::omega_circuits_bind_the_normal_map_their_programs_decode`** (60.8 s), with `psp2_twin_blend_ground_truth` (30.2 s) and `omega_transparency_ground_truth` (27.9 s and 26.4 s)
   - Unread work: each `psp2::build` gets a closure that calls `read_path` on each
     texture it names (`every_circuit_binds`, `build`). No memo spans models, so a
     texture shared by many circuits is decompressed once per circuit. The census
     reads only the bound and unbound counts.
   - Not the same lever as `workspace-layout.md`'s "parallel loop", which concerns
     the decode and wall time. A read memo saves CPU but is still unmeasured.
   - Change: a test-side memo from texture path to bytes, shared across the models
     of one test.
   - **2026-11-22:** the cost was not the texture reads. Each model build re-scanned
     every material file's GCN programs once per planner (60,504 scans of 895 files);
     `gcn::memoised` now scans each once per build, and `oag-bytes` builds at opt-level 2:
     the normal-map census 243 to 48 CPU-s alone, `psp2_twin_blend` 109 to 37
     (`workspace-layout.md`, "The byte readers at opt-level 0"). The texture-read memo
     stays open, with smaller stakes.
   - Saving: low confidence (20). Only the read share is saved.
   - Check: first count reads against unique paths with a counter in a scratch
     copy. Build the memo only if repeats are more than half the calls. Run the
     count under `systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0`,
     since the memo can hold gigabytes across 25 Omega models.

7. **`crates/game/tests/drive_ground_truth.rs::any_input_in_a_demo_returns_to_the_main_menu`** (23.4 s), **later**
   - Unread work: four `--drive` runs, each waiting out the authored idle minute
     (`PULSE_IDLE_TICKS = 60 * 60`, line 872) before the demo.
   - Change: enter the demo with `--demo` for three of the four inputs. This is a
     claim change, not a cut: the exit path may read how the demo was reached.
     The `--demo` path also skips front-end state, and the assertion reads
     `page=main`.
   - Saving: estimate 8 to 12 CPU-s.
   - Check: a maintainer decision, then the same assertions.

## Checked, no cut (so nobody re-looks)

- **`omega_gnf_ground_truth` slices** (eleven tests, 24 to 32 s each, plus the
  patch and front-end tests): the base level is decoded twice (`decode` and
  `decode_bc7_level`), but `base == drawn` is the claim. The level-1 box filter is
  the second claim. Full-size decode of every entry is what `decoded` and
  `corrupt` count. No cut without weakening.
- **`omega_wem_ground_truth`** shards and the embedded census (19 to 36 s): `silent`
  and `decoded` count every ATRAC9 decode. No cut.
- **`trackfile_ground_truth`, the Pulse PS2 round trip** (53.1 s): the second
  `to_parts` (byte stability) and the art proof are asserted. Cross-ref the doc's
  "measured and not done" entry.
- **`mp4_movie_ground_truth::the_intro_cache_is_lossless`** (34.4 s): the 90-frame
  ffmpeg reference and the cache are the claim. The proof PNG at frame 60 is not
  asserted, but it is one frame, under a second, and the test documents it as the
  human artefact.
- **`ballisticng` `the_workshop_tracks`** (33.9 s): already one shared walk. The
  texture decode feeds a count assertion, and a split would parse 32 bundles twice.
- **`replay_viewer_ground_truth::a_recorded_race_is_watched_and_a_mismatch_is_shown`** (31.5 s):
  the viewer must reach the end of the pose track, which is asserted
  (`end[0] == end[1]`).
- **`pause_menu_ground_truth::hd_screens::an_eliminator_race_resumes_with_its_flip_intact`** (31.6 s):
  three game runs. The plain run is the hash reference at tick 460, and the flipped
  run is the claim. No shared prefix.
- **`mod_shader_frame_ground_truth::hd_shows_the_road_further_on_in_its_float_target`** (28.6 s):
  the claim is decided at tick 1,022, so there is nothing to stop early. The `off`
  run feeds two frames with different cameras, which one process cannot do.
- **`net` `extract_ground_truth`** (full grid, one lap, first circuit): already
  `tick_sim_only`. The art copy is the mechanism under test, so `ribbon` would test
  a different extraction.
- **`net` `a_lan_host_is_found_joined_started_and_raced_by_two_over_real_sockets`**
  (41.7 s): the 35 s idle sleep is the claim (QUIC keep-alive past its 30 s idle
  timeout). The 16.7 ms pacing sleep per tick looks removable, but the host runs on
  wall-clock confirmations; removing it risks the `matches > 200` assertion.
- **`hd_opponent_variant_ground_truth::every_opponent_flies_its_own_variants_handling_not_the_players`** (23.5 s):
  a hoist of the `own` map saves nothing. Each player faces opponents with its own
  family suffix (`""`, `_c1`, `_n1`), so the team keys do not overlap across the
  three players. The load already has `ribbon: true`.
- **`hd_bomb_blast_ground_truth` and `photo_mode_ground_truth`**: frames are the
  claim; no cut found on reading.
- **`gxt_ground_truth` census** (24.9 s): `rgba.len()` is asserted, so the decode
  is read. A header-only check would weaken it.

## How to measure a row

- Per test, user CPU, alone: `time target/debug/deps/<binary> --ignored --exact <name>`,
  with the load average recorded, twice, as the `test-tail-4` table does.
- For the ribbon rows, the `Setup` field compare above, then the test's printed
  values before and after.
- For the hillside and drive rows, count drawn ticks per run before and after, then
  time the test.
- Gate: `just test-data-affected` with the assertion unchanged. A change to
  `oag-game` (hillside, if it needs a CLI change) needs `just gate-affected`.

## Next Steps

1. Lane: rows 1 to 4 as one ribbon-and-reuse batch, after the `Setup` probe.
2. Check: whether the hillside test can run as a `--drive` script (row 5).
3. Count the mesh census reads (row 6) under the memory cap before any memo.
4. Maintainer: whether the drive test may enter its demo with `--demo` (row 7).
