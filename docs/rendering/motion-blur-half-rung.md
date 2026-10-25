# The half-gather blur rung: what it saves and what it does (2026-10-25, `blur-half-rung`)

Dynamic resolution sheds the motion blur's full gather before it gives up pixels, and
restores it last ([ADR-0075](../architecture/adr/0075-dynamic-resolution-sheds-the-blur-gather-before-pixels.md),
`oag_present::drs::blur`). This page is the evidence: what the half gather saves on three
rigs, what a player was judged to see, and what the rung did in a running race.

Chosen workload, not measured against any original: the motion blur is this project's own
enhancement ([motion blur](motion-blur.md)). Pulse and HD, seed 1, autopilot, release
builds.

## What the half gather saves

### Rigs

| Rig | Machine | GPU, driver | Build | Load average | Lock | Command |
| --- | --- | --- | --- | --- | --- | --- |
| A | maintainer's workstation | AMD Raphael iGPU (`RADV RAPHAEL_MENDOCINO`, `MESA_VK_DEVICE_SELECT=1002:164e`), Mesa 26.2.5 | `30c1f87`, release, `perf-probe`, before the race-track prepass commit | 5 to 8 | gate lock | `just bench-render pulse --size 1280x800 --motion-blur <s> [--motion-blur-resolution half]` |
| B | Steam Deck | `RADV VANGOGH` | `30c1f87`, container-built (glibc 2.36), `perf-probe` | 0.5 to 1.6 | `deck.lock` | the same bench, `--size 1280x800` and `--size 3840x2160` (no upscaler, the scene at full 4K) |
| C | maintainer's workstation | as A | this lane's branch, `c31a29ae5`, release, `perf-probe` | 7.5 to 9.4 | gate lock | `OAG_RENDER_GPU_BENCH=40 OAG_RENDER_PERF=1 oag-game --race --mode single_race --seed 1 --autopilot --no-audio --size 3840x2160 --ticks 1500 --motion-blur high --motion-blur-resolution <r> --screenshot out.png` |

Rigs A and B are whole-race scene-only benches (`docs/tools/render-bench.md`), p50 over
the race; C is one tick re-drawn 40 times.

### Whole race, median milliseconds

| Rig | Title, size | off | low | medium | high | medium + half | high + half |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A, GPU / frame | Pulse 1280x800 | 3.9 / 4.6 | 7.0 / 7.8 | 8.0 / 8.8 | 8.7 / 9.5 | 6.1 / 7.1 | 6.2 / 7.0 |
| B, GPU | Pulse 1280x800 | 2.7 | | | 3.9 | 3.2 | 3.3 |
| B, GPU / frame | Pulse 3840x2160 | 13.5 / 14.6 | | | 32.6 / 33.8 | | 24.1 / 25.2 |
| B, GPU / frame | HD 3840x2160 | 21.6 / 24.6 | | | 40.7 / 44.0 | | 31.6 / 34.6 |

`gpu_blur_us` p50 on rig A: 4.6 ms full, 2.2 ms half. The half gather saves 45-52 % of
the chain: about 2.4 ms on the iGPU at 1280x800 and 8.5-9.1 ms on the Deck at 4K. The Deck
at 1280x800 is CPU-bound (CPU 3.4-3.9 ms, frame 3.8-4.6 ms), so there is nothing to gain
there. The blur strength moves the cost only a little (rig A, low 7.0 against high 8.7).

### The floor that does not scale with the gather

About half the chain stays whatever the gather does: 2.2 ms on rig A, about 10 ms on the
Deck at 4K. Rig C splits it by pass (median microseconds, one hard-turn tick):

| Pass | Full gather | Half gather |
| --- | --- | --- |
| prepare | 1,588 | 1,578 |
| tile-max (x) | 5,970 | 5,828 |
| tile-max (y), neighbour-max | 64 | 87 |
| reconstruct | 77,509 | 17,680 (a quarter of the fragments) |
| composite (half only) | | 4,813 |
| copy | 1,127 | 1,105 |
| chain | 86,210 | 31,079 |

The floor is `prepare`, `tile-max (x)`, `copy` and, at half, the full-size `composite`:
about 13.3 ms of the half chain on this frame. `tile-max (x)` is the largest single term
(49,680 fragments each reading `0.08 * height` pixels). This tick's reconstruct is far
dearer than the race median, so read the split, not the totals.

Confidence 80 for the savings (two rigs, two titles, whole races, GPU timestamps, which do
not move with load), 60 for the split (one tick, one rig).

### `tile-max (x)` unrolled (2026-10-25, `blur-floor`)

