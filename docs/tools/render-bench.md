# Whole-race rendering benchmark

One fixed race per title, drawn frame by frame with no window, every frame
measured. [`OAG_RENDER_BENCH`/`OAG_RENDER_GPU_BENCH`](../rendering/frame-audit.md)
re-draw **one** captured tick; this plays a whole race, so a corner that is
slow on lap 2 shows up where one frame never would.

```sh
just bench-render                      # every title that is on disk, release build
just bench-render pulse hd --repeat    # two titles, each run twice, tick count and state hash compared
just bench-render pulse --views 1,2h,2v --size 1280x800   # one view, then a split cut both ways
just bench-render pulse --views 2h,2v --motion-blur medium   # the blur on (the default is off), in a split too
just bench-render pulse --motion-blur high --motion-blur-resolution half   # the blur gathered at half resolution
just bench-render pulse --binary /path/to/oag-game --tag before   # a binary built from another commit
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

The split cases chose the opponents' AI before the second person was seated until 2026-10-14
(the original driver flew a two-human race); it is chosen after now, and the frame figures did
not move (see `docs/rendering/multi-window.md`).

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
- **A split draws its motion blur since 2026-10-15** (rule 3 in
  [split screen](../rendering/split-screen.md)), once per view after the last, one submission
  each, but the bench times none of those chains (its timers are one scene pass per view), so
  `gpu_blur_us` stays empty and the blur's cost is in `cpu_us` and `frame_us`. The blur is off
  unless `--motion-blur low|medium|high` asks (the bench's settings are the defaults, and the
  default is off); the setting is recorded in the result's `settings` and the command, so a
  blur-on result never compares clean against a blur-off one. HD's bloom chain *does* run, once per view after the
  last view (`split_exposure`), but outside every view's timed scene pass and untimed itself, so `gpu_bloom_us`
  stays empty too; Pulse's per-view glow (rule 4) is likewise in no GPU column. Both are in
  `frame_us`, which is where their cost is read (see "Per-view glow" in the split-screen page).
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

`--motion-blur-resolution full|half` sets `[render_profiles.<title>] motion_blur_resolution`
for every run, the blur's gather resolution (`half` gathers a quarter of the pixels and blends
the smear back at full size). It is recorded in `settings.motion_blur_resolution`, so `compare`
warns when a full-gather result is set beside a half-gather one. A result written
before the key existed lacks it, so `compare` also warns between an older result and a
newer one at the same gather: read the two `settings` dicts before reading that as a
difference. The summary carries
`gpu_blur_us` and `gpu_bloom_us` beside `gpu_scene_us` and `gpu_us` (since 2026-10-25; older
results lack the two keys and `compare` says so instead of a delta).

Keep them apart: `frame_us` is *not* `cpu_us + gpu_us`. On a fast GPU the frame
is CPU-bound and `frame_us` tracks `tick_us + cpu_us`; the GPU passes are the
second-order term.

## The result files

`data/bench/<tag>/<title>.csv` is every frame; `<title>.json` is the summary
and everything needed to reproduce the run:

- `title`, `run.source`, `run.track`, `run.command`, `mode`, `laps_target`,
  `tick_cap`, `settings` (size, MSAA, shadows, motion blur and its gather resolution, anisotropy, detail,
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

## Split CPU on the notebook (2026-10-15, `split-cpu`)

A second machine and a CPU lane: an **Intel Core Ultra 7 265H notebook** (6 P-cores 0-5, 8 E-cores
6-13, 2 LP-E cores 14-15; `powersave` governor, `balance_performance`), the **NVIDIA RTX PRO 2000
Blackwell Laptop GPU** (8 GB, proprietary driver, the default adapter) and the **Arrow Lake iGPU**
(`Intel(R) Graphics (ARL)`, Mesa ANV). Release build, 1280x800, the same races as above, the gate
lock held. **Every number here says which adapter it is from.**

**An unpinned bench on this machine is not readable.** The same binary measured Pulse 2h at 4,402,
3,541 and 2,134 us (CPU p50) on three runs, Pure 2h at 1,111 and 653 us, HD at 3,894 and 2,075 us,
and Pure's two cuts 1,111 against 711 us in one run: the scheduler moves the render thread between
P and E cores. **Pin it: `taskset -c 0-5 just bench-render ...`** (the P-cores) made the two cuts
agree (Pulse 2,134 / 2,098, Pure 658 / 655 us) and the repeats agree to a few percent. The
comparison below is an **interleaved A/B on the pinned cores**: the before and the after binary
alternate, three rounds each for Pulse and two for HD and Pure, so a load spike lands on both.
Other members' builds still ran on the machine (load 1.2 to 17 across the runs), so a single
round can still carry an outlier; every round is listed.

**The iGPU is selected with the Intel ICD, not `MESA_VK_DEVICE_SELECT`**: with an NVIDIA card in
the machine `MESA_VK_DEVICE_SELECT=8086:7d51` still ran on the NVIDIA GPU (the result line said so),
and `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/intel_icd.json` hides the NVIDIA one. Always read the
adapter off the `bench:` line.

**The profiler.** `samply` (built locally) refused to sample: `kernel.perf_event_paranoid` is 2 and
there is no root (`echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid` lets it). The numbers
below come from the in-tree `perf-probe` marks instead (`cargo build --release -p oag-game
--features perf-probe`, then `OAG_RENDER_BENCH=300 OAG_RENDER_PERF=1 oag-game ... --players 2
--screenshot x.png`, which prints a CPU span between consecutive `perfprobe::mark` calls), run on
the pinned cores.

**Where a Pulse view's CPU went (NVIDIA, the second view of a two-view frame, us):** 18 fog and
zone writes, 50 scenery and glass, 22 pads, 35 exhaust upload, 23 shadow gather, 40 recording the
scene pass, **234 `encoder.finish`**, 44 submit, and **282 for the ships' and the circuit's shine
passes** (`ship+shield-write`: 366 us on the circuit's per-vertex normal turn and 264 us on the
craft's environment-map rebuild when split out, in an earlier run). The shine passes were the
largest item that was not the driver's.

**What changed (`Scene::write_shine`, `oag_render::shine::TrackNormals`, `Drawable::draw_lists`):**

- The circuit shine pass turned and renormalised every vertex's normal on every view; none of it
  reads the camera. `TrackNormals` keeps the unit normals (for the whole race when the circuit
  has no animated node, once per clock value otherwise) and a view only takes two dot products.
  Bit-identical to the per-call arithmetic (`kept_normals_give_every_camera_the_bits_a_fresh_call_does`).
  It speeds up one view too (Pulse 1 view, about 2%).
- The craft shine passes rebuilt every hull's vertex buffer on every view from the craft's pose and
  flaps, which a camera does not decide; the first view of a frame does it now
  (`FrameWork::shine_maps`, asserted by `four_views_do_the_shared_work_once` and the window tests;
  dropping the gate fails the first). The second view's `ship+shield-write` went 282 to 63 us.
- HD's depth prepass bound a texture group per draw for a fragment stage that reads none
  (`fs_depth_only`); it binds one now. That is about 400 us of HD's CPU in one view and in two.

**Result, CPU p50 (us), `record + submit`, NVIDIA RTX PRO 2000 Blackwell Laptop GPU, pinned to
cores 0-5, interleaved A/B, before then after by round:**

| Title, case | before | after | 2-view cost over 1 view, before then after |
| --- | --- | --- | --- |
| Pulse, 1 | 1,308 / 1,364 / 1,324 | 1,301 / 1,302 / 1,272 | |
| Pulse, 2h | 2,116 / 2,098 / 2,068 | 1,897 / 1,832 / 2,105 | 1.58x to 1.46x (p50 of the rounds) |
| Pulse, 2v | 2,831 / 2,259 / 2,086 | 1,853 / 1,918 / 1,890 | |
| HD, 1 | 2,107 / 2,038 | 1,677 / 1,770 | |
| HD, 2h | 3,151 / 2,980 | 2,638 / 2,636 | 1.48x to 1.53x |
| HD, 2v | 2,950 / 2,914 | 2,653 / 2,611 | |
| Pure, 1 | 337 / 345 | 338 / 338 | |
| Pure, 2h | 654 / 648 | 648 / 650 | 1.9x, unchanged |
| Pure, 2v | 655 / 661 | 657 / 656 | |

Pulse's second view cost 774 us over the first and costs about 595 now. HD's saving is in the
shared depth prepass, so it lands on one view as much as on two and the ratio does not fall. Pure
has no shine pass and no prepass and did not move. The pipelined frame (p50) moved with the CPU:
Pulse 2h 2,225 / 2,205 / 2,178 to 2,009 / 1,943 / 2,227, HD 2h 3,513 / 3,336 to 3,124 / 3,138.

**Arrow Lake iGPU (`Intel(R) Graphics (ARL)`, Mesa ANV), same A/B, CPU p50 us, two rounds:**

| Title, case | before | after |
| --- | --- | --- |
| Pulse, 1 | 5,401 / 5,454 | 5,263 / 5,446 |
| Pulse, 2h | 9,078 / 9,332 | 8,408 / 8,801 |
| Pulse, 2v | 9,234 / 9,282 | 8,686 / 8,624 |
| HD, 1 | 2,514 / 2,481 | 2,325 / 2,312 |
| HD, 2h | 3,928 / 3,961 | 3,577 / 3,574 |
| HD, 2v | 3,855 / 3,820 | 3,527 / 3,546 |

The ANV driver spends about four times the NVIDIA driver's CPU on Pulse's frame (5.4 ms against
1.3 ms for one view, 9.2 ms for two): on this iGPU Pulse's split is **CPU-bound** at about 9 ms
pipelined, not GPU-bound as on the Raphael iGPU above (1.5 ms of GPU). The change is worth
about 7% there. HD stays GPU-bound (`frame` 5.9 ms with 2.8 ms of CPU) and its pipelined frame
moved by about 1%. Pure is untouched.

The first `split-cpu-before` / `split-cpu-after-pin` tag directories (sequential runs, not
interleaved) agree on direction but carry the load of other members' builds, which is why the
A/B above is the evidence: Pulse 1 view read 1,968 us in one after-run at load 4.6 against 1,300
in the A/B. Every state hash is unchanged (`final_state_hash`).

**Picture.** 15 stills compared byte for byte before and after, all identical: Pulse 2h and 2v at
ticks 300, 1200 and 2500, Pulse one view, three views, the cockpit camera in a split, two windows
of one seat each, HD one view and 2h, Pure 2h and 2v. A run of the before binary twice is
identical to itself, so the comparison has no noise floor.

**Left, and why** (the open-work thread is in the repository's handover tree): `encoder.finish` is now 40% of a Pulse view's CPU
(234 of about 590 us) and half of HD's (about 500 us at about 800 draws): wgpu validates the
recorded commands there and it scales with their count, so only fewer commands (a texture array
or atlas per drawable, render bundles for the view-independent draws, merging a craft's draws)
cut it, each a change to how the scene is built.


## Encoder finish and the uniform uploads (2026-10-15, `encoder-finish`)

Same notebook, same NVIDIA RTX PRO 2000 Blackwell Laptop GPU and Arrow Lake iGPU, release builds,
1280x800, the gate lock held, every run pinned to the P-cores (`taskset -c 0-5`). The `perf-probe`
build's `OAG_RENDER_BENCH=300` line `render_viewports CPU (record + submit)` is the number (it prints
a one-view race too, and it is the only one that includes `encoder.finish`); `scripts/bench-render.py`
is not used for the comparison, see "Whole-race medians" below. **Other members' builds shared the
machine**, so each cell is the **minimum over four interleaved rounds of the per-run median** (a quiet
round, not a typical one).

**What changed.**

- **One scene block per kind and view** (`oag_raceplay::drawable::SceneBlocks`). Bind group 2 holds
  `Scene` (fog, light rig, clock, shadow map, Zone grade, about a kilobyte) and every drawable owned a
  buffer for it that the frame filled with the same bytes. A drawable now binds one block of a shared
  buffer by dynamic offset, and the frame writes the circuit's, the craft's, the shine passes', the
  flame surfaces' and the behind-the-glass alternate's once a view. Writes through `queue.write_buffer`
  a view (`perf-probe`'s counter, frame 0; one view then two): Pulse 89 / 159 to 56 / 93, HD 78 / 139
  to 55 / 93, Pure 28 / 51 to 19 / 33; Pulse's bytes a view 454 to 282 KiB. Shield shells, weapon
  models and the sky never get a block written (or only while live) and keep a buffer of their own.
- **Every drawable's matrices share one buffer too** (bind group 0, `UniformBlocks` and `Uploads`
  in `drawable/scene_block.rs`): a region per view, a slot of 256 bytes per drawable, the hulls,
  shine passes, wrecks, flares, plumes, shields, absorb passes, circuit, sky, pads, collision,
  gantry, behind-the-glass drawables and the weapon pools claim a slot at load; a view stages its
  matrices in memory and writes them in runs of neighbouring slots just before its scene pass, and
  a slot a view did not stage keeps what that view's region held (what the old per-drawable buffer
  did). Writes a view (one view, then two): Pulse 56 / 93 to 36 / 45, HD 55 / 93 to 37 / 41, Pure 19
  / 33 to 8 / 11. `OAG_RENDER_PERF_SITES=1` prints a probe frame's writes by call site.
- **The craft's animation tables are written by the first view** (`FrameWork::craft_anims`): a
  function of the race clock that every view wrote (8 writes a later view on Pulse, 16 on HD).
- **Tried and not kept: contiguous batches of one texture as one `draw_indexed`** (commit `5b646661f`,
  reverted). Queued draws to issued calls a view, one view: HD 1,447 to 969, Pulse 423 to 363, Pure
  368 to 290, the triangles in the same order, every still identical. Measured quiet (four
  interleaved rounds, second view of a 2h frame, minimum of the per-run medians of the `encoder-finish`
  span): HD 294 to 275 us, Pulse 220 to 220, Pure 144 to 143, while queueing the draws costs 3 to 8 us
  in the recording (`scene-pass` span: HD 84 to 92, Pulse 36 to 39, Pure 22 to 25). Net about -11 us a
  view on HD and +2 to +3 on Pulse and Pure: the draw calls are not what `finish` pays for (see
  below), so it did not pay for its code.

**Result** (`render_viewports CPU`, p50 us, before then after the scene blocks; the merge column is the
reverted commit on top, kept as the record of what it did; "before" is `main` plus a commit that only
routes the writes through the counter):

| Title, case | before | + scene blocks | + merged draws (reverted) |
| --- | ---: | ---: | ---: |
| Pulse, 1 | 1,108 | 1,050 | 1,056 |
| Pulse, 2h | 1,679 | 1,608 | 1,609 |
| HD, 1 | 1,883 | 1,863 | 1,902 |
| HD, 2h | 2,101 | 2,116 | 2,021 |
| Pure, 1 | 551 | 471 | 498 |
| Pure, 2h | 700 | 684 | 667 |

The scene blocks are worth 4 to 5% of Pulse's CPU (about 1 us a `write_buffer` removed, 33 a view)
and Pure's one view (14%, 80 us), nothing measurable on HD (23 writes a view, 2,116 against 2,101 is
noise); the merged column's HD 2h gain did not reproduce in the quiet span runs above, so it is not
credited to the merge. **The Arrow Lake iGPU, same A/B (before then scene blocks plus the merge
column's commit, three rounds, minimum):** Pulse 1 6,903 to 6,267,
Pulse 2h 6,244 to 6,024, HD 1 6,032 to 5,964, HD 2h 8,389 to 8,125, Pure 1 1,839 to 1,830, Pure 2h
2,667 to 2,626 us. **Pictures and state, scene blocks:** 9 stills (Pulse, HD, Pure; one view, 2h, 2v) and 8 more
(HD Vineta K one view and 2h with the behind-the-glass target dumped, HD Zone stage 2 one view and
2h, 2048 and Omega one view, Pulse at tick 1500 one view and 2h, and the Ship Select page, whose 3D
preview draws through `oag_game::preview`) byte-identical before and after, the
before binary rendered twice is identical to itself, `oag-view` ship and circuit stills render (the
viewer binds offset 0 too), and
`final_state_hash` of all six `bench-render` races is the same before and after.

**The matrices and the animation tables, second step** (same method, minimum over four interleaved
rounds, the `render_viewports CPU` p50, scene blocks only then plus both: Pulse 1 1,284 to 1,306,
Pulse 2h 2,028 to 1,987, HD 1 2,058 to 2,016, HD 2h 2,951 to 2,780, Pure 1 567 to 564, Pure 2h 862 to
826 us, at load 3 to 17, so read it as a few percent). **The second view's spans** (`perf-probe`, the
quiet runs, sum of the span minimums: Pulse 483 to 476 us, HD 636 to 571, Pure 294 to 282): the
craft animation span goes to 0 (Pulse 9 us, HD 41), the hull and shine write span falls (Pulse 61
to 42, HD 22 to 12, Pure 12 to 4), `encoder.finish` does not move (216 to 213, 290 to 289, 143 to
140), as it counts commands. **Pictures:** the 9 stills, the 10 above, 19 with weapons and a wreck
(each weapon type fired on Pulse and HD, a bomb trip, a missile hit, a wreck, a two-view rocket)
byte-identical against `main`'s tip (38 of 38 on the merged branch), and
`tests::scene_blocks::two_views_leave_a_block_each_holding_their_own_eye` reads back both views'
craft matrices (dropping the share or the flush fails it).

**Where `encoder.finish` goes: the texture bind-group switches.** The second view of a 2h frame, in
`perf-probe`'s `encoder-finish` span, with and without a timing-only hack that never switches
group 1 again after the first bind (the picture is wrong; never committed), **minimum over four
interleaved rounds of the per-run median** (the medians of the four are Pulse 223 / 100, HD 285 /
102, Pure 139 / 41), us:

| | finish, as is | finish, no group-1 switches | CPU of the 2h frame, as is | no switches |
| --- | ---: | ---: | ---: | ---: |
| Pulse | 216 | 96 | 1,605 | 1,297 |
| HD | 273 | 98 | 2,283 | 1,814 |
| Pure | 133 | 39 | 680 | 420 |

So the per-draw texture bind groups are **55 to 65% of `encoder.finish` and 20 to 25% of a view's
CPU**: the most a texture array, an atlas or bindless textures could remove. Skipping eleven bind
group 2 sets (the craft's, same buffer and offset) moved Pulse's finish by 3 us and HD's by 4, so the
group-2 sets, with their thirteen textures, are not the cost; a switch is about 0.35 to 0.4 us of
finish on this driver (the table's no-switch difference over the switches a view issues: Pure 94 us /
262, Pulse 120 / 334, HD 175 / 442; each switch's share, not an upper bound on its price).

**Why the texture switches are not taken out here.** Sorting the draws by texture changes pixels
(2026-10-03, `scene/shine.rs`: a distance sort moved 1 to 419 pixels where coplanar batches tie);
`texture_2d_array` needs one size and format; an atlas breaks repeat wrapping; and a
`binding_array` indexed by the draw's instance index (`TEXTURE_BINDING_ARRAY` plus non-uniform
indexing) is native only, so the web and phone builds would keep the per-draw path next to it, and
group 1 holds five bindings (albedo, its sampler, lightmap, pad mask, wave map) that every title's
mesh shader reads. That is a lane of its own with this table as its ceiling. Render bundles do not
fit either: a bundle's commands are fixed at its creation, and every drawable's draw list changes
each view with the culling and the level of detail (and the dynamic offsets differ per view).

**Whole-race medians were not readable.** `bench-render pulse hd pure --views 1,2h` before and after,
two interleaved rounds each, at load 3 to 14 from other members' builds: Pulse 1 read 1,991 / 1,468
before and 1,435 / 2,382 after, HD 2h 3,042 / 2,896 and 3,337 / 4,422, Pure 352 / 330 and 322 / 575
(CPU p50 us). The same binary swings by more than the change. What those runs do show is the
state: every `final_state_hash` identical across all four runs. The in-process numbers above are the
evidence.

**A microbenchmark of `finish` per command kind** (400 to 800 draws, a pipeline with the mesh
layout's four groups, 13 textures in group 2) read about 0.1 us a draw, 0.4 us a `set_pipeline` and
0.5 to 1.1 us a `set_bind_group`, but two of its variants doubled between rounds while others stayed
flat, so treat its ratios, not its microseconds, as indicative; the ceiling run above is the
measured share.

**Steam Deck commands (not run: the Deck is the maintainer's), Pulse and HD first.** From the repo
root on the Deck, desktop mode, plugged in, nothing else open. Build the two binaries once each (the
private `XDG_*` directories keep the runs off the Deck's own settings and saves, as in the Cost recipe
of [split screen](../rendering/split-screen.md)):

```sh
git checkout main && cargo build --release -p oag-game --features perf-probe && cp target/release/oag-game /tmp/oag-game-main
git checkout encoder-finish && cargo build --release -p oag-game --features perf-probe && cp target/release/oag-game /tmp/oag-game-new
mkdir -p /tmp/oag-bench
for title in pulse-psp-eu.chd hdfury-ps3-eu-dec.iso; do for rep in 1 2 3; do for n in 1 2; do for bin in /tmp/oag-game-main /tmp/oag-game-new; do
  echo "== $title $bin players=$n load=$(cut -d' ' -f1 /proc/loadavg)"
  XDG_CONFIG_HOME=/tmp/oag-bench/config XDG_DATA_HOME=/tmp/oag-bench/data XDG_STATE_HOME=/tmp/oag-bench/state \
  OAG_RENDER_BENCH=300 $bin data/images/$title --race --no-audio --mode single_race \
    --players $n --split-cut horizontal --ticks 300 --size 1280x800 --screenshot /tmp/oag-bench/shot.png 2>&1 \
    | grep -E '^bench (split|render_viewports CPU)'
