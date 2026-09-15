# Collision: the arena, the class, and the MeshAABB narrowphase

The second sweep of `EBOOT.elf`, 2026-08-17. Eight names, all from
`Collision.cpp`, recovered from the `__FILE__` tag the allocation macro leaves
at every call site plus the class's own vtable.

Read [memory.md](memory.md) first, and in particular its section on the
per-function TOC. Every reading on this page was checked against that defect
before it was written down.

## Why these were findable, and why the TOC defect does not reach them

Two anchors, both cheap:

- **`"Collision.cpp"` at `0x0077afe0`**, reached through TOC slot
  `0x008a5ec8`. Raw bytes confirmed:
  `436f6c6c6973696f6e2e63707000` = `Collision.cpp\0`.
- **The base class stores its own `__FILE__` at object offset `0x30`.** Both
  constructors on this page and the ones in
  [race-manager.md](race-manager.md) run `param_1[0xc] = <its .cpp string>`
  right after chaining to the shared base constructor at `0x003271d8`. Two
  different classes, same offset, each holding its own filename. That makes the
  attribution a property of the object, not of one allocation call.

memory.md establishes that Ghidra uses one TOC (`0x008ad4d8`) for the whole
program while the framework's own functions use `0x008bd3c4`, so a Ghidra
string cross-reference in this binary can be a plausible wrong string. **Every
function named here was checked against its own OPD entry and every one
declares TOC `0x008ad4d8`**, which is the value Ghidra already assumes - so
their string and TOC reads resolve correctly. The OPD block at `0x00871110`
holds them in address order:

```
00871118: 00033dd0 008ad4d8      00871150: 00034aa0 008ad4d8
00871128: 00034168 008ad4d8      00871158: 00034c48 008ad4d8
00871138: 00034350 008ad4d8      00871160: 00034dc8 008ad4d8
00871140: 00034438 008ad4d8      00871168: 00034f40 008ad4d8
```

Independent corroboration of the two-TOC model: the call stub `0x006761f8`
reads `addis r2,r2,0x1 ; subi r2,r2,0x114 ; b 0x00391c58`, adding exactly
`0xfeec` - the difference between the two TOC bases - on the way into the
framework. Game code really is on `0x008ad4d8`.

**A full walk of the OPD - 27,127 entries - finds exactly two TOC values, and
their code ranges overlap.** `0x008ad4d8` covers 11,037 functions spanning
`0x010200`-`0x758110`; `0x008bd3c4` covers 16,090 spanning
`0x32d5e0`-`0x7579c0`. So the two tails are decidable from the address alone
and the middle is not:

| Address range | TOC | Is Ghidra right? |
| --- | --- | --- |
| below `0x32d5e0` | always `0x008ad4d8` | yes - read its xrefs directly |
| `0x32d5e0`-`0x758110` | either | **must check the OPD per function** |
| above `0x758110` | always `0x008bd3c4` | no - always wrong |

