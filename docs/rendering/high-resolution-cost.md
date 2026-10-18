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
