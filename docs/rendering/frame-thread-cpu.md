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

The stage name reads `race-paused` while the pause rule freezes the race (`--race` starts
frozen, a `start` tap in `--script` releases it).

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
Deck: load 0.4 to 1.2, battery (not charging), `powersave` governor. Raw tables are in the
lane's report; the commands are below.

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

`perf stat` on the process (Pulse, paused, 15 s): 1.517 GHz effective user clock
(`cycles:u / task-clock:u`), 1.63 instructions per cycle, against the part's 3.5 GHz boost. The
host that measured 1.2 ms runs near 5 GHz; clock alone is 3.3 times, the rest is Zen 2 against
a newer core. The Deck is simply that much slower per draw on a battery profile; there is no
blocking call to remove. `perf record -F 1999` (user cycles only: `perf_event_paranoid` is 2, no
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

1. **A lower CPU clock than this run's.** This run was at 1.5 GHz effective on battery with the
   `powersave` governor and nothing else running. Steam's own client, the overlay, a TDP cap or
   background downloads in the maintainer's session push it lower, and the frame thread scales
   with the clock. A 2x clock difference is the whole gap.
2. **A different profile or window.** The maintainer's figure is on a title and profile this lane
   did not ask for (the `Pulse (PS2)` profile has 4x MSAA and a `crt-aperture` filter).
3. Everything else in the build matches: same AppImage recipe, same settings file.

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

### Commands run on the Deck (about 25 `ssh` calls; every run under `flock -o ~/.cache/oag/deck.lock`)

`~/oag-perf/deck-read/run.sh LABEL pulse|hd 12 "SCRIPT" [flags]` launched
`squashfs-root/AppRun <image> --no-audio --frame-log --race --autopilot --script SCRIPT [flags]`
with `XDG_CONFIG_HOME` etc. under the lane directory; `SCRIPT` was `12:quit` (racing) or
`3:start,12:quit` (paused). Render-scale sweep: `--render-scale 50|100|150` with `target_fps = "off"`.
Ablation: `--render-scale 50 --shadows off|--reconstruction off|--motion-blur off|--screen-filter
off`. Profile: `PREFIX="perf record -D 9000 -F 1999 -o F" ./run.sh ... 20 "3:start,20:quit"` and
`PREFIX="perf stat -e task-clock,cycles:u,instructions:u -o F" ./run.sh ... 15`. The lane
directory was removed afterwards; the maintainer's install and settings were not touched.