Everything named on this page and on [race-manager.md](race-manager.md) lies
between `0x00033dd0` and `0x0005d138`, entirely inside the safe tail. The
per-function checks above were made before that boundary was known and are kept
because they are the primary evidence; the table is the cheaper route for
anyone working the same range later.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00033dd0` | `Collision_AllocateArenaBlock` | 84 |
| `0x00034168` | `Collision_FreeBuffers` | 82 |
| `0x00034350` | `Collision_AllocateBuffers` | 84 |
| `0x00034aa0` | `Collision_Construct` | 80 |
| `0x00034c48` | `Collision_DestructAndFree` | 80 |
| `0x00034dc8` | `Collision_Destruct` | 80 |
| `0x00034f40` | `Collision_TestMeshAabb` | 84 |
| `0x00038428` | `Collision_ProcessPairs` | 80 |
| `0x000345e0` | `Collision_AddObject` | 80 |
| `0x00037438` | `Collision_TestMeshObb` | 82 |

**84 is the ceiling for this whole sweep.** The rubric caps "decompilation
only, consistent call sites" at 84, and nothing here has a runtime trace or a
second binary. Where a row scores lower, the section says which leg is missing.

The sweep landed fourteen names across this page and
[race-manager.md](race-manager.md), against a brief asking for five to ten.
**Six of the fourteen are C++ ABI boilerplate rather than behaviour** - the
constructor and destructor sets of two classes. They came nearly free once the
vtable-slot pattern was established on the first class and held on the second,
and they are worth landing because they anchor which object each behavioural
function operates on. Four names describe actual behaviour.

A follow-up pass, 2026-09-02, picked up the "Not recorded" leftovers from that
sweep: `Collision_AddObject` and `Collision_TestMeshObb` are new names; the
shape-type enum section below is new; `0x00038c00` was read and is described
but deliberately left unnamed.

## The collision context struct

Three of the names operate on one global structure, reached by dereferencing
TOC slot `0x008a5e9c`. It is **not** the collision object the constructor
builds; the object registers itself into it at `+0x28` and `+0x2c`. Offsets
observed, and where each came from:

| Offset | Meaning | Observed in |
| --- | --- | --- |
| `+0x00` | arena bump offset | `Collision_AllocateArenaBlock` |
| `+0x04` | arena base, `0x400000` bytes | `Collision_AllocateBuffers` line `0x86` |
| `+0x08`..`+0x18` | five further buffers | `Collision_AllocateBuffers` lines `0x87`-`0x8b` |
| `+0x1c` | max MeshOBB tests in one frame | `Collision_Destruct` report |
| `+0x20` | max MeshAABB tests in one frame | written by `Collision_TestMeshAabb` |
| `+0x24` | max triangles handed to the MeshAABB callback | `Collision_Destruct` report |
| `+0x28`, `+0x2c` | the live collision object, cleared on destruction | `Collision_Construct` |
| `+0x30`, `+0x34` | this frame's test counters, reset every pass | `Collision_ProcessPairs` |
| `+0x38` | broadphase pair list, `u16` pairs | `Collision_ProcessPairs` |

### `Collision_AllocateBuffers` / `Collision_FreeBuffers`

`0x00034350` makes six allocations in a row through the allocator stub
`0x00676a78`, whose shape memory.md established as
`(size, alignment, file, line)`:

```
lwz  r28,-0x7610(r2)          ; r28 = "Collision.cpp"
0x400000, 0x80, r28, 0x86  -> stw r3,0x4(r29)
0x90,     0x80, r28, 0x87  -> stw r3,0x8(r29)
0x10,     0x80, r28, 0x88  -> stw r3,0xc(r29)
0x11b40,  0x80, r28, 0x89  -> stw r3,0x10(r29)
0xb440,   0x80, r28, 0x8a   -> stw r3,0x14(r29)
0x4000,   0x80, r28, 0x8b   -> stw r3,0x18(r29)
```

`r29` is `*(0x008a5e9c)`, the context above. Lines `0x86`-`0x8b` are
`Collision.cpp:134`-`139`, consecutive, which is what an allocation block in
one function looks like.

`0x00034168` is its exact inverse: six calls to the free stub `0x006761f8`,
on `+0x4`, `+0x8`, `+0xc`, `+0x10`, `+0x14`, `+0x18` of the same context, in
the same order. Neither function has any other statement in it.

The pairing is the evidence, and it is field-by-field. Each has exactly one
caller - `0x00018da0` for the allocate, `0x00018d68` for the free - which is
the shape of a subsystem start/stop hook.

`Collision_FreeBuffers` scores 82 rather than 84 only because "free" rests on
`0x006761f8` being the deallocation counterpart of `0x00676a78`; that stub's
target (`0x00391c58`) sits in the framework's own TOC region and has not been
read as carefully as the allocator was.

### `Collision_AllocateArenaBlock`

`0x00033dd0`, a bump allocator over the `0x400000` arena that
`Collision_AllocateBuffers` reserved:

```c
offset = *(u32 *)ctx;                       // ctx + 0x00
next   = (size + 0x7f & ~0x7f) + offset;    // 0x80-aligned, the arena's own alignment
if (next < 0x400000) { *(u32 *)ctx = next; return offset + *(u32 *)(ctx + 4); }
printf("No coll mem left");
return 0;
```

The bound `0x400000` in the test is **the same constant** as the arena's
allocation size at `Collision.cpp:134`, and the `0x80` rounding is the same
alignment that allocation asked for. Two independent constants agreeing across
two functions is what carries this to the top of the band. The failure string
is real - bytes at `0x0077aec8` are
`4e6f20636f6c6c206d656d206c656674` = `No coll mem left` - and it is
corroboration, not the basis of the name.

22 call sites across `0x00077ab0`, `0x00077f30` and `0x000a2828`. Naming this
one makes those three readable.

### `Collision_Construct`

`0x00034aa0`. Chains to the shared base constructor `0x003271d8`, installs
vtable `0x00862258`, writes the `"Collision.cpp"` pointer to `+0x30`, stores
`this` into the context at `+0x28` and `+0x2c`, and then walks the buffers
`Collision_AllocateBuffers` produced: `0x90`, `0x10`, `0x11b40`, `0xb440`,
`0x4000` are passed to `0x00038c00` in exactly the order and with exactly the
sizes of allocations `0x87`-`0x8b`. The `0x19c`-iteration loop that follows
strides the `0x11b40` and `0xb440` buffers by `0xb0` and `0x70` bytes -
`0x19c * 0xb0 = 0x11b40` and `0x19c * 0x70 = 0xb440`, both exact. Two array
sizes coming out to the byte is the strongest single check on this page.

80 rather than 84: "constructor" is read off the shape (base-class chain,
vtable install, field initialisation) and not off any symbol.

**A same-tag candidate on `ps4-omega-eu` was checked and not transferred**:
see [`ps4-omega-eu/collision.md`](../ps4-omega-eu/collision.md) - the
`0x19c`-count-times-stride invariant this section's own `0x11b40`/`0xb440`
check relies on has no counterpart in that binary's disassembled candidate,
and the structural evidence that first looked like a match turned out to be
generic tagged-object boilerplate shared across unrelated classes there.

### `Collision_Destruct` / `Collision_DestructAndFree`

The class's vtable at `0x00862258` holds, at slots 12 and 13:

```
00862258 + 0x30: 00871160 -> 00034dc8     (Collision_Destruct)
00862258 + 0x34: 00871158 -> 00034c48     (Collision_DestructAndFree)
```

Neither is called from anywhere else - the only cross-reference to each is its
vtable slot. The two bodies are identical except that `0x00034c48` ends with
`FUN_006761f8(param_1)`, freeing `this`. That is the Itanium C++ ABI's
destructor pair: the complete-object destructor first, the deleting destructor
second. The same pair appears at the same two slots in
[`RaceManager`](race-manager.md), which is why the slot reading is recorded as
a pattern rather than a one-off.

Both restore the vtable pointer, tear down the object list at `+0xf764` in a
`0x8000`-byte stride loop, clear the context's `+0x28`/`+0x2c`, and print five
high-water marks - which is where the context's `+0x1c`/`+0x20`/`+0x24`
meanings in the table above come from. The disassembly pairs them
unambiguously:

```
00034e44: lwz r3,-0x761c(r2)   ; -> 0077af48 "Max MeshOBB tests in one frame : %d"
00034e48: lwz r4,0x1c(r29)
00034e58: lwz r3,-0x7618(r2)   ; -> 0077af70 "Max MeshAABB tests in one frame : %d"
00034e54: lwz r4,0x20(r29)
00034e64: lwz r3,-0x7614(r2)   ; -> 0077af98 "Max triangles sent to MeshAABB
00034e68: lwz r4,0x24(r29)     ;              collision callback in one frame : %i"
```

`0x00676198` tail-calls `0x00455018`, which memory.md closes out as genuinely
`printf`. Both strings and their TOC slots were read as raw bytes, not taken
from Ghidra's labels.

### `Collision_AddObject`

`0x000345e0(collisionObject, object)`. Registers `object` into the object
array `Collision_ProcessPairs` and `Collision_Destruct` both already document
at `collision + 0xf764`:

```c
type  = *(int *)(object + 0xc);                 // shape type, ProcessPairs' own dispatch key
count = *(int *)(collisionObject + 0x17764);     // object count
*(int *)(collisionObject + count*8 + 0xf764) = object;
```

The `count*8 + 0xf764` slot is exactly `Collision_ProcessPairs`' `uVar8*8 +
0xf760`, offset by the same `+4` the reader there uses to reach the pointer
inside each 8-byte entry - two functions agreeing on the same stride and the
same base is the same kind of evidence `Collision_AllocateBuffers` /
`Collision_FreeBuffers` were named from.

Then, keyed on `type`:

- **`type != 1`** (everything except mesh): appends and returns. `collision +
  0x1776c` is set to `1` (a "list changed" flag) and the count at `+0x17764`
  is incremented. If `type == 3` (OBB, see the enum section below) and the
  object's flag word at `+0x78` already has bit `0x4` set, it also calls
  `0x00077718(collision + 0xd750, index, minVec, maxVec)` before returning,
  where `minVec`/`maxVec` come straight from the constant box at
  `PTR_DAT_008a5ecc` - registering the object into the broadphase (`collision
  + 0xd750`) immediately rather than waiting for the next
  `Collision_ProcessPairs` pass.
- **`type == 1`** (mesh): does the same append, but first computes the
  object's own bounding box by reducing over its vertices and hands that box
  to the same `0x00077718(collision + 0xd750, index, minVec, maxVec)` call.
  The vertex source is `*(u32 *)(object + 0x8c)`, the `0xc`-byte-stride vertex
  table `Collision_TestMeshAabb` step 5 already documents, walked
  `*(u16 *)(object + 0x98)` times with a per-component min into one
  accumulator and max into another (`vectorConditionalSelect` against the
  running accumulator, one call per axis). So the OBB and mesh branches feed
  `0x00077718` the same two-vector shape, one supplied by the caller and one
  computed here; `Collision_ProcessPairs` step 2's `0x00077860(collision +
  0xd750, index, minVec, maxVec)` is the same call shape used to *update* an
  already-registered object rather than insert a new one. A second counter at
  `collision + 0x17768` (distinct from the object count at `+0x17764`) also
  gets a `meshData + 0x600` pointer appended to the buffer at `ctx + 0x18` -
  one of the five buffers `Collision_AllocateBuffers` reserves - but what that
  particular offset holds was not chased further.

80 rather than higher: the overall append/count/flag pattern is corroborated
against two other documented functions, but the `type == 1` vertex-reduction
path and the `ctx + 0x18` bookkeeping were read only here, with no second
function to cross-check them against.

Six call sites across unrelated subsystems (`0x00287168`, `0x000ddd58`,
`0x000dfd90`, `0x0010c958`, `0x00281570`, `0x002eaba8`) each construct a new
object and register it here, which is the shape of a shared "add this to the
collision world" entry point rather than a Collision.cpp-internal helper.

### `Collision_TestMeshAabb`

`0x00034f40(collisionObject, meshObject, aabbObject)`. Structure first, string
second:

1. Increments the context's `+0x30` and maxes the result into `+0x20`, the
   field `Collision_Destruct` reports as **"Max MeshAABB tests in one frame"**.
   `Collision_ProcessPairs` zeroes `+0x30` once per pass, so `+0x30` really is
   per-frame. Reset site, increment site and report site all agree on the same
   two offsets of the same struct, and that agreement is independent of what
   the report is printed *with*.
2. Bails when that count exceeds the budget in `*(0x008a5ed0)` (`0x008c143c`).
3. Loads a second structure, `meshData = *(u32 *)(mesh + 0x88)`, and projects
   the two vectors at `aabb + 0x10` and `aabb + 0x20` onto the two axis vectors
   whose addresses sit at `meshData + 0x6f4` and `meshData + 0x700`
   (`vmaddfp` then three horizontal adds each). The four resulting scalars are
   compared against the floats pointed to by `meshData + 0x6f8`, `+0x6fc`,
   `+0x704` and `+0x708` - a min/max overlap test on both axes.
4. On overlap, calls `0x00078ea8(meshData, lo, hi, indexBuffer, 0x1cc)`, which
   returns a triangle count into a `u16` index buffer at `collision + 0xd3b4`.
5. Expands each index through a `6`-byte triangle table whose base is
   `*(u32 *)(mesh + 0x94)` and a `0xc`-byte vertex table whose base is
   `*(u32 *)(mesh + 0x8c)` into 16-byte-aligned `float4`s (`w = 1.0`) in a
   buffer at `collision + 0x17780`, writing the count to `collision + 0x19780`.
   Note the two bases hang off `mesh` itself, while step 3 and 4 work on
   `meshData` one indirection further in.
6. Dispatches through the function pointer at `aabb + 4` with
   `(aabb + 8, collision + 0x17780)`.

Step 6 is literally "sending triangles to the MeshAABB collision callback",
which is the third string the destructor prints. Its only caller is
`Collision_ProcessPairs`, which reaches it when the pair's shape types are
`4` and `1`.

### `Collision_TestMeshObb`

`0x00037438(collisionObject, meshObject, obbObject)`, the OBB counterpart to
`Collision_TestMeshAabb`. The two functions mirror each other field for field:

1. Increments the context's `+0x34` and maxes the result into `+0x1c`, the
   field `Collision_Destruct` reports as **"Max MeshOBB tests in one frame"**
   - the game's own string, and the same reset/increment/report agreement
   `Collision_TestMeshAabb` was named from, just on the OBB's own pair of
   offsets (`+0x34`/`+0x1c`) instead of the AABB's (`+0x30`/`+0x20`).
2. Loads `meshData = *(u32 *)(mesh + 0x88)`, the same indirection
   `Collision_TestMeshAabb` uses, and runs a full slab (separating-axis) test
   against the OBB's four vectors at `obb + 0x60` (center, used unabsoluted),
   `+0x90`, `+0xa0` and `+0xb0` (the three axes, `vectorMultiplyAddFloatingPoint`
   against the mesh axis then summed with `ABS`) - a center-plus-summed-radius
   test against `meshData + 0x6f4`/`+0x6f8`/`+0x6fc` on one axis and
   `meshData + 0x700`/`+0x704`/`+0x708` on the other, the same two mesh axis
   slots `Collision_TestMeshAabb` step 3 reads.
3. On overlap of both axes, calls `0x00078ea8(meshData, lo, hi, collision +
   0xd3b4, 0x1cc)` - the identical call, index buffer and constant
   `Collision_TestMeshAabb` step 4 uses - and on a nonzero triangle count,
   tail-calls `0x0003a3d0(obb)` to do the equivalent of that function's steps
   5-6 (triangle expansion and callback dispatch). `0x0003a3d0` was not read.

Reached from `Collision_ProcessPairs`' shape-type dispatch at both `(1, 3)`
(`0x00038b4c`) and `(3, 1)` (`0x00038bb4`), each normalizing the argument
order to `(collisionObject, meshObject, obbObject)` regardless of which slot
in the pair held which type - the same dual-order-converges-on-one-call-shape
evidence the enum section below uses for type 5.

82 rather than 84: every step up to the triangle count matches
`Collision_TestMeshAabb`'s already-84-rated reading exactly, but the final
tail call into `0x0003a3d0` was not read, so the last step is inferred from
symmetry rather than observed directly.

### `Collision_ProcessPairs`

`0x00038428(dt, collisionObject, outCount)`, the per-frame narrowphase pass:

1. Stores `dt` at `collision + 0x17774` and accumulates it into `+0x40`.
2. Walks the object array at `collision + 0xf764` backwards over the flag word
   at each object's `+0x78`: `& 1` clear ends the walk, and `& 4` set runs the
   broadphase update for that object. Two distinct tests on the same word.
3. Builds a candidate pair list of `u16` pairs into `ctx + 0x38`, capped at
   `0xffff` entries, and calls `0x00077330` to get the final pair count.
4. When the byte at `collision + 0x19790` is set, Fisher-Yates-shuffles the
   list using `FUN_006762f8` and then quicksorts it through `0x0067ca98`.
5. **Zeroes `ctx + 0x30` and `ctx + 0x34`** - the per-frame counters.
6. For each pair, looks both objects up, reads the shape type at `+0xc`, and
   dispatches: `(4, 1)` goes to `Collision_TestMeshAabb`, anything below `7`
   goes through a switch. An out-of-range index is reported through the string
   at TOC slot `0x008a5f00`, which holds `0x0077aff0`, `"Bad collision object
   ID returned from broadphase. numObjects = %x  iObj1 = %x  iObj2 = %x"` -
   and the three arguments the call passes are, in order, the object count and
   the two indices.
