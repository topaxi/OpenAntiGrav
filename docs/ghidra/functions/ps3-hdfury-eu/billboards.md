# Billboards: how `<TrackStartup>` becomes a 9-entry slot array

Binary: `PS3_GAME/USRDIR/EBOOT.BIN` from `hdfury-ps3-eu-dec.iso`, as
`/ps3-hdfury-eu/EBOOT.elf` in Ghidra.

The file's own layout - what `trackstartup.xml` declares, and the survey over
all 16 circuits - is [`oag_formats::trackstartup`](../../../../crates/formats/src/trackstartup.rs),
also summarised in [`docs/formats/README.md`](../../../formats/README.md)'s
"HD track startup" row. This page is the executable's side: what it reads out
of a `<Billboard>`, where it stores each one, and the one place a manifest's
own authoring is silently overridden.

**Nothing here is runtime-verified.** Per [visibility.md](visibility.md)'s
rule, static reading of one binary caps every score on this page at **84**.

## The trap that shapes every address below

The Ghidra database gives all 24,155 functions the entry point's TOC, which is
wrong for any function above `0x0032d5e0` - see [memory.md](memory.md). Every
displacement on this page is resolved with
[`scripts/ps3-toc.py`](../../../../scripts/ps3-toc.py), which walks the OPD
and uses each function's own TOC rather than Ghidra's. `TrackStartup_Load`'s
own decompile is unreadable without this: Ghidra spills its TOC pointer to a
stack slot it calls `iStack_1464`, so every `lwz rX,disp(r2)` in the pseudocode
shows as `*(iStack_1464 + disp)` instead of a named string - the same shape of
defect, one call deeper than the visibility page's.

## Finding the loader

`%s\TrackStartup.xml` (`0x007804e8`) is the format string; `scripts/ps3-toc.py
attrib 0x007804e8` names two functions, `0x000b2af0` and `0x000b19a8` - neither
opened to see which builds the path, since `attrib` on the
`Billboard`/`Num`/`Location`/`Color` element and attribute strings
(`0x00780358`, `0x00780368`, `0x00780370`, `0x00780380`) names only `0x000b19a8`,
which is enough to identify the parser without reading the other.

## The names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x000b19a8` | function | `TrackStartup_Load` | 82 |
| `0x0029adf8` | function | `Billboard_CreateFromLocation_q` | 65 |
| `0x0029af88` | function | `Billboard_CreateFromColour_q` | 62 |
| `0x0029a6a8` | function | `Billboard_ConstructResource_q` | 62 |
| `0x003a4b70` | function | `GetBillboardMeshIdFromName` | 90 |

The last row is a gift, not a reading: the name is the function's own debug
string (`"GetBillboardMeshIdFromName: No model loaded for billboard %i\n"`,
`0x007b0120`), so it carries no `_q` despite nothing else on this page being
runtime-checked.

## `TrackStartup_Load` (`0x000b19a8`, `Track.cpp`)

One function parses the whole file: `<LevelFx>` and its children
(`Weather`/`Tex`/`Alpha`/`DisplayScale`/`TexScale`/`AspectRatio`/`EnvPsys`/
`ScreenPsys`/`DriftY`/`DriftMistMult`/`MistInside`/`WindBase`/`WindRange`),
`<UnderwaterSound Type>`, `<WindSound Type>`, `<LoadSoundBank Filename>` (built
through the same `%s\%s` pattern as the top-level filename), and `<Billboard>`.
None of the `LevelFx` fields are consumed by this project yet; only the
billboard path was chased.

**A `<Billboard>`'s own attribute loop reads exactly `Num`, `Location`,
`Color`, `Colour` and `Glow` - and nothing else.** Confirmed by resolving
every TOC slot the loop compares against, in address order:
`0x0079b0ac`/`disp -0x542c` (`Billboard`, the element name), then within it
`-0x5424` `Num`, `-0x5420` `Location`, `-0x541c` `Color`, `-0x5418` `Colour`,
`-0x5414` `Glow`. `Color` and `Colour` write the same buffer - either spelling
reaches the field - and `Glow` is parsed as a 4-byte float, clamped between two
disc-configured bounds and then scaled by a third, before being carried into
the constructor call as its last argument. **`type` is never compared against
in this loop**, despite every one of the disc's 118 `<Billboard>` elements
authoring it - it is read by something else, or by nothing. **Checked against
every copy on the disc, not just one archive**: `scripts/psarc.py list` over
the whole image finds exactly 16 `trackstartup.xml` entries total, split across
`DATA00.PSARC` and `DATA02.PSARC` with no duplicate elsewhere, and
`scripts/psarc.py extract` pulled all 16. None authors `Colour` (the UK
spelling alone) or `Glow`, so this gap costs nothing on the shipped disc - the
reader in `oag_formats::trackstartup` is right to skip both.