done; done; done; done | tee /tmp/oag-bench/encoder-finish.txt
```

`bench split:` must name the Deck's GPU (RADV), not llvmpipe. Read the minimum of each binary's
medians. For one run of each binary add `OAG_RENDER_PERF=1` and read the `encoder-finish` mark.

## Bindless textures (2026-10-16, `bindless`)

Same notebook (NVIDIA RTX PRO 2000 Blackwell Laptop GPU and the Arrow Lake iGPU), release
`perf-probe` builds, 1280x800, `--mode single_race --ticks 300`, pinned (`taskset -c 0-5`), the gate
lock held, interleaved A/B on **one binary** (`OAG_BINDLESS=0` selects the per-slot groups), **minimum
of two rounds of the per-run median**: `render_viewports CPU`, `pipelined per frame` and the scene
passes' GPU timestamps from a plain run, and the `encoder-finish` span of the second view from a run
with `OAG_RENDER_PERF=1`. `main` (`b4581bf70`) reads the same as `OAG_BINDLESS=0`, so the
asked-for features and limits cost the per-slot path nothing: on the final build, one view, CPU
Pulse 978 / 1,027, HD 1,958 / 1,960, Pure 569 / 507 us (two rounds); on the build before
immediates, 2h, Pulse 2,042 / 1,989, HD 2,913 / 2,822, Pure 792 / 823.

**What it is** (`oag_mesh::mesh_render::bindless`): on a device with `TEXTURE_BINDING_ARRAY`,
`PARTIALLY_BOUND_BINDING_ARRAY` and `IMMEDIATES`, every race drawable binds group 1 once: its
distinct textures in one `binding_array` (at most 1,024), its distinct samplers in another (at most
16), and each draw pushes its slot's row of five indices (albedo, lightmap, pad mask, wave map,
sampler) as immediates when the slot changes. One WESL source: `shaders/bindings.wesl` declares the
arrays under `@if(bindless)`, and the three functions that read group 1 (`lit_texel`,
`holo_billboard`, `sample_at_slope_level`) declare the five old names from the row first, so the
default `mesh.wgsl` is byte-identical to `main`'s and the variant is `mesh_bindless.wgsl`.

**NVIDIA, 2h, us, per-slot then arrays:**

| Title | CPU | pipelined | scene passes, GPU | second view's `finish` | ceiling (no group-1 switch) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Pulse | 1,499 / 1,264 | 1,498 / 1,265 | 423 / 401 | 208 / 124 | 96 |
| HD | 2,064 / 1,916 | 3,545 / 3,660 | 1,516 / 1,660 | 283 / 160 | 98 |
| Pure | 638 / 552 | 1,021 / 992 | 514 / 515 | 138 / 88 | 39 |

**One view, NVIDIA, us (per-slot / arrays), two rounds:** CPU Pulse 1,027 / 890, HD 1,960 /
1,960, Pure 507 / 503; pipelined 1,093 / 902, 3,018 / 2,872, 779 / 740; scene pass GPU 333 / 361,
1,072 / 1,083, 398 / 387. One view switches less than a split's second view pays for, so only
Pulse's CPU moves; nothing regresses.

HD's GPU and pipelined cells were re-run three more times each way (scene passes 1,348 / 1,406 at
the minimum, pipelined 3,549 / 3,442): within the noise, and the pipelined frame is lower with the
arrays. The `finish` ceiling column is `encoder-finish`'s, measured on a busier machine (its
per-slot `finish` read 216, 273 and 133), so read the arrays as taking 53 to 70% of what that hack
took out (Pulse 84 of 120 us, HD 123 of 175, Pure 50 of 94), not as reaching it: a group 1 bind per drawable call and an immediate per slot change
are left, and the wgpu tracking of each array's elements when the group is bound.

**Arrow Lake iGPU** (`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/intel_icd.json`), 2h, us, per-slot
then arrays: pipelined HD 6,598 / 6,616, Pulse 2,535 / 2,508, Pure 1,804 / 1,809; second view's
`finish` HD 664 / 500, Pulse 342 / 258, Pure 486 / 370. The CPU line swings by 50% between
rounds here and is not quoted. The iGPU is GPU-bound in a split, so the frame does not move; what it
shows is that the variant costs its GPU nothing.

**Two designs that lost, and why** (both measured, neither kept):

- **One array element per slot and role, padded to a fixed 1,024**: the second view's `finish`
  *rose* on NVIDIA (Pulse 288 to 440 us, Pure and HD alike). wgpu walks a texture-initialisation
  check per array element every time the group is bound, so 4 x 1,024 elements a bind cost more than
  the switches. Padded to 256 instead it fell (Pulse 318 to 216, Pure 190 to 151), which is what
  identified the cause; the arrays now hold each distinct texture once and are bound short of their
  length (`PARTIALLY_BOUND_BINDING_ARRAY`), so the layout stays one and the pipeline cache valid.
- **The slot carried from the vertex stage** (the first instance as a flat varying, with a table of
  rows in a storage buffer): `finish` fell as now, but the iGPU's pipelined frame rose 6.5% on HD
  (6,571 to 6,995 us, three runs each) and 16% on Pure (1,827 to 2,122), and NVIDIA's scene passes
  about 15% on Pulse and Pure. A varying is not provably uniform, so naga decorates every array read
  through it `NonUniform` (four in a probe of the same reads, none with an immediate), and ANV, which
  has no native non-uniform indexing, pays for that per fragment. The variant now asks for no
  non-uniform indexing at all, and `oag-shader-check` validates it without that capability, so an
  index that is not uniform fails the build.

**Cost elsewhere.** Race load on HD: about +0.3 s on NVIDIA (4.1 to 4.3 s against 4.6 to 4.9,
three runs each), none on the iGPU; the first runs after a rebuild read +3 s on both while the
drivers compiled the variant's pipelines into their shader caches. Pipelines built per race: the
same (84 distinct for HD's Talon's Junction either way). Per title, the drawable with the most
distinct textures on the default circuit: Pulse 141, Pure 191, HD 192, Omega 176, 2048 167; at
most two samplers a drawable (the repeat sampler, and the advert cards' clamp sampler, which every
card slot now shares).

**Pictures.** Twelve stills (Pulse, Pure, HD Talon's Junction, HD Zone, 2048 and Omega, one view
and 2h, tick 300) are byte-identical against `main` on both paths, and
`oag_raceplay::tests::bindless` draws eight slots of eight colours (one pair sharing a texture, one
slot on the white fallback) through both paths and fails on a pixel's difference.

**How long a run takes.** This lane's first A/B (three titles, one view and 2h, three binaries,
plain and `perf` runs, three rounds) was 108 launches and about 50 minutes, almost all of it race
loads and the 300 ticks before the measured frames. The table above is the trimmed form: 2h only,
off against on (main was already shown equal to off), two rounds, about 10 minutes a GPU. An A/B
inside one process (load once, alternate the two paths frame by frame) would take most of the rest
out; not built.

**Steam Deck commands (not run: the Deck is the maintainer's).** First, whether RADV has the three
features and the limits under wgpu 30; the race log says which drawables took the arrays:

```sh
git checkout main && cargo build --release -p oag-game --features perf-probe && cp target/release/oag-game /tmp/oag-game-bl
mkdir -p /tmp/oag-bench
XDG_CONFIG_HOME=/tmp/oag-bench/config XDG_DATA_HOME=/tmp/oag-bench/data XDG_STATE_HOME=/tmp/oag-bench/state \
RUST_LOG=info,oag_mesh::mesh_render::bindless=debug /tmp/oag-game-bl data/images/hdfury-ps3-eu-dec.iso --race --no-audio \
  --mode single_race --ticks 60 --size 1280x800 --screenshot /tmp/oag-bench/shot.png 2>&1 | grep -E 'binding arrays|bindless:' | head
