# Particle fill: a Rocket hit near the camera

The maintainer, playing HD/Fury on the Steam Deck at 3840x2160 (2026-11-25): "When hitting craft
with rockets, especially when two hits happen close on the screen, the FPS tank quite a bit."

**Reproduced on the workstation (RX 7800 XT, RADV, Vulkan).** The cost is GPU fill, sustained
while the explosion lives. It is not a first-use hitch (pipeline creation, upload or allocation).
At the worst frame the particles stack about 51 full screens of blending. About a fifth of that
was quads whose colour alpha had already reached exactly 0. They are now dropped before upload,
byte-identical. The rest is the effects as the disc authors them.

## How to reproduce it

A scripted HD race, seed 1, autopilot steering, the player handed a Rocket every tick and
firing once a second from tick 600. The input script (`fire.txt`) holds `600 cross`, then forty
repeats of the two lines `1 cross square` and `29 cross`. Run it with:

```sh
OAG_RENDER_BENCH_RACE=<stem> target/release/oag-game data/images/hdfury-ps3-eu-dec.iso \
  --race --mode single_race --seed 1 --autopilot --no-audio --log-every 0 \
  --give rocket --input-script fire.txt --ticks 800 --size 3840x2160 --screenshot <stem>.png
```

The default circuit's run is deterministic (state hash `24662cfdee286b27` at tick 800). Rockets
strike craft 4 at ticks 761 and 762. The player then flies through the blast, so the frame
around tick 775 is white from edge to edge. `<stem>.csv` is the
[race bench](../tools/render-bench.md) per-frame record. The window read below is ticks 760 to 789
(30 frames).

## Where the time goes

At 3840x2160 the scene pass's GPU time (`gpu_scene_us`) climbs from 2.3 ms at tick 761 to
8.4 ms at tick 775, then falls back to 1.8 ms by tick 778. `cpu_us` does not move at the hit, and
neither `frame_us` nor `gpu_scene_us` steps at the first track blast (tick 636) or the first
craft blast (tick 761). A hitch would show there.

Each draw was left out in turn (a throwaway `OAG_SKIP_PASSES` patch, not kept), over the same
window at 3840x2160 with load 28 to 32:

| Left out | GPU scene mean | GPU scene max |
| --- | --- | --- |
| nothing | 2.36 ms | 6.07 ms |
| the `.pob` particles (`psys` additive and alpha-over draws) | 1.11 ms | 1.26 ms |
| the screen flash | inside noise | inside noise |
| HD's SPU point lights | 2.33 ms | 5.99 ms |

So the particles are the cost. A probe summed each quad's on-screen area, clipped to the near
plane and the screen, in units of one full screen:

| Tick | Additive | Alpha-over | Of which colour alpha exactly 0 |
| --- | --- | --- | --- |
| 753 (two track blasts ahead) | 11.6 | 2.2 | 0 |
| 768 | 1.4 | 4.9 | 2.2 |
| 771 | 3.9 | 15.7 | 5.5 |
| 774 | 8.9 | 42.0 | 10.6 |
| 777 | 4.1 | 23.8 | 3.0 |

The alpha-over smoke of the craft blast is the bulk. At 4K, 51 screens is 420 million blended
fragments into the scene target. The Deck's GPU has a small fraction of this card's fill rate
and bandwidth, so the same frame there is several times longer.

## What changed

`drop_dead_quads` (`crates/raceplay/src/scene/frame/particles.rs`) removes a quad from the
additive, alpha-over and underlay lists before upload when all six of its vertices have colour
alpha exactly 0. `psys.wesl` multiplies its output alpha by the interpolated vertex alpha, and
the pipelines write neither depth nor velocity. Such a quad therefore leaves the target exactly
as it was under both blends: the additive's `rgb * a` and the over's `a` and `1 - a`. Order is
kept, because the over blend is not commutative. (An additive quad with colour 0 at every vertex
would add 0 too, since every target path maps 0 to 0. No frame here had one, so the rule was
left out.)

Before and after, A and B runs interleaved (the before binary also carried the throwaway
skip and probe patch, which does CPU-side work only), window ticks 760 to 789:

| Size | Load | GPU scene mean before | after | GPU scene max before | after |
| --- | --- | --- | --- | --- | --- |
| 3840x2160 | 39 to 47 | 3.97, 4.01, 3.98 ms | 3.72, 3.72, 3.72 ms | 11.20, 11.20, 11.15 ms | 9.50, 9.47, 9.48 ms |
| 1920x1080 | 48 to 58 | 1.62, 1.62 ms | 1.55, 1.55 ms | 3.56, 3.50 ms | 3.17, 3.20 ms |

That is -6 % over the window and -15 % at the worst frame (4K). The card ran slower in this
session than in the skip table above (other members' gates, load 40 to 50), but each run of a
pair sat within 0.05 ms of the others. The CPU columns were too loaded to read. The rows above
are the first cut, which also dropped additive quads of colour 0. The landed alpha-only rule was
measured against a baseline built from main, at load 23:

| Circuit, size | Window | GPU scene mean before | after | GPU scene max before | after |
| --- | --- | --- | --- | --- | --- |
| Talon's Junction (HD), 3840x2160 | ticks 760 to 789 | 2.40 ms | 2.22 ms | 6.58 ms | 5.00 ms |
| Mallavol (Fury, `zone_2`), 3840x2160, load 32 to 36 | ticks 890 to 959 | 3.67, 3.68 ms | 3.61, 3.61 ms | 5.07, 5.03 ms | 5.05, 4.95 ms |

Mallavol's run is the same script with `--track /data/environments/zone_2/track.vex`. Its hits
land further from the camera, so less of the fill is fading smoke.

**Identity:** screenshots at ticks 768, 774 and 777, at 1920x1080 and 3840x2160, are
byte-identical before and after (`cmp`). So are Pulse's at ticks 700, 720 and 740 after its own
craft hits (gamma target, sampled-sprite path, `pulse-psp-eu.chd`, same script), and Mallavol's
at ticks 905 and 945 (3840x2160). The state hash is unchanged.

## On the Steam Deck

2026-11-26, a Steam Deck OLED on AC power, RADV VANGOGH. The reproduction above (`fire.txt`),
offscreen race bench with an audio stream running, four builds: `d66a52ae6` (with the craft
blast's sprite templates drawn); `790ad033b` (without them); `259948344`, which already drops dead quads but has no near
law, no `WO_ROCKET_EXPLO` sprites and no blast carry; and `0d662907c` (`e12903bfc^1`), before all of those. Each is the portable
container build with `--features extensions,dev-cli` (`--give` needs `dev-cli`). `d66a52ae6` was measured
in a second session, interleaved with `790ad033b`, which repeated there within the spread. The settings
file holds only the reporter's HD render profile (`scene_precision = "float"`,
`shadow_resolution = "ultra"`, `circuit_lighting = "simplified"`, FXAA, motion blur off), so the
race is the one described above: the state hash at tick 800 is `62b06c2ee353b11d` on all four
builds, as on the workstation. Three interleaved rounds, medians over the rounds; the rounds
spread by about 2 ms at most. `--force-rocket-hit` was not used: it does not exist in the older builds.

`frame_us` of the bench (pipelined, no vsync, scene and bloom only, no FXAA or HUD pass), in
milliseconds.

**The craft hit, ticks 760 to 789** (30 frames, so p99 is the worst frame):

| Size, build | mean | p95 | worst | frames over 33.3 ms | over 16.7 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| 3840x2160 `0d662907c` | 48.3 | 103.6 | 108.5 | 20 | 30 |
| 3840x2160 `259948344` | 45.9 | 91.9 | 97.0 | 18 | 30 |
| 3840x2160 `790ad033b` | 30.5 | 54.2 | 63.9 | 8 | 30 |
| 3840x2160 `d66a52ae6` | 31.6 | 54.1 | 63.5 | 8 | 30 |
| 1920x1080 `0d662907c` | 22.4 | 53.6 | 55.1 | 6 | 16 |
| 1920x1080 `259948344` | 21.4 | 49.7 | 52.9 | 6 | 16 |
| 1920x1080 `790ad033b` | 12.2 | 21.6 | 28.5 | 0 | 5 |
| 1920x1080 `d66a52ae6` | 12.4 | 21.7 | 29.2 | 0 | 5 |
| 1280x800 `0d662907c` | 15.7 | 37.8 | 41.3 | 3 | 7 |
| 1280x800 `259948344` | 14.9 | 36.2 | 38.5 | 3 | 7 |
| 1280x800 `790ad033b` | 8.9 | 13.1 | 17.0 | 0 | 1 |
| 1280x800 `d66a52ae6` | 9.0 | 13.3 | 16.9 | 0 | 1 |

**The whole barrage, ticks 700 to 829** (130 frames, four blasts):

| Size, build | mean | p99 | worst | frames over 33.3 ms | over 16.7 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| 3840x2160 `0d662907c` | 35.5 | 103.6 | 108.5 | 59 | 122 |
| 3840x2160 `259948344` | 34.8 | 91.9 | 97.0 | 57 | 122 |
| 3840x2160 `790ad033b` | 31.2 | 74.9 | 78.2 | 45 | 119 |
| 3840x2160 `d66a52ae6` | 31.6 | 74.7 | 78.7 | 45 | 121 |
| 1920x1080 `0d662907c` | 16.0 | 53.6 | 55.1 | 8 | 51 |
| 1920x1080 `259948344` | 15.9 | 49.7 | 52.9 | 8 | 51 |
| 1920x1080 `790ad033b` | 13.5 | 34.0 | 37.4 | 2 | 37 |
| 1920x1080 `d66a52ae6` | 13.5 | 34.2 | 37.1 | 2 | 39 |
| 1280x800 `0d662907c` | 11.5 | 37.8 | 41.3 | 3 | 12 |
| 1280x800 `259948344` | 11.5 | 36.2 | 38.5 | 3 | 12 |
| 1280x800 `790ad033b` | 9.5 | 19.8 | 21.1 | 0 | 5 |
| 1280x800 `d66a52ae6` | 9.6 | 20.3 | 20.9 | 0 | 5 |

What it says:

- **The craft hit is much cheaper**: at 3840x2160 the worst frame falls from 108.5 to 63.5 ms,
  at 1280x800 from 41.3 to 16.9 ms (`0d662907c` to `d66a52ae6`). In the frames themselves, at tick 774 `0d662907c` is white
  from edge to edge and `790ad033b` shows fireballs around the struck craft.
- **At 3840x2160 it is still not fixed.** 121 of the 130 barrage frames are over 16.7 ms on
  `d66a52ae6` (the whole hit window, 30 of 30). A dynamic resolution target would shed pixels here; the
  bench has none.
- **The worst barrage frames are blasts these changes did not reach**: ticks 726 and 756
  cost 78 to 79 and 75 ms on the two newest builds, and 80 and 74 to 75 ms on the older ones.
  `frame_us` trails the GPU work by two ticks here (the bench holds two frames in flight): those
  two frames carry the scene passes of ticks 724 and 754, whose `gpu_scene_us` reads 73 and
  69 ms ("Per effect: what fills the heavy frames").
- **Tick 784 got heavier**: 51 ms on `790ad033b` and `d66a52ae6` against 43 on `0d662907c` at
  3840x2160, and the window's worst is now tick 786. That is the frame the blast carry and the sprites changed
  ("Measured and dropped", "Carried with the struck craft").
- **The sprite templates add nothing measurable** here: `d66a52ae6` is within the spread of
  `790ad033b` at every size (they draw: tick 765 shows the yellow wash, a flat-bottomed band
  across the middle of the frame).
- At 1280x800 the offscreen runs leave the GPU partly idle and clocked down (median 1040 MHz on
  `790ad033b`, 1134 on `0d662907c`), so those rows understate the gain if anything. At the larger
  sizes it ran at 1600 MHz.

**A second race, the reporter's whole settings file** (Ace AI, Veteran controls, so a different
race, hash `f2263f07316b7041` at tick 830 on all three builds): the four blasts land on the
track wall instead of a craft. Ticks 700 to 829, worst frame at 3840x2160: 92.7, 87.9 and
57.8 ms for `0d662907c`, `259948344` and `790ad033b`, 59, 59 and 38 frames over 33.3 ms; at
1280x800: 31.5, 27.6 and 14.8 ms.

