# Billboards: `TrackStartup.xml` on PSP, and the transform it writes

Binary: `PSP_GAME/SYSDIR/BOOT.BIN` from `pulse-psp-usa.chd` (`UCUS-98712`), as
`/psp-pulse-usa/BOOT.BIN` in Ghidra, image base `0x08804000`.

This is the PSP side of the same question
[HD/Fury's `billboards.md`](../ps3-hdfury-eu/billboards.md) answers for that
platform, and [`oag_formats::trackstartup`](../../../../crates/formats/src/trackstartup.rs)
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
from consecutive addresses, not sixteen separate computed values). **Read
live, those sixteen floats are the literal 4x4 identity matrix**
(`[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]`), the same for every billboard
regardless of `num`. `param_1+0x88` is separately set to the bit pattern
`0xc1200000` (-10.0f), also a fixed constant.

## Construction writes no transform - true, but the conclusion drawn from it was wrong

`num` (1-8) is used purely as a **resource-slot index** into a 9-entry array
and as the seed for three derived resource-tag IDs at construction time. It
is never used there to select or compute a position, rotation or scale, and
the one transform `Billboard_ConstructResource_q` writes is a hardcoded
identity matrix, identical for every billboard. All of that still stands.

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
  (`FUN_0894402c`) that walks a child list and calls two functions through a
  vtable-shaped pointer at `+0x38` - unremarkable scene-graph plumbing,
  confirming this object sits in the ordinary per-frame update tree rather
  than being dead weight.
- **`0x0983a240`** is read overwhelmingly (6,261 of its hits) by one
  address, `0x0890cf34`, which has **no function boundary in Ghidra** -
  reached only through the same kind of indirect/virtual dispatch as
  `FUN_0894402c`'s vtable calls, which is almost certainly why auto-analysis
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

## Verdict

Construction-time placement is still absent, confirmed by the identity
matrix and the resource-tag derivation. **That is not the same claim as
"nothing overrides the disc's static content", which is false** - a live,
per-frame-updated resource, reached through the very tags construction
derives, sits between the manifest and whatever the original actually
draws, and the two objects behind it are being read and updated throughout
the countdown a stretched digit would otherwise occupy.
[`crates/formats/src/trackstartup.rs`](../../../../crates/formats/src/trackstartup.rs)
carries the corrected implication.

## What is still open

- **The final texture bind is untraced.** Nobody has caught a
  `Gfx_BindTexture` call for the gantry's screen position and confirmed it
  reads from `0x0983a240`'s resolved state - the causal chain above is
  strong but circumstantial.
- **The four state-group sub-objects** (three each, twelve total, off
  `0x0983a240+0x88/+0x90/+0x98/+0xa0`) were found but not decoded - unknown
  whether any names or points at a texture distinguishable from
  `billboard8.tga`.
- **`0x0890cf34`** (the animation accumulator) has no function boundary in
  Ghidra at all; neither it nor `FUN_088fe208`/`FUN_0894402c` were renamed -
  confidence did not clear the bar, and the first needs a `create_function`
  pass before it even has an address range to name.
- **The colour-path pool at `_DAT_002ae2b4+0x3c`** (walked by
  `Billboard_CreateFromColour_q`) is still unread and still a live,
  reached-during-load mechanism worth checking - it may be a second
  instance of the same registry-and-resolve pattern, for colour slots.
- The `func_0x00XXXXXX` / `+0x08804000` relocation gap above is its own
  open item, tracked separately.