vulkaninfo 2>/dev/null | grep -E 'descriptorBindingPartiallyBound|maxPushConstantsSize|maxPerStageDescriptorUpdateAfterBindSampledImages|maxPerStageDescriptorUpdateAfterBindSamplers'
```

No `group 1 as binding arrays` line means the Deck lacks one of the three; the race then draws as
`main` does. If the lines are there, the A/B (one binary, `OAG_BINDLESS` per run):

```sh
for title in pulse-psp-eu.chd hdfury-ps3-eu-dec.iso; do for rep in 1 2; do for b in 0 1; do
  echo "== $title bindless=$b load=$(cut -d' ' -f1 /proc/loadavg)"
  XDG_CONFIG_HOME=/tmp/oag-bench/config XDG_DATA_HOME=/tmp/oag-bench/data XDG_STATE_HOME=/tmp/oag-bench/state \
  OAG_BINDLESS=$b OAG_RENDER_BENCH=300 OAG_RENDER_GPU_BENCH=100 /tmp/oag-game-bl data/images/$title --race --no-audio \
    --mode single_race --players 2 --ticks 300 --size 1280x800 --screenshot /tmp/oag-bench/shot.png 2>&1 \
    | grep -E '^bench (split|render_viewports CPU|pipelined|sum of)'
