# Billboards: how `<TrackStartup>` becomes a 9-entry slot array

Binary: `PS3_GAME/USRDIR/EBOOT.BIN` from `hdfury-ps3-eu-dec.iso`, as
`/ps3-hdfury-eu/EBOOT.elf` in Ghidra.

The file's own layout - what `trackstartup.xml` declares, and the survey over
all 16 circuits - is [`oag_tables::trackstartup`](../../../../crates/tables/src/trackstartup.rs),
also summarised in [`docs/formats/README.md`](../../../formats/README.md)'s
"HD track startup" row. This page is the executable's side: what it reads out
of a `<Billboard>`, where it stores each one, and the one place a manifest's
own authoring is silently overridden.

**Nothing here is runtime-verified.** Per [visibility.md](visibility.md)'s
rule, static reading of one binary caps every score on this page at **84** -
except `Billboard_ConstructResource`, whose 85 rests on a second,
independently-compiled binary corroborating it (below), the same exception
that rule already carves out elsewhere in this project.

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
| `0x0029a6a8` | function | `Billboard_ConstructResource` | 85 |
| `0x003a4b70` | function | `GetBillboardMeshIdFromName` | 90 |
| `0x003a4da0` | function | `Billboard_LoadModelAndBind` | 80 |
| `0x008b6f38` | data | `g_BillboardSlots` | 84 |

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
reader in `oag_tables::trackstartup` is right to skip both.

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
reaches `Billboard_ConstructResource` one hop later, as *its* fourth
argument, which is the `num` this section's array indexing describes. Two
independent facts confirm the array is sized for indices `0..=8`, nine slots,
with `0` always wasted:

1. `Billboard_ConstructResource` (below) writes the finished object into
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

## `Billboard_ConstructResource` (`0x0029a6a8`): it instantiates

Both constructors above call this one with the resolved path/colour text.
**It settles "instantiated vs. supplies textures to existing geometry" in
favour of instantiation**: it inspects the text's own last three characters,
case-insensitively, and branches - `.vex` allocates a 0x2080-byte object
through a generic resource loader (`FUN_002c1ec8`, called with a magic
`0xfdb2` and a type tag `0x3e9`), `.mip` allocates a 0x100-byte object through
a different one (`FUN_002de5c8`). **`0x3e9` (1001) is not one of the 30 vex
node classes `oag_vex::vex` names** - checked directly against the
`CLASS_*` table, which runs `0x3bb`-`0x3e5` with no member at `0x3e9` - so this
is a tag in some other, unidentified resource-type enum, not a vex class; do
not assume otherwise. **The `.mip` arm is implemented and unexercised on this
disc**: no manifest's `location=` ends in anything but `.vex`, so every real
billboard takes the first branch. Same shape as the draw-order row's `0x31`
branch and the `LodGroup` two-tier-always-drawn finding - authored capacity
the disc's own content never reaches.