7. Writes the resulting contact count to `*outCount`.

**The switch is intra-function, not a table of narrowphase routines.** TOC slot
`0x008a5f04` holds `0x00038824`, and the seven words there are *relative*
offsets - `0x34`, `0x1c8`, `0x34`, `0x2d0`, `0x2c4`, `0x2a8`, `0x224` - which
added to that base give `0x00038858`, `0x000389ec`, `0x00038858`, `0x00038af4`,
`0x00038ae8`, `0x00038acc`, `0x00038a48`. Every one lands inside this
function's own body (`0x00038428`-`0x00038b37`). It is a GCC switch table, the
`bctr` Ghidra warns it could not recover, and types `0` and `2` both branch to
the same skip-this-pair label. So the remaining narrowphase cases are inlined
here rather than being six further functions to name.

80 rather than 84: the dispatch itself and all seven inlined cases are now
read (see the shape-type enum section below), but what several of them hand
off to is not - `0x0003a3d0` (`Collision_TestMeshObb`'s tail call),
`0x00035498` (the `(6, 6)` sub-shape pairwise test), `0x00035760` (the
flag-gated `(3, 3)` index-based call) and the step-4 shuffle/quicksort path
(`FUN_006762f8`, `0x0067ca98`) were identified but not decompiled. This row
tracks the least-read function still reachable from `Collision_ProcessPairs`,
not the dispatch itself.

## The shape-type dispatch is two-level, and type 6 is a compound shape

Following the switch cases this page previously left unread. The picture is
different from the one the switch table alone suggests: the table dispatches on
**the first object's** shape type, and each landing block then compares **the
second object's** type. It is a pair matrix written as nested comparisons, not a
list of narrowphase routines.

Type 1's block at `0x000389ec`:

```
cmpwi cr7,r10,0x4 ; beq 0x00038b94      (1,4)
ble  cr7,0x00038b38                     (1,<4)
cmpwi cr7,r10,0x5 ; beq 0x00038bbc      (1,5)
cmpwi cr7,r10,0x6 ; bne 0x00038858      (1,6), else skip the pair
```

Type 6's block at `0x00038a48` mirrors it, testing `1` then `6` and skipping
everything else.

**Type 6 is a compound shape.** Both blocks read a count from the object's
`+0x150` and loop that many times calling `Collision_GetSubShape(object, i)`,
but they do not call the same pairwise helper with it. `(1, 6)`/`(6, 1)`
(`0x00038a08` and `0x00038b54`) each expand the *type-6* side only and feed
every sub-shape into `0x00036da0` against the type-1 object. `(6, 6)`
(`0x00038a48`, `0x00038a9c`-`0x00038ab0`) is doubly nested - it expands
**both** objects' `+0x150` arrays and calls `0x00035498(collision, subB,
subA)` for every sub-shape pair. `0x00035498` was not read; a previous version
of this section named it `0x00036da0` for the `(6, 6)` case too, which the raw
bytes at `0x00038aa0` do not support - corrected here. Either way, a type-6
object is a container of sub-shapes and the narrowphase expands it before
testing.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00039cc0` | `Collision_GetSubShape` | 82 |