## Measured and dropped

- **A fragment `discard` where the particle's output alpha is exactly 0** (the corners outside
  the procedural profile's circle, a sprite's clear texels). Byte-identical, but against the
  cull alone at 4K it read 3.66, 3.65 and 3.65 ms against 3.71, 3.72 and 3.62 ms (load 79 to 95).
  At that load, with the GPU shared with other members' gate tests, it did not show beyond
  noise. The result is inconclusive, not a no. It is worth re-measuring on a bandwidth-bound GPU
  (the Deck at 4K).
- **Sampled billboards trimmed to their sprite's live texels** (2026-11-26, dropped). Each atlas
  cell was cut, at load, to the rectangle outside which every bilinear sample reads alpha 0
  (texel `i` reaches `t` in `[i - 0.5, i + 1.5)`, one mip level, plus a 1-texel margin). The quad
  was built over that rectangle alone, with its UVs on the same affine map. This was single-sample
  passes only, and procedural, streak, distort and mod-shaded quads kept the whole quad. HD's
  craft-blast cells leave a lot to cut: `WO_ROCKET_EXPLO`'s smoke and fire keep 0.40 to 0.77 of
  the cell under the rectangle and 0.31 to 0.62 under the convex hull. `SHIP_DEBRIS` keeps 0.37.
  Measured on the reproduction above, whose worst frame is now tick 784, with the blast carried:
  - **Fill** (in screens, 1920x1080, the probe above, the same binary with the trim on and off):
    37 to 59 % less over ticks 766 to 781. The peak over ticks 760 to 789 went from 18.0 to 13.2
    screens. At tick 784, though, the additive layer holds 5.2 of 14.5 screens and the trim
    leaves it untouched (5.21 against 5.22), so that frame loses only 9 %.
  - **GPU scene time**, six interleaved pairs against main, RX 7800 XT, RADV, load 47 down to 19:

    | Size | Mean over 760 to 789, main | trimmed | Worst frame, main | trimmed |
    | --- | --- | --- | --- | --- |
    | 3840x2160 | 2.90 ms | 2.60 ms | 4.75 ms (4.25 to 4.85) | 4.62 ms |
    | 1920x1080 | 0.97 ms | 0.91 ms | 1.72 ms | 1.70 ms |

    At 4K, ticks 774 to 782 lost 0.46 to 0.70 ms each (medians), but the worst frame lost 0.13 ms.
    One trimmed 4K run fell in the card's faster regime (worst 2.75 ms) and is outside the medians.
  - **Not byte-identical.** Tick 775 at 3840x2160 differs from main on 27,853 pixels: 27,174 by
    one level and about 600 by 2 to 72. Main against main is identical. With the particle depth
    test switched to `Always` in both builds, the tail goes and 26,826 pixels differ by exactly
    one level. So the tail is depth-test flips where a quad meets the track or a craft. The rest
    is bilinear weight rounding. Both come from the GPU interpolating a different triangle, so a
    smaller shape on the same plane cannot keep the bytes. A wider margin (8 texels) left the
    same tail.

  The keep rule was a worst-frame gain of 0.3 ms at 4K and every comparison pair unmoved to three
  decimals. The worst frame failed it, so the trim was not kept.

## The overdraw is not all authored: the original shrinks, fades and drops near particles

2026-11-25, `particle-fill-fable`. The section this replaces said the live overdraw was the
effects as the disc authors them. That was wrong for the case that costs most. HD's particle
vertex builder applies three rules to every billboard, by its view depth, that ours does not
([particle-render.md](../ghidra/functions/ps3-hdfury-eu/particle-render.md) has the code, Omega's
twin and the live vertices):

1. **Cull**: nearer than 1 unit, not drawn.
2. **Fade**: alpha times `(depth - 1) / 3` between depth 1 and 4.
3. **Cap**: size at most `depth * cap`, the emitter's float `+0xc80`. The CraftBlast smoke
   authors 0.5 on six of its emitters and 1.0 on two, against sizes of 4 to 11 units, so its
   smoke is capped whenever it is nearer than 8 to 22 units.

So a blast the camera flies into is the case the original treats, and ours draws it untreated.

### What the law does to ours, measured

A throwaway patch (not kept) read `+0xc80` into the emitter and applied the three rules in
`psys::layers::extend_layer` to billboards, with the view matrix's z row handed in from
`Scene::upload_particles`, each rule switched by an environment variable. The same hunk summed
each quad's on-screen area after clipping to the frustum, in full screens. Same reproduction as
above; the run's state hash at tick 800 is `62b06c2ee353b11d` on this main (the craft hit moved
with the Rocket's launch speed since the first section's numbers, the worst frame is still tick
775) and is the same with the law on, since the law is drawing only.

| Tick | Additive, as now | Alpha-over, as now | Additive, law | Alpha-over, law | Cap alone (additive + over) | Cull alone |
| --- | --- | --- | --- | --- | --- | --- |
| 771 | 4.24 | 9.98 | 3.34 | 9.50 | 12.84 | 14.22 |
| 772 | 5.84 | 14.54 | 3.73 | 13.11 | 16.86 | 20.36 |
| 773 | 7.86 | 20.80 | 3.47 | 17.02 | 20.49 | 28.66 |
| 774 | 9.75 | 30.07 | 2.48 | 17.94 | 20.56 | 39.69 |
| 775 | 11.01 | 41.69 | 1.05 | 13.74 | 16.79 | 50.70 |
| 776 | 4.89 | 46.46 | 0.29 | 7.97 | 15.62 | 43.99 |
| 777 | 1.41 | 25.11 | 0.41 | 4.11 | 15.52 | 15.52 |

The worst frame falls from 52.7 screens to 14.8, and the worst frame with the law (tick 774) is
20.4 screens: **the peak is 2.6 times lower**. The cap does nearly all of it. The cull's own
effect on fill is small, but it removes most of the list: at tick 775, 3,890 alpha-over quads
become 285, because the smoke of earlier blasts behind the camera is no longer built. The fade
moves no fill; it is what the player sees.

GPU scene time over ticks 760 to 789, six interleaved pairs each, RX 7800 XT, RADV, load 47 to 60
with other members' work on the same card (the card ran in two speed regimes; the pairs agree
inside each):

| Size | Mean, as now | Mean, law | Worst frame, as now | Worst frame, law |
| --- | --- | --- | --- | --- |
| 3840x2160 | 3.71 ms (2.49 to 3.76) | 3.12 ms (1.95 to 3.14) | 9.65 ms (5.90 to 9.74) | 5.28 ms (3.18 to 5.41) |
| 1920x1080 | 1.53 ms | 1.46 ms | 3.29 ms | 3.15 ms |

At 3840x2160 that is -45 % at the worst frame and -16 % over the window. At 1920x1080 the worst
frame barely moves (-4 %), and that is a finding of its own: ticks 770 to 774 cost the same with
and without the law (3.26 ms at tick 774 both ways, against 0.75 ms before the hit) while their
fill halves, and only ticks 775 to 777 fall (3.29 to 2.68, 2.67 to 1.99, 1.57 to 1.33 ms). About
1.7 ms of a hit frame at 1080p on this card does not scale with resolution or with covered
area. It was not profiled.

**The picture.** At tick 775 ours is white from edge to edge. With the law the blast is a
bounded cloud around the struck craft, the track and the craft stay readable, and the smoke
nearest the camera is thin. At tick 773 the fireball is about two thirds as wide. This is a
fidelity change before it is a speed one, and it has no matched original frame yet: the live
capture that proved the law had its blasts 70 to 350 units away.

### How the PS3 affords the rest

- **It draws fewer pixels.** The scene target is 1280x720 with 2x MSAA, or at 1080p
  `1920 - 20 * level` by 1080 with no MSAA: 1.40 to 2.07 megapixels against 8.29 at 3840x2160.
  Particles go into that target and nowhere else.
- **It narrows the frame under load.** The width level steps one notch (20 pixels, 1 %) a
  frame while the RSX's frame takes 16.1 ms or more
  ([scene-width-throttle.md](../ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md)). A nine
  frame blast can shed 9 % that way; a sustained load reaches -32 %.
- **It has no reduced-resolution particle buffer.** The effect files author a per-distance
  resolution mode and the executable computes it, but the flush binds the scene's own targets
  whatever the mode (particle-render.md).

Put together: our worst frame blends 52.7 screens of 8.29 megapixels, 437 million fragments. The
same frame under the law at 1300x1080 is 20.4 screens of 1.40, 29 million: fifteen times fewer.
What a real console does at such a frame (how far the width falls, whether it holds 60) is not
measured.

### Our draw path against the original's

| | Original (HD, PS3) | Ours, HD defaults | Verdict |
| --- | --- | --- | --- |
| Target | The scene target, `A8R8G8B8`, 8 bit | The scene target, `Rgba8Unorm` (`scene_precision = "original"`) | same |
| MSAA on it | none at 1080p, 2x at 720p | off (`msaa = "off"`) | same |
| Depth | test `LEQUAL`, no write | test `Less`, no write | same in effect |
| Blend | `(SA, 1)` and `(SA, 1-SA)`, alpha kept | the same two | same |
| Sort | none: one run in submission order | none | same |
| Extra attachment | none | the velocity target, write-masked, when motion blur is on | ours only, no fill of its own |
| Near particles | cull, fade, cap | the same law on HD, 2048 and Omega since 2026-11-25 ("Wired") | same |
| Particles behind the camera | not built | not built under the law (nearer than 1 unit); Pulse and Pure build, upload and clip them | same on HD, 2048, Omega |
| Resolution | 1.4 to 2.07 megapixels, dynamic width | the output's, fixed (`render_scale = 100`, `target_fps = "off"`) | the player's choice |

