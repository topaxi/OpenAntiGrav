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

## Open

- **No caller found for either tier function.** `get_function_callers` and
  `get_xrefs_to` on both addresses return nothing - Thumb functions are
  usually virtual-dispatched here. A byte-pattern search for the Thumb
  pointer (`7d 81 0f 81` little-endian for `0x810f817d`, `01 50 0f 81` for
  `0x810f5001`) lands both in the same vtable-shaped table, 0x6c bytes (27
  slots) apart, at `0x8150eae0` and `0x8150ea74` - consistent with two
  methods of one class, not two unrelated tables, but the class itself is
  unnamed. Naming that vtable's owner (most plausibly `FE3DCanvas` itself)
  is the concrete next step.
- **The per-tier scale/offset constants are unread.** `FE3DCanvas_AddHDCampaignEventButtons`
  indexes a flat table (`DAT_818808d4`.."e8`); `FE3DCanvas_AddFuryCampaignEventButtons`
  indexes an 8-byte-stride array by the node's own type field
  (`&DAT_81880630 + iVar8*8`). Neither table's contents were dumped this
  pass - doing so would likely name how many distinct node "types" the map
  authors beyond the HD/Fury split.
- **The per-tier tag constants (`piVar13[2] = -0x7ef08b35` / `-0x7ef0bbf1`)
  are unexplained.** They read like a hash (the same style `TouchCampaign`'s
  own redirect-target hash uses, per `2048-frontend.md`), not chased this
  pass.
- **`W2048DLC3PACKAGE`/`g_bDlc3Mounted` has no known reader.** Worth a search
  independent of the front end - it may gate content this project has not
  identified as DLC at all yet.
- **`SceAppUtil_2DB7BE3B` is named by call shape only.** A NID lookup against
  a Vita SDK header list would give it (and its sibling `SceAppUtil_*` calls
  in the same function) a real name.

## See also

- [2048-frontend](../../../formats/2048-frontend.md) - the page this corrects
- [2048-hud](../../../formats/2048-hud.md) - `DemoRaceManager_Construct`, found
  the same way (decompiled, string-corroborated, no runtime trace)
- [game-boot](game-boot.md) - `Game_Main`'s own boot-mode selector, the other
  function in this binary keyed off a literal string table the same way
