# Which GPU passes grow with the output size (2026-10-31, `hires-gpu-levers`)

The question: for HD and Pulse races, which passes grow with the pixel count, by how much, on
this host's AMD iGPU and dGPU and on the Steam Deck, at 1280x800, 1920x1080 and 3840x2160, and
what are the best levers. Measurement only, no rendering change. **Chosen workload, not
measured against any original.** Earlier work this extends: [high-resolution cost](high-resolution-cost.md)
(dGPU only, 4K and 8K), [HD GPU cost](hd-gpu-cost.md), [dynamic resolution](dynamic-resolution.md).

**Result.** Everything that draws the scene grows with the pixels (2.5x to 3.5x from 1080p to 4K,
4x pixels), and on every device the same four passes carry the growth, in this order on the Deck:
the track's solid draw (HD +18.1 ms 1080p to 4K, Pulse +9.0), the motion blur gather (+10.2 ms
when on), the craft draw (+2.9 HD), HD's bloom (+2.0). The present composite is small (+0.8 to
+1.1 ms). Shadows and the rest of `Scene::render` are flat (5.2 ms HD, 2.2 to 2.6 ms Pulse on the
Deck at any size). On the Deck at a 4K output the game already draws about 1906x1080 (`GAMESCOPE_PIXEL_BUDGET`, `settings::render_profile`), so
the 4K column below is the cost of an opt-in native 4K, not what the Deck pays by default; what
the default 4K output pays is the 1080p column plus the presented upscale (below).

## Method

`OAG_RENDER_GPU_BENCH=100 OAG_RENDER_PERF=1 oag-game <image> --race --mode single_race --seed 1
--autopilot --no-audio --size WxH --ticks 2500 --screenshot out.png`, release `perf-probe` build
of `1633cbb39` (the Deck: `dist` profile, `-C target-cpu=znver2`, built in the `oag-appimage-build`
container, run offscreen over ssh with no gamescope). GPU timestamps, median of 100 redraws of the
tick-2,500 frame, then the median of **3 separate processes**. HD is Talon's Junction, Pulse is PSP
EU's default circuit. Devices: `MESA_VK_DEVICE_SELECT=1002:747e` (RADV NAVI32) and `1002:164e`
(RADV RAPHAEL_MENDOCINO), each confirmed in the log's `bench split:` line; Deck `RADV VANGOGH`.
"Deck profile" is `--msaa 4x --motion-blur high`. The "present composite" row is the same run
with `--presented` and `OAG_RESOLVE_BENCH=60` (no upscaler, scale 100; it is serialised CPU plus
GPU wall time, so it carries about 60 us of submit and wait floor on the dGPU). "Rest of
`Scene::render`" is `Scene::render GPU` minus the race pass, the blur chain and the bloom: the shadow
passes plus work no span covers. Host load was 0.3 to 59 across the dGPU and iGPU runs (the host
was shared; GPU spans do not move with it, the iGPU less surely than the dGPU); the Deck sat at
load 0.6 to 1.7, AC power, governor `powersave`, GPU `auto`, vsync and frame limit not involved
(offscreen).

## Spans per pass, ms


#### HD, defaults (MSAA off, blur off), ms, medians of 3 processes

| pass | Steam Deck 800p / 1080p / 4K / 4K-1080p | host iGPU 800p / 1080p / 4K / 4K-1080p | host dGPU 800p / 1080p / 4K / 4K-1080p |
|---|---|---|---|
| race pass | 5.99 / 10.43 / 32.11 / +21.69 | 15.33 / 25.16 / 75.19 / +50.03 | 0.76 / 1.12 / 2.96 / +1.84 |
| - track solid | 4.62 / 8.22 / 26.32 / +18.10 | 12.09 / 20.42 / 62.24 / +41.82 | 0.48 / 0.77 / 2.23 / +1.46 |
| - craft, weapons, shields | 1.14 / 1.85 / 4.76 / +2.91 | 2.83 / 4.03 / 10.21 / +6.18 | 0.17 / 0.23 / 0.53 / +0.30 |
| HD bloom | 0.53 / 0.94 / 2.98 / +2.04 | 0.98 / 1.63 / 5.20 / +3.57 | 0.15 / 0.19 / 0.39 / +0.19 |
| rest of Scene::render (shadows, other; flat) | 5.15 / 5.21 / 4.70 / -0.52 | 7.17 / 6.12 / 5.56 / -0.56 | 0.77 / 1.13 / 1.12 / -0.02 |
| present composite, serialised CPU+GPU | 0.25 / 0.42 / 1.50 / +1.09 | 0.36 / 0.74 / 2.75 / +2.01 | 0.08 / 0.08 / 0.11 / +0.02 |