Counts, sizes and lifetimes are the disc's in both: ours plays the same records, and no
difference was found there. The law above is the difference.

## Wired

2026-11-25. The law runs in `oag_fx::psys` (`near`), switched per title by
`oag_title::Effects::near_law` (HD, 2048 and Omega;
[particle-render.md](../ghidra/functions/ps3-hdfury-eu/particle-render.md), "Wired", lists what
of ours it reaches). The state hash at tick 800 of the reproduction is unchanged
(`62b06c2ee353b11d`): the law is drawing only. With the law switched off, the same code draws
ticks 775 and 777 byte-identically to the build before it.

Fill, in screens, from a throwaway counter over the whole run (the same one as the probe above),
and it reproduces the probe to the hundredth:

| Tick | Before | Wired |
| --- | --- | --- |
| 771 | 14.22 | 12.84 |
| 772 | 20.38 | 16.84 |
| 773 | 28.66 | 20.49 |
| 774 | 39.82 | 20.42 |
| 775 | 52.70 | 14.79 |
| 776 | 51.35 | 8.26 |
| 777 | 26.52 | 4.52 |

The peak over ticks 760 to 789 falls from 52.7 screens to 20.5.

GPU scene time over ticks 760 to 789, six interleaved pairs each, medians, the main build against
this one, RX 7800 XT, RADV, one-minute load 19 to 65 with other work on the same card:

| Size | Mean before | wired | Worst frame before | wired |
| --- | --- | --- | --- | --- |
| 3840x2160 | 2.27 ms | 1.96 ms | 5.53 ms (5.27 to 5.96) | 3.21 ms (3.16 to 3.22) |
| 1920x1080 | 1.53 ms | 1.48 ms | 3.26 ms | 3.16 ms |

At 3840x2160 the worst frame falls by 42 % and the window by 14 %; at 1920x1080 by 3 %, the part
that does not scale with fill (above). The absolute numbers are lower than the probe's table:
the card was in its faster regime.

**What is unchanged.** The law caps a particle's size at any depth, not only near the camera: a
smoke particle of size 11 under cap 0.5 is capped out to 22 units. So "particles beyond depth 4
look as before" is false, and identity holds for a frame in which no particle is culled, faded or
capped. Ticks 429 and 540 of the reproduction draw particles and touch none: their screenshots
are byte-identical before and after at 1920x1080 and 3840x2160. Pulse's craft hits (ticks 700,
720, 740, 1920x1080) are byte-identical, the law being off there.

No comparison pair moves: all 79 HD, Omega and 2048 pairs (the Detonator ones included),
re-rendered on scratch copies with the build before and the build with the law, are
byte-identical. None has a particle the law touches in frame.

## Against the original at a near blast

2026-11-26, `blast-capture`. **Verdict: ours does not match the original's picture of a near
craft blast, and by pixels it is further from it with the law than without.** The law itself is
the original's, read on the blast's own vertices. The difference comes from what ours does not
draw.

### Capture

RPCS3 0.0.43-20574, HD EU, Talon's Junction, Racebox Single Race, Venom, weapons on, Feisar.
`scripts/rpcs3-hd-rocket-film.py` holds the player 12 units behind the nearest rival (AG
Systems), fires a Rocket, reads the hit off the rival's shield (140 to 130, 9 or 10 game
frames after the press) and keeps frames 0 to 0.55 s after it, with the camera, every craft's pose and the
particle vertex buffers. Three hits, from three restores of one save state:
`data/reference/hd-capture/rocket-craft-hit/hit1`, `hit2`, `hit3`. At 0.18 s the eye is 3.5 to
3.8 units from the hit point.

```sh
scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-racebox-talons-venom-weapons-grid-rpcs3-20574.SAVESTAT.zst --no-gdb
OAG_RPCS3_ATTACH=1 OAG_RPCS3_NO_GDB=1 uv run --with evdev python3 scripts/rpcs3-hd-rocket-film.py --out <dir>
scripts/hd-particle-quads.py <dir>
```

**Ours** was drawn by a throwaway patch, not kept: `WO_ROCKET_EXPLO` played 2.5 units under
the hit point and aged `60 * age` ticks, every craft posed from the capture (`--field-from`,
`--ticks 0`), the camera the original's own. The law was switched off by the same patch. The
claim was to be falsified if the original was closer to ours without the law than with it, by
pixels (RMSE, mean luma, near-white share) at matched ages.

### The law is on the blast

| Billboard quads | Live | Nearer than 1 unit | Largest half height / depth | On cap 0.5 |
| --- | --- | --- | --- | --- |
| Original, hits 1 and 2, every particle quad of the 8 dumped frames after the shot | 6,874 | 0 (nearest 1.037) | 2.000 (below) | 146 |
| Original, hit 3, its 5 dumped frames | 3,879 | 0 (nearest 1.217) | 2.000 | 115 |
| Ours, law, the blast alone | 1,171 | 0 (nearest 1.020) | 1.000 | 128 |
| Ours, law off | 2,453 | 42 (nearest 0.002) | 188 | 1 |

