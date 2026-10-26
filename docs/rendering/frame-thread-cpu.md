# The frame thread's CPU: where a race frame spends it (2026-10-21)

The Steam Deck's perf meter shows `CPU` 8 to 10 ms on a paused race, with `PRESENT` 8 ms and
`OUTSIDE` 0.7 ms. `CPU`, `PRESENT` and `OUTSIDE` partition the frame (see
`crates/present/src/perf/cost.rs`), so the frame thread is busy for most of a 60 Hz frame while
no tick runs. This page is what was measured on a desktop host and what is still open for the
Deck. Evidence is ratios and shares; the host is several times faster than the Deck's Zen 2.

## How to read it: `--frame-log` now prints a phase line

`oag-game --frame-log` (or `OAG_FRAME_LOG=1`) prints one extra line a second before the
frame-log line, `frame-phases`, with the mean milliseconds per frame of each stretch of the
frame thread (`crates/game/src/main/session/phases.rs`). They add up to `total`, which sits close
to `cpu_ms` plus `present_ms` less the acquire wait (the acquire, `draw_screens` and the
end-of-frame bookkeeping are outside the phases):

| Phase | What it covers |
| --- | --- |
| `ticks` | the fixed-step ticks and the stage update |
| `prep` | end-race flow and `begin_scene_frame` |
| `setup` | `draw` up to the stage dispatch: view, encoder, target sizing |
| `scene` | the stage's drawing into the encoder: for a race, the scene record |
| `compose` | HUD, bloom composite, menu and upscale passes |
| `overlay` | perf overlay, cursor, timestamp resolves |
| `submit` | `encoder.finish()` and `Queue::submit` (wgpu-core validation and tracking) |
| `readback` | mapping last frame's timestamps, feeding the resolution controller |
| `present` | `Queue::present` |

The stage name reads `race-paused` while the pause rule freezes the race: `Session::paused`
(`pause.rs`) is true for a Start pause and for the race's own start countdown, and false
otherwise. The 2026-10-21 Deck lane measured that on the current build a `3:start` script tap
*pauses* a running race (see its section below); an earlier version of this page said `--race`
starts frozen and a tap releases it, which holds only through the countdown.

## What was measured