#### Pulse, defaults (MSAA off, blur off), ms, medians of 3 processes

| pass | Steam Deck 800p / 1080p / 4K / 4K-1080p | host iGPU 800p / 1080p / 4K / 4K-1080p | host dGPU 800p / 1080p / 4K / 4K-1080p |
|---|---|---|---|
| race pass | 2.41 / 4.80 / 17.24 / +12.44 | 4.64 / 9.52 / 33.33 / +23.81 | 0.39 / 0.58 / 1.65 / +1.06 |
| - track solid | 1.65 / 3.37 / 12.35 / +8.98 | 3.28 / 6.68 / 23.59 / +16.91 | 0.17 / 0.31 / 1.08 / +0.77 |
| - craft, weapons, shields | 0.11 / 0.20 / 0.64 / +0.44 | 0.20 / 0.41 / 1.26 / +0.85 | 0.03 / 0.04 / 0.08 / +0.05 |
| rest of Scene::render (shadows, other; flat) | 2.19 / 2.57 / 1.90 / -0.67 | 0.90 / 1.29 / 1.65 / +0.36 | 0.45 / 0.46 / 0.48 / +0.02 |
| present composite, serialised CPU+GPU | 0.27 / 0.39 / 1.14 / +0.75 | 0.33 / 0.70 / 3.00 / +2.30 | 0.07 / 0.08 / 0.10 / +0.02 |

#### HD, Deck profile (`--msaa 4x --motion-blur high`), ms, medians of 3 processes

| pass | Steam Deck 800p / 1080p / 4K / 4K-1080p | host iGPU 800p / 1080p / 4K / 4K-1080p | host dGPU 800p / 1080p / 4K / 4K-1080p |
|---|---|---|---|
| race pass | 8.31 / 13.49 / 26.77 / +13.28 | 19.67 / 32.00 / 90.63 / +58.62 | 1.00 / 1.45 / 3.65 / +2.20 |
| - track solid | 6.08 / 10.07 / 20.64 / +10.56 | 14.76 / 24.58 / 71.70 / +47.12 | 0.62 / 0.96 / 2.56 / +1.60 |
| - craft, weapons, shields | 1.64 / 2.43 / 3.68 / +1.25 | 3.82 / 5.69 / 13.33 / +7.64 | 0.23 / 0.30 / 0.63 / +0.33 |
| HD bloom | 0.52 / 0.94 / 2.28 / +1.34 | 0.77 / 1.44 / 5.26 / +3.83 | 0.15 / 0.19 / 0.40 / +0.20 |
| motion blur chain | 2.86 / 6.56 / 18.58 / +12.01 | 8.44 / 20.28 / 86.01 / +65.73 | 0.33 / 0.66 / 2.51 / +1.85 |
| - blur reconstruct | 2.32 / 5.56 / 15.73 / +10.17 | 7.53 / 18.70 / 79.39 / +60.69 | 0.21 / 0.47 / 2.00 / +1.53 |
| rest of Scene::render (shadows, other; flat) | 5.15 / 5.41 / 4.83 / -0.57 | 6.06 / 4.75 / 5.01 / +0.26 | 0.81 / 1.13 / 1.32 / +0.19 |

#### Pulse, Deck profile (`--msaa 4x --motion-blur high`), ms, medians of 3 processes