The cull and the cap hold on the particles of a near craft hit, not only on the player's own
effects (confidence 85). The original's rows are not separated by effect: they hold every
particle in the frame, the struck craft's own sparks and smoke included, where our rows are the
blast alone (the no-blast render's quads removed). The fade was not separated. The ours rows
are the arm with the blast carried to the struck craft (below).

### The picture

Whole-frame RMSE against the original (0 to 255), the mean of ten frames: hits 1 and 2 at
0.1, 0.18, 0.23, 0.3 and 0.4 s. "Near-white" is the share of pixels with luma over 235; the
original's own is 27 to 70 % in these frames. The box is the central 1152x756.

| Ours | RMSE | RMSE, box | Near-white |
| --- | --- | --- | --- |
| As it ships, law | 130.9 | 139.2 | 4.5 % |
| As it ships, law off | 120.7 | 125.5 | 8.0 % |
| Sprites on, blast carried with the craft, law | 123.9 | 128.6 | 10.5 % |
| The same, law off | 109.2 | 114.2 | 22.5 % |
| No blast at all | 133.9 | 144.1 | 3.5 % |

- **Neither arm is within noise**: all four sit at 109 to 131, against 68 to 76 for the original
  against itself (below).
- **The falsifier fired too.** The law is the closer arm in 2 of 10 frames as shipped and 0 of
  10 with sprites and carry; with ours drawn 4 ticks younger, 1 of 10 and 0 of 10. The gap, 10
  to 15, has a steady sign and is small against what separates either arm from the original.
- For scale: ours against the original before the shot is 52 and 60. The original against
  itself (hit 1 against hit 2, same age, so the camera differs too) is 68 to 76 up to 0.23 s
  and 99 to 132 after.
- Screen fill from the dumped quads, the eight frames of hits 1 and 2 with a full dump: the
  original 74 screens; ours carried 16 with the law and 35 without; ours static 7 and 26.
  At the two heaviest frames, 12.3 against 4.6 and 11.4 (0.18 s), 19.7 against 5.1 and 14.1
  (0.23 s). The original's lists hold 3 to 4 times our live quads; they also hold whatever else
  the shot threw, which the patch did not inject, so this is not a per-effect count.
- With the blast placed whole at the struck craft's current position, law and law-off draw the
  same fill: at that distance nothing is capped. A frame like that cannot test the law.

So removing the law brings ours nearer only because it adds white to a picture that is far too
dark. It is not evidence that the original lacks the law, which its vertices carry. It does
mean the "bounded cloud, track and craft readable" of "What the law does to ours" above
describes ours, not the original: the original's frame is mostly white from 0.1 to 0.3 s.

### Why ours differs

1. **The blast travels with the struck craft; ours stays at the hit point** (confidence 85).
   Centroid of the original's large particles (half height over 2):

   | Hit, age | Craft moved | From the craft | From the hit point |
   | --- | --- | --- | --- |
   | 1, 0.18 s | 23.9 | 6.1 | 18.5 |
   | 2, 0.23 s | 29.0 | 9.6 | 19.4 |
   | 2, 0.40 s | 49.6 | 16.0 | 34.3 |
   | 3, 0.23 s | 32.0 | 11.3 | 21.5 |
   | 3, 0.40 s | 54.7 | 19.1 | 36.9 |

   The cloud lags the craft, as an effect whose emitters ride it and leave their smoke behind
   would. In ours the hit point is behind the camera after about 0.2 s, so the player flies
   through the blast and past it. **Read and wired 2026-11-26**: the blast is not parented; the
   spawn hands it the struck craft's velocity, which every particle carries times its emitter's
   `+0x4d0` (1.0 on the travelling roots), and drag makes it lag
   ([rocket-craft-blast.md](../ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md)).
2. **Our `WO_ROCKET_EXPLO` played no sprites** (it was not in HD's `Effects::sprites`, so every
   particle was the procedural white profile). Wired 2026-11-26: "With its sprites" below.
3. **The original washes the whole frame, HUD included, yellow to white.** Its particle lists
   hold single-quad additive draws with a 64x64 texture whose half height is exactly twice
   their depth, which covers the screen. All three hits have them in frames from the hit to
   0.23 s, none before the shot or from 0.3 s on. That they are the wash is not shown, and
   their source is not identified. Ours draws no screen flash on HD. Re-measured 2026-11-26:
   square, opaque, warm white to yellow, and 2 to 12 units from the spawn point while the
   struck craft is 13 to 30 units on, so they do not ride with the cloud
   ([rocket-craft-blast.md](../ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md)).
   **Named and wired 2026-11-26**: they are the blast's sprite templates, `glow_longer` on the
   root (the yellow) and the `Glow`s (the white of the hit frame), capped at 2 x depth; zeroing
   their gate in memory removed them from the original's frame
   ([rocket-craft-blast.md](../ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md#the-screen-wash-the-blasts-sprite-templates)).
4. **Streaks.** In the carried arm, 25 of our 1,055 quads whose corners differ in depth (our
   streaks) are nearer than 1 unit, with the law on or off: it does not touch streaks. The original's dumped lists hold only screen-facing quads,
   so where its `.pob` streaks are drawn is not known, and no comparison was made.

The picture lag under `SIGSTOP` was not measured: the frame on screen can trail guest memory by
a few ticks. The vertex buffers, camera and poses are from one stop. Ours does not vary with
`--seed`, so it has no noise floor of its own.

**Cross-title.** Omega and 2048 author their own `WO_ROCKET_EXPLO` and run the same law.
Their sprites were already read (below). Whether their blast travels with the craft is not
checkable here: no memory readout of Omega, and 2048 was not captured.

## With its sprites

2026-11-26. `WO_ROCKET_EXPLO` is in HD's `Effects::sprites`: all twelve emitters read their
`.gtf` and sit on the particle sheet. Fury is the same disc. 2048 (3 emitters, 3 sprites read)
and Omega (10 emitters, 10 sprites read) read theirs without a list, since `.gxt` and `.gnf`
load for every effect: **ported**, already
([particle_sheet_ground_truth.rs](../../crates/raceplay/tests/particle_sheet_ground_truth.rs)).

### The sprite sheet had to pack tighter

With twelve more sprites HD no longer fitted the 4096 sheet: `FIREBALL_TUMULT` was dropped and
`WO_QUAKE_DETONATOR`'s `EXPLOSIONS`, which fitted before, was pushed off. The packer filled
rows as tall as their tallest sprite and ran out under 60 % full. It now keeps the free space
as maximal rectangles and takes the highest, then leftmost, position
(`oag_fx::psys::sprite`, `packing`). Same sheet size, same padding, nothing dropped on HD,
2048, Omega (both effect sets), Pulse (PSP and PS2) or Pure; none of the others dropped one
before either, so the packer draws nothing new there.

A sprite that moves on the sheet can round differently when sampled. Main against this build,
HD, 1280x720 and the three Detonator comparison pairs: the pairs, a forced Missile hit and a
Detonator Quake frame (no wave in it) have identical pixels; two frames of a Detonator EMP
wave differ by 1 or 2 levels of 255 on 246 and 63 pixels, the same on every rerun.

### Against the capture

`--force-rocket-hit TICK:SLOT` puts a craft on a Rocket in flight, as `--force-missile-hit`
does. The scenario, hit 2's frames at 0.1 and 0.233 s: Talon's Junction, `--field-from` the
frame's own crafts (without it the opponents are inactive and no hit lands),
`--give rocket`, an input script of `709 none`, `1 square`, `400 none`,
`--force-rocket-hit 711:7`, `--ticks 718` or `726`, and the original's camera. Fire at 709 and
the hit at 711 put the blast about 12 units ahead of the held player, the original's gap at
the hit.

Where ours' player is held decides where the blast stands, so there are two geometries:

- **Held at the frame's own pose** (the field file's player row). The blast stands ahead of
  the player, roughly where the original's travelling cloud is. **Chosen**: ours does not
  carry the blast; this compares the pictures, not the motion.
- **Held at the original's pose at the hit** (that row replaced by `hit.json`'s
  `player_pose`). The blast stands at the original hit point, which is where ours' stays.

Whole-frame RMSE against the original, near law on in all, the same build with and without
the listing. The figure is `compare_env.overlap_score`'s `rmse` times 255, not the script
behind the ten-frame table above:

| Geometry | Age | Without sprites | With sprites |
| --- | --- | --- | --- |
| At the frame's pose | 0.1 s | 120.4 | 121.4 |
| At the frame's pose | 0.233 s | 146.0 | 129.8 |
| At the hit point | 0.1 s | 117.0 | 114.0 |
| At the hit point | 0.233 s | 138.5 | 133.2 |

These are two frames of one hit in a new scenario: they do not compare with the ten-frame
means above, which came from a throwaway patch. By eye the sprites turn white blobs and soft
discs into fire and smoke of the original's family; by RMSE three of the four improve and one
is level. All four stay far from the original, whose frame is washed nearly white
(reason 3).

### Carried with the struck craft

2026-11-26. With the carry wired, the same two pairs, the same tree with HD's
`craft_blast_carry` off (before) and on (after), `compare_env.overlap_score`'s RMSE times 255:

| Age | Before | After |
| --- | --- | --- |
| 0.1 s | 121.4 | 119.7 |
| 0.233 s | 129.8 | 126.2 |

The "before" column reproduces the table above exactly. These pairs hold our player at each
frame's own pose, chosen so a static blast stood near the original's travelling cloud; the
struck craft's speed in our pair run was not read. By eye the fireballs now surround the struck
craft instead of trailing off to the side. The frame is still far from
the original's near-white wash, whose source stays open.

## HD's particle sprites are bottom-up

2026-11-26. **HD's `.gtf` particle sprites were drawn upside down; the loader now turns them
top-down.** Confidence 95. The question was the far hit: a Rocket striking a craft about 30 units
ahead drew `glow_longer`'s half-disc sprite (`yellow_glow_hemi`) as a dome, and nobody had put it
beside the original.

**The original draws a flat-bottomed dome too, rising from the blast.** Two films, RPCS3
0.0.43-20574, HD EU, the weapons-grid save state, `scripts/rpcs3-hd-rocket-film.py --back 30
--mode hold`, the rival (AG Systems) struck at 30.0 units both times (a third film struck another
craft and is not used). The wash quad read from the command ring:

| Read | Original | Ours before |
| --- | --- | --- |
| Texture | 64x64 DXT45, rows 0 to 31 of GPU memory empty, the half disc in rows 32 to 63 | the same bytes, row for row (mean difference 0.2 of 255) |
| Which edge samples `v = 0` (row 0) | the quad's bottom edge | the quad's top edge |
| Half size | 40.0, then 22.12 | the same size curve: 22.16 at the age whose colour the quad carries (10 ticks) |
| Depth | test `LEQUAL`, no write | test `Less`, no write |
| Place | a fixed world point (identical at 0.1 and 0.233 s) | the spawn point, held |

So the half disc rises above the blast with its flat edge through the spawn point, and ours hung
below it with its flat edge on top.

**Why.** A `.gtf`'s rows run bottom-up ([hd-loading.md](../formats/hd-loading.md)), and
`oag_fx::psys::sprite::Sprite` is documented top-down, but `Sprite::from_gtf` kept the file's
rows. 2048's `.gxt` and Omega's `.gnf` of the same `yellow_glow_hemi` store the half disc in the
upper rows: **2048 and Omega: checked, differs** (their files are top-down and load as they
should). PSP and PS2 sprites come from top-down containers. The flip reaches every `.gtf`
sprite HD's particle effects load (`oag_title::Effects::sprites`), flipbook atlases included.
Which way each effect samples them, and which cell a flipbook starts on, are read below.

Pinned by the disc check `check_the_hemisphere_sits_in_the_top_half` (HD, 2048 and Omega).

### Against the original

Every HD pair of the comparison page was rendered with the build before and after (the 26 of
`hd` and the one of `hd-race`, which moved by 0.10 at the same score): of the 26, 23 did not move a
pixel, the Detonator cannon volley moved by 0.56 RMSE at the same score (44.03), and the two
Rocket pairs improved. RMSE against the original, in levels of 255; "dome" is the box
`200..1720 x 0..560` above the player's craft; seeds are `--stage-seed 1..4`, paired:

| Pair | Whole, before / after | Dome, before / after | Whole over seeds, before | Whole over seeds, after |
| --- | --- | --- | --- | --- |
| rocket-craft-hit 0.1 s | 118.0 / 94.2 | 142.1 / 89.5 | 111.2 to 116.2 | 93.3 to 96.1 |
| rocket-craft-hit 0.233 s | 125.2 / 123.6 | 143.3 / 122.3 | 94.0 to 126.1 | 104.6 to 113.8 |
| far hit 0.1 s | 34.3 / 34.1 | 31.1 / 30.1 | 34.4 to 35.0 | 34.0 to 35.2 |
| far hit 0.233 s | 68.5 / 64.7 | 83.0 / 76.6 | 68.2 to 84.0 | 62.4 to 79.5 |

The 0.233 s pairs swing by 10 and more with the blast's random arrangement; three of the near
pair's four seeds improved and one got worse (94.0 to 109.1). The far pair at 0.1 s is the white
flash filling the frame on both sides and says nothing about the half disc.

### Which effects draw a sprite

2026-11-26. HD's loader reads a `.gtf` only for an effect in `oag_hd`'s `SPRITES`
(`crates/hd/src/effects.rs`). Before this change 14 effects were listed; a census over the disc of
the 31 others found 89 emitter and template texture references, every one an existing
`/data/psys/tex/<stem>.gtf` that decodes.

**A listing draws a sprite only on a camera-facing emitter**, or on a streak whose effect is in
`FAR_BARS` (`EmitterSpec::samples_sprite`); any other streak samples nothing. So of the 31:

- **21 listed**, every camera-facing emitter now drawing its own sprite: `WO_MINE_EXPLO`,
  `WO_BOMB_SMOKERING`, `WO_MISSILE_EXPLO`, `WO_MISSILE_BOUNCE`, the three Detonator explosions,
  `WO_ROCKET_FLARE`, `WO_CANNON_SPARKS`, the four Shuriken effects, `WO_REPULSER_BLAST`,
  `WO_WEAPON_ABSORB`, `WO_SHIP_COLL_SPARK_DAMAGE`, `WO_SHIP_DEATH_DAMAGE_PLUME`,
  `WO_DEBRIS_FIRE`, `WO_DAMAGE_MILD`, `WO_DAMAGE_MODERATE`, `WO_MODESTO_STEAM_A`.
- **10 not listed, all-streak**: `WO_BOMB_RAYS`, `WO_REPULSER`, `WO_LEACHBEAM_ENERGY`,
  `WO_LEACHBEAM_CHARGING`, `WO_LEACHBEAM_ABSORB`, `WO_SHIP_SPARK_DAMAGE_LEACHBEAM`,
  `WO_SHIP_SPARK_DAMAGE_WEAPON`, `WO_SHIP_COLL_SPARK_NODAMAGE`, `WO_DAMAGE_CRITICAL`,
  `WO_BLUE_WELDER`.

The streak emitters of the 21 also stay procedural (the Mine's `streaks_fat`, `plasma_core` and
`sparks`, for instance). Listing one for `FAR_BARS` wants an original pair first.

The listing also gives these effects HD's flipbook walk, and each placed particle draws its spawn
frame from the stage's generator, so every effect after one in a frame is dealt another
arrangement. The sheet grows from 3076 to 3590 rows of 4096 on Talon's Junction (31 to 41
distinct sprites); nothing is dropped. Disc check: `every_hd_sprite_is_on_the_sheet_and_the_weapon_blasts_draw_theirs`
(`crates/raceplay/tests/particle_sheet_ground_truth.rs`).