The floor's largest term was latency, not bandwidth. `fs_tile_max_x` looped over
`constants.tile`, a runtime trip count, so each fragment issued one `textureLoad`, waited
for it and compared, 173 times at 2160 lines. Commit `43dba7cb9` unrolls the loop sixteen
wide: sixteen loads in flight, then sixteen comparisons in the same scan order with the same
strict `>`, then a one-at-a-time tail for what is left (`173 = 10 * 16 + 13` at 4K; 64 at
800 lines has no tail). Of two equally long velocities the first still wins, so the tile
grid, and so the frame, is bit-identical by construction.

Rigs: D is rig C's machine and method (one tick at 1500, re-drawn 40 times,
`OAG_RENDER_PERF=1`, blur `high`), release `perf-probe` builds of `e8d6dfc7e` (base) and
`43dba7cb9` (unrolled), load 3.6 to 10, gate lock held. E is the Steam Deck (`RADV
VANGOGH`), the same two commits container-built (glibc 2.36), the same tick at 3840x2160,
load 0.3 to 2, `deck.lock` held.

`tile-max (x)`, median microseconds, base -> unrolled:

| Rig, title, size | Full gather | Half gather |
| --- | --- | --- |
| D, Pulse 3840x2160 | 6,061 -> 1,755 | 6,231 -> 1,687 |
| D, HD 3840x2160 | 5,757 -> 1,679 | 5,922 -> 1,699 |
| D, Pulse 1280x800 | 449 -> 220 | 445 -> 217 |
| D, HD 1280x800 | 441 -> 218 | 446 -> 220 |
| E, Pulse 3840x2160 | 2,392 -> 686 | 2,677 -> 731 |

Eight wide measured 1,951 us on rig D (Pulse 4K full), against 1,651 for sixteen in the
same session. **Every frame is identical**: the base and unrolled screenshots of all eight
rig D cells (Pulse and HD, both sizes, both gathers) and both rig E cells differ in 0
pixels (max channel difference 0).

Whole race, `scripts/bench-render.py run --size 3840x2160 --motion-blur high
--motion-blur-resolution <r> --binary <bin> pulse`, median milliseconds, base -> unrolled
(6,836 frames each):

| Rig | `gpu_blur_us`, full | `gpu_blur_us`, half | GPU frame, full | GPU frame, half |
| --- | --- | --- | --- | --- |
| E (Deck) | 18.81 -> 17.40 | 10.47 -> 9.03 | 29.74 -> 28.36 | 21.37 -> 19.96 |
| D (iGPU), load 7.7 to 1.1 | 80.89 -> 79.61 | 30.34 -> 26.16 | 114.20 -> 112.11 | 62.48 -> 58.18 |

On the iGPU the full gather's reconstruct (about 77 ms of a 4K frame there) dwarfs the
pass, so the full-gather saving reads small against its own run-to-run spread; the half
gather's 4.2 ms is the pass's whole saving.

On the Deck at 4K the unrolled pass saves about 1.4 ms of a race's median blur chain at
either gather, and half the floor this section measured is now a third smaller. What is
left of the floor on rig E's tick at half: `prepare` 1.1 ms, `composite` 2.2 ms, `tile-max
(x)` 0.7 ms, `copy` 0.5 ms.

Not tried, and why: wider loads or `textureGather` (a gather returns one channel of four
texels, so velocity needs two and the result is the same loads in another shape), a
pairwise or tree max (it changes which of two equal lengths wins, so the grid would no
longer be bit-identical), folding the reduction into `prepare` or a compute pass with
workgroup memory (a bigger change for at most the 0.7 ms left on the Deck).

Confidence 85 for the pass timings (two machines, two titles, GPU timestamps, A/B binaries
on the same day), 90 for "identical" (by construction and by ten pixel diffs).

## What a player sees

The maintainer compared Pulse and HD at 3840x2160, FSR 3, blur `high`, tick 1500, one
hard-turn frame each: full gather at render scale 100, full gather at 75, half gather at
100. **75 % looks worse than half blur at 100 %**: the lower scale softens the hull, its
decals and the edges; the half gather keeps the hull sharp and coarsens only a background
the blur has already smeared. One still per title, no motion and no temporal check, so
this is strong but unproven. The maintainer also reports the half gather is more visible
at `high` than at `low` in some scenes, which is why the rung's thresholds are one set
and easy to change rather than tuned per strength.

Rig C's two frames, scaled to 1280x720, are indistinguishable by eye on Pulse's default
circuit at tick 1500.

## What the rung did in a running race

`just bench-render` runs no controller, so these are windowed sessions inside a headless
gamescope on rig C's GPU and build, with `--frame-log` (one line a second) and the
session's own `dynamic resolution: motion blur gathers at ...` line per flip:

```sh
XDG_CONFIG_HOME=<private> MESA_VK_DEVICE_SELECT=1002:164e \
  gamescope --backend headless -W 3840 -H 2160 -w 3840 -h 2160 -- \
  oag-game --race --autopilot --no-audio --frame-log --seed 1 --script "90:quit"