**Corroborated on `ps4-omega-eu`, confidence raised 62 -> 85.** That binary's
own `Billboard.cpp` tag (verbatim, under `System\Render\` rather than a flat
layout) resolves to a function that opens with the identical
last-N-characters-uppercased extension check (`.mip` first, `.vex` in the
`else`) and calls its own resource loader with the **same two magic
constants**, `0xfdb2` and `0x3e9` - values with no reason to match by chance
across two independently compiled binaries five console generations apart.
See [`ps4-omega-eu/billboards.md`](../ps4-omega-eu/billboards.md) for that
binary's own half of this finding.

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

## The four `321Go_*.vex` shapes are confirmed as genuinely different content, by rendering them

2026-09-02, from the race-start countdown handover thread, asset side only - does
not touch `mode_descriptor` or close the code-side lead above. Two things this
session's static reading could not settle without actually looking at the geometry:

1. **Are the four names really four different objects, or aliases/debris?** Settled
   by listing `data/images/hdfury-ps3-eu-dec.iso`'s own PSARC archives rather than
   reading strings out of the executable: `321go_startfinish.vex` (12,544 B),
   `321go_zone.vex` (8,368 B), `321go_hd_zone_battle.vex` (10,304 B),
   `321go_hd_detonator.vex` (11,568 B) are four distinct files with four distinct
   sizes under `/data/billboards/hd_adverts/321go/` in `DATA00.PSARC`/`DATA02.PSARC` -
   not one file aliased four ways.
2. **What do they actually look like?** `just view` renders a `.vex` straight off a
   PSARC entry the same way it does off a WAD - `oag-view
   "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC" --mesh
   "/data/billboards/hd_adverts/321go/321go_zone.vex" --screenshot out.png` - and the
   two renders answer the user's own memory directly. `321go_startfinish.vex` is a
   large multi-part structure - **19 mesh nodes, 1,530 triangles**, a checkered-flag
   banner reading `"FX-350 Official A-G Racing League"` in one of its panels, sized
   for a full gantry arch. `321go_zone.vex` is comparatively tiny - **3 mesh nodes,
   222 triangles** - a flat rectangular panel carrying one big texture
   (`321_go_zone.gtf`, **2048x1024**, dwarfing every other texture in the folder;
   the countdown digit texture `321_go_64.gtf` is 64x128 by contrast) that renders as
   a small stylised track-loop icon in red/green/blue, not digits or light bulbs.
   **This is a byte-for-byte match to the user's own description**: "it draws a
   small rectangular track, instead of the 3 2 1 GO" in a Zone race. The node names
   corroborate it structurally too - `321go_zone.vex` has exactly one content node
   beyond its two background panels, literally named
   `pasted__Go_HD_start_light_321go`, while `321go_hd_zone_battle.vex` and
   `321go_hd_detonator.vex` (10,304 B / 11,568 B, DLC3 assets per their own embedded
   Maya paths) each carry a handful of `polySurface`/`planarTrimmedSurface` nodes of
   their own - simpler than `StartFinish`'s nineteen, more than `Zone`'s three - so
   the four shapes read as four *actually different* pieces of authored content,
   not a placeholder and three unused siblings.

**This does not identify which mesh a given race instantiates or resolve
`mode_descriptor`'s shape** - the code-side lead a few paragraphs above is exactly
as open as it was. What it does confirm is that the lead is worth finishing: if the
`field_0x4c + 0xf0` pointer really does resolve to one of these four addresses per
mode, the payoff is not a cosmetic reskin, it is precisely the different display the
user remembers. Full account, cross-posted, in
[docs/ui/hud.md](../../../ui/hud.md#deferred-and-known) and the handover thread this
session opened.

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
from, and weaker than, "confirmed unreachable." **Re-run 2026-09-15 after the
lvlx reimport, same result - see the dated section at the bottom of this
page.** Not checked: a function
pointer built with an absolute `lis`/`addis`+`ori` pair instead of a plain
stored word.

Separately, `0x003a4da0` (unnamed, confidence held below 50 for the reason
below - do not rename) **is** reached by a real, traced call:
`Billboard_ConstructResource` calls it at `0x0029adb4` as
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
`*(int*)(g_BillboardSlots + index*0x100 + 0x10)`, where
`GetBillboardMeshIdFromName` uses `param_3` unadjusted and `0x003a4da0` uses
`num-1`. **This is now settled, not an open discrepancy - conditional on
`0x003a4da0` itself indexing correctly**, which the rest of this page cannot
guarantee: it already documents dead code and unexercised arms elsewhere in
this same subsystem.
`Billboard_CreateFromLocation_q`'s own decompile confirms the raw, 1-based
`Num` (the same value that indexes the manager's occupancy array) is what
reaches `0x003a4da0`'s `param_1` unchanged, which then 0-based-adjusts it
before indexing `g_BillboardSlots`. **`g_BillboardSlots` does not
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

**Why `0x003a4da0` stayed unnamed, and why it no longer does** (updated
2026-09-06): the worry was that path-building code referencing HUD XML strings
made it a generic per-resource loader shared across subsystems rather than
billboard-specific. Resolving its TOC displacements settles that against the
generic reading - it loads into a resource group literally named `billboards`,
builds the lookup key `"billboard" + num`, and owns both
`GetBillboardMeshIdFromName` printf strings. Named `Billboard_LoadModelAndBind`
at confidence 80; see the 2026-09-06 section below.

## What is still open

- **The transform.** Nothing in `TrackStartup_Load`, its two constructors, or
  either traced write into the per-slot table reads a position, rotation or
  scale - confirming the format page's own "nothing is placed" stance twice
  over now, not once.
- **What calls `GetBillboardMeshIdFromName`, if anything does.** No direct
  `bl` caller (reliable, unlike Ghidra's TOC) and no stored function pointer
  to call through (validated against a positive control) - no known
  invocation path, not confirmed dead. **Re-run 2026-09-15 after the lvlx
  reimport, same result - see the dated section at the bottom of this page.**
  Not checked: an absolute
  `lis`/`addis`+`ori`-built pointer. If a caller exists it must pass a
  0-based slot index, not the 1-based `Num` every other function on this page
  takes - true if `0x003a4da0` indexes correctly, per above.
- **`0x003a4da0`'s own purpose** - generic resource loader or billboard-only -
  its fifth parameter, and the four vectors' exact values (still not
  confirmed as identity; the operand permutation was not worked through, on
  either copy of the write).
- **`mode_descriptor`'s shape**, and whether the four `321Go_*.vex` names
  really are what `field_0x4c + 0xf0` resolves to. **Higher-value than before**:
  the four meshes are now confirmed genuinely different content by rendering them
  (above), including a byte-for-byte match to a player's own memory of what Zone's
  gantry looks like, so resolving this pointer is no longer just tidiness - it is
  what would let the difference actually be implemented rather than only observed.
  A static attempt this session (following `PTR_DAT_008b2dac`'s literal stored
  value) did not converge - the candidate address aliased hundreds of unrelated
  functions, which does not fit a single struct's base address and was not chased
  further. **Re-run 2026-09-15 after the lvlx reimport, same non-convergent
  result - see the dated section at the bottom of this page.**
  Live RPCS3 watchpoints, the way this thread's own history solved
  comparably stuck leads, are the likely next step.
- **`type`'s consumer**, if it has one - unread by this function despite being
  authored on every slot.

## 2026-09-06: the transform question is answered, and the answer is "the track supplies it"

This page's "What is still open" list opens with **The transform** - "nothing in
`TrackStartup_Load`, its two constructors, or either traced write into the
per-slot table reads a position, rotation or scale." That observation is
correct and this section does not overturn it. What it adds is *why*: there is
no position to read on this path because the billboard system never places
anything. It binds to geometry the **track model** already authors.

Three things landed together, in order of how much they change.

### 1. The four vectors really are an identity matrix - the operand permutation, worked through

This page has carried "still not confirmed as identity; the operand permutation
was not worked through, on either copy of the write" since it was written. Now
confirmed, from `disassemble_function` on `GetBillboardMeshIdFromName`
(`0x003a4b70`) and **not** from a decompile summary - the PowerPC vector-store
analogue of the VFPU quadword hazard [workflow.md](../../workflow.md) records:

```
003a4c2c: vspltisw v1,0x1         ; v1  = (1,1,1,1), words
003a4c30: vxor     v0,v0,v0       ; v0  = 0
003a4c40: vsldoi   v13,v1,v0,0xc  ; v13 = (1,0,0,0)
003a4c44: vsldoi   v1,v0,v1,0x4   ; v1  = (0,0,0,1)
003a4c50: vsldoi   v12,v0,v13,0x8 ; v12 = (0,0,1,0)
003a4c58: vsldoi   v0,v0,v13,0xc  ; v0  = (0,1,0,0)
          vcfsx    <each>,0       ; int -> float, scale 0
003a4c60: stw      r4,0x70(r11)   ; +0x70 <- the resource
003a4c74: stvx     v13,0,r11      ; +0x00 <- (1,0,0,0)
003a4c84: stvx     v0,r11,r9      ; +0x10 <- (0,1,0,0)   (r9 = 0x10)
003a4c78: stvx     v12,r11,r0     ; +0x20 <- (0,0,1,0)   (r0 = 0x20)
003a4c6c: stvx     v1,r11,r0      ; +0x30 <- (0,0,0,1)   (r0 = 0x30)
```

A literal 4x4 identity, row by row, written before the resource pointer at
`+0x70`. The same idiom occurs twice more inside `0x003a4da0`. **Confidence 88.**

### 2. `Billboard_LoadModelAndBind` (`0x003a4da0`) is the loader, and it binds by material name

Decompiled in full and every TOC displacement resolved through
`scripts/ps3-toc.py resolve 0x003a4da0 <disp>`:

| disp | address | contents |
| --- | --- | --- |
| `-0x648c` | `0x008b6f38` | -> `0x00c48180`, the per-slot table base |
| `-0x6470` | `0x008b6f54` | `'.vex'` |
| `-0x646c` | `0x008b6f58` | `'.rcsmodel'` |
| `-0x6464` | `0x008b6f60` | `'billboards'` |
| `-0x6450` | `0x008b6f74` | `'uvOffset'` |
| `-0x644c` | `0x008b6f78` | `'uvScale'` |
| `-0x6444` | `0x008b6f80` | `'billboard'` |
| `-0x647c` | `0x008b6f48` | `'GetBillboardMeshIdFromName: No model loaded for billboard %i\n'` |
| `-0x6478` | `0x008b6f4c` | `"GetBillboardMeshIdFromName: Couldn't find name %x\n"` |

In order, the function:

1. Takes the manifest's `location=`, **swaps `.vex` for `.rcsmodel`** and loads
   that into a resource group named `billboards`. The `.vex` is the scene
   description; the `.rcsmodel` beside it on the disc is the geometry.
2. Allocates one `0x90`-byte instance block per sub-mesh; initialises each to
   identity at `+0x00..+0x3f`, `+0x70 = 0`, `+0x50 = (0,0,0,0)`,
   `+0x60 = (1,1,1,1)`.
3. **Binds `+0x50`/`+0x60` as the shader constants `uvOffset`/`uvScale`**,
   matched by `Crc32_HashString`. A per-instance UV offset and scale, authored
   as named shader parameters - the same knob Pulse's gantry drives through its
   material's own offset track (see
   [`docs/rendering/start-gantry.md`](../../../rendering/start-gantry.md)).
4. Walks **the billboard's own `.vex`** scene graph (`param_4`, children at
   `+0x14`, siblings at `+0x10`) and writes `instance+0x70 = <the node>`,
   linking each `.rcsmodel` sub-mesh to its node *inside that model*. This is
   the model's internal layout, **not** per-circuit world placement - a
   distinction worth stating because the two are easy to fuse.
5. Walks the track's material set for materials flagged `0x8000`, builds the
   string **`"billboard" + num`** from the manifest's own `Num`, and rebinds
   the matching material's texture. **This is the only place the whole
   billboard path touches the track.**

**Confidence 84** - this page's static ceiling. The string resolutions
themselves are 95: file bytes through a validated resolver.

**A correction to this page's own "fixed array, not a name lookup" claim.** That
sentence says "nothing here looks up a node by a name the engine builds." The
*node* half stands - `amphiseum`'s `track.vex` has 838 nodes and not one named
`billboard` or `advert`, so the two stray `Billboard<digits>` nodes really are
debris. But the engine **does** build a name and look something up with it; the
lookup is keyed on a **material/texture** name, not a node name.

### 3. The disc confirms it independently

`billboard<num>` is not an inference about naming. The placeholder textures ship
in every circuit's own texture set:

```
billboard8.gtf  17 occurrences    billboard4.gtf  6
billboard7.gtf  17                billboard1.gtf  5
billboard5.gtf   7                billboard6.gtf  3
billboard3.gtf   7
billboard2.gtf   7
```

**`billboard7.gtf` and `billboard8.gtf` are present in all 17 environments** -
exactly the two slots the trackstartup survey found authored identically on
every circuit (`fx350.vex` and `321Go_StartFinish.vex`). In `amphiseum`'s
compiled `track.rcsmodel` they sit immediately after the material that binds
them:

```
444067  .../materials/billboarddiffuse.rcsmaterial
444134  .../textures/dds/billboard8.gtf
444190  .../textures/dds/billboard7.gtf
```

So the artist modelled the billboard's mounting surface into the track, textured
it `billboard<num>`, and the engine swaps the texture at load. **The placement
is authored per circuit, in world space, in the track model.** Confidence 90 -
straight off the disc, no decompilation.

### The slot-8 asymmetry, and what is still not established

`0x003a4da0`'s tail carries a special case for slot 8 alone:

```c
if (num - 1 == 7) { *(table_base + 0x834) = <that material's texture bind slot>; }
```

Slot 8 is `321Go_StartFinish.vex` on every circuit of both HD and Pulse. Caching
its texture binding slot in a global is the shape of "so something else can swap
this texture every frame," which is what a countdown needs. **No reader of
`table_base + 0x834` was found.** `search_instructions lwz 0x834(` returns 14
matches program-wide; the three in `FUN_000654e0` were decompiled and are a
different base entirely (a `RaceManager` sub-object at `+0x6ff0`, read as a
four-word cursor at `+0x830/834/838/83c`). The rest are unexamined. **Recorded
as not established**, at no confidence. **Re-run 2026-09-15 after the lvlx
reimport, same result (still 14, no new reader) - see the dated section at
the bottom of this page.**

Two more things deliberately left as hypotheses rather than findings:

- **Render-to-texture.** `FUN_005e5858(<table base>, &piVar29[0x3d])` fills the
  texture that step 5 binds, and `FUN_005ea2d0(<table base>, <64 B>, <16 B>,
  <64 B>, <16 B>, 0xffffffff)` runs alongside it - two 4x4 matrices and two
  vec4s next to a texture handoff reads as an offscreen render, which would also
  explain why a 19-node model can be bound to a flat track surface. **Neither
  callee was opened. Confidence 55.**
- Whether the arch geometry itself is drawn in the world in addition to the
  bound texture. Nothing here settles it.

### A live capture through a countdown, 2026-09-13: the digits do change at runtime, but the write is still not located

`docs/rendering/start-gantry.md`'s own confidence-40 section already argued
from a single reference screenshot that something writes the digit board's
`uvOffset` at runtime, since the static value alone cannot light any glyph.
This pass took a live RPCS3 capture through an actual countdown
(`scripts/rpcs3-drive.py capture --shots 5 --interval 1 --keep-dumps`,
Talon's Junction, ~1 s apart from the grid) rather than one screenshot: the
board's own geometry shows nothing, nothing, a faint sliver, a clear `3`,
then `3` and `2` together across the five frames - a progression, which a
static rest cell cannot produce. This raises confidence that *some* write
happens; it does not locate it.

**A raw byte diff between consecutive captures' pushbuffer dumps does not
isolate it**, and the reason is informative for whoever tries next: the
small region (the RSX FIFO ring itself, ~16 KB) differs by only a handful of
bytes frame to frame, but the two larger regions (~128 KB each) carry every
other draw's own per-frame data - camera, ship, scenery animation - and
differ in hundreds to thousands of places, with nothing in the raw bytes
alone to attribute a given changed word to the gantry material rather than
an unrelated draw nearby in the ring. A step-pattern scan (value stable
across the frames before the digit appears, then a clean jump) found four
candidates; all four reverted on the very next frame instead of continuing
to step, which fits ring-buffer command placement drifting frame to frame
more than it fits one stable, patched value.

**What this does and does not change about the open items above.** `uvOffset`/
`uvScale` are bound as named fragment-program constants patched **into the
shader microcode itself** (the `fslot` patch chain `scripts/ps3-microcode.py`
documents), not a separate constant register - so the write this section is
looking for, if it exists, changes the uploaded fragment-program bytes, not
a constant-buffer upload, which is one more reason a plain float-value diff
across the whole pushbuffer does not find it. The eleven unexamined
`lwz 0x834(` sites two paragraphs up remain the concrete next step, together
with a semantic, packet-aware walk of the pushbuffer against the fragment
program's own patch-slot table rather than another raw diff - this pass's
own negative result on the raw-diff route is itself evidence that route
needs the packet structure, not more captures of the same shape.

### The name of the table base was wrong on this page

Ghidra labelled the per-slot table base `PTR_s_WIP3OUT_008a704c` and this page
repeated it. **The real address is `0x008b6f38`** (holding `0x00c48180`);
`0x008a704c` is a wrong-TOC alias, `0xFEEC` low. Verified three ways: the
instruction is `lwz r30,-0x648c(r2)`; `ps3-toc.py resolve 0x003a4380 -0x648c`
gives `0x008b6f38`; and `get_xrefs_to 0x008b6f38` returns nothing while
`get_xrefs_to 0x008a704c` returns the read. Renamed to `g_BillboardSlots` in the
text above. Every *conclusion* this page draws about the table - stride `0x100`,
indexed by `Num`, nine words zeroed by the destructor - is unaffected. The
underlying tooling defect is written up in [workflow.md](../../workflow.md).

### 2026-09-15: the four caller-less negatives re-run after the lvlx reimport, all unchanged

`docs/reverse-engineering/toolchain.md#ps3`'s lvlx reimport (09:29 the same day) fixed
decode for ~213 KB of code across 294 HD functions that previously had no instructions
and no xrefs. Every negative on this page computed before that point is stale by
construction - a caller living inside one of those 294 functions could not have shown
up in any search run against the old database. This pass re-ran all four static
negatives this page and the sibling handover thread carried, on the reimported image,
using `program="/ps3-hdfury-eu/EBOOT.elf"` throughout. Positive controls first, so a
zero result below is a real zero, not a broken query.

**Positive controls.** `get_function_callers(0x0029a6a8)` (`Billboard_ConstructResource`,
already known to be called from both constructors) returns exactly its two known
callers. `search_instructions(mnemonic="bl", operand_pattern="29a6a8")` returns exactly
its two known call sites (`0029af04` in `Billboard_CreateFromLocation_q`, `0029b0fc` in
`Billboard_CreateFromColour_q`). `search_byte_patterns` on `Billboard_ConstructResource`'s
own code address (`0029a6a8`) finds exactly one hit, its own `.opd` descriptor at
`00881898` - and a second pattern search on that descriptor address as a literal finds
**zero** matches even though this function definitely has two direct `bl` callers. That
last result recalibrates this page's own "zero stored words" reasoning from the
`GetBillboardMeshIdFromName` section above: a function reached only by direct `bl`, the
normal case, legitimately has zero literal occurrences of its own descriptor address
anywhere else in the image. The `.opd`-literal route was never going to distinguish
"no caller" from "called only by `bl`" - it only catches the callback/vtable case, exactly
as the original write-up already said, now with a control proving it rather than arguing it.

**1 and 2. `GetBillboardMeshIdFromName` (`0x003a4b70`) - re-confirmed negative.**
`get_function_callers` returns none. `search_instructions(mnemonic="bl",
operand_pattern="3a4b70")` scans all 1,829,837 instructions in the reimported image and
returns zero - the same program-wide scan that found the two controls above, so a
previously-holed caller now decoding for the first time would have shown up here and
did not. `search_byte_patterns` on the function's own `.opd` descriptor (`0088b6f0`,
unchanged from the prior pass) finds one hit, the descriptor itself; a second pattern
search on `0088b6f0` as a literal elsewhere in the image finds zero, same shape as the
control. `get_xrefs_to(0x003a4b70)` still returns exactly the one data xref from
`0088b6f0` the original pass found, nothing more. The function's own body carries no
`lvlx` (`search_instructions(mnemonic="lvlx", operand_pattern="v",
function="GetBillboardMeshIdFromName")` returns zero of 77 instructions scanned), so it
was not itself one of the 294 previously-holed functions - the reimport could only have
surfaced a caller, not changed this function's own reading, and it surfaced none. Three
routes, three zeros, one calibrated control: **still no known invocation path**, now
dated after the reimport rather than before it.

**3. `mode_descriptor` (`PTR_DAT_008b2dac`) - re-confirmed non-convergent.**
`read_memory(0x008b2dac, 8)` gives `00 93 6f e8 00 86 9c 60` - the pointer's own stored
value is `0x00936fe8`. `search_byte_patterns("00936fe8")` against the reimported image
returns 151 hits, every one in the `0x008a5xxx`-`0x008c0xxx` data range - the same
"aliases far too many places to be one struct's base address" shape the original pass
found, reproduced at essentially the same order of magnitude. `get_xrefs_to(0x008b2dac)`
still returns exactly one reader, `Billboard_CreateFromLocation_q` at `0029af64
[READ]` - the same site this page already names as where `mode_descriptor` is read.
Nothing about the reimport changes this: the value read from the global does not
resolve to a single struct, and the global still has only the one known reader. Live
RPCS3 watchpoints remain the next step, as the original pass already said.

**4. `table_base + 0x834` reader - re-confirmed at the same count.** 
`search_instructions(mnemonic="lwz", operand_pattern="0x834(")` against the reimported
image returns the same 14 matches as the original pass, not more - meaning none of the
294 previously-holed functions contains this instruction shape, so the reimport had
nothing to surface here. Two of the previously-unexamined sites were decompiled this
pass: `FUN_001560b0` (`00156128`, `lwz r9,0x834(r26)`) indexes an entirely different
struct, accessed elsewhere in the same function at `param_3+0x7820`..`+0x7870`, nowhere
near `g_BillboardSlots`'s own layout; `FUN_002d2590` (`002d25e4`, `lwz r9,0x834(r10)`) is
a four-instruction dispatcher (`RaceManager_GetInstance` -> `FUN_000557e8` -> optionally
`FUN_002d2068`) whose decompile shows no `+0x834` field access at all, so that
instruction sits inside a callee not reached by this decompile view rather than in this
function's own body against `table_base`. Neither is a plausible reader. The other nine
sites (the `-0x834(r2)` group) remain what the original pass called them: ordinary
TOC-relative single-word globals, a different shape than the `RaceManager` four-word
cursor and unrelated to `table_base`. **Still not established, at no confidence** - the
count itself, unchanged after a reimport that added instructions to 294 functions
elsewhere in the binary, is now decent evidence this reader (if it exists at all) is not
reached through a plain `lwz` at this fixed offset.

**Net effect on the handover thread.** The thread's `## Open` list separately claims the
unnamed 4.5 KB bind function at `0x003a4da0` "has no confirmed caller or invocation
path." That was already wrong before this pass touched it - this page's own 2026-09-06
section (`Billboard_LoadModelAndBind`, above) traced a real call chain
(`Billboard_ConstructResource` at `0029adb4` through the pure TOC-fixup trampoline
`FUN_006791d8` to `0x003a4da0`), unrelated to the lvlx trap since a `bl`'s target is a
relative displacement, not a TOC-resolved `lwz`. Re-run here as part of the same pass for
completeness: `get_function_callers(0x003a4da0)` returns the trampoline
`FUN_006791d8`; `get_function_callers` on that thunk in turn returns
`Billboard_ConstructResource` and its own `.opd` descriptor - the identical chain, now
reproduced on the reimported image. Nothing changed; the thread's bullet was stale
independent of the reimport and is corrected below.