done; done; done | tee /tmp/oag-bench/bindless.txt
```

Read the minimum of each path's medians; the Deck is GPU-bound in a split like the iGPU, so the
pipelined frame is the line that decides, and it must not rise.

## HD on a notebook, before and after the craft cull (2026-10-15, `hd-gpu`)

Release build, 1280x800 and 1920x1080 scene only, Talon's Junction, seed 1, autopilot, the
gate lock held, **load 1 to 2.3 at each run's start**. Microseconds, p50 over the frames after
the first 60. Two adapters on one notebook: `Intel(R) Graphics (ARL)` (Arrow Lake iGPU, Mesa
anv, chosen with `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/intel_icd.json` because
`MESA_VK_DEVICE_SELECT` does not filter NVIDIA's own ICD) and `NVIDIA RTX PRO 2000 Blackwell
Generation Laptop GPU` (the default adapter). "Before" is `f82807832`'s `oag-raceplay`,
"after" the commit that stops drawing craft outside the frustum, same everything else.

| Adapter, size, case | CPU | GPU | Pipelined frame |
| --- | ---: | ---: | ---: |
| Intel, 1280x800, 1, before | 2,472 | 2,244 | 4,352 |
| Intel, 1280x800, 1, after | 2,474 | 2,182 | 4,249 |
| Intel, 1280x800, 2h, before | 3,959 | 2,865 | 5,942 |
| Intel, 1280x800, 2h, after | 3,863 | 2,735 | 5,739 |
| Intel, 1920x1080, 1, before | 2,587 | 3,304 | 5,417 |
| Intel, 1920x1080, 1, after | 2,550 | 3,257 | 5,373 |
| Intel, 1920x1080, 2h, before | 4,040 | 3,793 | 7,075 |
| Intel, 1920x1080, 2h, after | 3,955 | 3,678 | 6,949 |
| NVIDIA, 1280x800, 1, before | 2,085 | 1,037 | 2,492 |
| NVIDIA, 1280x800, 1, after | 2,131 | 1,054 | 2,489 |
| NVIDIA, 1280x800, 2h, before | 2,952 | 1,488 | 3,272 |
| NVIDIA, 1280x800, 2h, after | 3,216 | 1,303 | 3,561 |

**Read it as no change beyond noise**: one run each, and no run-to-run spread was measured on
this host (the NVIDIA 2h CPU reads 9 % worse, which the same code could do on another run); the
iGPU's GPU time falls 1 to 4 %, the NVIDIA card is CPU-bound and does not move. Two traps recorded by this lane: **the first bench after a
rebuild read 2x the CPU** (4,805 against 2,472 us for the same code), so a "before" taken
first is wrong; and **this iGPU is CPU-bound at 1280x800** (frame 4.3 ms, GPU 2.2 ms), so
`frame_us` hides a GPU saving there. The per-piece GPU spans, the settings matrix and the
Deck commands are in [`hd-gpu-cost.md`](../rendering/hd-gpu-cost.md).

## Threaded views (2026-10-15, `threaded-views`)

Same notebook (Core Ultra 7 265H, `powersave` governor, NVIDIA RTX PRO 2000 Blackwell Laptop GPU and
the Arrow Lake iGPU), release `perf-probe` builds, 1280x800, `--mode single_race --ticks 300`, every
run pinned to the P-cores (`taskset -c 0-5`) and under the gate lock, three interleaved rounds
(two on the iGPU), **minimum of the per-run medians** of `render_viewports CPU (record + submit)`
and of `pipelined per frame` (`OAG_RENDER_BENCH=300 OAG_RENDER_GPU_BENCH=100`), load 1.2 to 1.95 at
each run's start. "main" is `5db58a86f`; "new" is this lane's build, whose recording is picked
with `OAG_SPLIT_RECORDING` on one binary (`interleaved` is the old order and the default; `serial`
prepares every view and then records them in order; `threaded` records them on scoped threads).

**NVIDIA, microseconds (CPU / pipelined):**

| Title, views | main | new, interleaved | new, serial | new, threaded |
| --- | ---: | ---: | ---: | ---: |
| Pulse, 1 | 974 / 991 | 1,033 / 1,002 | | |
| Pulse, 2h | 1,520 / 1,518 | 1,527 / 1,512 | 1,537 / 1,612 | 2,025 / 2,067 |
| Pulse, 2v | 1,515 / 1,494 | 1,541 / 1,469 | 1,506 / 1,528 | 2,011 / 2,097 |
| Pulse, 4 | 2,442 / 2,563 | 2,461 / 2,593 | 2,509 / 2,782 | 2,856 / 3,194 |
| HD, 1 | 1,950 / 2,842 | 1,955 / 2,892 | | |
| HD, 2h | 2,094 / 3,514 | 2,093 / 3,542 | 2,265 / 3,594 | 2,521 / 3,562 |
| HD, 2v | 2,045 / 3,630 | 2,073 / 3,716 | 2,296 / 3,670 | 2,492 / 3,643 |
| HD, 4 | 3,813 / 5,675 | 3,633 / 5,729 | 3,954 / 5,821 | 4,033 / 5,679 |
| Pure, 1 | 521 / 767 | 516 / 766 | | |
| Pure, 2h | 612 / 1,031 | 593 / 1,020 | 641 / 1,018 | 1,192 / 1,068 |
| Pure, 2v | 606 / 817 | 604 / 813 | 673 / 848 | 1,189 / 1,067 |
| Pure, 4 | 1,199 / 1,311 | 1,213 / 1,277 | 1,216 / 1,256 | 1,524 / 1,608 |

The prepare and record split costs nothing measurable (main against new, interleaved; Pulse's one
view re-measured over five interleaved rounds: 1,040 against 1,032). **Threads lose everywhere on
this machine**, by 10 to 100% (Pure's views are the smallest bursts). Serial batching is free on
Pulse and 8 to 11% dearer on HD and Pure: the GPU starts on the first view only once every view is
prepared.

**Why the threads lose: the second core, not wgpu.** `encoder.finish` takes only a read lock on
the device, and the spans show both views' `finish` slowing (Pulse 2h: 353 and 204 us serial, 515
and 654 us threaded). Three experiments, Pulse four views, one build: one view at a time on its
own spawned thread (`staggered`) is as slow as all at once (6,064 against 5,962 us, a loaded
round); the threaded build pinned to one core matches serial (3,872 against 3,037, 2,834 against
2,656); and a standalone microbenchmark on the same cores between 12 ms sleeps reads an empty
`std::thread::scope` spawn and join at about 200 us median, a 2 x 250 us compute split gaining 4%
on a fresh thread, nothing on a parked worker woken by a condvar, 10 to 20% with the worker woken
at the start of the frame, and half on a worker spinning between frames. A view's recording is a
0.2 to 0.4 ms burst once a frame, and an idle P-core under `powersave` does not wake and clock up
for it.

**The Arrow Lake iGPU** (`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/intel_icd.json`, two rounds, new
build only, CPU / pipelined):

| Title, views | interleaved | serial | threaded |
| --- | ---: | ---: | ---: |
| Pulse, 2h | 3,497 / 2,469 | 2,971 / 2,530 | 3,426 / 3,031 |
| Pulse, 4 | 4,366 / 5,905 | 6,738 / 4,488 | 6,420 / 4,469 |
| HD, 2h | 5,509 / 7,472 | 6,775 / 7,276 | 6,445 / 7,339 |
| HD, 4 | 7,443 / 11,665 | 12,423 / 11,740 | 9,743 / 11,489 |
| Pure, 2h | 1,902 / 1,818 | 2,524 / 1,869 | 2,050 / 1,761 |
| Pure, 4 | 2,400 / 2,607 | 4,102 / 2,780 | 3,309 / 2,557 |

Here, where ANV's per-view CPU is about four times NVIDIA's, threads do beat serial batching (HD
four views 12,423 to 9,743 us, Pure four 4,102 to 3,309), but both batched recordings lose to the
interleaved order at four views and on HD and Pure two views; Pulse two views is the exception
(serial 2,971 and threaded 3,426 against interleaved 3,497, two rounds only). The pipelined frame
barely moves (the iGPU is GPU-bound). **Why batching costs here is not measured**: the hypothesis is
that ANV's submit or finish waits on GPU work the interleaved order has already started, but the
CPU line idles the GPU before each run, so that is a lead, not a finding. **The lever it points at,
not built**: record a view on a worker *while the next view is prepared*, and submit each as soon
as it is recorded. It needs the effect pipelines' uploads to stop holding `&mut` state the
recording reads (today `Recorder` holds their `RefCell` guards, so a prepare cannot run beside a
recording).

**Pictures and state.** 34 stills (Pulse, HD and Pure at 1, 2h, 2v and 4 views with the blur off
and on, Pulse/HD/Pure 2h and 4 at tick 1500, 2048 and Omega 2h) and 51 weapon stills (thirteen
weapons fired, a bomb trip, a wreck, a hit and a Leach lock on each of Pulse, HD and Pure, 2h) are
byte-identical against `main` in all four recordings; `final_state_hash` of a 600-tick two-view
autopilot race is the same on Pulse, HD and Pure. `oag-game` tests
`split_recording_ground_truth::` capture one frame (Pure 2h at tick 1500, Pulse four views with a
Rocket) interleaved, reversed and threaded and fail on a difference.

**Steam Deck commands (not run: the Deck is the maintainer's).** One binary, the recording picked
per run; the Deck's per-view CPU is closer to the iGPU's than to NVIDIA's, so threads have a
better chance there, and this A/B decides any per-platform default:

```sh
git checkout threaded-views && cargo build --release -p oag-game --features perf-probe && cp target/release/oag-game /tmp/oag-game-tv
mkdir -p /tmp/oag-bench
for title in pulse-psp-eu.chd hdfury-ps3-eu-dec.iso; do for rep in 1 2 3; do for n in 2 4; do for rec in interleaved threaded; do
  echo "== $title players=$n $rec load=$(cut -d' ' -f1 /proc/loadavg)"
  XDG_CONFIG_HOME=/tmp/oag-bench/config XDG_DATA_HOME=/tmp/oag-bench/data XDG_STATE_HOME=/tmp/oag-bench/state \
  OAG_SPLIT_RECORDING=$rec OAG_RENDER_BENCH=300 OAG_RENDER_GPU_BENCH=100 /tmp/oag-game-tv data/images/$title --race --no-audio \
    --mode single_race --players $n --ticks 300 --size 1280x800 --screenshot /tmp/oag-bench/shot.png 2>&1 \
    | grep -E '^bench (split|render_viewports CPU|pipelined)'
