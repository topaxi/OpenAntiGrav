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

### Re-measured under an interleaved split, with the colour half too (2026-11-04)

The split finish now puts each view's split-off scene pass back on the frame thread
(`split-screen.md`, "Split finish under an interleaved split"), so the parked depth half was
ported to that main and measured again, with a measure-only colour half on top. Neither
is on main: the port is commit `aa54e2afa`, the colour half `fc967bd1a`.

- **Depth half, ported.** Main's depth half now switches to the culling twin per draw, so a
  view's list is two runs (two-sided, then culled), each in the depth order. On Anulpha the
  culled run is empty: one multi-draw per view, 1,042 draws in two views.
- **Colour half, measure-only** (`OAG_INDIRECT=colour`). The shaded half goes out as
  multi-draws too, each draw's material slot as its first instance. A third mesh shader
  variant (`indirect_material`) reads the slot's row from a storage buffer at group 1
  binding 2, so the texture-array index is non-uniform. The list keeps the per-draw
  order, backwards with reversed indices, split only where the cull mode changes, so
  every coplanar tie under `LessEqual` resolves as before. Bucketing by material would
  reorder those ties.
- **Pictures.** Byte-identical across `OAG_INDIRECT=0`, `1` and `colour`: HD Anulpha at
  ticks 300 and 1500, one view and a two-player vertical split, Pure's Vineta K split, and
  Pulse's `16_Track`. Pure and Pulse circuits take the depth path too, since they have a
  prepass.

**Deck, HD Anulpha Pass, `--render-scale 100`.** Dist build (znver2, Debian container),
headless gamescope at the window size, `vsync = "off"`, `frame_limit = "unlimited"`, a
36 s race, mean after the first 6 s. One binary, the arms interleaved, 2 rounds, Deck load
0.8 to 2.5. Frame-thread `total` ms, GPU ms where the log has it (one view only):

| Views, split finish | Size | none | depth | depth + colour | GPU none / depth / colour | fps none / colour |
| --- | --- | --- | --- | --- | --- | --- |
| 2, on | 1280x800 | 12.55 | 12.80 | 12.72 | - | 78.4 / 77.3 |
| 2, on | 1920x1080 | 14.79 | 14.89 | 14.72 | - | 66.2 / 66.3 |
| 2, on | 3840x2160 | 19.14 | 19.09 | 18.79 | - | 26.6 / 26.2 |
| 2, off | 1280x800 | 12.24 | 12.32 | 12.01 | - | 80.2 / 81.7 |
| 2, off | 1920x1080 | 14.30 | 14.25 | 14.01 | - | 67.5 / 67.9 |
| 2, off | 3840x2160 | 19.00 | 18.57 | 17.68 | - | 26.6 / 26.3 |
| 1, on | 1280x800 | 7.19 | 7.32 | 7.22 | 3.58 / 3.59 / 3.65 | 128.5 / 127.7 |
| 1, on | 1920x1080 | 8.25 | 8.40 | 8.25 | 5.50 / 5.53 / 5.64 | 86.4 / 85.4 |
| 1, on | 3840x2160 | 11.81 | 11.74 | 11.13 | 16.04 / 16.13 / 16.52 | 29.6 / 29.2 |

Round-to-round spread of a paired delta is about 0.1 to 0.3 ms in the split and 0.05 in
one view below 4K. A depth-only matrix of 3 rounds on the first binary agreed: in the
split with the split finish on, depth against none was -0.08, +0.05 and +0.12 ms at the
three sizes, all inside the spread; one view lost 0.08 to 0.10 ms below 4K and gained 0.2
at 4K; Pure's split lost 0.16 ms.

What it says:

**The resolvable floor is about 0.3 ms in a split.** The same depth-against-none cell
(800p, split, split finish on) read -0.08 in one session and +0.25 in the other.

- **Depth alone does not pay in a split.** Every split delta is inside that floor.
- **The colour step against depth alone:** 0.08, 0.17 and 0.30 ms less frame thread in the
  shipped split (split finish on). That is at or under the floor; the 800p one ranges -0.27
  to +0.11. With the split finish off, where all of `finish` runs on the frame thread, it is
  0.24 to 0.89 ms, which clears the floor at 4K.
- **Against none, the combination is worse at 800p in the shipped split:** +0.17 ms (+0.11
  to +0.22 over rounds). At 1080p it is -0.07 and at 4K -0.35, both inside the floor.
- **It costs GPU time.** The colour arm adds 0.07, 0.14 and 0.49 ms of GPU a frame at the
  three sizes, 2 to 3%, against none. That bundles the non-uniform material index, a
  storage-buffer row load per fragment and wgpu's GPU-side validation of 1,042 more
  indirect draws (the depth step alone costs 0.01 to 0.10), so it is an upper bound for
  non-uniform indexing on RADV VANGOGH. At 4K, where the frame is GPU-bound, fps drops
  (29.6 to 29.2 in one view, 26.6 to 26.2 in the split) and p99 rises up to 1.1 ms in one
  view.
- **The largest frame-thread cut is the split finish off plus the colour half:** 12.01
  against the default 12.55 ms at 800p, 14.01 against 14.79 at 1080p. That is mostly the
  split finish itself (off is 0.3 to 0.5 ms cheaper in a split below 4K, as
  `split-screen.md` measured on Pure).

**Verdict:** not worth shipping as built. On this path the frame-thread saving and the
non-uniform index come together. The saving is collapsing about a thousand shaded
`draw_indexed` calls into multi-draws, and inside a multi-draw the material slot can only
be per-draw data. Bucketing by material would avoid that, but reorders coplanar ties under
`LessEqual`, so no known variant gets one without the other. The dynamic-offset material
buffer ("Material data in one buffer" in the cpu-bound-devices handover) is a separate
lever: still one bind and one draw per draw, with an unmeasured saving.

Commands (one binary for all three arms; one run per arm):

