# Pads

How a `Speedup Pad` is bound, how a craft is tested against one, and what the
test hands back. The force the hit then applies is
[`Ship_ApplySpeedupPad`](engine.md#the-track-section-force-is-the-speed-pad-boost-and-it-is-the-wrong-sign)
in `engine.md`; this page is the trigger side.

The data side - the payload layout and what the shipped tracks author - is
[`docs/formats/pads.md`](../../../formats/pads.md). The reimplementation is
`crates/formats/src/pads.rs`.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x089264f4` | `Pad_Bind` | 85 |
| `0x088866bc` | `Pad_ContainsPoint` | 85 |
| `0x0888686c` | `Pad_SweptTest_q` | 65 |
| `0x08887144` | `Pads_TestCraft` | 90 |
| `0x08a6be7c` | `SpeedupPad_ClassTag` | 90 |
| `0x08a6ed40` | `WeaponPad_ClassTag` | 85 |

## `Pad_Bind` (`0x089264f4`)

The class `0x3bd` bind handler, from the class table in
[`vex.md`](../../../formats/vex.md).

```c
uVar1 = Mesh_Bind(param_1, param_2);        // 0x0890e998
iVar2 = *(int *)(param_1 + 0x58);           // the payload the Mesh bind parked
*(param_1 + 0x1b0) = *(iVar2 + 0x10);       // four words: box minimum
*(param_1 + 0x1b4) = *(iVar2 + 0x14);
*(param_1 + 0x1b8) = *(iVar2 + 0x18);
*(param_1 + 0x1bc) = *(iVar2 + 0x1c);
*(param_1 + 0x1c0) = *(iVar2 + 0x20);       // four words: box maximum
*(param_1 + 0x1c4) = *(iVar2 + 0x24);
*(param_1 + 0x1c8) = *(iVar2 + 0x28);
*(param_1 + 0x1cc) = *(iVar2 + 0x2c);
*(float *)(param_1 + 0x1b4) -= 2.0;         // min.y
*(float *)(param_1 + 0x1c4) += 8.0;         // max.y
for (i = 0; i < 8; i++) *(param_1 + 0x1d0 + i * 4) = 0;
```

**The first call is what settles the class.** `0x0890e998` is the `Mesh` bind:
it fixes up vertex and index pointers, walks the material array at `+0x30`,
resolves textures, and builds the GE colour commands at `+0xdc`. So a
`Speedup Pad` **is a `Mesh`**, its payload is a mesh payload, and the pair of
`vec4`s it then reads at `+0x10` and `+0x20` is the mesh's own bounding box -
already documented at that offset in [`vex.md`](../../../formats/vex.md).

That is the finding that makes the pads drawable at all: they carry geometry, in
the track file, and nothing but the class id distinguishes them from any other
mesh. Confidence **85** - unambiguous decompilation, and the payload agrees
across all 762 pad nodes on the disc.

The vertical expansion is asymmetric and in `+y`, which is world up (see
[`track.md`](../../../formats/track.md)), so a pad is entered from above. It
turns a plate about 0.15 units thick into a volume about 10 units tall.

The eight words zeroed at `+0x1d0` are one per racer. What they hold is not a
latch - see `Pad_SweptTest_q` below.

### The literals are only meaningful because the scale is 1

The `Mesh` bind divides the same `+0x10`/`+0x20` floats by a quantisation scale
before storing its own bounds at `+0x80`. `Pad_Bind` does **not**: it copies them
raw and adds `-2.0` and `+8.0` in world units. That is only coherent if a pad
mesh's scale is `1.0`.

Checked rather than assumed: all 544 `Speedup Pad` boxes on the 40 PSP track
files come out between **9.55 and 9.71 units wide**, which is a craft's width and
one authored size. A quantisation scale would vary per mesh and would put these
orders of magnitude out. Asserted in
`crates/formats/tests/pads_ground_truth.rs`.

## `Pad_ContainsPoint` (`0x088866bc`)

```c
m = *(*(pad + 8) + 0x30);                    // the node's 4x3 matrix
d = vsub_t(point, *(m + 0x30));              // minus the translation
local = vtfm3_t(m, d);                       // into pad-local space
for each axis:
    if (local[a] < min[a])      out[a] = min[a] - local[a];
    else if (local[a] > max[a]) out[a] = max[a] - local[a];
    else                        out[a] = 0.0;
*param_4 = sqrt(dot(out, out));              // the caller's per-racer word
return (that == 0.0 && *(pad + 0x1a0) == 0.0);
```

Three things fall out, and all three matter to a reimplementation:

1. **The test runs in pad-local space.** The box at `+0x1b0`/`+0x1c0` is never
   compared against world coordinates; the point is inverse-transformed first.
   Rows 0 to 2 of a `.vex` matrix are orthonormal, so the inverse rotation is a
   dot against each row. Pads are rotated to follow the track, so an
   axis-aligned world-space test is not an approximation of this - it is a
   different answer.
2. **What it returns through `param_3` is the matrix, not the pad.** This is what
   `engine.md` previously read as a "section" with a "direction at `+0x20`". The
   matrix is at node `+0x30` in the row-major, translation-in-row-3 layout every
   `.vex` uses, so `+0x20` is floats 8 to 10: **row 2, the pad's local `+Z`**.
   That is the direction the boost pushes along.
3. **`pad+0x1a0` gates every hit.** A non-zero value suppresses a hit that is
   geometrically inside. **The writer was not found.** A `Weapon Pad`'s
   `<WeaponPad refresh_time>` is the obvious candidate, which would make this a
   pickup-respawn timer that a speedup pad never sets. `crates/formats/src/pads.rs`
   carries the term inert rather than dropping it.

Confidence **85**: unambiguous decompilation, and the direction reading is
corroborated by the pads' own geometry being flat plates whose local `+Z` runs
along the track.

## `Pad_SweptTest_q` (`0x0888686c`)

Named with `_q`: the function is read confidently, but the reading of its `25.0`
branch below is an inference.

```c
w = pad + slot * 4;
*(float *)(w + 0x1d0) -= moved;              // `moved` = |position - previous|
if (*(float *)(w + 0x1d0) > 0.0) return 0;   // cannot have reached it yet
if (moved < 25.0 && dot(previous, previous) > 0.0) {
    for (t = 0.25; t <= 1.0; t += 0.25)
        if (Pad_ContainsPoint(pad, lerp(previous, position, t), out, w + 0x1d0))
            return 1;
    return 0;
}
return Pad_ContainsPoint(pad, position, out, w + 0x1d0);
```

**The eight words at `+0x1d0` are a distance cache, not a trigger latch.** Each
holds the distance from that racer to this pad, as
`Pad_ContainsPoint` last measured it, and is decremented by how far the racer
moved since. The full test only runs once the racer could plausibly have reached
the pad. `docs/formats/track.md` previously recorded these as "per-ship trigger
latches"; they are the opposite - an optimisation, with no bearing on whether a
pad fires twice.

The four-point interpolation is a swept test against tunnelling: at 120 units a
second and a 60 Hz step a craft moves about 2 units a tick, but a pad is only
about 9 units long and a dropped frame is enough to step over one.

**The `moved < 25.0` guard is read as a teleport reject** - a respawn or a
warp moves further in one step than any craft drives, and sweeping across it
would sample a line through arbitrary geometry. That reading is an inference from
the magnitude, not from anything the code says, which is what holds this at
**65**. What would retire it: a breakpoint under PPSSPP showing the branch taken
on a respawn and not in ordinary driving.

## `Pads_TestCraft` (`0x08887144`)

```c
moved = |position - *(world + slot * 0x10 + 0xb50)|;   // since last tick
*out = 0;
hit = false;
for (i = 0; i < *(int *)(world + 0x108); i++)
    hit |= Pad_SweptTest_q(moved, *(world + i * 4 + 0x40), position,
                           world + slot * 0x10 + 0xb50, out, slot);
*(world + slot * 0x10 + 0xb50) = position;             // for next tick
return hit;
```

Walks a list of pads and remembers each racer's previous position at
`world + slot * 0x10 + 0xb50` - which is where `Pad_SweptTest_q`'s `previous`
comes from, and why the first tick of a race (`previous == 0`) skips the sweep.

### Retired: `world + 0x40` / `world + 0x108` hold `Speedup Pad` nodes only

**Confidence raised 65 -> 90.** The open question was whether the list
`Ship_ApplySpeedupPad` passes as `world` (`DAT_08b32c88`) is a *speedup*-pad
list, or one list shared with `Weapon Pad` that the caller filters. It is
neither guess: the list is built once, at track load, by a **class-filtered
scene-graph walk**, and the class it filters on is `0x3bd` - `Speedup Pad` -
to the exclusion of everything else in the tree, `Weapon Pad` included.

The chain, each link read at instruction level:

1. **`World_LoadTrack`** (`0x08883794`, already named, `vex.md`) finds the
   just-loaded track's root node in the World object's own child list, then
   calls `World_CollectNodeLists` on it.
2. **`World_CollectNodeLists`** (`0x088879d4`, already named, `fog.md`) is the
   writer of `+0x40`/`+0x108`, and of two sibling lists the same way:

   ```c
   local_94 = 0;
   FUN_08a6ed64(world, world + 0x40, 0x32, &local_94, filter);   // filter->class = SpeedupPad_ClassTag()
   world->0x108 = local_94;

   local_54 = 0;
   FUN_08a6ee14(world, world + 0x10c, 0x32, &local_54, filter2); // filter2->class = WeaponPad_ClassTag()
   world->0x1d4 = local_54;

   local_14 = 0;
   FUN_08a6eec4(world, world + 0x81c, 200, &local_14, filter3);
   world->0xb3c = local_14;
   ```

   `+0x40`/`+0x108` (capacity 50) and `+0x10c`/`+0x1d4` (capacity 50) are built
   by separate calls with separate class filters; `+0x81c`/`+0xb3c` (capacity
   200, unrelated to pads) is the list `fog.md` already documents this
   function for.
3. **`FUN_08a6ed64`** is the generic collector each of those three calls uses:

   ```c
   if (node->class_tag == filter->class_tag && *count < cap) {
       out[(*count)++] = node;
   }
   for (child = node->first_child; child != 0; child = child->next_sibling)
       FUN_08a6ed64(child, out, cap, count, filter);
   ```

   A pre-order walk of the whole node tree (`+0x10` first-child / `+0xc`
   next-sibling - the same layout `vex.md`'s `FUN_08a71364` walker uses),
   collecting every node whose `+4` class-tag field matches the filter's, into
   a fixed-capacity array plus a count. It is class-exclusive by construction:
   a `Weapon Pad` node cannot pass the `+0x40` list's filter.
4. **`SpeedupPad_ClassTag`** (`0x08a6be7c`) and **`WeaponPad_ClassTag`**
   (`0x08a6ed40`) are the two filter values, and each is tied to its class by
   the registration function that sets it, a `lui`/`ori`/`jal` triple down from
   the `Vex_RegisterClass` call:

   ```c
   Vex_RegisterClass(&DAT_08b64370, 0x3bd);   // Speedup Pad
   ...
   DAT_08b64374 = SpeedupPad_ClassTag();
   ```
   ```c
   Vex_RegisterClass(&DAT_08b65830, 0x3be);   // Weapon Pad
   ...
   DAT_08b65834 = WeaponPad_ClassTag();
   ```

   Both tag functions are the same idiom - `return <their own address>` - used
   across the class table as a cheap unique token; each is a leaf with no
   branches, so there is nothing left to misread. `SpeedupPad_ClassTag` is
   corroborated a second way: the `Speedup Pad` node constructor
   (`0x0892661c`, which also zeroes `pad+0x1a0` at construction - init only,
   not the still-open writer of that field) stores exactly this value to the
   new node's own `+4`, the field `FUN_08a6ed64` compares against.

**What this closes and what it doesn't.** It retires "does `+0x40` mix both
pad classes" - it does not, structurally. It does not touch `Pad_SweptTest_q`'s
own `_q` (the `25.0` teleport-reject reading), which is a separate inference
and stays at 65, or `pad+0x1a0`'s writer, which this trail never reaches -
`0x0892661c` only shows the field starts at zero.

## `ExhaustFlare_OnSpeedupPad` (`0x08904f10`)

What the original does to the *picture* when a craft enters a new pad. One
caller, `Ship_ApplySpeedupPad` at `0x08849078`, inside its new-pad branch, and it
is handed `*(racer + 0x78)`.

```c
void ExhaustFlare_OnSpeedupPad(ExhaustFlare *self)
{
    craft = self->0xc0;
    self->0xb8 = 0.8f;                      // unconditional - it is in a delay slot
    if (craft == 0) return;
    if (craft->0x368 == 0 && !(DAT_08ab07e3 == 0 && DAT_08b31048 == 2))
        FUN_0883e9b0(craft, DAT_08ac1dec, "SPEEDUPPAD", 0x400, 0);
    else
        FUN_089392b0(1.0f, craft->0x50, DAT_08ac1dec, 0, "SPEEDUPPAD", 0);
}
```

Confidence **88**: one caller, a literal string, and the two fields it touches
are both already documented on this object by
[exhaust.md](exhaust.md) - `+0xb8` the boost timer, `+0xc0` the owner found by
walking the parent chain.

**`racer + 0x78` is the craft's `Engine Flare` object.** That is the link this
call establishes and it is new: `Ship_ApplySpeedupPad` reads `craft->0x1c4` to
reach the racer, then `racer->0x78`, and the callee immediately uses `+0xb8` and
`+0xc0`, which only make sense on a flare. It is what any future effect trigger
needs.

### The flare's duration is a code literal, and it is not `<SpeedupPads time>`

`0x3f4ccccd` is built by a `lui 0x3f4c` / `ori 0xcccd` pair at `0x08904f18` and
stored to `self+0xb8`. That is **`0.8` seconds**, with no XML path and no
per-class table anywhere near it. Confidence **90**.

The consequence is worth stating plainly because it is counter-intuitive and
easy to "fix" the wrong way: the **force** runs for the speed class's own
`<SpeedupPads time>`, which is a fraction of `0.8` on every shipped class, so the
flare deliberately **outlives** the shove. A pad is a short push and a long look.
`oag_render::exhaust::BOOST_SECONDS` carries the constant and the argument.

Two more details a reader will otherwise trip over:

- **The store is unconditional.** It sits in the delay slot of the branch that
  tests `self->0xc0`, so a flare with no owner still gets its timer set.
- **Nothing in `Exhaust_Update` writes `+0xb8`** - it only reads it, in three
  places. The decay `boost_timer = max(0, boost_timer - dt)` lives in
  `Exhaust_UpdateEngineSound` (`0x08904cf4`). The sound function owning the
  visual's timer looks like a mistake and is not one.
- `<Team>boost.vex` is revealed **once** the timer passes `0.2`, and then runs on
  a **separate** 1.5-second timer (`flare+0x88`, zeroed at the reveal), so the
  plume long outlives this `0.8`. The reveal is also latched on its own visibility
  bit, so re-entering a pad while the plume is up does not extend it. See
  [exhaust.md](exhaust.md).

### The sound, and a mode constant that is not Zone's

Both branches name the same sound, `"SPEEDUPPAD"`, and pick between two emitters.
Which one is taken splits on `racer+0x368` - **the same field that gates the Zone
score and the pad-visit statistics** in `Ship_ApplySpeedupPad`. Three independent
uses, each separating one craft from the others, is a converging hypothesis that
`+0x368` distinguishes the local player from everyone else. Recorded as a
hypothesis at confidence **45**, which is below the naming threshold, so the
field stays unnamed.

**`DAT_08b31048 != 2` is not the Zone check.** This project already identifies
`DAT_08ab07e3 == 0 && DAT_08b31048 == 6` as the Zone-mode selector
([zone-mode.md](zone-mode.md)). Here the same pair is tested against **2**,
verified at instruction level (`xori a3, a3, 0x2`). So mode `2` is a second,
unidentified mode, and it must not be folded into the Zone story. Note in passing
that `0x08b31048` is `*(0x08b30f90 + 0xb8)` and the speed-class index
`DAT_08b31040` is `*(0x08b30f90 + 0xb0)`, so both live in one global game-state
block.

## An open question: `craft+0x318` and the curve it drives

Entering a new pad also zeroes `craft+0x318` (`0x08849060`), which
`Ship_UpdateCraft` accumulates by `dt` immediately after the pad call
(`0x08849cc0`). So it is **seconds since the last new pad**, a sibling of
`craft+0x2d0`, which is seconds since the craft was last *inside* one.

It feeds a piecewise-linear keyframe curve over a `.bss` block at `0x08ab0e30`:

| Offset | Contents |
| --- | --- |
| `+0x00 + i*4` | key values |
| `+0x0c + i*4` | key times |
| `+0x18 + i*4` | reciprocal spans, i.e. `1 / (t[i] - t[i-1])` |
| `+0x24` | key count |
| `+0x28` | period; when positive the input is wrapped as `frac(t / period) * amplitude` |
| `+0x2c` | amplitude |

The result is stored to `racer+0x7c`. `FUN_0884d074` initialises the block -
count `3`, period `0`, amplitude `1.0`, all three arrays zeroed - and the image
carries all zeros, so **it is inert as shipped and nothing here says what fills
it**. With period `0` the wrap is skipped; with zero keys the output is zero.

`racer+0x7c` is unidentified. Nothing is named and nothing is implemented:
this is recorded so the next reader has the layout and the addresses rather than
having to re-derive them. What would retire it: finding the writer of the three
arrays, most likely by breakpointing `0x08849db4` in a live race.

## What is not implemented

- **`pad+0x1a0`'s writer.** Carried inert. See `Pad_ContainsPoint` above.
- **Weapon pads.** The payload decodes identically and is asserted against the
  disc, but nothing consumes one: there is no pickup system.
- **The `"SPEEDUPPAD"` sound.** There is no audio system yet, so neither emitter
  path is reproduced. `oag_game::race` arms the flare and nothing else.
- **`craft+0x318`'s curve.** See the open question above.

## History

- **2026-08-10** - `Pads_TestCraft_q` renamed `Pads_TestCraft`, 65 -> 90. Found
  the writer of `world + 0x40` / `world + 0x108`: `World_LoadTrack` ->
  `World_CollectNodeLists` -> a generic class-filtered scene-graph walk
  (`FUN_08a6ed64`) keyed on a new pair of names, `SpeedupPad_ClassTag`
  (`0x08a6be7c`, 90) and `WeaponPad_ClassTag` (`0x08a6ed40`, 85), each tied to
  its class by the `Vex_RegisterClass(..., 0x3bd)` / `(..., 0x3be)` call that
  sets it. The list is `Speedup Pad` nodes only, structurally, not one shared
  list a caller filters - closing the "one list holding both pad classes"
  possibility this page carried since creation.
- **2026-08-03** - page created. `Pad_Bind` and `Pad_ContainsPoint` at 85,
  `Pad_SweptTest_q` and `Pads_TestCraft_q` at 65. Supersedes two readings in
  `engine.md`: the record `Ship_ApplySpeedupPad` locates is a pad's **matrix**
  rather than a track "section", and the counters it bumps on entering a new pad
  are pad-visit statistics rather than lap and sector counting.
- **2026-08-03, second pass** - `ExhaustFlare_OnSpeedupPad` (`0x08904f10`) added
  at 88, which closes "what fires the boost visual". It **corrects this
  project's own reimplementation**: an earlier revision armed the flare every
  tick inside the pad with `<SpeedupPads time>`, on the reasoning that the flare
  and the force should expire together. The original arms it once, on the entry
  edge, with a `0.8 s` code literal, so the flare outlives the force on every
  speed class. Also new: `racer+0x78` is the craft's `Engine Flare` object, mode
  `2` is a selector distinct from Zone's `6`, and `craft+0x318` drives an inert
  keyframe curve recorded above as an open question.
