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
"Deck profile" is a heavy case chosen for this page, `--msaa 4x --motion-blur high`; it is no
default of the game on the Deck or anywhere else: MSAA and the blur are off in every fresh
profile. The "present composite" row is the same run
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

**Anti-aliasing with MSAA off (the default, 2026-11-04).** `msaa` is `off` and `reconstruction` is
`off` in every fresh profile, on every title and device, pinned by
`settings::tests::a_fresh_profile_has_msaa_off_on_every_title`; a file that says `4x` keeps it, so a
player who chose it is untouched. With both off a player sees stair-stepped geometry edges and no
softening. The cheapest anti-aliasing the engine has is `--reconstruction fxaa`, one fullscreen
pass. HD at 1920x1080 on
the Intel iGPU, 2 rounds at load 21 to 25 (relative only, not a Deck number; the Deck is not
measured): present resolve median 499 to 730 us with `off`, 1,073 to 1,363 with `fxaa`, 1,470 to
1,479 with `smaa` (three passes), against `--msaa 4x` adding 2,611 us to the race pass and 597 us
between passes. So FXAA costs roughly a fifth of what MSAA 4x does here. The default stays `off`:
FXAA softens the picture ([ADR-0013](../architecture/adr/0013-anti-aliasing-architecture.md)), and
it is a player's choice like the others on that row.

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

## The Deck's verdict on the 2026-11-04 work (2026-11-05)

`RADV VANGOGH`, `dist` `znver2` `perf-probe` builds, offscreen, `--msaa off --render-scale 100`, the
arms interleaved over 3 processes, `OAG_RENDER_BENCH=300 OAG_RENDER_GPU_BENCH=100 OAG_RENDER_PERF=1`.
Before is `583530a01` (the spans alone), after `4328ffdc4` (the folds, the plain draws and the atlas
skip). HD, Pulse PSP EU and Pure PSP USA, the method's frame at tick 2,500.

| `Scene::render` GPU, ms | 1280x800 | 1920x1080 | 3840x2160 |
| --- | ---: | ---: | ---: |
| HD | 9.79 to 6.91 (-29 %) | 14.14 to 9.63 (-32 %) | 35.94 to 21.62 (-40 %) |
| Pulse | 3.38 to 2.83 (-16 %) | 4.80 to 3.40 (-29 %) | 11.56 to 7.41 (-36 %) |
| Pure | 2.68 to 1.53 (-43 %) | 3.74 to 2.54 (-32 %) | 11.10 to 7.41 (-33 %) |

| Pipelined per frame, ms | 1280x800 | 1920x1080 | 3840x2160 |
| --- | ---: | ---: | ---: |
| HD | 5.44 to 4.81 (-12 %) | 8.34 to 5.56 (-33 %) | 22.84 to 13.58 (-41 %) |
| Pulse | 2.70 to 2.68 | 2.79 to 2.73 | 6.52 to 3.75 (-43 %) |
| Pure | 1.54 to 1.54 | 2.13 to 1.63 (-24 %) | 6.98 to 4.51 (-35 %) |

The second table is the one to plan from. `Scene::render GPU` waits for idle after each frame, and
HD at 1080p reads 9.63 ms there and 5.56 ms back to back, under even the sum of its spans (7.35 ms).
So the wait-idle figures run high, most likely the clock falling in the idle gap (inferred, the
clock was not sampled), and **the unattributed residue (HD 1.7 ms, Pulse 1.0 ms at 1080p) is most
likely that gap, not work: do not hunt it**. The flat pipelined cells have a lower GPU figure, so
they are bound by the CPU, not unimproved.

HD's rows at 1080p, ms: track solid shaded 7.13 to 3.46 (-51 %, Pulse -52, Pure -50, against the
iGPU's -47 to -51), shadow atlas 0.32 to 0.04, advert cards 0.68 to 0.48, craft 0.76 to 0.65,
depth half, bloom and `between passes` flat. `RADV_DEBUG=shaderstats`, peak VGPRs and heaviest
shader: HD 168 to 128, 4,315 to 3,886 instructions, 8 subgroups a SIMD at 128; Pulse and Pure 96 to
48, 3,578 to 1,603 instructions.

