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

`oag_fx::exhaust::hd` implements what this page measures.

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
- **colour (Fury skin, flag 1): white to red over the first 27 rings** - green and blue fall
  linearly `255 -> 0` at ring 27 and stay 0; red never moves. A classic craft has a different ramp: see "The vertex colour ramp is per skin".
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

`oag_fx::exhaust::hd::Tube` now reproduces this: the first push after a
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
  trail streaks rather than clamping to an edge texel. `oag_fx::exhaust::hd::Tube` starts its phase at
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
`oag_fx::exhaust::hd::Tube::vertices` produces by adding the phase to the
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
newest recorded one. `oag_fx::exhaust::hd::Tube` extrudes each sample
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

**Implemented 2026-08-24**, and the split is deliberate. `oag_fx::psys`
plays the disc's own `.POB` - both variants load from the HD archives, pinned
by `crates/game/tests/hd_engine_flare_ground_truth.rs` - and
`oag_raceplay::Race::advance_trail_hits` fires it. **Nothing about the effect
is invented; only the moment it fires is.** Three things there are this
engine's and are named in that method's own doc rather than buried: the
geometric test (**any of the craft's own authored `Ship Collision Fx` locators
within the ribbon's measured half-width of its centre line** -
`exhaust::hd::Tube::nearest`), the rate (**every tick a craft is inside**), and
the attachment point (**the intruder's own authored `Ship Collision Fx` locator
nearest the contact**, re-spawned each tick so it rides the ship).

**The ten nodes are almost certainly the collision-spark locators.** Loading
every HD craft, each hull authors **7 to 10** of them - and
`Trail_SpawnHitEffect` walks exactly ten node slots at
`craft + 0x79d0 .. +0x79f4`, testing each for null and stopping at the first
one, which is what a hull authoring fewer than ten needs. The engine already
read that set for collision sparks under the name the disc gives it, and it
picks the nearest of them the same way.

**The contact those anchors are measured against is the hull's leading edge**,
and that part is derived rather than read - the original measures its ten nodes
against a point the decompiler lost. Measuring from the craft's *centre* was
tried and is degenerate: a craft already inside a ribbon has it running through
end to end, so nose and tail are equidistant and the pick is a coin toss, which
on Assegai landed at the exhaust. The nose is the mirror of the authored nozzle
offset, and `a_craft_inside_a_trail_sparks_from_its_leading_half` pins the
consequence on the disc's own hulls: **4.45 units ahead of the hull centre**. So the placement is the disc's data
rather than this engine's arithmetic. Confidence 75 on the identification -
the counts and the "nearest of" rule agree, but nothing has been read that ties
`+0x79d0` to the locator loader.
`crates/raceplay/src/tests/trail_hits.rs` pins all three.

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
- **The test was a sphere, and a hull is not one.** It first asked whether the
  craft's *centre* was within `hull_reach + 0.5` of the ribbon, and
  `hull_reach` is the origin-to-nozzle distance - about half a hull **length** -
  so as a radius it reaches far past a hull that is much narrower than it is
  long. A player reported the sparks firing before the craft touched the trail.
  Testing the hull's own authored anchors instead is both tighter and made of
  the disc's data: walked sideways out of a ribbon on a real HD hull, the
  anchor test stops firing at **0.50 units** off the centre line where the
  sphere fired out to **4.20**.
  `the_anchor_test_fires_later_than_the_sphere_it_replaced` measures it.
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
`oag_fx::exhaust::hd`'s constants against the file itself.

### All 41 rows, read 2026-09-01

The 13 rows above and the sprite's 9 (`the sprite flare draws` below) account
for 22 of the file's 41 lines. The rest, read in full for the first time -
`just psarc cat ...:DATA02.PSARC /data/ships/shipeffectstweaks.txt`, no
decryption step beyond the disc's own layer-1 key:

| Key | Value | Where it went |
| --- | ---: | --- |
| `Spikes Thrust Max Scale` | 1.5 | runtime storage unresolved - the `+0x490` the fourth session assigned by value is `Max Chromatic Dispersion` ("Ninth session" below), and the block's head holds 1.5 at both `+0x420` and `+0x42c`; no consumer read; the five spike shapes' own throttle scale, separate from `EF_Main`'s |
| `Spikes Boost Max Scale` | 2.0 | runtime storage unresolved - not `+0x480`, which is `Flare Radius Min` ("Ninth session"), and 2.0 sits at both `+0x428` and `+0x430`; no consumer read; the spikes' boost scale |
| `Engine Flare Particles Min Alpha` | 0.25 | runtime storage `+0x44c` (not `+0x464`, which is `Tex Scroll Speed Thrust Contrib` - "Ninth session") - no consumer read; see below |
| `Enable Engine Flare Particles` | 1 | runtime storage `+0x450`, a byte ("Ninth session") - no consumer read; see below |
| `Shockwave Cycle Speed` | 0.01 | runtime storage found (`+0x46c`) - no consumer read, no shockwave node identified yet |
| `Flare Highlight Power` | 32.0 | **read** (`+0x470`): the exponent of `powf(view_dot, 32.0)` in `EngineFlare_RenderTick`'s fade, "Ninth session" below - a view-angle highlight lobe, not `flame_test.rcsmaterial`'s own `power1` (10.0, already read on engine-flare.md) |
| `Flare Highlight Boost` | 1.0 | **read** (`+0x474`, the tie settled by row order): multiplies the sprite's alpha, "Ninth session" |
| `Flare Fadeout Dist` | 15.0 | **read**: `+0x484` of the tuning block, consumed by `EngineFlare_RenderTick`'s linear fade as `f23` - "Fourth session" below |
| `Flare Fadeout Range` | 15.0 | **read**: `+0x488`, the same fade's `f24` |
| `Flare Occluder Radius` | 1.0 | **read** (`+0x4ac`): the xyz scale of the occlusion-query proxy `EngineFlare_RenderTick` draws with z-pass counting on, "Ninth session"; the fade's clamp ceiling at `+0x4a8` is `Flare Opacity Max`, not this |
| `Flare Size Clamp` | 50.0 | runtime storage found (`+0x4b0`) - **not** the fade's clamp ceiling (that is `+0x4a8` = `Flare Opacity Max`); `EngineFlare_RenderTick` never loads it ("Ninth session"), no consumer read |
| `Flare Depth Bias` | -0.46 | runtime storage found (`+0x4b4`, already named on hd.rs's `Sprite` doc) - `EngineFlare_RenderTick` never loads it either; no consumer read |

**`Engine Flare Particles` is a third component, not a variant of the two
already drawn.** The flare is a `.rcsmodel` (the always-on flame, read on
engine-flare.md), a sprite (`Engine_Flare_Rich.gtf`, read below), and now a
named-but-unlocated particle emitter - `Enable Engine Flare Particles` gates
it and `Min Alpha` bounds it, the same shape `Enable Flare Sprite` and
`Flare Opacity Max` take for the sprite. No `.pob` or emitter table has been
matched to it; the small bright dashes visible trailing under the nozzle in
`data/reference/hd-capture/flare-size/original-rpcs3.png` are the leading
candidate for what this draws; this project has nothing playing there yet.

**`Flare Size Clamp` (50.0) is new: no code or doc named it before this
session.** Against a base `Flare Radius` of 3.0 the two numbers are over an
order of magnitude apart, which is consistent with the clamp bounding
something other than the sprite's raw world-space half-size (a projected or
screen-relative size, most likely) - but that reading is a hypothesis from
the numbers alone, not a traced consumer, and nothing here acts on it.

**`Flare Fadeout Dist`/`Range` (15.0/15.0) is ruled out as the cause of the
oversized-flare finding below - a correction to what this page said here
before "Seventh session" reread the numbers.** A fade starting at 15 world
units and finishing at 30 does the *opposite* of what would explain the
screenshots: it leaves anything closer than 15 units - both engines' own
chase camera sits at a few units - fully faded *in*, i.e. undiminished, and
only shrinks or fades something the camera is 15-30 units from. It cannot be
why the player's own flare reads small in the original and large in this
engine; whatever does that is unrelated to this term. What two sessions' worth of reading could not settle
is which function actually builds the sprite's quad - what follows is what
each ruled out. `EngineFlare_Init` (`0x002a1528`) writes two adjacent constants-table
entries (index `0x1686`/`0x1687`) into `+0xe4`, `+0x144` and four identical
`(u,v)` half-float pairs at `+0x280..+0x2b2` - the "four corner pairs" the
sprite loads, confirmed to be **UV span defaults**, not positions, since all
four pairs are identical at construction.

**Correction, same day: `+0xe4` is not the radius.** A first pass this
session read `+0xe4` as "possibly the persistent radius storage" because
`EngineFlare_Init` seeds it from the same constants-table index as `+0x144`;
decompiling `EngineFlare_Update` (`0x002a3100`) in full settles it as the
exhaust **intensity** field instead - the 0.25/s-rise, 0.5/s-fall ramp
already named on this page, clamped to `[0, 1]` against `DAT_008b2ef0`/
`DAT_008b2ef4`, which is the wrong range for a 2-3-unit radius. The
constants-table index `0x1686`/`0x1687` Init reads for both `+0xe4` and
`+0x144` is therefore not `Flare Radius` either - more likely a shared
`0.0` sentinel several `Init` routines read off the table rather than
embed as a literal, on the pattern this codebase's own PPC idiom already
shows elsewhere. Neither `EngineFlare_PlaceShapes` (`0x002a1f00`) nor
`EngineFlare_Update` reads `Flare Radius`'s storage back out under any
offset checked; grepped both functions' full decompile for every plausible
field. The function immediately between `EngineFlare_Init` and
`EngineFlare_PlaceShapes` in address order, `FUN_002a17e8`
(`0x002a17e8`-`0x002a1efb`), decompiles to the engine-sound state machine
(idle/cruise/boost audio cue selection, keyed off `craft+0xe0`'s speed
class) - not the sprite draw.

**A second session went looking for the sprite's own vertex buffer live**
(`scripts/hd-flare-sprite-dump.py`, reusing `rpcs3-trail-dump.py`'s
craft-pointer route) rather than more static reading, on the reasoning that
the corners are a world-space offset from the nozzle and so settle the
question independent of the camera. It found six RSX-range pointers
(`0xC0000000..0xD0000000`) inside the object, at `+0x70`/`+0x74` (a pair,
changes every tick), `+0xb8` (static across two pauses), `+0x1f0`/`+0x1f4`
(a pair, duplicated verbatim at `+0x230`/`+0x234`) and `+0x240` (single,
changes every tick) - but every one of them reads back as near-all-zero
32-bit words, not plausible vertex positions (the one non-zero exception, a
repeating `~9.16e34` every fourth word, is a packed non-float value
misread as one, not a position either). **The leading reading is that these
are occlusion-query result buffers, double/triple-buffered for GPU
readback latency**, not a vertex buffer - consistent with `Flare Occluder
Radius` and `Flare Depth Bias` both being named-but-unread tunables that
imply exactly this kind of query. `Billboard.cpp` (four functions found via
the `__FILE__` attribution trick, one already named
`Billboard_ConstructResource` on [billboards.md](billboards.md)) was also
checked and ruled out - it is the **track sponsor-signage** system
`<TrackStartup>` declares, unrelated to the ship's own effects.

**The function that turns `Flare Radius` into a world-space quad remained
unlocated after both a static and a live pass** across two sessions - this is
the same wall `engineflare_vp`/`fp` resolving through no registry read already
put up two rows above. (Located since: it is `EngineFlare_RenderTick` itself,
whole only after the 2026-09-15 reimport - "Ninth session" below has the
corner build and the size law.)

### Third session: it was on `EngineFlare`'s own vtable, not a separate manager

**2026-09-02.** Both untried angles from the second session's writeup pointed
at the same place from different directions - "the manager `EngineFlare` calls
into" and "the draw's own vertex-array command" turned out to be the same
function, reached by reading the object's **other** vtable slots rather than
hunting a second object. `EngineFlare_Update`'s own vtable (`0x00869d30`,
bound in both constructors via `PTR_PTR_008b2f14`) has two class-specific
slots neither prior session opened: slot 3 is the already-known
`EngineFlare_Update` (`0x002a3100`); the two that matter are **slot 5** and
**slot 7** - the identical two slots `TrailEffectManager`'s vtable uses for
`Trail_Enqueue` and `Trail_RenderTick` (see "the manager, in the executable"
above). Both were read live off the vtable at fixed 8-byte OPD-entry stride,
not decompiled from a symbol Ghidra had already named, since neither had one.

