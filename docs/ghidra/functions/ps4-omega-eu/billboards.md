# Billboards: `Billboard_ConstructResource` and `TrackStartup_Load`, transferred by tag, magic number and element census

2026-09-15. Functions in `eboot.bin` (WipEout: Omega Collection, PS4,
`CUSA05670`, EU), `x86:LE:64:default`, image base `0x01000000`. Continues the
`.cpp`-tag transfer technique [`README.md`](README.md) and
[`weapons.md`](weapons.md) already used, applied here to
`ps3-hdfury-eu/billboards.md`'s own `Billboard_ConstructResource` and
`TrackStartup_Load`.

**The names here are applied**, from [names.tsv](names.tsv).

## `Billboard_ConstructResource` - `0x016e53a0`

**Confidence: 87**

`search_strings("Billboard.cpp")` finds exactly one match, the tag
`C:\WOPS4\Wipeout\Code\System\Render\Billboard.cpp` (under `System\Render\`,
matching the subsystem HD's own tag names rather than a flat layout).
`get_xrefs_to` names two writers: `ModeManager_ConstructByMode` (a data
reference only - one of that dispatcher's many per-mode tagged sub-objects,
not chased further) and `FUN_016e53a0`, which writes it as
`param_1[0xb]` after the usual vtable/destructor pair, the same
tagged-constructor idiom every other name on this binary uses.

**The match to `ps3-hdfury-eu`'s own `Billboard_ConstructResource`
(`0x0029a6a8`, confidence 85 after this finding, up from 62) is structural,
not just the tag.** Both functions open by reading the resource path's last
few characters, uppercased, and branching on the extension:

- `.mip` (checked first on both binaries) takes one allocation path.
- `.vex` (the `else` branch on both) allocates a much larger object through
  a generic resource loader, called with **the same two magic constants**:
  `0xfdb2` and `0x3e9`. `FUN_0170e000(uVar18, param_2, uVar35|0x200000,
  0xfdb2, 0x3e9, 8)` here against HD's `FUN_002c1ec8(..., 0xfdb2, 0x3e9)` -
  arbitrary-looking values with no reason to agree by chance across two
  independently compiled binaries five console generations apart. HD's own
  page already ruled `0x3e9` out as a vex node class (`oag_vex::vex`'s
  `CLASS_*` table has no member there); this binary's exact match doesn't
  resolve what it is, only that it's the same tag in the same otherwise-
  unidentified resource-type enum on both.

This binary's version is far less inlined than HD's: the `.vex` branch here
also contains, inline, the two `GetBillboardMeshIdFromName`-shaped error
strings (`"GetBillboardMeshIdFromName: No model loaded for billboard %i\n"`,
`"...Couldn't find name %x\n"`) that HD keeps in a **separate** function
(`GetBillboardMeshIdFromName`, `0x003a4b70` on `ps3-hdfury-eu`) - the mirror
image of `weapons.md`'s own `WeaponExplosions`/`EMP` findings, where this
binary folds a sibling HD keeps separate into the caller's own body rather
than the other way around. No separate function was found here to carry
that name, so `GetBillboardMeshIdFromName` gets no `names.tsv` row on this
binary.

`get_function_callers` finds exactly one caller, `TrackStartup_Load`
(`0x01391130`, named below).

**Confidence 87** (same band as the nine weapon managers in `weapons.md`:
tag match plus a second, independently-compiled binary corroborating the
same class - here doubly so, since the corroboration includes two
arbitrary magic constants agreeing exactly, not just the tag and the general
constructor idiom).

## `TrackStartup_Load` - `0x01391130`

**Confidence: 85**

`Billboard_ConstructResource`'s only caller, decompiled in full. One
function parses the whole `TrackStartup.xml`-shaped element tree by literal
`strcasecmp` against element names - `TrackStartup`, `LevelFx`,
`UnderwaterSound`, `WindSound`, `LoadSoundBank`, `Billboard` - the same
element set `ps3-hdfury-eu/billboards.md`'s own `TrackStartup_Load`
(`0x000b19a8`) names, in the same nesting, down to `Billboard`'s own
attribute set (`Type`, `Num`, `Location`, `Color`/`Colour`, `Glow` - see
"`Type` is read here" below for the one attribute HD's own reading marks
as unconsumed) and a `Filename` attribute under `LoadSoundBank`. Both
constructor calls
(`Billboard_ConstructResource(pauVar18, uVar5, iVar9, uVar16)`, one per
branch) confirm this single function does what HD splits across
`TrackStartup_Load` plus `Billboard_CreateFromLocation_q`/
`_FromColour_q` - one more instance of this binary folding a sibling
HD keeps separate into one body, the same shape `weapons.md`'s
`WeaponExplosions`/`EMP` findings and this page's own
`GetBillboardMeshIdFromName` finding above already show.

**This also resolves `ps3-hdfury-eu/billboards.md`'s own long-open
`mode_descriptor` question** - "whether the four `321Go_*.vex` names really
are what `field_0x4c + 0xf0` resolves to." This binary's slot-7 special case
is not an opaque pointer chase: it's a direct `strcasecmp`-free branch on
`DAT_01f999e4` (the mode-selector global `weapons.md`'s
`Rocket_Construct`/`ModeManager_ConstructByMode` findings already establish
as "the current mode"), substituting the manifest's own
`321Go_StartFinish.vex` with one of exactly the same names HD's own string
census found sitting nearby in memory:

```c
if (mode == 0x15 || mode == 0xd)      strncpy(-> "321Go_HD_Zone_Battle.vex")
else if (mode == 0xe)                 strncpy(-> "321go_hd_detonator.vex")
else                                   strncpy(-> "321Go_Zone.vex")
```

Verified against the raw disassembly, not just the decompile, since a shared
branch target across two `CMP`s is exactly the kind of thing a decompiler
could collapse wrongly: at `0x01391cab`/`0x01391caf`, `CMP R14D,0x15` /
`JZ 0x01391cbd` is immediately followed at `0x01391cb7`/`0x01391cbb` by
`CMP R14D,0xd` / `JNZ 0x01391d0b` - so mode `0xd` falls straight through into
the *same* `0x01391cbd` write site mode `0x15` jumps to directly. Two
separate compares, one write, confirmed at the instruction level.

Gated on the manifest's own `321Go_StartFinish.vex` and, from
`0x01391c63`-`0x01391c7b`, an explicit **four-mode allowlist**, not a vague
"display-mode flag": `(mode - 6) < 0x10` bounds it to modes `6`-`21`, then
`BT ECX,EAX` against the literal `0x8181` tests one specific bit of that
range - `0x8181` has bits 0, 7, 8 and 15 set, i.e. modes `{6, 13, 14, 21}`
exactly, the same four the `else if` chain above branches on (`0xe`=14,
`0xd`=13, `0x15`=21) plus `6` itself, which falls through the chain to the
`else` (`"321Go_Zone.vex"`) rather than getting a case of its own. A second,
independent path into the same substitution exists if the allowlist test
fails: `byte [0x0203a728]` non-zero **and** `FUN_01669270()` non-zero - an
unread debug/build flag, not chased further. The same
"substitute only in the right context" shape HD's own reading inferred from
`mode_descriptor->field_0x90`. **HD's four-candidate lead is now a
confirmed mechanism on a sibling binary**, even though this doesn't reach
back and read HD's own `field_0x4c` struct - the two binaries could still
implement the mode-to-name mapping through different intermediate data,
just picking from the same small vocabulary of named `.vex` files.

**A second, PS4-only substitution exists alongside it**: when a different
global (`DAT_01e31910`, a race-manager-shaped pointer) is set and its
`+0x21c` field reads `1`, `321Go_2048.vex` (not `321Go_StartFinish.vex`) is
substituted with `321Go_2048_Combat.vex` - a fifth countdown-gantry variant,
named for a 2048-specific "Combat" mode this project has not previously
catalogued from this angle. Not chased further - which mode `+0x21c == 1`
identifies is a `RaceManager` question, out of scope for this pass.

## `Type` is read here, unlike on `ps3-hdfury-eu`

`ps3-hdfury-eu/billboards.md` states plainly that `type` "is never compared
against in this loop, despite every one of the disc's 118 `<Billboard>`
elements authoring it - it is read by something else, or by nothing," and
lists "`type`'s consumer, if it has one" as still open. On this binary it
is read: the first attribute `TrackStartup_Load`'s `<Billboard>` loop
checks is `strcasecmp(&DAT_0183a3ad, ...)`, and `inspect_memory_content` on
`0x0183a3ad` reads the literal `"Type"`. Its value is copied into a local
buffer, then matched against four literals to produce a small integer:

```c
mode = strcasecmp(value, "<none>") ? -1 : 0;
if (strcasecmp(value, "Square")    == 0) mode = 1;
if (strcasecmp(value, "Portrait")  == 0) mode = 2;
if (strcasecmp(value, "Landscape") == 0) mode = 3;
```

**Traced to a specific argument, not just "one of the arguments passed",
by disassembly rather than by trusting the decompiler's own argument
list**: at both call sites (`0x01391eb2` and `0x01392202`), `MOV
EDX,dword ptr [RSP + 0x44]` loads this enum from the stack slot it was
stored to at `0x01391db0`, and nothing between that load and the `CALL
0x016e53a0` a few dozen instructions later touches `EDX`/`RDX` - checked
instruction by instruction, not assumed. Under the SysV calling
convention (`RDI, RSI, RDX, RCX, R8`), that makes the enum
**`Billboard_ConstructResource`'s third argument (`param_3`)** - and
`Billboard_ConstructResource`'s own decompiled body (above) **never
references `param_3` anywhere**, grepped for it directly. So the `Type`
attribute is parsed and passed, but the function it's passed into does not
read it, at least not in the body this pass decompiled. Whether that
value reaches something through a path this decompile doesn't show (a
register spilled to an unread stack slot, say) was not chased further.

**This is a genuine cross-binary difference in what gets *parsed*, not a
corrected gap in HD's own reading** - `ps3-hdfury-eu/billboards.md`'s claim
is specific and already double-checked there (every attribute-loop
comparison traced by TOC slot); this binary's parser simply reads an
attribute HD's does not, even though - per the above - this binary's own
constructor may not do anything with it either. Not chased further:
whether any of this binary's manifests author a `type` other than the
default, and whether `param_3` is read anywhere `Billboard_ConstructResource`
itself doesn't show.

## Not chased this pass

- `+0x21c`'s meaning on the `DAT_01e31910`-shaped `RaceManager` object, and
  which mode selects the `321Go_2048_Combat.vex` gantry.

**Which of `ModeManager_ConstructByMode`'s branches owns this tag is now
traced, and settled as a single-owner finding rather than left open**: `get_xrefs_to`
on the `Billboard.cpp` tag string finds exactly two writers total across the
whole binary - `Billboard_ConstructResource` itself and one site inside
`ModeManager_ConstructByMode` - so the dispatcher writes this tag exactly
once, not once per branch. See
[`weapons.md`](weapons.md#the-mpelimination_modemanager-branch-also-builds-a-billboardcpp-tagged-object):
that one site is pinned to the `MPElimination_ModeManager` branch, confirmed
at the instruction level. What the resulting `0x78`-byte object itself is
remains unnamed.
