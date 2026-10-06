# Billboards: `TrackStartup.xml` on PSP, and the transform it writes

Binary: `PSP_GAME/SYSDIR/BOOT.BIN` from `pulse-psp-usa.chd` (`UCUS-98712`), as
`/pulse/BOOT-psp-pulse-usa.BIN` in Ghidra, image base `0x08804000`.

This is the PSP side of the same question
[HD/Fury's `billboards.md`](../ps3-hdfury-eu/billboards.md) answers for that
platform, and [`oag_tables::trackstartup`](../../../../crates/tables/src/trackstartup.rs)
is the shared file-format reader both binaries feed. Where HD's page is
capped at 84 (static reading of one binary, per
[visibility.md](../ps3-hdfury-eu/visibility.md)'s own rule), this one is not:
**it comes from a live PPSSPP debugger session**, 2026-08-28, a halting read
watchpoint on the `%s\TrackStartup.xml` format string
(`0x08a7d008`) that fired once during a real circuit load, followed live with
exec breakpoints on each callee's own `jr ra` and direct memory reads of the
constants involved. `docs/reverse-engineering/ppsspp-debugger.md` documents
the method; nothing here is a cold read of the decompiler alone.

## Why static analysis could not find this on its own

`0x08a7d008` and every sibling string in the same cluster (`TrackStartup`,
`Billboard`, `PI_BILLBOARD`) have **zero** xrefs in Ghidra's database, and a
byte-pattern search for the string's address stored as a pointer found
nothing either. This is not evidence nothing reads them - it is a known PSP
MIPS `$gp`-relative addressing trap, first found chasing `DAT_08b32428` in
the weapon code: a reference built as `lw $reg, offset($gp)` never appears
as an immediate operand or a stored pointer, so an instruction-level sweep
on a global or a far string can read as "nothing touches this" when the
game visibly uses it every race. The only way past it, short of resolving
every function's `$gp`
value by hand, is to watch the access happen.

## The names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x08883794` | function | `World_LoadTrack` | 85 (already named; see [`vex.md`](../../../formats/vex.md)) |
| `0x08953ddc` | function | `Xml_OpenFile` | 92 (already named; see [`xml-reader.md`](xml-reader.md)) |
| `0x08884ac4` | function | `TrackStartup_Parse` | 78 |
| `0x089012d4` | function | `Billboard_CreateFromLocation_q` | 65 |
| `0x08901004` | function | `Billboard_CreateFromColour_q` | 60 |
| `0x08900220` | function | `Billboard_ConstructResource_q` | 75 |
| `0x08a6bb30` | function | `Camera_GetTypeId_q` | 65 |
| `0x08901764` | function | `Camera_Construct_q` | 65 |
| `0x08901850` | function | `Camera_AttachChild_q` | 60 |
| `0x08902324` | function | `Camera_RegisterClass` | 80 |
| `0x08908aec` | function | `GridCamera_RegisterClass` | 85 |

## The call chain, walked live

A halting watchpoint on the string's bytes fired at `pc=0x089753ec`
(`a1=0x08a7d008`), inside a generic printf-style format dispatcher - not
billboard-specific. Its caller, found by placing an exec breakpoint on the
callee's own `jr ra` and reading `ra` there (a stack-offset guess for the
same value gave a plausible but *wrong* address once - trust the breakpoint,
not the frame arithmetic), is a generic `snprintf`-shaped wrapper. **That
function's** caller is `World_LoadTrack`, which builds
`%s\TrackStartup.xml` from `*(int*)(*(int*)(trackDescriptor+0x124)+0x94)` -
almost certainly the circuit's own directory name - as the format's sole
`%s` argument.

Immediately after the snprintf call, still inside `World_LoadTrack`:
`Xml_InitDocument`, then `Xml_OpenFile`, then, conditionally (only if not
already cached), `TrackStartup_Parse`.

**A second, related trap surfaced walking this chain**: several call
targets decompile as `func_0x00XXXXXX(...)`, low pseudo-addresses that are
not real and that `get_function_by_address` cannot resolve. Reading the raw
instruction word at the call site and hand-decoding the MIPS `jal` encoding
(`(next_pc & 0xf0000000) | (word & 0x03ffffff) << 2`) resolves them
correctly. Empirically, `real = pseudo + 0x08804000` - this binary's own
image base - held for every case checked (five calls and one data constant),
which matches the base this project already applies on import but which
Ghidra's own decompiler output is not consistently reflecting for these
particular call targets. Treat it as a cheap shortcut to check against the
byte-level decode, not a rule to trust blindly.

## `TrackStartup_Parse` (`0x08884ac4`): the element walker

A big element-by-element XML walk, tag names compared against literal
constants, with branches matching the schema's other top-level elements
(`LevelFx`'s sounds, `LoadSoundBank`). The `<Billboard>` branch reads
**exactly six** attributes: a 20-byte `type` string, a 4-byte `num` int, a
256-byte `location` string, two more 256-byte reads into the **same**
destination as each other (`color`/`colour` merging into one field -
confirmed live here, matching what this project's `trackstartup.rs` already
documented for the HD/Fury binary from static reading alone, now
independently corroborated on a second title), and a 4-byte `glow` float,
clamped to `[0,1]` then scaled by `255.0`. It then dispatches on whether
`location` is empty: `Billboard_CreateFromColour_q(type_hash, num, colour,
glow)` or `Billboard_CreateFromLocation_q(type_hash, num, location, glow)`.
**`num` is passed through unchanged to both** - the same "index, not
position" shape HD's own `TrackStartup_Load` has.

## The two constructors and the shared object builder

Both `Billboard_CreateFromLocation_q` and `Billboard_CreateFromColour_q`
check `*(int*)(&DAT_00058c28 + num*4) == 0` before proceeding - **`num`
indexes a 9-entry global array directly**, unadjusted, the same
"`array[Num]`" architecture [HD/Fury's page](../ps3-hdfury-eu/billboards.md)
already established from static reading, now confirmed independently on
PSP.

`Billboard_CreateFromColour_q` does one thing the location path does not:
before constructing anything, it walks a pool at
`*(int*)(_DAT_002ae2b4+0x3c)` (count at `+0x40`), matching entries whose
`+0xa0` equals the type hash and whose `+0xa4` bitwise-ANDs against the
colour value, takes the **matched entry's own `+0x94` field** as the string
handed to the shared constructor, and removes the entry from the pool
(swap-with-last). `_DAT_002ae2b4` reads as a per-track context singleton.
**This is the one part of the chain not walked with a breakpoint inside it**
- read from the decompiler only, hence the lower confidence - but it is a
real, reached-during-a-real-load mechanism, architecturally exactly where a
"candidate mount point" list would live if a circuit authors real positions
for its colour-only slots. See "What is still open" below.

Both call `Billboard_ConstructResource_q` (`0x08900220`):

- `*(int*)(&DAT_00058c28 + param_4*4) = param_1` is the array write itself
  (`param_4` is `num`).
- Three IDs are derived from `num` alone -
  `(num-1)*0x1000000 + {0x1100000, 0x1200000, 0x1400000}` - and read as
  material/mesh/layer-style resource tags, the same shape as HD's unnamed
  `0x3e9` type tag, not spatial coordinates.
- It branches on the location string's last three characters, case-folded,
  being `.VEX` or `.MIP` - allocating a 0x1d0- or 0x140-byte object - the
  same dual-branch, dual-size architecture as HD's
  `Billboard_ConstructResource_q` (there `.vex`/`.mip`, `0x2080`/`0x100`).
  Confirmed directly in the decompiler, not only inferred: the case-fold
  reads a byte's `0x28cd21`-offset flag and clears bit `0x20` before
  comparing, the standard uppercase-fold idiom, checked against all three
  characters of both suffixes.

**In the `.VEX` branch, it writes a 4x4 transform into the new object at
`param_1+0x50/+0x60/+0x70/+0x80`** (four consecutive 16-byte rows), sourced
from a fixed global block, `_DAT_0028c7a0` through `_DAT_0028c7dc` (sixteen
consecutive words - confirmed in the decompiler as sixteen literal loads
from consecutive addresses, not sixteen separate computed values). Fifteen
of those sixteen floats are the 4x4 identity matrix
(`[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]`), the same for every billboard
regardless of `num`; `param_1+0x88` is separately set to the bit pattern
`0xc1200000` (-10.0f), also a fixed constant. **Both halves of that were
already recorded on this page** - the finding below is not a new write, it
is that they were described as two independent facts ("the matrix is
identity" and, separately, "`+0x88` is -10.0f") when `+0x88` is actually
*inside* the matrix, which makes the two statements contradict each other
rather than coexist.

**Precision correction, from the raw disassembly rather than the
decompiler's summary (2026-09-06, confidence 92 - read directly off MIPS/VFPU
instructions, not inferred):** `param_1+0x88` is the fourth row's
(`+0x80..+0x8f`) second word - the row's `y` component, not a field outside
the matrix. The disassembly loads all four floats of that row from
`_DAT_0028c7d0` with one `lv.q` (a VFPU quadword load/store - see the trap
below) and stores them to `param_1+0x80`, then immediately overwrites just
`+0x88` with `0xc1200000` via a separate scalar `swc1`. So the row that
lands is `(row3.x, -10.0, row3.z, row3.w)`, not `(0, 0, 0, 1)` - **the
constructed object's translation is not the origin, and the matrix this
function writes is not fully identity; call it "identity rotation, non-zero
translation.y" rather than "identity."** This does not reopen the placement
question in the maintainer's favour: `-10.0f` is a fixed constant, the same
on every call regardless of `num`, `location` or which circuit is loading,
so it cannot by itself carry a given circuit's own start-line coordinates.
Read as a **local** offset - lowering the model's pivot 10 units on whatever
axis row 3 carries translation - it is consistent with (not proof of) the
object needing a *parent* transform to reach its circuit's actual gantry;
see "What is still open" below for why that reframes the question rather
than closing it.

**A decompiler trap, generalisable beyond this one function**: a VFPU
`lv.q`/`sv.q` quadword load/store moves four floats in one instruction, and
Ghidra's decompiler
renders it as four separate scalar assignments with no visual marker that
they came from a single 16-byte transfer. Summarising "sixteen consecutive
scalar loads from consecutive addresses" as "the identity matrix" is exactly
how a same-block, later scalar overwrite (the `+0x88` `swc1` here) reads as
an unrelated, separate fact instead of a partial override of the block just
loaded. Any future read of a transform/matrix write in this binary should
confirm row boundaries with `disassemble_function`, not stop at
`decompile_function`'s summary - this page did, for two passes.

## Construction writes no transform - true for placement, not true for "identity"

`num` (1-8) is used purely as a **resource-slot index** into a 9-entry array
and as the seed for three derived resource-tag IDs at construction time. It
is never used there to select or compute a position, rotation or scale.
What construction writes is **not** a hardcoded identity matrix - see the
correction above - but it is still a fixed constant, identical for every
billboard regardless of circuit, so "construction carries no per-circuit
placement" still stands even though "construction writes identity" does
not.

**What did not stand: reading "construction places nothing" as "nothing
overrides what the track mesh already shows".** That was this page's own
verdict for a few hours on 2026-08-28, and it was wrong - caught by the
project owner, who plays the original and said the start-line gantry does
not show a stretched digit there, only this project's own renderer does.
Checked directly rather than taken on faith: a live PPSSPP screenshot of
Talon's Junction's start line (`16_Track`) shows a green board reading
**"GO"**; `--draws` on this project's own render of the same file shows
node 74 painting the literal `billboard8.tga` digit icon there instead. The
premise held. See "The registered resource is live, not inert" below for
what actually overrides it - the mistake was stopping at construction and
not asking what happens to the object afterward.

## The registered resource is live, not inert

2026-08-28, continuing the same live session. The three tags
`Billboard_ConstructResource_q` derives from `num` are **lookup keys into a
shared registry**, not scratch values: a RAM-wide scan for the literal
4-byte values `0x08100000`/`0x08200000`/`0x08400000` (slot 8's tags) found
all three clustered at `0x08ba79bc`-`0x08ba79dc`, each paired with a pointer
- `(0x09837cb0, 0x08100000)`, `(0x09837cb0, 0x08400000)`,
`(0x0983a240, 0x08200000)`. The immediately adjacent entries carry
`0x07100000`/`0x07200000`/`0x07400000` - slot 7's tags, by the same
`(num-1)*0x1000000 + {...}` formula - confirming this one table is where all
eight billboard slots register, not something slot-8-specific. Two of slot
8's three tags resolve to the same object; the third resolves to a
different one.

**Both resolved objects are read constantly, not once at construction.**
Read/log watchpoints on each (`memory.breakpoint.add`, per
[`ppsspp-debugger.md`](../../../reverse-engineering/ppsspp-debugger.md))
armed through a full countdown-to-green cycle (~44 s), against the craft's
own body position as a live control:

| Object | Hits |
| --- | --- |
| `0x09837cb0` (mesh/layer, from the first two tags) | 12,264 |
| `0x0983a240` (material, from the third tag) | 7,173 |
| the registry table itself (shared, high-traffic) | 791,239 |
| control: craft body position | 15,251 |

- **`0x09837cb0`** is read mainly by a per-object world-transform composer
  (`FUN_088fe208`) and a generic component-update dispatcher
  (`Node_UpdateTree`) that walks a child list and calls two functions through a
  vtable-shaped pointer at `+0x38` - unremarkable scene-graph plumbing,
  confirming this object sits in the ordinary per-frame update tree rather
  than being dead weight.
- **`0x0983a240`** is read overwhelmingly (6,261 of its hits) by one
  address, `0x0890cf34`, which has **no function boundary in Ghidra** -
  reached only through the same kind of indirect/virtual dispatch as
  `Node_UpdateTree`'s vtable calls, which is almost certainly why auto-analysis
  never defined a function over it. Disassembled directly:
  ```text
  lbu   a1, 0xc0(a0)         ; flag byte
  bne   a1, zero, +0x1c      ; skip accumulation if the flag is set
  lwc1  f13, 0x40(a0)        ; f13 = current accumulator
  add.s f12, f13, f12        ; += elapsed time (computed just above)
  swc1  f12, 0x40(a0)        ; write back
  ```
  A **live per-frame animation-time accumulator**, gated by a flag. The same
  object also holds four pointers at a fixed offset table
  (`+0x88/+0x90/+0x98/+0xa0` - `0x08f6f450`, `0x08f6fbd0`, `0x08f6f630`,
  `0x08f6fdb0`), each an identically-shaped wrapper referencing three
  further heap sub-objects. Four state groups gated by a live clock is the
  right shape for a multi-frame countdown display; the sub-objects' own
  content was not decoded, so this is a structural match, not a confirmed
  content match.

**What this settles**: whatever draws at the gantry position very likely
resolves through this registration, not through `billboard8.tga` - the
object is a live scene participant read every frame during exactly the
window the original shows "GO", not a construction-time artefact nobody
reads again. **What it does not settle**: the actual texture bind
(`Gfx_BindTexture`, `0x08928460`) for that screen position was never caught
in the act, so "this object's resolved state is what gets drawn" is strong
circumstantial evidence - co-location, timing, and structural shape all
agreeing - not an instruction-level proof.

## The gantry is a separate instantiated mesh, not a retextured node 74

2026-08-28, a third pass, chasing the actual `Gfx_BindTexture` call for the
gantry's screen position. That specific catch was not made - see "A trap
found trying" below - but a RAM-wide scan for the literal bytes
`321Go_StartFinish` during a live Talon's Junction race found two genuine
hits: a fully parsed, resident `.vex`-shaped scene graph (`VEXX` header, the
source authoring path `Z:/WipeoutPSP/X2/Data/Environments/321_Go/321Go_StartFinish.mb`,
and real node names read forward from it - `world`, `camera1`,
`cameraShape1`, `start_lights`, `start_light_background`), and, separately,
the on-disc resource-table reference `Data\Environments\321_Go\321Go_StartFinish.vex`.

This settles the question the task chain set out to answer, just not by
catching a texture bind: **slot 8's registration instantiates a whole
separate mesh - the same asset `TrackStartup.xml` names as its `location`
on every Pulse circuit that authors a single billboard 8 - rather than
retexturing node 74.** Node 74's `billboard8.tga` quad reads as authored
track-mesh debris that the real gantry (with its own `start_lights` node -
plausibly the animated material object above) covers or the original never
exposes; this project's own renderer shows it only because nothing here
instantiates the real asset yet.

**A tension this does not resolve, and it is the highest-priority open
item now:** `Billboard_ConstructResource_q` writes a hardcoded **identity**
transform, and `321Go_StartFinish.vex` is the same file on all 16 circuits,
so its own geometry cannot carry Talon's Junction's world position baked
in. Something has to move this instantiated object from the origin to each
circuit's own start line, and no writer for that transform has been found.
Until it is, "instantiated" and "drawn at the gantry" are not the same
confirmed claim - the first is now solid, the second is inferred from the
asset's node names (`start_lights` fits a start-line gantry) and from
`TrackStartup.xml`'s own authored intent, not caught directly.

**The material object's liveness was re-confirmed with a non-halting
watchpoint** (53,065 hits on its own header over one countdown-to-green
cycle, alongside a healthy 381,086-hit craft-body control) - it is
genuinely read and written roughly 2,000 times a second, not dead. **A
separate sample of its four `+0x88/+0x90/+0x98/+0xa0` wrapper sub-objects,
taken after the race had already started, found plain front-end credits
markup text** (`"Assistant Lead Artists"`, `<b d="Ta`) - almost certainly
stale heap reuse rather than real content, matching the pool-recycling trap
[`ppsspp-debugger.md`](../../../reverse-engineering/ppsspp-debugger.md)
already documents, but **not re-checked live during an actual countdown**,
so this is flagged rather than resolved either way.

## A trap found trying: halting on a hot function silently voids a timed capture

An execution breakpoint on `Gfx_BindTexture` (`0x08928460`, called roughly
twenty times a frame) produced a plausible-looking, fully-saturated result
- 435 hits across 13 texture pointers, stable from the first hit onward,
reading like a clean "nothing else binds here". It wasn't: each halt costs
about 44 ms and freezes game time completely while halted, so 22 real
seconds of capture covered on the order of 0.3-0.5 s of actual game time,
nowhere near the ~24 s countdown window it was meant to observe.
[`ppsspp-debugger.md`](../../../reverse-engineering/ppsspp-debugger.md)
now documents this as its own trap - never halt on a function called every
frame when the thing being measured is "what happens over N seconds",
even briefly; watch the *object* the function operates on with a
non-halting watchpoint instead.

## `func_0x00267b30`'s type descriptor is the Maya `Camera` node class - not a placement mount point (2026-09-06, seventh pass)

The prior pass's reframed next step - resolve what type
`func_0x00267b30()` (real address `0x08a6bb30`) identifies, since it is the
key `Billboard_ConstructResource_q` searches its own newly-built child tree
for and stores the result of at `param_1+0x40` - is now answered, and the
answer **retires this as a placement lead rather than confirming it**.

**The getter itself.** Disassembled directly (not the decompiler summary):
`func_0x00267b30` is a 3-instruction stub (`lui`/`jr ra`/`addiu`) that
returns a literal 32-bit constant equal to its own address - the standard
"class type-id = the address of a dedicated static function" idiom, used
here with no vtable or name string attached to the getter itself. It sits in
a dense run of at least eight identically-shaped 12-byte stubs starting at
`0x08a6bb30` (`0x08a6bb30`, `3c`, `48`, `54`, `60`, `6c`, `78`, `a8`, each
disassembled directly and confirmed) - a table of per-class type-id getters,
not a one-off.

**Every caller of this exact getter**, found via `search_instructions`
(`mnemonic=jal`, `operand_pattern=267b30` - a program-wide search, not
limited to this function, and the fix for the `get_xrefs_to`
no-PSP-relocation gap this project already knows about): seven call sites in
six functions - `Billboard_ConstructResource_q` itself, plus
`0x08901764`, `0x08901850`, `0x08902324`, `0x08908aec`, `0x0887f9bc` and
`0x08886af4`.

**The class-ID link, the load-bearing part.** `0x08902324` is a static,
argument-less initialiser shaped exactly like this project's own
`DynamicShadowOccluder_RegisterClass`/`Shadow_RegisterClass`
([`shadow-occluder.md`](shadow-occluder.md)) and `exhaust.md`'s
`Vex_RegisterClass` template: it calls `func_0x00104eb8(0x88b08,0xf7)` -
`func_0x00104eb8` is `Vex_RegisterClass` itself (`0x08908eb8`, confirmed by
address arithmetic against [`exhaust.md`](exhaust.md)'s own disassembly
excerpt for the same call shape), called with a **class id of `0xf7`** and a
descriptor pointer (`0x88b08`) whose own `+4`/`+0x28`/`+0x38` fields this
same function fills with `func_0x00267b30()`'s value, the debug-allocator
tag `0x280bf0`, and the method-table pointer `0x2cd18c` - the exact three
fields every instance this getter tags also carries at those offsets.
**Class `0xf7` is `"Camera"`** in the already-established, ground-truthed
Maya class-ID table this project carries in
[`crates/vex/src/vex/class_names.rs:287`](../../../../crates/vex/src/vex/class_names.rs)
and documents in [`exhaust.md`](exhaust.md) at confidence 95 (self-validating
against ten independently-placed IDs). `0x08901764` (`Camera_Construct_q`)
is this class's own instance constructor - the only other caller that
stores the getter's result into its own `+4` "own type" field rather than
using it as a search key - and `0x08901850` (`Camera_AttachChild_q`)
allocates fresh `0xc0`-byte instances of this same class and parents them
onto a caller-supplied owner, the shape of a generic "attach a Camera-typed
child" constructor.

**`0x08908aec` closes an independent loop**: it registers a *different*
class, id `0x3dd` - `"gridCamera"` in the same table
([`crates/vex/src/vex/class_names.rs:897`](../../../../crates/vex/src/vex/class_names.rs))
- and copies `func_0x00267b30()`'s value onto that class's own descriptor
before a second block does the same with a sibling getter
(`func_0x00267ba8`, real address `0x08a6bba8`, independently disassembled
and confirmed as the same 3-instruction stub shape returning its own
address). `gridCamera` is the exact node name `oag-view --nodes` already
found as `start_grid.vex`'s one non-`World`/`Anim Transform` leaf, months
before this pass and for an unrelated reason (ruling `start_grid.vex` out as
a placement source). A Maya `gridCamera` node being a specialisation of the
base `Camera` class, sharing its type-id as a starting point before
overriding parts of its method table, is exactly what `Vex_RegisterClass`'s
own documented "assign the base method table, then overwrite it" pattern
predicts - two independently-reached facts (a class-ID string table read
here, a `--nodes` dump read passes earlier for a different reason) landing
on the same node type is real corroboration, not a coincidence being
over-read.

**`0x0887f9bc`**, one of the two other callers, further corroborates the
class identification from a different angle: its own constructor (a much
larger class, not itself the `Camera` VEX-node class) writes FOV/near/far
-shaped float fields (`0x3f800000` = 1.0, `0x3f000000` = 0.5, angle values
masked to 14 bits and cast to float) immediately around the same
`func_0x00267b30()` search-key construction - the field shape of a camera
*controller* that separately looks up a `Camera`-typed child, not the
`Camera` node itself.

**What this means for `param_1+0x40`.** `Billboard_ConstructResource_q`'s
search runs *after* its `.VEX` branch has already called `Vex_LoadModel`
(`0x08912b80` - already named at confidence 90; `func_0x0010eb80`'s real
address is this, not the `0x08914b80` this pass first miscalculated by a
simple hex-addition slip, corrected before publishing) to load
`321Go_StartFinish.vex` and store the result at the billboard's own `+0x3c`.
`Vex_LoadModel` does not call the child-list insert
(`func_0x00140bd4`, confirmed by direct disassembly at its real address
`0x08944bd4`: append to the singly-linked list at `+0x10`/`+0xc`, the exact
fields the type-search itself walks) anywhere in its own body - but it does
run three separate "collect every descendant of a given type" walks
(`func_0x0026d364` among them, itself confirmed by direct decompile to walk
the very same `+0x10`/`+0xc` list recursively) against 2000/1000/`0x40`
-entry output buffers, which only make sense against a real, populated node
tree - so the tree is built by `Vex_LoadModel`'s own callees
(`func_0x0013f330`'s parse, `func_0x001404d8`'s attach), not by
`Vex_LoadModel` calling the primitive insert directly. **The tree that
results is `321Go_StartFinish.vex`'s own scene graph**, and that file's
node dump (recorded in "The gantry is a separate instantiated mesh" above)
already lists `camera1`/`cameraShape1` alongside `world`/`start_lights`.
**`param_1+0x40` is therefore, at confidence 75, the mesh's own authored
Camera node - not a generic placement mount point the engine attaches to
carry a circuit's coordinates.** This retires the leading candidate from
the prior pass rather than confirming it: a Camera node is markedly less
likely to be "where a circuit's gantry position gets written" than the
generic Transform/Locator this page hypothesised for it two passes running.
Retracted here, not carried forward: that Transform/Locator reading was
this pass's own first hypothesis for several tool calls, built on the
matching field shape of a genuinely different, base-class API
(`func_0x00140544`/`func_0x00141220`/`func_0x00141254`, which really is
this engine's generic parent/local/world matrix accessor triad - just for
the `Transform` base class itself, id `0x6e`, method table `0x08ad22f4` per
`exhaust.md`, not for the `0xf7`-tagged class this getter names). It does
not carry a circuit's own placement either, and nothing in this pass ties
it to `param_1+0x40` specifically.

One correction to this pass's own working notes, so it is not carried into
a later one: applying `+0x08804000` to *every* low `func_0x0XXXXXXX` call
target in the same function briefly looked inconsistent (one attempt landed
on an unrelated, already-named function). That was an arithmetic slip in
this pass, not a defect in the correction - `shadow-occluder.md` already
documents `+0x08804000` as the standard whole-binary unrelocated-constant
offset, and every address in this section resolves correctly under it.

**None of this narrows who writes a per-circuit position for
`321Go_StartFinish`'s own top-level node** (its `world` node, not its
camera) **or for the registry object** (`0x09837cb0`/`0x0983a240`) **the
live capture two passes ago tied to the object actually read every frame
during the countdown.** Those remain the open questions; this pass answers
a different one (what `param_1+0x40` holds) and its answer points away from
where the prior pass hoped it would point.

## Verdict

Construction-time placement is absent (confirmed: a fixed local transform -
identity rotation, translation `(x, -10.0, z, w)`, see the correction above -
plus resource-tag derivation) - but that is not "nothing overrides the disc's
static content", which is false. The registration instantiates a real,
separate, live-updated mesh (`321Go_StartFinish.vex`, matching
`TrackStartup.xml`'s own `location`), read constantly during exactly the
window the original shows "GO" instead of a digit. What is still missing
is the transform that would place that mesh at each circuit's own gantry
rather than at the world origin - without it, "instantiated" is settled and
"drawn where the player sees it" is strong inference, not proof. The
seventh pass (above) resolved what `param_1+0x40` holds - the loaded mesh's
own authored `Camera` node, not a placement mount point - which narrows the
search by retiring a candidate rather than by finding the writer.
[`crates/tables/src/trackstartup.rs:173`](../../../../crates/tables/src/trackstartup.rs)
carries the same "literal identity matrix" wording this page used to (not
edited here - out of this lane's scope - flagged for whoever owns that
crate).

## What is still open

- **No transform writer has been found** that would move
  `321Go_StartFinish`'s instantiated object off the world origin to a given
  circuit's own gantry. **`param_1+0x40` is closed, with a negative result**
  (seventh pass, above): it is the loaded mesh's own authored `Camera` node
  (`camera1`/`cameraShape1`), found by an RTTI-style type match against
  class id `0xf7` - a real object, but not a placement mount point, and a
  markedly less promising lead than the prior pass's "plausible
  spatial-scene-graph entry point" framing hoped for. **What remains
  genuinely open**: who places `321Go_StartFinish`'s own `world` node (or
  the separately-registered `0x09837cb0`/`0x0983a240` objects the live
  capture two passes ago tied to the object read every frame during the
  countdown) at a given circuit's own gantry coordinates. No static lead
  currently points at a candidate for that; the two ruled-out options below
  and the closed `Camera`-node one above are what's been checked and
  retired, not what remains to check. A live capture - a breakpoint on the
  child-list insert (`func_0x00140bd4`, `0x08944bd4`, confirmed by direct
  disassembly this pass to append to the same `+0x10`/`+0xc` list the type
  search walks) during an actual track load, watching what gets parented to
  `321Go_StartFinish`'s own root or to the registry objects - is the
  concrete next step if this is picked up again; nothing static-only is
  left to try that this and the two prior passes haven't already covered.
  Two things checked and ruled out earlier, so as not to repeat them:
  `Billboard_ConstructResource_q`'s own `param_3` is declared and never
  referenced in its body, so the outer caller's discarded `param_1` cannot
  be a parent/locator reference reaching construction; and the alloc/
  `func_0x00140bd4` chain (billboard object -> a per-track container ->
  `_DAT_002ae2b4`) is an ownership/lifetime tree for cleanup, with nothing
  shown to give it a spatial transform of its own - it is not the
  renderer's transform-composition chain.
- **The final texture bind is untraced.** Nobody has caught a
  `Gfx_BindTexture` call for the gantry's screen position and confirmed it
  reads from the instantiated object - see the halting-function trap above
  for why that attempt does not count as a negative result.
- **The material object's live fields, during an actual countdown, are
  unread** - a light-intensity/phase value tied to `start_lights` is the
  best guess, not checked. The four `+0x88..+0xa0` wrapper sub-objects'
  *live* content (as opposed to the stale credits-text sample) is likewise
  unread.
- **`0x0890cf34`** (the animation accumulator) has no function boundary in
  Ghidra at all; neither it nor `FUN_088fe208`/`Node_UpdateTree` were renamed -
  confidence did not clear the bar, and the first needs a `create_function`
  pass before it even has an address range to name.
- **The colour-path pool at `_DAT_002ae2b4+0x3c`** (walked by
  `Billboard_CreateFromColour_q`) is still unread and still a live,
  reached-during-load mechanism worth checking - it may be a second
  instance of the same registry-and-resolve pattern, for colour slots.
- The `func_0x00XXXXXX` / `+0x08804000` relocation gap above is its own
  open item, tracked separately.
- **`Data\Environments\<circuit>\start_grid.vex` is ruled out as a placement
  source, checked this pass - and it is the pre-race flyby** (2026-10-01,
  [race-intro.md](race-intro.md)): `grid_camera1`'s keyed animation is the camera the
  original plays before the countdown. `oag-view --nodes` against `16_Track`'s copy
  (real disc, `oag-tools`/`oag-view`'s own `--nodes` flag, no modification)
  shows exactly 4 nodes: `World`, two `Anim Transform` (a camera rig group)
  and one `gridCamera` leaf - a Maya camera scene, not a billboard-shaped
  locator or any spatial content beyond it. Whatever supplies slot 8's
  per-circuit transform, it is not hiding in this file.
- **A live PPSSPP capture was attempted this pass and did not complete.**
  The plan (no breakpoint needed): read `*(u32*)(0x00058c28 + 8*4)` for
  slot 8's constructed object pointer, then 64 bytes at `ptr+0x50` across a
  countdown, to see whether anything besides construction ever writes a
  non-constant value there. `PPSSPPHeadless` reached Main Menu, set TIME
  TRIAL/VENOM and then died with `memory.read: CPU not started` right at the
  race-load transition (`expect(dbg, IN_GAME, ...)`), twice. The SDL+Xvfb
  path (this project's own recommended default for anything beyond a
  trivial read) never got past re-answering first-boot dialogs, twice - the
  actual cause, found afterward: `timeout 90 uv run --with websocket-client
  python3 <script>` does not kill the `python3` grandchild `uv run` execs
  into, so a timed-out reader survives its own timeout, its parent capture
  script hangs in `wait`, and the still-alive `PPSSPPSDL`/`Xvfb` from the
  previous attempt squats the debugger port and `:98` for the next one -
  `Failed to bind to port 47860` in the new instance's own log was the tell,
  found only after cleaning up by exact PID and re-reading it. Every
  process this pass spawned was identified and killed by PID before
  stopping; nothing was left running. Given the static reading above
  already explains the write this read would have checked, this was not
  retried a fifth time - see the Verdict.

## 2026-10-06 (billboards lane): the adverts are drawn through their own camera into a 128 x 128 texture - there is no placement transform to find

This closes the thread's top open item by retiring its question. **Nothing
places an advert in the world.** The object `Billboard_ConstructResource`
(`0x08900220`) builds is a *card*: the advert model, its own `Camera` node and a
128 x 128 render target. The model is drawn through that camera into the target
each frame, and the track's `billboardN.tga` quads show the target. The "identity
matrix the constructor writes" was never a transform of the advert - it is the
camera's cached view matrix, filled in on the first update. Evidence is a live
GE dump plus the instructions below; the earlier pages' confidence scores stand
for what they measured.

### 1. A live frame renders into an offscreen buffer and then samples it (92)

PPSSPP v1.20.4, Talon's Junction, Time Trial at the start line, own Xvfb and
port. `scripts/psp-ge-dump.py dump` gives 4,718 GE commands for one frame; a
scan of `FBPTR` (`0x9c`) and `FBW` (`0x9d`) shows the frame buffer moving from the
display buffer (`0x04088000`, width `0x200`) to **`0x04154000`, width `0x80`**
twice, back to the display buffer after each, and three main-pass prims
sampling `0x04154000` as their texture.

- **Pass A**: 13 prims (a clear sprite plus the 12 meshes of
  `AURICOM_LANDSCAPE_01.vex`). Viewport scale `+-64`, offset 2048: the whole 128 x 128.
  Clear colour `0x00000000` (the sprite's vertex colour), lighting off, fog on.
- **Pass B**: 8 prims, the 321Go gantry model. The same viewport.
- The two main-pass prims that sample the buffer have vertex-buffer boxes
  `x -254.5..-243.4, y -15.3..-0.3, z -270.6..-233.7` and
  `x 34.24, y -41.2..-30.6, z -208.2..-162.8`. Against `16_Track`'s `track.vex`
  these are **node 453, texture `billboard7.tga`** and **node 74, texture
  `billboard8.tga`** (`crates/render/tests/billboard_slots_ground_truth.rs`,
  within 1.0 on every bound), and the manifest's slot 7 is Auricom's model and slot
  8 the gantry. Slot N's advert is on the quad textured `billboardN`.
  The third sampling prim is a 2-vertex sprite, not a quad.
- The same view in the original's own pixels (`start.png`, scratch) shows a black
  panel with a white "m" glow at that quad; this project's render at the same
  tick (Time Trial, tick 300) draws the same panel and glyph.

### 2. The matrices of those passes are the advert's own camera (90)

| | pass A (Auricom) | pass B (321Go) |
| --- | --- | --- |
| view | pure translation `(-0.48542, -0.47794, -31.832)` | `-(0, 0, 30)`-shaped |
| projection x scale | 1.857 | 1.667 |
| projection y scale | 3.714 | 6.667 |
| `camera1` translation in the `.vex` | `(0.48542, 0.47794, 31.832)` | `z` 30.0 |
| camera payload `+0x22` (u16) | `0x5080` | `0x5812` |
| camera payload `+0x1c` (f32) | 2.0 | 4.0 |

`VexCamera_BuildProjection` (`0x08901dc4`, [camera.md](camera.md)) reads the field
of view off a u16 key/value curve at the camera payload (`+0x00` key count,
`+0x02` flag byte, `+0x04`/`+0x08` offsets of times and values) and scales a value
by `180/65535`; **that is a horizontal angle**: `0x5080` -> 56.60 deg, x scale
`1/tan(28.30 deg)` = 1.857, y scale `aspect` x that = 3.714; `0x5812` -> 61.93
deg -> 1.667 and, at aspect 4.0, 6.667. All four captured numbers fit with no
free parameter. The near plane is the literal `1.2` and the captured depth words
imply a far plane of 2017.7 on this frame (`g_display + 0x1698`; [exhaust.md](exhaust.md)
read 2000.0 on another). **The projection's 2:1 aspect is drawn into a square
target**, so the picture is squashed into the buffer and sampled back out across
a quad - the authors framed the advert for the quad, not for the buffer. The
view matrix is the rigid inverse of `camera1`'s world transform:
`VexCamera_BuildViewMatrix` (`0x089001a4`) transposes the camera's 3 x 3 rows at
`+0x60..+0x88` and negates the translation through them (`vmmov_t`,
`vtfm3_t` with a negating prefix).

Every advert a Pulse manifest names carries exactly one camera with a one-key
perspective curve: 19 distinct models over the 11 circuits that author a
manifest, `(fov, aspect)` in `{(45.00, 0.5), (56.60, 2.0), (68.88, 2.0)}`
(`crates/game/tests/billboard_adverts_ground_truth.rs`). The aspect-0.5 ones are
the portrait adverts.

### 3. The constructor's `.vex` arm builds exactly that (85, decompile)

`Billboard_ConstructResource`, the branch on a trailing `VEX`:

- `FUN_0891fe14(obj + 0x98, 0x80, 0x80)` - the render target, **128 x 128**.
  (The `MIP` arm calls `FUN_0891fcd0` with no size: a flat texture, no camera;
  no manifest on the disc uses it.)
- `Vex_LoadModel(child, name, tag, 0xfdb2, 0x3e9, 8)` loads the advert as a child
  at `obj + 0x3c`, and **every `Mesh` node of it gets `+0x4c` = the slot's tag**
  `(num - 1) * 0x1000000 + 0x1200000` (`obj + 0x9c` and `+ 0xa0` hold the same
  family at `0x110000` and `0x140000`: the three `Gfx_Enqueue` sort keys a
  `VexCamera_Submit` queues - begin, the meshes, end).
- `FUN_089000f8` finds the model's `Camera` child (`Camera_GetTypeId_q`), kept at
  `obj + 0x40`.
- Four vec4 constants are copied into `obj + 0x50..0x8f`: the identity, and
  `obj + 0x88 = -10.0`: a default view matrix with its translation at `z = -10`.
  The earlier live read of "a literal identity" was this matrix before its first
  update; either way it is overwritten from the camera, below.
- `Billboard_UpdateViewMatrix` (`0x089009a8`, the vtable slot after the
  constructor's) fills `obj + 0x50` from `VexCamera_BuildViewMatrix(camera)` when
  the camera is present and the mode word `obj + 0xa8` is in range.
  `VexCamera_Submit` (`0x08900884`) then installs it into the display's view stack,
  calls `VexCamera_BuildProjection(camera, 1)` and enqueues the two keys, and
  `VexCamera_Draw` (`0x08900a9c`) sets the GE state for the begin key (depth
  mask 0, fog, the same view/projection again). [exhaust.md](exhaust.md) named
  those two from the exhaust flare's use of the same camera type; **the billboard
  card is the other user of the one class**.

What was **not** caught: the instruction that makes the track quad's texture the
card's target. The register-by-name step is the registry the 2026-08-28 section
found (three tags per slot), and HD's `Billboard_LoadModelAndBind` does it by
building `"billboard" + num`; the two live samples above agree with that reading
but are two instances, not the code. Scored 85 for the binding, not 90.

### What this closes and what it leaves

- **Closed**: where the transform comes from (there is none), how a slot's
  advert reaches the quad (named by the slot number on the placeholder quad's
  texture), and the framing (the model's own camera). Slot 8's gantry is the same
  mechanism: pass B is the 321Go gantry, aspect 4.0, 61.93 deg.
  `oag_raceplay::gantry` still stands the model on the measured mount rather than
  drawing it to a texture (a placement that was measured against the original and
  accepted); this lane did not touch it.
- **Still open**: the advert's animation clock. The card's `Anim Transform`
  nodes match an authored rest pose at the captured instant, a pose the authored
  10.0 s loop (600 frames, found by scanning the model's node tracks) visits
  several times, so one dump does not pin the clock. Two attempts to fit the
  clock from further dumps (nine in all, 0.3 to 25 s apart in emulated time) were
  inconclusive: the letters sit at rest for most of the loop and the star nodes' poses
  did not match the sampled matrices. The project drives the cards off the scenery
  clock, **chosen, not measured**. And a **colour slot**:
  `Billboard_CreateFromColour` walks a per-track pool of `PI004`-shaped entries
  (type hash at `+0xa0`, colour mask at `+0xa4`, model name at `+0x94`) and swaps
  the match to the end of the list - first match in pool order, no random draw in
  the code read. The pool's *filling order* is not read, so a colour slot is
  still drawn nothing (on Talon's Junction: slot 3, which has no quad).
- **PS2** (`SCES_547.48`): not captured. The same advert files carry the same camera
  nodes and the same placeholder quads, the loader draws them, and a pixel diff of
  an autopilot frame shows the advert on the right wall; the mechanism is carried over
  by analogy, not measured on PCSX2.

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x089001a4` | function | `VexCamera_BuildViewMatrix` | 75 |
| `0x089009a8` | function | `Billboard_UpdateViewMatrix` | 70 |
