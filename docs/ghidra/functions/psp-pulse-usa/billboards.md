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

## Verdict: no placement logic in this chain, and that is now runtime-confirmed rather than inferred

`num` (1-8) is used purely as a **resource-slot index** into a 9-entry array
and as the seed for three derived resource-tag IDs. It is never used to
select or compute a position, rotation or scale. The one transform written
at construction time is a **hardcoded identity matrix from static data**,
identical for every billboard. `World_LoadTrack -> Xml_OpenFile ->
TrackStartup_Parse -> Billboard_Create*_q -> Billboard_ConstructResource_q`
is a complete, live-verified path from "read the manifest" to "instantiate a
billboard object" that never once reads or computes a world-space
transform for it. This settles, for PSP, the same question HD's page left
open from static reading alone - and PSP settles it the same way: **nothing
is placed here either.** [`crates/formats/src/trackstartup.rs`](../../../../crates/formats/src/trackstartup.rs)
carries the implication for what the renderer may (not) do with a parsed
manifest.

## What is still open

- **The colour-path pool at `_DAT_002ae2b4+0x3c`** is the single most
  promising lead either title now has for where a real placement mechanism
  could live - more promising than HD's `GetBillboardMeshIdFromName`, since
  this one is confirmed *reached* during a real circuit load, not merely
  referenced. Its entries' own layout (in particular whatever sits at their
  `+0x94` and neighbouring offsets) has not been read.
- Whether the identity-matrix write is later overwritten by something else
  (a draw-time placement step outside this construction path) is unread -
  this page only traces construction, not every later use of the object.
- The `func_0x00XXXXXX` / `+0x08804000` relocation gap above is its own
  open item, tracked separately.
