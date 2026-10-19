# Rendering at 4K and 8K: what scales with the pixels (2026-10-19, `hires-perf`)

The question was whether any pass makes a 3840x2160 or 7680x4320 frame expensive
on the maintainer's AMD workstation GPU, and what can come off it without
changing the picture. **One cut landed: Omega's circuit goes through the depth
prepass (pipelined frame 8K 9.44 to 4.91 ms, 4K 2.78 to 1.60 ms, picture
byte-identical).** Everything else scales linearly with the pixels and none of
it is a defect; the costs are ranked below with what each one is.

Chosen workload, not measured against any original. Every number is
`AMD Radeon RX 7800 XT (RADV NAVI32)`, release `perf-probe` build, offscreen,
the title's default circuit, seed 1, autopilot, tick 1,500 (600 for the
`--presented` runs), the gate lock held. GPU spans are timestamps and do not
move with the machine's load (the load average was 4 to 25 across the runs);
the `pipelined` line is the wall time of 40 frames back to back. Median
microseconds.

## How it was measured

```sh
OAG_RENDER_GPU_BENCH=40 OAG_RENDER_PERF=1 target/release/oag-game <source> --race \
  --mode single_race --seed 1 --autopilot --no-audio --size 7680x4320 --ticks 1500 \
  --screenshot out.png                       # Scene::render, race pass, spans, bloom, blur
OAG_RESOLVE_BENCH=40 ... --presented --reconstruction fsr3 --render-scale 67 --camera-jitter
                                              # the presentation resolve, FSR3 REN / OUT
```

`just bench-render` plays a whole race but leaves the post chain and the
presented resolve out (scene only), so the per-pass table comes from the single
tick benches that `docs/tools/render-bench.md` and
[`frame-audit.md`](frame-audit.md) describe. Peak GPU memory is the process's own
`drm-memory-vram` in `/proc/<pid>/fdinfo`, polled while it ran.

## Defaults, per title (MSAA off, blur off, no upscaler)

`Scene::render` GPU / race pass / HD bloom, at 1080p, 4K, 8K:

| Title | 1080p | 4K | 8K | Pipelined at 8K |
| --- | --- | --- | --- | --- |
| Pulse | 983 / 490 / - | 2,052 / 1,532 / - | 5,421 / 4,924 / - | 3,364 |
| Pure | 712 / 368 / - | 1,564 / 1,231 / - | 3,765 / 3,231 / - | 2,595 |
| HD | 1,795 / 791 / 214 | 3,507 / 2,202 / 491 | 7,554 / 5,404 / 1,442 | 5,837 |
| 2048 | 1,122 / 649 / 175 | 2,772 / 2,073 / 382 | 5,813 / 4,395 / 1,110 | 5,218 |
| Omega (before) | 1,826 / 1,424 / - | 4,681 / 4,140 / - | 9,158 / 8,168 / - | 9,247 |
| Omega (after) | 1,361 / 958 / - | 2,872 / 2,306 / - | 6,077 / 5,066 / - | 4,922 |

