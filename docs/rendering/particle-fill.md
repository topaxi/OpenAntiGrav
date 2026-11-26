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
explosions load no `.gtf` (no `sprite(s) read` on their line of the loader report) and draw the
procedural profile, so the flip could not reach them.

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
  before. The mechanism is HD's per-particle size multiplier, which `trail` authors as
  `0.2` to `1` (`particle-render.md`, "The per-particle size multiplier"); not wired.

### Posing a far hit

`--force-rocket-hit TICK:SLOT` moves the craft onto the Rocket in flight, so the tick sets where
the blast stands: the 0.233 s film's hit is `--force-rocket-hit 717:7` after the shot at 709
(spawn 1.1 units from the original's wash quad; 716 is 4.0 short). The 12-unit pairs use 711,
which was not checked against the original's spawn point. The ring's wash quad is a few ticks
younger than the film's labelled age (2 ticks at 0.1 s, 10 at 0.233 s, read off its colour and
size), so the far pairs render ours at the quad's age. Ours' struck craft keeps our own opponent's
velocity, so the travelling fireballs do not match.

## What is left

The Deck is measured ("On the Steam Deck"). The scene-width throttle is wired as TARGET FPS `original` and
measured on the workstation
([dynamic-resolution.md](dynamic-resolution.md#measured-the-rocket-hit-at-3840x2160)). The close-range picture
differs for the four reasons above. The ranked plan for the other two ideas is in the handover
thread.