```sh
# workstation: Deck binary from the worktree
podman run --rm -v <worktree>:/src -v data/appimage/container:/cache -w /src \
  -e CARGO_HOME=/cache/cargo -e CARGO_TARGET_DIR=/cache/target-indirect \
  -e RUSTFLAGS="-C target-cpu=znver2" oag-appimage-build:bookworm \
  cargo build --profile dist -p oag-game --no-default-features --features extensions,dev-cli
# Deck, under flock -o ~/.cache/oag/deck.lock, for W H, VIEWS (add --players 2
# --split-cut vertical), OAG_INDIRECT in 0 1 colour and OAG_SPLIT_FINISH in 1 0:
env ENABLE_GAMESCOPE_WSI=0 OAG_INDIRECT=$I OAG_SPLIT_FINISH=$SF \
  gamescope --backend headless -W $W -H $H -w $W -h $H -r 90 -- \
  ./oag-game ~/.local/share/oag/images/hdfury-ps3-eu-dec.iso --race --mode single_race \
  --seed 1 --track 'Data\Environments\15_Anulpha_Pass\track.vex' --autopilot --no-audio \
  --no-intro --frame-log --render-scale 100 --script "36:quit"
# mean of the frame-phases and frame-log stage=race lines after the first six
```

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
quiet pairs). The 5.59 ms before-frame is an outlier with both `ticks` and `prep` raised and no
measured cause (machine contention is the likely reading, not shown). The change matters where a
write is slow (a Deck SD card, a contended drive); not measured on the Deck.
Every after-run's profile held `[[records]]` (the finishing frame's save), `[[campaign]]` and
`[[loyalty]]` (the end-of-race saves) and the launch's `[[campaign_cursor]]` once the process exited, so no
save was lost. `records::load` returns the newest copy handed to the background writer (no write, no disk wait), so
a read right after a save sees it.
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

## Outpost 7 White race start on the Deck, from the menus (2026-10-31, `outpost7-deck-repro`)

The maintainer reported a very noticeable stutter or hang on the race start of Outpost 7 White
(`07_Track`) on Pulse PSP. **Not reproduced.** Talon's Junction White (`16_Track`) was the control.

