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
Pulse's Zone-mode frame (tick 2400) with and without the spectrum test at both sizes (4 frames,
which hash the same with and without the test, so they add little): every one equal, so the Intel difference (39 to 123 pixels by 1 to 2 levels) is the Intel driver's, not
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

## The 4K frame on the Deck pass by pass, one fold landed, seven cuts dropped (2026-11-18, `gpu-hires`)

The question: where a 3840x2160 frame's GPU time goes on the Steam Deck today (blur off, MSAA off),
and what can be cut. **Chosen workload, not measured against any original.**

**Result.** HD at native 4K is bound by the circuit's shading (48 % of the spans), the bloom
resolve (16 %) and the craft (13 %). One exact fold landed, 7.4 % off the circuit's shading at 4K
and 2.4 % off the whole-race GPU time (18.82 to 18.36 ms p50); seven other cuts were built,
measured and dropped, four of them on the craft shadow lookup. Native 4K still misses 60 Hz on HD
(frame p50 20.4 ms, p95 28.4): the internal resolution that holds it is 2573x1447 (67 %), frame
p50 11.4 ms and p95 15.0. Pulse holds 60 Hz at native 4K with room (4.3 ms p50, 6.6 p95). A
defect found on the way is fixed: the HD resolve drew every frame's bottom row and right column a
quarter too dark.

### Method

`RADV VANGOGH` (Mesa 26.2.0-devel), AC, `powersave`, GPU clock unpinned, offscreen over ssh, Deck
load 0.8 to 2.0. Release `-C target-cpu=znver2` `--features perf-probe` builds from the
`oag-appimage-build:bookworm` container; before is `main` at `550bffc24`. Spans:
`OAG_RENDER_GPU_BENCH=100 OAG_RENDER_PERF=1 oag-game <image> --race --mode single_race --seed 1
--autopilot --no-audio --size WxH --ticks 2500 --render-scale 100 --screenshot out.png`, the arms of
a comparison interleaved, 2 rounds; the rounds agree to 0.1 to 0.3 % at every size, 4K included.
Whole race: `scripts/bench-render.py run hd --binary ./oag-game --size WxH`. The second adapter is
this notebook's NVIDIA RTX PRO 2000 (host release build, same span command, load 2 to 11).

**The clock.** `amdgpu_top --json -s 250` beside every run: the serialised span bench sits at
1,040 MHz and a whole race at 1,600 MHz (133 to 154 of about 190 one-second samples of each 4K
race, the rest the load), so a race pays about 0.65 of an arithmetic-bound span and nearer all of
a bandwidth-bound one. `pipelined per frame` (frames back to back) runs at the race's clock and is
the row to plan from.

### Spans, defaults, us (medians of 2 rounds)