| pass | Steam Deck 800p / 1080p / 4K / 4K-1080p | host iGPU 800p / 1080p / 4K / 4K-1080p | host dGPU 800p / 1080p / 4K / 4K-1080p |
|---|---|---|---|
| race pass | 3.02 / 5.87 / 13.95 / +8.08 | 6.42 / 12.43 / 40.08 / +27.65 | 0.46 / 0.70 / 1.89 / +1.19 |
| - track solid | 2.02 / 3.97 / 9.50 / +5.54 | 4.35 / 8.41 / 26.93 / +18.52 | 0.21 / 0.38 / 1.20 / +0.82 |
| - craft, weapons, shields | 0.13 / 0.23 / 0.47 / +0.25 | 0.28 / 0.54 / 1.49 / +0.94 | 0.03 / 0.04 / 0.09 / +0.04 |
| motion blur chain | 2.62 / 6.26 / 18.22 / +11.96 | 7.60 / 18.94 / 82.97 / +64.03 | 0.30 / 0.61 / 2.27 / +1.66 |
| - blur reconstruct | 2.14 / 5.26 / 15.27 / +10.01 | 6.77 / 17.38 / 76.43 / +59.05 | 0.19 / 0.44 / 1.80 / +1.36 |
| rest of Scene::render (shadows, other; flat) | 2.64 / 2.53 / 2.81 / +0.28 | 1.42 / 2.60 / 1.90 / -0.70 | 0.51 / 0.50 / 0.70 / +0.21 |

Every cell is one row of three medians; the full per-span tables (sky, blended track, particles,
blur sub-passes) are reproduced by the commands above. Blur sub-pass rows not shown here:
reconstruct is 85 to 92 % of the chain on every device and scales 2.8x (Deck) to 4.4x (iGPU).

### Reading the table

- **Track solid is the biggest growing pass everywhere** (79 % of the HD race pass on the Deck at
  1080p, 82 % at 4K) and costs about 3.1 ns a fragment on the Deck at 1.0x overdraw (8.5 M
  fragments at 4K for 26.5 ms). That is about 5 GPU cycles of the whole 8-CU chip per fragment, a
  heavy fragment, and it is the pass the picture's exactness lives in.
- **The motion blur gather scales worst** (2.8x on the Deck, 4.2x on the iGPU, 4.3x on the dGPU), because
  the taps cost per pixel. On the Deck HD at 1080p the whole chain is 6.56 ms against a race pass of
  13.5 ms.
- **The iGPU is bandwidth-shaped**: its HD bloom and present composite grow 3.2x and 3.7x, more
  than the dGPU's 2.0x and 1.3x. The dGPU is at its fixed floors at 1080p already.
- **Rest of `Scene::render` is flat** and not small on the slow chips (HD: Deck 5.2 ms, iGPU 6.1
  ms against 0.8 to 1.1 ms on the dGPU).

## What a setting costs on the Deck at 1080p (3 rounds, ms)

| Setting | HD race pass | Pulse race pass | Notes |
|---|---|---|---|
| defaults, anisotropy 16x | 10.43 | 4.80 | |
| anisotropy 4x | 10.30 (-0.13) | 4.77 (-0.03) | |
| anisotropy off | 10.12 (-0.31, 3.0 %) | 4.49 (-0.30, 6.3 %) | the track is not texture-filter bound |
| `--msaa 4x` | 13.20 (+2.77, +27 %) | 5.97 (+1.17, +24 %) | |
| `--shadows off` (race pass, whole `Scene::render`) | 9.51 (-0.91), 14.1 from 16.7 (-2.6) | 4.78 (-0.02), 6.8 from 7.4 (-0.55) | shadows cost HD 3.5 ms in all |
| blur high, full gather (the chain) | 6.56 | 6.26 | |
| blur high, `--motion-blur-resolution half` | 3.24 (-3.3) | 3.19 (-3.1) | the DRS rung, [half rung](motion-blur-half-rung.md) |

`Scene::render GPU` after `--shadows off` still left 3.7 ms (HD) and 2.0 ms (Pulse) that no span
named. Every pass of the frame has a span now
([render-bench.md](../tools/render-bench.md), "Every pass of one frame by name"); the rows that
split those two numbers on the Deck are not taken yet.

## The presented 4K output on the Deck (headless gamescope, 3840x2160)

`gamescope --backend headless -W 3840 -H 2160 -w <w> -h <h> -- oag-game <image> --race --autopilot
--no-audio --frame-log --seed 1 --script 30:quit`, a window of w x h, vsync off and frame limit
240; median of the steady `frame-log` seconds (the first 8 dropped), 2 runs each, the two agree to
0.1 ms except where stated. The frame time includes our upscale and composite, **not gamescope's
own scale pass**, which runs in another process on the same GPU and was not measured.

