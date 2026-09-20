# The six-vs-four `GameModeChoice` tiles are not `GameModeChoice` at all - DLC1/DLC2 add hotspots to `FE3DCanvas` instead

Functions in `eboot.elf` (Wipeout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. Found chasing
[2048-frontend.md](../../../formats/2048-frontend.md)'s open question of why the
Vita3K capture showed six tiles on the `GameModeChoice` grid when
`NEWGUI/Definition.xml` declares four. A second, independent Ghidra project
(`/2048/eboot-vita-2048-eu-v104.elf`) was opened alongside the live
`/hdfury/EBOOT-ps3-hdfury-eu.elf` session another lane already had open,
per that page's own note on why this was deferred; both stayed open for the
whole pass and every write below named `program` explicitly.

## The answer: they are not `GameModeChoice` tiles

`HD CAMPAIGN` and `FURY CAMPAIGN` are not injected into `GameModeChoice`'s own
four-`<TouchButton>` grid. They are hotspots added to the persistent 3D
campaign map (`FE3DCanvas`, `2048-frontend.md`'s own "the literal explorable
city campaign map") by two symmetric functions, one per DLC:

- **`FE3DCanvas_AddHDCampaignEventButtons`** (`0x810f817c`), confidence 78
- **`FE3DCanvas_AddFuryCampaignEventButtons`** (`0x810f5000`), confidence 78

Both walk the same node array (`param_1+0xa0`, count `param_1+0xa4` - the
campaign map's own list of event nodes, the runtime shape of the ~50
`<CanvasLabel>` hotspots `2048-frontend.md` already reads out of
`Team_Definition.xml`/the canvas XML) and, for each node whose type index at
`+0x164` falls in this function's tier (`< 8` for HD, `>= 8` for Fury),
project the node's stored position (`+0x15c`/`+0x160`, cached to
`+0x168`/`+0x16c` on first use via a `-999` sentinel) through a per-tier
scale/offset table into a **401x120** local space, bounds-check it, and if it
fits, heap-allocate an 84-byte (`0x54`) button object tagged with the literal
string `"HD campaign event"` / `"Fury campaign event"` at field `+0x2c`
(`piVar6[0xb]`). Both log a count on completion:

```c
SceLibc_9A004680("Added %d HD canvas buttons\n", iVar19);   // 0x810f817c
SceLibc_9A004680("Added %d Fury canvas buttons\n", iVar18); // 0x810f5000
```

Both functions construct the identical button class - same two-phase vtable
assignment (`&PTR_FUN_8106fab4_1_8150ab88` then `&PTR_FUN_8106fab4_1_8150abb8`,
byte-identical in both decompilations) - so the two tiers are the same widget
type filtered to a different node range and given a different tag, not two
different classes. Immediately after this pass, the caller loads
`Data/FE/NewImages/canvasTextureHD.gxt` - the shared texture atlas both
tiers' icons and highlight states draw from.

**That atlas is real and decodes today.** It ships only in
`patch-v104/PSP2/data2.psarc` (not the base package, and not either DLC
archive - `psarc_list` against all five finds it nowhere else), a
2048x2048 PVRTC-II sheet of HD/Fury circuit wordmarks (`WARPED`, `FRENZY`,
`VERTIGO`, `MELTDOWN`, `DROPZONE`, `BLITZED`, `IMPACT`, `VOLTAGE`,
`TURBULENCE`, `VORTEX`, `NUKED`, `CORRUPTION`, `AFTERMATH`) and
bronze/silver/gold medal hexagons, plus the `WIPEOUT HD`/`WIPEOUT HD FURY`
wordmarks - exactly the badge set two DLC-gated campaign-event tiers would
need. Decoded via `cargo run -p oag-texture --example gxt_to_png --
canvasTextureHD.gxt` (`crates/texture`'s existing PVRTC-II support, see
[`docs/formats/2048-frontend.md`](../../../formats/2048-frontend.md#the-front-end-is-a-touch-icon-grid-not-feglobalsmenuskin)
for the base package's sibling atlas, `canvasTexture.gxt`, decoded the same
way).

**This resolves `2048-frontend.md`'s open six-vs-four question outright, not
just the "plausibly" it was written with.** The count discrepancy was never a
`GameModeChoice` fact - `GameModeChoice`'s own four `<TouchButton>`s are the
whole of that screen; `HD CAMPAIGN`/`FURY CAMPAIGN` are the map's own
DLC-conditional decoration, sharing the same touch-grid visual language but a
different object entirely. Neither function has a resolved caller this pass
(see Open, below), so which screen or refresh path invokes them is still
unmeasured - what is measured is what they build and what gates them.

## The gate is DLC-mount detection, not a guess

`2048-frontend.md` called the DLC-mount theory "plausibly." It is now
measured: both tier functions gate on a global byte
(`g_bDlc1Mounted`/`g_bDlc2Mounted`, at `0x8153fc34`/`0x8153fc38`) that has
exactly one writer in the whole binary.

**`Boot_CheckDlcPackageFlags`** (`0x810039fc`), confidence 75 - a boot-time
function whose tail (after unrelated `SceSysmodule`/`SceNet`/`SceScreenShot`
bring-up) checks three literal add-on content IDs via `SceAppUtil_2DB7BE3B`
(a Sce AppUtil "does this add-on content exist" call, matched only by its
argument shape - no NID/name recovered) and latches a flag per hit:

```c
SceLibc_9F87712D(auStack_48, "DLC1W2048PACKAGE", 0x10);
if (SceAppUtil_2DB7BE3B(auStack_48, 0) == 0) { g_bDlc1Mounted = 1; }

SceLibc_9F87712D(auStack_34, "DLC2W2048PACKAGE", 0x10);
if (SceAppUtil_2DB7BE3B(auStack_34, 0) == 0) { g_bDlc2Mounted = 1; }

SceLibc_9F87712D(auStack_20, "W2048DLC3PACKAGE", 0x10);
if (SceAppUtil_2DB7BE3B(auStack_20, 0) == 0) { g_bDlc3Mounted = 1; }
```

`g_bDlc1Mounted`/`g_bDlc2Mounted` are each read at dozens of sites across the
front end (front-end screen construction, HUD/canvas code); `g_bDlc3Mounted`
(`0x8153fc3c`, `W2048DLC3PACKAGE` - a third add-on this project has not
otherwise named) is set the same way but this pass did not trace any reader
for it. Confidence 75, not higher: decompilation and the literal ID strings
are unambiguous, but nothing here is a runtime trace, and `SceAppUtil_2DB7BE3B`
itself is identified by call shape rather than a resolved import name.

## 2026-09-20: the vtable's owner is `TouchCampaignFury_Item`/`TouchCampaignHD_Item`, not `FE3DCanvas`

**Confidence 82.** The vtable-shaped table the previous pass found by byte
pattern (`0x8150ea74`/`0x8150eae0`, 27 slots apart) is actually **two** back
to back 27-slot vtables, starting at `0x8150ea48` (containing the
`0x810f5001` Thumb pointer at slot 11, `+0x2c` = `0x8150ea74`) and
`0x8150eab4` (containing `0x810f817d` at slot 11, `0x8150eae0`) - the
"27 slots apart" reading was two mid-table hits, not one table's span. Of
the 15 slots read for both tables (relative slots 0-14 - slots 15-26 of the
HD table were not captured this pass, so the full divergence picture is
incomplete), most are byte-identical (inherited, unmodified), and seven
diverge: relative slots 3, 5, 7, 9, 11 and **10, 14** - the shape of two
sibling classes each overriding a handful of virtual methods of a shared
base, not one class's multiple-inheritance thunk pair (which would put the
*same* two function pointers in both tables). Slots 10 and 14 are the
payoff: they hold `TouchCampaignFury_Item_Construct`/`_Create`
(`0x813cf5e5`/`0x813cf605`, Thumb bit set) in the Fury table and
`TouchCampaignHD_Item_Construct`/`_Create` (`0x813cf749`/`0x813cf769`) in
the HD table - **the vtable's own contents name its owner's constructors**,
independent corroboration of the class-name finding below from a second
direction, not just "xrefs happened to point here." Slots 3, 5, 7 and 9 are
further per-class overrides, not identified this pass. Both tables' own
"offset-to-top"/typeinfo words (the two words immediately before each table
start) are zero - this binary has no RTTI (`-fno-rtti`), so the
"typeinfo string" approach the previous pass proposed finds nothing; the
class name had to come from the constructor instead.

`get_xrefs_to` on the table *start* addresses (not the slot-11 mid-table
addresses the byte search found) turns up real constructors:

- `0x8150ea48` (Fury): `TouchCampaignFury_Item_Construct` (`0x813cf5e4`,
  was `FUN_813cf5e4`) and `TouchCampaignFury_Item_Create` (`0x813cf604`,
  was `FUN_813cf604`).
- `0x8150eab4` (HD): `TouchCampaignHD_Item_Construct` (`0x813cf748`) and
  `TouchCampaignHD_Item_Create` (`0x813cf768`), byte-identical in shape to
  their Fury counterparts.

`TouchCampaignFury_Item_Create` writes the literal string
`"Frontend/Items/TouchCampaignFury_Item.cpp"` into the new object's field
`[0xb]` (a debug/leak-tracking "creation site" tag, the same convention as
the button object's own `"HD campaign event"`/`"Fury campaign event"` tag at
a different offset); `TouchCampaignHD_Item_Create` writes
`"Frontend/Items/TouchCampaignHD_Item.cpp"` into the identical field. These
are the original developers' own source paths, not an inferred name - as
strong as evidence gets short of a shipped symbol table. The object is
0xec (236) bytes, base class `LinkObj` (`Memory_Alloc(0xec, "Unknown
LinkObj", 0x1ef, 0)`, matching the `System\LinkObj.h` string found near the
vtable data). Capped at 82 rather than higher because this is decompilation
evidence, not a runtime trace or a second binary - see the confidence
rubric's ceiling table.

**Still open: no caller found for either constructor.** `get_xrefs_to` on
`0x813cf604`/`0x813cf768`/`0x813cf5e4`/`0x813cf748` all return nothing,
same as the tier functions before them. Given the vtable pointer is stored
by direct assignment rather than a virtual call and nothing statically
calls the constructor, the likeliest explanation is a C++ global/static
object constructor invoked from the PRX's `.init_array` (a data table the
loader walks, not a `BL` site) rather than dead code - consistent
with, not proven by, this pass. **The screen/refresh path that dispatches
the two tier functions is therefore still unnamed** - naming the
`.init_array` entry (or confirming this reading some other way) is the
concrete next step, not a class name anymore.

