# Collision: a tag-located candidate, not cleared to a name

2026-09-15. Function in `eboot.bin` (WipEout: Omega Collection, PS4,
`CUSA05670`, EU), `x86:LE:64:default`, image base `0x01000000`. A fresh area
for this binary - nothing under `docs/ghidra/functions/ps4-omega-eu/` names
collision code before this page.

**No name is applied here.** `search_strings("Collision.cpp")` finds exactly
one match on this binary, the tag
`C:\WOPS4\Wipeout\Code\Backend\General\Collision\Collision.cpp`, and
`get_xrefs_to` finds exactly one writer, `FUN_01393610` (one `LEA`+`MOV`
pair - one write, not "two writers" as a first pass at this page wrongly
counted the two data refs). That tag match alone was initially read together
with three structural observations as corroboration for
`ps3-hdfury-eu/collision.md`'s own `Collision_Construct` (`0x00034aa0`):

- A linked-context-list walk before installing itself.
- The vtable/tag/callback triple at `param_1[0]`/`param_1[0xb]`/`param_1[1]`.
- Several sized allocations immediately following the tag write.

**All three turned out to be shared base-class registration boilerplate on
this binary, not collision-specific evidence** - the identical instruction
sequence (busy-wait chain walk, `0x3006`/`0xffffffff` field writes, the same
vtable/tag/callback triple) already appears in all thirteen of
`ModeManager_ConstructByMode`'s branches ([`weapons.md`](weapons.md)), so it
proves nothing about this specific class beyond what the tag string already
said. Worth recording as a negative result for the next reader: **on this
binary, that idiom is generic tagged-object-construction boilerplate,
not evidence of a match to any specific sibling-binary class** - the tag
string itself is the only real signal in the first three bullets, echoing
`README.md`'s own `Collision/SimpleMesh.cpp` precedent (tag alone, without
the code lining up, wasn't enough there either).

**Checked for HD's own discriminating invariant, and it isn't present
here.** `ps3-hdfury-eu/collision.md` calls out a `0x19c`-iteration loop
striding two buffers by `0xb0` and `0x70` bytes, with `0x19c * 0xb0 =
0x11b40` and `0x19c * 0x70 = 0xb440` both landing exactly on allocation
sizes computed elsewhere - two independent constants agreeing is what
carried that reading to its own 80. Disassembling forward from the tag
write (`0x01393bca`-`0x01393d58`) finds one candidate strided loop, at
`0x01393cc0`-`0x01393cda`: `RAX` runs from `-0x8000` to `0` in `0x40`-byte
steps (`0x200` iterations), clearing a fixed range at `param_1[0x8880..
0x10880)`. That is a `memset` of a fixed in-object array sized at compile
time (`0x200 * 0x40 = 0x8000` exactly, but trivially so - the object's own
literal size, not an externally-computed allocation the way HD's two
buffers were), so it is not the same kind of check: nothing here ties an
*allocated* size to an iteration count the way HD's pair does. No other
loop in the disassembled window has that shape either.

**Left as a tag-located candidate, no `names.tsv` row** - the same outcome
`README.md` records for `Collision/SimpleMesh.cpp` and `vex-classes.md`
records for its own spot-check: the tag points at the right neighbourhood,
but the code doesn't clear the bar a name requires. Not chased further:
whether a matching invariant exists somewhere else in this function past
the disassembled window, and the rest of `ps3-hdfury-eu/collision.md`'s own
roster (`Collision_AllocateBuffers`, `Collision_AddObject`,
`Collision_ProcessPairs`) on this binary.