**The plain draws cost no CPU**: `render_viewports CPU` moves -8 to +2 % across the nine cells,
so the fallback to the rich pipeline over short runs is not needed. Pictures on RADV: Pulse and Pure
byte-identical in all 18 shots; HD differs in all 9, at 1080p by 1 level at most (rmse 0.001).

**Shadows**, HD 1080p, the same environment, 3 processes each: `--shadows off` takes 9.59 to 7.74 ms
wait-idle and 5.56 to 4.96 ms pipelined, so shadows cost 0.6 ms of throughput, not the 3.5 ms of the
table above, where the atlas skip has also landed. The named spans hold 0.32 ms of it.

**Two views, on the after binary, recorded interleaved** (`--players 2 --split-cut vertical`, 1 of 3
processes), ms:

| | `render_viewports` CPU | pipelined |
| --- | ---: | ---: |
| HD, 800p / 1080p | 8.17 / 8.16 | 9.29 / 9.61 |
| Pulse, 800p / 1080p | 4.20 / 4.51 | 4.46 / 4.59 |
| Pure, 800p / 1080p | 2.69 / 2.91 | 2.87 / 2.82 |

**A split is bound by the CPU**: the pipelined rate sits on the recording cost in every cell. One
view at 800p is too (HD: `total` 7.0 ms, 3.4 of it submit). The Deck's own screen no longer gains
from a GPU cut except in battery life; what is left pays at a docked 1080p and 4K.

What stays, in order, for 4K: track solid shaded, 10.5 of HD's 21.6 ms; HD bloom and encode, 4.4 ms.
Advert cards are not a draw-once: they animate on the scenery clock, and a split already draws them
with the first view only.

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

#### The emissive table folds where no vertex names a row (2026-11-04, `lf`)

`slots_present` could not touch the tests on the emissive table (`emissives.scale[entry].z > 3.5`
and thirty more): they read a uniform buffer, which a driver cannot fold whatever the index folds
to, and `ALWAYS` kept the material index for every model. Row 0 of the table is all zeros
(`Emissives::of`), so a pipeline whose vertices name no row now keeps no material bits and sets the
`emissive_rows` override to 0, and `bindings.wesl`'s two accessors return that zero as a constant
(`mesh_render::specialise`). `lit_texel` reads the row once, so a table that is read keeps no
extra loads.

Intel ARL iGPU (`anv`, Mesa 26.2.6, not the Deck): the largest fragment shaders of a Pulse race
went 3,075 to 1,858 and 2,009 to 1,461 instructions (`INTEL_DEBUG=fs`,
`MESA_SHADER_CACHE_DISABLE=true`), and 20 more compile at SIMD16 (29 to 49); HD's largest 4,870 to
4,849, unmoved as expected: its track names rows. Spans, 1920x1080, tick 2,500, A B interleaved,
3 rounds that agree to 0.3 %, load 12 to 18:

| Intel iGPU, us, medians | before | after |
|---|---:|---:|
| Pulse `track solid, shaded` | 1,707 | 1,350 (-21 %) |
| Pulse race pass | 3,177 | 2,615 (-18 %) |
| Pulse `Scene::render GPU` | 5,563 | 5,011 (-10 %) |
| HD `track solid, shaded` (2 rounds) | 6,383 | 6,271 (two clock modes, read as no change) |

Picture: `cmp` equal on HD, Pulse and Pure at ticks 1,200 and 2,500 with and without `--msaa 4x
--motion-blur high`, and on Omega at 2,500. 2048 at 2,500 differs in 3 of 921,600 pixels by one
level; three runs of one binary are equal, so it is the compiler's float order, not the logic.
The gap between the Intel and NVIDIA pictures of one binary is far larger (rmse 0.05 on a pair).
Deck: not run, Deck unavailable; the expected win on the Deck is **an estimate**, in the block's
direction (Pulse more than HD).

