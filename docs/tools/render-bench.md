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