HD, the default circuit (Talon's Junction):

| span | 1280x800 | 1920x1080 | 3840x2160 | share at 4K | grows with pixels |
|---|---:|---:|---:|---:|---|
| track solid, shaded | 2,142 | 3,690 | 11,359 | 48 % | yes, arithmetic bound, 1.00x overdraw |
| hd encode (the bloom resolve) | 445 | 976 | 3,851 | 16 % | yes, 4.0x for 4x pixels |
| craft, weapons, shields | 677 | 1,090 | 3,080 | 13 % | yes, about 350 us fixed |
| track solid (depth prepass) | 628 | 764 | 1,431 | 6 % | partly |
| hd bloom downsample (x10 to x12) | 157 | 320 | 1,110 | 5 % | yes |
| track blended | 119 | 190 | 593 | 2 % | yes |
| between passes (x31 to x33) | 578 | 587 | 569 | 2 % | no |
| advert cards (x5) | 506 | 505 | 501 | 2 % | no |
| sun occlusion (x4), self shadow (x4), shadow atlas | 574 | 579 | 592 | 2 % | no |
| bloom gate and two blurs | 60 | 100 | 387 | 2 % | yes, quarter size |
| sum of spans | 5,959 | 8,936 | 23,847 | | |
| pipelined per frame | 5,670 | 6,753 | 16,748 | | |

The self-shadow passes draw 0 fragments in 4 passes (176 us): the per-craft passes are the open
item of the handover thread, not a 4K matter. Pulse (PSP EU default circuit): race pass 1,088 /
2,015 / 6,943 us; at 4K track shaded 3,114, track shine 1,507, track blended 1,012, prepass 599;
pipelined 3,008 / 2,911 / 5,171 us.

Whole race, HD, before (frame p50 / p95 / p99 ms; GPU p50 / p95): 1280x800 5.67 / 7.09 / 7.72 (3.79
/ 4.68), 1920x1080 7.79 / 10.02 / 12.03 (6.09 / 8.05), 2573x1447 11.60 / 15.13 / 18.53 (9.64 /
13.20), 2880x1620 13.35 / 17.80 / 21.80 (11.40 / 15.86), 3840x2160 20.83 / 28.79 / 36.35 (18.82 /
26.87; scene 14.59, bloom chain 4.00). Pulse: 2573x1447 3.08 / 3.93 / 4.70, 2880x1620 3.22 / 4.25
/ 5.72, 3840x2160 4.32 / 6.57 / 9.54. A second 4K HD run of the same binary read 20.84 / 28.84
(GPU 18.79 / 26.90), so a difference over 0.2 % is not noise here. Every run finishes at tick
7,020 with state hash `94a2261ef5045e63`.

### The hot shader

`RADV_DEBUG=shaderstats` over an HD race compiles 131 pixel shaders; the heaviest is 5,943
instructions at 168 VGPRs (it was 3,886 at 128 on 2026-11-05: the materials added since grew the
rich pipelines by half). The one that matters is the circuit's plain pipeline
(`Prepass::plain`, 95 % of the circuit's pixels and 69 % of the screen on this frame, counted on a
false-colour frame at 1280x800): **2,039 instructions, 40 VGPRs, 24 waves a SIMD**, found by
diffing the statistics of two builds that differ only there. 1,200 of those instructions are the
shadow term: the craft lookup unrolled eight times and the `mapped` tier's eight depth taps, each
twice over (once for each arm of the wet road test). A pixel executes about 650 (an estimate: 48
always-executed instructions were 7.4 % of the time, below). Each `pow` is a `v_log_f32`, a
multiply and a `v_exp_f32`, and by the compiled blocks a pixel now runs fourteen: the lightmap
curve (3), the vertex light curve (3), the albedo's sRGB decode (3), the specular (1), the fog (1)
and the target encode (3).

Timing arms on the same binary (a pipeline constant read from an environment variable, not
committed, wrong pictures on purpose), `track solid, shaded`, us:

| arm | 1920x1080 | 3840x2160 | pipelined at 4K |
|---|---:|---:|---:|
| full shader | 3,632 | 11,264 | 16,341 |
| constant colour | 833 | 1,913 | 7,299 |
| albedo times lightmap, fogged, shadowed, encoded | 1,713 | 4,348 | 10,010 |
| no target encode | 3,483 | 10,692 | 15,990 |
| no decode of the stand-in result | 3,487 | 10,725 | 15,940 |
| no fog | 3,503 | 10,642 | 15,861 |
| no shadow term at all | 3,138 | 9,383 | 14,968 |
| the shadow lookup kept, its sun term a constant | | 10,892 | 16,408 |
| the sun term kept, the lookup gone | | 10,009 | 15,835 |
| the stand-in result alone (no rig path) | 2,020 | 5,643 | 10,894 |

### Landed: the plain draws that the rig lights at every vertex drop the stand-in result

`lit_texel` ends on `mix(stand-in, rig, enabled * lit)`. HD's rig is on for the whole race and
every vertex of the circuit's plain draws carries `lit` 1.0, so every pixel weighed the rig's
result at one, and the stand-in result with the sRGB decode of it (three `pow`) was computed and
thrown away. `mesh_render::specialise::plain_is_lit` checks the vertices, the build scope
remembers that its light is an authored rig (`BuildCacheScope::lit_by`), and the plain pipeline is
built with `lit_by_rig`, under which the mix is its second argument alone. 48 of the 2,039
instructions go. Only the prepassed circuit's plain half is told: other drawables of a race draw
under scene blocks of their own, which this lane did not audit.

| Deck, us, medians of 2 rounds | 1280x800 | 1920x1080 | 3840x2160 |
|---|---|---|---|
| `track solid, shaded` | 2,143 to 2,056 (-4.1 %) | 3,688 to 3,485 (-5.5 %) | 11,358 to 10,523 (-7.4 %) |
| race pass | 3,653 to 3,562 | 5,852 to 5,658 | 16,806 to 15,970 |
| sum of spans | 5,970 to 5,881 | 8,940 to 8,744 | 23,865 to 23,026 (-3.5 %) |
| pipelined per frame | 5,640 to 5,663 (CPU bound) | 6,779 to 6,660 (-1.8 %) | 16,753 to 16,227 (-3.1 %) |

| Deck, whole race, ms | 1280x800 | 1920x1080 | 2573x1447 | 3840x2160 |
|---|---|---|---|---|
| frame p50 | 5.67 to 5.56 | 7.79 to 7.69 | 11.60 to 11.36 | 20.83 to 20.37 (-2.2 %) |
| frame p95 | 7.09 to 7.04 | 10.02 to 9.88 | 15.13 to 15.05 | 28.79 to 28.42 (-1.3 %) |
| GPU p50 | 3.79 to 3.74 | 6.09 to 5.99 | 9.64 to 9.44 | 18.82 to 18.36 (-2.4 %) |
| GPU p95 | 4.68 to 4.66 | 8.05 to 7.95 | 13.20 to 13.05 | 26.87 to 26.58 |

The 4K pair was run twice, before, after, before, after: frame p50 20.830, 20.372, 20.840, 20.361
ms and GPU p50 18.815, 18.355, 18.789, 18.317 (`scripts/bench-render.py compare`: GPU p50 -2.4 %,
p95 -1.1 %, draws and triangles equal).

NVIDIA, `track solid, shaded`, us, 2 rounds: 354 to 355, 545 to 521 (-4.4 %), 1,525 to 1,537 (its
4K rounds spread 1,489 to 1,561, so read as no change); pipelined flat at every size.

**Picture.** Not byte-identical, by the driver's float order: on the Deck 0 of 1,024,000, 1 of
2,073,600 and 3 of 8,294,400 pixels differ, each by 1 of 255 (tick 2,500, two rounds equal). On
NVIDIA 1 to 15 of 2,073,600 at ticks 300, 600, 1,500 and 2,500. Against the binary before this
lane, 1280x800, ticks 1,200 and 2,500, NVIDIA: Pulse and Pure identical; 2048 and Omega 1 to 2
pixels by one level (their plain halves fold too); HD 0 to 2 pixels by one level besides the edge
fix below.

### Fixed on the way: the resolve's bottom row and right column

The half-texel resolve takes its bilinear read by hand (`resolve_scene_tap`), and clamped the far
texel to `floor(scene_max * size)`. A whole picture's `scene_max` is 1.0 (`sub_rectangle_at`), so
that was one texel past the last row and column, and a load from there reads zero: both edges
came out at half their light (a sky row read `(34, 88, 123)` under `(50, 121, 169)`). Now clamped
to the last texel, as a clamped sampler reads it;
`a_whole_pictures_last_row_and_column_keep_their_light` fails without the fix. 690 to 725 pixels of
a 1280x800 HD frame change, all in the last row and column, by up to 61 levels.

### Dropped: the resolve through a decoded copy of the scene

The resolve decodes each scene texel four times
([frame-thread-cpu.md](frame-thread-cpu.md), "HD encode: three exact cuts measured, none kept";
its untried cut 3). Built: one pass decodes the scene texel for texel into an `Rgba16Float`
target, and the resolve takes one hardware-filtered tap of that.

| us | 1280x800 | 1920x1080 | 3840x2160 |
|---|---|---|---|
| Deck `hd encode`, before | 445 | 976 | 3,850 |
| Deck decode + encode, after | 155 + 148 | 339 + 326 | 1,141 + 1,189 (-39 %) |
| Deck pipelined per frame | 5,627 to 5,727 | 6,757 to 6,777 | 16,771 to 16,345 (-2.5 %) |
| NVIDIA encode, before / decode + encode | 42 / 20 + 13 | 101 / 48 + 33 | 498 / 258 + 222 |

At the race's clock the two new passes are half bandwidth, which the clock does not speed up, so
1.5 ms of span is 0.43 ms of frame, and NVIDIA gains nothing at 4K. The picture is not the same:
the linear value is rounded to f16 before the filter, and 5.4 % of an HD frame's pixels move by 1
of 255 (112,575 of 2,073,600). This pass has been held to byte identity, so it is dropped. The
exact form needs an `Rgba32Float` copy, twice the traffic of a form that already gains only 0.43
ms; not built.

### Dropped: four cuts of the craft shadow lookup

The lookup (`craft_shadow_coverage`, eight craft a pixel) is 1,342 us of the 11,350 at 4K by the
arms above, 0.9 ms of a frame back to back. Four exact ways to do less of it were built; every
picture stayed byte-identical on the Deck and on NVIDIA, and none paid:

| cut | Deck `track solid, shaded` at 4K, us |
|---|---|
| the same eight sphere tests first, with no branch between them, and an early return | 11,360 to 11,314 |
| a pipeline with no lookup for the draws whose bounds clear every craft's sphere | 11,360 to 11,328 |
| a per-triangle craft mask from the vertex stage, for draws of triangles under 32 units | 11,364 to 11,514 (+1.3 %) |
| the sphere fitted to the receiver's slab (33.5 units round its middle, not 56 round the craft) | 11,371 to 11,338 |

Why: a false-colour frame of the mask says 51.5 % of the screen is circuit within reach of some
craft and 1.3 % is not (a pack, tick 2,500), so nothing is culled, and the compiled loop already
rejects a craft in seven instructions and one scalar load. The lookup's cost is its eight tests,
and those run for every pixel whatever is fitted round them. What would remove it is the
original's own structure: redraw the chunks near a craft through the `ShadowToAlpha` permutation,
blended (`Shadow_CompileShadowedTrackRedraw`), so no other pixel asks. That composites on encoded
values, as the console does, so it moves the shadowed pixels and is not a speed change alone.

### Dropped: the two light curves behind their conditions

The lightmap curve and the vertex light curve (three `pow` each) both run for every pixel and a
`select` keeps what the chunk's bits ask for. Behind `if`s on those flat bits: `track solid,
shaded` 10,528 to 10,562 us at 4K, 3,458 to 3,499 at 1080p, byte-identical. Most chunks need both.

### What a 4K output costs after this, and what holds 60 Hz

HD, whole race, after: 3840x2160 20.37 / 28.42 ms (frame p50 / p95), 2573x1447 11.36 / 15.05,
1920x1080 7.69 / 9.88. The presented resolve adds about 1.0 ms at a 4K output (two passes of 0.51
ms, above) and the HUD. So **HD holds 60 Hz at a 4K output at 67 % (2573x1447) on the median and
on p95 with about 0.6 ms to spare before the HUD, not at 75 % (p95 17.8 ms before the fold) and not
natively.**
Gamescope's default for a 4K output stays 1906x1080 (`GAMESCOPE_PIXEL_BUDGET`): 67 % has no
margin for a heavier circuit, and this is one circuit. Pulse can be given native 4K.

### Not captured

An RGP trace: `MESA_VK_TRACE=rgp MESA_VK_TRACE_FRAME=600` on the game under `gamescope --backend
headless` (`ENABLE_GAMESCOPE_WSI=0`) wrote no `.rgp` file, and the race sat in `race-paused`
there without `pause_on_focus_loss = false`. Omega and 2048 were not run on the Deck (it holds
the HD, Pulse and Pure images only).

## HD's scene in 8 bits: the resolve in one hardware tap (2026-11-18, `hd-scene-8bit`)

The question: what the original's own 8-bit `A8R8G8B8` scene target saves, read through an sRGB
view so the bloom resolve is one hardware-filtered tap rather than four sRGB decodes (the lever
"Dropped: the resolve through a decoded copy" above pointed at). It landed as the default,
`graphics.scene_precision = original`; `float` is the target before. Picture and pairs:
[hd-track-tone.md](hd-track-tone.md), "The scene target is 8-bit by default" (the pairs stay put,
`float` is byte-identical to the build before).

**Result on the NVIDIA RTX PRO 2000 (this notebook's dGPU).** The resolve span falls to a third
at every size, the bloom downsample by a fifth, and the scene's own passes write half the bytes;
at 4K the whole race's GPU time falls 17 % on the median.

Method: `OAG_RENDER_GPU_BENCH=100 OAG_RENDER_PERF=1 oag-game <HD image> --race --mode single_race
--seed 1 --autopilot --no-audio --size WxH --ticks 2500 --render-scale 100 --screenshot out.png`,
release `perf-probe` builds of `a9466c367` (before) and this change, the three arms interleaved,
3 rounds, medians of the rounds' medians, load 1.8 to 3.1. `float` is this change's binary with
`--scene-precision float`.

| us, NVIDIA | 1280x800 before / 8-bit / float | 1920x1080 | 3840x2160 |
| --- | --- | --- | --- |
| `hd encode` (the resolve) | 48 / 14 / 48 | 113 / 36 / 121 | 556 / 208 / 550 |
| `hd bloom downsample` | 35 / 31 / 35 | 54 / 46 / 55 | 164 / 120 / 172 |
| `track solid, shaded` | 404 / 410 / 401 | 601 / 610 / 614 | 1,775 / 1,672 / 1,714 |
| craft, weapons, shields | 104 / 106 / 104 | 150 / 152 / 154 | 490 / 459 / 469 |
| sum of spans | 1,269 / 1,244 / 1,267 | 1,681 / 1,606 / 1,701 | 4,121 / 3,547 / 4,018 |
| pipelined per frame | 2,239 / 2,156 / 2,212 | 2,812 / 2,790 / 2,977 | 7,334 / 6,124 / 7,398 |

Whole race, `scripts/bench-render.py run hd --binary <bin> --size WxH` (one run each, 7,019
frames both, load 2.1 to 3.4), before to after, ms:

| NVIDIA | 1280x800 | 1920x1080 | 3840x2160 |
| --- | --- | --- | --- |
| GPU p50 | 1.18 to 1.13 (-4.7 %) | 2.21 to 1.99 (-10.0 %) | 7.22 to 6.01 (-16.8 %) |
| GPU p95 | 1.87 to 1.77 (-5.2 %) | 3.23 to 2.99 (-7.5 %) | 9.94 to 8.64 (-13.2 %) |
| frame p50 | 2.12 to 2.06 | 2.94 to 2.74 | 8.03 to 6.72 (-16.3 %) |

**Result on the Steam Deck.** The resolve falls to 28 % of its cost at every size (3.88 to 1.10 ms
at 4K, against about 1.2 ms the lever's estimate expected), the bloom downsample by 30 %, and the
whole race's GPU time by 6 % at 800p, 10 % at 1080p and 13 % at 4K on the median.

Method: `RADV VANGOGH`, AC, GPU clock unpinned and sampled with `amdgpu_top --json` beside every
run (spans at a busy median of 1,040 MHz, whole races at 1,600 MHz for 130 to 145 of about 160
one-second samples at 4K), Deck load 0.8 to 2.8, offscreen over ssh under the Deck's lock. One
binary, the `oag-appimage-build:bookworm` container's release `perf-probe` build with `-C
target-cpu=znver2`; **before is the same binary with `--scene-precision float`** (byte-identical
to the build before on every HD pair), after is the default. Spans: this page's "Method" command,
the two arms interleaved (ABBA), 2 rounds, which agree within 0.3 %.

| us, Deck | 1280x800 float / 8-bit | 1920x1080 | 3840x2160 |
| --- | --- | --- | --- |
| `hd encode` (the resolve) | 448 / 130 | 984 / 284 | 3,881 / 1,104 (-72 %) |
| `hd bloom downsample` | 157 / 118 | 320 / 228 | 1,108 / 776 |
| `track solid, shaded` | 2,057 / 2,018 | 3,489 / 3,468 | 10,524 / 10,490 |
| craft, weapons, shields | 678 / 674 | 1,091 / 1,090 | 3,084 / 3,077 |
| sum of spans | 5,876 / 5,412 | 8,749 / 7,943 | 23,026 / 19,866 (-13.7 %) |
| pipelined per frame | 5,754 / 5,695 (CPU bound) | 6,651 / 6,124 | 16,249 / 14,138 (-13.0 %) |

Whole race, `scripts/bench-render.py run hd --binary <arm> --size WxH`, 7,019 frames in every run,
ms, float to 8-bit; the 4K pair was run twice (float, 8-bit, float, 8-bit), and the two agree:

| Deck | 1280x800 | 1920x1080 | 3840x2160, run 1 | 3840x2160, run 2 |
| --- | --- | --- | --- | --- |
| GPU p50 | 3.76 to 3.53 (-6.0 %) | 6.00 to 5.42 (-9.7 %) | 18.33 to 15.97 (-12.9 %) | 18.48 to 15.98 (-13.5 %) |
| GPU p95 | 4.63 to 4.40 (-5.1 %) | 7.95 to 7.34 (-7.7 %) | 26.62 to 24.36 (-8.5 %) | 26.98 to 24.38 (-9.6 %) |
| frame p50 | 5.56 to 5.51 | 7.76 to 7.28 (-6.2 %) | 20.43 to 18.06 (-11.6 %) | 20.37 to 18.04 (-11.5 %) |
| frame p95 | 7.01 to 6.88 | 9.95 to 9.33 (-6.3 %) | 28.47 to 26.30 (-7.6 %) | 28.76 to 26.34 (-8.4 %) |

The float arm reproduces the page's earlier 4K figures (GPU p50 18.36 ms after the `lit_by_rig` fold,
above), so the float arm is the before. Native 4K on the Deck still misses 60 Hz (frame p50 18.1 ms,
p95 26.3).

### Not landed: opaque draws writing through an sRGB attachment view

With the scene 8-bit, the circuit's opaque draws could write linear light through an
`Rgba8UnormSrgb` attachment view and drop `target_out`'s three `pow`; the blended draws must keep
blending encoded values, so they need the race pass split in two by attachment view. The bound,
measured with a timing arm that drops the encode from every mesh pipeline (wrong picture on
purpose, not committed): NVIDIA 4K `track solid, shaded` 1,540 to 1,491 us and craft 441 to 402 us,
medians of 3, about 1.5 % of a 6.2 ms frame before the second pass's cost; the Deck's own arm
above ("no target encode") is 0.57 ms of a serialised 11.3 ms span, about 0.37 ms at the race's
clock. Not worth a split race pass on this evidence; it stays recorded.

## 2048 and Omega on the Deck, pass by pass; two exact cuts landed (2026-11-18, `deck-vita-ps4-perf`)

The question: where the GPU time of Wipeout 2048 (Vita EU, Altima) and the Omega Collection (PS4
EU, Tech de Ra) goes on the Steam Deck at 1280x800, 1920x1080 and 3840x2160, blur and MSAA off,
and what can be cut without changing the picture. **Chosen workload, not measured against any
original.**

**Result.**

- **At 800p (the Deck's own screen) both titles are bound by the CPU.** Whole-race frame p50
  sits on `cpu_us`: 2048 2.50 ms against 2.02 ms of CPU, Omega 4.50 against 3.72. A GPU cut there
  buys battery, not frames.
- **At 1080p (docked)** 2048 is still CPU-bound (3.01 ms frame, 2.31 CPU). Omega is near balance
  (6.21 frame, 4.09 CPU).
- **2048 holds 60 Hz at native 4K** (frame p50 8.50 ms, p95 12.10 offscreen, about 13 with a
  presented output). Its circuit's shading is 56 % of the spans there.
- **Omega missed 60 Hz at native 4K on p95 only** (p50 13.44, p95 19.91 ms). Its circuit's
  blended list (the road's decal layers) was 30 % of the spans and shading-bound. One cut,
  exact by construction, takes that list from 6.70 to 2.55 ms at 4K. **Omega's whole-race frame
  p95 at native 4K went from 19.85 to 15.92 ms offscreen.** That is at the 60 Hz line, not under
  it: a presented 4K output adds about 1 ms of resolve, composite and HUD (HD's figure, "What a
  4K output costs after this" above), which was not measured for Omega. Docked at 1080p the same
  cut takes Omega's frame p50 from 6.18 to 5.60 ms (-9.4 %).
- The second cut, 2048's exposure readback, is exact but below what a whole race resolves.

### Method

`RADV VANGOGH`, AC, offscreen over ssh under the Deck's lock, Deck load 0.6 to 3.5. Release
`--features perf-probe` builds with `-C target-cpu=znver2` from the
`oag-appimage-build:bookworm` container; before is `main` at `72c135ee7`. The Deck's
`~/.local/share/oag/extracted` holds both titles: 2048 is `vita/PCSF00007` and Omega is
`ps4/` (`omega-eu` and `omega-eu-patch`). Fragment counts per span agree with
the host's (`omega-eu-merged`) to 0.3 %, so both adapters run the same workload.

- **Spans**: this page's span command (`OAG_RENDER_GPU_BENCH=100 OAG_RENDER_PERF=1 ...
  --ticks 2500 --render-scale 100`), the arms interleaved, 2 rounds. The rounds agree within
  0.5 % at every size. The serialised span bench runs at a busy median of 1,040 MHz.
- **Whole race**: `scripts/bench-render.py run <title> --binary <bin> --size WxH`, then
  `compare`. The busy median clock is 1,600 MHz at 1080p and 4K. At 800p the pair read 1,435
  and 1,512 MHz; those runs are CPU-bound, so read them loosely.
- **The whole race's `gpu_us` covers only the race pass, the motion blur and HD's bloom**
  ([render-bench.md](../tools/render-bench.md)). It holds neither 2048's `wo composite` chain nor
  Omega's tone map and bloom. A change in those chains shows in `frame_us` and the spans only.
- **Second adapter**: this notebook's NVIDIA RTX PRO 2000, a host release `perf-probe` build,
  the same span command, 2 rounds, host load 3 to 33. Its whole-race runs swung -25 % to +34 %
  on `gpu_us` between two binaries that differ only outside it (load 4 to 20 during them), so
  they are not used.

### Where the time goes, Deck spans, us, defaults (medians of 2 rounds, 1,040 MHz)

2048, Altima, tick 2,500:

| span | 1280x800 | 1920x1080 | 3840x2160 | share at 4K |
|---|---:|---:|---:|---:|
| track solid, shaded | 1,037 | 1,945 | 6,727 | 56 % |
| craft, weapons, shields | 234 | 461 | 1,610 | 13 % |
| wo composite (the vignette composite) | 132 | 291 | 1,128 | 9 % |
| track solid (depth prepass) | 238 | 335 | 896 | 7 % |
| wo composite gate | 88 | 199 | 770 | 6 % |
| wo composite step (exposure readback) | 36 | 89 | 273 | 2 % |
| wo composite blur (x2) | 30 | 55 | 194 | 2 % |
| advert card | 163 | 163 | 162 | 1 % |
| between passes (x8) | 99 | 99 | 107 | 1 % |
| sum of spans | 2,106 | 3,760 | 12,100 | |
| pipelined per frame | 2,651 | 3,055 | 8,807 | |

Omega, Tech de Ra, tick 2,500:

| span | 1280x800 | 1920x1080 | 3840x2160 | share at 4K |
|---|---:|---:|---:|---:|
| track solid, shaded | 2,365 | 3,344 | 8,380 | 38 % |
| track blended (the road's decal layers) | 854 | 1,794 | 6,707 | 30 % |
| track solid (depth prepass) | 1,205 | 1,368 | 2,033 | 9 % |
| omega tonemap apply | 198 | 422 | 1,623 | 7 % |
| omega bloom (bright, 5 blurs, 3 downs, 3 adds) | 572 | 667 | 1,141 | 5 % |
| craft, weapons, shields | 199 | 297 | 864 | 4 % |
| advert card (x5) | 505 | 501 | 500 | 2 % |
| between passes (x30) | 390 | 394 | 397 | 2 % |
| exhaust, sparks, clouds, ghost | 43 | 85 | 294 | 1 % |
| omega tonemap luma and halve (x8) | 96 | 151 | 201 | 1 % |
| sum of spans | 6,437 | 9,056 | 22,271 | |
| pipelined per frame | 5,386 | 7,141 | 16,247 | |

Whole race, Deck, before, ms (frame p50 / p95; CPU p50; `gpu_us` p50):

| | 1280x800 | 1920x1080 | 3840x2160 |
|---|---|---|---|
| 2048 (13,040 frames, hash `570a234b8d1de611`) | 2.50 / 3.07; 2.02; 1.44 | 3.01 / 4.01; 2.31; 2.02 | 8.50 / 12.10; 3.35; 6.02 |
| Omega (6,776 frames, hash `8967e698d2f4bb3b`) | 4.50 / 5.67; 3.72; 2.92 | 6.21 / 7.68; 4.09; 4.43 | 13.44 / 19.91; 4.67; 9.88 |

NVIDIA, the same spans at 4K, us: 2048 shaded 955, composite 231, step 331, gate 159. Omega
shaded 1,631, blended 1,124, prepass 341, tone map apply 276, bloom bright 207.

What carries over from HD's work and what does not:

- **Carries over**: the circuit folds (`slots_present`, the emissive fold, `Prepass::plain`, the
  rig and Zone folds, `lit_by_rig`) are generic and already apply to both titles.
- **Does not apply**: HD's 8-bit scene and its resolve are HD's alone. 2048 has the `wo_*`
  chain and Omega its tone map and bloom.
- **Whether an 8-bit scene would be the faithful default** (recorded, not assumed):
  - **Omega: no.** Its programs export fp16 into a float target that its tone map reads
    ([hd-track-tone.md](hd-track-tone.md), "Other titles"), so `Rgba16Float` is the original's
    precision.
  - **2048: no.** The original's target is `U2F10F10F10`, a 32-bit packed float
    (`oag_post::wo_composite`), not 8-bit. wgpu's nearest format, `Rg11b10Ufloat`, has other
    mantissa widths and no alpha, so it would move pixels. It is also not needed: 2048 already
    holds 60 Hz at native 4K.

### Landed: a circuit's plain blended draws shade through pipelines of their own

`Prepass::plain` gave a circuit's opaque draws that name no emissive row and no weapon role a
pipeline told so. The blended list kept the model-wide pipelines.

**Bisect.** A throwaway timing arm on `fs_main_blend` (not committed), Deck, Omega, us:

| arm | 1920x1080 | 3840x2160 |
|---|---:|---:|
| full shader | 1,795 | 6,703 |
| constant colour, blending on | 165 | 597 |
| colour write off | 41 | 146 |

So the decals were shading-bound: 91 % of the span was the shader, not the blend or the
bandwidth.

**Census.** 162 of Tech de Ra's 166 blended draws (124,020 of 124,296 indices) name no row and no
weapon role, and the OR of their `slots` is `0x1`. The four that name a row carry `0x3000100`
over 276 indices.

**First try, dropped.** Telling the blended pipelines the transparent list's own OR alone moved
nothing: 6,706 against 6,706 us at 4K. The four row-naming draws keep the emissive table in that
OR.

**Landed.** `specialise::plain_transparent_draws` splits the blended list as `plain_draws` splits
the opaque one. `mesh_render::PlainBlend` holds the four blended pipeline pairs again (alpha
over, additive, none, each authored equation), built with that half's `slots_present`, its
`emissive_rows` and, under a rig that lights every one of its vertices, `lit_by_rig`.
`Drawable::draw_lists_into` picks per draw and keeps the draw order. The exactness argument is the
opaque one: each draw's bits are a subset of what its pipeline is told. The split is for a
prepassed model (a circuit) only, the same scope as `Prepass::plain`.

| Deck, us, 2 rounds | 1280x800 | 1920x1080 | 3840x2160 |
|---|---|---|---|
| Omega `track blended` | 853 to 320 (-62 %) | 1,795 to 678 (-62 %) | 6,704 to 2,553 (-62 %) |
| Omega race pass | 4,663 to 4,142 | 6,913 to 5,813 | 18,459 to 14,270 (-23 %) |
| Omega pipelined per frame | 5,396 to 4,993 (-7.5 %) | 7,126 to 6,418 (-9.9 %) | 16,264 to 13,490 (-17.1 %) |
| HD `track blended` | 118 to 108 | 190 to 144 | 592 to 357 (-40 %) |
| HD pipelined per frame | 5,708 to 5,621 | 6,128 to 6,116 | 14,146 to 13,985 (-1.1 %) |

Omega, whole race on the Deck, `bench-render.py compare`, ms. The 4K pair was run twice and the
two agree to 0.1 ms. Every run finished at tick 6,777 with hash `8967e698d2f4bb3b`.

| Omega | 1280x800 | 1920x1080 | 3840x2160, run 1 | 3840x2160, run 2 |
|---|---|---|---|---|
| frame p50 | 4.55 to 4.34 (-4.6 %) | 6.18 to 5.60 (-9.4 %) | 13.28 to 12.92 (-2.8 %) | 13.41 to 12.92 (-3.6 %) |
| frame p95 | 5.74 to 5.60 | 7.69 to 7.01 (-8.8 %) | 19.72 to 15.86 (-19.6 %) | 19.85 to 15.92 (-19.8 %) |
| `gpu_us` p50 | 2.93 to 2.56 | 4.42 to 3.69 (-16.6 %) | 9.87 to 9.36 (-5.1 %) | 9.88 to 9.36 (-5.2 %) |
| `gpu_us` p95 | 4.10 to 4.04 | 5.75 to 4.97 (-13.5 %) | 16.29 to 12.42 (-23.8 %) | 16.32 to 12.41 (-24.0 %) |

NVIDIA spans, us: Omega `track blended` 266 to 78 at 1080p and 1,051 to 330 at 4K (-69 %). HD's
went 106 to 54 at 4K.

**Picture.**

- **Deck**: HD identical at all three sizes. Omega differs by one level in 1, 4 and 5 pixels at
  800p, 1080p and 4K.
- **NVIDIA**: HD (race at ticks 1,200 and 2,500, Zone at 1,500), Pulse, Pure and 2048 are
  identical. Omega at 2,500 with `--msaa 4x` and at 1,200 is identical. Omega at 2,500 and 4,000
  differs in 0 to 15 pixels by one level.
- **Why**: the same float-order rounding as the opaque split above. Every pixel was compared
  decoded, because the PNG bytes of two binaries differ in their metadata.
- **2048: checked, not timed.** Its pictures are identical. Whether Altima's blended list splits
  at all was not checked. Its opaque list names no row (the census table above), so its blended
  list most likely has nothing to split. Either way that list is 594 fragments, so there is
  nothing to gain.
- **Load cost not measured.** The split adds up to two pipelines for each blended class and each
  authored equation of a circuit. The cold shader cache race load (`MESA_SHADER_CACHE_DISABLE=true
  --ticks 1`, as for the opaque split) was not run.
- **The 2048 speckle fix** (`shade.wesl`'s sun-mask tests at -0.5 and 1.5) landed on `main` after
  these numbers. It moves two comparison constants and no instruction. The 2048 rows were not
  re-measured; to re-measure, rerun the span command on a build of `main`.

### Landed: 2048's exposure readback over 256 invocations

`cs_adapt`/`cs_prime` read the gate target's alpha at every fourth pixel of every fourth row
(`Exposure_AdaptFromReadback`). They used one 64-invocation workgroup, each invocation walking
whole rows: about 500 dependent loads an invocation at 4K, a latency-bound pass. Now the sampled
grid is flattened and dealt to 256 invocations, four independent loads at a time, and summed in a
tree in `u32`. Every partial sum is a whole number, so the order of the additions cannot change
the mean. Below 2^24 the old `f32` sum was exact too, and 4K's largest total is 8.3 million. The
test `the_readback_is_the_mean_of_every_fourth_gate_byte` pins it on known gate bytes over a
63 x 38 grid that does not divide among the invocations.

| `wo composite step`, us | 1280x800 | 1920x1080 | 3840x2160 |
|---|---|---|---|
| Deck (2 rounds) | 36 to under 20 | 88 to 28 | 269 to 94 |
| NVIDIA (2 rounds) | 24 to under 20 | 53 to under 20 | 325 to 78 |

Deck pipelined per frame 2,596 to 2,577, 3,060 to 3,019 and 8,833 to 8,737 us (-1.1 % at 4K).
Whole race: 2048's frame p50 moves -0.8 to -1.2 % at 1080p and 4K, within one run's noise, and
`gpu_us` cannot show it. Pictures are identical, decoded, on the Deck at all three sizes and on
NVIDIA. The state hash is the same.

**Omega: checked, differs.** Its exposure (`cs_coefficients`) reads the one texel at the end of a
256-square luma ladder, 5 us, and has no readback loop to widen.

### Ranked, not taken

Estimates are **estimates, not measurements**.

1. **Omega's depth prepass: 1.2 to 2.0 ms, nearly flat with size.**
   - What: it writes no fragments, so it is bound by vertices (about 413 k triangles a frame,
     drawn by both halves).
   - Lever: a position-only vertex entry for the prepass. That needs `@invariant` positions and a
     proof that the depth-equal test still passes on every vertex.
   - Measure: the `track solid` span at 800p, where it is 19 % of the spans.
2. **Omega's tone map apply, 1.6 ms at 4K.**
   - What: three fetches (distortion, scene, bloom), the curve and the piecewise sRGB encode for
     every pixel.
   - Lever: skip the distortion fetch where the distortion target is clear. It holds only what
     a refractive draw wrote.
   - Estimate: 0.2 to 0.4 ms at 4K.
   - Measure: the `omega tonemap apply` span.
3. **2048's composite and gate, 1.9 ms at 4K together.**
   - What: the composite runs once per output pixel as the vignette mesh, and the gate takes four
     scene taps per quarter pixel.
   - Lever: none found that keeps the picture. 2048 already holds 60 Hz at native 4K.
4. **The circuits' opaque shading** (2048 6.7 ms and Omega 8.4 ms at 4K) is what is left.
   `RADV_DEBUG=shaderstats` on their plain pipelines has not been read.
5. **Tooling: the race bench's `gpu_us` misses 2048's and Omega's post chains.** Timestamps around
   `wo_composite` and the Omega chain would let a whole race show a cut there.

### RGP capture

One capture was obtained, unlike the HD attempt above, with this command:

```sh
gamescope --backend headless -W 1920 -H 1080 -w 1920 -h 1080 -- env MESA_VK_TRACE=rgp MESA_VK_TRACE_FRAME=400 oag-game <omega> --race --autopilot --no-audio --frame-log --script 15:quit
```

The settings file said `pause_on_focus_loss = false`. The trace landed in `/tmp` as
`<binary>_<date>_frame401.rgp` (30 MB): Omega at 1080p, on the build after the blended split. It
is not committed (it is a capture of game content). The race at frame 401 is early in its first
lap, and nothing above is read from it.

## Circuit shading: exact cuts, the vertex curve and a lighting rung (2026-11-18, `circuit-lighting-rungs`)

The question: what of the circuit's shading (`track solid, shaded`, the biggest pass on HD, 2048
and Omega at 4K) can go exactly, in the original's own order, and what a player-chosen cheaper
lighting saves against a lower render scale. **Chosen workload, not measured against any
original.**

**Result.** Seven exact cuts (and the original's own order for the vertex light curve) take the
Deck's `track solid, shaded` at 4K from 10.49 to 8.65 ms on HD (-17.6 %), 6.71 to 5.15 on 2048 (-23.3 %)
and 8.38 to 6.32 on Omega (-24.6 %), and Omega's `track blended` from 2.55 to 1.64; the pictures are
byte-identical except HD's vertex-lit chunks, which the original's order moves. One option landed,
**`graphics.circuit_lighting`** (`original`, the default, or `simplified`): a further 20 % of HD's
circuit shading and 8 % of Omega's at 4K on the Deck, for a flatter, matte track; at a 4K output
that buys about 5 % of render scale, and exact lighting at the scale that costs the same looks
better. The second
option the brief named, approximated light curves, is not a speed option on this hardware and was
not added.

### Method

As "The 4K frame on the Deck pass by pass" above: `RADV VANGOGH`, AC, GPU clock unpinned (the
serialised span bench sits at 1,040 MHz, recorded per run with `amdgpu_top`), offscreen over ssh,
Deck load 1.3 to 2.5. Release `-C target-cpu=znver2 --features perf-probe` builds from the
`oag-appimage-build:bookworm` container, one binary per step. Spans: `OAG_RENDER_GPU_BENCH=100
OAG_RENDER_PERF=1 oag-game <image> --race --mode single_race --seed 1 --autopilot --no-audio
--size WxH --ticks 2500 --render-scale 100 --screenshot out.png`, the arms of each comparison
interleaved (order reversed every other round), 2 rounds; rounds agree to 0.1 to 0.5 %. HD is
Talon's Junction, 2048 Altima, Omega Tech de Ra (each title's default). `--mode time_trial` for
the one-craft frame. Whole races: `scripts/bench-render.py run hd 2048 omega --binary ./oag-game
--size WxH`. The dGPU is this notebook's NVIDIA RTX PRO 2000, host release build, same commands,
host load recorded (1.5 to 27; read its numbers as direction, not as tenths). Pixels: every pair
of screenshots of one comparison diffed, on the Deck at three sizes and on NVIDIA.

### The exact cuts, Deck `track solid, shaded`, us, 1280x800 / 1920x1080 / 3840x2160

| step | HD | 2048 | Omega | pixels |
|---|---|---|---|---|
| main (`b85b19789`) | 2,016 / 3,467 / 10,492 | 1,036 / 1,935 / 6,712 | 2,351 / 3,340 / 8,382 | |
| 1. craft-shadow loop ends at the last drawn record | 2,033 to 1,953 / 3,468 to 3,269 / 10,495 to 9,731 (-7.3 %) | flat (no shadows) | flat (no shadows) | identical |
| 1, time trial frame | 2,241 to 2,127 / 3,928 to 3,711 / 12,518 to 11,700 (-6.5 %) | | | identical |
| 5. fog runs one curve; rig, lit and `nova_vertex` folds | 1,961 to 1,868 / 3,283 to 3,160 / 9,791 to 9,244 | 1,041 to 986 / 1,948 to 1,831 / 6,791 to 6,313 (-7.0 %) | 2,349 to 2,142 / 3,359 to 3,016 / 8,423 to 7,502 (-10.9 %) | identical |
| A. the vertex curve in the vertex stage (default, the original's order) | 1,883 to 1,895 / 3,137 to 3,098 / 9,144 to 8,835 (-3.4 %) | 975 to 941 / 1,809 to 1,730 / 6,226 to 5,903 (-5.2 %) | 2,129 to 2,109 / 2,991 to 2,924 / 7,403 to 7,133 (-3.6 %) | HD moves, below |
| 4. no second-texture fetch where nothing reads it | 1,895 to 1,773 / 3,098 to 2,955 / 8,835 to 8,651 (-2.1 %) | flat | flat | identical |
| 6. no generic specular at a zero scale; no tangent branch without tangents | flat (1,775 to 1,768 / 2,961 to 2,956 / 8,656 to 8,647) | 940 to 857 / 1,731 to 1,540 / 5,909 to 5,145 (-12.9 %) | 2,107 to 2,025 / 2,928 to 2,708 / 7,143 to 6,320 (-11.5 %) | identical |
| all, main to final | 2,016 to 1,768 / 3,467 to 2,956 / 10,492 to 8,647 (-17.6 %) | 1,036 to 857 / 1,935 to 1,540 / 6,712 to 5,145 (-23.3 %) | 2,351 to 2,025 / 3,340 to 2,708 / 8,382 to 6,320 (-24.6 %) | HD's vertex-lit chunks only |

Omega's `track blended` (its road decals, plain blended pipelines): 2,555 to 1,637 us at 4K
(-36 %), from cuts 5, A, 4 and 6. Pipelined per frame at 4K, main to final: HD 14.10 to 12.65
ms, 2048 8.67 to 7.36, Omega 13.50 to 11.54. Each row is its own interleaved pair (the step
before it against it), so rows measured on different days of the stack do not add exactly.

What each is:

1. **The craft-shadow loop ends at the last drawn record.** `oag_render::shadow::map::Map::
   receiver_shadows` packs the records whose square was drawn (in sight, with geometry) to the
   front in slot order, and `craft_shadow_coverage` breaks at the first empty one. Exact: a record
   left out read a cleared square, a factor of `1 - 0`, and the kept ones multiply in the same
   order. It pays in a full pack too (-7.3 % at 4K): craft out of sight leave their record.
2. **(Cut 5) Fog runs one curve; three weights fold.** `fogged` branches on `scene.fog.curve`
   (0 or 1 on every title) instead of computing both factors and mixing them; under `lit_by_rig`
   the rig and `lit` weights are the constant one the override already promises; a
   `nova_vertices` override (an OR over the model's vertices) folds 2048's vertex-lit test
   elsewhere. Omega gained most (its fog factor work was the larger share).
3. **A. HD's `IleVertex` curve runs per vertex.** The original's vertex program computes
   `pow(colourSet, prelitBias) * prelitScaleSpecular` and interpolates it
   ([renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md), "The vertex curve is per
   vertex", confidence 90), so this is the original's order and the default. Three `pow` a pixel
   fewer; 2048 and Omega gain too, since their pixels no longer compute the HD curve the select
   threw away. The picture: 9,083 of 1,024,000 HD pixels move by up to 19 levels; against the 13
   HD race pairs, 7 move toward the original, 5 away (the page above has the table).
4. **(Cut 4) The second texture is fetched only where something reads it.** `second_read` is 0 on
   a plain pipeline whose draws carry none of the role bits that read it (albedo or alpha from
   it, the facing-ramp sheen), name no emissive row (the pane and twin programs) and have no glow
   mask sourced from it (`mesh_render::specialise::reads_second`). On HD's default circuit no
   plain draw reads it.
5. **(Cut 6) The generic specular where the rig's scale is zero, and the tangent branch where no
   vertex has a tangent.** 2048's and Omega's circuits author `specular x0.00`, so the generic
   specular is a product with zero; `BuildCacheScope::lit_by` pins `rig_specular` to 0 there.
   `tangents_present` (an OR over the model's vertices) folds the normal-map branches. Found
   because the lighting rung below saved 18 % on 2048 with no pixel changed.

**Measured and dropped:**

- **Cut 3, the sunless encode only where a shadow covers the pixel**: byte-identical, but slower
  on the Deck (HD 4K 9,721 to 9,800 us, time trial 11,680 to 11,764). The deferred values stay
  live across the shadow lookup. Intel (the brief's worry): 3,412 / 3,074 to 3,184 / 3,093 us at
  1080p, no regression, no gain.
- **An `unlit_plain` fold** (a plain pipeline every vertex of which has `lit` 0 skips the rig's
  block): byte-identical, flat everywhere (HD 4K 8,649 to 8,650 us).
- **Cut 2, the `programs_present` override for the rich pipelines: not built.** A throwaway arm
  that skips the rich half's draws altogether bounds it: HD 4K 9,139 to 8,565 us (575 us, 6 %),
  Omega 7,401 to 6,606 (795 us), 2048 nothing. The override would recover a fraction of that; the
  craft pass (3 ms at 4K, all rich) is where it would pay more, and is left open.

### The circuit lighting rung: `graphics.circuit_lighting`

`simplified` drops three things on a circuit's plain pipelines (the `simplified_lighting`
override, through `BuildCacheScope::simplified_lighting`, where the rig is on): the specular, the
normal map, and Wipeout HD's SPU vertex lights (the rig's per-vertex lights). It keeps the
albedo, the baked light (lightmap or colour set), the ambient, the sun on the surface's own normal
(so the craft shadow still takes out exactly the sun), glows, fog and shadows. **A departure from
the original's look, off by default**; a fresh and an existing profile stay `original`. Settings
key `[render_profiles.<title>] circuit_lighting`, `--circuit-lighting`, and the CIRCUIT LIGHTING
row under the graphics page, dropped on Pulse and Pure.

Deck `track solid, shaded`, us, exact to `simplified`, on the final exact build:

| | 1280x800 | 1920x1080 | 3840x2160 | pixels changed (4K) |
|---|---|---|---|---|
| HD | 1,768 to 1,459 | 2,956 to 2,400 | 8,647 to 6,940 (-19.7 %) | 30 % (2,507,308), up to 238 levels |
| Omega | 2,025 to 1,899 | 2,708 to 2,518 | 6,320 to 5,830 (-7.8 %); blended 1,637 to 1,502 | 0.2 % (18,691), up to 41 |
| 2048 | 857 to 788 | 1,540 to 1,426 | 5,145 to 4,784 (-7.0 %) | none |

On the build before cut 6 the rung read: HD 1,778 to 1,459 / 2,963 to 2,397 / 8,650 to 6,974 us
(-19.4 %), Omega 2,090 to 1,928 / 2,927 to 2,554 / 7,133 to 5,929 (-16.9 %) and its blended road
decals 2,026 to 1,504, 2048 942 to 795 / 1,730 to 1,441 / 5,906 to 4,833 with no pixel changed:
dead work, most of which cut 6 now takes exactly; the 7 % left on 2048 still changes no pixel
(open, below). Skipping the SPU sum in the
vertex stage as well (a vertex-stage constant, which the prepass sharing `vs_main` rules out for a
committed form) measured the same as skipping its use in the fragment stage.

**What a player sees.** On Talon's Junction (a tunnel) the ceiling and wall panels lose their
white glints and the road its sheen; the metal reads flatter and a little darker, and the circuit
is plainly the same. On a sunny circuit the loss is larger: Modesto Heights loses the sun's glare
on the road and the bright metal at the side, and the road turns matte grey-blue. Against the 13
HD race pairs it moves away from the original on 9 (Modesto Heights whole-frame error 3.12 to
5.77, Anulpha Pass 2.94 to 5.16), and stays level on Talon's Junction and The Amphiseum. Omega
changes only where its draws carry a specular or normal map (0.2 % of the frame).

**Titles.** HD and Omega share the shader family and are wired. 2048 is wired and changes nothing:
its circuits author no specular, carry no tangents on the plain draws and no SPU lights. Pulse and
Pure: skipped, the row is dropped (`Definition::drop_unavailable_circuit_lighting`, keyed on
`Looks::material_render_state`); their circuits have no authored rig to simplify.

### Approximated light curves: not added

A throwaway arm replaced the lightmap curve's `pow` with a cubic (`x^2 (0.9 x + 0.1)`, wrong
pictures on purpose, a cost bound for any polynomial fit): Deck 4K HD 8,655 to 8,506 us (-1.7 %),
2048 5,919 to 6,043 (+2.1 %), Omega 7,135 to 7,229 (+1.3 %); 1080p HD -1.0 %, 2048 +1.9 %. On
RDNA2 a `v_log_f32`/`v_exp_f32` pair is quarter rate, so a `pow` is about nine cycles, and a fit
that holds an authored exponent within a level costs as much. Its picture moves 79 % of HD's
pixels by up to 44 levels. A rung that changes the look for no speed was not added; the brief's
claim is false on this hardware.

### Whole race and the equal-cost comparison

Deck, `scripts/bench-render.py`, frame p50 / p95 and GPU p50 / p95, ms; every run finished at the
same tick with the same state hash (HD 7,020 `94a2261ef5045e63`, 2048 13,041 `570a234b8d1de611`,
Omega 6,777 `8967e698d2f4bb3b`), Deck load 0.7 to 2.5. HD's exact row is the build before cut 6
(cut 6 does not change HD's shading, above); 2048's and Omega's include it.

| | main | exact, final | `simplified` |
|---|---|---|---|
| HD 1280x800 | 5.52 / 6.89 (3.50 / 4.31) | 5.47 / 6.82 (3.42 / 4.26) | 5.47 / 6.74 (3.31 / 4.25) |
| HD 1920x1080 | 7.21 / 9.24 (5.33 / 7.16) | 6.88 / 8.95 (5.08 / 6.83) | 6.62 / 8.73 (4.82 / 6.60) |
| HD 3840x2160 | 17.76 / 25.54 (15.70 / 23.57) | 16.58 / 23.85 (14.59 / 21.93) | 15.54 / 23.26 (13.61 / 21.33) |
| 2048 1280x800 | 2.51 / 3.06 (1.42 / 1.93) | 2.56 / 3.10 (1.23 / 1.73) | |
| 2048 1920x1080 | 3.00 / 3.78 (2.01 / 2.67) | 2.84 / 3.53 (1.75 / 2.40) | |
| 2048 3840x2160 | 8.30 / 10.89 (5.96 / 8.48) | 7.11 / 9.37 (4.79 / 6.94) | 6.85 / 9.18 (4.57 / 6.68) |
| Omega 1280x800 | 4.34 / 5.58 (2.56 / 4.03) | 4.38 / 5.50 (2.34 / 3.86) | |
| Omega 1920x1080 | 5.63 / 7.07 (3.69 / 4.99) | 5.17 / 6.62 (3.22 / 4.54) | |
| Omega 3840x2160 | 12.93 / 15.89 (9.37 / 12.40) | 11.14 / 13.54 (7.68 / 10.06) | 10.90 / 13.18 (7.40 / 9.67) |

At 800p all three titles are bound by the CPU: the GPU drops, the frame does not. HD at a 4K output
still misses 60 Hz natively (16.6 ms p50); Omega is now under the line on p95 (13.5 ms offscreen;
a presented output adds about 1 ms), and 2048 has room.

**Equal cost.** Exact lighting at lower render scales (offscreen targets of the scaled size, which
is what the scale draws before the upscale), HD / Omega frame p50 / p95, ms:

| scale | HD exact | Omega exact |
|---|---|---|
| 67 % (2573x1447) | 9.52 / 13.03 | 7.21 / 8.78 |
| 75 % (2880x1620) | 10.91 / 15.04 | 8.17 / 9.85 |
| 80 % (3072x1728) | 11.89 / 16.64 | 8.82 / 10.66 |
| 85 % (3264x1836) | 13.00 / 18.51 | 9.58 / 11.55 |
| 90 % (3456x1944) | 14.11 / 20.09 | 10.35 / 12.57 |
| 100 % | 16.58 / 23.85 | 11.80 / 14.49 (before cut 6) |
| `simplified` at 100 % | 15.54 / 23.26 | 10.90 / 13.18 |

`simplified` at native 4K costs what exact lighting costs at about 95 % scale on HD (p50 between
the 90 % and 100 % rows, p95 near 100 %), and the same on Omega. **Exact lighting at 90 %, cheaper
than the rung, looks better than the rung at native**: crops at 1:1 of the 4K output (Talon's
Junction, tick 2,500, NVIDIA, `--render-scale 90` against `--circuit-lighting simplified`) show
the 90 % frame a touch softer on the edges and otherwise the same picture as native, while the rung
loses every white glint on the tunnel ribs and panels. The rung is a poor trade at a 4K output;
render scale is the better lever there. Where the rung could still matter is a fixed output on a
GPU-bound machine with no upscaler, which this lane did not find.

**NVIDIA whole race, HD** (`scripts/bench-render.py`, host load 4 to 46 because other lanes'
gates ran, so read the GPU column, not the frame): GPU p50 / p95 at 3840x2160 4.18 / 6.14 ms
(main), 3.69 / 5.59 (exact, -12 %), 3.46 / 5.31 (`simplified`); at 1280x800 0.96 / 1.69, 0.93 /
1.54, 0.81 / 1.23. The NVIDIA part is CPU bound below 4K, so frame times there say nothing about
shading. Every run finished at tick 7,020 with hash `94a2261ef5045e63`.


### B and D, recorded

- **B, the albedo decode through an sRGB view** (three `pow` a pixel, the analysis's ~0.5 ms at
  4K): a lane only behind the HD pairs. A hardware view decodes before the filter and the
  original after it (`hd-track-tone.md`, "The texture unit's decode is the sRGB curve"), so it
  moves every textured pixel; worth one small lane that measures both the span and the 13 pairs.
- **D, a trilinear sampler for the lightmap unit** (0.1 to 0.3 ms by the analysis): first read
  the original's lightmap unit filter state in an RPCS3 dump. Only if the original is not
  anisotropic there is it a lane (then it is fidelity and speed together).

### Left open

- The rich pipelines (`programs_present`, cut 2 above) and the craft pass, which runs only rich
  pipelines (3 ms at 4K on HD).
- 2048's rung saving left after cut 6 (7 % at 4K, no pixel changed): `tangents_present` is an
  OR over the whole model, so one textured hull-like draw keeps the branch for every plain
  pixel; a per-list OR, as `slots_present` has, is the next try.
- Omega's rung saving after cut 6 (7.8 % at 4K for 0.2 % of the pixels): most of that work is
  shown on few pixels. A census of which plain Omega draws carry the specular or normal map would say whether
  those draws want a pipeline of their own.