done; done; done; done | tee /tmp/oag-bench/threaded-views.txt
```

Read the minimum of each recording's `render_viewports CPU` medians; threads are worth a default
only where they win that by more than the rounds' spread.

## Overlapped recording (2026-10-16, `split-followups`)

`OAG_SPLIT_RECORDING=overlapped` (`Recording::Overlapped`): one scoped worker a frame, spawned
before the first view is prepared; it records view `k` while the calling thread prepares view
`k + 1`, and the calling thread submits view `k` as soon as it comes back (the last view is
recorded on the calling thread). What made it possible: each effect pipeline's draw handles are a
cloneable `Drawer` a view's `ViewPass` keeps, so a recording no longer holds the pipelines'
`RefCell` guards while the next view's upload needs them `&mut`.

Same notebook and protocol as "Threaded views": release `perf-probe` build, 1280x800, `--mode
single_race --ticks 300`, `OAG_RENDER_BENCH=300 OAG_RENDER_GPU_BENCH=100`, pinned to
`taskset -c 0-5`, under the gate lock, three interleaved rounds, **minimum of the per-run medians**,
one binary with the recording picked by the variable, load 0.9 to 2.9. CPU (`render_viewports CPU`)
/ pipelined, us:

| Title, views | NVIDIA interleaved | NVIDIA overlapped | iGPU interleaved | iGPU overlapped |
| --- | ---: | ---: | ---: | ---: |
| Pulse, 2h | 1,272 / 1,270 | 2,123 / 2,297 | 1,999 / 2,537 | 4,963 / 3,310 |
| Pulse, 2v | 1,239 / 1,292 | 1,985 / 2,210 | 3,094 / 2,493 | 4,966 / 3,367 |
| Pulse, 4 | 2,031 / 2,038 | 3,044 / 3,422 | 3,585 / 3,757 | 6,507 / 4,976 |
| HD, 2h | 1,987 / 3,771 | 4,177 / 4,626 | 5,909 / 7,622 | 7,620 / 7,651 |
| HD, 2v | 1,973 / 3,806 | 4,587 / 4,758 | 5,863 / 7,548 | 7,390 / 7,629 |
| HD, 4 | 3,127 / 5,836 | 6,048 / 6,390 | 7,517 / 11,726 | 11,377 / 11,783 |

**Overlapped loses everywhere on this notebook, by more than the scoped threads did**: 50 to 130 %
more CPU on NVIDIA, 25 to 150 % on the iGPU, and the pipelined frame follows on both. **Why,
measured** (NVIDIA, three rounds, minimum): the same binary pinned to **one** core (`taskset -c 0`)
reads Pulse 2h 1,336 us overlapped against 1,246 interleaved and HD 2h 2,199 against 1,980 (+7 to
11 %, the thread and channel bookkeeping), where on six cores it reads 2,135 and 4,426, and the
threaded recording 1,644 and 2,193. So the work does not get slower, the handoffs do: every view
is a round trip to a worker that sleeps on its channel between views, a wake-up per view where
`threaded` pays one per frame, and a cold P-core under `powersave` does not stay awake through a
0.2 to 0.4 ms prepare. **The default stays interleaved.** The worker is scoped to the frame, not
long-lived: the recording borrows the scene, the workspace denies `unsafe` (no lifetime erasure)
and there is no thread pool crate to lend a safe one; a parked long-lived worker woken once per
view would pay the same wake-up the one-core run isolates. A spinning worker is the one variant the
microbenchmark under "Threaded views" says could win, at the cost of a core burning between
frames; not built.

**The `Drawer` refactor costs nothing**: base (`022164d47`) against this branch, interleaved, the
same three rounds, Pulse one view 881 / 893 us and 2h 1,267 / 1,244, HD one view 1,956 / 1,987 and
2h 2,063 / 1,950 (an A/A pair: the draws are the same calls on the same handles).

**Pictures**: `split_recording_ground_truth::` compares overlapped with interleaved (Pure 2h, Pulse
four views with a Rocket) and fails if a later view's log is dropped (Pulse's four views, checked).

**Steam Deck**: build from `main` after this lane merged (the `threaded-views` branch has no
`overlapped`, and an unknown `OAG_SPLIT_RECORDING` silently records interleaved), then the
"Threaded views" loop with that binary:

```sh
git checkout main && cargo build --release -p oag-game --features perf-probe && cp target/release/oag-game /tmp/oag-game-ov
mkdir -p /tmp/oag-bench
for title in pulse-psp-eu.chd hdfury-ps3-eu-dec.iso; do for rep in 1 2 3; do for n in 2 4; do for rec in interleaved threaded overlapped; do
  echo "== $title players=$n $rec load=$(cut -d' ' -f1 /proc/loadavg)"
  XDG_CONFIG_HOME=/tmp/oag-bench/config XDG_DATA_HOME=/tmp/oag-bench/data XDG_STATE_HOME=/tmp/oag-bench/state \
  OAG_SPLIT_RECORDING=$rec OAG_RENDER_BENCH=300 OAG_RENDER_GPU_BENCH=100 /tmp/oag-game-ov data/images/$title --race --no-audio \
    --mode single_race --players $n --ticks 300 --size 1280x800 --screenshot /tmp/oag-bench/shot.png 2>&1 \
    | grep -E '^bench (split|render_viewports CPU|pipelined)'
done; done; done; done | tee /tmp/oag-bench/overlapped.txt
```

## The presentation resolve (2026-10-16, `split-followups`)

`OAG_RESOLVE_BENCH=N` on a `--presented` capture of a `perf-probe` build runs the presentation
resolve (the upscaler and the blit) `N` times over the captured frame, each submitted and waited
for, and prints `bench presentation resolve (serialised: CPU + GPU)`. The single-frame bench above
leaves the resolve out. First use: FSR 3 per split view, numbers in
[split-screen.md](../rendering/split-screen.md), "Per-view FSR 3".

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
- `--binary PATH` runs another `oag-game` (a before/after pair built from two commits) with the
  same flags; the result records the command, so say which commit the binary was in the tag.
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