**Against the original.** None of the 21 has a matched capture of its own (the original was seen
walking the sheets of the Detonator Mine and Bomb explosions,
[particle-render.md](../ghidra/functions/ps3-hdfury-eu/particle-render.md), "The atlas frame"). The HD comparison pairs
(31, `hd` and `hd-race`) were re-rendered before and after; the build is byte-deterministic (two
runs of each side identical), 26 pairs did not move a pixel and 5 did: the four Rocket hits and the
Detonator cannon volley (`WO_ROCKET_FLARE` is the only new effect in the 0.233 s near hit). The new listing deals the blast
another arrangement (above), so each moved pair was re-rendered over `--stage-seed` values, RMSE
against the original, mean (standard deviation), before then after:

| Pair | Seeds | Before | After | Difference |
| --- | --- | --- | --- | --- |
| rocket-craft-hit-0100 | 30 | 0.3655 (0.0028) | 0.3666 (0.0029) | +0.0011, 1.4 standard errors |
| rocket-craft-hit-0233 | 30 | 0.4110 (0.0406) | 0.4097 (0.0436) | -0.0013, 0.1 |
| rocket-craft-far-hit-0100 | 6 | 0.1368 (0.0020) | 0.1353 (0.0027) | -0.0015, 1.1 |
| rocket-craft-far-hit-0233 | 6 | 0.2974 (0.0238) | 0.2805 (0.0171) | -0.0169, 1.4 |
| detonator-talons-cannon-volley | 6 | 0.1738 (0.0031) | 0.1725 (0.0012) | -0.0014, 1.0 |

None is worse or better beyond the spread. One seed alone is not a measurement here: at the
pairs' own seed the 0.1 s far hit reads 0.003 worse and the 0.233 s near hit 0.035 better.

