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
   ([rocket-craft-blast.md](../ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md), "The
   screen wash: narrowed, not found").
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

## What is left

Neither the Deck nor the scene-width throttle has been measured. The close-range picture
differs for the four reasons above. The ranked plan for the other two ideas is in the handover
thread.
