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

80 rather than 84 because only the `(4, 1)` branch was actually read; the other
inlined cases at `0x000389ec`, `0x00038a48`, `0x00038acc`, `0x00038ae8` and
`0x00038af4` were not. Reading them would raise this row, and would document
the shape-type enum.

## Not recorded

- **`0x00034438`.** Byte-identical to `Collision_Construct` (same opcode hash
  `3b9563d3…`, 105 instructions, 420 bytes) and referenced by nothing but its
  own OPD entry at `0x00871140`. It is the unused half of the C++ ABI's
  constructor pair. Which half cannot be told from identical bodies, so naming
  it would assert something not observed.
- **`0x000345e0`, `0x00037438`.** Both touch the collision context and both sit
  inside the `Collision.cpp` address range, but neither was read.
- **`0x00038c00`.** The buffer-carving helper the constructor calls five times
  with the allocation sizes. Clearly meaningful, and one reading of it is not
  better than a guess yet.
- **The shape-type enum.** Seven values reach the switch in
  `Collision_ProcessPairs`; only `4` and `1` have been read, as the pair that
  reaches `Collision_TestMeshAabb`. The other five branches are inlined in that
  function at `0x000389ec`, `0x00038a48`, `0x00038acc`, `0x00038ae8` and
  `0x00038af4`. The obvious next hour of work on this subsystem, and it names a
  data type rather than a function.
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