| Case | render | HD default | HD Deck profile | Pulse default | Pulse Deck profile |
|---|---|---|---|---|---|
| game window 1920x1080, gamescope scales to 4K | 1906x1080 | 9.88 ms, gpu 6.82 | 15.72, gpu 8.43 | 5.99, gpu 4.40 | 9.12, gpu 5.31 |
| game window 3840x2160, game draws 1906x1080 (the default budget) and upscales | 1906x1080 | 11.57, gpu 6.90 | 17.27, gpu 8.46 | 9.04, gpu 4.63 | 11.93, gpu 5.29 |
| game window 3840x2160, `render_scale_chosen`, scale 100 (native 4K) | 3812x2160 | 29.63, gpu 22.97 | 51.70, gpu 25.97 | 22.55, gpu 17.88 | 36.56, gpu 20.61 |

So our own present at a 4K output costs the game **+1.7 ms (HD), +3.0 ms (Pulse) of frame time, +1.5 and +2.8
with the Deck profile**, against the game being given a 1080p window and gamescope doing the scale.
It is more than the single-tick composite (1.1 ms HD, 0.76 Pulse), because the composite also writes
the HUD at 4K and a 4K swapchain image. The cost of the picture this buys, the HUD drawn at 4K
([ADR-0036](../architecture/adr/0036-ui-composites-at-presentation-resolution.md)), is not measured
here. Native 4K misses 60 Hz on the Deck in every profile (HD 29.6 ms at best).

## The Deck's 4K rows are unstable (read the 1080p and 800p rows first)

`dpm` is `auto` and the 4K HD race pass sits in two modes: one run of 100 redraws has p10 27.5 ms and
p90 32.8 ms (400 redraws: 28.7 to 32.6; 1,000: 27.3 to 32.1). Three interleaved rounds at 4K read
race pass 32.6 (no MSAA), 39.0 (`--msaa 4x`) and 27.3 (`--msaa 4x --motion-blur high`): **MSAA
plus blur reads faster than no MSAA**, which is the clock, not the work (the blur run sat at 1,600
MHz in 85 of 110 samples, the others in 1,040 and 1,600). Repeats within one configuration agree to 0.3 %, so
the table's spread cannot show it. The race-pass rows at 800p and 1080p are stable to 0.1 to 1 % across rounds (the dGPU's whole-`Scene::render` row is noisier, up to 25 % between its clock states). A 4K Deck row is good to about 15 %.

## Ranked levers

Ranked by what each costs the Deck between 1080p and 4K, then by what it gives back. Craft (+2.9 ms)
and bloom (+2.0 ms) grow about as much as lever 3 but are held to exactness, so they are not
levers; lever 3 is one we can pull. Estimates are **estimates, not measurements**.

1. **Track solid fragment cost** (+18.1 ms HD, +9.0 Pulse, 4K-1080p; 8.2 ms of 10.4 at 1080p HD). Anisotropy is 0.3 ms of it,
   MSAA is not the cause, the velocity write is already gated. Lane: bisect the cost into ALU, texture and ROP with
   diagnostic runs, then cut the largest piece. Estimate 5 to 15 % of the track pass.
2. **The motion blur gather** (+10.2 ms at 4K on the Deck, 5.6 ms at 1080p, 2.8x). Already unrolled
   and with a half-gather DRS rung; the remaining lever is to skip the taps where the tile's
   largest velocity is under the blur's minimum. Estimate 1 to 3 ms at 1080p on the Deck, picture
   identical except where the smear is under a pixel.
3. **Our own 4K present** (+1.5 to +3.0 ms of game frame time against a 1080p window scaled by gamescope).
   Lane: measure gamescope's own pass to know whether the work moves or goes away, then decide
   between keeping our composite (sharp HUD) and a 1080p window with the HUD scaled.

