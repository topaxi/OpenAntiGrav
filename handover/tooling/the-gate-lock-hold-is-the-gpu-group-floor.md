---
status: measure
titles: [all]
updated: 2026-11-24
summary: "The gpu-vram group is the gate hold floor; launch CPU is the lever (module -29 % CPU); race-build pipeline compile is next"
size: M (45)
difficulty: medium (40)
---

# The gate lock hold is the `gpu-vram` group's floor

`docs/architecture/gate-wall-time.md` moved the compile out of the gate lock
(`gate-affected` hold 152 s to 89 s) and left the other half: a member's
`test-data-affected` holds the lock for 559 to 598 s when it selects the game and
raceplay packages, and the CPU is idle for most of it (load 4 to 14 on 24
threads).

## Open

- **The group is the floor.** In a 2,068-test selection 321 tests are in the
  `gpu-vram` group, 1,788 s summed; at the cap of 4 that is 447 s of the 552 s
  wall clock. A replay of the run's durations through a 24 slot scheduler gives
  514 s as queued, 506 s with the group first and 719 s last, so nextest
  priority is worth at most 8 s.
- **Where the group's seconds are.** `campaign_cursor_ground_truth` 210 s (33
  tests, mean 6.4 s), `drive_ground_truth` 193 s (24, 8.0 s),
  `pause_menu_ground_truth` 186 s (26, 7.2 s), `net_rejoin_ground_truth` 113 s
  (3), `photo_mode_ground_truth` 103 s (14), `footer_hint_ground_truth` 83 s
  (14). A typical test is one `oag-game --drive` launch. Where those 6 to 8 s go
  inside one launch is not measured (boot to first frame is 0.3 s on Pulse PSP,
  `docs/architecture/load-time.md`, so it is the script's ticks or the
  launch's own load).
- **The lead's slices are close to it as well**: 181 to 210 s of wall clock per
  slice against 118 to 133 s of group floor.

## Done

- A launch profiled (`docs/architecture/load-time.md`, "A scripted launch"): 0.9 s
  alone for a menu test, 2.6 to 4.4 s with a race; the gate's 6.4 s mean is that
  launch's CPU under contention (2x the module's mean at the cap alone), so user
  CPU per launch is the number to move.
- `oag_disc::chd_source` keeps idle CHD handles (148 `Chd::open` per launch, each
  decoding the hunk map) and `oag_formats::psarc::md5` / `path_digest` stopped
  copying: a menu launch 21.05 G -> 9.4 G user instructions, the
  `campaign_cursor_ground_truth` run 243 s -> 171 s of user CPU, its wall clock
  37.3 s -> 32.4 s (median of six interleaved pairs).

## Next Steps

1. **`race-build` pipeline compile is 31 to 35 % of a racing launch**
   (`create_render_pipeline` -> naga `process_overrides` 20 %, SPIR-V write 10 %).
   The scene already shares modules and pipelines across drawables; what is left
   is naga's per-distinct-override-set cost. Measure with the frame-pointer build
   (`docs/architecture/load-time.md`, "Method") and
   `perf report --children --comm race-build`. Candidates: fewer distinct
   override sets, or a pipeline cache (wgpu `PIPELINE_CACHE`, the thread
   `dependency-upgrade-benefits-not-yet-applied.md`).
2. **One launch per MENU BACK choice**: `leaves_on_the_resolved_button` and
   `help_opens_on_the_displaced_button` in `campaign_cursor_ground_truth.rs`
   (14 tests) and the loops in `footer_hint_ground_truth.rs` launch the game
   three times per test, once per `controls.menu_back` value read from
   `settings.toml` at boot. `controls.menu_back` applies live, but the drive
   `set` step only turns a row of the page on screen, so one launch cannot switch
   the choice from inside a pause menu or a picker. A drive verb that applies a
   setting (`config controls.menu_back=circle`) would fold each test to one
   launch with every `expect` kept (28 launches fewer in that module). Tried: the
   `set` step fails with `no row "controls.menu_back"`.
3. **The other archives' identify work** (`oag_sound::identify`, `menu_source`'s
   job, `launcher::examine`) opens the 2048, HD and Omega archives again on every
   thread: 19 % of a menu launch is still `match_paths_to_entries` after the
   faster digest. Reading each archive's manifest once per process would remove
   most of it; check `oag_assets::psarc::Archive::from_source` callers first.
4. **PS2 music decode under `--no-audio`** is 36 % of a PS2 launch (LZMA hunks on
   `menu-music` and `race-music`). The null mixer keeps the song selection (the
   marquee), so skipping the decode changes what is drawn: a decision for the
   audio owner, not a quiet change.
5. A larger cap is the maintainer's call (8 GiB card, 0.5 to 1.05 GiB per
   game); measure the peak with the cap raised before proposing it.

Measure with `just test-data-affected` on a branch that touches `oag-render`
through `scripts/gate-run.sh` (`gate-times.tsv` holds the hold) and the
per-module sums from its log (`docs/architecture/gate-wall-time.md`).
