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
destructor at `0x002e4888`. **Two different per-slot initialisers run, and
telling them apart matters below**: `Trail_InitCraftSlot` is the SPU job's
setup (the DMA lists that hand the slot's `0x1200` bytes to the `Trails` job),
called eight times from each ctor directly, while the buffers, the shared
index list and the material bindings come from one shared body,
`Trail_InitManagerBuffers` (`0x006b6870`), which both ctors also call and which
runs `Trail_ResetCraftSlotState` (`0x002e2188`) per slot. Vtable (`0x0086a940`), the slots that matter:

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
| `+0x11e4` | pointer to this trail's render-queue sort-key slot | the write is the last line of `Trail_BuildDrawState`'s per-craft loop: it reserves a 4-byte slot in the frame's command stream, writes the `depth << 20 \| 0x20000000`-shaped key into it, and stores the slot's address here. Dump-confirmed: eight values exactly `0x334` apart (one trail's full command packet), re-pointed every frame. An earlier reading called this "a dt-scaled per-frame global, equal across all eight" - both halves were the read-a-pointer-as-a-float trap again |
| `+0x1208` / `+0x120c` | **this trail's material-instance array, and its length** | `Trail_InitManagerBuffers` allocates `count << 2` bytes and fills it with one instance per material of the loaded ribbon model; `Trail_BuildDrawState` binds the array into the draw state at `+0xc4`. An earlier dump of 0x600 bytes at the target, over four trails and two frames, found no copy of the trail's `+0x1210` phase in either RSX word order - **correct, and now explained**: the instances hold a *pointer* to the phase at `entry+0x18`, never its value (see "the scroll phase is bound by pointer" below) |
| `+0x11e8` | how many craft contexts this frame holds | the last line of `Trail_WriteCraftContexts`' per-trail loop writes the running count into **every** trail's block, so each SPU job is told the length of the shared context array at `alloc + 0x116f0` as well as being handed it. Read live as `8` on all eight trails in six snapshots across three sessions. What the job does with the other seven craft is unread - see "does another craft disturb the ribbon" below, which rules out the obvious answer |
| `+0x11ec` | **the head `u`** | `1 - 0.6 * speed01`; equals the head vertex's `u` to 4 decimals on three craft in two frames |
| `+0x1210` | **the scroll phase** | `EngineFlare_PlaceShapes`' wrapping accumulator. Seeded to **1.0** - the ribbon model's own authored `TrailSpeed` - by `Trail_ResetCraftSlotState` (`0x002e2188`), which the constructor calls once per slot and which nothing else in the image calls at all. The material's `TrailSpeed` parameter is bound to this **address**, so it *is* the draw-time constant rather than being copied into one |
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

- **`u(k) = u_head * (1 - k * 4/54)`, and the scroll is not in it** - the
  constant `4/54` fits at `0.0741` on all six buffers, and `u_head` is the
  `+0x11ec` value (`1 - 0.6 * speed01`), so the streaks stretch with speed.
  `v` runs 0 to 1 across each fin. **The 2026-08-24 revision is the missing
  `+ phase`**: an earlier fit of this line carried one, which conflicted with
  the `+0x11ec` row above ("equals the head vertex's `u`") and could not be
  true at the same time. Re-measured against the seven `--early` snapshots,
  which keep the block and its vertex buffer from the *same* pause:
  `max |u(k) - u(0) * (1 - k * 4/54)|` over all 54 rings is **2e-6** at `e0`
  and **1e-6** at `r1`, while the same residual against `+ phase` is the
  phase itself (0.881 and 0.155). `r1` is the discriminating one, because its
  `u_head` of 0.613 is far enough from 1.0 to rule out a mod-1 coincidence:
  head vertex `u` 0.61510, `+0x11ec` 0.61336, `+0x1210` 0.15526. **The SPU
  writes no scroll into the vertex.** Where the phase does enter is the next
  section.
- **colour: white to red over the first 27 rings** - green and blue fall
  linearly `255 -> 0` at ring 27 and stay 0; red never moves.
- **alpha = brightness x attack x falloff** with `attack = min(k * 10/54, 1)`
  (a 5.4-ring ramp from the nozzle) and `falloff = 1 - k/54` (linear to
  nothing at the tail). At brightness `0.717` the measured peak is `162/255`,
  which is `0.717 * (1 - 6/54)` to the byte.

**Confidence 92** on all of the above: the numbers are the running game's
own, cross-checked between sessions, and the fin arithmetic (`954`, `0x2d90`)
closes twice over from independent constants.

## The ring at a race start: born full, bunched, and dark

Read live on 2026-08-24 (`scripts/rpcs3-trail-dump.py --early`): the debugger
attached while the race was still loading and polled for the manager; at the
**earliest frame the manager exists** the player's ring already holds 54 valid
samples - colour `(1,1,1,1)` on every one - bunched within **0.2 world units**
of the grid slot, and the SPU output buffer holds all 324 extruded vertices
with **peak vertex alpha 0**, because the brightness at `+0x11d8` is 0.0 below
the speed ramp's 100-unit floor. Five snapshots through the countdown (`e0`,
+1 s, +3 s, +7 s, +15 s) read the same; two more after thrust was held show
the mechanism of the fade-in: at +8 s of driving the ring spans 37.7 units,
brightness 0.151, peak alpha 32/255. **There is no fill gate anywhere** - no
part-empty ring ever exists to gate on. The trail appears by the brightness
ramp alone while the bunched ring stretches out behind the accelerating craft.

`oag_render::exhaust::hd::Tube` now reproduces this: the first push after a
construction or a clear seeds the whole ring with that sample, so the ring is
always full from the first tick exactly as the live one is, and the PSP's
0.9-second full-ring gate no longer applies to HD. What the original does to
the ring across a **respawn** is still unread; re-bunching at the new pose is
this engine's approximation, chosen because it is the same state the measured
race start begins from. **Confidence 90** on the race-start reading (five
independent snapshots, ring and vertex buffer agreeing), stated approximation
on the respawn.

## The draw state's two non-material constants, placed

`Trail_BuildDrawState` reads two things this page used to leave dangling:

- **The splatted vec4 is a clock, not the scroll.** Outside the per-craft
  loop it reads `*(*(*0x00936fd4) + 0xc4)` (TOC slot `0x008b44a0`), splats it
  to four lanes and binds it as a draw-state constant. `0x00936fd4` -
  named `Render_FrameContextPtr` at confidence 55, so it wears a `_q` -
  holds the object the present loop at `0x00018848` hands to the frame
  render calls; that loop is `Game_PresentLoop` (confidence 65, `_q`): it
  calls `cellSysutilCheckCallback` once per iteration - the call a PS3 title
  must make once per frame - runs a mirrored two-pass draw sequence for the
  stereo path, and loops until an exit flag. The context's `+0xc0`/`+0xc4`
  carry the **same monotonically increasing seconds value** - read live at
  nine pauses across one session it marched 48.97 to 119.37, advancing at
  game-time rate, one global value while the eight trails' `+0x1210` phase
  accumulators all diverged. So the "this splat is
  the live `TrailSpeed` patch" hypothesis is **refuted**; a global seconds
  clock bound to the trail material is instead the natural candidate for the
  flame surface's unread `Speed * time` scroll clock
  ([engine-flare.md](engine-flare.md)) - hypothesis only, confidence 40, no
  shader-side confirmation yet.
- Where the per-trail `TrailSpeed` value physically lives at draw time is
  **not** here either - both obvious carriers are excluded (the splat above is
  global; the `+0x1208` per-trail binding's target holds no phase *value*).
  The next section is where it turned out to be, and the reason both searches
  missed it is that nothing copies the phase anywhere: it is bound by address.

## The scroll phase is bound by pointer, once, at construction

This is the last open link in the scroll chain, and it closes in the
constructor rather than in any per-frame path - which is why two sessions of
frame dumps could not find it.

`Trail_ConstructManager` (`0x002e2da8`) and the second ctor (`0x002e2f40`)
both call one shared body, `Trail_InitManagerBuffers` (`0x006b6870`): it
allocates the two SPU output buffers per trail, builds the shared index buffer
as 3 fins x 53 segments of `{k, k+1, k+2, k+1, k+3, k+2}` - 954 indices, the
draw call's own count, arrived at here from the loop bounds rather than from
the packet - and then, per trail, does this:

```c
/* Trail_InitManagerBuffers, the per-trail tail of the loop */
instances = Alloc(materials * 4);
for (i = 0; i < materials; i++)
    instances[i] = Material_CloneInstance(model_materials[i], -1, 0);
block[0x120c] = materials;
block[0x1208] = instances;

if (!hash_done) { hash = ~Crc32_HashString(Trail_SpeedParamName); hash_done = 1; }
Material_SetInstanceParamPointer(instances, materials, 0, hash,
                                 &block[0x1210]);
```

`Material_CloneInstance` (`0x005d4d28`) is why each trail gets a slot of its
own to bind: with both flag bits set - and `-1` sets them - it copies the
record's 0x40-byte header, then its `count * 0x20` parameter entries to
`clone + 0x40`, then the value blob at `+0x38`/`+0x3c` behind those, and
**relocates every non-sampler entry's `+0x18` into the copy**. So the file's
value *offset* is an absolute pointer by the time anything draws, and
overwriting one touches that trail alone.

`Material_SetInstanceParamPointer` (`0x005d4bb0`) is then six lines of the
material record's own layout, the one [engine-flare.md](engine-flare.md)
validated disc-wide:

```c
for each instance:                       /* class filter is 0, so: all */
    for each 0x20-byte entry at inst[+0x34], inst[+0x30] of them:
        if (!(entry[+0x04] & 0x8000) && entry[+0x00] == hash)
            entry[+0x18] = value;        /* the value pointer */
```

Three things fall out, in order of how much they change:

- **`TrailSpeed`'s draw-time value is the live `+0x1210` phase**, read through
  a pointer the constructor wrote once. Nothing patches it per frame, which is
  why `Trail_BuildDrawState` (read line by line) never mentions it and why the
  0x334 command packet was the wrong place to look.
- **The string is read exactly once in the whole executable.** The TOC slot
  `0x008b4478` -> `0x007a2da8` `'TrailSpeed'` - `Trail_SpeedParamName` - is
  loaded by one instruction, `0x006b6cc8`, inside that body - found with
  [`scripts/ps3-toc.py`](../../../../scripts/ps3-toc.py)'s `scan_toc_loads`,
  because Ghidra resolves no `lwz rX,disp(r2)` here. `~crc32("TrailSpeed")` is
  `0x07431a35`, which is the hash the ribbon material declares
  (`ps3-sho.py hash TrailSpeed`), so the two ends of the lookup meet.
- **The accumulator starts at the authored value.** `Trail_ResetCraftSlotState`
  (`0x002e2188`), called eight times from this body and from nowhere else in
  the image, writes `+0x1210 = 1.0f` and clears `+0x1208`/`+0x120c`. It is not
  `Trail_InitCraftSlot`, which the ctors call separately for the SPU job. `1.0`
  is exactly what `enginetrail_bluered_triangle.rcsmodel` authors for
  `TrailSpeed`, and it is a no-op on a `REPEAT`-wrapped sampler. **The wrap
  state itself is unread** - the same gap this page already records for the
  sampler's sRGB remap - but `REPEAT` is the only consistent reading: the
  shader's `u + TrailSpeed` exceeds 1.0 for most of the ring at rest, the
  accumulator is kept in `[0, 1)` by its own wrap on every live read, and the
  trail streaks rather than clamping to an edge texel. `oag_render::exhaust::hd::Tube` starts its phase at
  `0.0` instead, which is the same thing after the first wrap and every live
  phase read on this page is in `[0, 1)`; the difference is not observable and
  is not worth a constant.

The material record on the disc agrees field for field: parsed straight out of
`/data/ribboneffects/enginetrail_bluered_triangle.rcsmodel`, its record at file
offset `0x60` declares `+0x30 = 6` entries at `+0x34 = 0xa0` - three samplers
(`0x2743292b`, `0xcc5bb827`, and the null `0x37b5db58` lightmap) and three
parameters, `0xe296b1ed = 0.15`, **`0x07431a35 = 1.0`** and
`0xbb48e390 = 0.0`. Those are exactly [engine-flare.md](engine-flare.md)'s
three patch values, now read from the model rather than inferred from the
material, and the `1.0` is the value `Trail_ResetCraftSlotState` seeds the
accumulator with.

### And the running game says the same, in one pause

`scripts/rpcs3-trail-dump.py --params` follows exactly that hop live: block ->
`+0x1208` -> the instance -> its parameter table -> the `0x07431a35` entry's
`+0x18`. Two pauses 0.75 s apart in a driven Fury race, trails 0..3:

| Slot | trail block | `TrailSpeed` value pointer | `block + 0x1210` | phase, pause 1 -> 2 |
| --- | --- | --- | --- | --- |
| 0 | `3357b7a0` | `3357c9b0` | `3357c9b0` | 0.194 -> 0.399 |
| 1 | `3357c9d0` | `3357dbe0` | `3357dbe0` | 0.308 -> 0.522 |
| 2 | `3357dc00` | `3357ee10` | `3357ee10` | 0.339 -> 0.555 |
| 3 | `3357ee30` | `33580040` | `33580040` | 0.264 -> 0.445 |

Four discriminators, none of which a coincidence survives: the four pointers
are **`0x1230` apart**, the trail-block stride; each equals its *own* trail's
phase address and no other's; all four are **byte-identical across the two
pauses while the floats they point at move**, which is what a live pointer
looks like and a stale copy does not; and each instance's table reads back as
the disc's own six hashes in the disc's own order - `0x2743292b`,
`0xcc5bb827`, `0x37b5db58`, `0xe296b1ed`, `0x07431a35`, `0xbb48e390` - with the
hit at entry 4 and its value pointer at instance `+0xd8`, exactly
`0x40 + 4 * 0x20 + 0x18` of `Material_CloneInstance`'s layout - and the scan
for that hash across each instance's first `0x200` bytes returns `0xc0` **and
`0x1f0`** on the two slots whose neighbours are adjacent, `0x130` apart, which
is `0x40 + 6 * 0x20 + 0x30` read back out of the live heap. The clone's size
closes from the disc and from memory independently. The same pause
read the head vertices: `u` `0.57444` against `+0x11ec` `0.574396` with the
phase at `0.194` - the vertex law again, with no scroll in it.

**Confidence 94.** Every function is fully decompiled, the entry layout is the
disc-validated one, the hash closes from both sides, the authored `1.0` closes
against the constructor's seed, and the whole hop is confirmed in the running
game on four trails in two frames. What is read at one call site only is
`Material_SetInstanceParamPointer`'s generality, which is what holds its own
`names.tsv` row below 90 rather than the mechanism.

**One trap this cost a run, worth the line.** The instance pointers live at
`0x40cbfbd0`-style addresses while the array pointing at them lives at
`0x3058....`; a heap-range guard that stopped at `0x40000000` rejected all four
and reported an empty hit list - which reads exactly like "the binding is not
there". That is the third variant of this page's recurring trap: **a plausible
range test can hide a pointer as thoroughly as a plausible float can.** The
probe now reports every field it looked at, misses included.

### What this settles about the scroll, end to end

With [engine-flare.md](engine-flare.md)'s reading of the ribbon's **vertex**
program (new the same day) the whole chain is now closed, and it says the
phase is applied exactly **once** to each of the two texture lookups:

| | coordinate | where the phase enters |
| --- | --- | --- |
| noise (`unit 0` alpha) | `f[TC3].zw` | the vertex program's `ADD o[TC3].z, u, c[210]` |
| colour (`unit 0`/`unit 1`) | `R2.zw` | the fragment program's `@0x02`, on the raw `u` its `@0x00` kept in `R0.z` |

Both constants are the same `c466`/fslot-`0x4c` `TrailSpeed`, and the SPU's
vertex `u` carries none of it. So the original's coordinates are
`(u0 + phase)` and `(u0 + phase + noise)` - which is what
`oag_render::exhaust::hd::Tube::vertices` produces by adding the phase to the
ring `u` once on the CPU and letting `exhaust.wgsl`'s `trail_shape` branch
displace from there. **The implementation was already right; its stated reason
was not.** No constant moved for this finding, and none should: what changed
is that the vertex law and the draw-time constant are now two measurements
instead of one fused guess.

## Does another craft disturb the ribbon? The geometry, no - but a craft flying through one **does** spark

A plausible memory of the original - and a plausible reading of the code -
is that flying through someone's exhaust pushes it aside. The code half is
suggestive: `Trail_WriteCraftContexts` builds a **compacted** array of
`0x60`-byte per-craft contexts at `alloc + 0x116f0` - a vec4 from
`0x000d1e48`, the craft's 4x4 matrix, its craft index at `+0x50` and the
Fury flag at `+0x54` - and writes the count into every trail's `+0x11e8`
(above). `Trail_InitCraftSlot`'s DMA list hands that whole array to each
trail's SPU job. So the job that extrudes **one** ribbon receives **every**
craft's transform, and a per-craft vec4 that could be a bounding sphere is
exactly the shape an intersection test would want.

**The extruded vertices say it does not happen.** Read 2026-08-24 from a
driven Fury race with all eight craft in a strung-out pack, two pauses ~1 s
apart, dumping every trail's ring *and* its live output buffer
(`scripts/rpcs3-trail-dump.py` with no flag, then the proximity split
below). Each craft's current nozzle is its own ring's newest sample, so the
positions and the geometry come from the same pause.

The test is a **spatial correlation**, not a threshold: if a craft pushed the
ribbon aside, the ribbon's departure from what its own ring predicts would
have to spike at the rings nearest that craft and be flat elsewhere. So each
trail's rings are split into those within 6 units of *another* craft and
those beyond 20, and both buckets are measured the same way.

- **The width never moves.** `|edge separation - 1.0|` is **0.0000** in every
  bucket of every trail in both pauses - including the five trails with
  another craft between **0.19 and 1.43 units** of the ribbon, which is
  inside or touching a tube of half-width 0.5. A craft sits *within* the
  ribbon and the ribbon is exactly as wide there as it is 70 units from
  anything.
- **The centre-line residual does not correlate with proximity.** Trail by
  trail, near against far: 0.145/0.115, 0.193/0.066, 0.112/0.131,
  0.169/0.190, 0.372/0.524, 0.028/**0.177**, and the two trails with no craft
  within 12 units read 0.363 and 0.182 - the same range. It is as often
  larger far away as near, and trail 7's *lowest* residual is at its closest
  rings. The second pause repeats it with craft as close as 0.19 units.

**That residual is this reconstruction's, not a deformation.** The fin
midpoints do not sit exactly on the stored ring samples - they are off by up
to ~0.5 units on a 50-unit ribbon, uniformly along it and with no craft
anywhere near. Why is **unread**: the likeliest causes are that the job
smooths or resamples the ring rather than extruding each stored sample where
it lies, or that the head sample is placed at the live nozzle rather than the
newest recorded one. `oag_render::exhaust::hd::Tube` extrudes each sample
where it is stored, which is a stated approximation until that is read.

**Confidence 88** on the negative: two pauses, eight trails, 864 rings, craft
inside the tube, and the one quantity that a rigid displacement could not
hide - the width - is bit-exact everywhere.


### But the craft sparks, and the disc names the effect for it

**A first pass of this page stopped at the negative above and was too narrow.**
The ribbon's *geometry* is untouched, which is what the vertex dump measures -
and the reaction happens somewhere the vertex dump cannot see: a particle
system on the struck **hull**. The disc names it, in two colours:

```text
007a1ac0  'Data\Psys\WO_TRAIL_HITSHIP_RED.POB'
007a1ae8  'Data\Psys\WO_TRAIL_HITSHIP.POB'
0x007a1cc8  'WO_TRAIL_HITSHIP_RED'    <- Trail_HitShipEffectNameRed
0x007a1ce0  'WO_TRAIL_HITSHIP'        <- Trail_HitShipEffectName
```

The last two are the bare names `Trail_HitShipEffect` looks the systems up by;
the first two are the paths `0x002d9270` preloads.

They sit in the ship-effect run **between**
`WO_SHIP_SPARK_DAMAGE_WEAPON.POB` and `WO_NITRO_DEBRIS_SPARKS.POB`, and
`0x002d9270` preloads them per craft in the same block as
`WO_SHIP_COLL_SPARK_DAMAGE.POB` / `..._NODAMAGE.POB` - so they are literally
authored as a sibling of the collision sparks.

The chain, read end to end:

1. **The SPU job raises a flag.** Bit 61 of the trail block's `+0x11d0`
   (`0x2000000000000000`) means "this trail hit a ship this frame";
   `+0x11f0` holds the trail slot of the craft involved and `+0x11dc` a float
   the spawner takes in `f1`. **Nothing on the PPU sets that bit** - a scan of
   every `ld`/`std`/`lwz`/`stw` in the image at displacement `0x11d0` finds 26
   accesses, all in the trail manager, and each one either sets the slot index
   (bits 55-58), bit 60, bit 63, or *clears* bit 61 after reading it. The whole
   field sits inside the `0x1200` bytes `Trail_InitCraftSlot` DMAs to and from
   the job. **So the `Trails` job does the intersection test** - which is what
   the per-craft contexts and the `+0x11e8` count are for, the question the
   section above had to leave open.
2. **`Trail_SpawnHitEffect` (`0x002e3858`, with an identical twin at
   `0x002e2678`) consumes it**, once a frame, over all eight slots. For a slot
   with the flag set it walks **ten attachment nodes on the craft**,
   `craft + 0x79d0` through `+0x79f4`, keeps the one nearest the contact point
   (sentinel distance `100000.0` at `0x008b443c`), gates on the craft being in
   neither of two states (`+0x5ed8`, `+0x5f42` - unread) or on a global mode
   flag, spawns, and clears bit 61.
3. **`Trail_HitShipEffect` (`0x002d9ec0`) spawns it**, parented to that node,
   under a four-character tag - `'TRLI'` for the blue variant and `'TRLR'` for
   the red.

**The colour follows the ribbon, from one instruction.** The call site is
unambiguous:

```text
002e3b78  lfs  f1, 0x11dc(r30)     <- the float the block carried
002e3b88  lwz  r11, 0x1204(r30)    <- the craft
002e3b90  lbz  r5, 0x7d2c(r11)     <- its Fury-skin byte, as the variant select
002e3b94  lwz  r3, 0x0(r9)         <- the nearest attachment node
002e3b98  bl   0x002d9ec0
```

`craft + 0x7d2c` is **the same byte** `Ship_SetFuryTrailFlag` writes and
`Trail_BuildDrawState` turns into the `engineTrail` colour-mix vec4 that makes
the ribbon red. So the sparks are red exactly when the ribbon is. In practice a
grid is uniformly Fury or classic (every craft of a Fury event derefs to
`concept1`), so the two always agree on screen.

**Confidence 88** on the chain: every function is decompiled, the two `.pob`
names resolve from the executable's own strings through
[`scripts/ps3-toc.py`](../../../../scripts/ps3-toc.py), the variant select is
read off the instruction, and the SPU's role is established by elimination over
every access to the flag word in the image. What is **not** read: the test
itself (the `Trails` job's code), what `+0x11dc` and the two craft-state gates
mean, and whether `+0x11f0` names the striking or the struck craft - the effect
attaches to *that* craft's hull either way.

**Implemented 2026-08-24**, and the split is deliberate. `oag_render::psys`
plays the disc's own `.POB` - both variants load from the HD archives, pinned
by `crates/game/tests/hd_engine_flare_ground_truth.rs` - and
`oag_game::race::Race::advance_trail_hits` fires it. **Nothing about the effect
is invented; only the moment it fires is.** Three things there are this
engine's and are named in that method's own doc rather than buried: the
geometric test (the craft's own origin-to-nozzle distance plus the ribbon's
measured half-width, against the trail's centre line -
`exhaust::hd::Tube::nearest`), the rate (**every tick a craft is inside**), and
the attachment point (the point on the intruder's own bounding sphere facing
the contact, re-spawned each tick so it rides the ship - this engine has no
equivalent of the ten hull nodes the original parents to).
`crates/game/src/race/tests/trail_hits.rs` pins all three.

**All three were wrong on the first pass, and only looking settled them.**
The third took **two** goes and is the sharpest lesson here. A player looking at
the corrected effect asked whether it was anchored right: the burst was spawning
at the contact point *on the ribbon*, so it read as sparks on the trail, where
the original parents to a hull node and rides the craft. The first fix clamped
the placement to `min(|contact - centre|, reach)` - which is a **no-op exactly
when it fires**, since the burst only fires when the craft is within `reach` of
the ribbon, so the clamp always picked the distance and the burst stayed on the
trail. The same player reported it unchanged. It is now `reach`
unconditionally, and `hull_contact_point` is a free function with that property
asserted directly, because a placement that looks right in the diff and is
wrong on screen is what this got twice. A player who could not trigger the effect in a race is what prompted
the measurement, and both faults hid behind the same symptom - nothing on
screen.

- **The reach was in the wrong space.** `mesh::Model::radius` is a *model*-space
  AABB half-extent and reads 69 to 379 across HD's hulls; as a world-space
  reach that put every craft inside every trail from tick 0, fired 24 bursts on
  the grid and then never re-armed for the rest of the race. The craft's own
  origin-to-nozzle distance is the same quantity in the right space: **3.7 to
  6.0 units**, about half a hull length, off a locator the disc authors.
- **The rate was edge-triggered, and the asset says it should not be.**
  `WO_TRAIL_HITSHIP` parses as a **one-shot**: duration 1 tick, `looping`
  false, 5 particles of 0.2..0.5 units with a size channel down to a third by
  17 % of its life, plus a 3-tick `CORE_IMPACT` child, both additive streaks
  against a 2,000 live cap. A system shaped like that is authored to be
  re-fired while its condition holds - and the original's own consumer *clears
  the flag every frame*, so a craft that stays inside re-raises it. Measured on
  screen at a dark section: **6 changed pixels** of 1,175,040 fired once per
  entry, **7,592** fired per tick. What is still unread is whether the SPU job
  really re-raises the flag each frame, which is why this stays an
  approximation rather than a reading.

**The player's own difficulty is itself a measurement**, and worth keeping: over
2,400 ticks of an autopiloted eight-craft race, the craft the camera follows
enters another's trail **three times**, twice in the first 1.5 seconds off the
grid. The effect is common between rivals up the road and rare in front of the
camera, which is why `--trail-sparks` exists - it forces the burst on the
player every tick so the *drawing* can be judged without waiting for the
*trigger*.

**Which craft it lands on was settled by a sighting, not by the code.** The
disassembly ties the node, the colour and the float all to the craft at
`+0x11f0`, and nothing read says whether that is the trail's owner or the craft
that flew through. Someone playing the original reported sparks on the *ship*,
in the trail's colours - so it is the intruder, and the burst takes the
intruder's own red flag. On a uniformly Fury or classic grid the two readings
give the same picture, which is why the code could not distinguish them and an
observation could.

## The third sampler is the lightmap slot, empty

`enginetrail_bluered_triangle.rcsmodel`'s material record names a third
sampler, hash `0x37b5db58` with a null path. That hash is already a recovered
preimage: **`lightmap`** (`~crc32`, 1,669 uses disc-wide - see
[rcsmaterial.md](../../../formats/rcsmaterial.md)). The trail material
declares the standard lightmap sampler slot and binds nothing to it; it is
inert, not a missing texture.

## Matched-view comparison against the running original

Done 2026-08-24, the first side-by-side at race speed: RPCS3 screenshots
every 4 s through a driven Talon's Junction Fury race
(`scripts/rpcs3-drive.py race --shots` route) against this engine's
`oag-game --race data/images/hdfury-ps3-eu-dec.iso --autopilot
--team feisar_c1 --ticks N --screenshot` at landmark-matched sections.
Quantified as perpendicular profiles of warm excess (`R - (G+B)/2`) behind
the nozzle at fractions of the on-screen ship width, on both sides. Note the
comparison is *not* frame-rate-contaminated: RPCS3 runs slow in wall time but
steps ~1/60 game-time per frame, so ring spacing in world units matches a
60 fps run.

- **Over comparable (dark-to-mid) background the tube matches.** Original,
  dark tunnel mid-corner: warm excess 110/75/55/40 at 0.15/0.3/0.5/0.7 ship
  widths. Ours, dark section: 79/78/43 at 0.15/0.3/0.4 (0.5 ran off-frame).
  Same character - white-hot core at the nozzle, red fringe, streaks.
- **The halo is not narrower.** Skirt widths normalised by ship width:
  quarter-max 0.079..0.109 (original) against 0.062..0.145 (ours);
  tenth-max 0.085..0.135 against 0.172..0.193 - our faint skirt is, if
  anything, wider. No bloom-share deficit is measurable at these views, so
  nothing here justifies touching `post::hd_bloom`'s recovered constants.
- **Over bright sections ours vanishes and the original's does not - and
  that is the scene, not the trail.** In our bright-section frames the
  background behind the trail sits at luminance 187..255 with green already
  saturated; a `SrcAlpha`/`One` additive tube physically cannot register
  there. The original's same corners keep background luminance 77..186. This
  is the known scene-wide brightness-calibration gap (its own work item),
  and no trail constant was changed in response.
- **The `v` orientation is confirmed, not flipped.** In three dumped buffers
  across two sessions, every fin's `v` runs 0 at the `-fin_dir` edge to 1 at
  the `+fin_dir` edge (fin directions at 0/60/120 degrees from up, rotating
  toward `forward x up`) - exactly `hd.rs`'s `(edge + 1) * 0.5`. The blue
  texture's near-transparent alpha row sits on the `-fin_dir` edge on both
  sides.

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
it. **That hash has a preimage now: `engineTrail`**, entry 80 of the engine
parameter table - so the patch is not a special case in the trail's draw path
but the ordinary engine-parameter mechanism, and the model's authored `0.0` is
the default it overrides. See
[renderer.md](renderer.md#the-engines-own-parameter-table-read-from-its-initialiser-2026-08-24). On the disc the variants live in the `*_c1` (concept) and `*_n1` (nitro)
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
- The flame surface's `Speed * time` scroll is **read** (2026-08-24) and not
  yet drawn: `time` is entry 0 of the executable's own 81-name engine
  parameter table, and the seconds clock at `*(*0x00936fd4)+0xc4` is what
  every draw-state builder writes into it - the hypothesis this bullet used to
  carry, now confirmed from the initialiser. See
  [renderer.md](renderer.md#the-engines-own-parameter-table-read-from-its-initialiser-2026-08-24)
  for the table and [engine-flare.md](engine-flare.md) for what the flame does
  with it. The ribbon's phase at `+0x1210` is still a *different* accumulator.
- The `Engine_Flare_Rich.gtf` sprite flare **draws now** with its authored
  radius (3, jittered by 0.5, floored at 2) and the `Slow Alpha Noise`
  opacity walk (0.5..0.8, chase 0.1, retarget every 10) - the flare's init
  (`0x002a1528`) loads the texture and four corner pairs, so the sprite is
  real, and it is most of the "solid core" the exhaust reads as in the
  original. Still unread and undrawn: its spin, chromatic dispersion,
  `Flare Fadeout Dist/Range` term and the occluder query - the shader pair
  (`engineflare_vp/fp`) resolves through no registry read so far.
- (Closed 2026-08-24, both halves.) The splatted seconds clock **is** the
  flame surface's `time`: it is written into engine parameter entry 0, which is
  named `time` in the table `Shader_InitEngineParams` builds - see
  [renderer.md](renderer.md#the-engines-own-parameter-table-read-from-its-initialiser-2026-08-24).
  (And where the per-trail
  `TrailSpeed` value lives at draw time, is **closed at 94**: the constructor
  binds the material parameter to `&block[0x1210]` and the running game shows
  the four pointers - see "the scroll phase is bound by pointer". Nothing in
  the renderer changed for it; the scroll was already applied the right number
  of times, for a reason the comments had wrong.)
- `WO_TRAIL_HITSHIP` / `WO_TRAIL_HITSHIP_RED` - **read and not drawn.** The
  trigger is the `Trails` SPU job's own ribbon-versus-craft test, which this
  engine has no equivalent of; the effect plays through `oag_render::psys`
  the moment one exists. See "the craft sparks" above.
- What `+0x11dc` carries into the spawner's `f1`, what the two craft-state
  gates at `+0x5ed8`/`+0x5f42` mean, and whether `+0x11f0` names the striking
  or the struck craft.
- What the original does to the ring across a **respawn** - the race start is
  read (born full, bunched, dark) but a respawn was never captured; this
  engine re-bunches at the new pose as a stated approximation.
- Whether the white-to-red vertex ramp differs on a classic (blue) HD grid -
  every craft in the dumped sessions was a Fury `concept1`.

## Reproducing this

```sh
uv run --with evdev python3 scripts/rpcs3-trail-dump.py /tmp/hd-trail-dump
uv run --with evdev python3 scripts/rpcs3-trail-dump.py /tmp/hd-trail-early --early
uv run --with evdev python3 scripts/rpcs3-trail-dump.py /tmp/hd-trail-params --params
# "does another craft disturb the ribbon": the default dump, then the split
python3 scripts/hd-trail-wash-check.py /tmp/hd-trail-dump s0
# the scroll chain, entirely offline - the ribbon's own two programs, the one
# instruction in the image that reads 'TrailSpeed', and the hash both ends use:
python3 scripts/psarc.py cat data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA06.PSARC \
    /data/ribboneffects/materials/hd_enginetrail_bluered.rcsmaterial > /tmp/trail.rcsmaterial
python3 scripts/ps3-microcode.py vp-file /tmp/trail.rcsmaterial 0
python3 scripts/ps3-microcode.py fp-file /tmp/trail.rcsmaterial
python3 scripts/ps3-sho.py hash TrailSpeed
python3 scripts/psarc.py cat data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC \
    /data/ships/shipeffectstweaks.txt
OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all hd_engine_flare
# our side of the matched-view comparison, at a landmark-matched tick:
target/release/oag-game --race data/images/hdfury-ps3-eu-dec.iso --autopilot \
    --team feisar_c1 --ticks 1050 --screenshot /tmp/ours-t1050.png
```
