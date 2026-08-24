# The engine trail: `TrailEffectManager`, read live

Read 2026-08-24, three ways at once: the executable in Ghidra, the running
game's own memory over RPCS3's GDB stub
([`scripts/rpcs3-trail-dump.py`](../../../../scripts/rpcs3-trail-dump.py) is
the reproducer), and the disc's own tuning file
`Data/ships/shipeffectstweaks.txt`, which turned out to name the runtime's
whole parameter block field for field. This page is the geometry and animation
of HD's exhaust - the ribbon's shape, colour, alpha and scroll, the flame's
breathing, and the boost plume's reveal.
[engine-flare.md](engine-flare.md) stays the shader-microcode record and
[trail-ribbon.md](../../../rendering/trail-ribbon.md) the cross-title asset
survey; both now point here for what used to be "unread".

`oag_render::exhaust::hd` implements what this page measures.

## The manager, in the executable

`Trail_ConstructManager` (`0x002e2da8`, and a second ctor at `0x002e2f40`)
stores itself in the global at `0x00AED460 + 0x6c` (TOC slot `0x008b4434`),
allocates `0x11b00` bytes into `this + 0x40`, and initialises eight per-craft
slots via `Trail_InitCraftSlot` (`0x002e21c0`) - each `0x1230` bytes at
`alloc + 0x84a0 + slot * 0x1230`, matching the eight teardown calls in the
destructor at `0x002e4888`. Vtable (`0x0086a940`), the slots that matter:

| Slot | Function | What it does |
| --- | --- | --- |
| 5 | `Trail_Enqueue` (`0x002e2610`) | enqueues the manager into the render queue with a `0x4d1`-layer / 20-bit-depth sort key - the same 12-over-20 key shape Pulse's `Gfx_Enqueue` uses |
| 7 | `Trail_RenderTick` (`0x002e4748`) | flips the double-buffer index at `+0x119f0`, then `Trail_BuildDrawState` (`0x002e30d8`) and `Trail_WriteCraftContexts` (`0x002e3bf8`) |

`Trail_BuildDrawState` binds the material's constant table - the same
0x20-byte-entry layout the `.rcsmodel` material record uses - and per craft
patches one vec4 to all-0.0 or all-1.0 from the craft byte at `+0x7d2c` (see
"the red trail" below). `Trail_WriteCraftContexts` writes, per active craft, a
`0x60`-byte context at `alloc + 0x116f0 + n * 0x60`: the craft's current
matrix (rows carrying the familiar global `0.75` scale), a bounding sphere,
the craft index and the `+0x7d2c` flag widened to a lane mask - plus six
world-space frustum planes built from `fov * pi/180`, aspect `16/9`, near
`0.1`, far `10000`. The extrusion itself runs in the `Trails` SPU job
(`0x4d40` bytes embedded at `0x00811680`, registered by `0x002e2a48`); the
PPU never touches a vertex.

**Confidence 88** on the roles above: every named function was decompiled,
and the structure was then confirmed against the live dumps below rather than
against the decompiler alone.

## The per-trail block, from the live game

Everything below is measured from `alloc + 0x84a0 + slot * 0x1230` in a
running Fury race (RPCS3, three sessions, eight craft, two frames each), not
inferred from code.

