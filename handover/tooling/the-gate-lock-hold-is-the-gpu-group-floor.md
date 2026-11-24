---
status: measure
titles: [all]
updated: 2026-11-24
summary: "The gpu-vram group is the gate hold floor; launch CPU is the lever (modules -29 %, -35 %); race-build pipelines and more folds are next"
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
- A PSARC archive's parsed manifest is shared by every open of the same bytes
  (`oag_assets::psarc::parsed`): a menu launch 4.85 G -> 3.24 G user instructions,
  a race launch 20.5 G -> 18.9 G (`docs/architecture/load-time.md`, "Third round").
- The drive's `config <setting>=<value>` step switches `controls.menu_back` from any
  screen; `campaign_cursor_ground_truth` (14 tests) and four `footer_hint_ground_truth`
  tests launch once for the three choices: the modules' user instructions -35 % and
  -19 %.
- Measured and not changed: a disk pipeline cache adds about 1 % over mesa's own shader cache (`docs/architecture/load-time.md`, "Third round"), one shader
  module per stage gains 1.5 to 2 % of a race launch.

## Next Steps

1. **The race build's pipelines are 42 to 45 % of a racing launch**
   (158 distinct for Pulse PSP, `docs/architecture/load-time.md`, "Third round").
   No safe disk cache exists. What is left: do not build the unblended pair
   where no transparent draw of `BlendClass::None` selects it (32 of the 158, about
   7 % of a racing launch; a placeholder pair needs a rule for draws added after
   the build), or one fragment module per entry point (not measured). Measure
   with `perf record -e cpu-clock:u --call-graph fp` on the frame-pointer build
   and `perf report --children --comm race-build`.
2. **More MENU BACK launches to fold with `config`**: the cell-selection and
   photo-mode tests of `footer_hint_ground_truth` and the photo/viewer tests of
   `campaign_cursor_ground_truth` keep a launch per choice (a campaign cursor is
   what their script reads; a photo test ends mid-race). Any other module that
   loops a launch over one setting is the same shape: `rg 'for choice in'
   crates/game/tests`.
3. **The other archives' identify work** (`oag_sound::identify`, `menu_source`'s
   job, `launcher::examine`) still opens each archive's table once per process
   (45 archives, `match_paths_to_entries` 12 % of a menu launch after the
   shared parse) and the CHD handle of every disc in `data/images/`
   (`Chd::open`'s map CRC 11 %, the disc identify's LZMA hunks 16 %). A cache of
   the survey on disk, keyed by file identity, would remove them from every launch;
   it needs an invalidation story.
4. **PS2 music decode under `--no-audio`** is 36 % of a PS2 launch (the CHD's LZMA
   hunks of `PS2MUSIC.WAD`, on `menu-music` and `race-music`). The null mixer keeps
   the song selection (the marquee) and advances a song by the `Sound`'s length, so
   skipping the read changes what is drawn unless a length-only `Sound` comes from
   the WAD's index: a decision for the audio owner, not a quiet change.
5. A larger cap is the maintainer's call (8 GiB card, 0.5 to 1.05 GiB per
   game); measure the peak with the cap raised before proposing it.

Measure with `just test-data-affected` on a branch that touches `oag-render`
through `scripts/gate-run.sh` (`gate-times.tsv` holds the hold) and the
per-module sums from its log (`docs/architecture/gate-wall-time.md`).
