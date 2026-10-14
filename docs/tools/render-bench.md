# Whole-race rendering benchmark

One fixed race per title, drawn frame by frame with no window, every frame
measured. [`OAG_RENDER_BENCH`/`OAG_RENDER_GPU_BENCH`](../rendering/frame-audit.md)
re-draw **one** captured tick; this plays a whole race, so a corner that is
slow on lap 2 shows up where one frame never would.

```sh
just bench-render                      # every title that is on disk, release build
just bench-render pulse hd --repeat    # two titles, each run twice, tick count and state hash compared
just bench-render pulse --views 1,2h,2v --size 1280x800   # one view, then a split cut both ways
just bench-render pulse --tag before   # results in data/bench/before/
# ... change something, rebuild ...
just bench-render pulse --tag after
just bench-compare data/bench/before data/bench/after     # or two .json files
```

## Take the gate lock (maintainer's rule, 2026-10-14)

A bench is only worth reading on a quiet machine, so it takes **the same
lockfile as the test suites**, `$HOME/.cache/oag/gate.lock`, for the runs. The
recipe does it itself (`scripts/bench-render.py`, an exclusive `flock` like
`flock -o`), builds the release binary *outside* the lock so a compile never
holds a gate, and says `waiting for the gate lock` when a gate has it: that is
not a hang.

- **Do not wrap `just bench-render` in `flock` yourself**, and never run it from
  inside a gate or a gate inside it: the same lock twice is a deadlock.
- `OAG_BENCH_NO_LOCK=1 just bench-render` skips the lock, for a human on an idle
  machine. Members never use it.
- Whatever the lock does, every result records the load average at its start and
  end (`/proc/loadavg`, 1, 5 and 15 minute). Read it before trusting a number: a
  run at load 20 and a run at load 2 are not the same machine.

## The view cases (split screen, 2026-10-14)

`--views` takes a comma-separated list of cases, `1` by default:

| Case | Flags it adds | Result file |
| --- | --- | --- |
| `1` | none: one view | `<title>.json` / `.csv` (as before the cases existed) |
| `2h` | `--players 2 --split-cut horizontal` (one view above the other) | `<title>-2h.json` |
| `2v` | `--players 2 --split-cut vertical` (side by side) | `<title>-2v.json` |

