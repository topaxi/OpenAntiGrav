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

## Measured and dropped

- **A fragment `discard` where the particle's output alpha is exactly 0** (the corners outside
  the procedural profile's circle, a sprite's clear texels). Byte-identical, but against the
  cull alone at 4K it read 3.66, 3.65 and 3.65 ms against 3.71, 3.72 and 3.62 ms (load 79 to 95).
  At that load, with the GPU shared with other members' gate tests, it did not show beyond
  noise. The result is inconclusive, not a no. It is worth re-measuring on a bandwidth-bound GPU
  (the Deck at 4K).

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
| Near particles | cull, fade, cap | none | **ours draws more** |
| Particles behind the camera | not built | built, uploaded, clipped | ours wastes vertices, not fill |
| Resolution | 1.4 to 2.07 megapixels, dynamic width | the output's, fixed (`render_scale = 100`, `target_fps = "off"`) | the player's choice |

Counts, sizes and lifetimes are the disc's in both: ours plays the same records, and no
difference was found there. The law above is the difference.

## What is left

The near-camera law is recovered and not wired. The ranked plan is in the handover thread.
