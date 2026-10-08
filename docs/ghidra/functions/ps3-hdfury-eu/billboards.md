# Billboards: how `<TrackStartup>` becomes a 9-entry slot array

Binary: `PS3_GAME/USRDIR/EBOOT.BIN` from `hdfury-ps3-eu-dec.iso`, as
`/hdfury/EBOOT-ps3-hdfury-eu.elf` in Ghidra.

The file's own layout - what `trackstartup.xml` declares, and the survey over
all 16 circuits - is [`oag_tables::trackstartup`](../../../../crates/tables/src/trackstartup.rs),
also summarised in [`docs/formats/README.md`](../../../formats/README.md)'s
"HD track startup" row. This page is the executable's side: what it reads out
of a `<Billboard>`, where it stores each one, and the one place a manifest's
own authoring is silently overridden.

**Nothing here is runtime-verified.** Per [visibility.md](visibility.md)'s
rule, static reading of *this* binary alone caps a score at **84** - except
`Billboard_ConstructResource`, whose 85 rests on a second,
independently-compiled binary corroborating it (below), per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md)'s own
85-94 band, not an exception `visibility.md` itself states.

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
| `0x003a5f68` | function | `Billboard_UpdateAndRender` | 82 |
| `0x003e4f18` | function | `Billboard_UpdateInstanceUvs` | 88 |
| `0x005f9bd0` | function | `AnimCurve_EvaluateChannels` | 78 |
| `0x0066c3c8` | function | `AnimCurve_SampleChannel` | 80 |
| `0x0066b840` | function | `EdgeAnim_EvaluateClip` | 85 |

The last three rows in the first block are 2026-09-17's - see "The runtime write
is located" below. The last two rows are also 2026-09-17's, from the later
same-day pass - see "The curve's on-disk source is Sony's Edge Animation
Tools format" further down. `AnimCurve_SampleChannel` was already used in
prose on this page before this pass (the two-line wrapper description below)
but was never actually renamed in Ghidra or recorded in `names.tsv` - a
`handover-threads-lag-docs` gap inside a single doc file, closed here rather
than propagated.

The last row of the first group is a gift, not a reading: the name is the function's own debug
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

**Corroborated on `ps4-omega-eu`, confidence raised 62 -> 85 - re-reading
this function itself, not just citing the other binary.** Re-decompiled
`0x0029a6a8` end to end for this pass rather than trusting the prose above
alone: the tag string (`PTR_s_Billboard_cpp_008b2d90`) is assigned to
`param_1[0xc]` directly, the `.mip`/`.vex` extension branch is a clean,
unambiguous case-insensitive last-three-characters check with no
unresolved paths, and both callers (`Billboard_CreateFromLocation_q`,
`Billboard_CreateFromColour_q`) are already established - "decompilation
unambiguous and every call site consistent" per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md)'s
85-94 band. What pushes it into that band rather than capping at 84 is
`ps4-omega-eu`'s own `Billboard_ConstructResource`: verbatim tag, the same
last-N-characters-uppercased extension check, and a call into its own
resource loader with the **same two magic constants**, `0xfdb2` and `0x3e9`
- values with no reason to match by chance across two independently
compiled binaries five console generations apart. See
[`ps4-omega-eu/billboards.md`](../ps4-omega-eu/billboards.md) for that
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

