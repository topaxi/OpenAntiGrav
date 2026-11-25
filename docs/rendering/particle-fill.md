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
additive, alpha-over and underlay lists before upload, when it cannot change a pixel:

- all six vertices have colour alpha 0, under either blend;
- all six vertices have colour 0, under the additive blend only (black smoke with alpha still
  darkens under the over blend).

`psys.wesl` multiplies its output by the interpolated vertex colour and alpha, and the pipelines
write neither depth nor velocity. A dropped quad therefore left the target exactly as it was.
Order is kept, because the over blend is not commutative.

Before and after, same binary otherwise, A and B runs interleaved, window ticks 760 to 789:

| Size | Load | GPU scene mean before | after | GPU scene max before | after |
| --- | --- | --- | --- | --- | --- |
| 3840x2160 | 39 to 47 | 3.97, 4.01, 3.98 ms | 3.72, 3.72, 3.72 ms | 11.20, 11.20, 11.15 ms | 9.50, 9.47, 9.48 ms |
| 1920x1080 | 48 to 58 | 1.62, 1.62 ms | 1.55, 1.55 ms | 3.56, 3.50 ms | 3.17, 3.20 ms |

That is -6 % over the window and -15 % at the worst frame (4K). The card ran slower in this
session than in the skip table above (other members' gates, load 40 to 50), but each run of a
pair sat within 0.05 ms of the others. The CPU columns were too loaded to read.

**Identity:** screenshots at ticks 768, 774 and 777, at 1920x1080 and 3840x2160, are
byte-identical before and after (`cmp`). So are Pulse's at ticks 700, 720 and 740 after its own
craft hits (gamma target, sampled-sprite path, `pulse-psp-eu.chd`, same script). The state hash
is unchanged.

## Measured and dropped

- **A fragment `discard` where the particle's output alpha is exactly 0** (the corners outside
  the procedural profile's circle, a sprite's clear texels). Byte-identical, but against the
  cull alone at 4K it read 3.66, 3.65 and 3.65 ms against 3.71, 3.72 and 3.62 ms (load 79 to 95).
  That is inside the spread, so it was not kept.

## What is left

The live overdraw is authored: the CraftBlast smoke's own sizes, counts and colour tables
(`Trigger::CraftBlast`). Particles stay as the disc authors them. What could still cut it
changes the picture, or needs a measurement first. These are listed in the handover thread.