**Our own frames** (HD EU, Talon's Junction, 960x544, not compared with an original). A Mine tripped
by a rival behind the player: the white soft discs became grey smoke and brown rock debris. A
Missile hitting the rival ahead, 3 ticks on: the soft white dots became yellow molten debris; from
about 9 ticks the explosion model and flash fill the frame and the change is under 0.1 % of pixels.

**2048 and Omega: not checked here** (out of this change's scope); they read sprites by platform,
not by this list.

### Every effect that loads a sprite, read off the command ring

2026-11-26, RPCS3 0.0.43-20574, HD EU, Talon's Junction. `scripts/rpcs3-hd-particle-ring.py` fires a
weapon and keeps the command ring and the bound textures frame by frame;
`scripts/hd-particle-uv.py` reads every particle quad's view-space corners and `uv`, and names each
texture by finding its dumped bytes in the disc's `/data/psys/tex/*.gtf`.

```sh
scripts/rpcs3-hd-particle-ring.py --out FILM --state 7 --back 30 --window 0.25,0.9   # on a weapons-on grid
scripts/psarc.py extract IMAGE:PS3_GAME/USRDIR/DATA02.PSARC GTF /data/psys/tex/       # and DATA00, 03, 06
scripts/hd-particle-uv.py --gtf GTF --per-frame FILM
```

Films: `data/reference/hd-capture/particle-uv/` (`rocket1`, `plasma1`, `plasma2`, `missile2`,
`quake1`, `quake2`, `emp1`, `cannon1`). The two Rocket films of the section above,
`data/reference/hd-capture/rocket-craft-hit/far1` and `far3`, carry no textures: read them with
`--textures data/reference/hd-capture/particle-uv/rocket1/tex/textures.json`.

**Which effects can have moved.** Only those whose sprites ours loads: the Rocket's two blasts, the
ship explosion, the Missile's head, the five Plasma effects, the Quake, the two trail hits, and the
Detonator's cannon spark and EMP. The Mine, Bomb, Missile explosion and Detonator Mine and Bomb
explosions loaded no `.gtf` at the time of these films (they do now, above), so the flip could not
reach them.

**Handedness.** The sign of the determinant of `d(view xy) / d(uv)` says whether a quad shows its
picture mirrored, whatever its roll. The reference is `yellow_glow_hemi`, whose orientation is
settled above (`u` right, stored row 0 on the bottom edge), and ours builds every billboard, rolled
quad, template and strip with that handedness. Confidence 94.

| Weapon fired | Particle quads read, same sign as the reference | opposite sign |
| --- | --- | --- |
| Rocket craft hit (three films, two boots) | 9,565 | 2 |
| Plasma (two films) | 9,789 | 0 |
| Missile (one film) | 7,722 | 0 |
| Quake (two films) | 10,294 | 0 |
| Detonator EMP (one film) | 21,458 | 0 |
| Detonator cannon (one film) | 133,436 | 180 (0.13 %), nearly all `spark_mblurred_grey`, a symmetric sprite; cause not read |

The counts are every particle quad in the frames, the effects' own and whatever else was in view.
Missile and Detonator are seen once. The Detonator save state is refused by this RPCS3 build, so its
grid was reached by walking Racebox from a cold boot (RACE TYPE Detonator, Talon's Junction); the EMP
was fired with `--button l1 --emp-charge 12 --press-frames 18`, the cannon with `--press-frames 90`.

**Strips (render mode 5 and 6): which end is which.** For a stretched quad whose long axis carries
`v`, the end farther from its texture's own centre in that frame, larger `v` against smaller `v`:
`explosion_01_blurred_64x64_ADD` 83 against 5, `..._yellow_streaks_upsidedown` 19 against 1,
`..._yellow_streaks_128x128` 192 against 4, `..._white_streaks_128x128` 80 against 6,
`fireball_1024x512_yellow` 101 against 0. So an outward streak leads with the authored top, as
ours' bar does (the particle's position takes the top row). The Plasma wind-up runs the other way,
7 against 192 and 45 against 1,559 on the two films (`glow_cleanedges_plasma`), 0 against 33 and
0 against 287 (`plasma_forks`), and its two emitters
author a negative speed (-0.50 and -0.48 units a tick): they fly inward. Confidence 90.

Ours samples no sprite at all on the strips of the Rocket's blasts, the Missile head, the Quake and
the ship explosion (`Effects::far_bars` does not list them), so this reaches the Plasma, the trail
hits and the two Detonator effects.

### HD advances its flipbooks, from the authored top-left

The four emitters of `WO_ROCKET_EXPLO` that are not random and spread their sheet over their life
(flag `0x40`; authored lifetime 64 ticks) give the order directly. Highest cell read per frame,
numbered row-major from the authored top-left, in the two films of "The original draws a
flat-bottomed dome too":

| Age since the hit | 0.05 s | 0.1 s | 0.167 s | 0.233 s | 0.4 s | 0.55 s |
| --- | --- | --- | --- | --- | --- | --- |
| Age in ticks | 3 | 6 | 10 | 14 | 24 | 33 |
| 8x8 sheets (`smoke_tumult`, `explosion_tumult`, `explosion_BURN_L`) | 2 | 5 | 10 | 13 | 23 | 32 |
| 8x4 sheet (`smoke_BLUCLD`) | 1 | 2 | 5 | 6 | 11 | 16 |

Cell numbering from the top-left is therefore right (confidence 94): numbered from the first stored
row the same cells would run backwards, 58 down to 24. The values are Pulse's over-life law,
`floor(age / lifetime * (frames - 0.02))` (`oag_fx::psys::frames`), to within a tick (confidence 90
for the over-life law).
The 8x8 rows alone would not say which law this is: 64 frames over 64 ticks is also one frame a
tick. The 8x4 sheet reads about half the age, which is the over-life spread; its emitter's
frame-rate channel (a constant 1) would have given the age itself.

**Ours does this on HD and Fury** (`oag_title::Effects::flipbooks`), with the rate law, the
authored spawn range and the hold frame, all read in the executable and checked against the same
films ([particle-render.md](../ghidra/functions/ps3-hdfury-eu/particle-render.md#the-atlas-frame)).
Before, every HD particle stayed on its spawn frame, so the Rocket craft blast's four layers
above stayed on the tiny seed of the explosion.

**2048 and Omega: the sheets are the same, their updates differ.** All 17 flipbook sheets HD's
listed effects name ship in both as `.gxt` and `.gnf`, and each matches HD's turned sheet
(correlation 0.99 to 1.00; the same sheet turned again gives -0.11 to 0.89). **Flip: checked,
differs** (their files are already top-down). **Advance: not wired**; the outcome per reader is
in [particle-render.md](../ghidra/functions/ps3-hdfury-eu/particle-render.md#the-atlas-frame).

### The pairs with the advance in, and the oversize trail

All 31 HD `pair.json` pairs under `compare-page`, rendered with the builds either side of the
change: 27 byte-identical, the four Rocket craft-hit pairs moved. Whole-frame RMSE against the
original, mean over particle-stage seeds (`--stage-seed`), levels of 255:

| Pair | Seeds | Before | After | Seed spread (after) |
| --- | --- | --- | --- | --- |
| `rocket-craft-hit-0233` | 1-8 | 113.6 | 106.5 | 83.1 to 120.2 |
| `rocket-craft-hit-0100` | 1-8 | 94.8 | 93.1 | 92.0 to 94.4 |
| `rocket-craft-far-hit-0100` | 1-6 | 34.9 | 34.9 | 33.9 to 35.5 |
| `rocket-craft-far-hit-0233` | 1-24 | 69.2 | 72.0 | 60.9 to 85.4 |

The far hit at 0.233 s is **worse by 2.8**, paired by seed (about three standard errors).

- It is the spawn range alone: with the advance and the hold but a full-grid spawn it is back to
  69.2 (seeds 1-8). The same range is what gains on the near pairs (without it
  `rocket-craft-hit-0233` reads 112.9 and `-0100` 94.5).
- The range is what the original does. `trail`'s sheet `smoke_new4_8x4` (only `trail` binds it)
  reads cells 1 to 17 at 0.233 s and up to 27 at 0.4 s on the far film `far3`: `[0, 3]` plus the
  age. A full-grid spawn would read up to 31. Those first two rows are solid white and
  yellow-white fireballs; before, ours drew mostly the dim later cells.
- **Our `trail` quads are too large**, which the dim cells hid. The original's half size on that
  frame (depth 33 to 84 units, so no near cap) is 2.35 at cells 3 and 4 and 3.57 at cell 14,
  about `2.2 + 0.09 * cell`; ours is the size channel's 3.5 to 3.8 over the same ages, 2.2 times
  the area at spawn. Both draw them white, opaque and alpha over.
- Falsifier: a scratch build holding `trail`'s half size to the original's growth reads 69.2 on
  the far pair (seeds 1-8, from 73.2), and 109.3 and 93.6 on the near pairs, still better than
  before. The mechanism is HD's per-particle size multiplier
  ([particle-render.md](../ghidra/functions/ps3-hdfury-eu/particle-render.md#the-per-particle-size-multiplier)).

### With the size multiplier wired

Same 31 pairs, 27 byte-identical. Means over the same seeds; "before" has neither change:

| Pair | Before | Advance | Advance and multiplier | Paired, multiplier build minus before |
| --- | --- | --- | --- | --- |
| `rocket-craft-hit-0233` | 113.6 | 106.5 | 109.2 | -4.3 +/- 3.2 |
| `rocket-craft-hit-0100` | 94.8 | 93.1 | 93.3 | -1.5 +/- 0.4 |
| `rocket-craft-far-hit-0100` | 34.9 | 34.9 | 34.9 | 0.0 |
| `rocket-craft-far-hit-0233` | 69.15 | 71.95 | 69.32 | +0.17 +/- 0.68 |

At the compare page's own seed `far-hit-0233` reads 64.71 before and 65.49 after. **The far
picture still shows a flat white cloud** a good deal larger than the original's, which the
multiplier does not remove: its cause is the pose, below.

### The pose throws the far blast at the camera

`trail` is the child of `FIRE_TRAILS_ONEQUAD`, whose pieces inherit the struck craft's velocity
([rocket-craft-blast.md](../ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md)). Each far
pair's `field.csv` puts the struck craft on its film frame's pose at the start of our run, and it
then drives more than 700 ticks under our driver before the forced hit. So it moves at
`(-128, -15, -46)` units per second when hit in the 0.233 s pair, where the film recorded
`(88, 11, 55)`. Ours therefore come toward the eye: the `trail` quads' median view depth is 32
against the original's 51, about 2.6 times the area on screen. It is not a particle law.

`--force-rocket-hit-velocity VX,VY,VZ` (builds with `dev-cli`) gives the struck craft a velocity
as the hit is forced, and its craft blast carries exactly that. Exactly, because the tick that
lands the hit changes the craft's own before the blast reads it: set to `(88, 11, 55)`, it read
`(89, 31, 56)` there.

**The film's value** is `88.067871,11.234680,55.156158`: the struck craft's body velocity
(`ship + 0x6944`, then `+0x190`, the vector `Rocket_SpawnCraftBlast` reads) in
`rocket-craft-hit/far3`, `hit.json` `struck_pose.vel`, read at the first stepped frame with its
shield down (140 to 130), the hit both far pairs show. It is read after that frame ran, so what
the spawn read may differ by a frame's change.

With it, whole-frame RMSE (`rocket-craft-far-hit-0233` over stage seeds 1-24, `-0100` over 1-6;
every build in a row given the same velocity):

| Pair | Neither change | Advance | Advance and multiplier | The same pair without the velocity, neither change |
| --- | --- | --- | --- | --- |
| `rocket-craft-far-hit-0233` | 67.44 | 66.94 | 67.19 | 69.15 |
| `rocket-craft-far-hit-0100` | 34.40 | 34.40 | 34.40 | 34.89 |

At 0.233 s both changes together are 0.25 +/- 0.04 better than neither (19 of 24 seeds), and the
multiplier gives back 0.26 of what the advance alone gains. At the compare page's own seed the
two far pairs read 66.50 and 33.60, against 64.71 and 34.06 without velocity or either change.

**The picture is still not the original's.** The cloud is smaller, `trail` reads as smooth white
lobes, and the grey smoke streaming toward the camera is missing (pixels with every channel over
235 at 0.233 s: 5.9 % against 11.1 %). The near pairs are posed below ("Posing the near hits").

### Posing a far hit

`--force-rocket-hit TICK:SLOT` moves the craft onto the Rocket in flight, so the tick sets where
the blast stands: the 0.233 s film's hit is `--force-rocket-hit 717:7` after the shot at 709
(spawn 1.1 units from the original's wash quad; 716 is 4.0 short). The 12-unit pairs are below
("Posing the near hits"). The ring's wash quad is a few ticks
younger than the film's labelled age (2 ticks at 0.1 s, 10 at 0.233 s, read off its colour and
size), so the far pairs render ours at the quad's age. The struck craft's velocity is posed by
`--force-rocket-hit-velocity` ("The pose throws the far blast at the camera").

### Posing the near hits

2026-11-26. The near pairs (`rocket-craft-hit-0100`, `-0233`) are the film `hit2`. **The original
puts its blasts at two points; ours cannot be put on both, and on the 0.233 s pair on neither.**

**Where the original's stand** (confidence 90). Method: the world centroid of every capped single
additive quad of a frame (the draws `scripts/hd-particle-quads.py` reads), through that frame's
camera. Every such quad of `hit1`, `hit2` and `hit3` stands on one of two points per film, each
the same in every frame that has it:

| Film | P1 (first wave) | P2 (second wave) | Struck craft's shield |
| --- | --- | --- | --- |
| hit1 | (489.92, -31.00, -63.88) | (498.51, -30.69, -56.60) | 140, 130, then 110 between 0.05 and 0.1 s |
| hit2 | (525.15, -28.26, -34.80) | (533.12, -27.12, -27.35) | 140, 130, then 110 between 0.05 and 0.1 s |
| hit3 | (493.56, -30.99, -51.43) | (504.80, -30.18, -42.51) | 140, 130, then 110 between 0.1 and 0.183 s |

- **The volley is three blasts in two waves** (confidence 85). P2 carries twice P1's template
  quads (2 yellow and 4 white against 1 and 2) and the shield falls 10, then 20: one Rocket
  hits, then the other two together 5 to 6 ticks later. Why the pair arrives later is unread.
- **Each site is where the struck craft was about 2 ticks before the blast was born**
  (confidence 90). The birth tick comes from `glow_longer`'s colour (blue 105 falling to 21 over
  30 ticks); the position from the struck craft's recorded poses, dropped 2.5:

  | Film, site | Craft stood there at tick | Distance off | Blast born at tick |
  | --- | --- | --- | --- |
  | hit1 P1, P2 | -1.0, 4.0 | 0.30, 0.05 | 0.9, 6.0 |
  | hit2 P1, P2 | -2.0, 3.0 | 0.29, 0.01 | -0.1, 4.9 |
  | hit3 P1, P2 | -1.0, 5.0 | 0.30, 0.02 | 1.0, 6.9 |

  Ticks count from the film's first frame with the shield down (`hit.json`). A negative tick is
  that pose stepped back by its velocity. The 2-tick lag is measured on the P2 rows, which
  fall between recorded poses; the P1 rows agree only within that step back (their 0.3 units may
  be the hit changing the velocity, unread). The lag's cause is unread (an old position copy, a
  late template clock or the ring trailing guest memory).

**Ours.** The pairs hold the player at `field.csv` row 0, each film frame's own pose (the
`--pose` flag in the same pair loses to `--field-from`), and the Rocket leaves from there. All
three of our Rockets strike on the forced tick at one point. Read with a throwaway print of each
blast's position, the earliest possible tick being 709:

| Pair | Forced tick | Our spawn | From P1 | From P2 |
| --- | --- | --- | --- | --- |
| 0100 | 709 | (532.19, -27.74, -28.48) | 9.5 | 1.6 |
| 0100 | 710 | (534.93, -27.32, -26.04) | 13.2 | 2.2 |
| 0100 | 711 (was) | (537.65, -26.92, -23.61) | 16.8 | 5.9 |
| 0233 | 709 | (545.23, -25.81, -16.91) | 27.0 | 16.0 |
| 0233 | 711 (was) | (550.71, -25.00, -12.02) | 34.4 | 23.4 |

The velocity the blast was handed without the flag is our struck opponent's,
`(-104, -11, -37)`, back toward the camera. The film's is `(89.31, 13.92, 96.76)` (`hit.json`
`struck_pose.vel`).

**The pairs.** `overlap.mean_grey` first (the page's number), then whole-frame RMSE in levels of
255; "seeds" is the mean over `--stage-seed 1..8`, the difference paired against the current
flags with its standard error. Ages count from our spawn: 716 minus 710 is 6 ticks. "P2 age" is
the second wave's age (1 tick at 0.1 s, 9 at 0.233 s), "label" the film frame's (6 and 14).
Wherever `--ticks` moves, `--world-clock-offset` moves by the same time the other way, so the
scenery stays in phase. The film velocity in these rows is `89.31,13.92,96.76`.

| Pair and flags (tick / `--ticks`) | mean_grey, own seed | RMSE, own seed | mean_grey, seeds | Paired difference |
| --- | --- | --- | --- | --- |
| 0100 current (711 / 718) | 72.13 | 93.11 | 72.17 | |
| 0100 film velocity (711 / 718) | 74.11 | 94.28 | 74.19 | +2.03 +/- 0.33 |
| 0100 P2 age, with or without velocity (709 / 711) | 76.75 | 98.06 | 76.72 | +4.56 +/- 0.33 |
| **0100 label age, film velocity (709 / 716)** | **65.75** | **88.15** | **65.74** | **-6.42 +/- 0.33** |
| 0100 label age, no velocity (709 / 716) | 59.74 | 87.01 | not run | |
| 0233 current (711 / 726) | 62.86 | 96.52 | 74.16 (63.5 to 85.5) | |
| 0233 film velocity (711 / 726) | 102.94 | 127.82 | 102.97 | +28.81 +/- 2.84 |
| 0233 P2 age, no velocity (709 / 719) | 56.16 | 93.03 | 63.34 | -10.82 +/- 4.16 |
| 0233 P2 age, film velocity (709 / 719) | 90.04 | 117.02 | 89.99 | +15.82 +/- 2.86 |
| 0233 label age, film velocity (709 / 724) | 87.03 | 114.58 | 86.91 | +12.75 +/- 2.86 |

Every row's own seed rendered twice byte-identical.

- **0100 takes the 709 / 716 pose with the film velocity.** It is the second wave's place at
  the first wave's age: three blasts at one point cannot stand in for both waves. With the
  velocity written to six decimals (`89.309692,13.924740,96.755127`) it reads 65.75 twice.
  Without the velocity it reads lower still, because our opponent's backward velocity brings the
  cloud to the camera.
- **0233 keeps its flags.** The original's eye there is about 6.8 units from P2 and ours cannot
  spawn nearer than 16. Its current score is flattered by the wrong velocity carrying the cloud
  into the eye (and by its seed: 62.86 sits at the low end of its spread); the film's velocity
  carries it away. The rows without the velocity flag score better for that reason and are not
  posed.
- **Not reachable with `--force-rocket-hit`**: P1 on either pair (it is behind the held player's
  Rocket), and the 1 then 2 volley. `--force-rocket-hit-at` reaches both ("The volley posed at
  its two points").

**2048 and Omega: not checked** (an HD measurement; neither was filmed).

### The volley posed at its two points

2026-11-26. `--force-rocket-hit-at TICK:X,Y,Z[:COUNT]` (dev only, repeatable) plays `COUNT`
craft blasts at a world point at the end of a tick and takes as many of the player's Rockets out
of the air; `--force-rocket-hit-velocity` gives the blasts their velocity. No craft is struck,
so the struck craft's hit sparks and shield bulge, which a `--force-rocket-hit` frame has, are
not drawn. It shares the real hit's flash and effect call, and a tick-for-tick test holds it to a real
forced hit (`crates/raceplay/src/tests/craft_blast_carry.rs`). Each wave of `hit2` is one flag:

```text
--force-rocket-hit-at 709:525.15,-28.26,-34.80      # 0.1 s pair (--ticks 716): P1, then
--force-rocket-hit-at 714:533.12,-27.12,-27.35:2    # P2 five ticks later
--force-rocket-hit-at 711:525.15,-28.26,-34.80      # 0.233 s pair (--ticks 726): P1, then
--force-rocket-hit-at 716:533.12,-27.12,-27.35:2    # P2
--force-rocket-hit-velocity=89.309692,13.924740,96.755127
```

The ticks give each wave the film's age (6 and 1 at 0.1 s, 14 and 9 at 0.233 s), and the pairs
keep their `--ticks` and `--world-clock-offset`. None of the player's other Rockets hits anything
between the waves.

`overlap.mean_grey`, then whole-frame RMSE in levels of 255. "Seeds" is the mean over
`--stage-seed 1..8`, the difference paired by seed against the current flags with its standard
error; negative is closer to the original.

| Pair and flags | Own seed | Seeds | Paired difference |
| --- | --- | --- | --- |
| 0100 current (one point, 709) | 65.75, 88.15 | 65.74, 88.13 | |
| 0100 two waves, film velocity | 67.10, 90.25 | 67.08, 90.22 | +1.34 +/- 0.08, +2.09 +/- 0.09 |
| 0100 two waves, no velocity | 67.03, 91.41 | 66.36, 90.71 | +0.62 +/- 0.33, +2.58 +/- 0.16 |
| 0100 first wave only, film velocity | 61.46, 82.82 | 63.90, 85.32 | -1.84 +/- 0.30, -2.80 +/- 0.29 |
| 0233 current (one point, 711) | 62.86, 96.52 | 74.16, 108.64 | |
| 0233 two waves, film velocity | 61.54, 100.01 | 71.05, 110.11 | -3.11 +/- 2.21, +1.48 +/- 1.92 |
| 0233 two waves, no velocity | 83.14, 119.49 | 79.66, 115.77 | +5.50 +/- 6.01, +7.14 +/- 5.39 |
| 0233 first wave only, film velocity | 78.15, 114.74 | 89.75, 125.00 | +15.59 +/- 3.25, +16.36 +/- 2.72 |

- **The faithful pose does not bring the 0.1 s pair closer**: it is further on all eight seeds.
- **At 0.233 s it is not separable from the current flags**: mean grey falls by 1.4 standard
  errors and RMSE does not fall.
- **What differs by eye** (0.1 s). Both waves together fill the centre white and hide the held
  craft, which the original's frame shows clear of the cloud. Along the view axis the drawn craft
  (`field.csv` row 0; `--field-from` wins over `--pose`) is 15.9 units out, P1 is 10.4 and P2 21.2.
  So the first wave stands about 5.5 units in front of the craft and covers it, and the second
  stands behind it: alone it leaves the craft in view and draws as a hard-edged white sheet. Why
  the original's cloud does not cover its craft is answered below ("The first wave and the drawn
  hull"): the original draws its craft 2 ticks further back, in front of the cloud.
- The first-wave-only row is not a pose: it leaves out two of the three Rockets.

**2048 and Omega: not checked** (the numbers are HD's; the flag itself is title-agnostic).

### The first wave and the drawn hull

2026-11-26. **The first wave's cloud is not what covers the craft. The pairs draw the craft in the
wrong place.** `field.csv` is the RAM pose read in the pause, 2 whole ticks ahead of the push-buffer
frame that both the picture and the camera come from. The original draws its craft at view depth
11.6, against our 15.9. P1 is at 10.36, and the cloud is carried forward past the craft. So the
original's craft stands in front of nearly all of the cloud, and ours stands behind it. This is the
pause skew in [compare-page.md](../tools/compare-page.md), "The skew is the pause, not the
original's draw order" (confidence 90 there), now measured on these frames (confidence 90).

**The read.** Walk `hit2`'s command stream for frame `a0100` and take the vertex constants each
draw loads. The player's hull matrix (rotation rows scaled 0.75, which are the RAM rows) is loaded
by draws 96 to 125 at `(526.054, -25.561, -33.823)`. The RAM body is
`(529.363, -25.017, -30.999)` with velocity `(98.31, 16.65, 86.59)`: 2.02, 1.95 and 1.95 ticks
back on the three axes, 0.069 units off the velocity line. `a0233` gives 2 ticks, 0.068 off. The
far films' player is held at rest (0.35 units a second), so nothing is measurable there, and their
field is unchanged.

The dump also holds the next frame's push buffer, under construction at `0x40496000`, with the
hull 1 tick back. A scan of raw memory finds that one first, because it is nearer the body. Only
the walked stream's matrix is the picture's. Rendered at rewinds of 1, 2 and 3 ticks, the hull is
the original's size and place only at 2: 1 is small, high and yellow under the cloud, and 3 is too
large.

**Ruled out first.**

- **Draw order.** The particle run is draws 507 to 586 of 660, after the hull's, with depth test
  on and no depth write, the same as ours.
- **The cloud's emitters.** Leaving out `WO_ROCKET_EXPLO` emitters 0, 1, 4, 7 and 9 freed our
  craft, but mean grey stayed at 67.24 (from 67.10). Leaving out the yellow wash (emitter 12)
  made it worse, at 73.21.
- **Size and travel.** Matched per emitter against the original's draws 62 to 79, our
  quads' half sizes and depths agree. Only the newest particle of each root differs: ours is born
  at the spawn point, the original's one tick on.

**The tool.** `python3 scripts/compare_env.py film-pair-field FILM STEM PAIR_DIR` reads the hull
off the frame's walked stream and checks the skew with `ps3_pose.whole_ticks`, which refuses a
non-whole skew. It moves every craft in `field.csv` back that many ticks of its own velocity and
records `original.presented_lag.craft_ticks` in `pair.json`. A player at rest reads 0. Its
selftest runs under `just check-pair-provenance`.

**The pairs.** Mean grey on the own seed, rendered twice byte-identical. "Seeds" is the
difference paired against current+stored over `--stage-seed 1..8`. Negative is closer.

| Pair | Flags, field | Own seed | Seeds, mean grey | Seeds, RMSE |
| --- | --- | --- | --- | --- |
| 0100 | current, stored | 65.75 | | |
| 0100 | current, rewound | **53.96** | -11.65 +/- 0.03, 8 of 8 | -10.98 +/- 0.08 |
| 0100 | two waves, stored | 67.10 | +1.34 +/- 0.08, 0 of 8 | +2.09 +/- 0.09 |
| 0100 | two waves, rewound | 64.39 | -0.64 +/- 0.14, 7 of 8 | -0.34 +/- 0.13 |
| 0233 | current, stored | 62.86 | | |
| 0233 | current, rewound | **50.36** | -11.48 +/- 5.30, 7 of 8 | -10.64 +/- 4.92 |
| 0233 | two waves, stored | 61.54 | -3.11 +/- 2.21, 6 of 8 | +1.48 +/- 1.92 |
| 0233 | two waves, rewound | 90.03 | +1.09 +/- 3.64, 2 of 8 | +4.51 +/- 3.25 |

The far pairs (`rocket-craft-far-hit-0100`, `-0233`) are unchanged at 12.91 and 45.99.

- **0100, current flags with the rewound field.** By eye this is the closest our frame has been:
  the craft is the original's size and place with the cloud behind it.
- **0233.** It improves on every seed metric. By eye our craft is still lost under white, where
  the original's is tinted yellow and in plain view.
- **The two-wave pose.** Rewound, it beats the stored current flags on 0100 but not the rewound
  current flags. Its two washes are cut by the track floor into hard edges, while the original's
  cover the frame. That is not read.

**2048 and Omega: not checked.** The brief says "Scope: HD only", and the skew is an RPCS3 pause
property.

### The trails start on the blast's hemisphere

2026-11-27. **On the 0.233 s near pair our player's craft was hidden under white because the
craft blast spawned every emitter at its anchor. HD spreads them by the extent law: the four
`FIRE_TRAILS_ONEQUAD` heads start on their authored hemisphere of radius 8.824, so the `trail`
smoke they leave fans out around the blast instead of stacking along the carry axis in front of
the camera.** Confidence 85: the disc value and three films agree, the spawn routine that applies
it on HD is not read (below).

**Ruled out first** (falsifiers written before each run; `hit2` frame `a0233`):

- **Depth, stencil, depth bounds, soft particles.** The particle run (draws 516 to 596) sets
  depth test `LEQUAL`, mask off, stencil, depth bounds and alpha test off; no texture unit points
  at the zeta surface. Its projection gives window depth within `1e-6` of the scene's at every
  distance. The alpha-over fragment program (local `0x01111000`, dumped with the ring) is
  `TEX R0, f[TEX0], tex0; MUL R0, f[COL0], R0; END`, the same `texel * colour` as ours, and the
  hull is drawn (draws 436 to 466) before the particles, as ours is.
- **The sprites.** `trail` samples `smoke_new4_8x4.gtf` in both, and the original's chains sample
  flipbook cells 3 to 12, the dense first half, as ours do.
- **The heads' speed.** HD's two velocity routines (`0x00291f18` for velocity mode 1,
  `0x00292118` for 0 and 2) take the speed as `0x0028c978(instance +0x70, +0x74)`, centre plus
  spread times `U(-1, 1)`, our form. Scaling our heads' speed by 1.55 (throwaway) left 13 to 18
  `trail` quads over the craft on seeds 1 to 4, against the threshold of 5.
- **The tint.** With our alpha-over smoke muted the craft reads yellow, and stays yellow with
  `glow_longer` muted too: the tint is the additive fireballs in front of the hull, so it needs
  the blast where the original's stands (the pose), not a law.

**What differs.** Over the craft (screen point `(-0.02, -0.24)`), the original stacks about 3
`trail` quads, ours 13 to 20. Plotted on the screen, the original's four chains per blast sit
apart around the frame, ours pile into one blob over the craft. Each chain's nearest quad to its
blast's `glow_longer` (the blast's site) is the head's spawn point plus at most a tick of travel
(`scripts/hd-trail-onset.py`):

| Film, frame | Chains | Onset, units from the site |
| --- | --- | --- |
| hit1 `a0167` | 12 | 4.3, 6.7, 7.6, 7.8, 8.2, 8.4, 8.4, 8.7, 9.0, 9.0, 9.0, 14.1 |
| hit2 `a0100` | 4 | 8.3, 8.4, 8.5, 9.1 |
| hit2 `a0233` | 11 | 8.6, 8.8, 9.0, 9.2, 9.2, 9.5, 9.6, 12.0, 15.0, 18.5, 19.6 |
| hit3 `a0100` | 4 | 8.4, 8.4, 8.4, 8.6 |
| hit3 `a0233` | 12 | 5.2, 7.9, 8.0, 8.5, 8.7, 9.2, 9.2, 9.3, 9.4, 12.3, 13.0, 13.9 |

The disc authors `FIRE_TRAILS_ONEQUAD` as shape 7 (hemisphere), `+0x34 = 8.824`, radius mode 0
(exactly the extent), spread `7.745` unused in that mode. Mode 1 would scatter onsets from about
1 to 17; the films cluster at 8.3 to 9.6. The longer onsets are chains whose first quads the
near law drops (the sites are 3.7 units or less from the eye), the shorter ones a chain measured
against the other blast's site. Ours spawned every head at the anchor (`Spawn::Point`, by
`without_extents`).

**Wired.** `WO_ROCKET_EXPLO` joins `oag_hd::effects::EXTENTS`. Every emitter of the craft blast
now spawns as its record says: the root, `SMOKE_BLUCLD` and both fireballs on a 2.255
hemisphere (mode 1, spread 0.667), `SHIP_DEBRIS` 3.235, `SPARKS` 2.059, `MOLTEN_DEBRIS` 8.824,
`FIRESPIKES` 0.1, `trail` and the smoke trails on their rings. The extent co-factor is 1 (no
driver writes one for this effect; the onsets agree with 1). Tests:
`near_law::hds_craft_blast_plays_the_screen_wash_templates` checks the heads' sphere (disc), and
`hd_effect_sprites::the_extent_law_is_lifted_only_for_the_effects_each_title_lists` the list.

**The pairs**, on their current flags, rendered twice byte-identical:

| Pair | Before | After |
| --- | --- | --- |
| `rocket-craft-hit-0233` | 50.36 | 64.14 |
| `rocket-craft-hit-0100` | 53.96 | 54.38 |
| `rocket-craft-far-hit-0100` | 12.91 | 11.89 |
| `rocket-craft-far-hit-0233` | 45.99 | 43.18 |

Over `--stage-seed 1..8`, paired: 0233 mean grey 62.68 to 54.74 (-7.94 +/- 8.39, closer on 6
of 8), RMSE -7.72 +/- 8.14; 0100 +0.54 +/- 0.63 (3 of 8 closer). 0233's own seed is worse: its
50.36 was the second-lowest of its nine seeds, and its single-point pose stands the blast 22.7
units out with our opponent's velocity, which carried the cloud into the eye ("Posing the near
hits"). Now the craft stands in plain view where the original's does, without the original's
yellow (the pose, above). The faithful two-wave pose ("The volley posed at its two points")
goes from 90.03 to 73.23 on its own seed.

**2048 and Omega.** 2048's own `WO_ROCKET_EXPLO` (`BANG_GLOW`, `DEBRIS_NO_TRAILS`, `SPARKS` and
the root) authors no trail heads: checked, differs. Omega's two copies (`particles2048` and
`particles`) both author HD's `FIRE_TRAILS_ONEQUAD` at 8.824: checked, applies, not wired (no
capture of either's hit).

## Per effect: what fills the heavy frames

2026-11-26, `blast-fill-split`, RX 7800 XT, RADV, 3840x2160. **The heavy frames of the
reproduction are the disc's own effects at the disc's own sizes and counts. Nothing found in
them is an invention of ours that would shrink the fill, so nothing was changed.**

The brief's ticks 726 and 756 are `frame_us` peaks. The GPU work is two ticks earlier, at 724
and 754, and at 784 for the third ("On the Steam Deck" above: scene pass 73, 69 and 58 ms at
those ticks, 17 ms at tick 760). At 724 and 754 the volley fired 30 ticks earlier has just hit
the track wall in front of the camera, and the next one has just left it.

The run is the reproduction above with only the HD render profile in the settings file, state
hash `62b06c2ee353b11d` at tick 800 on every run. Both probes were throwaway patches, not kept:
a counter of each quad's on-screen area clipped to the frustum (the probe of "Where the time
goes", keyed by effect, emitter and instance), and a switch that left one effect, the lights or
the screen flash out. The probe does not move the state hash.

### Fill, in screens

Quads in brackets. Emitters under 0.3 screens at every tick are left out; the totals hold them.

| Effect and emitter | Draw | 724 | 726 | 754 | 756 | 784 |
| --- | --- | --- | --- | --- | --- | --- |
| `WO_ROCKET_EXPLO_TRACK` `FIRESPIKES` | additive streak | 7.77 (106) | 6.23 (80) | 7.34 (88) | 5.43 (55) | 3.17 (30) |
| `WO_ROCKET_EXPLO` `trail` | alpha-over | 0 | 0 | 0 | 0 | 4.62 (100) |
| `WO_ROCKET_FLARE` `launch_smoke` | alpha-over | 2.20 (24) | 0.00 (1) | 1.74 (24) | 0 | 2.14 (24) |
| `WO_ROCKET_EXPLO` `FIREBALL_TUMULT_SMOKE_TRAIL` | alpha-over | 0 | 0 | 0 | 0 | 1.03 (4) |
| `WO_ROCKET_FLARE` `launch_flare` | additive streak | 0.71 (18) | 0 | 0.76 (18) | 0 | 0.90 (20) |
| `WO_ROCKET_EXPLO_TRACK` root | alpha-over | 0.78 (30) | 0.76 (30) | 0.79 (30) | 0.84 (30) | 0.33 (10) |
| `WO_ROCKET_FLARE` `WO_ROCKET_FIRE` | additive streak | 0.42 (39) | 0.48 (49) | 0.40 (34) | 0.36 (45) | 0.48 (31) |
| `WO_ROCKET_EXPLO` `FIREBALL_FIJETS_SMOKE_TRAIL` | alpha-over | 0 | 0 | 0 | 0 | 0.46 (4) |
| `WO_ROCKET_EXPLO` `SMOKE_BLUCLD` | alpha-over | 0 | 0 | 0 | 0 | 0.46 (1) |
| **All particles** | | 13.18 | 8.98 | 12.72 | 8.55 | 14.48 |
| of which additive | | 9.88 | 7.73 | 9.75 | 7.13 | 5.22 |

- **Tick 784's 5.2 additive screens are named**: the track blast's `FIRESPIKES` (3.17) and the
  player's own launch (`launch_flare`, `WO_ROCKET_FIRE`, 1.38). No sprite template is on
  screen at 784, and the streaks draw the procedural profile, which is why the sprite trim of
  "Measured and dropped" left that layer untouched.
- **The spikes are fresh.** At 724, 754 and 784 every `FIRESPIKES` quad belongs to a blast 4 to
  10 ticks old: one blast a rocket, three a volley. No older instance re-emits them.

### GPU time, by leaving one thing out

`gpu_scene_us` in milliseconds, twelve arms interleaved over six rounds, one-minute load 10 to
30, under the gate lock. The card runs in two clock states about 1.6 times apart, so each cell
is the median of the rounds within 12 % of that arm's best (two to six of the six rounds).

| | 724 | 726 | 754 | 756 | 784 |
| --- | --- | --- | --- | --- | --- |
| Scene pass, nothing left out | 3.05 | 2.01 | 2.96 | 1.91 | 2.88 |
| Scene pass, no particles | 0.70 | 0.71 | 0.75 | 0.74 | 0.78 |
| **Saved: all particles** | **2.34** | **1.29** | **2.21** | **1.16** | **2.10** |
| Saved: `WO_ROCKET_EXPLO_TRACK` | 1.29 | 1.05 | 1.20 | 0.95 | 0.57 |
| of which `FIRESPIKES` alone | 1.10 | 0.79 | 0.96 | 0.68 | 0.42 |
| Saved: `WO_ROCKET_FLARE` | 0.86 | 0.15 | 0.85 | 0.14 | 0.92 |
| of which `launch_smoke` alone | 0.47 | 0.02 | 0.40 | 0.02 | 0.39 |
| Saved: `WO_ROCKET_EXPLO` | 0.02 | 0.02 | 0.07 | 0.02 | 0.42 |
| Saved: HD's SPU point lights | 0.01 | 0.02 | 0.07 | 0.01 | 0.05 |
| Saved: the screen flash | 0.00 | -0.04 | 0.00 | -0.02 | 0.02 |

The unpatched build, run in the same rounds, reads 3.04, 2.02, 2.97, 1.92 and 2.85. Two builds
agree within 0.03 ms, so treat anything under about 0.1 ms as noise: the lights and the flash
cost nothing measurable at these ticks.

### Authored or ours

| Heavy effect | Verdict | Why |
| --- | --- | --- |
| `FIRESPIKES` (track blast) | the disc's | Counts, sizes and lifetimes are the file's, one blast a rocket. HD reads no fade and no size cap on this render mode (5) ([particle-render.md](../ghidra/functions/ps3-hdfury-eu/particle-render.md), "The near-camera law"). Ours draws it with the procedural strip rather than HD's bar ([engine-trail.md](../ghidra/functions/ps3-hdfury-eu/engine-trail.md), "The class-6 draw law"); with that bar, its sprite and its near cull switched on for the probe, the same quads cover 8.44, 6.47, 7.27, 5.22 and 3.22 screens, -4 to +9 % |
| `launch_smoke`, `launch_flare`, `WO_ROCKET_FIRE` (own launch) | the disc's | The billboards already run the near law with the file's cap; the streaks are render mode 5 in the file, like the spikes |
| `trail` and the craft blast's smoke (784) | the disc's | Under the near law with the file's cap (0.5 on `trail`). At a near craft blast the original's lists cover more than ours, not less ("Against the original at a near blast") |

What this does not settle:

- **No original frame holds a `.pob` streak quad** (the live dumps listed screen-facing quads
  only), so the verdict on the spikes rests on the file's data and the executable's builder
  (confidence 65), not on a picture.
- The blast's severity of 1.0 is **chosen, not measured**: nothing on HD's rocket path has been
  read feeding it.
- Ours caps an instance at 256 particles, and every track blast older than a few ticks sits at
  or near that cap, almost all `TRAIL_SMOKE` (tiny quads, 0.1 to 0.2 screens a tick). A cap can
  only make ours draw less.

**Cross-title:** HD only. A measurement; no format, law or asset finding to check on 2048 or
Omega.

## What is left

The Deck is measured ("On the Steam Deck"). The scene-width throttle is wired as TARGET FPS `original` and
measured on the workstation
([dynamic-resolution.md](dynamic-resolution.md#measured-the-rocket-hit-at-3840x2160)) and on the Deck, where it
does not hold the barrage and `default` does better
([dynamic-resolution.md](dynamic-resolution.md#on-the-steam-deck-target-fps-original-against-the-budget-loop)).
The close-range picture differs for the four reasons above. The ranked plan for the other two ideas is in the handover
thread.