Passes the brief named that this tooling cannot separate or that are small: **fog** has no span (it is shaded inside track solid); **MSAA resolve** lands inside the race pass span (the +2.8 ms HD at 1080p is in the pass, while the rest of `Scene::render` does not grow with it); **glass and transparent overdraw** is the `track blended` span, +0.4 ms 4K-1080p on the Deck HD, small; **render-target formats and bandwidth** are not measured directly, the iGPU's bloom and composite growing faster than the dGPU's is the indirect evidence.

Also open, not top three: the Deck's split of the 2.0 to 3.7 ms of `Scene::render` outside the
spans above (every pass is named now, the Deck has not run it), MSAA 4x (+2.8 ms HD at 1080p), and
the iGPU's bandwidth-shaped bloom and composite (+3.6 and +2.0 ms at 4K).

## The three levers measured (2026-10-31, `hires-track-and-present`)

Deck `RADV VANGOGH`, `dist` `znver2` builds with `perf-probe` from the `oag-appimage-build`
container, offscreen over ssh, load 0.8 to 2.0. Same command as the span tables above
(`--race --mode single_race --seed 1 --autopilot --ticks 2500`, 100 redraws), baseline and
candidate run interleaved, A B A B.

### Lever 1, track solid: landed, the shader's own branches were the cost

**Bisect** (diagnostic switch, not committed: a pipeline constant that returns a constant colour, a
constant colour plus one albedo tap, or turns the colour write off), Deck 1080p, 2 rounds that
agree to 0.5 %:

| track solid, ms | full shader | constant colour | constant + one tap | colour write off |
|---|---|---|---|---|
| HD | 8.21 | 1.69 | 1.83 | 1.47 |
| Pulse | 3.37 | 0.70 | 0.76 | 0.35 |

So shading is 79 to 80 % of the pass (ALU and the latency of its fetches), one texture tap costs
0.06 to 0.15 ms, ROP and depth 0.2 to 0.35 ms. The host iGPU (also RDNA2) agrees: HD 19.7, 3.1,
4.4, 3.3 ms. `RADV_DEBUG=shaderstats` on the iGPU shows why: every `mesh.wesl` pixel shader is
2,500 to 4,700 instructions at 128 VGPRs (8 waves a SIMD), 168 VGPRs (6 waves) for the largest.
`lit_texel` branches on the flat `slots` word, and the compiler sizes registers and code for every
branch whether a model's vertices can reach it or not.

**Cut.** `shade.wesl` ANDs a per-model override, `slots_present`, into its copy of `slots` before
any branch reads it (`mesh_render::slots_present`): the OR of every vertex's word, so
`x & present == x` for each vertex and the picture cannot change, while every branch on a bit no
vertex carries folds away. Four bit groups are always set (alpha channel, `NO_AMBIENT`/`NO_SUN`,
`LIT_ALPHA`, material index) and a weapon effect model (any of `RIM_GLOW`, `RIM_EDGE`,
`CLOCK_SCROLL_RING`, `CLOCK_SCROLL_HALO`, `BOMB_HALO`) keeps all bits: those choices hold the extra
pipelines to HD 112 (from 90; 196 with a bare OR) and Pulse 116 (from 96).

| Deck, ms, medians | 800p before / after | 1080p before / after (3 rounds) | 4K before / after |
|---|---|---|---|
| HD track solid | 4.62 / 4.30 (-7 %) | 8.23 / 7.65 (-7.0 %) | 22.7 to 26.5 / 24.2 to 24.5 (bimodal clock) |
| HD craft, weapons, shields | 1.14 / 0.62 | 1.85 / 0.93 (-50 %) | 4.0 to 4.8 / 2.28 |
| HD race pass | 5.99 / 5.13 | 10.43 / 8.89 (-15 %) | 27.6 to 32.3 / 27.4 to 27.7 |
| HD `Scene::render` GPU | 11.7 / 10.0 | 16.75 / 14.34 | 35.0 to 40.0 / 34.4 to 35.4 |
| Pulse track solid | 1.66 / 0.83 | 3.38 / 1.64 (-52 %) | 12.35 / 5.82 |
| Pulse race pass | 2.43 / 1.24 | 4.81 / 2.37 (-51 %) | 17.25 / 8.18 |
| Pulse `Scene::render` GPU | 4.8 / 3.2 | 7.33 / 4.60 | 19.1 / 10.4 |