```

with `[render_profiles."Wipeout Pulse (PSP)"]` at `render_scale = 100` (chosen),
`minimum_resolution = 25`, `motion_blur = "high"`, `motion_blur_resolution = "full"`,
`pause_on_focus_loss = false`, the PSP aspect (ceiling 3812x2160), and the target below.
"Base" is the same tree before the rung (`a2160add7`); 88 seconds of race each, load 7.5
to 9.4, gate lock held.

| Case | Build | Median fps | Median p99 ms | Median scale | Mean pixels | Gather flips |
| --- | --- | --- | --- | --- | --- | --- |
| 3840x2160, target 30 | base | 31.4 | 40.7 | 0.45 | 1.73 MP | - |
| 3840x2160, target 30 | rung | 31.1 | 39.7 | 0.60 | 2.71 MP | 1 (shed at 0:06, never restored) |
| 1280x800, target 90 | rung, first restore rule | 112.9 | 11.7 | 1.00 | 0.89 MP | 21 |
| 1280x800, target 90 | rung, final | 128.3 | 8.8 | 1.00 | 0.89 MP | 3 |

At 4K the rung holds the same frame rate on **57 % more pixels**: it gives up the gather
first, and the extent then settles higher. The 1280x800 run is the oscillation the first
restore rule had (32 frames of patience, a 300-frame relapse window): each restore was
predicted to fit for its 32 frames and undone by the scene's next spike 1 to 5 seconds
later. With three seconds of patience and a relapse window as long as the longest
backed-off wait (ADR-0075), the same run shed, restored once, shed again and held.

### On the Steam Deck

Steam Deck (`RADV VANGOGH`, Mesa 26.2.0-devel), headless gamescope at 3840x2160,
container-built `perf-probe` builds, Pulse PSP EU, seed 1, autopilot, `--frame-log`,
`--script 90:quit` (88 logged seconds), `render_scale = 100` (chosen),
`minimum_resolution = 25`, blur `high` at `full`, target 30 fps, private XDG dirs,
`deck.lock` held, load about 1. Base is `2c926e4` (the commit the rung's branch was cut
from), rung is `db8d821`. Run by the lead.

| Build | Median fps | Median p99 ms | Mean pixels | Gather flips |
| --- | --- | --- | --- | --- |
| base, run 1 | 35.3 | 33.8 | 6.76 MP | - |
| base, run 2 | 35.5 | 33.6 | 6.75 MP | - |
| rung, run 1 | 38.5 | 31.4 | 7.91 MP | 5 |
| rung, run 2 | 38.5 | 31.2 | 7.91 MP | 3 |

The rung held the full 3812x2160 for the whole race and ran 3 fps faster, where the base
settled near 81 % of that area. Two runs of one build differ by under 0.5 fps. Confidence
75: one machine, one title, two runs each, a chosen target.

HD on the same rig, except the title (Wipeout HD, PS3, the decrypted EU ISO) and the
display aspect `free`, so it fills 16:9 (ceiling 3840x2160):

| Build | Median fps | Median p99 ms | Mean pixels | Gather flips |
| --- | --- | --- | --- | --- |
| base, run 1 | 35.4 | 31.5 | 4.79 MP | - |
| base, run 2 | 35.3 | 31.6 | 4.79 MP | - |
| rung, run 1 | 34.5 | 33.3 | 6.49 MP | 1 |
| rung, run 2 | 34.8 | 33.0 | 6.47 MP | 1 |

The rung settles the extent at about 3456x1944, then 3264x1836 for the rest of the race.
The base settles at about 57 % of the full area, so the rung draws 35 % more pixels. It
costs about 0.7 fps and **about 1.5 ms of p99**, though both builds stay above the 30 fps
target. Pulse got more pixels *and* a faster frame. HD's scene alone is about 22 ms at 4K
here, which the half gather cannot fit, so the extent still falls: the rung shed once and
never restored, in both runs. Two runs of one build differ by under 0.3 fps. Confidence 70:
one 88-second autopilot race per run, and a worse p99 that a player may notice.

Not checked: a player's eye on a flip in motion, on either title.