**Resolved on the live frame, 2026-10-08 (`hd-gantry`):** the mode does pick the file. `GetMode()` read
from `TTY.log` is 6 in a Zone race, 13 in Zone Battle and 14 in Detonator, and each shows its own slot-8 board
before the release (Single Race and Eliminator, 3 and 8, show `3 2 1 GO`). Those are the ids the PS4
binary's allowlist branches on (`ps4-omega-eu/billboards.md`). The *code site* on this binary is still unread.
See `docs/rendering/start-gantry.md`. Confidence 85.

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
  **Corroborating context from `ps4-omega-eu`, 2026-09-15**: that binary's
  own `TrackStartup_Load` implements the equivalent slot-7 substitution as a
  direct branch on its own mode-selector global rather than an opaque
  pointer chase, picking from exactly the same three of these four names
  (`321Go_HD_Zone_Battle.vex`, `321go_hd_detonator.vex`, `321Go_Zone.vex` -
  `321Go_StartFinish.vex` itself is the unsubstituted default, matching this
  page's own slot-8-is-plain reading) - see
  [`ps4-omega-eu/billboards.md`](../ps4-omega-eu/billboards.md#trackstartup_load---0x01391130).
  This confirms the four names are the right vocabulary and the mechanism is
  mode-keyed substitution, without itself resolving what `mode_descriptor`'s
  `field_0x4c` points at on *this* binary - the two could still route
  through different intermediate data to the same small set of files.
- **`type`'s consumer**, if it has one - unread by this function despite being
  authored on every slot. **Not the same on every binary**: `ps4-omega-eu`'s
  own `TrackStartup_Load` does read `type`, mapping `<none>`/`Square`/
  `Portrait`/`Landscape` to a small enum and passing it as its own
  `Billboard_ConstructResource`'s third argument (traced to that register at
  the disassembly level) - though that function's own decompiled body never
  references the argument either, so parsing it is as far as the trail goes
  there too. See
  [`ps4-omega-eu/billboards.md`](../ps4-omega-eu/billboards.md#type-is-read-here-unlike-on-ps3-hdfury-eu).
  This doesn't settle what (if anything) reads it here; it's a confirmed
  cross-binary difference, not evidence this function was mis-read.

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
using `program="/hdfury/EBOOT-ps3-hdfury-eu.elf"` throughout. Positive controls first, so a
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

## 2026-09-17: the runtime write is located, live, with a patched RPCS3 GDB watchpoint

**`lane/hd-gantry-glyph-walk`.** The confidence-40 question this page and
`docs/rendering/start-gantry.md` carried since 2026-09-13 - "a live capture
shows the digit board's glyphs light up during an actual countdown, but no
write to `uvOffset`/`uvScale` was ever isolated" - is closed on the location
and mechanism, not on the authored data behind it. `just build-rpcs3-watchpoints`
(the patched RPCS3 with working GDB `Z2`/`Z3` watchpoints,
[`rpcs3-debugger.md`](../../../reverse-engineering/rpcs3-debugger.md)) makes
this the first pass on this page with a real write trap rather than a raw
pushbuffer diff.

### Method: watch all 19 of slot 8's own instances at once, found live rather than computed

The earlier raw-diff attempts guessed at a pushbuffer byte; this attempt
instead read `Billboard_LoadModelAndBind`'s own decompile (above) for the
*runtime* addresses a watch needs, rather than computing them from the ELF's
static layout - the instance blocks are a `FwMemAllocator`/`_opd_FUN_005a2f50`
heap allocation, not something the executable's own address space fixes.
Live, over a real GDB session (`scripts/rpcs3_debugger.py`, port 2350, a
private config copy, `Assume External Debugger: true` already in the shared
stock config so a stop reply's own PC is trustworthy per that page's own
finding):

1. Read `g_BillboardSlots`' own pointer (`0x008b6f38`) for its current value
   (heap-allocated, drifts boot to boot - `0x00c48180` both times measured,
   but read fresh rather than trusted).
2. Slot 8's own per-slot struct sits at `g_BillboardSlots + 7*0x100 + 0x10`
   (`Num`-1 = 7, the same `iVar39` `Billboard_LoadModelAndBind` computes).
   Its own `+0xf8` word (`piVar29[0x3e]` in that function's own decompile) is
   the instance array's base address, read directly rather than derived a
   second way.
3. Read `*(resource+0x1c)` for the submesh count - **19**, matching
   `start-gantry.md`'s own node count for `321go_startfinish.rcsmodel`
   exactly, confirming the address arithmetic before arming anything.
4. Arm one `Z2,<instance+0x50>,0x20` per submesh (19 watches, one GDB session,
   the registry is an unbounded `std::vector` per
   [`rpcs3-debugger.md`](../../../reverse-engineering/rpcs3-debugger.md) -
   no need to guess which of the 19 is the digit board ahead of time), then
   resume and let an actual countdown play.

Connecting the GDB stub pauses the emulator immediately, so all of this - the
three reads and the 19 arms - costs zero game-time; the countdown does not
start until `resume()` is called with every watch already in place.

### Result: one write, same PC, two independent boots

Both runs (`scratch/gantry-watch-run1.log`, `scratch/gantry-watch-run3.log` in
this lane's own worktree) landed on the exact same instance index and PC,
from a fresh boot each time with a different heap layout:

| Run | instance base | hit address | submesh index | node pointer |
| --- | --- | --- | --- | --- |
| 1 | `0x3325cb40` | `0x3325ccb0` | 2 | `0x3064c1e0` |
| 3 | `0x3325fc60` | `0x3325fdd0` | 2 | `0x3064c0e0` |

Both: `GDB: Write watchpoint hit: 4 byte(s) at <addr>` at PC **`0x005f9c9c`**,
on a **non-main** PPU thread (`36320740`, not `main_thread`) - the write is
`AnimCurve_EvaluateChannels`'s own `*(float *)((uVar2 & 3) * 4 + iVar4) =
(float)dVar9;` store, 4 bytes at the target's `uvOffset.x` component. Only one
hit occurred across a 45-second real-time window in run 3 (interpreter mode
runs well under real-time, so this covers several times the authored 6.000 s
loop-close) - **the write happens once, not every frame**, consistent with a
state-transition write rather than a continuously re-evaluated one.

The value written back was `0.0` - identical to the field's own rest value,
which is why a value-diff approach (as opposed to a write trap) would have
missed this entirely. That is not a null result: `AnimCurve_EvaluateChannels`
samples `fmodf(time, curve->period)` through a curve, and a curve landing on
`0.0` at an early sample is exactly what the live-capture progression
("nothing, nothing, a faint sliver, a clear `3`, then `3` and `2`") already
showed for the board's first visible state.

### The call chain, three newly named functions

`Billboard_UpdateAndRender` (`0x003a5f68`, confidence 82) is a **per-frame**
counterpart to `Billboard_LoadModelAndBind`: it walks `g_BillboardSlots` with
the identical gating (`*PTR_g_ZoneEffectsActive_008b6f34 == 0 || slot == 7`),
does its own per-slot RSX render-to-texture setup (`Rsx_SetMethod(.., 0x1fec,
0)` / `..1)` bracketing the update, a viewport calculation through
`_opd_FUN_005c2d08`) - the first concrete evidence for
["Render-to-texture"](#the-slot-8-asymmetry-and-what-is-still-not-established)'s
own confidence-55 hypothesis above, though this is a *different* per-frame
pass from the load-time `FUN_005e5858`/`FUN_005ea2d0` pair that hypothesis
named, not a confirmation of those two specifically. For each active slot it
calls `Billboard_UpdateInstanceUvs(clock_value, *slot_resource,
slot_instance_array)` - `*slot_resource` and `slot_instance_array` matching,
address for address, the `resource handle`/`instance array base` this pass
read live off `g_BillboardSlots[7]` during an actual countdown.

`Billboard_UpdateInstanceUvs` (`0x003e4f18`, confidence 88) does two passes
per call: first, for every instance with a bound `.vex` node (`+0x70 != 0`),
a conditional overwrite of `uvOffset.xy`/`uvScale.zw` from a **per-node**
table at `node+0xe4` (a `0xffffffff` sentinel per component means "leave this
alone") - a static, per-node UV override this page had not identified before,
separate from the animated write below and consistent with the two nonzero
rest-state instances this pass's own post-window reads found (submesh 9:
`uvOffset (1.0, 0.25, 0, 0)`; submesh 18: `uvOffset (0, 0.022, 0, 0)`) on both
runs, both already resolved by load time and untouched by any watch during
the run. Second, it walks the resource's own animated-target list (`+0x2c`
count, `+0x30` array) and calls `AnimCurve_EvaluateChannels(clock, curve,
target)` for every entry whose curve pointer (`target+0x20+0xc`) is non-null -
this is the call that produced the watched hit.

`AnimCurve_EvaluateChannels` (`0x005f9bd0`, confidence 78) is a **generic**
curve evaluator - `fmodf(time, curve->period)`, then for each of the curve's
channels, `AnimCurve_SampleChannel` (`0x0066c3c8`, a two-line wrapper over an
unnamed deeper evaluator `_opd_FUN_0066b840`, left unrenamed - confidence
would be a guess at the parent's own generality) samples one float and the
result is stored into the target's own float table by `(index, component)`.
**It has five callers total and this reading is generic, not billboard-only**
- it is confirmed as *a* mechanism billboards route through, not confirmed as
existing for billboards alone.

### What this does and does not settle

**Settled:** a runtime write to the digit board's own `uvOffset` genuinely
happens, its exact address (relative to a live-read instance base, not a
static one), its exact PC, and the three-function call chain from the
per-frame billboard dispatcher down to the generic curve-sample-and-store
leaf. Reproduced identically on two independent boots.

**Not settled, and worth being precise about why implementing this is not
this pass's own next step:** the *curve data itself* - what `target+0x20+0xc`
points at, how many keyframes it carries, and where in `321go_startfinish`'s
own `.vex`/`.rcsmodel` bytes (or elsewhere) it is authored - was not decoded.
The resource handle `Billboard_UpdateInstanceUvs` receives is the loaded
`.rcsmodel`'s own in-memory object (`Billboard_LoadModelAndBind`'s own
`_opd_FUN_005da6b8` return value), so the animated-target list is very likely
populated from that same file at load time, in a section this project's
`oag-rcs` parser does not yet decode - a new format-recovery task, not
(necessarily) a `mesh/rcs.rs` question by itself. Playing it back would need,
in order: (1) locating and decoding this curve-list chunk in the `.rcsmodel`
format (`oag-rcs`), then (2) replaying it per-tick into the draw's own
`uvOffset`/`uvScale` shader constants the way `Model::write_node_anims`
already replays an `Anim Transform` track - which does reach
`crates/mesh/src/mesh/rcs.rs`, owned by `lane-hd-material-curve` while that
lane is active. Writing a synthetic offset here instead would be exactly the
invented mechanism `CLAUDE.md`'s "never invent what the assets already
author" rule exists to stop, now with less excuse than before: the real
write's address and caller are known, only its authored content is not.

Not chased further, and worth naming for whoever does: the per-node `+0xe4`
static-override table `Billboard_UpdateInstanceUvs` reads (a second, distinct
mechanism from the animated one).

## 2026-09-17: the curve's on-disk source is Sony's Edge Animation Tools format, located but not decoded

**`lane/hd-gantry-anim`, same day as the section above, later pass.** The
previous section closed the location and call chain of the runtime write and
named the "curve data itself" and "the resource it hangs off" as the open
question. Both are now answered *structurally* - the exact struct field the
curve pointer lives at, and the exact middleware whose format the bytes past
that pointer are - without decoding the middleware's own bit-packed keys,
which is a separate, larger task named at the end of this section.

### The "animated-target list" is the material table itself

`Billboard_UpdateInstanceUvs`'s own resource-level `+0x2c` count / `+0x30`
array (the previous section's "animated-target list") are **the same two
header fields `crates/rcs/src/rcsmodel.rs` already documents**:

```text
+0x2c  u32   material count
+0x30  u32   offset of the material offset table
```

`oag_rcs::rcsmodel::Model::parse`'s own `materials` field is this exact list.
Confirmed on `321go_startfinish.rcsmodel` (extracted from
`DATA02.PSARC` on `hdfury-ps3-eu-dec.iso`, the same file the geometry section
above already reads): the file's own `+0x2c` is `5`, and `Model::parse` reports
5 materials for it - matching exactly, not by coincidence of a round number,
since the file also has 19 mesh chunks and 19 submeshes, neither of which is
5. Each "target" `AnimCurve_EvaluateChannels` evaluates is one material
record, not a per-node or per-instance object of its own.

### The curve pointer is the material record's own `+0x20` field, previously undecoded

`crates/rcs/src/rcsmodel/material.rs` reads a material record through `+0x1c`
(the alpha-test reference) and stops; `+0x20` was never claimed by anything,
which is exactly the shape `oag_rcs::rcsmodel::coverage`'s own doc warns
about - "a parser cannot fail on a field it does not know about." Reading it
directly against `321go_startfinish.rcsmodel`'s own five material records:

| # | material | `+0x20` | `[+0x20]+0xc` (curve pointer) |
| --- | --- | --- | --- |
| 0 | `simpletexture.rcsmaterial` | `0xb14` | `0` - no curve |
| 1 | `simpletextureuvoffsetscale.rcsmaterial` | `0xb2c` | `0x1a30` |
| 2 | `simpletextureandtexturealphauvoffsetscale.rcsmaterial` | `0x1a38` | `0x1ed0` |
| 3 | `simpletextureandtexturealphauvoffsetscale.rcsmaterial` | `0x1ed8` | `0x2bf0` |
| 4 | `simpletextureandtexturealphauvoffsetscale.rcsmaterial` | `0x2bf8` | `0x3790` |

**Confidence 90** - not a guess at a plausible offset: every material whose
own name carries `uvoffsetscale` has a live, non-null curve pointer, and the
one material on this file whose name does not (`simpletexture.rcsmaterial`,
slot 0's own material, the one node with nothing to animate) has a null one.
The correlation with the material *type* is the reading, not the address
pattern alone.

At the curve pointer (e.g. `0x1a30` for material 1): `+0x00` is a further
pointer (`0xb50`, called `inner` below) to the middleware object identified
below, and `+0x04` onward is a small array of `u16` channel descriptors -
target-table index in bits 2-15, component (0-3) in bits 0-1, read directly
off `AnimCurve_EvaluateChannels`'s own decompile (`uVar2 & 0xfffc` for the
index, `uVar2 & 3` for the component). **The channel count is not in the
curve struct itself** - `AnimCurve_EvaluateChannels` reads it from
`inner + 0x24` (`*(ushort *)(iVar3 + 0x24)`, `iVar3` being `*param_2`, the
curve's own `+0x00`), so reading the descriptor array without checking that
word first would be guessing how many to print. Checked on all four live
curves of `321go_startfinish.rcsmodel`: `inner+0x24` is `2` on every one,
matching `curve+0x04`/`+0x06` exactly - `0x0008`/`0x0009`, both target index
2, components 0 and 1.

**Target index 2 is `uvOffset`, not inferred from the parameter's position
but read end to end.** `AnimCurve_EvaluateChannels` resolves a channel's
destination through the material's own parameter table at `+0x34` (the same
0x20-byte-stride table `oag_rcs::rcsmodel::material::parameters` decodes):
`(target_index & 0xfffc) * 8` (i.e. `target_index * 0x20`) indexes into it.
Material 1's own table has four entries; index 2's own name hash is
`0x1eb13436` and index 3's is `0xea1dcc4c` -
**`oag_rcs::rcsmaterial::name_hash("uvOffset")` and `name_hash("uvScale")`
respectively, exact matches, both kind `0` (a parameter, not the `0x8001`
sampler kind entries 0 and 1 carry)**. So target index 2, components 0 and 1,
is confirmed as **`uvOffset.x` and `uvOffset.y`** by the same hash this
project's own parser already computes, not by assuming the animated
parameter must be the UV one - matching the field the 2026-09-17 GDB watch
(previous section) caught being written.
All four also share one more value at `inner+0x04`, the period `fmodf`
divides by: `13.333333` seconds, unread by anything else on this page and
not obviously tied to the 6.000 s teleport or the measured 4.533 s thrust
gate - a fourth number in the same "nothing here reconciles the countdown's
several timings" family `start-gantry.md` already tracks.

### The middleware: Sony's own Edge Animation Tools, identified by its own assert strings

`AnimCurve_SampleChannel` (`0x0066c3c8`, a two-line wrapper, renamed this pass
- it was described in prose on this page before today but never actually
applied in Ghidra or `names.tsv`) calls a deeper evaluator at `0x0066b840`,
renamed `EdgeAnim_EvaluateClip` this pass. That function's own body carries
two `_SCE_Assert` calls whose string arguments are not this project's own
code and not previously seen anywhere else on this page:

```text
0x007d48b0  "edgeanim_evaluate_ppu.cpp:469 (anim->offsetPackingSpecs == 0)"
0x007d48f0  "edgeanim_evaluate_ppu.cpp:283 (frameInteger <= intraFrameCount)"
```

**`edgeanim_evaluate_ppu.cpp` and `anim->offsetPackingSpecs` are Sony's own
source file name and struct field name, from the PS3 SDK's Edge Animation
Tools middleware** (part of the same Edge suite PS3 titles across many
studios link for compressed skeletal/curve animation) - not Wipeout's own
code, not decompiled from a stripped symbol, read straight off the binary's
own embedded assert strings the same way `GetBillboardMeshIdFromName` was
named off its own debug string. That identification - "this is Edge's own
evaluator, not a bespoke Wipeout format" - is as solid as a self-describing
string gets. **The function's own name, `EdgeAnim_EvaluateClip`, is softer
than that**: the strings prove the middleware and the source file, not that
this particular entry point evaluates a whole clip rather than one track or
one channel within Edge's own call layers, which is inference from this
function's decompile shape rather than from a string. **Confidence 85**
reflects the gap between the two - the same shape `AnimCurve_EvaluateChannels`
itself sits at 78 for.

The evaluator's own body (fully decompiled, not excerpted here for length)
does exactly what a compressed keyframe codec looks like: `fmodf` against a
period, a binary search over a frame-index table, several bit-packed
sub-tables located by non-constant offsets read out of the clip header
(`+0x38`, `+0x3c`, `+0x40`, `+0x44`, `+0x48`), fixed-point-to-float
conversion, and a `vectorReciprocalSquareRootEstimateFloatingPoint`-based
slerp for quaternion tracks. That shape is consistent with Edge's own
documented purpose (bit-packed, quantised keyframe animation) and
inconsistent with a hand-rolled Wipeout format - nothing this simple.

**This also gives a candidate location for the file's own largest coverage
gap, though not a settled one.** Before any of the above was traced,
`oag_rcs::rcsmodel::coverage` over `321go_startfinish.rcsmodel` reported one
14,388-byte unclaimed run at `0x5dc`-`0x3e10`, between the mesh offset table
and the first mesh chunk header. The first 1,060 bytes of it are the file's
own relocation table (`render_block.rs`'s documented `{count, offsets[]}`,
just never `seen.claim`ed by `coverage.rs`); in the remaining ~13.3 KB,
`0xa00`-`0x3e10`, **every pointer this pass traced lands inside it** - the
four `curve` structs at `0x1a30`, `0x1ed0`, `0x2bf0`, `0x3790` and the four
`inner` objects at `0xb50`, `0x1a50`, `0x1ef0`, `0x2c10` - with structured,
non-zero bytes around each rather than padding. **Confidence 75** that this
region is the Edge clip payload and nothing else: the pointers are measured,
but no clip's own *extent* is known (that needs the clip header layout,
which is exactly what is undecoded), so whether four clips tile the whole
13.3 KB or leave a further, unrelated section inside it is not established.
Still useful for whoever decodes Edge's own format next: the byte range and
eight entry points are already found, without re-running the run-detection
scan this pass used to first notice the region was structured.

### This is not 2048's `.rcsanimclip`/`.rcsskeleton`, and not the same container magic

Checked directly before concluding a new format was needed, per this lane's
own brief: no `.rcsanimclip` or `.rcsskeleton` file exists in any of HD's
seven `DATA0*.PSARC` archives (`psarc_list` filtered on both substrings
against all seven returns zero matches on `hdfury-ps3-eu-dec.iso`), and
`321go_startfinish.rcsmodel`'s own bytes contain no occurrence of `0xca5caded`
in either byte order - `2048-animation.md`'s own container magic. HD's curve
data is a third, independent mechanism from both Pulse's material-authored
`TEXOFFSET` track and 2048's skeleton/clip pair: Sony's Edge Animation Tools,
reached through a material-record field this project's own parser had not
read yet.

### What this does and does not settle

**Settled:** the curve pointer's exact address, relative to a material
record this project already parses (`material_record + 0x20`, then `+ 0xc`);
which materials on this file carry one (4 of 5, all and only the
`uvoffsetscale` variants); the channel-to-component mapping for material 1
(index 2, components 0 and 1 - `uvOffset.x`/`.y`); and the identity of the
middleware whose format the bytes past the curve pointer are in, from the
middleware's own embedded assert strings rather than from a guess.

**Not settled, and the real next step:** Edge Animation Tools' own bit-packed
key encoding - the frame-index table, the per-component bit widths, the
fixed-point scale/bias and the quaternion reconstruction - is not decoded.
That is a format-recovery task in its own right, scoped to Edge's own
container rather than to Wipeout's, and plausibly reusable well past this one
billboard if Edge is used elsewhere in HD's animation system (nothing here
checked that). Per this lane's own standing instruction, no value is
synthesised in its place: `crates/rcs/examples/hd_gantry_curve_probe.rs`
reproduces every reading on this page directly off the extracted file and
stops exactly where the bytes stop being Wipeout's own to read.

## 2026-09-17: Edge's own byte layout is decoded (non-bit-packed path only), and the curve is a wipe, not a glyph selector

**`lane/hd-edgeanim`, later same day.** See
[`docs/formats/edge-animation.md`](../../../formats/edge-animation.md) for
the full byte layout, self-relative offset convention, frame-set tables and
scalar-keyframe evaluator, implemented in `oag_rcs::edgeanim` and
`oag_rcs::rcsmodel::material::curve`, validated against all four curves of
`321go_startfinish.rcsmodel` by `hd_gantry_curve_ground_truth.rs`. In short:
every clip this pass found has `offsetPackingSpecs` absent (the bit-packed
path the previous section left undecoded is simply not used by any of the
four curves this file authors) and no rotation/translation/scale channel, so
the "not decoded" scope above turned out to be for a codec path these
particular curves never exercise - the scalar path they do use has no
bit-packing at all, only a sparse-keyframe presence bitmap and plain `f32`
values. **What the four curves actually author is a per-material wipe/reveal
ramp staggered across the shared 13.333 s loop, not a four-state glyph
selector** - see the format page for the measured shape of all four.
Playback is not wired: no time base connects this curve's own loop to the
countdown a player sees, and the per-node `+0xe4` static UV override table
this page's earlier section already named is at least as likely the real
glyph-selection mechanism - unchased, per this lane's own scope, and the lead
for whoever picks this up next.

## 2026-09-17: playback is wired generically, and the `+0xe4` table is chased and ruled out

**`lane/hd-gantry-wire`.** Two things, in the order this lane did them.

### The generic curve replay is wired

`docs/formats/edge-animation.md`'s own updated section has the full account:
every `.rcsmodel` material carrying a curve now reaches
[`oag_mesh::mesh::AnimTrack::Rcs`](../../../../crates/mesh/src/mesh/anim_track.rs),
sampled each frame through the same per-model animation clock Pulse's own
`TEXOFFSET` tracks already ride - no new time base, and not gantry-specific:
79 of the disc's 379 `.rcsmodel` files carry at least one live curve, and all
79 now replay instead of freezing at frame zero. **On the digit board this
turns out to be the whole countdown mechanism, and it is one curve, not
four**: material 2's own curve alone reaches the screen, driving both the
digit glyph mesh (`pasted__Go_HD_start_light_321goShape`, the node with the
five UV cells) and the backdrop panel (`Go_HD_start_light_backgroundShape`)
that share its texture. Materials 1/3/4's own curves are real and now
replay correctly too, but their geometry - the chequered-flag state, slot
7's embedded `fx350` art and the `FINAL LAP` state respectively - is removed
from every frame by mechanisms this project already had
(`oag_render::gantry::clip_to_panel`, `::strip_fx350_art`), so they never
reach a player's eye regardless. Confirmed by actually playing it
(`scratch/lane-wire-report.md`'s captures) and by a control render with
every curve disabled, which drops the backdrop and the digits together -
not by reading the curves' own sampled numbers in isolation, which is what
made a single curve look like a wipe with no state to select in the first
place, and what made "four curves" look like the mechanism before the
node-level check ran.

### `Billboard_UpdateInstanceUvs`'s first pass, read precisely

The function's own Ghidra plate comment (already at confidence 88, this
lane's own re-decompile just reads it rather than re-deriving it) is more
precise than this page's earlier prose: **`node+0xe4` is a pointer, not an
inline table.** Read off the decompile directly:

```c
iVar2 = *(int *)(*(int *)(instance + 0x70) + 0xe4);   // the node's own pointer field
if (iVar2 != 0) {
    // per-component override, one float each, contiguous at [iVar2+0x18..0x28):
    instance->uvScale.z  = sentinel(*(float *)(iVar2 + 0x18), instance->uvScale.z);
    instance->uvScale.w  = sentinel(*(float *)(iVar2 + 0x1c), instance->uvScale.w);
    instance->uvOffset.x = sentinel(*(float *)(iVar2 + 0x20), instance->uvOffset.x);
    instance->uvOffset.y = sentinel(*(float *)(iVar2 + 0x24), instance->uvOffset.y);
}
```

where `sentinel(v, keep) = keep if v's bit pattern is 0xffffffff else v` - the
per-component "leave this alone" rule this page already named, now with the
exact four float slots it applies to (`uvScale.z`/`.w`, `uvOffset.x`/`.y`,
packed contiguously at the pointer's own `+0x18`) rather than "a per-node
table" left unspecified. **Confidence 88, unchanged** - this is the same
decompile the previous pass already scored, read more carefully rather than
re-derived.

### Live result: `node+0xe4` is `NULL` on every one of slot 8's 19 instances, at every point checked

`scratch/gantry_dump_e4.py` (this lane's own script, reusing
`scripts/rpcs3-drive.py`/`rpcs3_debugger.py` exactly as
`scratch/gantry_watch.py` does - no new watchpoints, a plain memory read) walks
to a race on Talon's Junction under the patched interpreter build, connects
GDB the instant "Loading Screen Finished" prints (which pauses the CPU with
zero game-time elapsed), and reads `*(node+0xe4)` for all 19 of slot 8's own
`321go_startfinish.rcsmodel` instances - then resumes, waits, pauses and
re-reads twice more, at +3 s and +6 s of real countdown time (the second
squarely inside `GO`'s own 3.6-5.97 s window per `start-gantry.md`).

**All 19 pointers are `0x00000000` at all three checkpoints**, including
submesh index 2 - the digit board's own submesh, the one
`AnimCurve_EvaluateChannels`'s 2026-09-17 watch found actually receiving the
animated write. The first pass's own `if (iVar2 == 0) goto skip` branch fires
on every instance, every time this was checked: **the whole first pass is a
no-op for this file, on this circuit, across an entire countdown.** This is a
live, direct read of the pointer itself, not an inference from its absence of
effect - as strong a negative as this project's own static-Ghidra ceiling
allows a dynamic check to be. **Confidence 90** - a live GDB read, reproduced
at three points across a real countdown, is the same evidentiary shape
`billboards.md`'s own confidence-90 disc-reading rows already carry, capped
below the 95 a cross-boot reproduction would need since only one boot did the
check this time (the 2026-09-17 curve-write pass above did reproduce across
two boots; this did not, for time).

**This retires the "at least as likely" candidate status this table has
carried on this page since the write chain was first traced.** The per-node
`+0xe4` table is not the digit-selection mechanism for `321go_startfinish.vex`
- it is dead weight on this file, at least on the one circuit checked. Nothing
about a per-node field baked into the `.vex`'s own node payload should vary by
circuit (the file's bytes are the same wherever it loads), so this is treated
as settled for the file rather than circuit-specific, though only one circuit
was actually run.

**This also resolves a loose end from the previous pass, rather than leaving
it dangling.** That pass found two non-identity rest-state `uvOffset` values
(submesh 9: `(1.0, 0.25, 0, 0)`; submesh 18: `(0, 0.022, 0, 0)`) and attributed
them to "this table... resolved by load time" without checking the table
itself. With `+0xe4` now confirmed `NULL` throughout, those two values cannot
come from this mechanism at all - they are simply the affected materials' own
**static** `uvOffset`/`uvScale` parameters (`Billboard_LoadModelAndBind`'s own
step 3, "binds `+0x50`/`+0x60` as the shader constants... matched by
`Crc32_HashString`", a separate, earlier load-time write from this function's
own first pass), the same static parameters
[`oag_rcs::rcsmodel::material::parameters`](../../../../crates/rcs/src/rcsmodel/material/parameters.rs)
already decodes and [`curve_track::material_anim_tracks`](../../../../crates/mesh/src/mesh/rcs/curve_track.rs)
now reads as every curved material's own rest value.

### What this settles, and what it still leaves open

**The `3`/`2`/`1`/`GO` selection mechanism on HD is resolved: it is material
2's own Edge Animation curve, alone, driving both the digit glyph mesh and
its backdrop panel - the same "one shared offset on one material" shape as
Pulse's own gantry, not a distributed multi-curve one.** Of the two named
candidates for "what picks a state", one is now closed without a positive
(the per-node `+0xe4` override, this section) and the other resolved to a
single curve once a node-level check ran: materials 1, 3 and 4 do carry real
curves too, but their own geometry (the chequered-flag state, slot 7's
embedded `fx350` art, and the `FINAL LAP` state) is removed from every frame
by `oag_render::gantry::clip_to_panel`/`::strip_fx350_art`, mechanisms this
project already had before this lane. Confidence 88, from real headless
captures across a countdown plus a curve-disabled control render
(`scratch/lane-wire-report.md`), not from re-reading the curve bytes alone.

**What is not settled**: `3` and `2` reveal together in the captures rather
than strictly one after another, the full sequence takes about 4 s of the
asset's own clock against the ~6 s the measured thrust gate takes, and only
one circuit and one boot were checked. None of these were smoothed over or
explained away - they are reported as measured. The `+0xe4` table's own
negative is unaffected by any of this and stands on its own live evidence.

## 2026-10-04: the curve's time argument is the gantry node's clock, windowed by the race manager

**`hd-go-pulse`.** The clock `Billboard_UpdateAndRender` hands
`Billboard_UpdateInstanceUvs` is `AnimNode_GetTime(slot+0xc)`, the `.vex`
node's own `+0xc0` animation time, and HD's race manager holds that time in a
per-lap window (`[3.83, 5.25)` s before the first line crossing). The full
chain, the window table and the identity argument for slot 8's node are on
[gantry-clock.md](gantry-clock.md).

## 2026-10-06 (billboards lane): the Pulse render-to-texture finding, checked against `Billboard_LoadModelAndBind` - applies, not wired

A live PPSSPP frame of Pulse shows each advert drawn through its own `Camera`
node into a 128 x 128 offscreen buffer that the circuit's `billboardN` quad then
samples ([psp-pulse-usa/billboards.md](../psp-pulse-usa/billboards.md), 2026-10-06
section). Re-reading `Billboard_LoadModelAndBind` (`0x003a4da0`, 80) with that in
hand, **it is the same shape** (static reading only, no RPCS3 capture, so 60):

- the slot's `.rcsmodel` loads into `g_BillboardSlots + (num - 1) * 0x100`, and the
  function collects the model's nodes of one class (the loop filling
  `piVar29[4]`/`piVar29[5]`; the class tag is read through a TOC pointer and not
  identified here - Pulse's analogue is `Camera`, `0xf7`) and gives each a matrix block;
- `FUN_005e5858(*g_BillboardSlots, slot + 0xf4)` and `FUN_005ea2d0(..., 0xffffffff)`
  then build a render target for the slot and set it up with a set of matrices and a
  clear colour - the object a render pass draws into;
- the materials named `"billboard" + num` get their texture pointer set to
  `*(slot + 0xf4) + 0x20`, **the target's own colour texture**, which is what "binds
  by material name" meant.

So on HD too the advert is drawn to a texture and the placeholder quad's material is
pointed at it, and the placeholder-draw WARN (32 on HD) is the same absence. **Not
wired here, deliberately**: the HD pass's projection law, target size and clear are
unmeasured (the Pulse law is the `.vex` camera's u16 curve; HD's `.rcsmodel` adverts
and its own `VexCamera`-class equivalents were not read), and a Zone race takes the
other branch of the function (the `else` that binds one shared `"billboard"` texture to
every slot but 8). Status: **checked, applies, not wired**; the next step is an RPCS3
capture of one advert pass, then `oag_raceplay::adverts::load` with HD's model reader.


## 2026-10-06 (billboards-3 lane): the advert pass measured on RPCS3, and wired

The earlier "checked, applies, not wired" section is closed: the target, the
projection law and the clip planes were read live and `oag_raceplay::adverts` now
draws HD's adverts through them (`oag_title::adverts::Adverts`, `Title::adverts`).

### Method

`scripts/rpcs3-drive.py capture --region` on a private display and GDB port, into a
campaign race on Talon's Junction, stopped 60 s after the load, with the chain
`0x008b6f38@:0xa00` (the word at `g_BillboardSlots` is the heap block, `0x00c48180` on
both boots) and further chains to the objects it points at. Slot `k` (1 to 8) is
`P + 0x10 + (k - 1) * 0x100`, as `Billboard_LoadModelAndBind` indexes it. Dumps:
`data/scratch/billboards-3/cap2/`.

### The per-slot struct, as read

| Offset in the slot struct | Content | Confidence |
| --- | --- | --- |
| `+0x30` (4 x 4 floats) | the view matrix: identity until the slot's pass runs, then the inverse of the advert's `camera1` (slot 2: translation `(0, 0, -30)`, the `.vex` camera's z; slot 3: `-35.4028`; slot 5: `-12`; slot 8: `-30`) | 90 |
| `+0x70` | view x projection, written by `Billboard_UpdateAndRender` (zero where the slot did not run) | 85 |
| `+0xb0` | the projection | 90 |
| `+0xf0` / `+0xf4` | the render-target objects, whose `+0x1c` is `0x200` and `+0x20` is `0x100`, `+0x28` (pitch) `0x400` | 88 |

### The numbers (all eight slots, one frame)

`x` and `y` are the projection's `[0][0]` and `[1][1]`; `fov` and `aspect` are the
slot's own `.vex` `Camera` node (`oag_vex::camera::Camera`, the u16 at payload `+0x22`
times `180 / 65535`, and `+0x1c`).

| Slot | Model | fov | aspect | x live | y live | `1 / tan(fov / 2)` |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `Icaras/Looping_Background` | 14.4747 | 1 | 7.8745 | 7.8745 | 7.8745 |
| 2 | `EGX/EGX_LANDSCAPE_03` | 68.8771 | 2 | 1.4584 | 2.9167 | 1.4585 |
| 3 | `AG_Systems/AG_Systems_Square` | 58.3218 | 2 | 1.7922 | 3.5843 | 1.7925 |
| 4 | `Piranha/Piranha_LANDSCAPE` | 9.6764 | 2 | 11.8143 | 23.6285 | 11.814 |
| 5 | `Auricom/Auricom_LANDSCAPE_01` | 18.0398 | 2 | 6.2996 | 12.5992 | 6.2995 |
| 6 | `Ignition/Ignition_LANDSCAPE_01` | 19.1879 | 2 | 5.9161 | 11.8323 | 5.916 |
| 7 | **not `fx350`** (below) | 70.6267 | 2 | 1.4117 | 2.8233 | 1.4117 |
| 8 | `321Go/321Go_StartFinish` | 61.7276 | 4 | 1.6733 | 6.6931 | 1.6733 |

- **The law is Pulse's**: `x = 1 / tan(fov / 2)`, `y = aspect * x`. **Confidence 90**
  (eight of eight slots to four digits). Pinned by
  `adverts::tests::hd_projection_matches_the_matrices_read_live_off_rpcs3`.
- **Near plane exactly 0.5; far between 5400 and 5700.** The matrix is GL style
  (`-(f + n) / (f - n)` = -1.0001818, `-2fn / (f - n)` = -1.0000910): the near term
  solves to 0.5000000 and the far term moves by one float step across that range, so
  5500 is a value inside it. A flat card does not depend on it. **85 near, far a range.**
- **The target is 512 x 256.** Two sources: the literal rectangle
  `FUN_005d7838(ctx, 0, 0x200, 0, 0x100, ...)` in `Billboard_UpdateAndRender`
  (`0x003a5f68`; the callee passes `(x1 - x0, y1 - y0)` through to a viewport setter,
  **60** for that reading alone) and the live objects
  (`0x200`, `0x100`, **88**; a pitch of `0x400` at 512 wide would be 16 bits per pixel, an inference). Pulse's
  is 128 x 128, so the two titles differ in size but not in law: a wide advert is
  not squashed into a square on HD (aspect 2 on a 2:1 target frames undistorted).
- **Clear**: `Rsx_SetColorClearValue(ctx, 0)` then `Rsx_ClearSurface(ctx, 0xf1)`, a
  transparent black like Pulse's. **70**.
- **Proximity gate**: slots 1, 4 and 6 had identity views and zero products in the
  sampled frame, because `Billboard_UpdateAndRender` runs a slot only when its bit in
  `*PTR_DAT_008b6f28` is set and `*(char *)(slot[2] + 0xa0)` is nonzero - the same
  gate the original PS2 card pass showed. This project draws every card every frame.
  **80** for the gate existing, nothing known about what sets the bit.

### Slot 7 is not `fx350` in a campaign race

`mode_descriptor` (`PTR_DAT_008b2dac`) read live on the same frame:
`[[0x008b2dac] + 0x90]` points at the string `grid8_3_1`, `[0x008b2dac] + 0x58` is
`grid8`, and `[[[0x008b2dac] + 0x4c] + 0xf0]` is the string
`Data/Billboards/HD_Adverts/Blitzed/Blitzed_board.vex` - the exact branch
`Billboard_CreateFromLocation_q` takes for `num == 7` (above). The slot's live camera
(fov 70.6267, aspect 2, z 12) is the camera all eight `<grid>_board.vex` files share
(`aftermath`, `blitzed`, `corruption`, `impact`, `nuked`, `turbulance`, `voltage`,
`vortex`; read off DATA00). **Confidence 90 for the campaign case.**

**Outside a campaign** the branch condition (`field_0x90 != 0`) is false when no event
id is set, so the manifest's `fx350.vex` stands, which is what this project draws.
That is an inference from the condition, **60**, not a read: the next capture is
`0x008b2dac@+0x90` in a race that did not come from the campaign. This project does
not map a campaign grid to its board (`grid8` is `Blitzed`; the other seven pairings are
unread), so an HD campaign race, once one exists, would show `fx350` where the original
shows a board.

### What is wired, and what is not

- Wired (Talon's Junction, default race): slots 2, 3, 5, 7 have a quad and a model and
  get a 512 x 256 card; the load report's `31 billboard-slot placeholder draw(s)
  suppressed` WARN is gone. Frames: `data/scratch/billboards-3/hd-cards.png` (the four
  cards: EGX, AG Systems "A Friend in Speed", Auricom, FX-350, each upright).
  Tests: `adverts::tests::hd_builds_a_512_by_256_card_for_each_slot_with_a_quad`,
  `hd_cards_draw_something_into_their_targets`,
  `tests::scene_build::hd_points_every_served_placeholder_at_its_card`.
- **Not drawn, and the WARN names it**: a Zone race (`g_ZoneEffectsActive` makes
  `Billboard_LoadModelAndBind` bind one shared `"billboard"` texture to every slot but
  8; that texture is unread, `Adverts::zone_shares_one_texture`); a colour slot with a
  quad (`04_chenghou_project` slots 1 and 2): `Billboard_CreateFromColour_q`'s pool order
  is unread on HD, `Adverts::colour_pool` is false. Circuits whose colour slots have no
  quad (`02_track`, `05_ubermall`, `12_sol_2`, `10_sebenco_climb`) log nothing.
- **Orientation**: HD's quads author V running down (slot 2: `v` 0.16 at `y` -59.9 and
  0.89 at `y` -82.9), so the target, top row first, is sampled upright with the authored
  V and `Adverts::flip_v` is false. **Unverified against an original frame**: the
  RPCS3 captures (`cap5-sheet.png`) show the original's boards (`blitzed` upright on boards in
  four of the eight frames) but ours had no hoarding in an equivalent view.
- Not measured: the card clock (slot 8's is `gantry-clock.md`; the others run on
  this project's scenery clock, chosen), the 16-bit target format (ours is RGBA8).

### Other titles

- **Omega**: checked, applies, not wired. `Billboard_ConstructResource` carries HD's two
  magic constants (above); no PS4 capture path exists here to measure the pass.
- **2048**: checked, not wired. No `trackstartup.xml` or `billboard` quad was found on a
  default race, so there is nothing to wire; not searched beyond that.
