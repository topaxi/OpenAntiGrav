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

Not checked: the rung on the Deck (the 4K numbers above are its motivation, not a run of
it), HD in a running session, and a player's eye on a flip in motion.
