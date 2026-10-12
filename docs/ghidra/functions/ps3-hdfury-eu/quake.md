# Wipeout HD / Fury: the Quake - fired by state 2, a wave of fire and a lifted road

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), PowerPC, big endian. 2026-10-12, lane
`hd-quake-render`. Read with the Ghidra bridge, `scripts/ps3-toc.py` (every function in this
module takes TOC `0x008ad4d8`, so the decompiler's `PTR_*` loads are right), live memory on RPCS3
and a film of a fired Quake. Pulse's Quake is
[`../psp-pulse-usa/cannon-quake-leachbeam.md`](../psp-pulse-usa/cannon-quake-leachbeam.md); the
HD module is the same machine with a second way to move the road.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x00147d18` | `QuakeManager_Fire` | 85 | Clears slot `+0x200` bit `0x8`, writes the spent state `-1` to slot `+0x204`, arms the Quake object at `manager + 0x6c` (`+0x40 = 1`, `+0x44` the firing craft, `+0x48` a fire counter), calls `Quake_Init` and a cue (`0x0012d8c8(2)`). State 2's handler sets that bit. Live: state 2 written, triangle pressed, a wave leaves. |
| `0x001560b0` | `Quake_Init` | 80 | Locates the craft on the track (`0x002d10f0` returns the `AI track data` resource), arms the launch span through `0x002d44c8` with a lead of `+-15.0` (`0x41700000`, the sign from the craft's heading), and sets the busy byte `quake + 0x4c`. The Pulse `Quake_Init`'s shape. |
| `0x002d2a78` | `Quake_Update` | 75 | Per frame: age `+0x140 += dt`, the wave's `t` `+0x148` advanced by `dt * (+-270) / segment`, the two edge points from `Quake_SampleSpan`, then the first-frame `Psys` spawn of `WO_QUAKE` and `~QUAKETRAVEL` (both strings attribute to it through `ps3-toc.py attrib`). The decompile truncates at the AltiVec blocks (`halt_baddata`), so the matrix build is read from the disassembly only in part. |
| `0x002d2630` | `Quake_SampleSpan` | 75 | `(span, &A, &B, &scale, &valid)`: two points across the road at the wave's `t`, the amplitude into `+0x380`. Piecewise: `age * 40` below `0.3`, then `12 - 12 (age - 0.3) / (5 - 0.3)` (constants `0x008b3c48/4c/50`: `0.3`, `12.0`, `40.0`). |
| `0x002d3760` | `Quake_UpdateSpan` | 70 | Walks a span record's packed `short3` vertices and adds or subtracts a displacement: Pulse's `Quake_UpdateSpan` on the same `0x80`-byte record (`+0x3a` vertex stride, `+0x3c` parameter stride, `+0x50` count). The constants are Pulse's (`12.0`, `0.3`, `50.0`, `25.0`: `0x008b3c4c/48/54/58`). |
| `0x00ae98c0` | `g_quake` (data) | 85 | The singleton `0x008b3c38` points at; layout below, measured live. |

`0x002d44c8` (`ArmSpan`), `0x002d10f0` (the track-data accessor) and the other fifteen functions
of `0x002d0f90..0x002d49dc` (`Quake_Importer.cpp` at `0x007a11d0` attributes `0x002d1178`,
`0x002d11c0`, `0x002d14e8`, `0x002d1840`, `0x002d1f98`) are read in outline and not renamed.

## Which held weapon is the Quake: state 2

`Weapon_FireFromSlot` (`0x0012d970`) indexes sixteen handlers (table `0x0012d9b8`). State 2's
(`0x0012daa0`) sets slot `+0x200` bit `0x8` and calls `0x00129048(2)`; `QuakeManager_Fire` is the
consumer of exactly that bit. Confidence 90, plus live: the Quake of `rpcs3-capture.md`'s
"state 2, unresolved" row (a burst `3.4 s` after the shot, filling half the frame) is this wave.

## The state, measured live: `g_quake` at `0x00ae98c0`

`scripts/rpcs3-hd-quake-poll.py`: `/proc/<pid>/mem` at about 100 Hz, no pause, a Quake fired from
the grid. Confidence 85 (one boot, 1,259 samples; every field below follows the same law the
Pulse page recovered).

| Offset | Value | Reading |
| ---: | --- | --- |
| `+0x70` | `5.0` | lifetime |
| `+0x140` | `0.0141 .. 5` | age, game seconds (the HUD clock's, not the host's) |
| `+0x148` | `0.4504 +` | the wave's `t` on its path; `0.45041 -> 0.51023` over `1.1234 s` is `270 u/s` over a `5,071`-unit path |
| `+0x150` | `270.0` | signed speed - **Pulse's `270.0` exactly**, not authored per class |
| `+0x78` | `1` | spans in flight (`0..2`) |
| `+0x380` | `0.566, 11.965, 9.898` at ages `0.0141, 0.3138, 1.1234` | amplitude: `40 * age` to `0.3 s`, then `12 - 12 (age - 0.3) / 4.7`; every sampled value is within `0.005` of it |

## The road: a vertex program, and the registers it reads

Wipeout HD builds a `Static` and a `StaticQuake` permutation of a material's vertex program (the
class is the `[0]` half of the variant key, `docs/formats/rcsmaterial.md`). The two differ by a
displacement in front of the transform. Decoded with `scripts/ps3-microcode.py vp-file` from
`talons_junction/materials/track_surface.rcsmaterial`, vertex block 40 (confidence 85; every
instruction read, the registers' meaning confirmed live):

```text
P   = pos * c[210] + c[211]                 world position (positionScale, positionBias)
M   = (A + B) / 2                            A = c[208] quakePointA, B = c[207] quakePointB
d   = |(A - P) x c[209].xyz| / c[209].w      distance from the line AB   (quakeOffset)
lift = B.w * cos(min(d / A.w, pi/2)) * sqrt(saturate(1 - |M - P| / A.w))
P'  = P + c[206].xyz * lift                  quakeTrackUpNormal
```

`d` is `P`'s distance from the line through the two edge points, which runs across the road, so the
bump is a quarter cosine along the track, and the second factor clips it to a disc of radius `A.w`
round the midpoint. The normal is not touched.

**What the registers hold** (`scripts/rpcs3-hd-whiteout.py --quake --no-targets` with
`HOOK_RING_ONLY=1`, `scripts/hd-quake-uniforms.py`; two boots' frames, confidence 85):

| Register | Reading | Live values |
| --- | --- | --- |
| `c[207]` `quakePointB` | `xyz` a track-edge point at the wave, `w` the amplitude | `(251.09, -49.27, -162.18, 9.561)` and, a second frame, `(459.09, -32.61, -48.17, 10.772)` |
| `c[208]` `quakePointA` | `xyz` the other edge point, `w` the radius | `(252.89, -42.47, -201.26, 60.0)`; `A.w` was `60.0` in every frame (two moments of two waves), **constant** |
| `c[209]` `quakeOffset` | `xyz = B - A`, `w = |B - A|` | `(-1.80, -6.80, 39.08, 39.71)`: the road's width, `39.71`, equals `|B - A|` |
| `c[206]` `quakeTrackUpNormal` | the road's up at the wave | `(-0.044, 0.977, 0.207)` on a banked stretch, `(-0.069, 0.997, 0.047)` further on |

`B.w` is `g_quake + 0x380`, the amplitude above: in the second frame the eye (`x = 274.0`, the
camera, `c[465]` of the other programs) was `185` units from `B`, which at `270 u/s` from a `15`-unit
lead is age `0.63 s` and `amplitude(0.63) = 11.2` against `10.772` read; the craft had moved from
where the wave was fired, so the two agree to the capture's accuracy and no better. The two `StaticQuake` groups of one frame differ by `6.8` units in `B` and `0.064` in
`B.w`, about one and a half ticks: the registers are set per span, a span a frame ahead of its
neighbour, as Pulse's ripple page records.

**Which draws run it**: `scripts/hd-quake-vp-class.py` renders every vertex program a dumped
frame uploads and calls the one with a `COS` a `StaticQuake`. A frame with no wave: none. With the
wave 400 units from the eye, 8 of 596 scene-target draws; 190 units away, **50 of 596**, ten distinct
programs. Matching those programs' bytes against Talon's Junction's materials finds `track_wall`,
`track_surface`, the pad and glass families: walls and road both, not the road alone. The 50
draws of the second frame are **50 distinct chunks** (distinct index-buffer offset and batch), each
drawn once. Of the
disc's 1,632 materials, **381 ship a `StaticQuake` row** (`oag_rcs::rcsmaterial::Class`: 377 beside a
`Static` row, 4 alone); on Talon's Junction **281 of this project's 2,211 draws** are of such a
material. **Which chunks the original draws under that class is not established**: none in a frame
dumped right after the shot, 8 with the wave far, 50 with it near, so it is not "every flagged chunk
while a Quake is up"; the draw units differ (this project's `DrawCall` against the original's chunk
draws) and the flagged draws within a radius `R` of the second frame's midpoint, bounds included, are
`15, 21, 22, 22, 27, 30, 36, 53` at `R = 40, 60, 80, 100, 120, 150, 200, 300`, so no radius reproduces
50 cleanly. Ours lifts every vertex of a flagged material inside `A.w = 60` of the midpoint, which
is the program's own reach, and may lift chunks the original does not issue as `StaticQuake`.

## What a player sees, from the original (RPCS3, Talon's Junction, a craft at rest)

`scripts/rpcs3-hd-quake.py`: state 2 written, triangle pressed after the `GO`, filmed at 30 fps;
frames read at the HUD clock, the wave leaving a craft that stands on the grid.

- **At the shot (race clock `7.2`)**: a yellow-white wash across the whole picture with orange
  streaks, the green pipes turned lime and the hull orange. A fire band across the **full road
  width**, wall to wall, close to the craft.
- **`7.3` to `8.0`**: the band shrinks with distance down the straight and stays as wide as the
  road: an orange-yellow line of fire, sparks and debris, with a dusty smear behind it. It reaches
  the far gantry about `0.5 s` after the shot; at `270 u/s` that is the measured speed.
- **A wave that overtakes the craft** (`--ahead 110`: the craft moved forward after the shot, the
  wave arrives `0.4 s` later): the scene goes yellow and white for about a third of a second, the
  walls and pipes lean, and the camera rises and falls as the road under it does.

## What this project draws now

- **The fire** - `WO_QUAKE`, HD's own file (`DATA02.PSARC:/data/psys/wo_quake.pob`, five emitters:
  `WO_QUAKE`, `molten_debris_colliding`, `debris`, `FIRE_STREAKS`, `SPARKS`), is played with the
  **emitter extent law** Pulse's line emitter has (`oag_title::Effects::extents`, HD lists
  `WO_QUAKE`) and its own five `.gtf` sprites (`Effects::sprites`). Before, it was one offset
  blob of grey puffs. **Inherited from Pulse, matched to the film by eye**: a band as wide as the
  road, as bright, at the same speed; HD's own `ParticleSystem_EmitLine` is not read, so the law is
  not a measurement of HD. No confidence score.
- **The road** - `GpuVertex::quake`, `Scene::quake` and `crates/mesh/shaders/quake.wesl`: the
  program above, for the surfaces of materials that ship a `StaticQuake` row, fed the two edge
  points the particle wave already rides, the track's up there, `oag_render::ripple::amplitude(age)`
  and `A.w = 60`. Ground truth: `crates/render/tests/hd_quake_lift.rs` (the GPU against the
  program's own arithmetic, one marked and one unmarked triangle, amplitude zero the identity) and
  `crates/game/tests/quake_orientation_ground_truth.rs` (the flag on Talon's Junction's draws, and on
  none of Pulse's).
- **Pulse, Pure and the PS2 are pixel-identical** to `9d95d3936` with a Quake in flight (AE `0` at
  six, four and two ticks), and HD with no wave is too: nothing but a `StaticQuake` material
  carries the flag.

## Open

- **The top item: which flagged chunks the original lifts.** The dump above shows a selection by
  something this page has not recovered (span membership is the likely rule: the table's batches are
  the ones `Quake_UpdateSpan` walks). If it is narrower than "flagged material within `A.w`", ours
  over-lifts (walls and pipes the original leaves still).
- **The stationary-craft pair does not show the lift matching.** At `7.3` the film's walls and road
  look straight where ours bends them; the evidence that the lift is real and large is the decoded
  program, the live registers and the overtaking film (`--ahead 110`), not that pair, which is too
  washed out beside the band to judge by eye.
- **The launch white-out and the scene-wide yellow tint.** Ours has the fire and no wash. The tint
  tints the HUD's translucent panels and the hull as well, so it is not a mesh effect; none of the
  twelve callers of `SpuLight_AddCandidate` (`0x006778c8`) is in this module, and `Quake_Update`'s
  callees (twenty-four, listed from its `bl` sites) include no flash. Not found.
- **The pale flat plane** ours draws on a wall in the first ticks is the `molten_debris_colliding`
  emitter's own sprite (`molten_debris_64x64.gtf`): with only that emitter's sprite kept it is the
  whole of it, and with it procedural it is gone. A "colliding" emitter's particles may lie on the
  surface they hit (flat decals) where ours billboards them; the film shows pale translucent planes
  of a similar kind on the walls at `7.3` to `7.7`, so it is unjudged, not wrong.
- **`Quake_UpdateSpan` (`0x002d3760`) on HD** rewrites packed `short3` positions on the CPU as
  Pulse's does, and every HD circuit authors a `Quake` node (class `0x3c7`, `Quake_Importer.cpp`)
  whose span table `oag_vex::quake::spans` cannot read: it is big-endian (`quake span records ends
  at 57985050080 but the file is 3066400 bytes` on Talon's Junction). Whether that CPU write also
  reaches what the RSX draws, in addition to the shader, is unmeasured.
- **The flipbooks.** All five sprites are drawn at frame 0; HD's frame advance is unread
  (`oag_title::Effects::sprites`).
- **The launch lead.** `Quake_Init` arms the spans `15.0` ahead; the shader's points are placed at
  the wave itself here, which a matched pair at the very first ticks has not tested.
- **The same ship and circuit.** The original film is the Fury campaign's first event (a dark delta
  hull no team here matches); ours is the default Assegai. The wave, not the craft, is compared.

## Cross-title

- **Omega: checked, applies, not wired.** `eboot.bin` names `quakePointA`, `quakePointB`,
  `quakeOffset` and `quakeTrackUpNormal` (`0x0182a592..0x0182a5b6`) and `WO_QUAKE.POB`
  (`0x01816d1c`); `data00.psarc` ships `Data/particles/WO_QUAKE.pob` (5.6 MB, a different file) and
  `quake_shockwaves_512x512_ALPHA*.gnf`. Its materials are PS4 shaders `ps3-microcode.py` cannot read
  and no PS4 emulator is in the toolchain, so the program itself is **not checkable**; `oag-omega`
  lists no `Effects::extents`.
- **2048: checked, applies, not wired.** The Vita executable names `wo.VBE.StaticQuake` 547 times
  (one per material variant), the same vertex class under another prefix.
- **Pulse and Pure**: unchanged; Pulse moves its road on the CPU
  (`oag_render::ripple`), Pure has no road lift.