Method: Steam Deck, gaming-mode gamescope (`DISPLAY=:1`, Steam's `GAMESCOPE_*` env), release
AppImage built from main `1c1451296` (`just appimage-deck`), Pulse PSP EU, audio on. The menus were
walked by `--script` (timed taps: START twice, RACEBOX, MODE right x3 to SINGLE RACE, five downs to
START, 22 downs to Outpost 7 White or none for Talon's, CROSS twice for the track and the ship) with
`--unlock-all` so the circuit is in the list. A copy of the maintainer's `settings.toml` and
`music.toml` in private `XDG_*` dirs (Pulse PSP profile: `weather = "enhanced"`, shadows `mapped`,
msaa 4x, motion blur `high`, FSR 1), only `pause_on_focus_loss = false` changed, and `[race] track`
reset so the picker starts at the top. The movie, soundtrack and DLC caches were copied in
(a cold movie cache transcodes the intro and holds the boot in `loading`). `--frame-log`, a race
started with the intro flyby left to play, `--script ...:quit` 70 s after the ship pick (load, 25 s
flyby, countdown, GO and about 35 s of racing).

| Variant | Circuit | WARN frames (over 40 ms) from the load onward | Worst per-second p99 in the race |
| --- | --- | --- | --- |
| vsync off, no limit, weather enhanced, cold then warm Mesa cache | Outpost 7 White | 0 and 0 | 14.4 and 11.6 ms |
| same, weather original | Outpost 7 White | 0 and 0 | 11.8 and 16.1 ms |
| same, weather enhanced | Talon's Junction White | 0 and 0 | 22.3 and 22.3 ms |
| same plus `--autopilot` (the ship flies through GO and the start line) | Outpost 7 / Talon's | 0 / 0, 0 / 0 | 22.3 / 22.3 ms |
| maintainer's own vsync `smooth`, `frame_limit` 90, `--autopilot`, cold then warm | Outpost 7 White | 0 and 0 | 11.2 ms |
| page cache evicted for the image, extracted data, DLC and caches (`posix_fadvise DONTNEED`) | both | 0 | 11.2 ms |

The load (press on the ship to the first race frame) takes 1.0 s to `CraftsBuilt` and the first
race frame about 1.6 s after the press, on a worker, with the loading screen drawing at 90 fps.
No `frame-spike`, soundtrack or first-draw stall appears at the flyby, the countdown or GO. The
2026-10-19 off-thread soundtrack start has not regressed: `race music started in 25 us`.
A Mesa shader cache emptied before the run changes nothing, so no first-draw pipeline on RADV is
involved for this circuit with these settings.

What the runs do show, on every circuit and unrelated to the race start: the menus run slow frames
on the Deck. Entering the track picker costs one 266 ms frame, the ship screen one 163 ms frame,
and every Down step in the picker one 66.6 ms frame (4 vblanks at 60 Hz, so a wait rather than
CPU work; the frame thread was not on-CPU in a `perf record` of the scroll, which the stripped
AppImage could not symbolise). A player scrolling to a late circuit such as Outpost 7 White
(index 22 of 32 in the list) sees 22 of them. It is the one thing on the way to this race that
could be felt as a stutter. **Not measured:** a race started from the campaign grid (it goes
through the same `launch_race`, with the campaign's own AI field and rules), and the Black layout.

## Pressing Cross during the pre-race flyby on the Deck (2026-10-31, `flyby-skip-stall`)

Claim tested: "pressing X during the Outpost 7 White flyby stalls the game for a second or more".
**Not reproduced.** Steam Deck, gaming-mode gamescope, release AppImage of `1972e2d` (and the
2026-10-24 Deck build), Pulse PSP EU, audio on, the maintainer's settings copied (vsync off, no
limit), menus walked with `--script`, Outpost 7 White (22 Down steps from Talon's) and Talon's
Junction White as control, Deck load average 1.3 to 2.1.

| Press | Circuit | Flyby ended after | Frames over 40 ms from the press to GO+5 s | Worst per-second p99 after |
| --- | --- | --- | --- | --- |
| 1.5 s into the flyby (mashed 8 times) | Outpost 7 White | 68 ticks | 0 | 11.3 ms |
| 3 s | Outpost 7 White | 191 ticks | 0 | 11.2 ms |
| 5 s | Outpost 7 White | 240, 343 ticks | 0 | 11.3 ms |
| 3.5 s, 5 s | Talon's Junction White | 243, 333 ticks | 0 | not parsed |
| Square, Circle, Triangle, Start (4 runs) | Outpost 7 White | no skip | 0 | n/a |

The skip is a held Cross read in `Race::tick_intro` (`IntroCamera::step`), so it runs the same
code as the flyby running out. Nothing it calls waits on a worker: the soundtrack fetch applies
on a later tick (`Audio::finish_race_music` never waits), and the race load finished before the
flyby began (`CraftsBuilt` after about 940 ms). The same `--race --track` path on the lavapipe
software renderer, pressed at 2 s, 7 s and 21 s into the flyby, shows no frame at the press above
its neighbours either.

What the runs do show, on a fresh profile only: the first entry to the track picker costs one
frame of 0.92 to 1.06 s (the profile has no cache yet). With the profile kept, the second and
third runs cost 271 ms (the figure of the 2026-10-30 section). A maintainer's warm profile
therefore sees 271 ms, not a second, unless an update invalidates that cache; **what that cache
is has not been identified**. Not tested: a touch press (`pointer.clicked` calls `skip_intro`)
and a real held press on the Deck's own controls.

## The speed plans off the frame thread (2026-10-31, `speed-plan-off-frame`)

Claim: "no speed plan is learned on the frame thread: a menu-started Pulse Outpost 7 race's
first ticks contain no frame over the control circuit's bound, and the plans the race drives are
bit-identical to today's".

Cause (the maintainer's Deck log, reproduced here): `Race::ensure_speed_plans` ran at the head of
the first `Race::tick`. Outpost 7 (`07_Track`) has a fork, so it learns the ring, route and ten
hull plans even under the original driver (`needs_speed_plans`), 1.3 to 1.7 s of one frame.
Earlier lanes measured other paths: the flyby does not tick the race (`tick_intro`), so the cost
lands at the end of the flyby or on the first tick under `--no-intro`.

Fix: `Race::begin_speed_plans` after the launch's last setter (`session/load.rs`), a `speed-plans`
thread on a copy of `RaceSim`, the loading screen's hand-over waits for it. Starting the job
takes 0.13 ms (the copy), adopting it 25 us; learning on the frame thread took 1.34 to 1.74 s
(release-like debug test run, `plans_learned_ahead_are_the_plans_the_first_tick_learns`).

Method: release build, Xvfb with lavapipe, `--no-intro --mode single_race --class rapier
--track 'Data\Environments\07_Track\track.vex' --measure-race-load 1`, Pulse PSP EU, 3 s of
race frames. The machine was contended (load noted per row), so only the large effect is read.

| Binary | Circuit | Load average at start | Worst race-stage frame | Loading frames |
| --- | --- | --- | --- | --- |
| main (before) | Outpost 7 | 7.9 | **1549.0 ms** (first tick) | 456 |
| branch (after) | Outpost 7 | 14.5 | 481.9 ms (once, frame 141 of the race, not a plan: no plan line follows the first tick) | 783 |
| branch (after) | Outpost 7 | 19.6 | 76.9 ms | 781 |
| branch (after) | Outpost 7 | 7.6 | 50.0 ms | 639 |
| main | Talon's Junction (control) | 7.9 | 37.3 ms | 460 |
| branch | Talon's Junction (control) | 6.9 | 40.0 ms | 457 |

The loading screen is held about 1.3 s longer on Outpost 7 (plans now finish before the swap, on
a thread, the screen drawing). One loading frame of 880 ms appeared in one of five after-runs
(load 7.6); the others read 156 to 171 ms, as before. The plan thread and its per-hull threads share the
cores with the loading frame, so a Deck with fewer cores may show a slower loading bar.
HD's default track (original driver, no fork): `needs_speed_plans` is false, the job is not
started, and neither binary logs a plan line (`--mode single_race`, HD EU); load numbers there
are cold-cache noise (8.5 s first run against 0.6 s) and not a comparison.

Deck confirmation, for the maintainer: a build of this branch, the same menu route to Outpost 7
White with `--frame-log`: the `WARN frame: 1639 ms` after `ai speed plan` lines should be gone and
the plan lines should print during the loading screen.

## The speed plans started on the race-load thread (2026-10-31, `speed-plan-overlap`)

Claim: "on a menu-started race on Pulse Outpost 7 (`07_Track`) and HD Talon's Junction, the
total time from confirming the race to its first tick drops on an idle machine, with the speed
plans still bit-identical and never relearned on the first tick". **True for Outpost 7; HD
Talon's Junction learns no plans** (no fork, the original driver flies: no `ai speed plan`
line before or after), so it is a control and stays unchanged, as does Pulse Talon's Junction
(`16_Track`).

Cause: the previous section's job started after the race-build thread joined, so it
overlapped nothing and Outpost 7's loading screen grew by the whole learn. Fix
(`docs/gameplay/ai.md`, "Started on the race-load thread, overlapped"): on a forked circuit
the race-load thread starts the race and the plans as soon as the circuit is read, so they
learn beside `build_race_stage` and `warm_up`; the job is kept by its plan inputs, not the
state hash, so the menus' setters keep it; the learner runs routes and hull ring copies at
once.

Method: release, Xvfb `:91` with lavapipe, Pulse PSP EU and HD EU, `--no-audio --no-intro
--mode single_race --measure-race-load 6` (`--class rapier --track
'Data\Environments\07_Track\track.vex'` or `16_Track`; HD its default circuit), the first
of each process's six races dropped (cold cache), two processes per binary interleaved
before/after: 10 races per row. `--measure-race-load` now prints "confirm to first race
frame" (from the press to the end of the first race frame, the first tick). The machine
was otherwise idle (load 0.6 between runs); the load average at each process start, 2.6 to
15, is the previous process's lavapipe threads. Before is main `1633cbb39` plus that
probe line.

| Circuit | Binary | Confirm to first race frame, median [range] | Loading worst frame | First race frame |
| --- | --- | --- | --- | --- |
| Outpost 7 (`07_Track`) | before | 5840 ms [5484-5949] | 100.5 ms [73.2-118.4] | 45.5 ms |
| Outpost 7 (`07_Track`) | after | **4312 ms** [3993-4435] | 102.9 ms [73.1-111.4] | 44.0 ms |
| Talon's Junction (`16_Track`, no job) | before | 4325 ms [3995-4431] | 89.3 ms | 30.3 ms |
| Talon's Junction (`16_Track`, no job) | after | 4286 ms [3956-4424] | 87.1 ms | 29.9 ms |
| HD Talon's Junction (no job) | before | 5358 ms [5014-5471] | 179.2 ms | 196.4 ms |
| HD Talon's Junction (no job) | after | 5348 ms [5010-5504] | 178.1 ms | 194.2 ms |

Outpost 7 now loads as fast as a circuit without a fork: the plans finish inside the scene
build (3.5 s of `build_race_stage` on lavapipe). The plan lines are the same set before and
after; no run logged a relearn or a restart. A first build also started the race on the
load thread for a circuit without a fork, which only added a copy of the setup there: HD
Talon's Junction measured 5488 against 5339 ms, so the load thread now starts it only on a
fork. The loading screen's worst frame on Outpost 7 read 77 against 101 ms median in the
first two rounds and 100 against 103 ms in these, so any cost of the plan threads sharing
the cores with lavapipe is inside the noise here; the Deck's four cores are not measured.

The learner alone (`tests::speed_plan_overlap`, release, `--no-capture`, 5 runs each, load
7 to 9): Outpost 7 1.56 s against 1.17 s overlapped; 2048 `altima` 1.91 s against 1.28 s.
The "before" there is `learn_speed_plans_serially`, main's learner rebuilt in the test (the
routes one after another, then each hull on a thread), not main's binary; the link to main
itself is the identical plan lines of the end-to-end runs.

## The race box's selection screens on the Deck (2026-10-31, `deck-menu-stalls`)

Claim: "walking the menus from a fresh profile to a race on the Steam Deck (main menu, Track
Select, 22 Down steps, Ship Select) has no frame over 50 ms caused by our code". **True**: after
the change no frame from the menus opening to the quit on the ship screen passes 36 ms (worst at
3840x2160) or 18 ms (1280x800 and 1920x1080); before, every walk had 24 such frames.

Method: Steam Deck, gaming-mode gamescope (`DISPLAY=:1`, Steam's `GAMESCOPE_*` env) for 1280x800,
a headless `gamescope --backend headless -W W -H H -w W -h H` for 1920x1080 and 3840x2160, Pulse PSP
EU, audio on, the maintainer's settings copied with `vsync = "off"` and `frame_limit = "unlimited"`,
`[race] track` reset to `16_Track`. The walk is the earlier sections' `--script` (START twice,
RACEBOX, MODE right x3, five downs, CROSS into Track Select at 19 s, 22 Downs 0.4 s apart, CROSS
into Ship Select). "Fresh" is new `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME` (the
movie, audio and DLC caches hardlinked in); "warm" re-runs the same profile. `--frame-log` with
`OAG_PHASE_SPIKE_MS=0` prints every frame's phases; the worst `total` between one script tap and
the next is the transition's frame. Before is the main AppImage `5a1373f`, after is this branch on
`d3a7a8e6d`; the walks interleave. Deck load average 0.7 to 1.4.

| Output | Profile | Track Select entry | Ship Select entry | Down step, worst of 22 | Down step, mean |
| --- | --- | --- | --- | --- | --- |
| 1280x800, before (2 walks) | fresh | 1064 to 1115 ms | 167 to 170 ms | 71 to 73 ms | 70 ms |
| 1280x800, after (2 walks) | fresh | **16.5 to 17.3 ms** | 12.4 to 13.0 ms | 15.0 ms | 13.2 ms |
| 1280x800, before | warm | 288 to 290 ms | 169 ms | 71.5 ms | 70 ms |
| 1280x800, after | warm | **15.3 ms** | 12.3 to 12.9 ms | 15.1 ms | 13.4 ms |
| 1920x1080, before / after | fresh | 1064 / **16.7 ms** | 169 / 12.5 ms | 71.8 / 14.9 ms | 70 / 13 ms |
| 1920x1080, before / after | warm | 280 / **14.8 ms** | 173 / 14.9 ms | 72.2 / 14.8 ms | 70 / 13 ms |
| 3840x2160, before / after | fresh | 1141 / **35.6 ms** | 182 / 20.6 ms | 83.0 / 25.3 ms | 74 / 22 ms |
| 3840x2160, before / after | warm | 291 / **34.2 ms** | 179 / 20.5 ms | 85.0 / 24.4 ms | 73 / 22 ms |

Attribution. The frame-phase clock put every slow frame in the `ticks` phase (the stage update),
not in submit or present, so the 66.6 ms Down step was CPU work on the frame thread, not a
vblank wait (the 2026-10-30 section guessed a wait from its length). `perf record` on the Deck
(`perf_event_paranoid` 2, unstripped binary of the same BuildID in the build-id cache) named
`DiscImage::open` and the CHD map's LZMA and bit reader as the frame thread's top own symbols,
but its DWARF unwinding did not resolve our call chains, so the split below comes from wall-clock
probes around each step (one walk, fresh profile, removed again before the commit):

| Step (frame thread) | Track Select entry | Each Down | Ship Select entry |
| --- | --- | --- | --- |
| `unlock_gate`: `open_source` and `Gate::read` | 104 ms | | |
| `open_picker`: `packs_from_defaults` then `open_source` | 9 + 88 ms | | 9 + 88 ms |
| `load_slideshow` (stills read, sheet extended) | 9 ms | 8 to 10 ms | 0.4 ms |
| preview model read | 3 ms | 1 ms | 7 ms |
| `Preview::new` | **711 ms** fresh, 63 ms warm | **57 ms** | 60 ms |

`Preview::new` calls `mesh_render::build` with no `BuildCacheScope` open, so every selection
recreated the shader module and every pipeline (not split further between naga and the driver). The fresh-profile second is
the driver's shader cache, which lives under `XDG_CACHE_HOME` (`mesa_shader_cache`,
`radv_builtin_shaders`): a fresh profile whose cache directory is seeded with only those two from
a warm run enters Track Select in 283 ms, the warm figure. A player sees it once after an install
or a driver update; the earlier section's "cache not identified" is this one.