**A sibling element, `<Render>`, is recognised and then does nothing.**
Reached the same way `Billboard` is (`disp -0x5408`), the name comparison
against it (`FUN_006764d8`) runs and its boolean result is discarded outright
- the code unconditionally continues to the next sibling regardless of match.
No manifest on the disc authors a `<Render>` element either, so this is dead
code on real data, the same shape as the draw-order row's unexercised `0x31`
branch.

## The slot array: 9 entries, indexed by the raw `Num`

A `<Billboard>`'s attributes flow into two possible constructors depending on
whether `Location` was present:

- **`Billboard_CreateFromLocation_q`** (`0x0029adf8`) when `Location` was read.
  Takes `(text_id, num, location_ptr, glow)`.
- **`Billboard_CreateFromColour_q`** (`0x0029af88`) when it was not, and
  `Color`/`Colour` was. Takes `(text_id, num, colour_ptr, glow)`.

Both are guarded by the same occupancy test against one singleton's array,
`*(manager + num*4 + 4)`, before constructing anything - a later element
reusing an already-filled `Num` is silently dropped. **The index is `Num`
itself, unadjusted, confirmed by direct control flow, not by frame-size
arithmetic**: at `0x000b2790`, `lwz r4,-0x5424(r2)` loads the `Num` string and
compares it against the current attribute name; the match arm falls straight
through to `0x000b27a8` (`addi r4,r1,0x74` then `bl 0x00677248`), which is the
only store to that stack slot in the whole attribute loop. That same slot is
loaded at `0x000b239c` (`lwz r4,0x74(r1)`), `extsw`'d and passed on as the
argument `Billboard_CreateFromLocation_q`/`_FromColour_q` test for occupancy -
no instruction anywhere on that path subtracts from it. The same value then
reaches `Billboard_ConstructResource_q` one hop later, as *its* fourth
argument, which is the `num` this section's array indexing describes. Two
independent facts confirm the array is sized for indices `0..=8`, nine slots,
with `0` always wasted:

1. `Billboard_ConstructResource_q` (below) writes the finished object into
   `manager + (num << 2) + 4` on construction, `num` being the argument that
   traces back to the `Num` attribute as above.
2. The manager's own destructor zeroes exactly nine consecutive words at
   `manager+0x4` through `manager+0x24` before anything else.