**Bonus, while chasing this: the two "tag" constants below are resolved
(see the dated section after Open) and turned out to name the actual
tap-handler for each button**, which itself forwards into what reads as a
race-launch sequence (`FUN_810f7060`/`FUN_810f3fa4`, not renamed this pass -
reads the tapped node's Class/Track/Weapons/Opponents/Damage/SkillLevel/
Tournament fields and ends with a call shaped like "start this race").

## 2026-09-20: the per-tier tables, the tag constants, the NID, and DLC3

**The per-tier scale/offset tables (confidence 84 - exact, reconstructed
from the initializer's own immediate operands).** Both tables are `.bss`:
zero in the static image, populated at runtime by inline initializer code
immediately adjacent to each tier's `TouchCampaign*_Item` construction
(`~0x810f6e80`-`0x810f7044` for Fury, `~0x810fa0f0`-`0x810fa1b8` for HD) -
there is no live emulator in this pass, so the values below come from
reading the `mov`/`movw`/`movt` immediates the initializer stores, not from
a memory dump (a dump of the static image at either address is all zero,
confirmed). Both tiers share the *same* scale/bias quad:

| Field | HD (`0x818808d4`) | Fury (`0x81880660`) |
| --- | --- | --- |
| X scale | 3.0 | 3.0 |
| Y scale | 2.0 | 2.0 |
| X bias | 55.0 | 55.0 |
| Y bias | 25.0 | 25.0 |

Per-type (X,Y) pixel-offset table, stride 8 bytes (two `i32`), immediately
after each scale/bias quad - HD's own table (`0x818808e4`, node type field
0-7, matching the `iVar8 < 8` gate) reads:

| Type | X | Y |
| --- | --- | --- |
| 0 | 1 | 1 |
| 1 | 20 | 6 |
| 2 | 44 | 4 |
| 3 | 68 | 6 |
| 4 | 64 | 16 |
| 5 | 37 | 17 |
| 6 | 6 | 18 |
| 7 | 37 | 27 |

Fury's own layout mirrors HD's exactly - a scale/bias quad at `0x81880660`
followed by its own 8-entry table at `0x81880670` (`+0x10`, same as HD) -
but the *source* indexes it by the raw node-type field (8-15, matching the
`7 < iVar8` gate) rather than `type - 8`, so the compiler folded that `-8`
into the base pointer instead of emitting a subtraction: `&DAT_81880630` is
simply `0x81880670 - 8*8`, a computed address 64 bytes before the real
table, not a second, wider array with unused low slots (and it does not
alias HD's table - HD's is a distant, unrelated block at `0x818808e4`). The
table itself (node type field 8-15) reads:

| Type | X | Y |
| --- | --- | --- |
| 8 | 1 | 1 |
| 9 | 21 | 3 |
| 10 | 42 | 0 |
| 11 | 61 | 5 |
| 12 | 75 | 11 |
| 13 | 52 | 16 |
| 14 | 28 | 16 |
| 15 | 5 | 17 |