Fix, in two parts:

1. **The previews keep their pipelines** (`oag_mesh::mesh_render::KeptBuildCache`, held on
   `Gpu::preview_pipelines`). It opens a build scope on its shader module and pipelines for one
   `Preview::new` and takes them back afterwards; textures are not kept. Inside an already open
   scope it defers to that one. A Down step's `Preview::new` went from 57 ms to 0.8 to 1.3 ms.
2. **The race box's disc reads move to a worker** (`session::menu_source`): on the first menu
   frame it opens the source once, reads the unlock gate and builds the preview the two screens
   open on (the current circuit and team) into its own kept cache, which the frame thread adopts.
   Track Select and Ship Select share those archives (`picker_stage::SharedArchives`) instead of
   opening the disc each. On the Deck the worker takes 105 ms to open and 760 to 820 ms (cold
   driver cache) or 75 ms (warm) for the 9 pipelines, long before the player can reach Track
   Select; if a screen opens first it waits for the worker and logs `menu source: waited`. No walk
   logged a wait. The web build opens on demand as before (wgpu's web types are not `Send`).
   The archives carry the player's override layer (`oag_source::layers::mount`) as read when the
   worker opened them: an edit to the override directory now reaches the selection screens'
   previews after a restart, not on the next screen opening (a race load still opens afresh).
   The MODS page's switches do not touch the archives.

The walk, run on the Deck as `walk.sh LABEL BINARY fresh|keep [W H]` with a profile directory
of its own (`OAG_PHASE_SPIKE_MS=0` in the environment), then the per-transition summary:

```sh
# walk.sh: the profile setup is omitted (new XDG_* dirs, the settings copied with vsync off,
# no frame limit, pause_on_focus_loss false, [race] track = "16_Track", and for W H
# window_size = "WxH")
S="3:start,6:start,9:down,11:cross,14:right,14.5:right,15:right"
t=16; for i in 1 2 3 4 5; do S="$S,$t:down"; t=$(awk "BEGIN{print $t + 0.5}"); done
S="$S,19:cross"
t=22; for i in $(seq 1 22); do S="$S,$t:down"; t=$(awk "BEGIN{print $t + 0.4}"); done
t=$(awk "BEGIN{print $t + 2}"); S="$S,$t:cross"; t=$(awk "BEGIN{print $t + 4}"); S="$S,$t:quit"
while IFS= read -r -d '' kv; do
  case $kv in GAMESCOPE_*|XDG_CURRENT_DESKTOP=*|ENABLE_GAMESCOPE_WSI=*|XDG_RUNTIME_DIR=*|SteamDeck=*) export "$kv";; esac
done < /proc/$(pgrep -x -o steam)/environ
unset WAYLAND_DISPLAY; export DISPLAY=:1
gs=""; [ -n "$W" ] && { unset DISPLAY GAMESCOPE_WAYLAND_DISPLAY; gs="gamescope --backend headless -W $W -H $H -w $W -h $H --"; }
flock ~/.cache/oag/deck.lock $gs "$BIN" ~/.local/share/oag/images/pulse-psp-eu.chd \
  --unlock-all --frame-log --script "$S" > "$LABEL.log" 2>&1
```

```awk
# the worst frame-phase total (ms) per transition of one walk's log
/script: tap Cross on menu:race/ {cur="picker-entry"; next}
/script: tap Down on menu:picker/ {cur="down"; nd++; next}
/script: tap Cross on menu:picker/ {cur="ship-entry"; next}
/script: tap/ || /script: quit/ {cur=""; next}
/frame-spike/ && cur!="" { for (i = 1; i <= NF; i++) if ($i ~ /^total=/) v = substr($i, 7) + 0
  if (cur == "down") { if (v > dmax[nd]) dmax[nd] = v } else if (v > m[cur]) m[cur] = v }
END { for (k = 1; k <= nd; k++) { s += dmax[k]; if (dmax[k] > w) w = dmax[k] }
  printf "picker-entry %.1f ship-entry %.1f down-worst %.1f down-mean %.1f\n", m["picker-entry"], m["ship-entry"], w, s / nd }
```

Pictures: a headless `--drive` walk on this machine (RX 7800 XT, release, input devices hidden)
wrote Track Select after 0, 1 and 4 Downs and Ship Select after 0 and 1, before and after. Ship
Select and Track Select after 4 Downs are byte-identical; the other frames differ only in the
Distance(m) value, which the circuit-lengths worker fills in when it lands (a dash at load 28 to
34, the number at a lighter moment), with the same pixels everywhere else. The same runs put the
worst menu frame-phase total at 652 ms before and 18 to 34 ms after.

**Open (closed by the next section):** each selection still read its stills and its model on the
frame thread; the campaign screens and Pure's team gate (`Session::open_title_archives`) still
opened the disc on the frame that opens them.

## The remaining menu reads off the frame thread (2026-10-31, `menu-io-remaining`)

Four reads were still on the frame thread after the section above. All four are off it now:

1. **Each selection's stills and model** (`PickerStage::refresh_preview`): `picker_stage::ReadJob`
   reads the stills, extends the sheet and reads and parses the mesh and its ramp on an
   `oag_thread::Task`; the frame thread only does the GPU half (`finish_preview`, with the kept
   pipelines) when `poll_preview` sees the task finished. Until then the previous entry's picture
   stays up. A newer request waits for the task in flight and replaces any older one. The screen's
   **first** read is the exception: the opening frame joins its task, because nothing earlier
   exists to keep up and a hexagonal window or a hull missing for a few frames is a hole the
   original does not have (a first version without the wait drew a Track Select with no stills in
   a headless walk). That one read is 13 to 17 ms on the Deck (37 ms at 4K with the first GPU
   upload), the remaining frame-thread disc read of the race box.
2. **The campaign screens' disc open** (`Session::open_title_archives`): the menus' worker opens a
   second, owned `Archives` right after the first (before the previews' pipelines), and RACE
   CAMPAIGN takes it (`take_menu_archives`); a refill is opened on another worker at once. The
   Pure chain keeps its archives for its art player, which is why it needs an owned one and not
   the shared one. Pure's team gate (`pure_locked_teams`) and a 2048 DLC campaign cell read
   through the shared archives (`with_menu_archives`), the web build and a changed source open
   inline as before.
3. **A cell's launch**: `resolve_campaign_ai_skill_scale` opened `FEData.wad` fresh to read the
   circuit's `stats.xml`. That is a 35 ms frame on the Deck, found by timing the pieces of
   `launch_campaign_cell` (49.5 ms against 12 for the Ship Select it opens). The worker opens the
   file with the source now. HD's fallback (its `stats.xml` lives in the mounted archives) reads
   through the shared archives, so it now sees the DLC packs and the override layer as the other
   menu reads do; the bytes of the shipped file are the same.
4. **Pipelines for every team's hull.** The first Down in the campaign's Ship Select cost 50 ms on
   the Deck: the worker only built the current team, and another hull's first `Preview::new`
   compiled a pipeline on the frame thread. The worker now builds all teams' hulls (with no skins
   or variant, which draw through the same pipelines).

Method: the section above's (`deckwalk.sh`, `OAG_PHASE_SPIKE_MS=0`, Deck at vsync off and no
frame limit, `gamescope --backend headless` at 1280x800, 1920x1080 and 3840x2160; Pulse PSP EU,
`--unlock-all`, `OAG_MODS` empty). The race walk is the one above plus seven Downs on Ship
Select; the campaign walk is START, START, then Cross three times (Grid Selection, the cell
screen, the cell: Ship Select opens) and a Down on Ship Select. Worst frame-phase `total`
before and after, ms, one run each (the Deck was otherwise idle):