**Bytes 0..0x10e0 are the history ring: 54 samples of 0x50 bytes** - three
orientation basis rows (the craft's, normalised), the position, and a colour
that is `(1,1,1,1)` on every sample of every craft. One sample is pushed per
rendered frame. The rest:

| Offset | What it is | How it is known |
| --- | ---: | --- |
| `+0x10e0..0x1140` | six frustum planes | rows match the fov/aspect build above |
| `+0x1140..0x1180` | the live head transform | equals the craft's current basis and nozzle |
| `+0x11d4` | this frame's output buffer | flips between the two RSX addresses below |
| `+0x11d8` | **the trail's brightness** | written by `EngineFlare_Update` as `engine blend x speed ramp`; the dumped vertex alpha peak is exactly `brightness * (1 - 6/54)` |
| `+0x11e4` | a per-frame global (dt-scaled) | equal across all eight trails, varies per frame |
| `+0x11ec` | **the head `u`** | `1 - 0.6 * speed01`; equals the head vertex's `u` to 4 decimals on three craft in two frames |
| `+0x1210` | **the scroll phase** | `EngineFlare_PlaceShapes`' wrapping accumulator, patched into the material's `TrailSpeed` |
| `+0x1204` | the craft | chases to the `EngineFlare` at `craft + 0x5f70` |
| `+0x1214/+0x1218` | the double-buffered SPU output | `0xC2B21800`-style RSX addresses; `+0x121c/+0x1220` the same as IO offsets |

## The extruded geometry, from the output buffers

The output buffer is `0x2d90` bytes - the size the ctor registers - and the
manager's own attribute table at `alloc + 0x11a00` declares the layout:
`position` f32x3 at 0, `normal` f32x3 at 0xc, `Uv1` f32x2 at 0x18,
`VertexColour1` u8x4 at 0x20, **stride 0x24**. So 324 vertices; the shared
index buffer (`0x780` bytes at `alloc+0x119f4`'s copy) covers the draw call's
**954 indices** - and 954 is exactly `53 segments x 3 faces x 2 triangles x 3`.

**The ribbon is three flat fins through the trail line**, not a closed tube
and not camera-facing: at every ring, each fin's two edge vertices sit
diametrically opposite at exactly `0.5` world units from the ring centre -
full width `1.0`, no taper anywhere - with the fins at **0, 60 and 120
degrees from the sample's up axis**, rotating toward `up x back`. As *lines*
that is the same set as "up and +-60", but the fin *directions* decide the
single-sided normals and those decide everything under the facing fade: the
dumped normals are `cross(back, fin_dir)` with `back` the older-minus-newer
tangent - `(0, 0.87, -0.5)` and `(0, 0.87, +0.5)` for the two off-vertical
fins, **both tilted toward up** - so a camera anywhere above the trail keeps
two fins lit. Reconstructed the mirror-naive way (`+-60`,
`cross(forward, fin)`) one fin faces fully away from a chase camera and the
trail all but vanishes dead-astern, which is how this was caught: the
first implementation drew exactly that. Normals are left unnormalised where
samples bunch. The buffer is face-major: fin 0's 108 vertices, then fin 1's,
then fin 2's.

Per-ring vertex attributes, fitted exactly against six buffers (three craft,
two frames, speeds from near-rest to race pace):

- **`u(k) = u_head * (1 - k * 4/54) + phase`** - the constant `4/54` fits at
  `0.0741` on all six buffers; `u_head` is the `+0x11ec` value
  (`1 - 0.6 * speed01`), so the streaks stretch with speed; `phase` is
  `+0x1210`. `v` runs 0 to 1 across each fin.
- **colour: white to red over the first 27 rings** - green and blue fall
  linearly `255 -> 0` at ring 27 and stay 0; red never moves.
- **alpha = brightness x attack x falloff** with `attack = min(k * 10/54, 1)`
  (a 5.4-ring ramp from the nozzle) and `falloff = 1 - k/54` (linear to
  nothing at the tail). At brightness `0.717` the measured peak is `162/255`,
  which is `0.717 * (1 - 6/54)` to the byte.

**Confidence 92** on all of the above: the numbers are the running game's
own, cross-checked between sessions, and the fin arithmetic (`954`, `0x2d90`)
closes twice over from independent constants.

## The tuning file is the parameter block

`Data/ships/shipeffectstweaks.txt` (41 lines, all
`"Ship Effects.Engine Trails.*"`) matches the runtime constants read from
`EngineFlare_Update` (`0x002a3100`) and `EngineFlare_PlaceShapes`
(`0x002a1f00`) one for one - `Thrust Chase Rate` 0.2 is the lag rate whose
`* (1 - 0.2)` per 120 Hz substep produced the measured `0.8^(2k)` blend
decay, `Spikes Random Scale Min/Max` 0.65/0.85 the per-shape flicker range,
`Tex Scroll Speed Delta Min/Max` 0.08/0.10 the per-call phase advance,
`Tex UScale Max` 0.6 the `u_head` coefficient, `Tex Scroll Speed Ship Range`
1000 and `Thrust Contrib` 0.25 the `speed01` law, `Thrust Min/Max Scale`
XY 1.0/1.5 and Z 0.25/1.5 the flame's throttle scales, and
`Thrust Extra Boost Scale` XY 1.4 / Z 2.0 the boost's additions.
`crates/game/tests/hd_engine_flare_ground_truth.rs` pins
`oag_render::exhaust::hd`'s constants against the file itself.

## The flame and the plume breathe; nothing gates the plume

`EngineFlare_Update` is Pulse's exhaust state machine wearing PS3 constants -
literally the same numbers, read at `0x008b2fe0..0x008b3018`: intensity rise
**0.25/s** and fall **0.5/s**, per-call decay **0.1**, speed ramp floor
**100 km/h** over a **500 km/h** span (`0.002`), floor share **0.6**, and the
boost gate `EngineFlare_BoostGate` (`0x008b2ff4`) = **0.2** - the PSP's
`BOOST_GATE` value exactly,
which raises the "matched mechanism" reading of the plume gate from 88 to
**92**. **The speed field `+0x104` is world speed times ~5.4, and a fourth
session read it directly** - all eight craft, two snapshots, alongside the
ramp at `+0x108` and the trail block's brightness:

- `brightness = ramp = (field - 100) / 500` holds to six decimals on every
  row (fields 248..592 gave ramps 0.296..0.983, each exact), and the engine
  blend sits at 1.0 throughout a driven race - so a craft at pace trails at
  or near full brightness, where Pulse's own ramp would still be climbing.
- `u_head = 1 - 0.6 * (field / 1000 + 0.25 * throttle)` back-solves to a
  throttle of exactly 1.0 on every thrusting craft.
- The field against each craft's own ring spacing (at the game's 1/60 step)
  is `world speed * 5.0..6.6`, median 5.3 - two readings of this page took
  the `* 3.6` in `EngineFlare_Update` for m/s-to-km/h and then for a gain on
  km/h; both are refuted by the direct read. The composition is the read
  3.6 times an unexplained ~1.5 already in `node[0x4c4]`; only the product
  is load-bearing, and `oag_render::exhaust::hd::SPEED_FIELD_GAIN` carries
  it as 1.5 over this engine's own km/h. `Exhaust::speed_ramp` stays
  Pulse's.

`EngineFlare_PlaceShapes` then scales rather than shows/hides:

- `EF_Main`: cross-section `lerp(1.0, 1.5, s) + b * 1.4`, length
  `lerp(0.25, 1.5, s) + b * 2.0`, where `s` is the smoothed throttle and `b`
  the boost blend (`+0x144`, snapped to 1.0 while the timer at `+0x12c` is
  above the 0.2 gate, decayed `* 0.8` per substep after, plus the slower
  afterburner blend at `+0x150`).
- `EF_Boost`: **length `b * 2.0`, no visibility branch at all** - the plume
  grows out of the nozzle and collapses back in ~10 frames rather than
  blinking. This corrects the picture the `Exhaust::plume_visible` wiring
  drew; `oag_render::exhaust::hd::Flame` is the replacement and
  `race::scene::frame::hd_flame_transform` applies it.
- The five spike shapes flicker at `RandRange(0.65, 0.85)` each, per frame -
  read, and **not implemented** (the scene draws the flare as two groups, not
  ten shapes; the load report says so).
- `EngineFlare_Update` also writes the trail's brightness
  (`+0x11d8` above) and the scroll phase/`u_head` pair - the flare object
  owns the trail's animation inputs.

## The red trail is the Fury skin

`Ship_SetFuryTrailFlag` (`0x000d0b28`, with twins at `0x000df7d8` and
`0x000e1814` in the other two craft-construction paths) sets the craft byte
`+0x7d2c` to 1 exactly when the
model-variant name at `*(craft+0x6298)+0x78` is - `strcasecmp`, so
case-blind - one of **`chrome_c1`, `nitro`, `detonator`, `concept1`**
(`0x00781e58..`). `Trail_BuildDrawState` turns that byte into the all-ones
vec4 it patches over the material's `0xbb48e390` colour-mix parameter, and
the fragment program lerps the blue texture's colour toward the red one's by
it. On the disc the variants live in the `*_c1` (concept) and `*_n1` (nitro)
ship directories; `chrome_c1` matches no shipped directory and reads as a
leftover. Confirmed live: all eight craft of a Fury-campaign event carry
`deref = "concept1"`, flag 1, and the reference frame's trail is orange-red.
Confidence 82: the mechanism and both consumers are read; what is not
followed is where `+0x6298` is assigned.

## What arms the boost timer stays open, minus two wrong answers

Polling all eight flares' `+0x12c` through a Fury race caught values up to
`0.70`, all exact multiples of `1/30`, decaying while the blend at `+0x144`
already ran its `0.8^(2k)` course - so the snap threshold sits *above* every
caught value, consistent with the 0.2 gate only revealing on larger arms
never caught in this session. Two candidate writers dissolved on reading:
`0x0008a5b8` and `0x00089fa8` write `+0x12c` of HUD sprite objects (a
rotation angle - the lock-on reticle and the race-progress strip), not of any
flare. The one confirmed writer, `0x00090d30`, mirrors a craft countdown at
`craft+0x108` (decaying at `2 * dt`) into the flare - gated on mode 10,
plausibly Zone Battle, where boost is banked. What arms `craft+0x108`
elsewhere - pads, barrel rolls, the start boost - is unread, so the boost
*duration* this engine uses remains its own number; the gate, the blends and
the scales above are not.

## What follows for the renderer, and what stays open

Implemented in `oag_render::exhaust::hd` (the tube and the flame blends),
`exhaust.wgsl` (facing fade, depth fade `saturate(window_z * 0.75)`, the
blue-red mix, the baked scroll) and `oag_game::race` (per-slot state, the
Fury flag from the team directory, the group scales). **The tube skips the
renderer's gamma decode on the linear scene target**: its 29-instruction
fragment program applies no transfer function anywhere, so the original adds
gamma-space samples into its linear target as they are, and decoding them
cut the trail's green and blue channels to a third. The sampler's sRGB-remap
state is the one unread bit of that claim. Divergences, stated:
samples push at the fixed 60 Hz tick rather than per rendered frame
([ADR-0007](../../../architecture/adr/0007-fixed-timestep-vs-original.md));
the ring is reconstructed from `{position, up}` rather than the full stored
basis; the per-call phase advance lands per tick.

Open, in rough order of visible cost:

- The spike flicker and the afterburner blend (`Afterburner Chase Rate`
  0.03, `Afterburner Scale` 0.5) - both read, neither drawn.
- The flame surface's `Speed * time` scroll: `time`'s provider is still
  unread ([engine-flare.md](engine-flare.md)); the ribbon's phase at
  `+0x1210` is a *different* accumulator and does not answer it.
- The `Engine_Flare_Rich.gtf` sprite flare: `Enable Flare Sprite` is 1 and
  the file authors its radius (3), fade distances (15/15), jitter and
  chromatic dispersion - but its shader (`engineflare_vp/fp`) is unresolved,
  so it stays undrawn and reported.
- What the SPU does with a part-empty ring at a race start; this engine
  reuses the PSP's full-ring gate.
- `+0x11e4`, the dt-scaled per-frame global every trail shares, unplaced.
- Whether the white-to-red vertex ramp differs on a classic (blue) HD grid -
  every craft in the dumped sessions was a Fury `concept1`.

## Reproducing this

```sh
uv run --with evdev python3 scripts/rpcs3-trail-dump.py /tmp/hd-trail-dump
python3 scripts/psarc.py cat data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC \
    /data/ships/shipeffectstweaks.txt
OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all hd_engine_flare
```
