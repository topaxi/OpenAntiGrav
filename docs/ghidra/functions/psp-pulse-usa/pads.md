# Pads

How a `Speedup Pad` is bound, how a craft is tested against one, and what the
test hands back. The force the hit then applies is
[`Ship_ApplySpeedupPad`](engine.md#the-track-section-force-is-the-speed-pad-boost-and-it-is-the-wrong-sign)
in `engine.md`; this page is the trigger side.

The data side - the payload layout and what the shipped tracks author - is
[`docs/formats/pads.md`](../../../formats/pads.md). The reimplementation is
`crates/vex/src/pads.rs`.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x089264f4` | `Pad_Bind` | 85 |
| `0x088866bc` | `Pad_ContainsPoint` | 85 |
| `0x0888686c` | `Pad_SweptTest` | 88 |
| `0x08887144` | `Pads_TestCraft` | 90 |
| `0x0888727c` | `WeaponPads_TestCraft` | 90 |
| `0x08a6be7c` | `SpeedupPad_ClassTag` | 90 |
| `0x08a6ed40` | `WeaponPad_ClassTag` | 85 |
| `0x089265f0` | `Pad_UpdateRefreshTimer` | 90 |
| `0x0892c034` | `WeaponPad_UpdateRefreshTimer` | 90 |

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
latch - see `Pad_SweptTest` below.

### The literals are only meaningful because the scale is 1

The `Mesh` bind divides the same `+0x10`/`+0x20` floats by a quantisation scale
before storing its own bounds at `+0x80`. `Pad_Bind` does **not**: it copies them
raw and adds `-2.0` and `+8.0` in world units. That is only coherent if a pad
mesh's scale is `1.0`.

Checked rather than assumed: all 544 `Speedup Pad` boxes on the 40 PSP track
files come out between **9.55 and 9.71 units wide**, which is a craft's width and
one authored size. A quantisation scale would vary per mesh and would put these
orders of magnitude out. Asserted in
`crates/vex/tests/pads_ground_truth.rs`.

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
   geometrically inside, and it is a countdown, not a flag - see
   `Pad_UpdateRefreshTimer` below.

Confidence **85**: unambiguous decompilation, and the direction reading is
corroborated by the pads' own geometry being flat plates whose local `+Z` runs
along the track.

### Retired: `pad+0x1a0`'s writer is `<WeaponPad refresh_time>`, exactly as guessed

**Confidence 90.** The hypothesis this page carried since creation - "a
`Weapon Pad`'s `<WeaponPad refresh_time>` is the obvious candidate" - is
confirmed exactly, plus one detail nobody guessed: a second, mode-gated value.

`Xml_ReadGlobalSettings` (`0x0883aa14`, already named) parses `<WeaponPad>`'s
two float attributes into per-skill-class global arrays, not into any pad:

```c
if (Xml_AttributeNameIs(attr, "refresh_time"))
    (&DAT_08b34328)[g_handling_parse_class] = Xml_AttributeAsFloat(attr);
if (Xml_AttributeNameIs(attr, "elimination_refresh_time"))
    (&DAT_08b34338)[g_handling_parse_class] = Xml_AttributeAsFloat(attr);
```

**`WeaponPads_TestCraft`** (`0x0888727c`) is the writer, and it is the exact
mirror of `Pads_TestCraft` for the `+0x10c`/`+0x1d4` `Weapon Pad` list this
page's other retired section documents - same swept test, same per-racer
previous-position cache, but on a hit it stamps the pad:

```c
if (DAT_08ab07e3 == '\0' && DAT_08b31048 == 8)
    value = (&DAT_08b34338)[DAT_08b31040];   // elimination_refresh_time
else
    value = (&DAT_08b34328)[DAT_08b31040];   // refresh_time
*(int *)(pad + 0x1a0) = value;
```

(`DAT_08b31040` is the speed-class index this page already names below, in
`ExhaustFlare_OnSpeedupPad`'s mode-constant paragraph.)

`DAT_08b31048 == 8` is a **third** mode literal distinct from Zone's `6`
([zone-mode.md](zone-mode.md)) and the `2` this page's `ExhaustFlare_OnSpeedupPad`
section already flags as unidentified - `8` reads as Eliminator, Pulse's other
pickup-centric mode, and would explain why only this one pad interaction cares
about it.

**`pad+0x1a0` counts back down every tick**, in the class's own `update`
method - the method-table slot at `+0x24` of `node+0x38` that
[`exhaust.md`](exhaust.md) established, corroborated here on a second class
table (`Speedup Pad`'s `DAT_08adaa50`, `Weapon Pad`'s `DAT_08ad23ec`, both
found off the pad node constructors' own `+0x38` store). Read directly off the
disassembly, both classes' slots decode without a decompiler needed:

- **`Pad_UpdateRefreshTimer`** (`0x089265f0`, `Speedup Pad`'s slot) is
  `pad->0x1a0 = max(0.0, pad->0x1a0 - dt)` and nothing else - inert, since
  nothing ever writes a `Speedup Pad`'s `+0x1a0` non-zero. This is what makes
  `pad+0x1a0` a field on the shared base rather than something `Weapon Pad`
  privately owns.
- **`WeaponPad_UpdateRefreshTimer`** (`0x0892c034`) does the same decrement,
  then, while cooling down, packs a fixed `0x3f3f3f` grey into `pad+0x6c`; once
  the timer reaches zero it instead cross-fades `pad+0x6c` through a small
  colour keyframe table at `1/3`-second steps (`param+500` ramping at `dt*3`,
  indexing `pad+0x1f0`) - the pad's ready-to-collect colour cycle. `pad+0x6c`
  is a full colour *replacement*, not a modulation of the mesh's own baked
  vertex colour - `Pad_Bind` calls `Mesh_Bind` first, which already builds
  the GE colour commands from the file's own materials, and the timer
  overwrites that slot every tick regardless.

  **`WeaponPad_ColourKeyframes` (`0x08ac00c8`), the table itself, is now
  named, addressed and read.** `WeaponPad_UpdateRefreshTimer` builds the
  pointer from a bare `lui`/`addiu` pair (`0x0892c0b0`/`0x0892c0b8`,
  immediate `0x002bc0c8`) that Ghidra's loader never relocated, so
  `get_xrefs_to`/`get_xrefs_from` find nothing at either end. Image base
  (`0x08804000`) plus that immediate predicts `0x08ac00c8`, inside `.data`
  (`08ab0600`-`08ad9717`); `read_memory` there returns 72 clean bytes - six
  `[r, g, b]` triples, `0..255` per channel, stride 12, matching the
  disassembly's own `* 0xc` and wraparound-at-5 arithmetic exactly - and
  `search_byte_patterns` for the first entry's 12 bytes
  (`00 00 4c 43 00 00 cc 42 00 00 cc 42`) finds **exactly one** match in the
  whole image, at that address. Confidence **85**: exact agreement with
  shipped data plus a structural prediction (stride, entry count, section),
  capped below a live trace because the relocation is inferred rather than
  read. Values: `[204,102,102]`, `[121,210,121]`, `[121,121,210]`,
  `[210,210,121]`, `[140,216,216]`, `[216,140,216]` - a rainbow cycle,
  reproduced in `oag_render::weapon_pad`.

`crates/vex/src/pads.rs` can drop "carries the term inert" - the term is
identified, just still unconsumed because there is no pickup system yet (see
"What is not implemented" below).

## `Pad_SweptTest` (`0x0888686c`)

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

**The `moved < 25.0` guard is a teleport reject, confidence 88, measured live
against the running original.** `Pads_TestCraft` and `WeaponPads_TestCraft`
are the *only* writers of the per-racer previous-position cache
(`world + slot*0x10 + 0xb50`) - every other reader of `DAT_08b32c88` (thirteen
call sites checked) either reads a different field entirely or, in the two
constructor cases, zeroes the whole block once at race start - so nothing
resets this cache on a respawn separately, and whatever teleports a craft
leaves last tick's real position sitting in `previous`.

Measured 2026-08-10, PPSSPP v1.20.4 (SDL, `--graphics=gles` under Xvfb),
`pulse-psp-usa.iso`, Talon's Junction: broke at the followed craft's own
`Ship_UpdateCraft` entry, wrote a 200-unit jump into `body+0x030` (the same
mechanism `scripts/psp-drive.py place` uses), then swapped the armed
breakpoint to `Pad_SweptTest`'s entry before resuming, so that same tick's own
pad test ran against the freshly-written position - and read `$f12` (`moved`
at entry, confirmed at the instruction level: `sub.s f14,f14,f12` at `+0x14`
consumes it first, then `lui 0x41c8`/`mtc1`/`c.lt.s f12,f14` at `+0x68`-`+0x70`
is the `25.0` compare itself):

| Condition | `moved` (all pads that tick, if any) | Guard's own branch |
| --- | --- | --- |
| Craft stationary (start line) | `0.000162` | neither call site reached - the `+0x1d0 > 0.0` distance-cache early-out above fires first, live confirmation of that mechanic too |
| Craft driving, thrust held 2 s | `0.778459` (identical on 6/6 hits) | below `25.0` |
| Craft teleported 200 units in one write | `199.999969` (identical on 6/6 hits, every pad that tick) | at/above `25.0` |

**Independently confirmed by control flow, not just the register value.**
`Pad_SweptTest`'s two calls to `Pad_ContainsPoint` are at different addresses -
the sweep loop's at `0x088869f4`, the direct tail call's at `0x08886aa4` - so
each is its own execution breakpoint. Armed alone (only one execution
breakpoint fires at a time on this build - see `ppsspp-debugger.md`), the
direct-call site (`0x08886aa4`) fired within the same tick as the 200-unit
write; it does not fire during ordinary ticks in the table above.

**What this does and does not retire.** It is a scripted `memory.write`
teleport rather than the game's own fall-off-track respawn firing on its own,
so it does not trace *that specific trigger* end to end - but the guard reads
only `position` and its own cached `previous`, with no way to tell a script's
write from the game's, so the mechanism measured is the one a real respawn
would exercise too. Confidence **88** rather than higher for that reason: a
live measurement of the actual branch, on the actual guard, at the actual
threshold - short of also reverse-engineering and triggering the native
respawn path itself.

## `Pads_TestCraft` (`0x08887144`)

```c
moved = |position - *(world + slot * 0x10 + 0xb50)|;   // since last tick
*out = 0;
hit = false;
for (i = 0; i < *(int *)(world + 0x108); i++)
    hit |= Pad_SweptTest(moved, *(world + i * 4 + 0x40), position,
                           world + slot * 0x10 + 0xb50, out, slot);
*(world + slot * 0x10 + 0xb50) = position;             // for next tick
return hit;
```

Walks a list of pads and remembers each racer's previous position at
`world + slot * 0x10 + 0xb50` - which is where `Pad_SweptTest`'s `previous`
comes from, and why the first tick of a race (`previous == 0`) skips the sweep.

### Retired: `world + 0x40` / `world + 0x108` hold `Speedup Pad` nodes only

**Confidence raised 65 -> 90.** The open question was whether the list
`Ship_ApplySpeedupPad` passes as `world` (`DAT_08b32c88`) is a *speedup*-pad
list, or one list shared with `Weapon Pad` that the caller filters. It is
neither guess: the list is built once, at track load, by a **class-filtered
scene-graph walk**, and the class it filters on is `0x3bd` - `Speedup Pad` -
to the exclusion of everything else in the tree, `Weapon Pad` included.

**A same-day pass narrowed this first, and one of its two claims about
`DAT_08b32c88` itself needs a correction.** It confirmed the global is the
World object rather than a list (fifteen readers, none of them a pad-class
comparison) and ruled out two things worth keeping ruled out:

- **`Pad_Bind` does not register into any list.** It initialises the pad
  node's own box at `+0x1b0`/`+0x1c0` and eight per-racer disable timers at
  `+0x1d0`, and returns. The class distinction is not made at bind time.
- **There is no sibling global.** `0x08b32c8c`, the next word after the world
  pointer, has no references at all.

But it named `FUN_08887094` as "the world constructor" on the strength of it
being *a* writer of `DAT_08b32c88`, and that function's body is
`DAT_08b32c88 = 0` - it **clears** the global, sharing an identical
vtable-stamp preamble (`node+0x38`/`+0x3c`) with the function below, which
reads as a destructor paired with that constructor rather than the
constructor itself. The one that matters for this chain, and the one that
actually sets `DAT_08b32c88` to a live object, is `FUN_08886af4` - traced
below, since it is also where `World_LoadTrack` is called from.

The chain, each link read at instruction level:

0. **`FUN_08886af4`** is the World constructor: it stamps the vtable and class
   tag, sets `DAT_08b32c88 = param_1` (the corrected attribution above), zeros
   the per-racer position caches at `+0xb50`/`+0xbd0` for all eight racers, and
   at its tail calls `World_LoadTrack` on the object it just built.
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
   next-sibling - the same layout `vex.md`'s `Vex_CollectNodesByClass` walker uses),
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
pad classes" two independent ways - structurally, since `FUN_08a6ed64` filters
on the node's own class tag, and behaviourally, since `Ship_ApplySpeedupPad`
applies no class filter of its own to what `Pads_TestCraft` returns (every hit
gets the boost, the flare and the telemetry - a `Weapon Pad` on the same list
would boost the craft, which the game does not do). It does not touch
`pad+0x1a0`'s writer, which this trail never reaches - `0x0892661c` only shows
the field starts at zero. (`Pad_SweptTest`'s own `25.0` teleport-reject guard
was a separate open item on this page and is retired further down, in its own
section, by a live measurement rather than this trail.)

### A weapons-off race neither draws `Weapon Pad`s nor can trigger them

**Not a separate mystery - the same function, a branch this page had not yet
quoted.** `World_CollectNodeLists` builds the `+0x10c`/`+0x1d4` list above
unconditionally, then immediately does this to it:

```c
if (g_weapons_enabled == '\0') {
    for (i = 0; i < *(world + 0x1d4); i++)
        *(uint *)(*(world + 0x10c + i*4) + 0x2c) &= 0xfffffff9;  // clear bits 0x2, 0x4
    *(world + 0x1d4) = 0;   // the trigger list's own count, zeroed
}
```

`+0x2c`'s bit `0x4` is already named on this project's own evidence - the
visibility bit `exhaust.md` reads the boost plume clearing on the same field
(`+0x2c &= ~4`, "hidden from birth"). So a weapons-off race does not merely
skip granting the pickup: it hides the mesh (both bits, `0x2` and `0x4`) and
empties `WeaponPads_TestCraft`'s own loop bound, so the trigger side sees zero
pads for the rest of the race. Bit `0x2` is not independently identified; read
here as gating the same visibility path rather than something else, on the
strength of the two bits always being cleared together.

**`g_weapons_enabled` is `0` for every mode this crate implements**, checked
live against the Custom Race screen for all seven of the original's race
types - see
[shield.md](shield.md#g_weapons_enableds-per-mode-default-spelled-out-and-checked-against-every-race-type),
which is where `Race_ReadSetupOptions` (the writer of the global this branch
reads) is decoded in full. `oag_race::Mode::weapons_enabled` is the port, and
`Scene::new` in `crates/game/src/race/scene.rs` is where the reimplementation makes
the same "decoded, but never uploaded to the GPU" choice this branch does -
see its own doc comment for why the mesh decode itself stays unconditional
where this branch's visibility clear does not.

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

**The new-pad branch also keeps the pad-visit statistics, and their per-lap half is the
`EndRace Results` third column.** For `racer+0x368 == 0` it adds one to a per-race pad total
(`racer+0x8d0`), one to a profile-side counter (`DAT_08b31774 + 0x11c`), and one to
`craft + 0x900 + (lap - 1) * 0x10 + 0x94`, where `lap` is `craft + 0xac8` and the slot is
bounded by `0x14`. A live write watchpoint on that per-lap field (2026-09-30) caught only
`PC = 0x0884910c`, the store of that increment. See
[`endrace-screens.md`](endrace-screens.md#the-third-column-counts-speedup-pads-entered-and-ship_applyspeeduppad-is-its-writer-2026-09-30).
This is a sixth use of `+0x368` that separates the local player from everyone else.

Both branches name the same sound, `"SPEEDUPPAD"`, and pick between two emitters.
Which one is taken splits on `racer+0x368` - **the same field that gates the Zone
score and the pad-visit statistics** in `Ship_ApplySpeedupPad`. Three independent
uses, each separating one craft from the others, is a converging hypothesis that
`+0x368` distinguishes the local player from everyone else. Recorded as a
hypothesis at confidence **45**, which is below the naming threshold, so the
field stays unnamed.

**Two more uses, 2026-08-24, and the hypothesis moves 45 -> 60 - still below the
naming threshold.** Reading positional audio settled what the *other* branch
here actually is: `FUN_0883e9b0` reaches `FUN_0893a768`, a play path that takes
**no emitter at all** and is handed volume `0x400` - the maximum - with pan
zero. So the split is not "two emitters", it is *positional* against *dry and
full*, which is exactly the shape of "the local player's own sound". The fifth
use points the same way: `Exhaust_UpdateEngineSound` multiplies the engine's
volume by `0.85` when `+0x368` is **set**, which is what a mix does to everybody
except the listener. Five converging uses is more than three, and it is still
not a reading - the field is never written anywhere this project has looked, so
it stays unnamed. See [positional-audio.md](positional-audio.md), and
`oag_sound::sfx::Placement::CraftUnlessPlayer`, which is the one line that
moves if this turns out to mean something else.

**2026-09-01: the write site, and it settles this.** `Craft_Construct_q`
(`0x08840c74`) writes `*(craft + 0x368) = param_2` unconditionally at
construction - `param_2` also drives three other branches in the same
function (`param_2 < 1`, `< 2`, `< 4`), so it is the constructor's own
"controller class" argument, not something inferred from later behaviour. A
live single-race capture read it off all eight racers by the correct path
(`Ship_UpdateCraft`'s `a0` dereferenced through `+0x1c4` to the entity
`racer+0x368` actually lives on - reading `a0+0x368` directly, without that
step, returns an unrelated field and was the first, wrong attempt here):
**the human-controlled craft read `0`, and all seven AI opponents read `2`,**
with no other value seen. That is the write site plus a clean 8-for-8 live
read, well past the naming threshold - confidence **85**. Named
`controller_class`, and read now: `oag_trace::trace::Frame::controller_class`
(2026-09-02), wired through `scripts/psp_trace_fields.py`'s already-updated
capture list the same way every other optional column is. `0` and `2` are the
only values confirmed live, so whatever `1` and `3` select (multiplayer? a
second local pad?) is still open, and `oag_sound::sfx::Placement::CraftUnlessPlayer`'s
`== 0` check is confirmed rather than merely uncontradicted.

**`DAT_08b31048 != 2` is not the Zone check.** This project already identifies
`DAT_08ab07e3 == 0 && DAT_08b31048 == 6` as the Zone-mode selector
([zone-mode.md](zone-mode.md)). Here the same pair is tested against **2**,
verified at instruction level (`xori a3, a3, 0x2`). So mode `2` is a second
mode - **`Demo`**, the attract-mode race, per the enum on
[state-machine.md](state-machine.md) - and it must not be folded into the Zone story. Note in passing
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

- ~~**The `Weapon Pad`'s colour cycle, `pad+0x1a0`/`pad+0x6c`.**~~ **Implemented
  2026-08-17.** The pickup system landed 2026-08-11 and consumes the trigger
  and the refresh timer - see [pickups.md](../../../gameplay/pickups.md). The
  *look* is `oag_render::weapon_pad` (the recovered grey and
  `WeaponPad_ColourKeyframes` table, above) plus
  `oag_game::race::drawable::Drawable::tint_weapon_pads`, which recolours
  each pad's own vertices by its refresh timer every frame. One simplification,
  recorded rather than silent: the original only advances a pad's phase while
  *that* pad is ready, so two pads that became ready at different moments are
  out of phase with each other; this port has no per-pad phase to resume from
  and samples every ready pad at the race clock instead, which cross-fades at
  the right rate but keeps them in lockstep. See `oag_render::weapon_pad`'s
  own doc comment.
- ~~**The `"SPEEDUPPAD"` sound.**~~ **Played**, off `hud.bnk`, on exactly the
  edge this function arms the flare on - `oag_game::race::pads` raises the cue
  beside the `exhaust[slot].boost(...)` call. ~~What is *not* reproduced is the
  two-emitter split.~~ **The split is reproduced too, 2026-08-24**: every craft
  raises its own cue carrying its own slot, the player's is played dry at full
  volume and a rival's goes through its craft emitter at
  `oag_audio::Emitter::CRAFT_RADIUS`. The branch rides the `+0x368` hypothesis
  above and says so at the one line that implements it. See
  [positional-audio.md](positional-audio.md), `oag_sound::sfx` and
  [psp-audio.md](../../../formats/psp-audio.md#a-cue-owns-a-run-of-the-command-table).
- **`craft+0x318`'s curve.** See the open question above.
- **Mode `8`.** Read as Eliminator by elimination, not confirmed independently
  - see `Pad_ContainsPoint` above.

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
- **2026-08-10, second pass** - found the writer of `pad+0x1a0`:
  `WeaponPads_TestCraft` (`0x0888727c`, 90, the `Weapon Pad` mirror of
  `Pads_TestCraft`), stamping `<WeaponPad refresh_time>` (or
  `elimination_refresh_time` under a newly-seen mode `8`) on a hit, parsed by
  the already-named `Xml_ReadGlobalSettings`. `Pad_UpdateRefreshTimer`
  (`0x089265f0`, 90) and `WeaponPad_UpdateRefreshTimer` (`0x0892c034`, 90)
  count it back down every tick and, for `Weapon Pad`, drive an unimplemented
  `pad+0x6c` colour cycle - confirming the hypothesis this page carried since
  creation exactly, including the detail nobody guessed. Also checked, without
  retiring it: `Pads_TestCraft`/`WeaponPads_TestCraft` are the only writers of
  the per-racer previous-position cache `Pad_SweptTest`'s `25.0` guard reads,
  which rules out a separate respawn-reset path but was not yet the live trace
  the confidence called for - see the third pass below.
- **2026-08-10, third pass** - `Pad_SweptTest_q` renamed `Pad_SweptTest`,
  65 -> 88, against a live PPSSPP session (Xvfb, `--graphics=gles`, SDL build).
  Measured `$f12` (`moved`) at the function's own entry across three regimes -
  stationary (`0.000162`), driving under thrust (`0.778459`), and a scripted
  200-unit same-tick teleport (`199.999969`, at/above the `25.0` compare) - and
  independently confirmed which of the two `Pad_ContainsPoint` call sites
  (`0x088869f4` sweep, `0x08886aa4` direct) executes in each case by arming
  each as its own execution breakpoint. The teleport reliably takes the direct
  path; ordinary movement, including the stationary case, never reaches either
  call site because the `+0x1d0 > 0.0` distance-cache early-out this page
  documents above fires first - a live confirmation of that mechanic too, not
  only the `25.0` one.
- **2026-08-10, fourth pass** - documented `World_CollectNodeLists`'
  `g_weapons_enabled` branch, previously quoted on this page with that part
  cut: a weapons-off race clears every `Weapon Pad` node's visibility bits
  (`+0x2c`, the same field `exhaust.md` names for the boost plume) and zeroes
  the trigger list's own count. `g_weapons_enabled` is `0` for every mode this
  crate implements - see [shield.md](shield.md), where the writer
  (`Race_ReadSetupOptions`) is checked live against all seven of the
  original's race types. Ported as `oag_race::Mode::weapons_enabled`, gating
  `Scene::new`'s upload rather than the mesh decode - see that function's own
  comment for why the two are deliberately different.
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