#### A circuit's plain draws shade through a pipeline of their own (2026-11-04, `lf`)

One `Model` is a whole circuit, so `slots_present` is the OR of every material on it, and one rare
material keeps every branch for every pixel. By draw the list is lopsided (opaque indices of the
race's circuit, `slots` role bits per draw):

| Circuit | draws | signatures | emissive rows | indices naming no row |
|---|---:|---:|---:|---:|
| HD Talon's Junction | 1,783 | 18 | 17 | 2,108,769 of 2,268,321 (93 %) |
| Omega Tech de Ra | 2,950 | 4 | 4 | 6,976,659 of 6,980,733 (99.9 %) |
| 2048 Altima | 2,391 | 3 | 0 named | all |
| Pulse, Pure | 1,013 to 1,555 | 1 | 0 | all (the emissive fold above already has them) |

`specialise::plain_draws` marks each opaque draw that names no row and no weapon role, and a model
with both kinds gets a second shading pipeline for the depth prepass's shaded half
(`Prepass::plain`, four on HD: culled or not, velocity written or not), told the OR of those draws
alone and `emissive_rows = 0`. `Drawable::draw_lists` picks per draw and keeps the draw order, so
coplanar ties resolve as before. The rest of the list, the cutouts and the blended list keep
the model-wide pipelines. The same exactness argument as `slots_present`: each draw's bits are a
subset of its pipeline's.

| Intel ARL iGPU, us, medians, 1080p, A B interleaved, load 2 to 5 | before | after |
|---|---:|---:|
| HD `track solid, shaded` (3 rounds) | 7,260 | 3,820 (-47 %) |
| HD race pass | 9,702 | 6,194 (-36 %) |
| HD `Scene::render GPU` | 15,655 | 11,678 (-25 %) |
| HD pipelined per frame (3 rounds) | 6,125 | 5,013 (-18 %) |
| Omega `track solid, shaded` (2 rounds) | 5,206 | 3,507 (-33 %) |
| Omega race pass | 11,157 | 9,097 (-18 %) |
| 2048 `track solid, shaded` against the commit before the fold above (2 rounds) | 2,431 | 1,808 (-26 %) |
| 2048 race pass, same | 3,767 | 2,882 (-24 %) |

The rich pipeline is SIMD8 with spills on this compiler, so Intel gains more than a driver without
that cliff will. Costs: HD `pipeline_sets` over the same bench run 16,385 to 24,215 (+48 %, the
classes interleave); `Scene::render CPU` read bimodal in both binaries with no shift (559 to 1,098
us) and pipelined per frame fell, so the CPU side is not visible here. Cold race load (no shader
cache, HD, 3 rounds) 14.7 to 15.8 s against 14.6 to 15.7 s, inside the noise.

Picture: `cmp` equal against the commit before on nine shots (HD, Pulse, Pure, Omega, 2048, ticks
1,200 and 2,500, MSAA and blur on and off). On the 13 race pairs drawn with the pairs' own flags,
9 are byte-identical and 4 HD pairs move 1 to 11 pixels of 2,073,600, by one level except on Moa
Therma, where 8 pixels reach 6 levels (not traced; the two pipelines compile differently). Distance
to the original, track crop:
mean rmse 18.929 to 18.929, SSIM 0.8339 to 0.8339, every pair equal to three decimals; the same
binary's gap between the Intel and NVIDIA GPUs is 0.008 rmse in the mean and up to 0.056 on a pair.

**Deck: not run, Deck unavailable.** RADV's cost is its VGPR count (128 to 168 for these shaders),
which this split should lower for the plain half; the win there is **an estimate** until the Deck
block (`RADV_DEBUG=shaderstats` and the span bench) has run.

#### The rig and the Zone recolour fold where the race has neither (2026-11-04, `lf`)

`scene.light.enabled` is 0 for a whole race on every circuit with no `.envsettings` (Pulse, Pure),
and `scene.zone.enabled` is 0 for every race with no Zone grade, but both are uniforms, so every
pipeline kept both paths' registers and fetches. `BuildCacheScope` pins two overrides to 0, as it
does `nova_prelit`: `light_rig` from `lit_by` and `zone_race` from `zoned`, and `lit_texel` reads a
constant zero for them. Neither moves mid-race: `ZoneGrade::light` hands a stand-in back untouched
and nothing else writes the scene's light; a race with no grade writes a zero `Zone`.

| Intel ARL iGPU, us, medians, 1080p, 3 rounds, load 2 to 5 | before | after |
|---|---:|---:|
| Pulse `track solid, shaded` | 1,350 | 887 (-34 %) |
| Pulse race pass | 2,614 | 1,944 (-26 %) |
| Pulse `Scene::render GPU` | 4,955 | 4,224 (-15 %) |
| HD `track solid, shaded` | 3,815 | 3,048 (-20 %) |
| HD race pass | 6,187 | 5,340 (-14 %) |
| HD `Scene::render GPU` | 11,570 | 10,667 (-8 %) |

**End to end, the binary with the spans alone against the one with all four changes**, A B
interleaved in one session, 3 rounds, medians of the rounds' medians, Intel iGPU, 1080p, load 2.6 to
4.4. HD's before binary reads in two clock modes (track shaded 6,166 to 7,261), so its row is the
one to take with a range; the after binary stayed at 3,076 to 3,307:

| us | HD before | HD after | Pulse before | Pulse after |
|---|---:|---:|---:|---:|
| `track solid, shaded` | 6,393 | 3,157 (-51 %) | 1,709 | 903 (-47 %) |
| race pass | 9,042 | 5,522 (-39 %) | 3,178 | 1,967 (-38 %) |
| `Scene::render GPU` | 14,847 | 10,789 (-27 %) | 5,561 | 4,217 (-24 %) |
| pipelined per frame | 6,088 | 4,200 (-31 %) | 1,887 | 1,365 (-28 %) |

`INTEL_DEBUG=fs`: Pulse's largest fragment shader 1,858 to 1,431 instructions, and all 67 shaders
now have a SIMD16 build (49 before; 29 before the emissive fold); HD's largest 4,849 to 4,343,
spills 111 to 80. Picture: `cmp` equal against the commit before on the same nine shots, and on HD
and Pulse Zone races (`--mode zone`, tick 2,400) against the binary from before this lane, which
is the case the fold must not touch. Deck: not run, Deck unavailable.

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

## The present composite's two dead `pow` (2026-11-10, `wesl-items`)

`upscale.wesl` runs once per presented frame in `resolve_scene` (the blit into the presentation
target) and once in `composite` (the graded copy to the surface), both over every output pixel.
Before, `fs_main` evaluated `srgb_decode` (three scalar `pow` a channel) for every pixel and threw
it away with a `select`, because `decode` is 0 on every real call site (ADR-0020 forces a non-sRGB
target, so the sampler or the pass before it has already decoded), and it raised the colour to
`grade.exponent` with a `pow` that is the identity at the neutral gamma.

**Result.** The decode sits behind `if grade.decode > 0.5` and the `pow` behind
`if grade.exponent == 1.0 { return ... }`. Pictures are byte-identical on both GPUs
(`cmp`, below); at a non-neutral grade the `pow` still runs.

GPU time of the pass, `Rgba8Unorm` target, microseconds, median of 4 interleaved processes of 95
timed runs each (`before` is `main` at `5feb0a961`; host load 1.7 to 2.9; **desktop GPUs, not the
Deck**). Intel is the Arrow Lake iGPU under Mesa ANV (the Deck proxy), NVIDIA the RTX PRO 2000
Blackwell laptop dGPU:

| Pass, output size | Intel before | Intel after | NVIDIA before | NVIDIA after |
|---|---|---|---|---|
| `resolve_scene` 1280x800 | 108.1 | 82.5 (-24 %) | 15.4 | 10.8 (-30 %) |
| `resolve_scene` 1920x1080 | 202.7 | 147.8 (-27 %) | 32.7 | 22.3 (-32 %) |
| `resolve_scene` 3840x2160 | 725.1 | 529.6 (-27 %) | 131.1 | 91.8 (-30 %) |
| `composite` 1280x800 | 116.3 | 88.7 (-24 %) | 16.6 | 10.6 (-36 %) |
| `composite` 1920x1080 | 206.0 | 149.2 (-28 %) | 34.3 | 21.4 (-38 %) |
| `composite` 3840x2160 | 756.0 | 528.0 (-30 %) | 140.6 | 90.0 (-36 %) |

A presented frame runs both passes, so at a 4K output the saving is about 0.42 ms a frame on the
Intel iGPU and 0.09 ms on the NVIDIA dGPU. The scout's estimate was 0.2 to 0.5 ms on the Deck;
the Deck measures more than that, below.

**On the Deck (2026-11-10, `deck-remeasure`).** `RADV VANGOGH`, AC, `powersave`, load 0.6 to 1.7,
HD Anulpha Pass, `--presented --ticks 60 --render-scale 100`, the throwaway timer below on both
arms, `dist` `znver2` builds of `59900b483` with and without this change's `upscale.wesl`. Arms
interleaved, 3 rounds, median of the 95 timed runs per process, us; the rounds agree within 0.5 %.
Method:
[frame-thread-cpu.md](frame-thread-cpu.md), "Four desktop GPU cuts on the Deck".

| Pass, output size | before | after |
|---|---|---|
| `resolve_scene` 1280x800 | 141.8 | 65.2 (-54 %) |
| `resolve_scene` 1920x1080 | 303.5 | 135.4 (-55 %) |
| `resolve_scene` 3840x2160 | 1178.2 | 513.2 (-56 %) |
| `composite` 1280x800 | 156.0 | 70.6 (-55 %) |
| `composite` 1920x1080 | 306.8 | 135.4 (-56 %) |
| `composite` 3840x2160 | 1193.2 | 515.6 (-57 %) |

Both passes run each presented frame, so the Deck saves 0.16 ms at 800p, 0.34 ms at 1080p and
**1.34 ms at a 4K output**. That is more than twice the scout's top estimate and three times
the Intel iGPU's saving.

**The Deck's picture is not byte-identical.** Same two builds, `cmp` of the `--presented`
screenshots: 452 of 1,024,000 pixels differ at 800p, 59 of 2,073,600 at 1080p and 387 of 8,294,400
at 4K, **every one by 1 of 255** (spread over the whole frame), identical across three runs of
each build. NVIDIA and Intel gave byte-identical frames (above). The likely cause is the skipped
`pow(x, 1.0)`, which RADV evaluates as `exp2(log2(x))` with the hardware's approximate
instructions, so the old picture carried a one-level rounding error the new one does not; the cause
was not isolated. 0.004 % of the pixels by 1 level is invisible; it matters only to a test that
compares a Deck capture byte for byte against one taken before this change.

**The form of the `pow` skip matters on Intel.** Three shapes of the same arithmetic, Intel 4K,
`resolve_scene` / `composite`: decode skipped and the `pow` left in 644.7 / 667.5; `var graded =
clamp(..); if exponent != 1.0 { graded = pow(..) }` **695.0 / 715.5 (slower than leaving the `pow` in)**;
the early return that landed 529.6 / 528.0. NVIDIA does not tell the three apart (4K `composite`
89.4, 86.6, 90.0). The shape is a code generation choice of the driver compiler, so a change here is
timed on a second vendor.

**Picture.** `cmp` of `--presented` captures, Pulse and HD, before against after: 1280x800, 3840x2160,
1920x1080 with `--reconstruction fsr1 --render-scale 67`, 1920x1080 with `fxaa`, a 4K capture with
`[display] brightness = 110, gamma = 130` (integers; a string value is ignored with a warning, and
the capture is then silently neutral, which is how a first attempt proved nothing), and a plain
non-presented capture. All byte-identical on NVIDIA and on Intel, so `exp2(log2(x))` and the skipped
`pow` agree on these frames. The `decode` arm is reached by no real call site; the existing sRGB
tests in `upscale/tests.rs` cover it. The new test
`the_neutral_grade_is_exact_and_a_gamma_off_neutral_follows_its_curve` pins the neutral levels
exactly and a gamma of 60 and 140 against the CPU curve.

**Method (throwaway, not committed).** The present crate has no GPU span for these two passes and
the HUD has none either. A static `QuerySet` that `present_view` and the UI renderer's frame pass
put in their `timestamp_writes`, and a loop in `race_capture.rs` that re-encodes the pass 100
times, each submit waited for, read the pairs back. Each reading is the pass's own GPU time, not a
serialised wall time.

### Zone textures, 9 fetches to 3: flat on the desktop, 6 to 7 % on the Deck, landed

`zone_sample` and `zone_glow` fetch the track and scene textures at both layers and `select`. Two
rewrites were built and timed: the layer picked with a per-lane `select(1i, 0i, inside)` (a
fetch each for the track and scene texture), and that plus a flat branch on `zone_is_track(slots)`
(one fetch). Neither paid on the desktop GPUs; the Deck run below reopened the second, which
landed.

- **Picture.** NVIDIA: byte-identical on five HD frames (four of them Zone) and on a Pulse Zone frame. Intel: **39 to 123 pixels of
  1 to 2 million differ by one or two levels** on all four HD Zone frames (ticks 1260, 1350, 1500;
  with and without `--zone-spectrum-test`), so the rewrite is not byte-identical there. The cause was not
  determined (a per-lane layer at the sphere boundary, or the driver re-scheduling the lighting
  arithmetic around the fetches, are both open). The differing pixels sit in one blob mid-frame, e.g.
  x 531 to 758, y 442 to 684 at tick 1260 (1280x800) and x 739 to 1066, y 493 to 809 at tick 1500
  (1920x1080); the two rewrites differ from `before` identically.
- **Time.** An upper bound was built to see whether the fetches matter at all: a binary whose
  `zone_sample` and band fetch return a constant (wrong picture on purpose), HD Talon's Junction, tick
  1500, `OAG_RENDER_GPU_BENCH=100`, `track solid, shaded`, median of process medians, microseconds:

  | | before | rewrite (flat branch, except where stated) | no Zone fetches (bound) |
  |---|---|---|---|
  | NVIDIA 1280x800 | 599 | 605 | 602 |
  | NVIDIA 1920x1080 | 552 | 545 | 543 |
  | NVIDIA 3840x2160 | 2165 | 2061 (-4.8 %) | 1959 (-9.5 %) |
  | Intel 1280x800 | 2504 | 2270 (-9 %) | 2099 (-16 %) |
  | Intel 1920x1080, batch of 8 | 3232 (spread 2911 to 4261) | 2887 (-11 %, layer select only) and 3368 (+4 %, flat branch) | |
  | Intel 1920x1080, bound batch of 5 | 3045 (spread 2443 to 4045) | | 2824 (-7 %) |
  | Intel 3840x2160 | 4790 (2 runs) | 4944 | 5507 (spread 3772 to 7241) |

  Removing every Zone fetch moves NVIDIA by 0 % at 800p and 1080p and 0.2 ms at 4K. The Intel iGPU
  swings 30 % between processes at the same size (clock state), more than any step here, and its
  1080p and 4K rows do not order. The "about a fifth of the race pass" comment in `shade.wesl` was
  read on an integrated GPU before the folds; the nine fetches are cached, small textures, and the
  pass is bound by other things now. The 0.3 to 2 ms the scout estimated is not there on these two
  GPUs, and the picture is not identical where it was tried (Intel). The Deck is below.

**On the Deck (2026-11-10, `deck-remeasure`).** `RADV VANGOGH`, AC, `powersave`, load 0.4 to 1.6,
HD Talon's Junction in Zone mode (`--mode zone --zone-spectrum-test`, tick 1500, `--render-scale
100`, `OAG_RENDER_GPU_BENCH=100`), `dist` `znver2` builds of `59900b483` with and without the
flat-branch rewrite (the layer a per-lane `select`, a branch on the chunk's flat track bit, one
fetch in `zone_sample` and one in `zone_glow`). Arms interleaved, 6 rounds (3 for the first
batch, then 3 more for 4K), medians of the 100 redraws, us. Method:
[frame-thread-cpu.md](frame-thread-cpu.md), "Four desktop GPU cuts on the Deck".

| Span | 800p before / after | 1080p before / after | 4K before / after |
|---|---|---|---|
| `track solid, shaded` | 2014 / 1864 (-7.4 %) | 3525 / 3323 (-5.7 %) | 8187 / 8131 (-0.7 %, noise) |
| race pass | 4188 / 4023 (-3.9 %) | 7602 / 7342 (-3.4 %) | 18650 / 18937 (+1.5 %, noise) |

At 800p and 1080p all six rounds order the same way (before 1995 to 2038 us against after 1848 to
1893 at 800p; before 3517 to 3532 against after 3320 to 3331 at 1080p), a saving of 0.15 and 0.20
ms on the Deck's frame, where the NVIDIA bound found nothing at these sizes and Intel showed 0.23
ms at 800p. The 4K row is not resolved: the Deck's 4K Zone rows swing 10 to 19 % between rounds (the
paired deltas run -1154, +235, -58, -37, +638, -167 us), more than the saving, as "The Deck's 4K
rows are unstable" says. The Deck's own 4K output is drawn at about 1906x1080
(`GAMESCOPE_PIXEL_BUDGET`), so the 1080p row is the nearer to what a player gets.

**The Deck's picture is byte-identical.** `cmp` (`md5sum`) of the screenshots of the two builds,
HD Talon's Junction at ticks 1260, 1350 and 1500, each with and without `--zone-spectrum-test`, at
1280x800 and 1920x1080 (12 frames), the tick 1500 spectrum frame at 3840x2160 in six rounds, and
Pulse's Zone frame (tick 2400) with and without the spectrum test at both sizes (4 frames): every
one equal, so the Intel difference (39 to 123 pixels by 1 to 2 levels) is the Intel driver's, not
the rewrite's arithmetic on RADV. The frames are real Zone frames (the Zone grid and glow are
drawn; the spectrum test changes tick 1350 and 1500, not 1260). NVIDIA was byte-identical too.
**Landed** as the flat-branch form: the Deck is the device this pass costs most on, both reopen
conditions held (a saving beyond the round spread, a byte-identical Deck picture). It is not
byte-identical on the Intel iGPU, where up to 123 of 1 to 2 million Zone pixels change by 1 or 2
levels.

### HUD fetches, 4 to 1: closed, flat

`ui.wesl`'s `fs_main` fetches the glyph atlas, sprite sheet, face atlas and buttons atlas for every
fragment. A version that branched on the flat `mode` (with `@diagnostic(off, derivative_uniformity)`)
compiled and was timed on the real race HUD, presented, tick 200, GPU time of the UI frame pass (the
sum of its passes), microseconds, median of 3 processes of 95 timed runs:

| | Pulse 800p | Pulse 1080p | Pulse 4K | HD 800p | HD 1080p | HD 4K |
|---|---|---|---|---|---|---|
| Intel before / after | 72.7 / 69.4 | 108.2 / 107.4 | 319.1 / 319.4 | 94.2 / 87.0 | 151.0 / 143.4 | 429.6 / 439.2 |
| NVIDIA before / after | 8.5 / 8.6 | 13.2 / 14.1 | 41.6 / 42.0 | 10.5 / 10.9 | 17.4 / 17.4 | 59.8 / 59.9 |

The whole HUD pass is 0.32 to 0.43 ms on the Intel iGPU at 4K and 0.04 to 0.06 ms on NVIDIA, so even
a free HUD cannot reach the scout's 0.05 to 0.3 ms; the change moved it by -7 to +10 microseconds in
both directions. Reverted. Pictures were not compared, since nothing landed.