| Step | 800p before | 800p after | 1080p before | 1080p after | 4K before | 4K after |
| --- | --- | --- | --- | --- | --- | --- |
| RACE CAMPAIGN opens | 112.6 | 16.9 | 111.0 | 15.4 | 126.5 | 30.5 |
| worst menu frame, campaign walk to the race launch | 112.6 | 16.9 | 111.0 | 17.8 | 126.5 | 33.7 |
| Track Select entry | 16.0 | 18.3 | 17.3 | 18.1 | 38.3 | 37.8 |
| Ship Select entry | 12.5 | 14.5 | 12.1 | 16.7 | 20.2 | 23.3 |
| worst picker Down (Track and Ship Select) | 50.1 | 5.2 | 50.7 | 4.2 | 64.0 | 16.8 |
| mean picker Down | 13.9 | 3.6 | 13.8 | 3.6 | 22.8 | 7.8 |

The entries are flat or slightly up (the opening read is joined, and the worker's second archive
open competes for the disc during the walk's early seconds). No campaign or race-box menu frame
passes 50 ms at any of the three sizes now.

Pictures: a headless `--drive` walk on this machine (Track Select after 0, 1 and 5 Downs, Ship
Select after 0 and 1, RACE CAMPAIGN's first two screens), before and after, with ticks enough for
the worker to land: byte-identical. With too few ticks the after pictures differ only where the
previous entry's picture is still up, as designed.

**Open:** the opening read above; a variant or skin the worker did not build (HD's per-circuit
models, a Zone hull) may still compile on the frame thread once; the Pure chain's art player and
HD's campaign screens read the disc on the frame thread from their own archives (not measured:
only Pulse's campaign was walked).

## Per-frame allocations: five removed from the shadow, pad and advert paths (2026-11-04, `alloc-batch1`)

A read-only scout ranked the frame path's heap allocations; its first five items are fixed here,
one commit each, every picture byte-identical. The instrument is `perf-probe` (Rust allocations
only, through `oag_gpu::perfprobe::Counting`): `cargo build --release -p oag-game --features
perf-probe`, then `OAG_RENDER_BENCH=50 OAG_RENDER_PERF=1 oag-game <image> --race --mode
single_race --seed 1 --autopilot --no-audio --ticks 300 --screenshot x.png`, one view or `--players
2 --split-cut vertical`. The numbers are the median over the split bench's frames
(`render_viewports`: record, finish, submit), so a split's two views are one frame. "Tick" is
`OAG_RENDER_BENCH_FRAMES_PER_TICK=1` (every frame a new tick, the shadow hull gather redone), "hit"
is `=1000`. Pulse is Talon's Junction, Pure Vineta K, HD its default circuit, eight craft.

| Rust allocs / KiB per frame | before | after |
| --- | --- | --- |
| Pulse, one view, tick | 618 / 1289 | 376 / 576 |
| Pulse, one view, hit | 430 / 780 | 375 / 576 |
| Pulse, split, tick | 945 / 1924 | 671 / 1017 |
| Pure, one view | 288 / 559 | 247 / 365 |
| Pure, split | 607 / 1168 | 525 / 779 |
| HD, one view | 1000 / 1265 | 867 / 1039 |
| HD, split | 1575 / 2205 | 1441 / 1787 |

What each item took off (allocs / KiB per frame, the probe's stage marks):

1. **Shadow upload's vertex list** (`oag_render::shadow::Pipeline::upload`): a
   `Vec::with_capacity(MAX_VERTICES)`, 1968 vertices or 192 KiB, per view per frame on every
   title, now kept on the pipeline. 192 KiB a view, one allocation.
2. **The `original` tier's hull gather** (`HullGather::at_tick`, `hull_triangles`,
   `conform_to_floor`, `Occluder::outline`): about 187 allocations and 509 KiB a tick on Pulse,
   now worked in lists the hull cache keeps (`HullScratch`, `OutlineScratch`, the dropped gather's
   lists). The tick frame now costs what a cache-hit frame does. Pure's default tier runs no hull
   gather, so it is unchanged there.
3. **HD's sun-occlusion and self-shadow layers**: a queue write per craft per pass (sixteen a
   frame on a full grid) and a `Vec` of near draws per craft, now one write per buffer
   (`occlusion::Maps::write_uniforms`) and a single walk. 112 allocations a frame on HD (697 to
   584 at the mark that holds the passes).
4. **Weapon-pad tint** (`Drawable::recolour_nodes`): a queue write per pad per view, now one per
   run of pads built back to back, which is all of them. 33 a view on Pulse, 41 on Pure.
5. **Advert cards' uniforms** (`adverts::Cards::render`): a constant matrix written per card per
   frame, now once. 21 a frame on Pulse and HD (Pure draws no card on Vineta K).

**Dropped: skipping an advert card's pass when its clock has not moved.** The scenery clock is
blended by the draw alpha between ticks (`Race::clock_seconds`), so every frame of a race draws
every card at a new time, and no card on Talon's Junction (PSP and PS2) or HD's default circuit
shows one picture at four clock readings (0, 1.3, 2.5 and 7.1 s, the card read-back tests with
`OAG_ADVERT_TIMES` and `OAG_ADVERT_DUMP`): each changes between consecutive readings, and slot 1
on PSP and PS2 returns to its 0 s picture at 7.1 s, a loop. Only the render bench, which pins
the alpha at 1.0, would ever see the skip fire.

**Counting every `malloc`, not only Rust's** (heaptrack, `OAG_RENDER_BENCH=300` against `=1`, the
difference over the 628 frames the two bench loops draw, Pulse and HD one view, twice for Pulse):
Pulse 620 and 621 to 361 and 356 a frame (-42 percent), HD 1190 to 1057 (-11 percent). The rest
of HD's is wgpu's own encoder and pass recording.