`0x00039cc0(object, index)` is one line - `return *(u32 *)(object + index*0x20 +
0x90)` - a `0x20`-stride array based at `+0x90`. The arithmetic closes: `0x90 +
6 * 0x20 = 0x150`, exactly where the count lives, so the array holds six entries
and the count sits immediately after it. That, plus every call being inside a
loop bounded by that same `+0x150`, is the evidence. 82 and not higher because
an accessor proves the layout it reads and not what the elements *are*; "sub
shape" comes from how the two call sites use the result, one function away.

## The shape-type enum

Reading the three remaining switch blocks (`0x00038acc`, `0x00038ae8`,
`0x00038af4`) and the shared targets they and the earlier blocks branch to
(`0x00038b38`, `0x00038b54`, `0x00038b94`, `0x00038bbc`) fills in what each of
the seven values dispatches to. `Collision_ProcessPairs` keeps its 80; this
section documents a data type, not a function, so it carries its own
per-value confidence instead of a names.tsv row.

| Value | Meaning | Confidence | Evidence |
| --- | --- | --- | --- |
| `1` | Mesh | 84 | Three independently-read functions (`Collision_TestMeshAabb`, `Collision_TestMeshObb`, `Collision_AddObject`) agree on the same layout hanging off a type-1 object: `+0x88` is a `meshData` pointer, `+0x8c` a vertex table, `+0x94` a triangle table. Both `(4, 1)` (`0x00038840`) and `(1, 4)` (`0x00038b94`) normalize the call to put the type-1 object in the `meshObject` argument slot regardless of which side of the pair it was on. |
| `4` | AABB | 84 | `Collision_TestMeshAabb`'s own reading (see above): a two-vector `+0x10`/`+0x20` min/max box. One `Collision_AddObject` caller (`0x00287168`) sets up a new object with vectors at exactly those two offsets before registering it, though the write of that object's own `+0xc` type field was not located, so this leg supports the offsets and not the type value directly. Dead-code check: once dispatch reaches `0x00038ae8` (the type-4 landing block when the object *isn't* already known to be type 1), `r10 == 1` is provably false - the `(4, 1)` case was already peeled off earlier in the same block at `0x00038840` - so `0x00038ae8`'s own `cmpwi r10,1 / bne 0x38858` always takes the branch and `0x00038af0` is unreachable. **AABB only ever collides with Mesh** in this narrowphase; no other pairing is wired up. |
| `3` | OBB (oriented box) | 82 | `Collision_TestMeshObb` bumps `ctx + 0x34` and maxes into `ctx + 0x1c`, the field `Collision_Destruct` labels **"Max MeshOBB tests in one frame"** in the game's own string - independent of the vector-shape reading below. Structurally, the type-3 object's four vectors (`+0x60` center, `+0x90`/`+0xa0`/`+0xb0` axes, the last three `ABS`-summed) are a center-plus-summed-axis-radius slab test, the shape of an oriented box. `(1, 3)` (`0x00038b4c`) and `(3, 1)` (`0x00038bb4`) both normalize to `Collision_TestMeshObb(collision, meshObject, obbObject)`, the same dual-order convergence `Collision_TestMeshAabb` shows for type 4. `(3, 3)` (`0x00038afc`) does not go through `Collision_TestMeshObb` at all - it is flag-gated on `collision + 0x7778` and, when taken, calls `0x00035760(collision, index1, index2)` with the pair's raw indices, not decompiled further. |
| `5` | Tested as a center-plus-radius volume | 68 | `(1, 5)`/`(5, 1)` (`0x00038bbc`, `0x00038acc`) both call `0x00036da0(collision, otherObject, meshObject)`, which reads `otherObject + 0x60` as a single (unabsoluted) center vector and `otherObject + 0x88` as a **scalar** used in the same radius position `Collision_TestMeshObb` used a full box for - a center+radius (sphere) test against the mesh's two axis slots. Lower than 3/4: no corroborating string, `0x00036da0` was read but not renamed (its only other caller is the type-6 sub-shape expansion below, so "sphere" describes the test shape observed here, not necessarily the enum's own name for it). |
| `6` | Compound (container of sub-shapes) | 82 | Already documented above via `Collision_GetSubShape`. |
| `0`, `2` | Not determined | - | Both are the switch table's `0x38858` (skip-this-pair) target in either object slot, and neither is ever tested against anything: type 4's dead-code check above and every other block's exhaustive `cmpwi` chain never matches `0` or `2`. That is enough to say **no pair involving a type-0 or type-2 object ever reaches a narrowphase test in `Collision_ProcessPairs`**, but nothing observed says what the values themselves represent, so they are left unnamed rather than guessed. |