Headless gamescope at 1280x800 on the host's RX 7800 XT, release build, `--race --autopilot
--no-audio --frame-log`, load average 16 to 22 (wall-clock milliseconds are not comparable with
the Deck's; the shares are). Command:
`gamescope --backend headless -W 1280 -H 800 -w 1280 -h 800 -r 60 -- oag-game <image> --race
--autopilot --no-audio --frame-log --script "12:quit"` (add `3:start,` for racing).

Share of the frame thread's CPU, **paused**, before any change (three runs, `total` 1.2 to 1.5 ms
on Pulse, 1.5 to 1.9 ms on HD):

| Phase | Pulse | HD/Fury |
| --- | --- | --- |
| `submit` (finish and submit) | 47 % | 58 % |
| `scene` (record) | 32 % | 25 % |
| `present` | 13 % | 10 % |
| `compose` | 3 % | 3 % |
| everything else | 5 % | 4 % |

- **A paused frame costs nearly what a racing one does.** Pulse paused `total` 1.26 ms against
  1.86 ms racing; HD 1.87 against 2.02. The ticks are 0.01 ms either way: the cost is the draw,
  not the simulation.
- **Draw count drives it.** The whole-race bench (`just bench-render pulse hd`) p50: Pulse 321
  draws, `record_us` 848, `submit_us` 420; HD 617 draws, `record_us` 1,015, `submit_us` 1,181.
- **Submit is wgpu-core, not our code.** A flat profile of the frame thread (perf, no call
  graph; the dwarf call graph does not unwind through the Mesa driver) puts wgpu-core's
  command-buffer work at the top: `encode_render_pass`, `TextureUsageScope::merge_single`,
  `BufferUsageScope::clear`, and `CommandBufferTextureMemoryActions::register_init_action` with
  a `Vec::retain` on `TextureSurfaceDiscard` (an init action per bound texture per pass, the
  bindless texture arrays making that long). RADV and the kernel's DRM ioctls are another
  third of the thread; our own `oag_*` code is about a fifth.
- **The GPU is not what the CPU row waits on, on this host.** Raising the render target from
  1280x725 to 7624x4320 (gpu_ms 0.36 to 3.6) moved `cpu_ms` from 2.5 to 3.8 on the same run.
  The Deck's GPU is far slower, and a shared-memory APU can stall a `write_buffer` or a submit
  on the GPU's use of the same memory. That cannot be seen from here; see the handover.
- **The build is the same.** The Deck builds in `docs/rendering/hd-gpu-cost.md` and
  `docs/tools/packaging.md` are release builds (`perf-probe`, `znver2`). `perf-probe` adds
  counters on the frame path (allocation counting, write tallies), not wgpu validation; this
  host's profile was the default release, which has the same wgpu-core validation.

## What changed (one commit each, no pixel change)

1. **`shine::TrackNormals`** turned every circuit vertex's normal again on each new clock value
   when the circuit has any animated node (Pulse's do). It now turns only the vertices that ride
   a node; the rest are placed once. Bit-identical arithmetic per vertex. It was 9 % of the whole
   bench thread's samples. Pulse bench `cpu_us` p50 1,277 to 1,074 and 1,111 us (two runs; the
   same build run twice differs by 3.5 %), `record_us` 848 to 712. HD has no animated circuit
   shine, no change.
2. **Animation tables and hull shine coordinates are written only when their inputs moved.**
   `Drawable::write_anims`/`write_node_anims` are functions of the race clock; a paused race, or a
   second frame inside one tick, wrote identical bytes. `write_environment_map` is a function of
   the craft's pose and flaps. Each now remembers what it last wrote. Paused `scene` phase on
   Pulse 0.40 to 0.27 ms (three interleaved before/after runs), on HD 0.45 to 0.31 ms. Racing
   frames are unaffected by this one (the clock moves each tick).

Both leave screenshots byte-identical (`cmp` of `--screenshot` at 1, 400 and 1500 ticks on Pulse,
HD and Pure). Neither is in a simulation crate.

The sum is about 0.15 ms of 1.2 to 1.5 on this host, about a tenth of a paused frame. It does
not explain a Deck `CPU` of 8 to 10 ms by itself.

## Dropped or not done

- Memoising the wgpu work itself is not possible from here: `submit` is wgpu-core walking the
  recorded passes. The levers are fewer draws and passes, or render bundles, neither a no-pixel-
  change quick fix.
- GPU timestamp rows for the untimed passes (shadow, particles, HUD overlay, composite) were not
  added: on this host `gpu_ms` is 0.3 to 0.9 ms and `frame-phases` already shows the CPU side, so
  the open question is the Deck's, and the handover thread lists what to read there.

## Cross-title

The changes are in `oag-render` and `oag-raceplay`'s drawables, which every title shares. Pulse
and Pure share the Pulse path; HD, Omega and 2048 use the same drawables. Measured on Pulse and
HD only; the screenshots were also compared on Pure.

## On the Steam Deck (2026-10-21, `deck-read`)

Measured on the Deck itself: release AppImage (`just appimage-deck`, znver2, `perf-probe`), run
extracted, native under the Deck's gamescope on `DISPLAY=:1` with the environment copied from
Steam's process, `--no-audio --frame-log --race --autopilot`, a private `XDG_*` tree holding a
copy of the maintainer's `settings.toml` (read, never modified; only `pause_on_focus_loss =
false`). Pulse is the EU PSP image, HD the EU PS3 image. Each run is 12 s; the first 4 `frame-
phases` and frame-log lines (load, resolution settling) are dropped and the rest averaged.
Deck: load 0.4 to 1.2, on AC (`ACAD/online` 1, battery full), `powersave` governor.
The commands are below.

The frame-log stage name is `race-paused` after a `3:start` tap and `race` without it, so on
this build the tap *pauses* (ticks 0.06 ms against 0.07 to 0.25 ms racing). Rows below use the
stage name the log printed.

### The split (ms per frame, the maintainer's settings, DRS on)

| Run | scene | compose | submit | present | total | CPU row | PRESENT row |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Pulse racing (`race`) | 1.17 | 0.28 | 3.39 | 1.00 | 6.12 | 5.1 | 9.7 |
| Pulse paused | 0.90 | 0.28 | 3.03 | 1.01 | 5.51 | 4.6 | 9.7 |
| HD racing | 1.80 | 0.18 | 3.59 | 0.92 | 6.97 | 6.0 | 7.1 |
| HD paused | 1.08 | 0.16 | 3.01 | 0.75 | 5.26 | 4.5 | 6.7 |

`CPU row` is `cpu_ms - present_ms` from the frame-log line, which is what the meter draws
(`CpuCost::rows`). It equals `total - present` of the phases line within 0.1 ms, so the two
instruments agree. The window is 3840x2160 under this gamescope and DRS settled between 1728x972
and 2880x1620.

- **`submit` (`encoder.finish()` plus `Queue::submit`) is 55 to 60 % of the thread**, about 3 ms
  on both titles; `scene` is 1 to 1.8 ms, `present` about 1 ms (the `PRESENT` row's 7 to 10 ms is
  mostly the swapchain acquire wait, which is the GPU and the limiter, not CPU).
- **A paused frame costs 5.3 to 5.5 ms of thread time, about 4.5 ms of it in the `CPU` row**, 4 to
  5 times the host's 1.2 to 1.9 ms. The maintainer's 8 to 10 ms `CPU` row was **not reproduced**:
  this lane reads 4.5 to 6.0 on both titles. See "What could still differ" below.

### Render scale: not the GPU

`target_fps = "off"` in a copy of the settings (no DRS), `--render-scale N`, same title and
tap. Render sizes are N % of the 3840x2160 window.

| Run | render | scene | submit | total | CPU row | gpu_ms | frame_ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Pulse paused, 50 | 1920x1080 | 0.87 | 3.16 | 5.60 | 4.63 | 3.7 | 11.1 |
| Pulse paused, 100 | 3840x2160 | 0.85 | 2.82 | 5.15 | 4.08 | 13.0 | 27.5 |
| Pulse paused, 150 | 5760x3240 | 0.81 | 2.47 | 4.60 | 4.13 | 29.1 | 55.0 |
| HD paused, 50 | 1920x1080 | 1.09 | 3.02 | 5.27 | 4.58 | 5.6 | 12.6 |
| HD paused, 100 | 3840x2160 | 1.02 | 2.86 | 4.98 | 4.40 | 17.0 | 31.4 |
| HD paused, 150 | 5760x3240 | 1.06 | 2.85 | 5.12 | 4.87 | 34.8 | 61.7 |

The GPU time rises 8 to 10 times across the range, and `scene`, `submit` and the `CPU` row do not
move (they fall slightly, the frame thread idling in the acquire instead). **So the Deck's CPU
row is not waiting on the GPU. It is per-frame CPU work that does not depend on pixels**, the
cost of wgpu-core and RADV recording and submitting the passes. (The racing runs of the same
sweep show the same flatness: CPU row 4.1 to 4.9 for Pulse, 4.4 to 6.2 for HD.)

### Why it is 4 to 5 times the host

`perf stat` on the process (Pulse, paused, 15 s, whole run including the load): 11.64 G user
cycles over 7.67 s of task-clock (user plus sys) is 1.52 GHz, over 6.17 s of user time alone
1.89 GHz, so **1.5 to 1.9 GHz effective** (a range because the process has loader and music
threads, and the load is in it), against the part's 3.5 GHz boost. The host (Ryzen 9 7900,
`lscpu` max 5.49 GHz, not measured under load) is 2.9 to 3.6 times that on clock alone, and its
core is newer than Zen 2. That, not a blocked call, is most of the 4 to 5 times. A second `perf
stat -D 9000` of the steady state gave 1.12 (Pulse) and 1.00 (HD) instructions per cycle. The
cost is per-frame and per-pass work in wgpu-core and RADV, independent of pixels; it is not
simply per draw (`submit` is 3.0 ms on Pulse's 321 draws and on HD's 617, where the host's
scaled 2.8 times), and the ablation below points at the post passes. `perf record -F 1999` (user cycles only: `perf_event_paranoid` is 2, no
kernel samples) of the steady-state paused thread: 54 % in `oag-game` (of which wgpu-core usage
tracking, `new_usage_scope`, `track::texture::update`, `new_pooled`, is the largest named
block, with the `Vec` and `ArrayVec` code under it; our own `oag_*` code is about 4 %), 27 % in
`libvulkan_radeon.so` (RADV command recording), 16 % in libc (`malloc`, `free`, `memcpy`). The
AppImage binary is stripped; symbols were resolved with `addr2line` against the unstripped
`dist/oag-game` of the same BuildID, so the names are good to the function, not to the line.

### Ablation: the post chain costs 1.3 to 1.9 ms of the 5.5

Paused, 50 % render scale, no DRS, 90 fps limit, one flag at a time (`submit` / `total`):

| Run | Pulse | HD |
| --- | --- | --- |
| maintainer's settings | 3.10 / 5.50 (3.17 / 5.60 repeat) | 2.96 / 5.16 (2.99 / 5.22) |
| `--shadows off` | 3.21 / 5.72 | 2.84 / 5.02 |
| `--reconstruction off` (SMAA) | 2.13 / 4.14 | 2.85 / 4.92 |
| `--motion-blur off` | 1.80 / 3.90 | 2.83 / 5.09 |
| `--screen-filter off` | 2.25 / 4.18 | 2.99 / 5.21 |
| all four off | 1.80 / 3.57 | 2.15 / 3.91 |

The repeat of the baseline differs by 0.07 ms in `submit`. On Pulse (SMAA, medium blur and the
`psp-3000` filter) turning off *any one* of the three drops `submit` by 0.9 to 1.3 ms and all
together no more, so the cost is not a sum of three passes: it looks like a step that three
chained post passes trip together (not investigated further). On HD only turning all off helps
(0.8 ms). Shadows (`blob`) cost nothing measurable.

### What could still differ on the maintainer's Deck

The meter reading 8 to 10 ms is about twice this lane's. Not measured, in the order of how well
the evidence supports it:

1. **A lower CPU clock than this run's.** This run was at 1.5 to 1.9 GHz effective, on AC, with
   the `powersave` governor and nothing else running. Steam's own client, a TDP cap or a
   background download in the maintainer's session could push it lower, and the frame thread
   scales with the clock. A 2x clock difference would be the whole gap. Not measured.
2. **A different profile or window.** The maintainer's figure is on a title and profile this lane
   did not ask for (the `Pulse (PS2)` profile has 4x MSAA and a `crt-aperture` filter).
3. **The build: ruled out.** The maintainer's install is
   `~/Desktop/OpenAntiGrav-x86_64-portable.AppImage` (`--version` bb68bbb, 27 commits before the
   merge this lane ran on, no `frame-phases` line). Copied into the lane directory (never run in
   place) and run interleaved with this lane's znver2 build, two runs each, 50 % render scale, no
   DRS, 12 s, CPU row from the frame-log (`cpu_ms - present_ms`), ms:

   | | portable, old | znver2, new |
   | --- | --- | --- |
   | Pulse racing | 5.64, 5.53 | 5.15, 5.26 |
   | HD racing | 5.16, 5.17 | 5.30, 5.20 |
   | Pulse, `3:start` tap | 4.97, 5.12 | 4.58, 4.67 (paused) |
   | HD, `3:start` tap | 4.77, 4.82 | 4.42, 4.50 (paused) |

   The old build reads 4.8 to 5.6 as well, so the build does not explain 8 to 10 ms. (On the old
   build the `3:start` tap left the stage name at `race`, so those two rows are racing frames:
   the pause rule differs, not investigated.)

To settle it, read `frame-phases` from the maintainer's own launch (`--frame-log` in the Steam
launch options) next to the meter, and `cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq`
while it runs.

### Ranked options (nothing changed in the code)

No blocking readback, per-frame buffer re-creation or validation layer turned up: `readback` is
0.03 to 0.05 ms and `submit` does not move with the GPU. The lever is the amount of recorded work:

1. **Fewer, cheaper post passes on the Deck.** The ablation above is 1.3 to 1.9 ms of a 5.5 ms
   frame, the cheapest lever and a settings decision (what the Deck's default profile enables),
   not a code change.
2. **Render bundles or a held scene for a paused frame.** Not attempted; see the host sections.
3. **Why does one post pass off remove the whole step on Pulse?** Worth a look in `oag-post`'s
   pass chain (intermediate targets, init actions); open.

### Commands run on the Deck (about 30 `ssh` calls; every run under `flock -o ~/.cache/oag/deck.lock`)

`~/oag-perf/deck-read/run.sh LABEL pulse|hd 12 "SCRIPT" [flags]` launched
`squashfs-root/AppRun <image> --no-audio --frame-log --race --autopilot --script SCRIPT [flags]`
with `XDG_CONFIG_HOME` etc. under the lane directory; `SCRIPT` was `12:quit` (racing) or
`3:start,12:quit` (paused). Render-scale sweep: `--render-scale 50|100|150` with `target_fps = "off"`.
Ablation: `--render-scale 50 --shadows off|--reconstruction off|--motion-blur off|--screen-filter
off`. Profile: `PREFIX="perf record -D 9000 -F 1999 -o F" ./run.sh ... 20 "3:start,20:quit"` and
`PREFIX="perf stat -e task-clock,cycles:u,instructions:u -o F" ./run.sh ... 15`. The lane
directory was removed afterwards; the maintainer's install and settings were not touched.

## HD Anulpha Pass on the Deck (2026-10-26, `deck-anulpha-submit`)

The maintainer's 60 fps on Anulpha Pass (vsync `smooth`, `frame_limit` 90, 1280x800) is the
frame thread passing 11.1 ms, so frames alternate one and two vblanks.

**Measure a moving race: `--no-intro`.** Without it, `--race` on HD plays the pre-race flyby
(`start_grid.vex`) for as long as the run lasts: ticks 0.06 to 0.08 ms, compose 0.01, total 7.5 to
8.4, a steady 90. With `--no-intro --autopilot` the race moves (ticks 0.9, compose 0.2) and
reproduces the maintainer's played numbers (submit 5.3 against 5.1, total 10.1 against 9.4,
fps 75, p99 19 to 20).

Native on the Deck, under the gaming-mode gamescope (`DISPLAY=:1`, `ENABLE_GAMESCOPE_WSI=1`,
`GAMESCOPE_WAYLAND_DISPLAY=gamescope-0`). Private `XDG_*` with a copy of the maintainer's
`settings.toml` (`pause_on_focus_loss = false`), release build (`-C target-cpu=znver2 -C
force-frame-pointers=yes`, Debian bookworm container). `--race --autopilot --no-audio
--frame-log --seed 1 --no-intro --script 30:quit`, plus `--track
'Data\Environments\15_Anulpha_Pass\track.vex' --mode single_race` for Anulpha. Two interleaved
runs each; the first four `frame-phases` lines are dropped and the rest averaged. Deck load 0.6
to 1.5, on AC. Ms per frame:

| Run | ticks | scene | submit | total | fps | p99 |
| --- | --- | --- | --- | --- | --- | --- |
| Anulpha, before | 0.90 | 2.92 | 5.33 | 10.12 | 75.5 | 19.8 |
| Anulpha, arrays hold used slots | 0.83 | 2.92 | 5.11 | 9.83 | 81.0 | 17.1 |
| Anulpha, plus pooled buffers | 0.84 | 2.94 | 4.70 | 9.49 | 82.8 | 16.1 |
| Anulpha, plus no shadow passes for culled hulls (other runs) | 0.78 | 2.84 | 4.49 | 9.09 | 85.5 | 14.3 |
| Anulpha, plus the trail-hit box reject (other runs) | 0.60 | 2.90 | 4.45 | 8.97 | 86.2 | 13.6 |
| Anulpha, plus split finish (other runs) | 0.50 | 2.92 | 3.47 | 7.97 | 90.0 | 11.3 |
| **Anulpha, before, 60 s runs** | 0.95 | 2.93 | 5.38 | 10.24 | 74.5 | 20.3 |
| **Anulpha, after, 60 s runs** | 0.59 | 2.96 | 3.13 | 7.76 | 89.9 | 11.65 |
| HD default circuit, before (other runs) | 0.23 | 1.61 | 2.75 | 5.59 | 90.0 | 11.2 |
| HD default circuit, after | 0.22 | 1.46 | 1.66 | 4.31 | 90.0 | 11.3 |
| Pulse default circuit, before (other runs) | 0.18 | 1.51 | 2.09 | 4.78 | 90.0 | 11.2 |
| Pulse default circuit, after | 0.16 | 1.54 | 1.26 | 3.89 | 90.0 | 11.2 |
| HD default circuit, before | 0.25 | 1.63 | 2.75 | 5.66 | 90.0 | 11.2 |
| HD default circuit, after the first two | 0.25 | 1.58 | 2.42 | 5.22 | 90.0 | 11.2 |
| Pulse default circuit, before | 0.19 | 1.63 | 2.14 | 5.00 | 90.0 | 11.2 |
| Pulse default circuit, after the first two | 0.15 | 1.39 | 1.72 | 4.19 | 90.0 | 11.2 |

### Where `submit` goes (Deck profile, `perf record -g --call-graph fp -F 999`)

`encode_render_pass` is about 40 % of the frame thread. Before the changes:

- per-element tracking of bound groups, about 13 %: `flush_bindings` 8.3 % (texture init actions
  for each element) and `merge_bind_group` 5.0 %;
- per-pass setup and teardown, about 8 %: wgpu-core sizes each pass's usage scope to the
  device's whole buffer and texture count, then clears it;
- `begin_render_pass` in wgpu-hal, 3.6 %;
- per-draw validation, the rest of `draw_indexed`.

A frame on Anulpha opens about 54 render passes.

### What changed (pictures byte-identical: HD Anulpha, HD and Pulse defaults, five HD weapons and two Pulse weapons live)

1. **A drawable's binding arrays hold only the slots its draws name**
   (`oag_mesh::mesh_render::bindless::used_slots`). The speed pads and weapon pads are built over
   the circuit's 593-slot material table, and each bound its 269 distinct textures to draw a few
   pads. Elements bound per frame: 1,370 to 842.
2. **Projectile pool slots sub-allocate one buffer per pool** (`Drawable::instances`). There are
   128 slots per weapon model, and each slot had its own uniform buffer and three animation
   buffers, so 6,235 buffers were alive. Every pass's scope was sized to that count. Now 907.
3. **No sun-occlusion or self-shadow pass for a craft whose hull is culled**
   (`Scene::hull_seen`, single-view frames only). Only that hull and its wreck read the two
   layers, and the test is the hull draw's own (`craft_cull::hull_in_view`, same frustum), so a
   skipped layer is one no pixel reads. Against the row above, in a separate pair of runs:
   total 9.51 to 9.09 ms, submit 4.75 to 4.49.
4. **The trail-hit test rejects by the ribbon's bounding box first** (`Tube::bounds`). Each tick
   tested about 10 hull anchors of every craft against every other craft's HD trail with
   `Tube::nearest` (53 segments each); a point farther from the ring's box than the reach plus a
   unit of slack cannot pass. Same answers. Ticks 0.79 to 0.60 ms.
5. **Split finish** (`oag_gpu::split_finish`). The race frame's encoder is closed after the
   early passes and after the race pass; a worker thread finishes each closed part while the
   frame thread records the rest, and the frame submits all of them in order in one
   submission. Total 8.94 to 7.97 ms, submit 4.44 to 3.47, 90 fps. `OAG_SPLIT_FINISH=0` turns it
   off for an A/B.

In the 60 s runs, 3 of 54 one-second windows after the change still read under 90 (89.x, a
single double-vblank frame each). Not traced.

### Why `submit` is the bottleneck, and what can be done

wgpu (`wgpu-core` 30) checks every command on the CPU. Rendering records passes, binds and draws
cheaply; `encoder.finish()` then replays them through `encode_render_pass`, which for each:

- **pass** builds a usage scope sized to every buffer and texture the device has alive
  (`UsageScope::new_pooled`, `set_size`), merges into it, and clears it at the end: cost per
  pass grows with the live object count, not with what the pass touches;
- **bound group** merges each element's usage (`merge_bind_group`, `merge_single`) and, when the
  group is flushed before a draw, registers each texture's init action (`register_init_action`,
  `check_action`, a linear walk over the frame's discarded surfaces);
- **draw** validates the pipeline, groups and vertex buffers (`is_ready`, `flush_vertex_buffers`).

The Deck's Zen 2 runs this at 1.5 to 1.9 GHz with a cache several times smaller than a desktop
part's, so the same frame costs 4 to 5 times the host.

What helps, measured here: fewer live objects (buffers 6,235 to 907), fewer elements per bind
(1,370 to 842), fewer passes (16 shadow passes skipped for culled hulls), and finishing parts of
the frame on another core. What would help further: fewer passes in the post chain (bloom
downsample 10 a frame, motion blur 7), shared 1x1 fallback textures (116 drawables make 11 each:
1,276 of 1,863 textures), and a third split point if the post chain grows.

What cannot be done in safe wgpu: turn the validation off. Bypassing it means `wgpu-hal`
directly or an unsafe passthrough, which gives up the safety the renderer relies on. A patched
`wgpu-core` works (`[patch.crates-io]`; this lane used one to count passes and objects), and
three changes look upstreamable:

- skip the init-action registration for a texture that is already fully initialised;
- make the usage scope's size and clear sparse instead of O(live objects);
- skip re-merging a bind group that is already merged in the same pass.

A fork is a maintenance cost on every wgpu upgrade, so upstream is the better route.

Measured and dropped:

- reusing `probes::mix`'s buffers, and eliding the repeated per-draw binds in the sun-occlusion
  pass: neither moved scene or submit on the Deck (scene 2.92 against 2.93);
- drawing each still advert card once: all seven of Anulpha's cards animate, so seven passes a
  frame remain;
- merging back-to-back draws whose index ranges touch: submit 4.48 to 4.40, total inside the
  run-to-run noise.

The binding-array and buffer counts come from `perf-probe` (`array_binds` on the `perf frame`
line) and from a local probe in a copy of wgpu-core (not committed). The probe counted usage
scopes and the tracker sizes at `Queue::submit`.