16 total node types across both tiers (8 each). The type field selects which
(X,Y) offset gets added to the node's own projected position before the
401x120/`< 401.0`/`< 120.0` bounds check both tier functions already share -
i.e. it is a per-event-badge layout slot inside that bounded area, not a
name or a category. Reconstructed by hand-tracing register-held immediates
across two Thumb disassembly windows rather than a clean decompile (Ghidra
has not created a `Function` over either init block, so `decompile_function`
was not available for it). Both tiers' identical scale/bias values
cross-validate each other, and both blocks stop writing at `+0x3c` (bounding
each table at 8 entries independent of any assumption); still capped at 84
rather than 95 because the method itself (manual register-liveness tracing)
is weaker than a decompile or a runtime trace even when, as here, it holds
up. `create_function` at each init block's entry followed by
`decompile_function` would be the clean confirmation - not done this pass.

**The "tag" constants are not a hash - they are Thumb function pointers
(confidence 90).** `-0x7ef08b35` as `u32` is `0x810f74cb`; `-0x7ef0bbf1` is
`0x810f440f`. Both are exactly the Thumb-bit-set entry addresses of two real
functions already in this binary: `0x810f74ca` and `0x810f440e`. Tested
against this project's own documented name-hash style first
(`wad::hash_name`, `~crc32(name)`, lower-cased/`\`->`/`-normalized) against
36 candidate strings - the button tags, both source paths, both DLC content
IDs, both atlas filenames, `FE3DCanvas`, `TouchCampaign`, `Launch 2048`, and
assorted casing/spacing variants of "HD/Fury campaign event" - **no
candidate matches either constant.** The address reading fits far better and
is exact, not a guess: renamed `0x810f74ca` to `TouchCampaignHD_Item_OnTap`
and `0x810f440e` to `TouchCampaignFury_Item_OnTap`. Both check
`param_1+0x3c == 2` (a stored event-type field, "tap"/"select" hypothesised
but not confirmed) and `param_1+0x14 != 0`, then forward to `FUN_810f7060`/
`FUN_810f3fa4` respectively - both of which read the tapped node's own
race-setup fields (string tags "Class", "Track", "Weapons", "Opponents",
"Damage", "SkillLevel", a `"Tournament"`/`"yourTourney"` branch) and end in
a call shaped like "start this race or tournament." Capped at 90 rather than
the 95+ "runtime trace" band because the address match, while exact
arithmetic, is still read from static decompilation - not a breakpoint. Not
renamed this pass: `FUN_810f7060`/`FUN_810f3fa4` themselves (the race-launch
handlers), left as a concrete next step with the evidence above.

**`SceAppUtil_2DB7BE3B` is `sceAppUtilDrmOpen` (confidence 90).** Looked up
against the vitasdk/vita-headers NID database
(`db/360/SceAppUtil.yml` at
`github.com/vitasdk/vita-headers`, firmware 3.60 - the only firmware
directory in that repo carrying `SceAppUtil.yml`): NID `0x2DB7BE3B` is
`sceAppUtilDrmOpen`. Its siblings in the same function
(`Boot_CheckDlcPackageFlags`) resolve too and corroborate each other: NID
`0xDAFFE671` (called once, first, before any content-ID check) is
`sceAppUtilInit`, and NID `0x6A140498` (called right after the first
`sceAppUtilDrmOpen`, on the `"W2048NETWORKPASS"` check only) is
`sceAppUtilDrmClose` - exactly the init-once / open+close-per-ticket shape
the names imply. All three renamed in Ghidra at their own thunk-stub addresses
(`SceAppUtil_2DB7BE3B` at `0x813f2b90`->`sceAppUtilDrmOpen`,
`SceAppUtil_6A140498` at `0x813f2bc0`->`sceAppUtilDrmClose`,
`SceAppUtil_DAFFE671` at `0x813f2c40`->`sceAppUtilInit`) per this project's own
`names.tsv` convention that a NID-resolved import keeps its real API name
rather than the `Subsystem_VerbNoun` scheme (see `ps4-omega-eu/names.tsv`'s
header comment for the precedent). Not capped by the decompilation-only
ceiling: this is agreement with an independently-authored, external
database across three separate identifiers, each fitting its call-site role
exactly.

**`W2048DLC3PACKAGE`/`g_bDlc3Mounted` *does* have a reader (confidence 85) -
the earlier "no known reader" claim was wrong.** `get_xrefs_to` on
`g_bDlc3Mounted` (`0x8153fc3c`) finds two reads inside `FUN_810a19e0`, an
unnamed ~0x148-field object constructor (references `"ONL_CRJ_GLIST"` and
`data/fe/newimages/callout/cross.gxt` - not identified further this pass).
It combines all three DLC flags into one new global:
`g_bNoDlcMounted` (`0x8151d504`, renamed this pass) =
`!g_bDlc1Mounted && !g_bDlc2Mounted && !g_bDlc3Mounted` - i.e. a single "no
add-on content mounted at all" flag, plausibly gating a placeholder/upsell
shown on whatever screen `FUN_810a19e0` builds (one caller,
`FUN_813c8d14`, not chased). `FUN_810a19e0` itself was not renamed - not
enough evidence yet for what screen it is.

## Open

- **The screen/refresh path that dispatches the two tier functions is still
  unnamed.** Neither `TouchCampaignFury_Item_Construct`/`_Create` nor their
  HD siblings have a resolved static caller; the likeliest explanation is a
  `.init_array`-driven global/static-object constructor (see the dated
  section above). Confirming that reading, or finding the real caller some
  other way, is the concrete next step - it is also probably the way to
  find what the DLC1/DLC2 gate actually decides at a screen level, not just
  a function level.
- **`FUN_810f7060`/`FUN_810f3fa4` (the race-launch handlers the two on-tap
  callbacks forward to) are not renamed.** They read a tapped campaign-map
  node's own race-setup fields (Class/Track/Weapons/Opponents/Damage/
  SkillLevel/Tournament) and end in what looks like "start this race" -
  worth a dedicated pass, and likely relevant to whatever eventually reads
  `Team_Definition.xml`-style per-event race config for this screen.
- **`FUN_810a19e0` (the `g_bNoDlcMounted` reader) is unnamed.** One caller,
  `FUN_813c8d14`, not chased. The `"ONL_CRJ_GLIST"` string and
  `callout/cross.gxt` texture suggest an online/leaderboard-adjacent screen
  with a DLC-upsell callout, not confirmed.

## See also

- [2048-frontend](../../../formats/2048-frontend.md) - the page this corrects
- [2048-hud](../../../formats/2048-hud.md) - `DemoRaceManager_Construct`, found
  the same way (decompiled, string-corroborated, no runtime trace)
- [game-boot](game-boot.md) - `Game_Main`'s own boot-mode selector, the other
  function in this binary keyed off a literal string table the same way