The pipelined per-frame bench (HD 1080p) went 10.09 to 8.69 ms. Picture: 24 frames byte-identical
(`cmp`) against the baseline binary on the Deck, HD and Pulse at ticks 600, 1200, 1800, 2500, 3000
and 3600, with the defaults and with `--msaa 4x --motion-blur high`. Cost: race load with a cold
shader cache (`MESA_SHADER_CACHE_DISABLE=true`, `--ticks 1`, 3 runs) HD 8.8 to 10.1 s to 9.8 to
9.9 s, Pulse 7.0 to 7.1 s; with a warm cache +0.08 s HD, +0.05 s Pulse.

### Lever 3, our 4K present against a 1080p gamescope window: dropped

The Deck has no 4K display (`DP-1` disconnected), so the real output path was not measured. Per
process GPU time came from DRM fdinfo (`drm-engine-gfx` plus `-compute`, the `gamescope` binary
copied to drop its file capability so its `/proc` is readable), with the game at a 60 fps frame
limit, vsync off, the default profile, interleaved rounds. HD at 60 fps sat at 1040 MHz with the
1080p window and 1,352 to 1,600 MHz with the 4K one, which hid its cost (its `gpu_ms` even fell),
so HD was re-run at a 40 fps limit, 3 rounds, all six at 1040 MHz:

| GPU time a frame, ms | HD window 1080p / 4K | Pulse window 1080p / 4K |
|---|---|---|
| game, headless gamescope 4K output, 1040 MHz | 14.40 / 16.76 (+2.4, 40 fps) | 10.55 / 13.45 (+2.9, 60 fps) |
| headless gamescope itself | 0 / 0 | 0 / 0 |
| nested gamescope (Wayland backend in the Deck's session), its own composite | 3.8 / 3.4 to 3.7 (1,352 to 1,600 MHz) | 3.8 to 4.3 / 4.3 to 5.3 (1040 MHz) |

fdinfo counts the time a job occupies the ring, so the game's numbers exceed its timestamp spans
(HD `gpu_ms` 10.7). Headless gamescope does no GPU work at all, even with `--force-composition`,
so it cannot price the scale. A nested gamescope composites in both cases (it cannot pass the
game's buffer through) and its 1080p to 4K scale costs 3.8 to 4.3 ms at 1040 MHz: more than our own
4K present adds (2.4 HD, 2.9 Pulse). On a real 4K output gamescope is expected to scan a 4K window
out directly (not measured), and whether it scales a 1080p window with a display plane (near-free)
or a composite (the nested cost) was not observable here. The earlier table's frame-time gap (+1.7 HD, +3.0 Pulse) is `present_ms`, not GPU:
the game's `gpu_ms` moved by 0.1 ms between the two windows.

HUD both ways (host, tick 1,800, scene at 1920x1080 in both): our 4K composite against the 1080p
frame scaled bilinearly to 4K, mean absolute difference per channel. HD's vector HUD is visibly
softer scaled: 2.94/255 over the speed readout and 1.32 over the lap counter against 0.22 over a
scene-only box. Pulse's pixel-art HUD barely moves: 0.43 and 0.27 against 0.13 (`just frame-compare`
is built for 480x272 emulator pairs and flags every pixel of a 4K pair, so it was not used). Keeping ADR-0036's presentation-size
composite; no setting change proposed.

### Lever 2, skip the blur gather where a tile cannot smear: dropped, the skip already exists

`fs_reconstruct`'s gather already returns early per pixel when the bilinear blend of the four
nearest neighbour-max tiles is under 0.5 px of reach (`motion_blur.wesl`, `gather`), which is the
per-tile test the lane proposed and the right one (a tile's own max would stop the smear bleeding
into still tiles). Counted with a marker colour on that branch (diagnostic, not committed), host,
`--msaa 4x --motion-blur high`, 1920x1080: **0.0 % of pixels** take it on HD and Pulse at ticks 300
(160 km/h), 600, 1200, 1800 (a full-speed corner), 2500 and 3000, against 75 % on the stationary
grid at tick 60. In a race every neighbourhood moves, so a skip saves nothing; prediction 0 ms,
not implemented.
