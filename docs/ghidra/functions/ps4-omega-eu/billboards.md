# Billboards: `Billboard_ConstructResource`, transferred by tag and magic number

2026-09-15. Functions in `eboot.bin` (WipEout: Omega Collection, PS4,
`CUSA05670`, EU), `x86:LE:64:default`, image base `0x01000000`. Continues the
`.cpp`-tag transfer technique [`README.md`](README.md) and
[`weapons.md`](weapons.md) already used, applied here to
`ps3-hdfury-eu/billboards.md`'s own `Billboard_ConstructResource`.

**The name here is applied**, from [names.tsv](names.tsv).

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

`get_function_callers` finds exactly one caller, `FUN_01391130` (unnamed,
not chased - the likely `TrackStartup_Load`/`Billboard_CreateFromLocation`
equivalent, out of scope for this pass).

**Confidence 87** (same band as the nine weapon managers in `weapons.md`:
tag match plus a second, independently-compiled binary corroborating the
same class - here doubly so, since the corroboration includes two
arbitrary magic constants agreeing exactly, not just the tag and the general
constructor idiom).

## Not chased this pass

- `FUN_01391130`, this function's only caller - the probable
  `TrackStartup.xml`-parsing/dispatch equivalent of HD's
  `Billboard_CreateFromLocation_q`/`_FromColour_q` pair. `TrackStartup.xml`
  exists verbatim on this binary too (`search_strings` confirms), but reading
  the parser itself is a separate pass.
- Whatever `ModeManager_ConstructByMode`'s own `Billboard.cpp`-tagged
  sub-object is - one more of its twelve per-mode branches, not
  distinguished from the others in this pass.