`just bench-compare A B` lines two directories up by file name, so `pulse-2h.json`
meets `pulse-2h.json`; `run.case` in the result names the case and a compare of two
different cases warns. `--size WxH` sets the offscreen target for every case (default
1280x720; the Steam Deck's panel is 1280x800). Both people fly on `--autopilot`
(`Race::autopilot_flies` covers every person's slot) and the race ends when the last of them
has crossed the line.

What a split frame records differs from one view in four ways, all in
`race_bench.rs`:

- The scene is drawn by `Scene::render_viewports`, which records **and submits** its own
  command buffers: `record_us` is the whole call, `submit_us` is empty and `cpu_us` is
  `record_us`. The frames-in-flight window waits on an empty submission queued after
  the call, so it still holds two frames outstanding.
- `gpu_view<N>_us` is the GPU timestamp pair around view N's scene pass, in the CSV
  (after `triangles`) and as `metrics.gpu_view<N>_us` in the summary. `gpu_scene_us` and
  `gpu_us` are their **sum**. Views overlap on the GPU, so the sum is work, not elapsed time;
  `frame_us` is the number a frame budget is read against.
- A split draws no motion blur and no HD bloom chain (rule 3 in
  [split screen](../rendering/split-screen.md)), so `gpu_blur_us` and `gpu_bloom_us` stay
  empty.
- `settings` gains `views` and `split_cut`, so a split result never compares clean against a
  one-view one.

The simulation is the same race whichever way the cut goes: `2h` and `2v` end on one
`final_state_hash` (`render_bench_ground_truth::a_split_race_is_benched_per_view_...`),
and differ from `1` because a second person is on the grid.

## The races

Defined in one place, the `TITLES` and `COMMON` tables at the top of
`scripts/bench-render.py`. **All of it is chosen, not measured**: nothing is
read off an original, it is a repeatable workload.

| | |
| --- | --- |
| Circuit | each title's default (`--track` left out); the loader's `racing on` line is recorded in the result |
| Mode, field | `single_race`: the full AI field, weapons on |
| Player | `--autopilot` from the start to the player's own last crossing |
| Class | the loader's default speed class |
| Seed | 1 |
| Target | 1280x720 offscreen (`--size`), scene only (no HUD, as the single-frame bench) |
| Settings | the defaults: the run's settings file is a throwaway directory, so nothing of a player's `settings.toml` reaches it. The effective values are in the result |
| Length | the player's third lap; the tick cap is 40,000 so a stuck race ends `finished: false` |
| Adapter | whatever `graphics.renderer` selects, which is the default adapter headless; printed in the result |

Titles: `pulse` (PSP EU disc), `pure` (PSP EU), `hd` (decrypted PS3 ISO), `2048`
(extracted Vita package, `data/extracted/vita/PCSF00007`), `omega` (extracted PS4
packages, `data/extracted/ps4`). A title whose source is missing is reported
and skipped.

Changing a row of that table changes what the numbers mean: keep old results
for the old table only.

## What each frame records

`OAG_RENDER_BENCH_RACE=<path stem>` on a `--race --screenshot` run turns it on
(`crates/game/src/race_capture/race_bench.rs`); the recipe sets it. The loop is
tick, then draw, until the player crosses the line for the last time. It does
not need the `perf-probe` feature.

| Column | What it is |
| --- | --- |
| `tick_us` | the simulation step, plus the audio cue drain that rides it (null backend) |
| `record_us` | CPU time encoding the scene (`Scene::render`) |
| `submit_us` | CPU time finishing and submitting the encoder |
| `cpu_us` | `record_us + submit_us` |
| `gpu_scene_us`, `gpu_blur_us`, `gpu_bloom_us`, `gpu_us` | GPU timestamps around the race pass, the motion-blur chain, HD's bloom chain, and their sum. Read back late without waiting, so the pipeline is not drained to take them. Empty where the device cannot write timestamps or a chain did not run |
| `frame_us` | wall time from the previous frame's start to this one's: **the pipelined frame time**, no idle between frames, at most 2 submissions outstanding (the loop waits on submission N-2). This is the number a frame budget is read against |
| `draws`, `culled`, `triangles` | what the scene submitted |
| `lap` | the lap the player was on |

Keep them apart: `frame_us` is *not* `cpu_us + gpu_us`. On a fast GPU the frame
is CPU-bound and `frame_us` tracks `tick_us + cpu_us`; the GPU passes are the
second-order term.

## The result files

`data/bench/<tag>/<title>.csv` is every frame; `<title>.json` is the summary
and everything needed to reproduce the run:

- `title`, `run.source`, `run.track`, `run.command`, `mode`, `laps_target`,
  `tick_cap`, `settings` (size, MSAA, shadows, motion blur, anisotropy, detail,
  culling, camera, FOV, autopilot), the seed is in `run.command`
- `git_hash`, `build` (debug assertions or release), `run.profile`
- `adapter` (name, type, backend, driver, driver info, whether GPU timestamps
  worked)
- `load_average_start`, `load_average_end`, `run.lock` (held, seconds waited)
- `final_tick`, `final_state_hash`, `frames`, `finished`, `wall_seconds`
- `metrics.<name>`: `count, min, mean, p50, p95, p99, max` over the frames after
  the first 60 (`warmup_frames_excluded`), nearest-rank percentiles so every
  value is a frame that really happened
- `per_lap`: `frame_us`, `cpu_us`, `gpu_us` summaries per lap

**Replayable means deterministic, not a replay file.** The same title, circuit,
seed and settings give the same race: `--repeat` runs each title twice and
checks the tick count, frame count and `final_state_hash` agree. A mismatch
would be a finding (render state reaching the simulation), not a flaky bench.

## Comparing

`just bench-compare A B` prints, per metric, p50, p95, p99 and max for A and B,
the delta and the percentage, and warns when the two runs are not the same
workload (different title, circuit, settings, adapter or driver, or a different
race: a changed final tick or hash means the simulation moved, not only the
renderer). Give it two files, or two directories for every title they share.
Treat any delta inside the spread between two runs of the same build as noise:
run the same build twice and compare it with itself to see how wide that is on
the machine in front of you.

## First results (2026-10-14, commit 5a0d357 plus this lane)

Release build, `AMD Radeon RX 7800 XT (RADV NAVI32)`, Mesa 26.2.4, 1280x720, taken
on the shared drive machine **at load average 4 to 11** while the gate lock was
held (other members' builds and bots still ran). Microseconds, p50 / p99 over
each race's frames after the first 60. Relative evidence that the tool works,
not a performance claim.

| Title | Circuit | Ticks | CPU record+submit | GPU (timestamps) | Pipelined frame |
| --- | --- | --- | --- | --- | --- |
| Pulse (PSP EU) | `16_Track` | 7,507 | 1,288 / 1,848 | 323 / 614 | 1,356 / 1,934 |
| Pure (PSP EU) | `01_Vineta_K` | 7,396 | 334 / 530 | 200 / 252 | 415 / 638 |
| HD (PS3 EU) | `talons_junction` | 7,788 | 2,022 / 2,936 | 702 / 898 | 2,306 / 3,270 |
| 2048 (Vita EU) | `altima` | 14,226 | 913 / 1,401 | 438 / 559 | 1,147 / 1,653 |
| Omega (PS4 EU) | `tech_de_ra` | 7,620 | 496 / 766 | 430 / 629 | 591 / 877 |

Every title was run twice on the same build and agreed on the tick count, the
frame count and the final state hash (Pulse `8390365e07d082a5`, Pure
`ddd14fc9964495bd`, HD `f84806f3e8041d2d`, 2048 `91f5e1416a255e56`, Omega
`63527812544adad3`). Pulse's two runs compared with `just bench-compare` differ
by +0.6% at the CPU median and -3.9% at p99 (a few outliers at the max): that
is the noise floor on that machine at that load. On this fast GPU every title is
CPU-bound: the pipelined frame is within 5% to 26% above the CPU time (the simulation step is the difference) and the GPU
passes are a fraction of it.

## Split results (2026-10-14, `split-bench`)

Release build at the lane's commit, **1280x800** (the Deck's panel), scene only, the default
circuit of each title, seed 1, both people on autopilot, Mesa 26.2.4 RADV, the gate lock held.
Microseconds, p50 over each race's frames after the first 60; p99 of the pipelined frame in
brackets. `GPU` is the timestamped scene passes summed (`gpu_us`); `view` is `gpu_view0` /
`gpu_view1` p50. **The load average is the machine's, taken at each run's start from
`/proc/loadavg` (1 minute): it fell from 16 to 3 across the dGPU runs and from 20 to 1 across
the iGPU ones, because other members' builds ran until they finished, so read the ratios
between cases of one title, not the absolute microseconds.** Chosen workload, not measured
against any original.

RX 7800 XT (discrete, `AMD Radeon RX 7800 XT (RADV NAVI32)`), tag `dgpu-800`:

| Title, case | load | CPU | GPU | views | Pipelined frame (p99) | draws |
| --- | --- | --- | --- | --- | --- | --- |
| Pulse, 1 | 16.3 | 1,346 | 325 | | 1,418 (3,238) | 363 |
| Pulse, 2h | 13.2 | 2,040 | 462 | 239 / 222 | 2,132 (3,132) | 752 |
| Pulse, 2v | 10.1 | 2,036 | 460 | 236 / 223 | 2,127 (2,444) | 724 |
| Pure, 1 | 8.1 | 326 | 193 | | 406 (499) | 221 |
| Pure, 2h | 7.5 | 611 | 261 | 130 / 130 | 720 (874) | 463 |
| Pure, 2v | 6.5 | 606 | 241 | 120 / 121 | 712 (908) | 458 |
| HD, 1 | 5.7 | 1,846 | 710 | | 2,125 (2,600) | 762 |
| HD, 2h | 4.6 | 2,760 | 856 | 438 / 415 | 3,068 (3,640) | 1,646 |
| HD, 2v | 3.6 | 2,722 | 822 | 419 / 403 | 3,037 (3,594) | 1,561 |

Raphael iGPU (`AMD Ryzen 9 7900 12-Core Processor (RADV RAPHAEL_MENDOCINO)`, selected with
`MESA_VK_DEVICE_SELECT=1002:164e`), tag `igpu-800`:

| Title, case | load | CPU | GPU | views | Pipelined frame (p99) | draws |
| --- | --- | --- | --- | --- | --- | --- |
| Pulse, 1 | 19.8 | 1,413 | 4,307 | | 5,004 (12,120) | 363 |
| Pulse, 2h | 9.8 | 2,063 | 4,613 | 2,338 / 2,248 | 5,330 (10,189) | 752 |
| Pulse, 2v | 5.5 | 2,268 | 4,748 | 2,441 / 2,316 | 5,457 (10,419) | 724 |
| Pure, 1 | 3.7 | 389 | 3,423 | | 3,519 (4,462) | 221 |
| Pure, 2h | 3.6 | 720 | 3,869 | 1,965 / 1,887 | 4,035 (5,264) | 463 |
| Pure, 2v | 2.6 | 668 | 3,444 | 1,725 / 1,712 | 3,602 (4,322) | 458 |
| HD, 1 | 1.9 | 2,248 | 9,600 | | 14,010 (19,054) | 762 |
| HD, 2h | 0.6 | 3,970 | 11,123 | 5,841 / 5,313 | 16,492 (22,479) | 1,646 |
| HD, 2v | 16.4 | 3,233 | 10,790 | 5,626 / 5,198 | 16,108 (21,369) | 1,561 |

What it says (relative, one machine):

- **The split roughly doubles the CPU and draws on the discrete card** (Pulse 1.5x CPU for
  2.1x draws, Pure 1.9x, HD 1.5x) and costs 1.2 to 1.4x the GPU (each view fills half the
  pixels, so the fill does not double). The pipelined frame is CPU-bound everywhere on this card.
- **On the iGPU the GPU is the limit and the split costs little more than one view**: Pulse
  5.0 ms to 5.3 ms, Pure 3.5 to 4.0 ms, HD 14.0 to 16.5 ms pipelined. The fill is
  the same pixels either way, and the 2x draws land on a CPU that is not the limit. HD at
  1280x800 is over a 60 Hz frame on this iGPU with or without the split, so the split is not
  what makes it slow there.
- **Horizontal against vertical is inside the noise** for Pulse and HD; Pure's vertical cut is
  8 to 11% faster on the iGPU (4,035 against 3,602 at p50), the one gap larger than the
  spread of the other pairs. One run each, loads that differ: a hint, not a finding.
- The `2h` and `2v` runs of a title end on one state hash, and the one-view and the two-view
  races differ because a second person is on the grid.

## On the Steam Deck (the maintainer runs this)

Not run by this lane. On the Deck, in desktop mode, with the repository checked out and the
Pulse and Pure images under `data/images/`:

1. `just bench-render pulse pure --views 1,2h,2v --size 1280x800 --tag deck` (builds a release
   `oag-game`, then runs six races of about 7,500 ticks; the Deck's APU is the only GPU, so no
   `MESA_VK_DEVICE_SELECT`). Nothing opens a window and the recipe passes `--no-audio`. It takes the gate lock
   (`$HOME/.cache/oag/gate.lock`), which on a Deck nothing else holds.
2. Read the printed `bench: pulse-2h:` lines: the `AMD ... (RADV VANGOGH)` adapter must be
   named at the end of each line, not `llvmpipe`. **`frame p50/p99` is the number a frame budget is
   read against**, `gpu per view` the scene passes. A 60 Hz budget is 16,667 us; the 40 Hz
   cap is 25,000 us.
3. `just bench-compare data/bench/<other> data/bench/deck` against the workstation's tags
   lines each case up by file name; expect a WARNING for the different adapter.
4. Put the `data/bench/deck/*.json` files where the lane can read them; they carry the load
   average, driver and the exact command.

To bench an integrated GPU on a desktop with two: `MESA_VK_DEVICE_SELECT=<vendor>:<device> just
bench-render ...` (the ids are in `vulkaninfo --summary`; the Raphael iGPU here is `1002:164e`),
and read the adapter on the result line.

## Reading it honestly

- Numbers are only comparable on the same machine, adapter, driver, profile and
  roughly the same load. The result carries all of that.
- The first results here were taken on a shared, loaded machine (the load
  average is in each file), so they are relative evidence that the tool works,
  not a performance claim.
- A `--profile debug` run (`just bench-render --profile debug`) keeps debug
  assertions on and is several times slower on the CPU side; use it only to
  iterate on the tool.
- A windowed, presenting bench (vsync, the compositor) is not done: this is
  offscreen, so it never waits on a display. A multi-window race is
  refused by name; benching those is the existing `OAG_RENDER_BENCH` path in
  [multi-window](../rendering/multi-window.md). A split race is benched (`--views`), scene only: the
  HUD of each view and the composite are not in the number.
- Adapter: headless, `graphics.renderer` picks the default adapter. On a machine
  with an integrated and a discrete GPU, set the renderer in your settings file
  (or `MESA_VK_DEVICE_SELECT`) to the one the question is about, and read the
  adapter line of the result.

## On your own desk (RADV)

```sh
just bench-render --tag radv-desk --repeat      # all titles you have, a release build
just bench-render pulse --tag radv-desk         # one title
just bench-compare data/bench/radv-desk data/bench/after-my-change
```

Close everything else first, or accept the lock-free caveat above; on a machine
nothing else uses, `OAG_BENCH_NO_LOCK=1` is fine.