**Slot 5, `EngineFlare_Enqueue` (`0x002a06e0`), confidence 84.** Byte-for-byte
the same function as `Trail_Enqueue` (`0x002e2610`) with only the sort-key
literal changed (`0x4d000000` vs. Trail's `0x4d100000`, both masked `&
0xfffff` against a depth field at `+0x11c` of the render-queue object): same
three-line body, same offsets (`+0x630` the object pointer, `+0x634` the sort
key, `+0x44b0` the running count). **Not just the same shape - the same
object**: `Trail_Enqueue` reaches it through `PTR_DAT_008b4438` and
`EngineFlare_Enqueue` through `PTR_DAT_008b2f5c`, two different TOC slots that
both hold the identical pointer value `0x00ae9c94`, read directly
(`read_memory` on each TOC slot, not decompiled). Both functions enqueue into
the one shared render queue Pulse's `Gfx_Enqueue` also uses, at the layer
`EngineFlare` picks for itself.

**Slot 7, `EngineFlare_RenderTick` (`0x002a08a8`), confidence 78.** This is
the function two sessions were looking for - it is gated, like
`Trail_RenderTick`, on a flag inside the *same* global object (`0x00aea540`,
confirmed live: `PTR_DAT_008b2f20` in this function and `PTR_DAT_008b44d0` in
`Trail_RenderTick` both hold `0x00aea540`), just at a different byte offset -
`+0x494` here, `+0x4b8` there. Past the gate it loads a dozen floats out of
that same object (`+0x470`..`+0x4a8`, unidentified past "shared render-frame
state") and reaches a chain of small TOC-fixup trampolines (`std
r2,0x28(r1)` / `addis`/`subi r2` / `b <target>`, jumping into a
different code region entirely - the two-TOC pattern
[memory.md](memory.md) documents) that lands on **`Rsx_SetMethod`**, an
already-named function, plus several more in the same `0x005a4000`-`0x005f1000`
neighbourhood that are not yet named but clearly build and submit a quad (one
of them, `0x005a40f8`, takes six pointer arguments shaped like a screen-space
rect plus per-corner UV/colour blend flags, and ends by calling
`Rsx_SetMethod` itself). Capped at 84 for decompilation-only per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md); not
runtime-verified, and scored a few points under that cap because the internal
float loads (the dozen fields off `0x00aea540`) and the exact vertex/UV
submission are not yet read - only that this is the function that reaches the
draw call is established, not what values it draws with.

**What is still open, now scoped tighter than "find the draw call":** which
of `EngineFlare_RenderTick`'s loads is `Flare Radius` (or its
`Flare Fadeout Dist`/`Range`/`Flare Size Clamp` companions from "all 41 rows"
above) is unread - the function's body is long (`0x002a08a8`-`0x002a1500`+)
and most of it is vector/RSX submission plumbing rather than the tuning
values themselves. The two ruled-out things from session two - the six
RSX-range pointers inside the `EngineFlare` object being occlusion-query
buffers, not vertex data, and `Billboard.cpp` being unrelated track signage -
still stand; this session did not need either. **Two destructor-shaped
slots on the same vtable were read and left unnamed**: slot 12 (`0x002a0470`)
and slot 13 (`0x0029f680`) both reset the vtable pointer to the shared base
(`PTR_PTR_008b2f14`) and call a free routine - consistent with a plain and a
virtual destructor pair, not investigated further because neither is on the
draw path.

### Fourth session: the tuning block itself, and a puzzle it did not settle

**2026-09-02, same day.** Two things: `EngineFlare_RenderTick`'s middle
(`0x002a0be8`-`0x002a1110`) turned out to be readable after all - Ghidra's
*decompiler* bails there ("Control flow encountered bad instruction data"),
but the raw bytes are ordinary AltiVec, confirmed with `disassemble_bytes`
against the plain instruction decoder rather than the function-level
analysis. **Corrected 2026-09-15: the cause was never a decompiler-analysis
failure.** `0x002a0be8` is an `lvlx` instruction (`lvlx v13,0,r8`, the
first of three in this function - `0x002a0bec` and `0x002a0db0` are the
others), a Cell extension stock Ghidra's sleigh does not decode; the
disassembler halted at the word and never resumed, so the function had
1,772 of its 3,192 bytes and the decompiler was reporting a genuine hole
in the *listing*, not a misreading of the bytes. `disassemble_bytes` read
past it only because that tool decodes one instruction at a time and skips
what it cannot decode. The program was reimported on 2026-09-15 under a
language that decodes `lvlx`, and the function now decompiles whole with no
warning - see [toolchain.md#ps3](../../../reverse-engineering/toolchain.md#ps3),
"Some Cell vector instructions are missing", for the mechanism and the
count (851 sites, 294 functions), and "Ninth session" below for what the
complete decompile says. And the shared global both `EngineFlare_RenderTick`
and `Trail_RenderTick` read (`0x00AEA540`) is the tuning file's own runtime
storage, live-confirmed by exact value.

**What the middle does, read at the instruction level.** Past the two
already-known gates it computes a normalised inverse-length from a vector at
`(this+0x38)+0x20` against the camera-context basis `FUN_00679278` returns -
the `vrsqrtefp` / Newton-Raphson-refine sequence a distance or 1/distance
term takes - extracts one lane to a scalar (`f25`), then runs a **linear
fade**: `saturate((f30*f28 - f23) / f24)`, inverted (`1 - x`), multiplied by
`f25` and by a per-instance value at `this+0x194` (or a shared `1.0` default
when a flag is clear) times a global scale, clamped to at most one further
global float, and the result stored at `this+0x18c` right before an
early-out (`ble` if `<= 0`, skipping the rest of the function).

**The live dump (`scripts/hd-flare-tuning-dump.py`) confirms which tuning
rows `f23`/`f24` are.** `0x00AEA540` is a pointer *variable*, not the struct
base - a first run of the script skipped the second dereference
(`lwz r9,0x5a48(r2)` resolves the TOC slot to this address;
`lwz r9,0x0(r9)` loads the pointer it actually holds, a heap allocation whose
address moves between runs) and read every offset as `0.0`. Fixed,
`struct_base + 0x460`..`0x4b4` matches the tuning file's remaining unread
rows almost one for one, live and exact
(`data/reference/hd-capture/flare-size/tuning-dump/`). **The row column
below was assigned by value and is wrong wherever two rows share a value;
"Ninth session" re-derives it from the whole dump and the file's row order
(rows 14-41 sit at `+0x44c`..`+0x4b8` in file order, 4 bytes each) - the
corrected label is in bold after each wrong one, and that section's table
is the one to cite:**

| Offset | Live value | Tuning row |
| --- | ---: | --- |
| `+0x464` | 0.25 | `Engine Flare Particles Min Alpha` - **wrong, `Tex Scroll Speed Thrust Contrib`; `Min Alpha` is `+0x44c`** |
| `+0x46c` | 0.01 | `Shockwave Cycle Speed` |
| `+0x470` | 32.0 | `Flare Highlight Power` |
| `+0x474` | 1.0 | ambiguous - `Flare Highlight Boost`, `Enable Engine Flare Particles` and `Flare Occluder Radius` are all `1.0` - **settled: `Flare Highlight Boost`** |
| `+0x480` | 2.0 | `Spikes Boost Max Scale` - **wrong, `Flare Radius Min`** |
| **`+0x484`** | **15.0** | **`Flare Fadeout Dist` - this is `f23` in the formula above** |
| **`+0x488`** | **15.0** | **`Flare Fadeout Range` - this is `f24`** |
| `+0x490` | 1.5 | `Spikes Thrust Max Scale` - **wrong, `Max Chromatic Dispersion`** |
| `+0x4a8`/`+0x4ac` | 1.0 | ambiguous, same three-way tie as `+0x474` - `+0x4a8` is `f26`, the clamp ceiling the fade result is `min()`-ed against right before the store - **settled: `+0x4a8` is `Flare Opacity Max`, `+0x4ac` is `Flare Occluder Radius`** |
| `+0x4b0` | 50.0 | `Flare Size Clamp` |
| `+0x4b4` | -0.46 | `Flare Depth Bias` |

This settles two things at once: **the leading hypothesis from session one is
now structural, not just numerical** - `Flare Fadeout Dist`/`Range` are read
at exactly the offsets the disassembly consumes in a `saturate()`-shaped
linear fade, not merely present somewhere in a block that also happens to
hold them. And **`Flare Size Clamp` (50.0) is not what clamps the fade
result** - that clamp uses `f26` at `+0x4a8`, which reads `1.0` live, so the
"order-of-magnitude gap against `Flare Radius`" reading from session one's
writeup does not hold up; `+0x4a8` is more likely `Flare Occluder Radius`
reused as a generic ceiling, or a distinct always-`1.0` field the three-way
tie cannot separate. `Flare Size Clamp` itself is read from the tuning file
into this struct (confirmed, `+0x4b0` = 50.0) but this session found no
consumer for it.

**What this session could not settle: whether any of this executes for the
sprite that is actually on screen.** `this+0x18c` - the fade's own output,
stored right before the early-out - read `0.0` across four samples taken a
half-second apart, from a craft confirmed mid-race, thrusting, and visibly
flared in the same session's own screenshot
(`data/reference/hd-capture/flare-size/tuning-dump/race.png`). Both gates
this session could check read as passing: the byte flag at `struct_base +
0x494` was `1` (live), and the count-style field at `craft+0x5fa4` was `1` -
which looks like it should fail the `ble` gate at face value, but that
compare is **unsigned** (`cmplwi`), so `1 - 4` wraps to `0xfffffffd` and
reads as far greater than `2`, meaning the gate is skipped only for
`craft+0x5fa4` in `{4, 5, 6}` and passes for everything else including `1` -
not the `count >= 7` reading session three's writeup implied. With both
checked gates open, `this+0x18c` should have taken a nonzero value at some
point in four samples if this is the code path that draws the visible
sprite. It did not. Two readings, neither confirmed: `EngineFlare_RenderTick`
is not the branch that reaches the draw call for this craft this session
(a third, untraced gate, or the wrong vtable slot entirely), or it is the
right function but the fade genuinely evaluates to `0` at the sampled
distance and `this+0x18c` feeds something other than the visible quad's
size or alpha (an occlusion-query parameter would fit `+0x4a8`'s likely
identity as `Flare Occluder Radius` just as well as a fade ceiling does).
**Settling this needs a breakpoint trace, not more reading** - a `Z0`
breakpoint at `EngineFlare_RenderTick`'s entry only fires under `PPU
Decoder: Interpreter (static)`
([rpcs3-debugger.md](../../../reverse-engineering/rpcs3-debugger.md#z0-breakpoints-fire-but-only-under-the-interpreter)),
about 75s to boot against the default recompiler's 30s, not attempted this
session.

### Fifth session: a real result (the entry fires), and a wrong reading of a second one

**2026-09-02, same day.** `scripts/hd-flare-rendertick-break.py` (new) ran
`EngineFlare_RenderTick` under a breakpoint trace for the first time.
**What actually holds**: an armed breakpoint at the function's own entry
(`0x002a08a8`) fired on the very first 0.4s poll of a driven, thrusting
race - real runtime verification, not decompilation, that the function
executes essentially every frame rather than being dead code gated
permanently off.

**What this session got wrong, caught on review rather than in-session**:
it then armed `0x002a1110` believing that address to be the shared landing
for the function's two early-out *gates*, and read 12 of 12 hits landing
there (across two races) as "the gate stayed closed both times." **`0x002a1110`
is not a gate landing - it is the function's one epilogue**, the register
restore and `blr` every path returns through, including the full-work path:
`0x002a1434: b 0x002a1110` sits right after the RSX submission block
(`0x002a1210`-`0x002a1424`, the `bl`s to `0x00679278`/`0x006792a8`/
`0x006792b8`/`0x00323aa0`/`0x006792c8` that reach the RSX submission
trampolines - none of the five has been read far enough to confirm it
issues a *draw* rather than, say, an occlusion-query dispatch; see
"Seventh session" below). A
breakpoint there fires on *every returning call*, successful or not - "12 of
12 hits" is equivalent to "12 of 12 calls returned," which was never in
doubt and says nothing about which branch ran. The doc previously claimed
this as a measured finding; it was not one, and the "which gate is closing"
framing this session left as the open question was chasing a distinction
this data cannot make.

**What does still stand, read correctly**: the *other* breakpoint,
`0x002a0d78` (right after `stfs f13,0x18c(r31)` and its own `ble`, so only
reached when the just-computed fade value is `> 0`), recorded zero hits in
the same runs. That is consistent with either a gate closing before the
fade math runs at all, *or* the fade math running every time and evaluating
to `<= 0` - two different findings this session's breakpoint placement
cannot tell apart, and next session needs a ladder of checkpoints rather
than the two endpoints used here to do it: has the fade math started at all
(`0x002a0bb8`, right where the camera-relative length computation begins),
has the store itself executed (`0x002a0d70`, the `stfs` instruction, fires
regardless of what value it stores), and only then whether it was positive
(`0x002a0d78`).

**A separate, real bug in the same script also undermines the "zero hits"
count specifically**: `step_off_breakpoint` resumes, pauses and re-arms
without checking which address the thread is actually stopped at - a hit
on the *other* armed breakpoint landing inside that 0.05s window would be
silently consumed and never tallied. So even "zero `0x002a0d78` hits" is
not fully trustworthy as measured; it wants a rerun with the fix.

**The GDB-stub trap the script found is unaffected by any of this and still
stands**: resuming a thread parked exactly on an armed breakpoint does not
step over it first - the thread never visibly advances, the stub never
sends a further stop reply, and the next unrelated command hangs for a full
socket timeout, reading exactly like a dead emulator. It cost two full
crashed runs before the pattern was clear (remove the breakpoint, resume
briefly, pause, drain, re-arm) and still recurs occasionally even with that
fix - the trigger is not settled, only that a script built on this API
should assume it will happen and save partial results rather than lose a
whole race's worth of hits, as the script now does (`stub_desync` in its
`meta.json`). Worth adding to
[rpcs3-debugger.md](../../../reverse-engineering/rpcs3-debugger.md)'s own
trap list once a session characterises it further.

**The corrected rerun, same session: zero hits anywhere inside the fade math
in 45 seconds of active racing.** A rewritten `hd-flare-rendertick-break.py`
arms three points *inside* the fade rather than its two endpoints -
`0x002a0bb8` (the camera-relative length computation begins, confirming the
fade math runs at all), `0x002a0d70` (the `stfs f13,0x18c(r31)` store
itself, fires whatever value it computed), `0x002a0d78` (only if that value
was `> 0`) - and also fixes the swallowed-hit bug in `step_off_breakpoint`
(it now checks which address the thread is actually stopped at before
re-arming, rather than assuming). **None of the three fired once** in a
45-second window against a craft confirmed thrusting, racing and visibly
flared in the run's own screenshot
(`data/reference/hd-capture/flare-size/rendertick-break-v2/race.png`) - a
materially longer and more sensitive observation than the flawed first
attempt, since 45s at this function's every-frame call rate is on the order
of 2,500 invocations, all of which apparently return before `0x002a0bb8`.
This is now a real, clean finding, distinct from the discredited "12 of 12"
one it replaces: **for this craft, `EngineFlare_RenderTick` consistently
exits before the fade math it computes ever runs at all** - not "runs and
evaluates to `<= 0`," but never entered. Which of the gates before
`0x002a0bb8` is responsible (the byte-flag check at `struct_base+0x494`,
the count-field check at `craft+0x5fa4`, or something else entirely between
`0x002a08a8` and `0x002a0bb8` this thread's sessions have not yet isolated)
is the question the next session inherits, and now has firm ground to stand
on: the fade math is not running, not "might not be running."

### Sixth session: the third gate, found and confirmed live

**2026-09-02, same day.** Both known gates measured open (five live samples
across `count_field_ble` and `byte_flag_beq`, all "would not skip" - see
below), yet the fade math still never ran. Re-reading the stretch between
the byte-flag gate (`0x002a0974`) and the fade math's own start
(`0x002a0bb8`) found a third branch neither prior session had traced:

```
002a0b34: bl  0x006765e8            ; a boolean query, args none
002a0b3c: rlwinm r3,r3,0x0,0x18,0x1f
002a0b40: cmpwi cr7,r3,0x0
002a0b44: beq cr7,0x002a1210        ; false -> a DIFFERENT code path, not the epilogue
002a0b48: lwz r11,0x134(r31)        ; true -> falls into the r10 computation,
...                                  ; itself gated again at 0x002a0bb4 before
002a0bb8: ...                        ; finally reaching the fade math
```

**`scripts/hd-flare-gate-check.py` (new) breakpoints each gate individually
and decodes `cr7` at the hit, rather than inferring the outcome from a
separate memory poll.** Single-breakpoint runs proved markedly more stable
than arming several at once (two- and three-breakpoint attempts got zero
hits before an immediate stub desync; single ones reliably got several).
Measured, `PPU Decoder: Interpreter (static)`, switched back after:

| Gate | Address | Samples | Result |
| --- | --- | --- | --- |
| `count_field_ble` | `0x002a0960` | 1 | open (`GT` set - does not skip) |
| `byte_flag_beq` | `0x002a0974` | 4 | open (`EQ` clear - does not skip) |
| **`alt_path_beq`** | **`0x002a0b44`** | **2** | **closed - `EQ` set, branches to `0x002a1210` every time** |

**This is the gate.** `FUN_006765e8()` (a two-instruction trampoline into
`0x003c7dd8`, which compares two 8-byte fields of a global at
`PTR_DAT_008b7838` and, only if the first is less than the second, reads a
byte flag at `+0x38` of the same object) returns false for the sampled
craft on both calls caught, sending execution to `0x002a1210` - genuinely
different code, not the shared epilogue this thread mistook a different
address for in "Fifth session" above. `FUN_006765e8`'s other call sites -
`Game_PresentLoop_q` (three calls) and `PhotoMode_Update` (three calls), no
other caller anywhere in the image - are consistent with a **per-frame
budget or throttle check** (the two 8-byte fields reading like a spent/limit
pair), though that is a hypothesis from the call-site pattern and the
comparison shape, not a traced value.

**What `0x002a1210` itself does is not fully resolved, and it changes the
picture again.** It is not dead weight or a simple bail-out: past two more
gates of its own (`bl 0x006765f8` and a byte flag at `struct_base+0x5aac`,
both of which can loop **back** to `0x002a0b48` - the same entry the r10
computation uses, so this is not strictly one-way), it builds a small
ring-buffer entry (`craft+0x270`, modulo 3, matching the same indexing
shape `0x002a1234`-`0x002a12b4` used before) and reaches the same family of
RSX-submission trampolines (`0x00677ff8`, `0x00679298`, `0x00679288`) this
thread already traced as far as `Rsx_SetMethod`. **So the function has (at
least) two branches that both eventually touch the RSX pipeline - the fade
math this thread has been tracing, and this one - and which of them (if
either) actually builds the visible sprite's quad is now the open question,
not simply "does the fade math run."** A live read of `0x002a1210`'s own
work - whether it touches the sprite's radius, the occlusion query, or
something this thread hasn't named yet - is next.

The user's report also suspected the trail itself, and named two possible
splits: a different trail between classic HD and Fury, and a Zone-race-
specific one. Both checked and refuted, disc-wide:

- **No second trail asset exists.** All seven `DATA0N.PSARC` archives list
  exactly one live trail stem, `enginetrail_bluered_triangle` (`DATA06`),
  plus the dead `enginetrail_triangle` (`DATA02`, already established as
  unreferenced by the executable). No `zone`- or `fury`-prefixed trail file
  exists anywhere on the disc - `grep -i "zone\|fury"` over every archive's
  trail-matching entries comes back empty. HD and Fury share the one asset;
  what differs is the `engineTrail` colour-mix parameter (`0xbb48e390`) the
  Fury-skin byte patches, already read and implemented.
- **The trail's own material declares no Zone parameter.** HD's Zone-mode
  environment recolour is a live, separate, unsolved thread
  ([zone-effectsettings-loader.md](zone-effectsettings-loader.md)) with its
  own 16-name parameter block (`zoneColourTint`, `zoneEffectInner/Outer`,
  `zoneBaseInner/Outer`, `zoneBaseAltInner/Outer`, `zoneOrigin`,
  `zoneTexInner/Outer` and their `Nearest` pairs, `zoneTexVis`,
  `zoneAnisoPalette(Outer)`, `zoneAnisoPower`) whose hashes are already
  known (renderer.md). Both `hd_enginetrail_bluered.rcsmaterial` and
  `flame_test.rcsmaterial`'s raw bytes were checked against all 16 hashes,
  little- and big-endian: zero hits in either. So if a Zone race's exhaust
  genuinely reads different on the original, it is not this material
  sampling zone state directly - consistent with renderer.md's own finding
  that the Zone recolour is track/scenery-side and still unlocated, not
  something to re-derive here. `oag_render`'s exhaust pipeline
  (`exhaust.rs`/`.wgsl`) carries no zone awareness at all, same as the disc
  material it reproduces - not a gap against this material, though it would
  need one if the still-open Zone mechanism turns out to reach ships too.

**The finding this session is confident of, screenshot to screenshot: this
engine's sprite flare is drawn much larger, relative to the hull, than the
original's.** `data/reference/hd-capture/flare-size/` holds the pair -
`original-rpcs3.png` (`scripts/rpcs3-drive.py race --drive 15 --shots`, a
driven Talon's Junction lap at 154 km/h) against `ours.png` (`just play hd
--race --press cross --ticks 300`, same track, comparable chase distance and
craft framing). The original's core is a compact glow tucked at each nozzle,
narrower than the hull; this engine's is one white disc that spans past both
nozzles and reads as roughly the width of the hull itself. **Ruled out as the
cause**: a second locator - `crates/livery/src/lib.rs`'s `engine_flare`
already establishes every checked team's hull authors exactly one `Engine
Flare` node, so the single-sprite-per-craft shape in
`Race::hd_sprite_quad` matches the disc's own locator count and is not
collapsing two nozzles into one. The constants themselves are also not the
cause: `the_sprite_flares_constants_are_the_discs_own` pins `SPRITE_RADIUS`
(3.0), `SPRITE_RADIUS_MIN` (2.0) and `SPRITE_RADIUS_JITTER` (0.5) against
this same file, byte for byte.

### Seventh session: the fade branch is geometrically ruled out, and the size question moves to the renderer

Prompted by review: the sixth session's "which branch draws the sprite"
framing was itself the wrong question to keep pushing on with more Ghidra
tracing. Two things follow from evidence this page already had, neither
acted on until now.

**`Flare Fadeout Dist`/`Range` (15.0/15.0) cannot be what makes the
player's own flare oversized, regardless of which branch runs.** Both this
engine's own chase camera and the original's sit a few world units behind
the craft - nowhere near the 15-unit distance the `saturate()` fade (see
"Fourth session") starts biting at. A fade keyed to a 15-unit start is
fully faded-*in* (no reduction at all) at ordinary chase range whether or
not `EngineFlare_RenderTick`'s fade math ever executes for that craft. The
"Open" bullet below has called this "the leading candidate" since the
2026-09-01 screenshot comparison; that framing was wrong on the numbers
alone, not just unconfirmed - correcting it here rather than leaving it
stand.

**A second look at the four screenshots already on disc, not just the
one this page cited, confirms the size gap is real and repeats.**
`data/reference/hd-capture/flare-size/{original-rpcs3.png (first
session), tuning-dump/race.png (fourth), rendertick-break-v2/race.png
(fifth)}` are three independent RPCS3 captures, taken across three
sessions with different craft speeds and track sections, and all three
show the same shape: a small, tight, blue-white point of light tucked
directly between the two engine nozzles, no wider than roughly a sixth of
the hull's own span at that framing. `ours.png` (this engine, comparable
chase framing) shows a large white-to-yellow blob spanning most of the
gap between the side nacelles and bleeding below the hull silhouette -
different in extent, shape and colour, not merely brighter. Cropped
side-by-side comparison, not a pixel-diff tool (none of the four share a
camera angle or resolution closely enough for one): the three original
captures agree with each other and disagree with `ours.png` the same way
each time, which is what "the gap is real" needs to mean here, short of a
matched-view capture (see "Matched-view comparison" above for what that
takes).

**What actually sizes the sprite, per the renderer's own code, is a flat
constant with no distance term at all**: `oag_fx::exhaust::hd::SPRITE_RADIUS`
(3.0, jittered ±0.5, floored at 2.0) is applied unconditionally every
frame (`crates/raceplay/src/load.rs`'s own report already says so - "its
radius, alpha walk and texture do draw"). Nothing this page has traced in
`EngineFlare_RenderTick` writes that half-size; the function's own two
known consumers of the tuning block are the fade scale (gated off, per
the sixth session) and whatever `0x002a1210` does past the third gate.
Re-reading `0x002a1210` with this in mind: its shape - a
`craft+0x270`-indexed ring buffer entry (modulo 3) feeding the same RSX
submission trampolines the fade path's own tail does - matches an
occlusion query dispatched once per frame far better than it matches a
second draw path, and `Flare Occluder Radius`/`Flare Depth Bias` (both
already read live, "All 41 rows" above) are real tunables with no located
consumer anywhere else on this page. An occlusion query, whichever of
`0x002a1210`'s branches it turns out to be, does not set a quad's size -
so confirming that reading would close the "what does 0x002a1210 draw"
question without touching the size question at all.

**This reframes the open item.** It is not "which branch draws the
sprite" (neither branch this page has found sizes it) and it is not "read
the fade math" (ruled out by the fade's own numbers). It is: what, if
anything, in the original scales `Flare Radius` by anything other than the
`Max Radius Jitter` band this engine already applies - and if nothing
does, then 3.0 half-size at this craft's own scale is either what the
original actually draws (in which case the size gap is a renderer-side
bug: wrong units, wrong hull-to-flare scale ratio, or a missing
occlusion/depth clip against the craft's own mesh that the original gets
for free and this engine does not) or the tuning row's consumer is a
different field than the one currently believed to be `Flare Radius`. Both
halves are renderer/asset questions from here, not further disassembly of
`EngineFlare_RenderTick` - see "Next Steps" on the handover thread.

### Eighth session: it is the sprite after all - a flawed metric said otherwise first

Following the seventh session's redirect to the renderer, this session
tried to attribute the blob to a specific draw call by disabling each of
this craft's additive layers in turn (`crates/raceplay/src/scene/frame.rs`,
one `continue` at a time, same deterministic 300-tick capture each time -
`just play hd --race --press cross --ticks 300 --screenshot`) and counting
bright pixels (`RGB > 200,200,200`) in a fixed crop around the engine gap.
**That metric was wrong, and it said so with false confidence**: disabling
the sprite (`hd_sprite_quad`) moved the count from 7472 to 6480 (~13%),
halving its radius barely more (6662), and disabling the flame mesh
(`self.flares`, the `engineflare.rcsmodel` drawable) alone accounted for
nearly as much (7244) - reading as "the sprite is a minor contributor, look
at the mesh instead." Both numbers were real; the conclusion from them
was not. Several additive layers (the sprite, the flame mesh, the tube's
own bloom) already saturate the same pixels past 200 independently in this
region, so removing any *one* rarely drops many pixels back under the
threshold even when that one layer's actual extent is enormous - a
saturating threshold cannot see through an already-blown-out area.

**A pixel-diff isolation, not a threshold, settles it.** For each layer,
subtracting a render with it disabled from the same deterministic tick's
render with it enabled gives that layer's *own* additive contribution,
independent of what else saturates the same pixel - and thresholding *that
difference* at a low, non-saturating level (`> 30`, not `> 200`) finds its
real footprint. Boost plume: **zero** pixels changed, byte for byte
(`plume_visible()` is false for the sampled slot this run - cleanly ruled
out, not just quiet). Flame mesh disabled: 1,954 pixels changed inside a
340x280 crop, a small, tight, roughly-circular difference right at the
nozzle. **Sprite disabled: 20,838 pixels changed** - a difference image
(`data/reference/hd-capture/flare-size/sprite-only-diff.png`, not
committed, gitignored like the rest of `data/reference/`) shows a bright
cone-shaped glow reaching from the nozzle almost to the cockpit and
spilling onto the track below the hull. **The sprite is the dominant
contributor after all** - the opposite of what the threshold-based count
said, and the reason it was wrong is now on this page rather than in a
committed doc claim, because the mistake was caught before writing it down
rather than after.

**The `CRAFT_ROW_SCALE` hypothesis the seventh session tried and read as
"no visible change" is, measured this way, real and substantial.**
Isolating the sprite's own footprint (same subtraction, against a render
with the sprite fully disabled) at three radii, same tick, same crop:

| Radius | Footprint (px > 30) | Footprint bbox | Total additive contribution |
| --- | --- | --- | --- |
| 3.0 (unscaled) | 15,684 | 189x220 | 1,928,035 |
| 2.25 (`* CRAFT_ROW_SCALE`, 0.75) | 8,568 (-45%) | 157x191 | 1,120,181 (-42%) |
| 1.5 (flat half) | 4,530 (-71%) | 149x100 | 501,944 (-74%) |

Both cuts are real and roughly monotonic with radius - not the "invisible
either way" the naked eye read off two blown-out crops. **Implemented**:
`race::effects::hd_sprite_quad` now scales `sprite.radius()` by
`exhaust::CRAFT_ROW_SCALE` before building the quad, the same factor
`model_matrix_of` already applies to the sprite's own *position* via
`nozzle_of` - so the radius lives in the same space the position does,
rather than one scaled and the other not.
`oag_fx::exhaust::hd::SPRITE_RADIUS`'s doc comment is corrected to say
so. This is not confirmed against the original's own scale factor for this
specific field (no live or Ghidra evidence that the original divides
`Flare Radius` by the same 0.75 rather than by something else, or nothing
at all) - it is an internal-consistency fix, justified by every other
per-craft magnitude in this file needing the same factor
(`TRAIL_DIRECTION_SCALE`'s live-confirmed `200000 * 0.75 = 150,000`
precedent) and now empirically measured to move the footprint a real 45%
in the right direction, not asserted to close the gap outright: the 1.5
row above shows there is room to go further if 0.75 alone is not enough
once checked against the original's own measured proportions.

**The calibration against the original's own proportions, done next, found
a second measurement trap and a real but partial result.** The obvious way
to check "does 0.75 make this look right" is a brightness threshold on the
final composited frame, matching each glow's bbox against the engine bay's
own width (nozzle-to-nozzle) - the same kind of measurement "Seventh
session" used across three screenshots. It does not work cleanly on either
side: `RGB > 170` on `original-rpcs3.png`'s engine bay returns a
93x60-pixel box against a 95x65 box (bathtub-full - because the metal
turbine housings either side of the actual glow carry their own bright
specular highlights, and the threshold cannot tell a glowing plasma core
from a shiny cowling). The same threshold on this engine's own frame
returns a box pinned to whatever crop margin is given, for the same
reason plus the hull's own white livery paint at the fuselage edges. A
threshold that cannot separate "flare" from "everything else bright
nearby" cannot calibrate against one either - this is a second, distinct
measurement trap from the "saturating threshold across overlapping
additive layers" one above, hitting a *single*, non-additive frame this
time (the original's, and this engine's own composited output, not an
isolated diff).

Falling back to reading the *glowing, blue-tinted core* out of a zoomed,
gridded crop by eye - not thresholded, but a human distinguishing plasma
glow from metal specular the way the threshold could not - and comparing
its width to the engine bay's own nozzle-to-nozzle span in the same crop:

| Capture | Glow width | Engine-bay width | Ratio |
| --- | --- | --- | --- |
| `original-rpcs3.png` | ~36px | ~85px | ~0.42 |
| this engine, unscaled (`SPRITE_RADIUS` unmultiplied) | ~125px | ~170px | ~0.74 |
| this engine, `* CRAFT_ROW_SCALE` (landed) | ~105px | ~170px | ~0.62 |

By this reading the landed fix closes roughly a third of the visual gap
(`(0.74-0.62)/(0.74-0.42) ≈ 0.38`), not all of it - matching the pixel-diff
footprint numbers' own message rather than contradicting them. Reaching
the original's ~0.42 ratio from unscaled's ~0.74 would need close to
`0.42/0.74 ≈ 0.57` of the current radius (≈1.7, near the flat-half row's
1.5, already measured to overshoot toward ~0.3 the other way) - **and that
number is not implemented**, deliberately: it is fitted to two screenshots
read by eye, not to any traced consumer in the original, and this project
elsewhere holds the opposite standard on purpose (`post::bloom`'s own
module doc: "every constant here is read out of the executable; none is
fitted to a screenshot"). Landing it would trade one honestly-labelled
partial fix for one invented-looking exact one. What closes the rest
honestly is either Ghidra evidence for what the original itself multiplies
`Flare Radius` by (nothing this page has read names a consumer for it, per
"the original's own scale factor for this specific field" above), or an
explicit, disclosed decision to fit a value anyway - which is a call for
whoever picks this thread up next, not one to make silently mid-session.

### Ninth session: the complete decompile - the quad, its size law, and why the fade never ran

**2026-09-15.** The program was reimported under a language that decodes
`lvlx` (see the correction at the top of "Fourth session"), and
`EngineFlare_RenderTick` decompiles whole: 3,192 bytes, `0x002a08a8` to
`0x002a1524`, no bad-instruction warning, zero unreachable blocks. Read
against the disassembly, with the TOC constants read off `0x008ad4d8`
(the function sits below `0x32d5e0`, so Ghidra's TOC is the right one -
[memory.md](memory.md)) and the tuning block re-labelled from the archived
live dump, it answers four things this thread had open and narrows the
fifth to one breakpoint.

**1. The tuning block is the file in row order, and three earlier labels
were wrong.** `data/reference/hd-capture/flare-size/tuning-dump/global_block.bin`
is `struct_base + 0x420`..`0x4cf`, dumped 2026-09-02. Rows 14-41 of
`shipeffectstweaks.txt` sit at `+0x44c`..`+0x4b8` in the file's own order,
four bytes each, and every one of the 28 words matches: `+0x44c` 0.25 `Min
Alpha`, `+0x450` byte 1 `Enable Particles`, `+0x454`..`+0x468` the six
`Tex Scroll` rows (0.08, 0.1, 0.0, **1000.0**, 0.25, 0.6), `+0x46c` 0.01
`Shockwave`, `+0x470` 32.0 `Highlight Power`, `+0x474` 1.0 `Highlight
Boost`, `+0x478` **6.283185** `Max Rotate Angle`, `+0x47c` 3.0 **`Flare
Radius`**, `+0x480` 2.0 **`Flare Radius Min`**, `+0x484`/`+0x488` 15.0
`Fadeout Dist`/`Range`, `+0x48c` 0.5 **`Max Radius Jitter`**, `+0x490` 1.5
**`Max Chromatic Dispersion`**, `+0x494` byte 1 **`Enable Flare Sprite`**,
`+0x498` int **10** `Slow Alpha Noise Timer`, `+0x49c`/`+0x4a0`/`+0x4a4`
0.5/0.8/0.1 the `Slow Alpha Noise` band and chase, `+0x4a8` 1.0 **`Flare
Opacity Max`**, `+0x4ac` 1.0 **`Flare Occluder Radius`**, `+0x4b0` 50.0
`Size Clamp`, `+0x4b4` -0.46 `Depth Bias`, `+0x4b8` byte 1 `Enable Engine
Trails`. Five of those values are unique in the file (1000, 6.283185, 50,
-0.46, 0.01) and three are integers landing on words the code loads as
integers (`lwz r21,0x498(r9)`, `lbz r0,0x494(r9)`), so the mapping is
overdetermined - **confidence 92**. The head of the block (`+0x420`..
`+0x448`) is *not* in file order (`+0x42c`/`+0x430` are the `Spikes` pair,
`+0x444`/`+0x448` the `Afterburner` pair reversed) and no claim is made
there beyond what the values pin - `+0x42c`/`+0x430` read 1.5/2.0 and
*could* be the `Spikes` pair, but 1.5 also sits at `+0x420` and 2.0 at
`+0x428`, so that is a value guess of exactly the kind this paragraph is
correcting, not a mapping. Consequences: the fourth session's
`+0x480` = `Spikes Boost Max Scale` and `+0x490` = `Spikes Thrust Max
Scale` were value coincidences (2.0 and 1.5 each appear twice in the file);
the byte-flag gate at `+0x494` this thread has been calling "a byte flag"
is `Enable Flare Sprite` itself; and the three-way `1.0` tie collapses.

**2. This function builds the sprite's quad - the three-session hunt for
"the function that turns `Flare Radius` into a world-space quad" is over.**
Past the fade store the decompile is unambiguous:

```
002a0d78: addi  r3,r26,0x250
002a0d7c-88: fsel x2           ; a = clamp(fade, 0.0, 1.0)
002a0d8c: lfs   f13,0xc(r3)    ; this+0x25c, a per-instance 0..1
002a0d94: fmadds f13,f21,f13,f19   ; MaxRadiusJitter * this[0x25c] + FlareRadiusMin
002a0dc8: fmadds f12,f18,f12,f13   ; FlareRadius * a + that
002a0dd4: stfs  f12,0x1e0(r1)  ; half-height, world units
002a0df4: vmaddfp v5,v29,v7,v6 ; cam_row0 * half
002a0dfc: vmaddfp v7,v28,v7,v6 ; cam_row1 * half
002a0e00: vmaddfp v5,v5,v10,v6 ; * 4.0  (v10 = splat of *(r2+0x5aa4) = 0x40800000)
```

then four corners `pos - v5 - v7`, `pos + v5 - v7`, `pos + v5 + v7`,
`pos - v5 + v7`, each multiplied through the 4x4 at `FUN_00678f68()`
(`*0x008b713c`, the camera object's first matrix - world-to-clip by use),
converted lane by lane with `FUN_00677cb8` (float to half) and stored as
three `sth` at `this+0x274/6/8`, `+0x284/6/8`, `+0x294/6/8`, `+0x2a4/6/8`
- the same 16-byte stride `EngineFlare_Init` seeds the UV pairs into at
`+0x280..+0x2b2`, so the vertex is `{half3 position, pad, u32 colour, half2
uv}` - "position" in whatever space that first matrix maps to, clip by the
65-confidence reading above. The colour word at `+0x27c/+0x28c/+0x29c/+0x2ac` is
`0xffffff00 | alpha_byte`, where `alpha_byte = (int)(fade * 255.0)`
(`*(r2+0x5ab4)` = `0x437f0000`), `0xff` if `fade > 1`. Then
`FUN_002c4ad0(MaxChromaticDispersion * (1.0 - view_dot), *(r2+0x5aa8),
this+0x1a0, this+0x274, 4, 2)` submits it: four vertices, the texture handle
at `this+0x1a0`, and a dispersion amount that grows as the view leaves the
nozzle axis. Decompiler line for the submit:
`_opd_FUN_002c4ad0((double)(float)(dVar35 * (double)(float)((double)fVar2 - dVar50)),uVar3,*(undefined4 *)(param_1 + 0x1a0),(undefined2 *)(param_1 + 0x274),4,2);`
with `dVar35` = `+0x490` and `dVar50` the view dot. **Confidence 88** for
"this is the sprite's own draw" (the vertex block, the UV seeding from
`Init`, the texture handle and the four-vertex submit all sit on the same
object); the matrix's identity as world-to-clip is by use only, 65.

So **the size law is** `half_height = Flare Radius Min + Flare Radius *
clamp(fade, 0, 1) + Max Radius Jitter * jitter01`, with `half_width = 4 *
half_height`. The `4.0` at `0x008b2f7c` scales the camera's row-0 axis only
and matches the texture: `Engine_Flare_Rich.gtf` is **1024 x 256** (GTF
header, `DATA02.PSARC`, format `0x86`, 11 mips), a 4:1 streak, so the quad
keeps the texel aspect rather than squashing it. `oag_fx::exhaust::hd`'s
`RADIUS +- rand * JITTER, floored at MIN` is a different law (its own doc
says the law was unread; it now is), and `exhaust::sprite` draws a square.
Neither is changed by this docs-only session - see the handover thread.

**3. `this+0x18c` is the sprite's alpha, and the fade reading is confirmed
with two terms the raw-byte reading missed.** The decompiler's own
expression, variables named after the offsets they load:

```
dVar44 = (double)FUN_00677868(dVar50,dVar47);                      // powf(view_dot, +0x470 = Flare Highlight Power)
dVar41 = (double)(float)((double)(float)(dVar49 * dVar46 - dVar41) / dVar42);   // (dist * k - +0x484) / +0x488
dVar38 = dVar48; if ((float)(dVar41 - dVar48) < 0.0) dVar38 = dVar41;   // min(x, 1.0)
if (dVar38 < 0.0) dVar38 = dVar45;                                  // max(., 0.0)  -> saturate
dVar38 = (double)((float)((double)(float)(dVar48 - dVar38) * dVar44) * (float)(dVar51 * dVar40));
                                   // (1 - saturate) * powf(...) * (alpha_walk * +0x474 Flare Highlight Boost)
if ((float)(dVar43 - dVar38) < 0.0) dVar38 = dVar43;                // min(., +0x4a8 Flare Opacity Max)
*(float *)(param_1 + 0x18c) = (float)dVar38;
```

`FUN_00677868` is a TOC-fixup stub onto `0x0042a568`, which Ghidra already
names `powf` (the `_FDunscale`/`_FLog`/`_FExp` body of a libm pow).
`dVar48`/`dVar45` are `*(r2+0x5a1c)`/`*(r2+0x5a18)` = `0x008b2ef4`/
`0x008b2ef0` = 1.0/0.0. `dVar49` is a **distance**, not an inverse length
- `0x002a0d04: vmaddfp v1,v1,v0,v7` multiplies the squared length by its
refined `rsqrt`, i.e. `len`, of `flare_pos - cam_row3` - and `dVar46` is
`((float *)*(r2+0x5aa0))[max(view_index, 0)]`, a per-view table entry, not
a per-instance value. `dVar51` is `this+0x194` (the walked alpha; the
countdown at `this+0x190` reloads from `+0x498` = `Slow Alpha Noise Timer`
and re-targets `this+0x198 = lerp(NoiseMin, NoiseMax, rand() * 2^-30)`,
`rand` returning `& 0x3fffffff`) - or 1.0 when the camera object below is
in mode 3 or 4. `dVar50` is the view dot: the camera's row-2 axis and the
flare node's own Z axis (`*(this+0x38) + 0x20`), both normalised with a
zero-length fallback, negated on one side, summed - and the fade only runs
when it is positive (`0x002a0c8c: ble -> epilogue`), so the sprite is a
one-hemisphere `cos^32` lobe around the nozzle axis. Which hemisphere in
world terms depends on two sign conventions not settled here. So:
`fade = min((1 - saturate((dist * k - 15) / 15)) * powf(dot, 32) * alpha_walk * 1.0, 1.0)`,
stored at `+0x18c`, then used as the vertex alpha and as the size's
`clamp(fade, 0, 1)` term. **Confidence 90** - the raw-byte reading's
`1 - saturate((f30*f28 - f23)/f24)` is confirmed, and the fourth session's
alternative reading (b), "`+0x18c` feeds an occlusion-query parameter", is
refuted: it feeds the colour word and the half-height, nothing else. The
seventh session's geometric point stands as stated: at chase range the
distance term is 1.0, so it is the `powf(dot, 32)` factor, not the
distance, that keeps a drawn sprite small and dim off-axis.

**4. `0x002a1210` is the occlusion query, traced rather than guessed, and it
rejoins the fade path.** In order: `FUN_006765f8()` (stub onto
`0x003c81a0`, `(A[0x4c] == 0) && (A[8] < A[0]) && A[0x39]` on the object at
`*0x008b7838`) false, and the byte at `**(r2+0x5aac)` set; read ring slot
`(this[0x270] + 2) % 3` and, if it holds a report, `visible = report.value
!= 0` (`lwz r0,0x8(r9)` - `CellGcmReportData.value`, the z-pass pixel
count), else `visible = 1`; advance `this[0x270]`; take a 16-byte report
from the lwmutex'd pool at `0x005cc710`; zero it; `Rsx_SetMethod(ctx,
0x17cc, 1)` (`NV4097_SET_ZPASS_PIXEL_COUNT_ENABLE`), depth-write off via
`FUN_00678148(ctx, 0)`, colour mask `0,0,0,0`; draw the proxy with
`FUN_006792c8(r, r, r, 1, ctx, 0, node + 0x30, -1)` where `r` = `+0x4ac` =
**`Flare Occluder Radius`** (1.0) - a unit proxy scaled by the radius at the
flare node's position; `0x005c766c` writes method `0x1800`
(`NV4097_GET_REPORT`, `1 << 24 | slot`); then z-pass counting off,
depth-write and colour mask restored, and **`bne -> 0x002a0b48`** if
`visible` else return. `FUN_006765e8()` true skips the query entirely and
goes to `0x002a0b48` directly. So there is no second draw branch: the
third gate the sixth session found routes through a two-frame-latent
occlusion test and then rejoins the *same* fade path. **Confidence 88.**

**5. Why the fade never ran, narrowed to one gate - and it looks like the
function does not draw the viewing player's own sprite.** With the query
rejoining at `0x002a0b48`, the only thing between it and `0x002a0bb8`
(where all three of the fifth session's zero-hit breakpoints sat) is
`0x002a0bb4: beq -> epilogue` on `r10`:

```
002a0b54: lwz  r9,0x7a60(craft)      ; craft+0x7a60
002a0b58: cmpwi r9,-1 ; beq -> r10 = 1
002a0b60: lwz  r0,0x0(r25)           ; *0x008c1430, the view index (0/1 halve the; viewport at 0x002a0ad8-0x002a0b2c; < 0 skips that)
002a0b68: blt r0 < 0 -> r10 = 0
002a0b70-80: r10 = (r9 != r0)        ; xor / abs / sign-bit idiom
002a0b84-0ba4, 002a11b0-11d4, 002a14d0-14fc:
   if the camera object at **0x008b2eb4 has (+0x34 & 6) and its +0x1ec == craft:
       r10 = (mode = +0x40) > 11 || !((1 << mode) & 0x9c4)   ; 0 for modes 2,6,7,8,11
       modes 3 and 4 also force alpha_walk to 1.0 (r29)
```

`craft+0x7a60` is written in exactly two places in the image (`stw` sweep
over 1,829,837 instructions: `0x000de770` in `FUN_000ddd58` and
`0x000e07ac` in `FUN_000dfd90`, two craft constructors that register with
the collision world per [collision.md](collision.md)), both storing the
constructor's seventh argument. Its three callers pass: `FUN_0005cb40`
(the human-craft spawner - kind `0`, allocates the `0xa0` pad object with
the same index, stores the craft at `raceManager+0x13e8` when the index is
0) the **player index**; `FUN_00044a00` (kind `1`) the literal **-1**;
`FUN_00058e38` (kind `2`) a loop counter. So `+0x7a60` is the local player
that owns the craft, -1 for none, and `r10` reads as "this craft is not the
one the current view belongs to": always 1 for a `-1` craft, and for a
player's craft 1 only in another player's viewport or under a camera object
targeting it in a mode outside `{2, 6, 7, 8, 11}`. In the single-view races
every live sample so far was taken in, the player's own craft has
`+0x7a60 = 0` and a view index of 0 or -1, and either way `r10 = 0` -
**the fade math and the quad are skipped for the craft the camera belongs
to**, which is what 0 hits at `0x002a0bb8` over ~2,500 calls with both
earlier gates measured open already said. Two confidences, because two
different things are being claimed: the branch logic itself - `r10 =
(owner != view)` from the `xor`/`abs`/sign-bit idiom at `0x002a0b70`-
`0x002a0b80`, skipped at `0x002a0bb4` when equal - is mechanical, **90**,
and the `+0x7a60` identity rests on its two writers and three spawners,
**85**. What holds the *conclusion* at **75** is narrower: that
`*0x008c1430` is the current view index (read from the split-screen
halving at `0x002a0ad8`-`0x002a0b2c`, never measured) and what the
camera object's `0x9c4` mode set means. The fifth session's breakpoints
were also not filtered by craft, so whether an AI craft ever reaches
`0x002a0bb8` is not known either (an AI craft's `+0x7a60` is -1 by the
spawner above, so it should - unless its query read zero, unless
`craft+0x5fa4` was in `{4, 5, 6}` for it, or unless it was never enqueued).
What follows if it holds: the compact glow on the player's own nozzles in
all three original captures is the flame `.rcsmodel` and whatever `Engine
Flare Particles` draws, **not this sprite**, and this engine draws on the
player's craft a quad the original reserves for the other crafts. **The
one measurement that settles it** is not the branch outcome (the listing
already gives that) but the two values feeding it: a breakpoint at
`0x002a0bac` logging `*(*(r31+0x134)+0x7a60)`, `*0x008c1430` and `r31` on
every craft's call, plus one at `0x002a1428` logging `r28` (the query
result) - `scripts/hd-flare-gate-check.py` already has the shape, it
needs the register and memory reads added. **A cheaper check that needs no
breakpoint**: the reading predicts an asymmetry a single frame shows - no
sprite on the player's craft, a wide 4:1 streak on every opponent whose
nozzles face the camera within ~15 units. None of the three captures on
disc has an opponent nearer than the horizon (the player is in eighth
place in all three), so a start-grid frame - seven crafts bunched a few
units ahead, nozzles toward the camera - is the picture to take.

**What this function never loads**: `+0x478` `Flare Max Rotate Angle`,
`+0x4b0` `Flare Size Clamp`, `+0x4b4` `Flare Depth Bias`, and nothing
below `+0x470`. The rotate and the two occlusion-adjacent rows have a
consumer somewhere else or none; `FUN_002c4ad0` and the proxy draw
`0x005f0c10` are where to look, neither read this session.

Reproduce: `decompile_function 0x002a08a8` and `disassemble_function
0x002a08a8` with `program="/hdfury/EBOOT-ps3-hdfury-eu.elf"`; `read_memory
0x008b2ef0 160` for the TOC constants; `search_instructions
mnemonic=stw operand_pattern="0x7a60("` for the two writers;
`python3 scripts/psarc.py cat data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/DATA02.PSARC /data/tex/engineflare/engine_flare_rich.gtf | head -c 48 | xxd`
for the texture header; and the block decode is
`struct.unpack('>f', global_block.bin[off - 0x420:][:4])` per offset.

### Tenth session: the owner gate, measured - the player's craft is skipped every frame, all seven opponents pass

**2026-09-15, live on RPCS3 `0.0.42-19777` under `PPU Decoder: Interpreter
(static)`, audio `"Null"`, three boots of a Campaign single race on Talon's
Junction (seven AI opponents), `scripts/hd-flare-owner-break.py flare` and
`flare-gate`.** The ninth session left one gate between the occlusion query
and the fade math, `0x002a0bb4`, and held its reading - "the sprite is not
drawn for the craft the current view belongs to" - at 75 because its two
inputs had never been seen live. Both have now, on every craft, on every
frame of a 30-frame window, and the reading holds.

**What was armed and read.** One `Z0` breakpoint at a time. At
`0x002a0bac` (`rlwinm r0,r10,0,0x18,0x1f`, the instruction before the
`beq`), each hit read `r31` (the `EngineFlare`), `r10` (the value the branch
tests), `*(r31+0x134)` (the craft), `*(craft+0x7a60)` (the owner index),
`*0x008c1430` (the view index, `r25`'s target - TOC slot `0x008b2f64`), and
the camera object `*0x009878c0` (the ninth session wrote this as
`*0x008b2eb4`; that is the TOC slot, `lwz r9,0x59dc(r2); lwz r9,0(r9)`
dereferences it twice) with its `+0x34` flags, `+0x40` mode and `+0x1ec`
target. Two controls in the first boot: `0x002a1104`, the `bl 0x002c4ad0`
that submits the quad, reached only through the fade path; and
`0x002a1428`, where `r28` is the occlusion query's answer.

**Sampling every call needed a hop, and the first two boots show why.**
Moving a parked thread off the breakpoint with a 30 ms free run
(`run_for(0.03)`) lets the rest of the frame's flares go by, so the re-armed
breakpoint catches the *first* flare of the next frame almost every time:
boot 1 sampled one AI craft 152 times in 160 gate hits, boot 2 one craft 172
times in 214. `vCont;s` did not move the thread at all (0 of 213 attempts;
the thread's PC read the breakpoint address afterwards every time). What
works is arming `address + 4` and resuming - the thread executes one
instruction and parks again, the shape `hd-fury-backdrop-break.py` already
alternates its two addresses in - and boot 3 then caught all eight crafts
in a fixed order, 30 frames running:

| queue slot | craft | `+0x7a60` owner | `r10` at `0x002a0bac` | hits | `+0x190` countdown | `+0x18c` alpha | `+0x270` ring |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `0x33ebca00` | -1 | 1 | 30/30 | 10..1, cycling | 0.0 | advances |
| 2 | `0x3330ed70` | -1 | 1 | 30/30 | 10..1, cycling | 0.0 | advances |
| 3 | `0x33e39460` | -1 | 1 | 30/30 | 10..1, cycling | 0.0 | advances |
| 4 | `0x33fbe710` | **0** | **0** | 30/30 | **1, never moves** | **0.0, never written** | advances |
| 5 | `0x33f3e2d0` | -1 | 1 | 30/30 | 10..1, cycling | 0.002-0.003 | advances |
| 6 | `0x33db81b0` | -1 | 1 | 30/30 | 10..1, cycling | 0.0 | advances |
| 7 | `0x33284b50` | -1 | 1 | 30/30 | 10..1, cycling | 0.0 | advances |
| 8 | `0x3338f330` | -1 | 1 | 30/30 | 10..1, cycling | 0.0 | advances |

This 30-frame window is the start grid, not a spread across a lap:
`craft+0x5fa4` read `0` on every hit where boot 2 saw it go to `1` after
83 hits, and boot 1's `race-gate.png` shows `GO` at 41 km/h on lap 1. That
is why all eight flares are enqueued at once and why six of the seven AI
fades read 0.0 - the field is ahead with its nozzles away from the camera.
Boot 1's 160 gate hits and its controls run on past the spread (its
submit hits stop once the field opens up), so the two boots together cover
more than one instant. Constant across all 240 hits: `*0x008c1430 = -1`, camera object
`0x309c....` present with `+0x34 = 0x3000` (so `& 6 == 0`, the override
path at `0x002a11b0` never runs), `+0x40 = 10`, `+0x1ec = 0` (targets no
craft), `r29 = 0`, `craft+0x5fa4 = 0` (boot 2, which ran longer, saw it
go to `1` after 83 hits - the race start). The three columns on the right
are the fade path's own footprints, read off the flare each hit, and they
say the same thing a second way: the `Slow Alpha Noise Timer` countdown at
`+0x190` only decrements inside the fade path (`0x002a0cac`), and it walks
10 -> 1 on the seven AI flares and sits at 1 on the player's; `+0x18c` is
only stored at `0x002a0d70`, and it is 0.0 on the player's flare throughout
while one AI flare (slot 5, the craft ahead whose nozzles face the camera)
carries a small live value. The `+0x270` ring index advances on all eight,
the player's included - the occlusion query at `0x002a1210` runs for every
craft.

Boots 1 and 2, biased sampling and all, agree on every value they saw:
gate hits across boot 1's seven distinct crafts and boot 2's eight were
`owner -1 -> r10 = 1` (211 + 159 hits) and `owner 0 -> r10 = 0` (3 + 1
hits), view `-1`, camera target `0`. Boot 1's two controls: the **submit**
at `0x002a1104` was reached 27 times, every one on AI craft `0x33f3bb00`
(owner -1), and then not at all for three 1.2 s windows once the field
spread - the quad is genuinely built and submitted for an opponent, and
only while the fade is positive. The **query answer** at `0x002a1428` read
`r28 = 1` on all 40 hits, 11 of them on the player's craft (owner 0) - so
the query is not what stops the player's sprite; the owner gate right after
it is.

**What this settles.** With `view = -1` in a single-player race, the
listing's `blt cr7 -> r10 = 0` at `0x002a0b6c` is the branch the player's
craft takes: `r10 = 1` only for an owner of `-1`. Measured: every AI craft
reaches the fade math and, when its lobe faces the camera, the quad; the
player's own craft is turned away at `0x002a0bb4` on every frame, before
the fade is computed, and its `+0x18c` stays at the 0.0 the fifth session's
`tuning-dump` first read. **The sprite is not drawn for the viewing
player's own craft - confidence 92.** Runtime-verified on one binary, three
boots; short of 95 for want of a second binary, and the split-screen and
camera-object cases (`view >= 0`, a camera in a mode outside
`{2, 6, 7, 8, 11}` targeting the craft) were not exercised - `view` read
`-1` and `+0x1ec` read `0` in every sample, so what they would do is still
the listing's word alone. The consequence the ninth session drew stands
with it: the compact glow on the player's nozzles in the original captures
is the flame `.rcsmodel` and the particles, not this sprite, and a renderer
that draws the sprite on the viewing player's craft draws something the
original never does there.

**Other numbers the run gave for free.** The render queue calls
`EngineFlare_RenderTick` for all eight flares every frame in one fixed
order, the player's craft fourth - so the fifth session's ~2,500 calls
were about 310 frames of all eight, not 2,500 of one, and its 0 hits at
`0x002a0bb8` mean it happened to be reading the player's flare. `Slow Alpha
Noise Timer` reloads at 10, as the ninth session read it off `+0x498`.
The camera object's mode read 10 in a chase view throughout; the ninth
session's `0x9c4` mode set (`{2, 6, 7, 8, 11}`) does not contain it, so a
camera targeting a craft in this mode would *not* be an exception - which
is consistent with the chase camera never targeting the craft through
`+0x1ec` in the first place.

Artefacts: `data/reference/hd-capture/flare-owner/` - `flare-run1/`
(gate, submit, query), `flare-gate-run2/`, `flare-gate3/` (the hop run,
the table above is its `flare.json`), each with `logs/rpcs3.log`,
`race-loaded.png` and `race-gate.png`. The emulator's own `RPCS3.log`
for every boot opened with

```
SYS: argc: 7, argv: '/opt/rpcs3/AppRun.wrapped' '--no-gui' '--input-config' 'oag' '--config' '.../data/tools/rpcs3-scratch-config.yml' '.../hdfury-ps3-eu-dec.iso'
SYS: Used configuration:
Core:
  PPU Decoder: Interpreter (static)
...
Audio:
  Renderer: "Null"
```

Reproduce: `python3 scripts/rpcs3-drive.py display`, then
`uv run --with evdev python3 scripts/hd-flare-owner-break.py flare-gate
<out>` (about fifteen minutes on a loaded machine: a 5-minute interpreter
boot, the six-tap walk, then 240 stops at roughly 2 s each), and
`python3 scripts/rpcs3-drive.py stop` after.

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
  is load-bearing, and `oag_fx::exhaust::hd::SPEED_FIELD_GAIN` carries
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
  drew; `oag_fx::exhaust::hd::Flame` is the replacement and
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

**2026-09-05: static-only session reframes the question rather than answering
it, plus a clean negative.** Confidence 75 for the reframe, 85 for the
negative - both read straight off disassembly, not the decompiler's
pseudocode for `0x00090d30`, which carries `halt_baddata()` and eleven
"Removing unreachable block" warnings and should not be trusted for control
flow in this function; the raw instruction listing is.

- **`craft+0x108` is not "armed" by any external writer at all - it is a
  self-contained rise/fall ramp entirely inside `FUN_00090d30`, and the
  whole block it lives in (`+0xf4` through `+0x110`) only runs at all when
  the RaceManager slot for this craft's team reads mode `10`** (`lwz
  r9,0x208(r11); cmpwi cr7,r9,0xa`, or the `+0x204` twin) - confirming
  engine-trail.md's existing "gated on mode 10" claim independently, at the
  assembly level. Inside that gate, two `stfs ...,0x108(r31)` sites do all
  the work: `0x00091418` rises by `fmadds f1,f0,f27,f1` where `f0` is `2.0`
  or `4.0` selected by a byte at `craft+0xfc`, and `0x0009167c` falls by
  `fsubs` at a flat `2 * dt` - both paths run every tick this block is live,
  so there is no separate "arm" event to find inside this function; the
  byte at `+0xfc` is the only thing that changes behaviour, and it selects
  a *rate*, not a duration.
- **Negative, checked exhaustively for the per-tick path**: `FUN_00090d30`
  has exactly one caller, `FUN_0009e3d0` (confidence 90 this is the craft's
  own master per-tick `Update`, since it also calls the two dissolved `+0x12c`
  writers `0x0008a5b8`/`0x00089fa8` and about twenty other per-tick
  subsystem functions on the same `param_2`). Batch-decompiling all twenty
  of those siblings and grepping for `0x108` found it in **none of them** -
  so nothing else the craft's own tick calls touches this field either
  directly or through a documented subsystem call. If something external
  arms it, it is event-driven code reached from *outside* the per-tick
  chain (pad contact, barrel-roll input, race-start), not a sibling method.
- **A candidate for what sets the `+0xfc` rate-selector byte, confidence 35,
  below naming and stated as a hypothesis only**: the same function has a
  block (`0x00091104`-`0x0009111c` and its `0x000917d8` branch) that reads a
  sub-object at `craft+0x40`, checks its `+0x58` field against the constant
  `0xb`, and on match reads a `+0x5f70` pointer - the same offset
  `nozzle_local_of` corrects for elsewhere on this page - before loading a
  transform via `lvx` into the same `+0xf4..+0x108` block. The shape (a
  class-ID compare gating a transform copy from a track-side object) reads
  like a pad/surface-contact check, which would make the rate-selector -
  and so the ramp's *rise* - pad-driven. **Not verified**: `0xb`'s meaning as
  a class ID is not read from any enum, and the block was reached only by
  static tracing, never observed live. **A whole-binary `stb`/`stbu` sweep
  for `0xfc(` addressing found only two writers of the byte at all, both
  inside `0x00090d30` itself (`stbu r0,0xfc(r28)` at `0x0009113c` and
  `0x00091b30`), and both clear it to `0`** - nothing sets it to `1` through
  this addressing form anywhere in the binary, so either a whole-word/struct
  write elsewhere overlaps this byte, or it is reached through indexed
  addressing (`stbx`) a mnemonic-filtered sweep does not catch. Read for
  that live before trusting a breakpoint on the literal `+0xfc(r31)` address
  to fire on the arming write.
- **A tooling trap worth carrying forward**: `search_byte_patterns`'s
  `mask` parameter does not implement a wildcard. `d0000108`/`fc00ffff`
  (byte0 masked to the `stfs` opcode, byte1 wildcarded, byte2/3 exact)
  returned zero matches against a function disassembly-confirmed to contain
  three `stfs ...,0x108(r31)` instructions at that exact address; the
  identical mask against the *actual* byte1 value (`d03f0108`) found all
  three. The mask is accepted and silently ignored rather than rejected -
  a masked byte-pattern sweep for "any register" reads as "not found"
  regardless of what is actually there. `search_instructions` with a
  `mnemonic`/`operand_pattern` filter is the reliable equivalent and found
  48 real `stfs ...,0x108(...)` sites in one call. Full account in
  `HANDOVER.md`'s Traps section.
- Also ruled out this session, each read in full: `FUN_000f9628` (writes
  `world+0x2d0+0x108`, a time-dilation ramp reached from
  `Physics_TickWorld`, gated on `world+0x3d4` - nothing to do with a craft);
  `FUN_0010d580` (`+0x10d`/`+0x50`/`+0x140`, an unrelated struct);
  `FUN_002bf3e8` (one-time geometry/node-hierarchy construction, writing
  `+0x104` through `+0x118` once from a model's own header fields, not a
  per-tick craft value); `FUN_0024f2e0` (an eight-slot decay array at
  `+0xf4..+0x110` each paired with a light handle at `+0x114..+0x130` -
  the same offset range by coincidence, a different, larger struct).

**2026-09-15: the negative re-run on the whole image, after the `lvlx`
reimport - re-confirmed, confidence 85.** The 2026-09-05 negative above
was drawn from decompiles of functions the disassembler had holed at an
undecoded Cell `lvlx` (`0x00090d30` and its caller `0x0009e3d0` both
carried one - see [toolchain.md#ps3](../../../reverse-engineering/toolchain.md#ps3),
"Some Cell vector instructions are missing"), so it was computed over an
image missing code. Redone on the reimported program, where `0x00090d30`
decompiles clean (718 lines, no `halt_baddata()`, no unreachable-block
warning; its two `lvlx` at `0x000918ec`/`0x000918f0` decode) and so does
`0x0009e3d0` (9,556 bytes, no warning). What was actually searched:

- **The per-tick chain.** `get_function_callees(0x0009e3d0)` lists 50
  callees (40 `.opd` functions plus ten TOC stubs, `Image_SetVertexColours`
  and `RaceManager_GetInstance`). An inline script decompiled every one and
  scanned both its listing and its pseudocode for `0x108`: **the only
  function in the set that touches `+0x108` off anything but the stack is
  `0x00090d30` itself** - five sites (`lfs`/`stfs f1,0x108(r31)` at
  `0x00091408`-`0x0009142c` and `0x00091674`-`0x0009167c`), the same
  rise/fall pair the 2026-09-05 reading had. Four other callees have
  `0x108(r1)` only (`std`/`stfd` register saves in `0x00089fa8`,
  `0x0008c6f8`, `0x0008caf0`, `0x00297790`). No callee decompiles with a
  warning any more.
- **The whole image.** Every store instruction with a `0x108(` displacement:
  **1,006** (`std` 548, `stw` 317, `stfd` 71, `stfs` 50, `stb` 19, `stfsu`
  1), of which **770 are `0x108(r1)`** - stack-frame saves, the noise the
  2026-09-05 note warned about. Of the 236 on another base register, an
  inline script kept the ones whose function also touches any displacement
  in `0x5000..0x7eff` (the craft is a `0x7f00`-byte allocation and every
  craft method read so far reaches some such offset): **20 sites in 10
  functions**, every one classified from its decompile - `0x00090d30` (3,
  the ramp itself); `EngineFlare_Update` (3, the *flare* object's own
  `+0x108`, base `r31` = `this`, whose `+0x134` is the craft);
  `PhotoMode_Update` (1, its own object); `FUN_0005ac00` (1, a
  300-byte per-slot record at `raceManager + slot * 0x12c + 0x674`);
  `FUN_000c23e0`/`FUN_000c2bf8` (2, `this[0x42] = 1.0f` at construction of
  an object that holds a craft pointer at `+0x14c`);
  `FUN_000ca5f8`/`FUN_000ca8e8` (4, `+0x154` of the `0x1b4`-byte pad object
  the human-craft spawner allocates beside the craft); `FUN_00059870` (1, a
  21-entry ranking table swapping `+0x108`/`+0x10c`); `FUN_00071890` (1, a
  HUD object copying its own `+0xa8`); `FUN_000dbdc8` (1, a string-hash
  cache the craft's parameter loader fills, then reads back into
  `craft+0x7e4c`); `FUN_001340d8` (1, a bomb's vector triple at
  `+0x108..+0x110`); `FUN_001395f0` (2, a weapon's timer set
  `+0x100..+0x134`). The 216 remaining sites are in functions that never
  touch a craft-sized offset; the 14 `stfs` among them were still opened
  one by one - a rocket's four-float parameter (`0x00125690`, keyed by hash
  from the weapon manager's `+0x204c` table), a sin/cos triple on an object
  with 16-byte matrices at `+0x1b0..` (`0x0028fa50`), a global at
  `*0x008b35a0` (`0x002b2c60`, the call the human-craft spawner makes after
  the last player), `RenderManager_PrepareEye_q`, `SoundManager_Construct`,
  and the four already ruled out on 2026-09-05 - none takes a craft. The
  heuristic's own blind spot is a small helper that takes a craft and
  writes only `+0x108`, which would show `baseMax=0x108` and nothing
  larger; the dozen such sites in the craft-method address range
  (`0x00083510`, `0x00085270`, `0x0008530c`, `0x00085340`, `0x000853c4`,
  `0x000853f8`, `0x00085704`, `0x000857d4`, `0x000858a8`, `0x00087b50`,
  `0x00088b50`, `0x000890e8`, plus the four `0x00049xxx` twins) were opened
  for that reason: every one is `*(obj + 0x108) += 1` or `-= 1` on a
  sub-object loaded from the argument - an integer reference count on a
  shared resource, the pair `0x00085298`/`0x00085350` being the
  acquire/release the human-craft spawner calls on a global - not a float
  and not a craft. Outside that range the same shape is not excluded,
  only made unlikely by the per-tick argument above.
- **Not covered by a displacement search, stated so nobody reads this as
  more than it is**: indexed stores (`stfsx`/`stwx`), 16-byte vector stores
  (`stvx`) over the `+0xf4..+0x110` block, and `memcpy`-shaped struct
  copies. `0x00090d30` itself has none of those on the craft (`stvx` only
  to `r1`), and the `+0xfc` rate-selector byte now has **three** clearing
  writers in it, not two - `stbu r0,0xfc(r28)` at `0x0009113c`,
  `0x00091a68` (in the stretch the hole hid) and `0x00091b30`, all storing
  zero - and still no setter anywhere in the image through `stb`/`stbu`.

So the reframe stands with the corruption caveat removed: `craft+0x108` is
the self-contained ramp - now readable in the decompiler as
`+0x108 += (byte@+0xfc ? 4.0 : 2.0) * dt` on the rising branch (`bVar24`),
`-= 2 * dt` otherwise, clamped through `FUN_00677538`, and mirrored into
`*(craft+0x590) + 0x12c`, which is the flare object's boost timer - and the
decompiler now also shows the rising branch is entered only past the
`*(craft+0x40)+0x58 == 0xb` class check and a `+0x5f70` transform read the
2026-09-05 note called the pad-contact candidate. That reading is
unchanged (still 35, still a hypothesis, still never observed live); the
one thing this session adds to it is that the decompiler and the
disassembly now agree.

## What follows for the renderer, and what stays open

Implemented in `oag_fx::exhaust::hd` (the tube and the flame blends),
`exhaust.wgsl` (facing fade, depth fade `saturate(window_z * 0.75)`, the
blue-red mix, the baked scroll) and `oag_raceplay` (per-slot state, the
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
- The `Engine_Flare_Rich.gtf` sprite flare **draws by the traced law since
  2026-09-15, and never on the viewing player's craft**:
  `oag_fx::exhaust::hd::Sprite` is `EngineFlare_RenderTick`'s own quad
  (4:1, `half_height = Min + Radius * clamp(fade) + Jitter * rand01`,
  alpha `= fade`, the fade of the ninth session - a `cos^32` lobe into the
  nozzle that is whole inside 15 units and gone at 30) and
  `race::effects::hd_sprite_quad` skips slot 0, the craft `Race::view`
  frames, which is the owner gate the tenth session measured (92). Every
  constant is a tuning-file row pinned by
  `the_sprite_flares_constants_are_the_discs_own`; two things are
  **chosen, not measured** and say so in the code: the per-view distance
  scale (`*(r2+0x5aa0)[view]`, unread, taken as 1.0) and which hemisphere
  of the view dot draws (the page's two unsettled sign conventions; the
  camera looking into the nozzle is the one taken). Still not drawn: the
  chromatic dispersion scalar and the occluder query. Verified on this
  engine's own start grid
  (`the_sprite_flare_skips_the_players_craft_and_fades_the_rest_by_the_law`):
  the field stands 34..148 units from the player's camera, so all seven
  opponents' fades are 0 and no quad is built for any of them (the quad
  is built only while the fade is positive, the `ble` after the store at
  `0x002a0d70`) - the same reading as the breakpoint run's `+0x18c`
  column - and the player's oversized white disc is gone;
  a camera posed 10 units behind an opponent shows its streak. **The
  history below is kept as written.** It drew, from 2026-09-01 to
  2026-09-15, with its authored radius (3, jittered by 0.5, floored at 2)
  and the `Slow Alpha Noise` opacity walk (0.5..0.8, chase 0.1, retarget
  every 10) - the flare's init (`0x002a1528`) loads the texture and four
  corner pairs, so the sprite is real, and it is most of the "solid core"
  the exhaust reads as in the original. Then unread and undrawn: its spin,
  chromatic dispersion, `Flare Fadeout Dist/Range` term and the occluder
  query - the shader pair (`engineflare_vp/fp`) resolves through no
  registry read so far. **And a
  2026-09-01 screenshot comparison confirms this matters**: drawn at the
  literal 3.0/2.0/0.5 with no distance term, the sprite is visibly oversized
  against the original at ordinary chase distance - "All 41 rows" above has
  the evidence and the ruled-out causes; `hd::Sprite`'s radius is
  unchanged rather than fitted to the screenshots. **`Flare Fadeout
  Dist/Range` (15/15) is ruled out as the cause, not just unread** - see
  "Seventh session" below: a 15-unit fade start is fully faded-in at
  ordinary chase distance regardless of whether the fade math runs, so
  reaching it would not shrink anything. **Two 2026-09-01 sessions
  hunted the draw call and did not find it** (`+0xe4` turned out to be
  intensity, not radius storage - a correction to this page; neither
  `EngineFlare_PlaceShapes` nor `_Update` reads any radius-shaped field back
  out; live-dumped RSX pointers inside the object read as occlusion-query
  buffers, not vertex data; `Billboard.cpp` is the unrelated track-signage
  system) - **and a third, 2026-09-02, found it**: "Third session: it was on
  `EngineFlare`'s own vtable, not a separate manager" above.
  `EngineFlare_RenderTick` (`0x002a08a8`, confidence 78) is the function; it
  is neither `EngineFlare` owning geometry directly nor a call into a
  separate manager but a third shape neither session had framed yet - a
  sibling virtual method on the same object, found by reading the two
  still-unopened vtable slots rather than hunting a second object or
  scanning the pushbuffer. What it draws the quad *with* - which of its
  loads is `Flare Radius`, `Flare Fadeout Dist/Range`, or `Flare Size
  Clamp` - remained unread until the function decompiled whole on
  2026-09-15 ("Ninth session": `+0x47c` is `Flare Radius`, the half-height
  is `Min + Radius * clamp(fade) + Jitter * rand`, the width four times
  that, and `Size Clamp` is never loaded); before that most of the function
  past its gate read as RSX submission plumbing. **A fourth
  session (same day) confirmed `Flare Fadeout Dist`/`Range` (15.0/15.0) are
  exactly the two constants a `saturate()`-shaped fade inside the function
  consumes, live; a fifth then breakpoint-traced the function and confirmed
  the entry runs essentially every frame - real runtime verification the
  function is not dead code - but misread its second breakpoint's target
  (`0x002a1110`) as a gate landing when it is the function's one shared
  epilogue, hit by every returning call including the full-draw path, so
  "12 of 12 hits there" was never evidence of a closed gate** - see "Fourth
  session" and "Fifth session" above, the latter corrected after review, and
  then rerun with a fixed script and a three-point ladder inside the fade
  math itself instead of the two endpoints originally tried. **That rerun
  is a clean result**: zero hits at any of the three checkpoints in 45
  seconds of active, visibly-flared racing - `EngineFlare_RenderTick`'s
  fade math does not merely evaluate to a non-positive number, it is never
  entered at all for this craft. **A sixth session then found and confirmed
  which gate**: not the byte-flag check (`struct_base+0x494`, open, 4/4
  live) or the count-field one (`craft+0x5fa4`, open, 1/1 live), but a third,
  previously-untraced branch on `FUN_006765e8()`'s return value
  (`0x002a0b44`) - closed, 2/2 live, routing to `0x002a1210` instead of the
  fade math. That address is real code, not the shared epilogue, and itself
  reaches the same RSX-submission trampolines the fade path does. **A
  seventh session reframed this again**: neither branch this page has
  found writes the sprite's half-size at all - that comes from a flat,
  distance-independent constant in the renderer (`SPRITE_RADIUS`) - and
  `0x002a1210`'s own shape (a modulo-3 ring buffer entry, feeding the same
  trampolines) reads far more like the occlusion query `Flare Occluder
  Radius`/`Flare Depth Bias` want than like a second draw path. An
  occlusion query would not explain the size gap either. See "Seventh
  session" above for why the size question points at the renderer rather
  than at either of this function's branches. **An eighth session settled
  which renderer piece, with a corrected method**: a first attempt
  attributed the blob to the flame mesh instead of the sprite off a
  saturating brightness-threshold count, which turned out to be
  measuring the wrong thing (several additive layers already saturate the
  same pixels, so removing any one barely moves a threshold count even
  when its real extent is enormous). A pixel-diff isolation - one layer's
  own additive contribution, disabled render subtracted from enabled -
  reverses that reading: the sprite is the dominant contributor after
  all, boost plume is zero, flame mesh is a small tight one. **Fixed**:
  `race::effects::hd_sprite_quad` now scales the sprite radius by
  `exhaust::CRAFT_ROW_SCALE` (0.75, the same factor already applied to the
  sprite's own *position*), measured to cut the sprite's isolated
  footprint by ~45% - real and substantial, though not confirmed to be
  the exact factor the original itself would apply, and not confirmed to
  close the gap outright rather than only narrow it. See "Eighth session"
  above for the method, the numbers, and what is still open.
- `Flare Size Clamp` (50.0) and `Engine Flare Particles`
  (`Enable`/`Min Alpha` 1/0.25) are named for the first time this session -
  "All 41 rows" above - and neither is read past the tuning file.
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
  engine has no equivalent of; the effect plays through `oag_fx::psys`
  the moment one exists. See "the craft sparks" above.
- What `+0x11dc` carries into the spawner's `f1`, what the two craft-state
  gates at `+0x5ed8`/`+0x5f42` mean, and whether `+0x11f0` names the striking
  or the struck craft.
- What the original does to the ring across a **respawn** - the race start is
  read (born full, bunched, dark) but a respawn was never captured; this
  engine re-bunches at the new pose as a stated approximation.
- ~~Whether the white-to-red vertex ramp differs on a classic (blue) HD grid~~ -
  **it does**, see "The vertex colour ramp is per skin, 2026-10-10" below.

## The vertex colour ramp is per skin, 2026-10-10 (`hd-trail-colour-flare-ghost`)

**The "white-to-red vertex ramp is universal" reading was wrong, and it was never
measured on a classic craft.** The maintainer's play report - an HD trail that
should be cyan throughout, ours cyan "with a gradient to red" - is right on the
mechanism. Every dump before this session was a Fury `concept1` grid (the open
item above said so), and a classic ramp was inferred from the Fury one.

**Method.** RPCS3's guest memory, including the RSX local memory holding the SPU's
output buffers, is readable through `/proc/<pid>/mem` with no debugger. For all
eight trails (`alloc + 0x84a0 + slot * 0x1230`: craft pointer `+0x1204`, current
buffer `+0x1214`) read the 324-vertex buffer, `VertexColour1` u8x4 at `+0x20` of
each 36-byte vertex. Then, in **one** GDB session, write `0` over `craft + 0x7d2c`
(the Fury-skin byte, read as 1 on every craft) for all eight craft, resume, wait one
second, and read the buffers again. Vertices come in pairs, a ring is 2 of them, a
fin is 108. Scripts: `scripts/rpcs3-hd-trail-flag-ab.py` (the A/B),
`scripts/rpcs3-hd-trail-colours.py` (read-only full table), `scripts/rpcs3-hd-trail-flag.py`
(read or set the byte), `scripts/rpcs3-hd-trail-film.py` (frames).

| ring k | flag 1 (Fury) rgb | flag 0 (classic) rgb |
| --- | --- | --- |
| 0 | 255, 255, 255 | 63, 255, 255 |
| 3 | 255, 245, 245 | 84, 240, 255 |
| 13 | 255, 198, 198 (ring 12) | 106, 226, 255 (ring 12) |
| 27 and every ring after | 255, 0, 0 | 254, 127, 255 |

Both are a linear lerp over the first 27 rings and then held. The classic ramp is
cyan to light violet with blue pinned at 255, and it never passes through red; the
Fury ramp is white to pure red. Same result on all eight slots. The SPU job
evidently picks the ramp from the lane mask `Trail_WriteCraftContexts` writes from
the same byte (reading, not yet disassembled; the colours changed within one second
of the write on a running race, which is the evidence, and the buffers read the same
on all eight slots).

**What it does to the picture.** The fragment program multiplies the blue texture's
colour by this vertex colour (`MUL H0.xyz, H0, f[TC0]`). Ours used the Fury ramp for
every craft, so a classic tail was `texture * (1, 0, 0)` - dark red-violet, the
reported "gradient to red". It now follows the flag: `oag_fx::exhaust::hd::tint_at`.
Pinned by `the_vertex_ramp_follows_the_fury_flag_as_the_running_game_wrote_it`.

**Confidence.** 85 for the two ramps (read from the running game's own buffers, eight
trails, two flag states, one launch; the Fury ramp also matches the three earlier
sessions' dumps). Not reached on RPCS3: a classic craft in a race. Every route on this
disc (Racebox Feisar, the HD campaign, three save states) loaded `concept1` with flag
1 on all eight craft, so the classic frames are a **Fury hull with its byte
overwritten**, labelled so under `data/reference/hd-capture/trail-colour/classic-trail-flag0/`.
Those frames show an orange to pink strip behind the nozzle over a dark floor, which
is the flame plume at the nozzle, not the ribbon; no frame isolates the ribbon's tail
colour, so **the all-cyan tail is not confirmed by pixels**. The disc never ships a
classic hull to compare, and a launch where the race was loaded from a restored
save state aborted in the game itself at the first frame (`abort()` right after
`Loading Screen Finished`), so reach states from a fresh boot.

Omega: checked, applies in principle, not wired - its `data00.psarc` ships
`HD_EngineTrail_BlueRed{,_1,_2}.rcsmaterial` (PS4 shaders, see below) and the SPU job
has no PS4 equivalent we can read; not checkable on a capture path.

## Motion blur smeared the engine glow, 2026-10-10

HD's original has no motion blur; the maintainer's "the glow ghosts a little on tight
airbrake turns" is therefore ours. The exhaust pass wrote no velocity (write-masked
empty, so a pixel kept the surface behind it), and on a hard turn under a chase camera
the track's screen motion is large, so the blur dragged the plume and trail along with
the floor. Frames at tick 300 of `verification/scenarios/hd-airbrake-left.inputs`
(`--team feisar_c1`) with `--motion-blur high` before and after, plus `off`, are in
`data/reference/hd-capture/flare-ghost/`. The exhaust fragment entry
`fs_main_velocity` now writes zero motion weighted by its own alpha (the velocity
target blends source-alpha over). **Chosen, not measured**: no original to compare.
The engine flare sprite is not drawn for the player's own craft, so the glow in
question is the flame mesh and the tube. The same reasoning applies to the other
alpha-blended fx (particles, clouds); left as they were.

## Reproducing this

```sh
uv run --with evdev python3 scripts/rpcs3-trail-dump.py /tmp/hd-trail-dump
uv run --with evdev python3 scripts/rpcs3-trail-dump.py /tmp/hd-trail-early --early
uv run --with evdev python3 scripts/rpcs3-trail-dump.py /tmp/hd-trail-params --params
# the sprite flare's own object, hunting for its vertex buffer among the RSX
# pointers it holds - see "the leading reading is that these are occlusion-
# query result buffers" above:
uv run --with evdev python3 scripts/hd-flare-sprite-dump.py /tmp/hd-flare-sprite-dump
python3 scripts/ps3-toc.py attrib 0x0079bdd8   # who loads Engine_Flare_Rich.gtf
python3 scripts/ps3-toc.py map | grep -i flare  # Billboard.cpp etc., ruled out
# EngineFlare_RenderTick, found on the object's own vtable rather than a
# separate manager - via the ghidra-mcp read_memory tool, no script:
#   read_memory 0x008b2f14 16    # PTR_PTR_008b2f14, the vtable pointer slot
#   read_memory 0x00869d30 128   # the vtable itself, 8-byte OPD-entry stride;
#                                 # slot 5 (+0x28) and slot 7 (+0x38) are the
#                                 # two class-specific entries besides slot 3
#                                 # (EngineFlare_Update); read each slot's OPD
#                                 # pair the same way to get the real function
#                                 # address the two generic-vtable pointers
#                                 # (401/386/etc.-xref shared stubs) are not
python3 scripts/ps3-toc.py toc 0x002a08a8      # confirms the render function's
                                                 # TOC matches EngineFlare_Update's
# the shared tuning block EngineFlare_RenderTick and Trail_RenderTick both
# read, plus the fade-scale storage at the flare's own +0x18c/+0x194 - see
# "Fourth session" above:
uv run --with evdev python3 scripts/hd-flare-tuning-dump.py /tmp/hd-flare-tuning-dump
# the breakpoint trace - needs `PPU Decoder: Interpreter (static)` in
# config.yml first (shared, session-scoped edit, switch back after), see
# "Fifth session" above:
uv run --with evdev python3 scripts/hd-flare-rendertick-break.py /tmp/hd-flare-rendertick-break
# which of the three gates before the fade math is closing - one at a time,
# see "Sixth session" above for why not all together:
uv run --with evdev python3 scripts/hd-flare-gate-check.py /tmp/hd-flare-gate-check byte
uv run --with evdev python3 scripts/hd-flare-gate-check.py /tmp/hd-flare-gate-check count
uv run --with evdev python3 scripts/hd-flare-gate-check.py /tmp/hd-flare-gate-check alt
# the screenshot comparison behind "Seventh session" above - crop each PNG
# around the engine gap and eyeball flare width against hull width; no
# script, three original-side captures already on disc:
#   data/reference/hd-capture/flare-size/original-rpcs3.png
#   data/reference/hd-capture/flare-size/tuning-dump/race.png
#   data/reference/hd-capture/flare-size/rendertick-break-v2/race.png
#   data/reference/hd-capture/flare-size/ours.png             # this engine
# the pixel-diff isolation behind "Eighth session" above - no emulator, this
# engine only, deterministic ticks so two runs are byte-identical apart from
# the one thing disabled between them:
just build
just play hd --race --press cross --ticks 300 --screenshot /tmp/a.png   # baseline
# ...then a `continue` right before the draw call under test in
# crates/raceplay/src/scene/frame.rs (frame.rs:760-772 for the flame mesh,
# :773-787 for the boost plume) or an early return in
# crates/raceplay/src/effects.rs's hd_sprite_quad, rebuild, and:
just play hd --race --press cross --ticks 300 --screenshot /tmp/b.png   # one layer disabled
uv run --with pillow --with numpy python3 -c "
from PIL import Image; import numpy as np
box = (560, 420, 900, 700)  # the engine gap, at this reproduction's framing
a = np.asarray(Image.open('/tmp/a.png').convert('RGB').crop(box), dtype=np.int32)
b = np.asarray(Image.open('/tmp/b.png').convert('RGB').crop(box), dtype=np.int32)
contrib = np.clip(a - b, 0, 255).sum(axis=2)  # a's layer's own additive contribution
print('footprint px (>30):', int((contrib > 30).sum()), '  total:', int(contrib.sum()))
"
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

## Two-sided facing term, 2026-10-08 (`hd-leach-beam`)

`python3 scripts/ps3-microcode.py fp-file` on `hd_enginetrail_bluered.rcsmaterial`
(DATA06 `/data/ribboneffects/materials/`) prints
`@0x0b MIN H0.w, |H4.xxxx|, {..} [const@slot 0xc = 0xe296b1ed]`: the facing
dot `H4.x = dot(norm(TC2), norm(TC1))` goes through NV40's `SRC0_ABS` before the
`MIN`, so the fade is `saturate(clamp(|dot|, 0, c) / c)`, two-sided, the same
bit as the Rocket smoke ribbon ([rocket-trail.md](rocket-trail.md)). Confidence
85 (instruction read; the rocket page's live eye-vector argument is the
independent half). `crates/fx/shaders/exhaust.wesl` now takes `abs(facing_dot)`
(it drew the ribbon one-sided before). Matched frames, autopilot `--race`,
ticks 300/330/360/1100 byte-identical (camera dead astern, dot > 0); tick 700
differs in 1133 px around the plume's edge, slightly more plume on the edge.
Confidence 85 rests on the instruction read alone (the Rocket ribbon's live
eye-vector dump is a different ribbon). Omega: checked, differs - its
`data00.psarc` ships `Data/ribboneffects/HD_EngineTrail_BlueRed{,_1,_2}.rcsmaterial`
as PS4 shaders and its `eboot.bin` names `TrailEffectManager.cpp`, so the
family carries forward, but `ps3-microcode.py` cannot read PS4 code; not
wired on Omega, no PS4 capture path.