Three more addresses came out of this read and were not renamed:
`0x00036da0` (the sphere-vs-mesh test, called for `(1, 5)`/`(5, 1)` and for
every type-6 sub-shape tested against a type-1 mesh), `0x00035498` (the
type-6-vs-type-6 sub-shape pairwise test), and `0x00035760` (the `(3, 3)`
path, flag-gated on `collision + 0x7778` and called with the pair's raw
indices rather than either object pointer - not a geometry test in the shape
these others are, and not read beyond that). All three are now characterized
enough to be the obvious next names to land on this page.

## Not recorded

- **`0x00034438`.** Byte-identical to `Collision_Construct` (same opcode hash
  `3b9563d3…`, 105 instructions, 420 bytes) and referenced by nothing but its
  own OPD entry at `0x00871140`. It is the unused half of the C++ ABI's
  constructor pair. Which half cannot be told from identical bodies, so naming
  it would assert something not observed.
- **`0x00038c00`.** One line: `*param_1 += param_2`, a byte-usage accumulator,
  not the "buffer-carving helper" an earlier version of this page guessed -
  it returns nothing and computes no address, so it cannot be what hands
  `Collision_Construct` its five buffer pointers (those come straight out of
  `ctx + 0x08..0x18`, written by `Collision_AllocateBuffers`). Every call site
  passes it the exact byte count of an allocation made immediately before it:
  in `Collision_Construct` the five calls are `0x90`, `0x10`, `0x11b40`,
  `0xb440`, `0x4000`, matching `Collision_AllocateBuffers`' own sizes exactly.
  In `0x00077ab0` (one of `Collision_AllocateArenaBlock`'s three callers,
  unrelated to Collision.cpp) the identity is arithmetic, not just
  positional: three `Collision_AllocateArenaBlock(0x234)` calls are followed
  by `FUN_00038c00(counter, 0x69c)`, and `3 * 0x234 = 0x69c` exactly; two
  `Collision_AllocateArenaBlock(lVar17)` calls are followed by
  `FUN_00038c00(counter, (param_2 & 0x3fffffff) << 2)`, and that shifted value
  is `2 * lVar17` exactly. The same function also tracks plain heap
  allocations (`0x70`, `0x3d0` sizes after `FUN_00676a78`) in that same
  caller, so it is not arena-specific either. Left unnamed rather than given
  a `Collision_` prefix: its callers span at least five unrelated functions
  outside `Collision.cpp` (`0x00077ab0`, `0x00077f30`, `0x000a2828`,
  `0x00039a90`, `0x00039b90`, plus `Collision_Construct` and its unused twin
  `0x00034438`), so it is a shared utility with no `__FILE__` tag or vtable of
  its own, and naming it `Collision_*` or inventing a subsystem for it (e.g.
  `Mem_*`) would assert ownership not observed.
- **A false lead worth not re-opening.** Ghidra reports cross-references to
  `"Max triangles sent to MeshAABB collision callback"` (`0x0077af98`) from
  `0x00352f38` and `0x00354738`. Their OPD entries declare TOC `0x008bd3c4`,
  not `0x008ad4d8`, so those references are the wrong-TOC artefact memory.md
  describes and mean nothing. The real reference is in `Collision_Destruct`,
  where Ghidra created no cross-reference at all. **In this database the string
  cross-reference table is wrong in both directions.**

  Both of those addresses sit in the `0x32d5e0`-`0x758110` overlap in the table
  above, where the address does not decide the TOC - which is exactly why they
  had to be checked individually and exactly the case the safe-tail shortcut
  does not cover. A reader who applies the shortcut outside its range will
  reproduce this false lead.