**CPU on the notebook (Core Ultra 7 265H, NVIDIA dGPU, powersave governor): below the noise.**
`render_viewports CPU` median from `OAG_RENDER_BENCH=300`, pinned with `taskset -c 4`, six
interleaved base and branch pairs a configuration, load 1.4 to 2.4. Single runs are bimodal
(Pulse one view reads about 720 or about 1000 us), so the means move with which mode a run
landed in:

| us, mean of the six medians (median of them) | base | branch | pairs faster |
| --- | --- | --- | --- |
| Pulse, one view | 852 (836) | 765 (731) | 3 of 6 |
| Pulse, split | 1224 (1155) | 1336 (1106) | 5 of 6 |
| Pure, one view | 510 (515) | 474 (466) | 4 of 6 |
| HD, one view | 2083 (2113) | 2062 (2061) | 3 of 6 |

The spread inside one binary (Pulse one view 718 to 1001 us) is larger than any difference, so
the CPU claim here is none. The allocation counts and bytes are the result.

**On the Steam Deck: no measurable change at 800p or 4K; at 1080p one configuration, Pure split,
reads 0.25 ms slower, with the CPU clocked lower rather than more work done.** Both binaries built in the
Debian container (`cargo build --profile dist -p oag-game --no-default-features --features
extensions,dev-cli`, `RUSTFLAGS="-C target-cpu=znver2"`), one race each per configuration:

```sh
ENABLE_GAMESCOPE_WSI=0 gamescope --backend headless -W $W -H $H -w $W -h $H -r 90 -- \
  ./oag-game <image> [--track 'Data\Environments\15_Anulpha_Pass\track.vex'] \
  --race --mode single_race --seed 1 --autopilot --no-audio --no-intro --frame-log \
  --render-scale 100 --script "26:quit" [--players 2 --split-cut vertical]
```

with the profile's `vsync = "off"` and `frame_limit = "unlimited"`, every run under `flock -o
~/.cache/oag/deck.lock`, a short warm-up race per binary, title and view count first. The figure is
the mean of the `frame-phases` lines' `total` (ms of frame-thread CPU) after the first six
seconds, then the mean over three rounds, the range across rounds in brackets. Base ran before
branch in every pair. Pure is Vineta K, Pulse Talon's Junction, HD Anulpha Pass.

| ms `total` | 800p base | 800p branch | 1080p base | 1080p branch | 4K base | 4K branch |
| --- | --- | --- | --- | --- | --- | --- |
| Pure, one view | 3.31 (3.28-3.33) | 3.32 (3.29-3.35) | 4.33 (4.29-4.37) | 4.28 (4.19-4.36) | 5.34 (5.29-5.43) | 5.30 (5.25-5.36) |
| Pure, split | 6.52 (6.50-6.54) | 6.52 (6.50-6.54) | 7.41 (7.36-7.45) | 7.54 (7.43-7.62) | 10.35 (10.29-10.43) | 10.27 (10.21-10.32) |
| Pulse, one view | 4.42 (4.41-4.44) | 4.40 (4.40-4.40) | 6.05 (5.98-6.10) | 6.01 (5.98-6.04) | 7.02 (6.97-7.06) | 7.00 (6.94-7.04) |
| Pulse, split | 8.16 (8.10-8.26) | 8.36 (8.33-8.41) | 9.62 (9.52-9.70) | 9.79 (9.76-9.81) | 13.63 (13.62-13.65) | 13.61 (13.56-13.65) |
| HD Anulpha, one view | 7.15 (7.08-7.23) | 7.09 (7.02-7.16) | 8.20 (8.15-8.23) | 8.03 (7.89-8.11) | 11.15 (10.86-11.70) | 11.14 (11.09-11.18) |
| HD Anulpha, split | 12.91 (12.81-12.96) | 12.75 (12.63-12.93) | 15.00 (14.95-15.04) | 14.91 (14.88-14.95) | 18.77 (18.29-19.02) | 18.38 (17.83-18.99) |

Every cell is within 2.5 percent, most within 1. The frame rate is not paced here: one view at
800p runs at 210 to 245 fps, the splits at 76 to 148, and 4K is GPU-bound at 26 to 30 fps
whatever the CPU does, so `total` is the CPU signal. The allocator time removed is a few
microseconds a frame against 3 to 19 ms, so this is the size of change to expect.

Base always ran first above, so the two splits that read slower were run again with the branch
first (three rounds) and Pure split at 1080p a third time in ABBA order (branch, base, base,
branch, twice):

- **Pulse split was the order**: branch first, 800p 8.33 to 8.28 ms and 1080p 9.86 to 9.64 ms.
- **Pure split at 1080p is not**: 7.35 to 7.75 ms branch first, 7.40 to 7.65 ms in ABBA order,
  slower in 9 of 10 pairs whatever ran first. fps does not move (106, the GPU's limit at that
  size) and neither does the same configuration at 800p (6.52 against 6.52, CPU-bound at 148
  fps) or 4K. Every phase of the `frame-phases` line stretches by the same few percent, compose,
  overlay and present included, which this batch does not touch, and the profile's settings read
  the same in both logs (`governor=0`, `render=surface=1920x1080`). The CPU clock sampled during
  four more runs (`/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq`, every 0.5 s, the
  `powersave` governor) reads 2084 and 1849 MHz on the branch's runs against 2132 and 2146 MHz on
  the base's. A GPU-bound frame with less CPU work in it lets the governor clock down, and the
  same work then takes longer. That fits, but it is not proven, and it is not attributed to one
  item: Vineta K runs only the shadow upload (item 1) and the pad tint (item 4) of the five.

## Per-frame allocations, batch two: the Zone lookup, the hull copy, the exhaust quad, the scene blocks, the small view lists and the HUD (2026-11-04, `alloc-batch2`)

The scout's items 6 to 9, 11 and 12, one commit each, every picture byte-identical (26
captures: Pulse, Pure, HD's default circuit and Anulpha Pass; one, two and four views; ticks 300,
900 and 1800; a rocket fired; a Zone race on each of the three). Same instrument and command as
batch one above; the HUD, which the bench loop does not draw, is counted by `perf-probe` marks
around its draw (`hud-readout`, `hud-frame`, `hud-paint`, in `hud_views::draw` and
`Overlay::draw`/`draw_views`), once per capture at tick 300.

| Rust allocs / KiB per frame, scene (bench median) | before | after |
| --- | --- | --- |
| Pulse, one view, tick | 376 / 576 | 334 / 389 |
| Pulse, one view, hit | 375 / 576 | 333 / 389 |
| Pulse, split, tick | 671 / 1017 | 591 / 816 |
| Pure, one view | 247 / 365 | 212 / 346 |
| Pure, split | 525 / 779 | 455 / 742 |
| HD, one view | 867 / 1039 | 844 / 1022 |
| HD, split | 1441 / 1786 | 1398 / 1752 |

| HUD draw list (`hud-frame`), allocs a frame | one view | split | four views |
| --- | --- | --- | --- |
| Pulse | 34 to 30 | 144 to 94 | 303 to 191 |
| Pure | 26 to 26 | 105 to 69 | 200 to 128 |
| HD | 148 to 103 | 290 to 202 | 1042 to 837 |

What each item took off (the stage marks, per view unless said):

6. **The Zone visualiser lookup** (`zone::write_vis_changed`, `Drawable::write_zone_vis`): four
   1 KiB texture writes a view, Zone or not, now made only when the texels change. The scene
   keeps what each of the four lookups was last given, `None` at build and after
   `rebind_zone_art`. Skipping is safe across a
   split because every view's writes land in the order the views were prepared
   (`oag_gpu::writes`). 17 allocations a view on Pulse and Pure, 11 on HD.
7. **The `original` tier's hull copy** (`ShadowGathered`, `HullCache::current_vertices`,
   `HullGather::blend_into`): at a tick's own alpha the upload reads the cached gather in place
   instead of a clone, a blend refills a kept list, and the models are compared without
   collecting. 163 KiB and 2 allocations a frame on Pulse.
8. **The flare quad** (`Exhaust::vertices`): six vertices returned on the stack, not in a `Vec`.
   8 a view on Pulse and Pure.
9. **The scene blocks** (`SceneBlocks::write_view`): a view's circuit, ship, shine and flame
   blocks are adjacent, so one write for each run of used kinds, one where four were. 12 a view
   on Pulse, 8 on Pure, 4 on HD.
11. **The small per-view lists**: the shadow runs and the prepass's sort list come back from the
   `ViewPass` after `Scene::render` records it (`keep_view_lists`); `MotionState::advance` lends
   the previous snapshot rather than cloning it; `Race::hd_sprite_quad` returns its quad in place;
   HD's craft shadow ranking works in kept lists. 3 to 4 a view and 12 KiB, 8 on HD.
12. **The HUD**: a bar's fill is cropped without copying its `name` and `src` (`crop_geometry`,
   `bar_tip::crop_geometry`, drawn by `sprite_draw_from`), an untipped bar no longer copies the
   whole sprite first, a split view with a lock-on sight draws without copying its `Readout`
   (`draw_list_without_sight`), and a frame placed into a view or the safe area moves its draws
   instead of copying them (`Frame::into_placed`). The last is the largest (52 of HD's 148 in one
   view were `Draw` copies there) though the scout did not name it.

**Left, with the reason.** The `DrawList` borrows the frame's drawables, so keeping it between
frames needs its lifetime erased; the split's `noted` list and `timestamp_clones` live in the
split-recording files; `ChunkSet::around` (HD) and `oag_mesh`'s `order_blended` each need an API
change for one list. On the HUD, `text_for`'s `String` per label is what `Draw::Text` owns, HD's
`runtime::tinted` copies a sprite per tinted widget (8 a view), and `Overlay::paint` (about 48 a
frame, the renderer's own buffers and wgpu's writes) was not in scope.

**A kilobyte in every drawable cost Pulse a slow mode; on the scene instead it is gone.** Item 6
first kept its memo inline in each `Drawable` (a `Mutex<Option<[[u8; 4]; 256]>>`, about 1 KiB,
also copied into every weapon-pool instance). `render_viewports CPU` (median per run,
`OAG_RENDER_BENCH=300`, `taskset -c 4`, round-robin over the binaries, load 1.8 to 4.7) then put
Pulse one view in a slow mode in about half its runs (855 to 952 us) that the base never showed
in 30 runs (648 to 700 us); the split's slow runs were about 480 us over, so about 240 us a view.
The binary with items 6, 8 and 9 alone already had it; with the four memos on the `Scene` it went
away (20 of 20 runs 627 to 682 us). The allocation counts and the pictures are the same either
way. Read: a drawable is walked by every view, and its size is part of what a frame touches.

Six interleaved pairs of the final branch against its base (same command, load 2.3 to 3.4):

| us, mean of the six medians (range) | base | branch | pairs faster |
| --- | --- | --- | --- |
| Pulse, one view | 685 (668-692) | 652 (629-673) | 5 of 6 |
| Pulse, split | 1057 (1044-1071) | 1027 (1014-1048) | 6 of 6 |
| Pure, one view | 408 (359-516) | 498 (406-649) | 0 of 6 |
| HD, one view | 2125 (1972-2251) | 2032 (1904-2122) | 6 of 6 |

Pulse (3 to 5 percent) and HD (4 percent) read faster in every or all but one pair. **Pure one view
reads slower and is not explained**: its base alone spans 358 to 642 us across runs, a
round-robin of base and the intermediate binaries put the medians at 417, 421, 417 and 433 us
(base, items 6/8/9, plus 7/11, final), and a second one, taken as the machine's load rose to 9.9,
at 425 against 493. Vineta K runs items 6, 8, 9 and 11 of these (no hull shadow, no HUD in the
bench), none of which adds work there; the Steam Deck run is what should settle it.

**The Steam Deck was not measured for this batch** (the Deck was in use): the matrix and the
harness are batch one's above, with the base at `cb561e9c6`.