Nothing is near a 60 Hz budget at 8K by default, and the CPU side is size
independent (the per-mark CPU spans of an 8K and a 1080p frame agree, the one
size-keyed span being the first mark's wait for the queue).

## Ranked by cost at 8K, with how each scales

| Cost (HD unless noted) | 1080p | 4K | 8K | Scales |
| --- | --- | --- | --- | --- |
| Motion blur chain, `high`, MSAA 4x | 720 | 2,605 | 7,259 | linear in pixels |
| - its gather (`motion blur reconstruct`) | | 1,895 | 5,629 | linear (taps are capped at 15) |
| - prepare / tile-max x / copy | | 214 / 325 / 68 | 700 / 697 / 218 | linear; tile-max x reads `0.08 * height` pixels a fragment |
| FSR 3 resolve at render scale 67 (REN + OUT) | 543 | 1,744 | 4,768 | linear; OUT (accumulate, RCAS) is the part that does not fall with the render scale |
| FSR 3 at render scale 100 (8K) | | | 10,413 | pointless at 1:1 |
| Race pass, HD track solid (MSAA 4x) | | 1,844 | 3,881 | linear, about 117 ps a fragment at 1.0x overdraw |
| HD bloom | 214 | 491 | 1,442 | a 210 us floor plus about 40 us a megapixel (the two full-size passes are bandwidth) |
| SMAA / FXAA / FSR 1 resolve (Pulse, scale 100 / 67) | 227 / 155 / 306 | 507 / 333 / 849 | 1,505 / 1,041 / 2,401 | linear |
| Present composite (resolve with no upscaler) | 85 | 190 | 454 | linear |
| Shadow passes | | | about 700 | fixed (the map's size, not the frame's) |

MSAA 4x is +0.7 ms on HD's race pass at 8K and +0.1 ms on Pulse's. Nothing
failed or came near a limit: the worst realistic 8K setting (MSAA 4x, blur high,
FSR 3 at 100 %) peaks at **6.4 GiB** of GPU memory (3.9 GiB without FSR 3,
1.8 GiB at defaults, 1.0 GiB at 4K, 0.65 GiB at 1080p), the targets are
allocated once per size and nothing is recreated per frame in steady state.

## What landed: Omega's circuit has a depth prepass

Omega's opaque circuit was the one place the span table showed an outlier:
**30.4 M fragment invocations at 4K for 8.3 M pixels (3.7x), 120 M at 8K**,
against 1.0x on HD, whose circuit goes through the depth prepass
(`mesh_render::Prepass`), and 1.7x on Pulse. The prepass was keyed on HD's
bloom chain alone (`hd.is_some()`); it is now
`scene::circuit_prepass(hd_chain, omega_chain)`, the same two chains the
bloom choice already reads, and the nearest-first sort that feeds it follows
the same rule (`Recorder::sorts_opaque`).

| Omega, `tech_de_ra` | before | after |
| --- | --- | --- |
| Track solid invocations, 1080p / 4K / 8K | 7.80 M / 30.4 M / 120.3 M | 2.14 M / 8.5 M / 34.0 M |
| Track solid span, 1080p / 4K / 8K | 1,164 / 3,488 / 7,189 | 707 / 1,684 / 3,675 |
| Race pass, 1080p / 4K / 8K | 1,418 / 4,087 / 8,355 | 958 / 2,306 / 5,066 |
| **Pipelined frame**, 1080p / 4K / 8K | **1,122 / 2,775 / 9,438** | **933 / 1,604 / 4,922** |

Picture: byte-identical (`cmp`) against `main`'s binary at ticks 60, 400, 1500,
3000, 5000 and 7000 (1920x1080), and at tick 2500 with `--msaa 4x`, with a
two-player horizontal split and with `--presented --reconstruction smaa`
(1600x900). The prepass lays depth with the same vertex stage and shades
`LessEqual` in reverse order, which is why coplanar ties resolve as before (the
mechanism is `mesh_render::Prepass`'s, proved on HD). Pulse's and HD's frames are
untouched (Pulse tick 600 invocations 2.70 M before and after).

**The price, measured and accepted.** The prepass draws the opaque list twice,
so a CPU-bound frame pays for the extra calls. Omega at 1280x720, one view:
pipelined 787 and 868 us before, 637 and 643 after (a gain even there); at
1280x800 split, two views horizontal: 1,931 and 997 before, 1,943 and 1,096
after (CPU 1,565 / 900 before, 2,055 / 1,084 after), four views 1,676 and 1,695
before, 2,053 and 2,059 after: **a four-view split of 640x400 views costs this
GPU +20 % (about 0.35 ms), the one measured regression**. Those frames are CPU
bound on a fast GPU and are far under a 60 Hz frame; on a GPU-bound adapter
(the Deck's APU, an iGPU) the saving is the larger term. A per-view switch
(prepass only above some view area) would remove it and was not built.

## Measured and not taken

- **Prepass for Pulse and Pure.** `circuit_prepass` forced true: Pulse 1.8x to
  1.0x invocations, race pass 4K 1,536 to 1,167 us, 8K 5,015 to 4,387 us, the
  pipelined frame at 8K 3,413 to 2,627 us. But at 1080p the pipelined frame is
  934 to 956 and 977 to 986 us (Pulse), the title is CPU bound (1.3 ms record
  and submit) and the extra draws only help once a GPU is the limit. Pure: 1080p
  469 to 368 us, 8K 2,624 to 1,974 us. A candidate for a size-gated prepass;
  the picture was not compared.
- **Skipping the blur gather's colour fetch when a tap weighs 0.** Bit-identical
  by construction and by `cmp` at 4K and 8K, and no faster at 4K (1,860 to 1,845
  us reconstruct; the 8K pair read 5,585 and 8,778 in one round and 5,581 and
  4,680 in the next, which is the noise of this host). The gather is bound by
  its arithmetic and latency, not the fetch. Reverted.
- **`motion_blur_resolution = half`** exists and cuts the gather to a quarter of
  the fragments (the documented 660 us at 1080p on HD scales to about 5 ms at
  8K); it is a quality setting and the default stays `full`.

## Left open (handover thread `high-resolution-rendering-is-linear-and-cheap-by-default`)

- The blur gather (5.6 ms at 8K) and FSR 3's REN dispatches (7.1 ms at 8K, native
  scale) are the two biggest opt-in costs; both are arithmetic bound.
- A size-gated prepass for Pulse and Pure, and a per-view gate for Omega's
  split.
- The offscreen bench does not time the windowed present, HUD overlay or the
  swapchain on a real 4K/8K surface; this host's RADV cannot present on Xvfb.

## Steam Deck, 4K (2026-10-19, `deck-4k-upscale`)

The question: what a 3840x2160 frame costs on the Deck's APU, native and with each
upscaler, against the 1280x800 panel. **Chosen workload, not measured against any
original.** Verdict first:

- **Native 4K does not fit a 60 Hz frame on the Deck for any title but Pure**
  (frame p50 / p99, whole race: Pure 10.2 / 12.9 ms, Omega 16.4 / 26.1, 2048 13.7 / 21.8,
  HD 20.8 / 31.1, Pulse 12.1 / 33.3). Against the 1920x1080 frame (p50) it costs 2.4x (HD) to 3.6x (Pure), the pixel count being 4x; the
  panel baseline is CPU bound for Pulse, HD and 2048, which is why 4K over 800p reads 3.1 to 6.3x, not 8.1x.
- **FSR 1 at a 4K output adds 7.0 ms of GPU time (serialised), FSR 3 17.4 to 20.0 ms**,
  whatever the title: the resolve is priced by the output pixels, not by the scene. FSR 3's
  `OUT` (accumulate + RCAS) alone is 11.8 to 12.2 ms at any render scale. At a 4K output
  FSR 3 cannot make 60 fps on this GPU at any render scale; FSR 1 can on Pulse, Pure,
  Omega and 2048 at scale 50, and not on HD.
- **The gamescope pixel-budget default (1920x1080, so scale 50 at a 4K output) is the
  right place** and is left alone (see "Recommendation").

### Rig and state

| | |
| --- | --- |
| Machine | Steam Deck (LCD APU, 8 cores), SteamOS kernel 6.18.50-valve2, Mesa 26.2.0-devel |
| Adapter | `AMD Custom GPU 0932 (RADV VANGOGH)` on every result line (no llvmpipe) |
| Power | AC connected, `power1_cap` 15 W, `power_dpm_force_performance_level = auto`, Steam in game mode (gamescope running, the benches are offscreen and open no window) |
| GPU clock | 1,600 MHz for 96 % of every whole-race run (`pp_dpm_sclk` sampled every 0.5 s); gpu temperature 52 to 67 C across the session, no clock drop seen |
| Build | `main` 678f0b339 plus nothing that touches rendering, `cargo build --release -p oag-game --features perf-probe` in the AppImage build container (Debian bookworm glibc), `-C target-cpu=znver2` (the workstation table has no target-cpu; only CPU columns can differ) |
| Seed / race | the `scripts/bench-render.py` races (seed 1, autopilot, full field, default circuit, third lap, about 7,500 ticks), scene only; load average 0.6 to 2.2 during runs |
| Lock | `flock -o ~/.cache/oag/deck.lock` around every session; private `XDG_*` directories under `~/oag-perf/deck-4k-upscale` on the Deck; game data read from the Deck's own `~/.local/share/oag` through symlinks |

### Whole race, scene only, frame_us p50 / p95 / p99 (ms)

`just bench-render` has no `--render-scale`, so an upscaled frame is measured as the
scene at the scaled size (validated: the single-tick Pulse scene at `--size 3840x2160
--render-scale 50` reads 7,540 us GPU, at `--size 1920x1080` 7,388 us). 2573x1447 is
scale 67 of 4K. Command, from a mini checkout on the Deck whose `data/images` and
`data/extracted` are symlinks to the Deck's own:

```sh
python3 scripts/bench-render.py run pulse hd --binary ./oag-game --size 3840x2160 --tag <tag>
# sizes: 3840x2160 (scale 100), 2573x1447 (67), 1920x1080 (50), 1280x800 (panel)
```

| Title | 1280x800 | 1920x1080 (4K at 50) | 2573x1447 (4K at 67) | 3840x2160 | 4K / 800p |
| --- | --- | --- | --- | --- | --- |
| Pulse | 3.85 / 4.64 / 5.81 | 4.36 / 6.94 / 9.30 | 6.11 / 11.01 / 15.67 | 12.08 / 22.36 / 33.26 | 3.1x p50 |
| HD | 6.48 / 7.38 / 8.45 | 8.53 / 10.72 / 11.71 | 11.98 / 15.32 / 16.80 | 20.75 / 26.59 / 31.05 | 3.2x |
| Omega | 4.13 / 5.53 / 7.98 | 6.41 / 8.32 / 9.53 | not run | 16.44 / 24.06 / 26.10 | 4.0x |
| Pure | 1.62 / 2.15 / 2.65 | 2.87 / 3.51 / 3.94 | not run | 10.24 / 12.12 / 12.90 | 6.3x |
| 2048 | 2.84 / 3.43 / 3.81 | 4.29 / 5.61 / 6.60 | not run | 13.71 / 17.80 / 21.77 | 4.8x |

GPU p50 / CPU p50 (ms) at 4K: Pulse 11.11 / 4.45, HD 18.01 / 6.00, Omega 14.56 / 2.74, Pure 9.67 / 1.57,
2048 13.15 / 2.58. At 1280x800 Pulse (CPU 3.5 vs GPU 2.4), HD (5.4 vs 3.7) and 2048 (2.1 vs 2.4) sit near or
above their GPU time: the panel baseline is CPU bound, which is why the ratio is under the 8.1x pixel count
(Pulse's p99 is the slow stretch, 33 ms against its median 12). **Pulse at native 4K was run three times**:
frame p50 12.07 / 12.09 / 12.08, p99 33.30 / 33.22 / 33.26 ms (spread under 1 %), with a hot SoC
(66 C) on the third; no thermal effect shows. HD's "28 fps worst stretch" in the older handover was not a
stretch: it was one stalled frame at race start, a synchronous soundtrack read, fixed on 2026-10-19
([HD GPU cost](hd-gpu-cost.md#the-decks-28-fps-was-one-frame-at-race-start-2026-10-19-deck-hd-28fps)).
This scene-only bench was never missing a 20 ms pass.

### The presented resolve at a 4K output, single tick (serialised GPU, median ms)

`OAG_RESOLVE_BENCH=200 OAG_RENDER_GPU_BENCH=200`, tick 1,500, one tick per process:

```sh
OAG_RENDER_GPU_BENCH=200 OAG_RESOLVE_BENCH=200 ./oag-game <source> --race --mode single_race --seed 1 \
  --autopilot --no-audio --ticks 1500 --size 3840x2160 --presented --render-scale <S> \
  --reconstruction <off|fxaa|smaa|fsr1|fsr3> [--camera-jitter] --screenshot out.png
```

| Resolve at 3840x2160 output | scale | Pulse | HD | Omega | Pure | 2048 |
| --- | --- | --- | --- | --- | --- | --- |
| none (present composite) | 100 | 1.54 | 1.44 | 1.26 | 1.31 | 1.30 |
| FXAA | 100 | 3.02 | 3.01 | 2.56 | 2.53 | 2.46 |
| SMAA | 100 | 5.27 | 4.50 | 5.51 | 4.66 | 4.36 |
| FSR 1 | 67 / 50 | 7.02 / 6.99 | 7.01 / 7.01 | 7.17 / 7.08 | 6.99 / 6.99 | 7.10 / 7.11 |
| FSR 3 (REN + OUT) | 67 | 19.81 | 19.94 | 19.99 | 19.94 | 20.01 |
| FSR 3 (REN + OUT) | 50 | 17.46 | 17.42 | 17.41 | 17.43 | 17.44 |
| - of which REN / OUT | 67 | 5.67 / 11.93 | 5.64 / 12.07 | 5.76 / 12.12 | 5.60 / 12.03 | 5.76 / 12.16 |
| - of which REN / OUT | 50 | 3.39 / 11.81 | 3.32 / 11.85 | 3.40 / 11.94 | 3.40 / 11.83 | 3.40 / 11.97 |

Every resolve is within 2 % across the five titles, as the output-pixel pricing predicts; SMAA
(and FXAA) are priced as a native-4K resolve, they change nothing about the scene cost (SMAA is also
the one that varies by title, 4.4 to 5.5 ms). At 1280x800 the present composite is 0.22 to 0.25 ms.
FSR 3.1 intermediates are 290.7 MiB (scale 50) and 365.7 MiB (67) as the game logs them. At the
workstation FSR 3 at 4K scale 67 costs 1.74 ms (table above, HD): the Deck is 11x slower there against
9x on Pulse's serialised scene (2.05 vs 19.2 ms), so the temporal chain is the worst-scaling pass on this APU.

**Clock caveat.** A single-tick serialised bench leaves the GPU idle between submissions, so
the governor sits at 1,040 MHz for most of it (sampled), against 1,600 MHz in a whole race.
The same Pulse native-4K scene reads 19.2 ms serialised and 11.1 ms in the race, a ratio of 0.58
against the clock ratio 0.65. So the resolve rows above are an **upper bound for a steady 60 fps
loop**; at 1,600 MHz expect about 0.65x (FSR 1 about 4.5 ms, FSR 3 about 11 to 13 ms). That
scaling is an estimate, not a measurement; GPU memory was not read (the Deck
exposes no `drm-memory` fdinfo query this lane tried, and only the FSR 3 intermediates above are known).

### A frame, estimated: scene frame plus resolve at 1,600 MHz (ms, p50 / p99)

Sum of the two measured parts above, **estimated, not a measured per-frame total**
(resolve scaled by 0.65 as stated; HUD, bloom and the gamescope compositor are not in it).
The 60 Hz budget is 16.7 ms.

| 4K output, title | scale 50, bilinear | scale 50, FSR 1 | scale 50, FSR 3 | scale 67, FSR 1 | native 4K |
| --- | --- | --- | --- | --- | --- |
| Pulse | 5.3 / 10.2 | 8.9 / 13.8 | 15.7 / 20.6 | 10.6 / 20.2 | 13.0 / 34.2 |
| HD | 9.4 / 12.6 | 13.0 / 16.2 | 19.8 / 23.0 | 16.5 / 21.3 | 21.7 / 32.0 |
| Omega | 7.3 / 10.4 | 10.9 / 14.0 | 17.7 / 20.8 | not run | 17.4 / 27.0 |

### Windowed 4K

No 4K display is attached to the Deck (`card0-eDP-1` connected, `card0-DP-1` disconnected), so the
presented windowed run with `--frame-log` was skipped. The Deck's gamescope presents a 1280x800
panel.

### Recommendation (chosen, not measured)

- **Keep the default.** At a 4K output under gamescope `scale_for` picks `sqrt(1920 x 1080 /
  3840 x 2160) = 50 %`: Pulse, Omega, Pure and 2048 land at 6 to 10 ms p99 scene frame and
  HD at 11.7 ms (before HUD, bloom and compositor). 67 % fails on HD (p99 16.8) and Pulse (p99
  15.7) scene-only, and native 4K fails everywhere but Pure. The 1080p budget stands.
- **Do not default an upscaler on at a 4K output on the Deck.** FSR 1 at 50 adds about 4.5 to
  7 ms (HD ends at 16 ms p99, no headroom), FSR 3 cannot hold 60 fps on any title. Pure and
  2048 are the only titles with room for FSR 3 at 30 or 40 fps.
- **The menu floor was 50.** The render-scale row offered 50 to 200 and the floor default was
  `OFFERED[0]`, so 720p drawn for a 4K output (33 %) was only reachable by editing the file. It now
  offers 25, 33, 50, 67, 75, 100, 125, 150, 200 (`Scale::RANGE` was already 25 to 200); the floor
  row's default is the named `Scale::HALF`. At 33 the scene costs about the 1280x800 column
  (720p is 0.9x its pixels, so about 3.5 ms p50 on Pulse), the 4K output stays at the
  fixed 1.3 to 1.5 ms present, FSR 1 about 4.5 to 7 ms. That 33 case is derived, not run.

### Not done

Omega, Pure and 2048 at 2573x1447 (scale 67) were not raced; GPU memory was not read on the Deck;
the resolve was not measured at a steady 1,600 MHz (no forced performance level on the Deck:
that is a system setting and the Deck is the maintainer's); SMAA/FXAA at scale below 100 were not
run (they are resolve-only at scale 100 here).
