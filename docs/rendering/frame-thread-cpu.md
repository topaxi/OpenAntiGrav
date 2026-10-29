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

**Measure unlocked (maintainer, 2026-10-27).** A CPU change on the Deck is judged with the frame rate
unlocked: `vsync = "off"`, the frame limit off or far above the panel, and gamescope's own framerate
limit not applied. Capped at the panel's 90 Hz, fps and p99 hold at the cap whatever the CPU does
(`cpu-governor.md`: p99 11.2 ms either way). A capped run is a second check, that the race still holds 90.
Under gamescope our settings are not enough: with `ENABLE_GAMESCOPE_WSI=1` (Steam's default) a race
with vsync off and no limit still presents at 90.0 fps, because gamescope's WSI holds the image. Launch
with `ENABLE_GAMESCOPE_WSI=0` for an unlocked run (`governor-unlocked`, 2026-10-27).

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

## Indirect multi-draw on the Deck: measured and dropped (2026-10-27, `gpu-driven-submit`)

Step 1 of the GPU-driven plan put HD's prepass depth half on one `multi_draw_indexed_indirect`
per view (commit `14bd57567`, reverted by `016546310`).

- **How it worked.** The circuit's opaque list was culled once in `Scene::prepare_view`.
- **What the list held.** That view's indirect arguments went into its own region of one
  buffer.
- **The depth half.** It is one pipeline with no texture read, so its 505 draws on Anulpha
  (tick 1500) became one call.
- **Path choice.** Vulkan and Dx12 took the indirect path; `OAG_INDIRECT=0` kept the per-draw
  path.
- **Pictures.** HD Anulpha (ticks 300 and 1500), the HD default and the Pulse default were
  byte-identical across the base build, the new path and the fallback.

**Why only the depth half.** The shaded draws read their material slot from an immediate
(`mesh_render::bindless`), and an immediate cannot change inside one multi-draw. Carrying the
slot per draw makes the texture-array index non-uniform. The bindless lane measured that at
6.5 to 16% more GPU time on ANV and NVIDIA.

**A standalone wgpu 30 program** (RX 7800 XT, RADV, load 13 to 25, 500 draws in one pass, indirect
validation on) put `finish` at 31 to 48 us with `draw_indexed` against 12 to 26 us with one
multi-draw, and `queue.submit` at 17 to 24 us against 18 to 28 us. So on paper this was a small net
win.

**The Deck says otherwise**: two runs each, 60 s, interleaved, the race and settings of the
section above, Deck load 1.2 to 2.4. Ms per frame:

| Run | ticks | scene | submit | total | fps | p99 |
| --- | --- | --- | --- | --- | --- | --- |
| base, split finish on (the default) | 0.56 / 0.50 | 2.98 / 2.86 | 3.14 / 3.15 | 7.76 / 7.58 | 89.9 / 89.9 | 11.67 / 11.61 |
| indirect, split finish on | 0.65 / 0.60 | 2.97 / 3.05 | 3.26 / 3.24 | 7.94 / 7.98 | 89.9 / 88.5 | 11.72 / 12.60 |
| base, vsync off, frame limit unlimited | 0.66 / 0.64 | 3.01 / 3.08 | 3.13 / 3.12 | 7.86 / 7.90 | 89.9 / 89.9 | 11.63 / 11.75 |
| indirect, vsync off, frame limit unlimited | 0.64 / 0.59 | 3.05 / 3.08 | 3.28 / 3.21 | 8.03 / 7.96 | 89.9 / 89.9 | 11.67 / 11.86 |
| base, `OAG_SPLIT_FINISH=0` (40 s) | 0.61 / 0.62 | 3.04 / 3.06 | 4.53 / 4.49 | 9.19 / 9.18 | 87.1 / 86.1 | 13.87 / 14.72 |
| indirect, `OAG_SPLIT_FINISH=0` (40 s) | 0.60 / 0.60 | 3.01 / 3.04 | 4.41 / 4.41 | 9.04 / 9.07 | 86.8 / 86.8 | 14.11 / 14.01 |

On one encoder it saves the 0.1 ms the bench predicts. With the split finish, which is what
ships, the race pass is finished on the worker, so the per-draw validation it saves comes off the
worker. Meanwhile wgpu's indirect-argument validation adds work on the frame thread: one
`StagingBuffer` and one source bind group per pass that carries indirect draws, every frame
(`wgpu-core` `indirect_validation/draw.rs`), freed later. The frame thread loses about 0.1 ms.

**The unlocked rows are not unlocked.** With `vsync = "off"` and `frame_limit = "unlimited"`,
gaming-mode gamescope still paced the Mailbox swapchain at 11.11 ms (`Swapchain received new
refresh cycle`), so fps stayed at 90. Compare the phase times; an unlocked fps on the Deck needs
another gamescope setting, not found here.

**What this means for any lever that cuts `finish` work in the race or shadow passes** (more
indirect draws, render bundles): it saves worker time, and the Deck's frame does not get
shorter unless the frame thread also gets cheaper. Profile of the base build (`perf record -g
--call-graph fp -D 15000 -F 999`, 30 s Anulpha, split by `--tid`). Ms per frame, 8.2 ms on the
frame thread in all:

| Where | ms per frame |
| --- | --- |
| frame thread, `Scene::prepare_view` | 3.4 |
| frame thread, `queue.submit` | 1.25 |
| ... of which `maintain`, dropping the finished encoders (`EncoderInFlight`, wgpu-hal `reset_all` 0.55, `destroy_buffer` 0.13) | 0.8 |
| frame thread, `finish` of its own parts (the post chain) | 1.0 |
| frame thread, `queue.write_buffer` | 0.63 |
| frame thread, `submit` phase waiting on the worker (3.5 ms phase less 2.3 ms on CPU) | about 1.2 |
| `oag-finish` worker, almost all `encode_render_pass` | 4.4 |

The frame thread does wait on the worker, but cutting the worker's race-pass work by 0.15 ms
did not shorten the wait measurably. Its own work is the larger lever:

- the encoder drop in `maintain` (fewer encoders a frame means fewer `reset_all` calls);
- `prepare_view` (`probes::mix` alone is 0.5 ms);
- the post chain's passes;
- the 51 `write_buffer` calls a frame.

A fourth upstreamable wgpu-core change: pool indirect validation's staging buffer and source
bind group across frames instead of making them per pass.

## What `prepare_view` spends on the Deck (2026-10-27, `prepare-view-breakdown`)

The per-thread profile above puts `Scene::prepare_view` at 3.4 ms of the frame thread's 8.2 ms on
Anulpha. This section splits it.

**Method.** Release build of `main` (`b7191aa76` code, `-C target-cpu=znver2 -C
force-frame-pointers=yes`, Debian bookworm container, `perf-probe`), native on the Deck under the
gaming-mode gamescope, maintainer's settings copy, `--race --autopilot --no-audio --frame-log --seed 1
--no-intro --script 30:quit`, `perf record -g --call-graph fp -D 15000 -F 999`. Samples are cut to the
frame thread by `tid`, then every sample whose stack holds `prepare_view` is charged to the function
called directly from it (inlined frames count, so a name is a source function, not a symbol). Ms
per frame take the Anulpha run's 8.2 ms of frame-thread CPU as 1,709 frames and use the same frame
count for the other two runs, which hold 90 fps for the same window. Shares are exact; the ms are
good to about 0.05. Each row is one run, no repeat. The first-order picture is large enough not to
need one, but a row under 0.1 ms is noise.

Frame-thread CPU and `prepare_view` in ms per frame:

| Race | frame thread | `prepare_view` | share |
| --- | --- | --- | --- |
| HD Anulpha Pass | 8.20 | 3.45 | 42% |
| HD default circuit | 5.43 | 1.68 | 31% |
| Pulse default circuit | 4.25 | 1.54 | 36% |

### Ranked table (Anulpha, ms per frame and share of `prepare_view`; the other two titles beside it)

| # | What | Class | Anulpha | HD default | Pulse |
| --- | --- | --- | --- | --- | --- |
| 1 | `write_hull_uniforms`: the hull's environment probes blended on the CPU every frame (`probes::mix` 0.45, the `Vec<Arc<Decoded>>`/`Vec` allocations and drops 0.31, texel decode 0.15) | per-object build, allocation | 0.96 (28%) | 0.23 | 0.00 |
| 2 | `render_shadow_passes`: sun occlusion 0.47, shadow map 0.17 (recording, `realloc`, bind group clones) | shadow-view setup | 0.64 (19%) | 0.23 | 0.00 |
| 3 | `render` called direct (advert cards, per-drawable `write_uniforms`, `write_buffer`, node animation) | uniform writes | 0.36 (10%) | 0.32 | 0.20 |
| 4 | Scenery animation sampling: `write_scenery_anims` 0.24, `write_node_anims` 0.10, `write_anims` 0.05 (`AnimTrack::sample` builds a `Vec` of 4x4 matrices) | per-object matrices, allocation | 0.39 (11%) | 0.44 | 0.30 |
| 5 | `extend_hd_trail_vertices` (HD trail ribbon geometry, `GpuVertex` copies) | vertex building | 0.23 (7%) | 0.04 | 0.01 |
| 6 | `upload_particles` 0.19 and `upload_ribbons` 0.19 (magstrip wake `Tube::nearest` 0.08) | vertex building, upload | 0.38 (11%) | 0.11 | 0.04 |
| 7 | `write_zone_vis` | uniform writes | 0.13 (4%) | 0.07 | 0.12 |
| 8 | `write_absorb_overlays` (Pulse shine: `write_view` `refresh` 0.31, environment texcoords 0.08) | per-object build | 0.00 | 0.00 | 0.57 |
| 9 | `upload_shadow_geometry` | upload | 0.01 | 0.01 | 0.11 |
| 10 | Everything else (airbrake deflect, bomb blasts, scene blocks, `Draw` `with_capacity` 0.04) | other | about 0.22 | about 0.16 | about 0.12 |

**Classes that cost almost nothing here:**

- **CPU culling.** `select_lod` is 0.008 ms. The PVS visible-set build is 0.018 ms (Pulse) and
  below the noise on HD. The frustum is built once.
- **Sorting.** There is no sort in the frame thread's profile of `prepare_view`; no stack reaches
  0.005 ms.
- **Between-ticks blending.** `MotionState::advance` is 0.005 to 0.011 ms. The between-ticks
  work is in the other rows, not a row of its own.

**Anulpha specific.** Rows 1 and 2 are Anulpha's extra 1.8 ms: the circuit ships baked environment
probes, so every hull blends two images per frame (the HD default circuit may ship none; not checked),. HD's default
circuit pays 0.23 ms for the hull uniforms and Pulse none. Row 4 is on every title and is the
largest on HD default.

**What the stack walk cannot say.** With frame pointers, the walk drops some inlined frames, so the
split inside a row (`mix` against the `Vec` drops) is good to a few hundredths of a millisecond. It
sees no GPU time and no wait.

### Verdict for the next drive

**GPU compute culling is not the lane that moves the frame thread.** Culling, LOD selection and
sorting are under 0.05 ms of the 3.45 in `prepare_view` on all three races. Compute culling would
remove draws from the render pass's `finish`, which the split finish already runs on the worker, and
the `gpu-driven-submit` lane showed that a worker saving does not shorten the Deck's frame. The
frame thread's `prepare_view` is made of uniform builds, uploads and a few CPU image blends, none of
which a culling pass touches. A CPU-side fix is cheaper and bigger. Top two:

1. **Blend the hull's environment probes on the GPU, or not every frame** (Anulpha-like circuits
   only, about 0.9 ms, 11% of the frame thread's 8.2). `EnvProbes::write` already skips an unchanged
   tap set, but the weights move with the craft, so it remixes each craft's two images
   every frame (decode, `Vec<u8>` accumulate, allocate, `write_texture`). Either upload the
   two or three source probes once and blend in the hull shader (the weights are a 3 to 4 element
   uniform), or keep the CPU blend and re-run it only when the weights move by more than a step.
   The first keeps the picture exact. It is outside `oag-core`'s determinism rules (render side).
2. **Cache the scenery animation matrices per tick and stop allocating in `AnimTrack::sample`**
   (every title, 0.30 to 0.44 ms, saves roughly 0.1 to 0.2 ms, 1 to 2% of the frame thread). The
   clock is the race tick, so at 90 Hz over 60 Hz ticks a third of the frames resample an identical
   value, and the matrices are built into a fresh `Vec` each call.

Next after those: the sun-occlusion pass recording (0.47 ms on Anulpha, bind group clones and
`realloc`), and Pulse's `write_view` `refresh` (0.31 ms). Not measured here: whether the mixed
probe images can be cached across the grid cell, the sun occlusion's own work per craft, and a
repeat run for noise.

### Commands

Per race: `PREFIX="perf record -g --call-graph fp -D 15000 -F 999 -o X.data" deck_run.sh TAG bin/oag
30 --no-intro [--track 'Data\Environments\15_Anulpha_Pass\track.vex' --mode single_race]` (`SRC=pulse-psp-eu.chd`
for Pulse), under `flock -o ~/.cache/oag/deck.lock`, then `perf script -F tid,ip,sym`, one frame-thread
`tid`, and a count of samples per direct callee of `prepare_view`. Files on the Deck were removed.

## The reported phone regression, 2026-10-22 against 2026-10-27 (`android-cpu-regression`)

The maintainer read the S24's perf meter `CPU` at about 9 ms on a 2026-10-22 build and 12 to 13 ms
on one of 2026-10-27 (before `between-ticks`), from the pre-race flyby on. The phone's own log names
the two builds, `e5c996527` and `78275344a`, both on HD/Fury Vineta K with the phone's settings
(HD: `fsr1`, motion blur `medium` at half resolution, shadows `original`, `target_fps` 60,
`frame_limit` 120, `cpu_governor` on). **Not reproduced; on the Deck the newer build is faster.**

**Steam Deck**, unlocked (`vsync = "off"`, `frame_limit = "unlimited"`, `ENABLE_GAMESCOPE_WSI=0`),
native gaming-mode gamescope, znver2 container builds (`build-appimage.sh --container --target-cpu
znver2`), HD Vineta K, `--race --autopilot --no-audio --frame-log --seed 1 --track
'Data\Environments\01_Vineta_K\track.vex' --mode single_race --script 30:quit` (racing adds
`--no-intro`), three interleaved rounds, first four lines dropped, medians, Deck load 1 to 2.
`total` ms per frame, three runs each:

| Scene | `e5c996527` (10-22) | `905e2feab` (main 10-27 plus the launch file) |
| --- | --- | --- |
| flyby, maintainer's Deck settings | 7.14, 7.22, 7.20 | 5.59, 5.52, 5.56 |
| racing, maintainer's Deck settings | 9.53, 9.55, 9.42 | 7.85, 7.65, 7.75 |
| flyby, the S24's HD profile at render scale 200 (GPU-bound, 68 to 71 fps) | 7.02, 7.07, 7.00 | 5.51, 5.51, 5.55 |
| racing, the same | 10.85, 10.88, 10.96 | 8.17, 8.32, 8.24 |

The phase that moved is `submit`, 5.0 to 5.8 ms down to 3.2 to 3.6: the `deck-anulpha-submit`
section above. `scene` and `ticks` hold within 0.15 ms.

**S24**, the same race through the launch file (`docs/tools/android.md`), 40 s, each run from a
battery at 27.0 C (it reads 29.5 to 30 C at the end), `--no-audio`. CPU row (`cpu_ms - present_ms`,
what the meter draws), median per run: main with the split finish 7.94 and 10.03, main with
`OAG_SPLIT_FINISH=0` 9.00 and 9.13, `e5c996527` 8.58 (one run; the maintainer needed the phone
back). The phone is GPU-bound in this scene (about 57 fps, `present` 8 to 12 ms) and the dynamic
resolution settled alike on both builds (GPU 8.7 against 9.1 ms). Within one split-finish run
`submit` swings between 3.5 and 7.7 ms from second to second, where without the split it holds at
about 6.4; the means agree. That looks like the finishing thread landing on different core classes
(the S24 reports `cpu_capacity` 253 on four cores, 653 on three, 730 on two, 1024 on one; the Deck
and a desktop Zen report 1024 on every core), but it is not a cost on this evidence. Nor is it a pacing cost:
the per-second `p99_ms` after the first four lines has a median of 19.41 and 18.77 ms with the split
and 18.64 and 18.54 without (worst 32.4, 21.6 against 21.5, 21.8; the old build's one run 19.94, worst
31.4).

The `acquire waited` figures in the maintainer's own log are 0.08 to 0.12 ms on both builds, so no
wait moved into the frame thread's accounting either. This host was not used: at load 20 to 60 the
same binary read 2.1 to 5.2 ms `total`.

### The maintainer's scene: the split finish costs the phone about 1 ms

The maintainer's scene was the HD campaign's first event (single race, Vineta K, Venom, weapons
on), read during the flyby and the first seconds after GO. Reached on the phone by taps from the
disc picker through RACE CAMPAIGN, the HD card, event 1 and Ship Select (Mirage on both builds),
`--frame-log` through the launch file, sound on, every run from a battery at 28.7 C, interleaved.
CPU row median over the first 20 race seconds (the flyby) and the 25 after:

| Run | flyby CPU row | flyby `scene` | flyby `submit` | after GO CPU row |
| --- | --- | --- | --- | --- |
| `e5c996527`, two runs | 9.82, 9.78 | 2.25, 2.27 | 7.27, 7.20 | 9.54, 9.59 |
| main, split finish on, two runs | 10.72, 10.72 | 3.33, 3.35 | 7.02, 7.06 | 8.70, 9.41 |
| main, `OAG_SPLIT_FINISH=0` | 9.72 | 2.46 | 6.81 | 8.81 |

The frame thread's `total` is 18.5 to 19.4 ms in every run; the phone is GPU-bound (50 to 53 fps in
the flyby) and the meter's CPU row is `cpu_ms - present_ms`, so time the split adds to `scene`
shows up as CPU while `present` shrinks. On the S24 the split adds about 1 ms of recording and
saves no `submit`, where on the Deck it saved 1 ms of `submit`. That is the measurable part of the
report; the rest of the 9 against 12 to 13 did not reproduce in interleaved, temperature-matched
runs.

**Fix:** the split is on only where every core reports the same `cpu_capacity`
(`oag_gpu::split_finish::wanted`); the S24 (253, 653, 730, 1024) runs one encoder, the Deck and a
desktop Zen (1024 on every core) keep the split. `OAG_SPLIT_FINISH=0`/`1` still overrides. On the
S24 that is exactly the `OAG_SPLIT_FINISH=0` row above.

## Between-ticks drawing on the Deck (2026-10-27, `deck-between-ticks-measure`)

Claim: the between-ticks drawing (`85e7f2b5d`) plus the mist blend (`2fd6f966d`) add no CPU to the
Deck's frame thread. **True.** `e9baf8c42` (`85e7f2b5d^1`) against `1a7e06314`, znver2 container
builds, HD/Fury Anulpha Pass (`--track 'Data\Environments\15_Anulpha_Pass\track.vex' --mode
single_race`), `--race --autopilot --no-intro --no-audio --frame-log --seed 1 --script 40:quit`,
unlocked (`vsync = "off"`, `frame_limit = "unlimited"`, `ENABLE_GAMESCOPE_WSI=0`), gaming-mode
gamescope on `:1`, surface 1280x800. Four runs per build interleaved old, new, old, new, 25 s
rest between, Deck load 0.5 to 1.4, on AC. First four lines dropped, median per run, then the
median of the four runs (range of the four in brackets), ms per frame:

| Build | scene | compose | submit | present | total | CPU row |
| --- | --- | --- | --- | --- | --- | --- |
| `e9baf8c42` before | 2.80 (2.77-2.83) | 0.21 | 3.43 (3.36-3.46) | 0.18 | 7.34 (7.21-7.44) | 7.36 (7.25-7.44) |
| `1a7e06314` after | 2.82 (2.81-2.83) | 0.21 | 3.42 (3.42-3.45) | 0.18 | 7.42 (7.33-7.42) | 7.45 (7.39-7.45) |

The differences (`scene` +0.02, `submit` -0.01, `total` +0.08, CPU row +0.09) sit inside the old
build's own spread (0.23 ms on `total`, which drifted down from 7.44 to 7.21 over its four runs,
so the Deck warmed or settled during the sequence). `ticks` 0.40 against 0.43. Both builds hold
about 117 fps unlocked. Every Deck core reports `cpu_capacity` 1024 (read from sysfs; the log
does not name the path), so both builds take the split encoder finish.

## The finish frame (2026-10-28, `finish-stutter`)

The maintainer reported a stutter when crossing the finish line on a Steam Deck (Pulse PSP,
CPU bound, 90 Hz). **Cause, measured on the desktop: `Session::build_endrace` ran on the frame
that crosses the line.** It opened the disc image again (`open_source`), read
`EndRace_Definition.xml` and the sprite sheet (`oag_game::endrace::load`), uploaded the sprites
(`Renderer::new`) and, in a campaign cell, built the trophy previews (`Preview::new`). None of it
depends on how the race went. The frame's `prep` phase carried all of it.

`--race` never shows this (no shell, so no EndRace flow is built). The runs below go through the
menus: headless gamescope on the RX 7800 XT, release build with line tables, Pulse PSP EU,
`--autopilot --no-audio --no-intro --frame-log`, **vsync off and `[display] frame_limit =
"unlimited"`** (the race frames read about 1,100 fps; the keys sit under `[display]`, at the top
level of `settings.toml` they are ignored), `OAG_PHASE_SPIKE_MS=0`. That switch is new: it logs
one `frame-spike` line (at `info`) per frame over the limit, with that frame's own phases and the
race tick; `frame-phases` only averages a second. Use `OAG_PHASE_SPIKE_MS=11` to see only frames
over the Deck's 90 Hz budget.

```sh
OAG_PHASE_SPIKE_MS=0 gamescope --backend headless -W 1280 -H 800 -w 1280 -h 800 -r 60 -- \
  oag-game data/images/pulse-psp-eu.chd --autopilot --no-audio --no-intro --frame-log \
  --script "3:start,6:start,12:cross,18:cross,24:cross,32:cross,40:cross,48:cross,165:quit"   # Single Race, campaign cell
# Time Trial through RACEBOX (the race page opens on MODE = Time Trial), quit at 215 s:
#   --script "3:start,6:start,10:down,14:cross,20:down,22:down,24:down,26:down,28:down,32:cross,40:cross,48:cross,215:quit"
```

The finishing frame is the first `frame-spike` line after `race finished` in the log. The window
is the race ticks from 60 before to 60 after the finishing tick (about 1 s of simulation either
side, 1,800 to 2,800 frames). Before and after runs alternated, three each, load average 0.6 to
1.6 at every start and end.

| Race | Finishing frame, before (`total`, of which `prep`) | After | Steady median | Frames over 11.1 ms in the window |
| --- | --- | --- | --- | --- |
| Single Race (campaign cell, with trophy) | 64.1, 63.9, 64.1 ms (`prep` 61.5) | 2.37, 2.28, 2.49 ms (`prep` 0.6) | 1.1 to 1.2 ms | 1 before, 0 after, every run |
| Time Trial (RACEBOX, no trophy) | 42.5, 42.1, 41.9 ms (`prep` 41) | 1.21, 1.19, 1.20 ms (`prep` 0.26) | 0.65 ms | 1 before, 0 after, every run |

An earlier set at the default 240 fps cap read 63 to 125 ms for Single Race and 51 ms for Time
Trial on the same code; the spread is the page cache of the disc image (a first open after other
work reads slower). The cause and the fix are the same. Over the whole race the only frame above
11.1 ms besides the finishing one is the first race tick of a Single Race (184 to 192 ms, before
and after alike: the loading hand-off, not the finish; Time Trial has none). The prefetch added
no frame over budget anywhere in the race.

Where the before time went on the finishing frame (one instrumented run at the 240 fps cap,
Single Race): 6 ms gathering inputs, 44 ms `open_source`, 4 ms reading the screens, 2 ms
`Renderer::new`, 69 ms trophy `Preview::new`. Time Trial: 44 ms disc open, 4 ms screens. Both
scale with the CPU: expect several times that on the Deck.

**The fix.** `Session::start_endrace_prefetch` starts an `oag-thread` task on the race's first
frame (`session/endrace_prefetch.rs`). It does the disc open, the screen read, `Renderer::new`
and, for a campaign cell, the three trophy previews, and returns an `EndRacePrep`. The finishing
frame only joins it (`take_endrace_prep`) and assembles `EndRaceRuntime` with the results. If the
worker has not finished the frame skips and retries (the race draws its scoreboard meanwhile); if
it panicked or never started the build runs inline as before, so nothing is lost, only slower.
**Not on the web**: wgpu's web types belong to the page's thread (the race build is inline there
too), so a browser build still builds the screens on the finishing frame, as before; the
`wasm32-unknown-unknown` check builds. A structural test
(`the_finishing_frame_reads_no_disc_and_builds_no_gpu_resource`) fails if `build_endrace` calls
any of those again.

**What was compared.** Race End Photo, Results, Rewards and the menu were each seen on the new
build (screenshots on llvmpipe in a private Xvfb); only Rewards was put beside the old binary,
and the two read the same (Gold medal, loyalty row). The trophy mesh was built in both and
neither frame shows it (it is held at its first frame).

**Not the cause, measured.** The result capture (`records::save`, ghost save, tournament fold)
costs 0.12 to 0.14 ms on the finishing frame and a ghost write 0.5 to 0.6 ms at a lap crossing;
the two loyalty saves in `build_endrace` are the same kind of write (`std::fs::write`, no fsync).
They stayed on the frame thread then; since 2026-10-29 they go through `records::save_in_background`
(see "The save on the finish frame" below). `ticks` is 0.2 to 1.5 ms across the finish.

### The save on the finish frame (2026-10-29, `cursor-pure-2048-saves`)

The finishing frame's result capture (`records::save`) and the endrace loyalty saves were the last
inline writes on the frame thread; the saves of race results, loyalty and `escape.rs` now go through
`records::save_in_background`, as the campaign cursor and difficulty do. Same method as above
(headless gamescope, release with line tables, Pulse PSP EU, vsync off, unlimited frame limit,
`OAG_PHASE_SPIKE_MS=0`, `--autopilot`), but walked with `--drive` (the `--script` seconds no longer
reach a race with the built-in `oag-extras` rows): Single Race on the campaign grid's default cell.
Before is the same tree with the three call sites put back to an inline `records::save`; the runs
alternate, load average 10 to 12.

| Run | Finishing frame `total` (`ticks`, `prep`) |
| --- | --- |
| before 1, 2, 3 | 0.82 (0.25, 0.56), 0.89 (0.27, 0.62), 5.59 (2.36, 3.24) ms |
| after 1, 2, 3 | 0.69 (0.11, 0.57), 0.76 (0.15, 0.61), 0.92 (0.17, 0.75) ms |

The save was 0.12 to 0.14 ms on this host's SSD, so the win is small here (about 0.13 ms in the two
quiet pairs) and the one 5.59 ms before-frame is the write meeting a busy disk; that is the case the
change is for (a slower Deck SD card or a contended drive). Not measured on the Deck.
`records::load` writes a copy still waiting before it reads, so a read right after a save sees it.
**At exit** the process must join the last background write: `finish_background_saves`
(`main/app.rs` on exit, `main/drive.rs` at the end of a `--drive` run); any new exit path calls it.

**Limits.** (1) `--no-audio` is the null output backend: the mixer, cues and music fetch still
run, and the frame thread's `ticks` phase shows no spike, so the audio work on the frame thread is
not the cause here. A real device's callback thread competing with the new worker was not
measured. (2) The worker builds the screens in the language live when the race starts (as a parked
race's HUD already does); a LANGUAGE change from the pause menu shows at the next race. (3) The
worker costs one thread's time early in the race (about 100 ms of CPU on this host, more on the
Deck). (4) The Deck was not touched: re-measure it with a throwaway profile (`XDG_*` under a scratch
directory, one `--script "8:quit"` launch to write `settings.toml`, then `[display] vsync = "off"`
and `frame_limit = "unlimited"`), `OAG_PHASE_SPIKE_MS=11 --frame-log --autopilot`, audio on, the
menus walked by hand to a race, and read the first `frame-spike` line after `race finished` in
that profile's log.