That matches the manifest's own comment, present in all 16 files: `<!-- You
can edit and add up to 8 of these billboard definitions, each with a unique
"num" (1-8) -->`. **This settles the open question from the trackstartup
format page in favour of a fixed array, not a name lookup**: nothing here
looks up a node by a name the engine builds, so the two stray
`Billboard<digits>` nodes on `02_track` and `05_ubermall` are artist debris,
unrelated to this mechanism.

## `Billboard_ConstructResource_q` (`0x0029a6a8`): it instantiates

Both constructors above call this one with the resolved path/colour text.
**It settles "instantiated vs. supplies textures to existing geometry" in
favour of instantiation**: it inspects the text's own last three characters,
case-insensitively, and branches - `.vex` allocates a 0x2080-byte object
through a generic resource loader (`FUN_002c1ec8`, called with a magic
`0xfdb2` and a type tag `0x3e9`), `.mip` allocates a 0x100-byte object through
a different one (`FUN_002de5c8`). **`0x3e9` (1001) is not one of the 30 vex
node classes `oag_formats::vex` names** - checked directly against the
`CLASS_*` table, which runs `0x3bb`-`0x3e5` with no member at `0x3e9` - so this
is a tag in some other, unidentified resource-type enum, not a vex class; do
not assume otherwise. **The `.mip` arm is implemented and unexercised on this
disc**: no manifest's `location=` ends in anything but `.vex`, so every real
billboard takes the first branch. Same shape as the draw-order row's `0x31`
branch and the `LodGroup` two-tier-always-drawn finding - authored capacity
the disc's own content never reaches.

## The surprising part: `Num == 7` is not what its own manifest says it is

`Billboard_CreateFromLocation_q` special-cases its `num` argument:

```
if (num == 7) {
    if (mode_descriptor->field_0x90 != 0 && mode_descriptor->field_0x4c != 0) {
        location = *(mode_descriptor->field_0x4c) + 0xf0;
    }
}
```

`mode_descriptor` is a single global (`PTR_DAT_008b2dac`), read from nowhere
else this session looked - its shape is otherwise unidentified. **`num == 7`
is slot 7, not slot 8**: `trackstartup.md`'s own survey found slot 7 authored
as `fx350.vex` on all 16 circuits, exclusively, while slot 8 is
`321Go_StartFinish.vex`, also exclusively. If this branch fires, the engine
throws away the manifest's own `fx350.vex` and substitutes whatever
`mode_descriptor` names instead - **unconditionally, regardless of what the
disc authored for that slot**. Four strings sit in the same TOC window as the
`Billboard` attribute names: `321Go_StartFinish.vex`, `321Go_Zone.vex`,
`321Go_HD_Zone_Battle.vex`, `321go_hd_detonator.vex` - one per known
opponent-free mode this project has already found (single race / time trial,
Zone, Zone Battle, Detonator). **That match is a lead, not a finding**: the
computed pointer (`*(field_0x4c) + 0xf0`) was never traced to one of these
four addresses, only observed to sit near them in the same data region.
Slot 8 is not special-cased anywhere in this function; its `321Go_StartFinish`
content is used exactly as authored.

## `GetBillboardMeshIdFromName` has no found caller - but its write recurs in a function that does

`GetBillboardMeshIdFromName`'s only xref Ghidra reports is a **data**
reference from `0x0088b6d0`, not a call. That address is inside the module's
`.opd` section (Ghidra's own naming gives it away: neighbouring entries import
as `.opd.FUN_003a4650` etc.) - checked by reading 64 bytes on both sides of
it, and the same `{entry, 0x008bd3c4}` eight-byte pattern continues in both
directions well past any plausible billboard-sized table, so this is the
ordinary PPC64 function-descriptor section, not a billboard-specific dispatch
table.

**The load-bearing fact is that Ghidra shows no direct `bl` caller either,
and that check is reliable here.** A `bl` target is a relative displacement
encoded directly in the instruction - no TOC involved - so this page's TOC
trap (which breaks `lwz`-based data resolution, not call encoding) does not
apply to it, and the xref tool is demonstrably not silent when a direct call
exists: the identical query against `0x003a4da0` returns
`From 006791e4 in FUN_006791d8 [UNCONDITIONAL_CALL]` (below). For
`0x003a4b70` it returns nothing.

That leaves the indirect route: is this function's address ever stored
somewhere to be called through, the way a callback or vtable slot would?
`scripts/ps3-toc.py`'s `word_addresses` finds every aligned word in the whole
image, code or data, equal to a given value, independent of Ghidra's TOC.
Run against `0x0088b6f0` (the function's own OPD descriptor address - the
value a stored "function pointer" actually holds on this ABI, not the bare
code address) it found **zero** words. On its own that is unremarkable: the
same query against 20 neighbouring `.opd` entries found only **one** stored
anywhere - `0x0088b6d8` (`0x003a46e8`, the distance-comparator read above,
confirmed as a real callback: its descriptor sits at data slot `0x008b6f98`
and is loaded by `0x003a5f68` and `0x003a6950`) - so most functions in this
binary are never referenced as data at all, direct-`bl` being the norm, and
a zero here mainly rules out the callback route rather than adding a second
independent vote for deadness. **Together**: no direct call, and no stored
pointer to call through - no invocation path is known, which is different
from, and weaker than, "confirmed unreachable." Not checked: a function
pointer built with an absolute `lis`/`addis`+`ori` pair instead of a plain
stored word.

Separately, `0x003a4da0` (unnamed, confidence held below 50 for the reason
below - do not rename) **is** reached by a real, traced call:
`Billboard_ConstructResource_q` calls it at `0x0029adb4` as
`FUN_006791d8((int)param_4, param_2, param_1, uVar23)`, i.e. `(num,
location_ptr, the billboard object, the just-allocated .vex mesh resource)`.
`FUN_006791d8` is a pure TOC-fixup trampoline (`std r2,0x28(r1)`; recompute
r2; `b 0x003a4da0`, no register touched), so those four arguments pass
through unchanged; Ghidra's decompile shows a fifth parameter on
`0x003a4da0` with no value supplied at this call site, so it is either read
from a stale register or genuinely unused on this path - not resolved either
way.

`0x003a4da0` is 4.5 KB, mixing resource-database path building (`%s` around
`TimeTrial_HUD.xml`-shaped strings) with logic this session did not work
through. But its tail loop **inlines `GetBillboardMeshIdFromName`'s exact
write**, against a lookup of the same stride and offset:
`*(int*)(PTR_s_WIP3OUT_008a704c + index*0x100 + 0x10)`, where
`GetBillboardMeshIdFromName` uses `param_3` unadjusted and `0x003a4da0` uses
`num-1`. **This is now settled, not an open discrepancy - conditional on
`0x003a4da0` itself indexing correctly**, which the rest of this page cannot
guarantee: it already documents dead code and unexercised arms elsewhere in
this same subsystem.
`Billboard_CreateFromLocation_q`'s own decompile confirms the raw, 1-based
`Num` (the same value that indexes the manager's occupancy array) is what
reaches `0x003a4da0`'s `param_1` unchanged, which then 0-based-adjusts it
before indexing `PTR_s_WIP3OUT_008a704c`. **`PTR_s_WIP3OUT_008a704c` does not
waste index 0 the way the manager array does**: `0x003a4da0`'s own
`iVar38 = param_1 - 1` is unguarded down to 0, and `Num == 1` (`iVar38 == 0`)
takes the identical write branch as any other slot whenever `HUD_Style` is
empty (`*PTR_s_HUD_Style_008a7048 == '\0'`, the gate is on that flag or on
`iVar38 == 7`, never on the index itself) - so index 0 is a real, used slot
from this function's own control flow, not a wasted one. Taking that at face
value leaves one branch of the disjunction standing: **if** a caller for
`GetBillboardMeshIdFromName` exists (see above - increasingly not settled
that one does), it must already be passing a 0-based slot index, not the
1-based `Num` every other function on this page takes. On a hash match both
functions write the same four 16-byte vectors at
`+0x00`/`+0x10`/`+0x20`/`+0x30` plus a resource pointer at `+0x70`. That
confirms the write is a repeated **bind idiom** - reset four vectors, stamp a
resource - not a one-off, and not evidence of placement either way: this call
chain passes no position, only `num`, a location string and two pointers.
`0x003a4da0`'s low confidence is about its *surrounding* resource-database
logic, not about this shared write, which is confirmed by direct control-flow
tracing the same way the rest of this page is.

**Why `0x003a4da0` stays unnamed**: it is reached correctly from a billboard
call site, but path-building code referencing HUD XML strings suggests it may
be a generic per-resource loader shared across subsystems rather than
billboard-specific - below the 70 needed to commit to either reading.

## What is still open

- **The transform.** Nothing in `TrackStartup_Load`, its two constructors, or
  either traced write into the per-slot table reads a position, rotation or
  scale - confirming the format page's own "nothing is placed" stance twice
  over now, not once.
- **What calls `GetBillboardMeshIdFromName`, if anything does.** No direct
  `bl` caller (reliable, unlike Ghidra's TOC) and no stored function pointer
  to call through (validated against a positive control) - no known
  invocation path, not confirmed dead. Not checked: an absolute
  `lis`/`addis`+`ori`-built pointer. If a caller exists it must pass a
  0-based slot index, not the 1-based `Num` every other function on this page
  takes - true if `0x003a4da0` indexes correctly, per above.
- **`0x003a4da0`'s own purpose** - generic resource loader or billboard-only -
  its fifth parameter, and the four vectors' exact values (still not
  confirmed as identity; the operand permutation was not worked through, on
  either copy of the write).
- **`mode_descriptor`'s shape**, and whether the four `321Go_*.vex` names
  really are what `field_0x4c + 0xf0` resolves to.
- **`type`'s consumer**, if it has one - unread by this function despite being
  authored on every slot.
