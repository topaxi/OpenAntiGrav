# The 1.07 patch eboot: base -> patch address translation

Every address in this directory, and in `names.tsv`, is in the **base** package's `eboot.bin`
(v1.00, Ghidra program `/omega/eboot-ps4-omega-eu.bin`). The game that runs on the 1.07 patch boots
the **patch** package's `eboot.bin`, a different build: code and data moved by amounts that differ
from region to region, so a base address read against the running game lands on unrelated bytes.
This page gives the patch address for each named function and for the data anchors that live
readouts need, with the evidence for each. Lane `omega-patch-ghidra`, 2026-11-01.

**Confidence that the docs are base-only and the patch differs: 98.** Measured, not inferred, below.

## The two executables

Both are fake SELFs (magic `4F 15 3D 1D`, 10 segment records, none encrypted) with the ELF header at
file offset `0x160`. Ghidra address = `0x01000000` + ELF vaddr on both (checked: image base `01000000`
in both programs, and the predicted string below).

| | Base (v1.00) | Patch (1.07) |
| --- | --- | --- |
| Path | `data/extracted/ps4/omega-eu/uroot/eboot.bin` | `data/extracted/ps4/omega-eu-patch/uroot/eboot.bin` |
| Size | 10,493,719 | 11,004,033 |
| sha256 | `24a4a941...cc083e3c448` | `3a52e910...cc951c326a362bcc4775f29e6ff7f644c` |
| Text (PT_LOAD 0, r-x) | vaddr `0x0`, `0x8fcc04` bytes | vaddr `0x0`, `0x96651c` bytes (+`0x6991c`) |
| Relro (`0x61000010`) | vaddr `0x900000`, file `0x30048`, mem `0x34000` | vaddr `0x968000`, file `0x31c60`, mem `0x34000` |
| Data + bss (PT_LOAD 2, rw-) | vaddr `0x934000`, file `0x3c1c8`, mem `0x8007cf` | vaddr `0x99c000`, file `0x47b28`, mem `0x87fb6f` |
| Entry | `0xa0` | `0x80` |

Static check of the docs' addresses, bytes read at the same Ghidra address in each file:

| Address | Doc says | Base | Patch |
| --- | --- | --- | --- |
| `0x01813574` | `Data\XML\Zone_HUD.xml` ([race-hud-selection](race-hud-selection.md)) | that string | zeros |
| `0x0163b920` | `Game_Main` prologue | `55 48 89 e5 41 57 41 56` | `c5 fc 11 05 ...` (mid-instruction) |
| `0x017359c0`, `0x013c28f0`, `0x012e38d0` | function entries | the same prologue | mid-instruction bytes |

In the patch the string sits at `0x0187567c` (`+0x62108`), and Ghidra's patch program reads
`Data\XML\Zone_HUD.xml` there. The "text delta `0xf9def8`" and "seg1 delta `0xf99590`" a live probe
derived (ghidra minus patch vaddr) are these shifts seen from the base side: `0x01000000 - 0x62108`
and `0x01000000 - 0x66a70`. They hold only near the anchors they were derived from; text moves by
`0xcea0` near `0x01240000` and by `0x5e6cc` near `0x01800000`.

## The patch program in Ghidra

Imported headless with GhidraOrbis' `GhidraOrbisSelfLoader`, `x86:LE:64:default:gcc`, full
auto-analysis (about 10 minutes), as `/omega/eboot-ps4-omega-eu-v107.bin`, the counterpart of
2048's `-v104` naming. The import went into a separate headless project because the GUI held the
main project's lock; to import it into the main project with the GUI closed:

```sh
cp data/extracted/ps4/omega-eu-patch/uroot/eboot.bin /some/empty/dir/eboot-ps4-omega-eu-v107.bin
/opt/ghidra/support/analyzeHeadless "$PWD" OpenAntiGrav/omega \
  -import /some/empty/dir/eboot-ps4-omega-eu-v107.bin -loader GhidraOrbisSelfLoader \
  -analysisTimeoutPerFile 7200
```

Its executable sha256 reads back as `3a52e910...f644c`. No names are applied to it: `names.tsv`
replays into the base program (`scripts/apply-ghidra-names.py` maps `ps4-omega-eu` to
`/omega/eboot-ps4-omega-eu.bin`), so the patch addresses below live on this page only.

## How each address was found

1. **String votes.** For a base function, every C string it loads RIP-relatively, kept only when no
   other base function loads it. The same string in the patch's rodata, the patch function that
   loads it (function starts are 16-aligned `push rbp; mov rbp, rsp`), majority vote.
2. **Interpolation.** Link order is kept, so a function without a usable string is looked for within
   `0x8000` of the position interpolated between its two nearest string-anchored neighbours, among
   functions of 0.6x to 1.6x its size, scored by the similarity of their first 60 mnemonics; taken
   only at 0.85 or better and at least 0.05 ahead of the next.
3. **Mnemonic similarity** is reported for every row: the first 60 instructions' mnemonics of base and
   patch (difflib ratio). A string-voted row with low similarity (below 0.6) is most likely the same
   function rebuilt differently (inlining or a split), but it is scored down.
4. **Data.** Each translated function pair's instruction streams are aligned (mnemonic plus operand
   shape, RIP displacements and constants masked); aligned RIP-relative operands give a
   (base target, patch target) pair, 6,810 in all. A data anchor's patch address is the vote over
   every aligned reference to it; structs are checked field by field.

All 87 patch function addresses were confirmed as function entries by Ghidra's own analysis of the
patch program (`getFunctionAt` returns a function at each).

Confidence for a row is that the patch function **is** the base function, not the name's own
confidence (that stays on its page). The function rows are static evidence only, so the rubric's
"decompilation only, consistent call sites" ceiling of 84 binds
([rubric](../../../reverse-engineering/confidence-rubric.md)); none of them was hit live. Bands:
80 for string votes from two or more strings with similarity 0.85+, 75 for one string, 70 for
similarity 0.6 to 0.85 (all "probable"), 55 below 0.6 ("plausible"); 70 for an interpolated match
at 0.95+ with a 0.3 margin, 60 at 0.85+ with a 0.2 margin, 50 otherwise. Names keep their page's
confidence on the base program; use the lower of the two on the patch. What would raise a row: a
breakpoint or a live read of state the function owns, on the patch.

## Data anchors, checked live

The live check: a native PC port of the PS4 executable (`wipeout_pc`, commit `e56904d`), which
runs the **patch** eboot natively at image base `0x800000000`, read with `/proc/<pid>/mem` at the
Vineta K grid in a Race Box single race (the walk picks the track and the first Feisar craft and
does not touch the class; 8 racers on the grid, one human). One dump from one boot (run 3 of 3).
The port applies its own 212 code writes to the text (none to data). Its dump of the relro segment
(`0x34000` bytes from vaddr `0x968000`) and the data segment (`0x87fb6f` bytes from `0x99c000`)
matches the patch file's static bytes on 87.8 % and 95.6 % of the file-backed part (the rest is
relocated pointers, `0x8_00xx_xxxx`, and written globals), so **runtime = `0x800000000` +
(patch Ghidra address - `0x01000000`)** on every segment.

Which live reads discriminate: the relocated pointer tables (23 mode names, the five class names,
the language rows), `Feisar` at `+0x28` of the state, and the state's byte layout (`0` at `+0x10`,
`1` at `+0x11`). A zero (class 0, level 0, a null object) agrees with the translation and cannot
confirm it; those rows are scored on their static evidence. Rubric: a runtime read caps at 94 until a
second binary agrees; one dump keeps these below that.

Raw bytes read live at the grid:

```text
0x02012800: 00000000 ffffffff 00000000 03000000 00010000 ff07ff07 ff07ff07 030e1e00
0x02012820: 601f9302 10000000 46656973 61720000          ("Feisar" at +0x28)
0x021dbdb8: 40d74302 10000000                             (qword 0x10_0243_d740, a heap pointer)
0x0198f890: 1f068900 08000000 25068900 08000000 ...      (0x8_0089_061f = base + "Debug")
0x01a7ae00: c0163e0a 10000000 a04d620a 10000000           (racer 0; rows 1-7 alike, row 8 not)
0x01eb4820: 00000040 00000000 00002040                    (2.0, 0.0, 2.5)
0x020e89d8: 01000000
```

| Base | Name | Patch | Static evidence | Live at the grid | Confidence |
| --- | --- | --- | --- | --- | ---: |
| `0x01f999d8` | game state: class ([ai-controls](ai-controls.md)) | `0x02012800` | 12 aligned loads in 10 functions agree; 1 (in `AIManager_Update`) lands on `0x02012808`, an alignment slip | `0` (Venom): agrees, does not discriminate | 86 |
| `0x01f999e0` | level | `0x02012808` | 3 aligned loads in 2 functions | `0`: does not discriminate | 80 |
| `0x01f999e4` | mode id | `0x0201280c` | 107 aligned loads in 31 functions | `3`, `Arcade` in the live mode-name table: a Race Box single race | 90 |
| `0x01f999e8` | human count | **`0x02012811`** (a byte) | 4 aligned byte loads in 4 functions | `1`, with `0` at `+0x10` | 88 |
| (none) | team name | `0x02012828` | not traced; read by eye | `Feisar` | 60 |
| `0x01f99990` | campaign cell | `0x020127b8` | 4 aligned loads in 3 functions | `0` (no cell in Race Box): does not discriminate | 80 |
| `0x01e31910` | current game-mode object | `0x01ea7bc0` | 41 aligned loads in 19 functions | `0` (no event object): does not discriminate | 84 |
| `0x01a07390` | racer table, stride `0x9b0` | `0x01a7ae00` | 2 aligned loads in 2 functions | rows 0-7 start with two heap pointers each, row 8 does not (8 racers) | 86 |
| `0x01926700` | `g_GameModeNames` | `0x0198f890` | 6 aligned loads in 5 functions | 23 relocated pointers to `Debug` ... `Multiplayer Ranked` | 92 |
| `0x01926620` | class-name table | `0x0198f7b0` | 3 aligned loads in 2 functions | `Venom`, `Flash`, `Rapier`, `Phantom`, `SuperPhantom`, null | 90 |
| `0x0212a7d8` | `ToneMap_Instance` ([tonemap](tonemap.md)) | `0x021dbdb8` | 2 aligned refs in `PostTarget_Create`, and the same reference pattern in 4 more function pairs (below) | a heap pointer (`0x10_0243_d740`): a pointer, not the object; fields unread | 84 |
| `0x01e3de80` | `EnvSettings_NovaPrelitScaleBiasPower` ([lightmap-prelit](lightmap-prelit.md)) | `0x01eb4820` | 3 aligned refs in 3 functions | `2.0, 0.0, 2.5`, a triple six of the patch's `.EnvSettings` files carry (not the static default `1.4, 0.2, 1.5`); Vineta K's own file not read | 82 |
| `0x01947160` | `Language_PluginTable` ([language](language.md)) | `0x019b89a0` | same rows (`0x12, 0x12`, name, `0xd`, ...); the patch function compares the same fields unrolled | relocated name pointers in the first rows | 86 |
| `0x02037c98` | `Language_RegionMask` ([language](language.md)) | `0x020e89d8` | 2 aligned refs in `Game_Main` | `1`, the EU value the page states | 84 |
| `0x0193f750` | `g_hud_pickup_widget_table` ([omega-pickup-table](omega-pickup-table.md)) | `0x0197b970` | 1 aligned ref; all 14 rows' names and words equal the base's | rows 0-3 read live: relocated name pointers, the same words | 88 |

Findings the table carries:

- **The game-state struct changed in 1.07.** Class, level and mode keep their offsets (`+0`, `+8`,
  `+0xc`), but the base's human count at `+0x10` is a byte at `+0x11` in the patch, and the
  `+0x12..+0x20` fields `WeaponManager_DispatchFire` reads shifted by 1 to 2 bytes. Read the patch
  struct field by field, not by the base layout.
- **The pickup widget table's rows shrank from `0x20` to `0x10` bytes**: name pointer, icon word,
  backdrop word. The 14 names and the 28 words are unchanged
  ([omega-pickup-table](omega-pickup-table.md)'s `0x80BE1E69`, `0x801CBC1C`, `0x802B1CBC` rows);
  the widget slot and the two `1.0f` are no longer in the row.
- **`ToneMap_Instance` is a pointer** to a heap object (`0x210` bytes, [tonemap](tonemap.md)); its
  fields need a read through the pointer, not at the global. Its references pair one to one:
  `PostTarget_Create` `0x0178a160` -> `0x017da2d0` (load at `+0x73`, store), the four-slot teardown
  `0x01789f40` -> `0x017d7dc0`, `ToneMap_ApplyEnvSettings` `0x01620980` -> `0x0164bc80` (two loads,
  then the same `+0x98`, `+0x80`, `+0x84`, `+0x88`, `+0xa0..+0xbc` writes), `PostTarget_Resolve`
  `0x017864a0` -> `0x017d4560` (three loads), and the brightness pair `0x0155e450`/`0x01561f20` ->
  `0x01581e70`/`0x01582840`. The patch adds one reference the base does not have, in `0x01742e90`.
- A second relro table at `0x0198d090` reads `Venom`, `Flash`, `Rapier`, `Phantom`,
  `Speed_Class_C_0`, `Speed_Class_B_0`: it is **not** the docs' class-name table; that one is
  `0x0198f7b0`.

## Function translation (names.tsv rows)

117 function rows: **83 translated**, 34 unresolved. By method: 45 string votes, 37 interpolated, 1 by hand.
Unresolved rows are mostly small leaf functions (the five `BlendState_*` setters) and functions with
no strings whose neighbourhood has several look-alikes; they need Ghidra's version tracking or a
reading by hand.

| Base | Name | Page | 1.07 patch | Method | Mnemonic similarity | Confidence |
| --- | --- | --- | --- | --- | --- | --- |
| `0x012e38d0` | `MagstripWake_Construct` | [ships-effects](ships-effects.md) | `0x012fc910` | string xrefs 5/5 | 0.95 | 80 |
| `0x01723ae0` | `ShipCollisionFx_Trigger` | [ship-collision-fx](ship-collision-fx.md) | `0x0176fb90` | string xrefs 3/3 | 0.92 | 80 |
| `0x012f7500` | `Ship_DispatchCollisionFx` | [ship-collision-fx](ship-collision-fx.md) | `0x0132a3b0` | interpolated + mnemonics 1.00 (next 0.53) | 1.00 | 70 |
| `0x0163b920` | `Game_Main` | [game-boot](game-boot.md) | `0x0167a020` | string xrefs 89/89 | 0.95 | 80 |
| `0x017359c0` | `SoundManager_Construct` | [game-boot](game-boot.md) | `0x01784120` | interpolated + mnemonics 0.98 (next 0.47) | 0.98 | 70 |
| `0x013c28f0` | `FrontendRoot_Construct` | [game-boot](game-boot.md) | `0x013f3ac0` | string xrefs 34/34 | 0.65 | 70 |
| `0x01366f70` | `QuakeManager_Construct` | [weapons](weapons.md) | `0x013946b0` | interpolated + mnemonics 0.88 (next 0.53) | 0.88 | 60 |
| `0x01368610` | `Rocket_Construct` | [weapons](weapons.md) | `0x01396150` | string xrefs 2/2 | 0.98 | 80 |
| `0x0136cd70` | `RocketManager_Construct` | [weapons](weapons.md) | `0x0139b220` | interpolated + mnemonics 0.90 (next 0.60) | 0.90 | 60 |
| `0x013574d0` | `MissileManager_Construct` | [weapons](weapons.md) | `0x0137ed90` | string xrefs 2/2 | 0.65 | 70 |
| `0x0133df00` | `EMPManager_Construct` | [weapons](weapons.md) | `0x013684e0` | string xrefs 1/1 | 0.32 | 55 |
| `0x013275a0` | `BombManager_Construct` | [weapons](weapons.md) | - | unresolved | - | - |
| `0x01333f90` | `CannonManager_Construct` | [weapons](weapons.md) | `0x0135a740` | string xrefs 12/12 | 0.92 | 80 |
| `0x0134aae0` | `LightBarrierManager_Construct` | [weapons](weapons.md) | `0x01372460` | string xrefs 4/4 | 0.50 | 55 |
| `0x01361b00` | `PlasmaManager_Construct` | [weapons](weapons.md) | `0x013a1200` | string xrefs 6/8 | 0.60 | 70 |
| `0x0128e550` | `RaceManager_Construct` | [weapons](weapons.md) | `0x012a16a0` | string xrefs 8/8 | 0.88 | 80 |
| `0x0124fe10` | `ModeManager_ConstructByMode` | [weapons](weapons.md) | `0x0125cb40` | string xrefs 1/1 | 0.53 | 55 |
| `0x01284690` | `MPRaceManager_Construct` | [weapons](weapons.md) | `0x01296440` | interpolated + mnemonics 1.00 (next 0.50) | 1.00 | 70 |
| `0x0124f040` | `AIBatchRaceManager_Construct` | [weapons](weapons.md) | `0x0125bf20` | string xrefs 2/2 | 0.95 | 80 |
| `0x0125ba00` | `DemoRaceManager_Construct` | [weapons](weapons.md) | `0x012683f0` | interpolated + mnemonics 0.95 (next 0.43) | 0.95 | 70 |
| `0x01279ed0` | `MPArcadeRaceManager_Construct` | [weapons](weapons.md) | `0x0128b3d0` | interpolated + mnemonics 0.98 (next 0.73) | 0.98 | 60 |
| `0x0127ad50` | `MPEliminationRaceManager_Construct` | [weapons](weapons.md) | - | unresolved | - | - |
| `0x0127d940` | `MPNitroRaceManager_Construct` | [weapons](weapons.md) | `0x0128f1f0` | interpolated + mnemonics 1.00 (next 0.47) | 1.00 | 70 |
| `0x0127ecd0` | `MPTimeTrialRaceManager_Construct` | [weapons](weapons.md) | `0x012906e0` | interpolated + mnemonics 0.97 (next 0.78) | 0.97 | 50 |
| `0x01280490` | `MPTournamentRaceManager_Construct` | [weapons](weapons.md) | `0x01291f60` | interpolated + mnemonics 0.90 (next 0.45) | 0.90 | 60 |
| `0x012a0b90` | `SPArcadeRaceManager_Construct` | [weapons](weapons.md) | `0x012b6840` | interpolated + mnemonics 0.95 (next 0.45) | 0.95 | 70 |
| `0x012a4610` | `SPDetonatorRaceManager_Construct` | [weapons](weapons.md) | `0x012ba160` | string xrefs 30/30 | 0.98 | 80 |
| `0x012a7e60` | `SPEliminationRaceManager_Construct` | [weapons](weapons.md) | - | unresolved | - | - |
| `0x012a98f0` | `SPFreePlayRaceManager_Construct` | [weapons](weapons.md) | `0x012bf820` | interpolated + mnemonics 0.95 (next 0.40) | 0.95 | 70 |
| `0x012ad580` | `SPNitroRaceManager_Construct` | [weapons](weapons.md) | `0x012c3570` | string xrefs 4/4 | 0.97 | 80 |
| `0x012af230` | `SPTimeTrialRaceManager_Construct` | [weapons](weapons.md) | `0x012c4f50` | string xrefs 5/5 | 0.88 | 80 |
| `0x012b1e60` | `SPTournamentRaceManager_Construct` | [weapons](weapons.md) | - | unresolved | - | - |
| `0x012b4960` | `SPZoneRaceManager_Construct` | [weapons](weapons.md) | `0x012ca3f0` | string xrefs 3/3 | 0.72 | 70 |
| `0x015a7020` | `GameModeRaceManager_Construct` | [weapons](weapons.md) | `0x015d18b0` | string xrefs 3/3 | 0.97 | 80 |
| `0x0129c120` | `RaceManager_ConstructArcadeHud` | [weapons](weapons.md) | `0x012b1d30` | string xrefs 8/8 | 0.63 | 70 |
| `0x012ac060` | `NitroRaceManager_ConstructTuning` | [weapons](weapons.md) | `0x012c2090` | string xrefs 31/31 | 0.95 | 80 |
| `0x016e53a0` | `Billboard_ConstructResource` | [billboards](billboards.md) | `0x0172d6f0` | string xrefs 1/1 | 0.93 | 75 |
| `0x01391130` | `TrackStartup_Load` | [billboards](billboards.md) | `0x013c23c0` | string xrefs 16/16 | 0.90 | 80 |
| `0x0135e140` | `PlasmaManager_Update` | [plasma](plasma.md) | `0x0138c640` | string xrefs 2/3 | 0.88 | 80 |
| `0x0135cad0` | `Plasma_Init` | [plasma](plasma.md) | `0x0138ab50` | string xrefs 2/2 | 0.90 | 80 |
| `0x0135d5d0` | `Plasma_Launch` | [plasma](plasma.md) | `0x0138b7e0` | string xrefs 1/1 | 0.97 | 75 |
| `0x0135bd70` | `Plasma_UpdateVisual` | [plasma](plasma.md) | `0x0138a0d0` | interpolated + mnemonics 0.97 (next 0.58) | 0.97 | 70 |
| `0x0135dff0` | `Plasma_Teardown` | [plasma](plasma.md) | `0x0138c410` | string xrefs 1/1 | 0.77 | 70 |
| `0x01360970` | `PlasmaManager_Destruct` | [plasma](plasma.md) | - | unresolved | - | - |
| `0x01371990` | `WeaponExplosions_Update` | [plasma](plasma.md) | `0x013a0390` | interpolated + mnemonics 1.00 (next 0.73) | 1.00 | 60 |
| `0x013719e0` | `WeaponExplosions_Collapse` | [plasma](plasma.md) | `0x013a0ad0` | string xrefs 1/1 | 0.97 | 75 |
| `0x01371e20` | `WeaponExplosions_Reset` | [plasma](plasma.md) | - | unresolved | - | - |
| `0x01371f10` | `WeaponExplosions_Draw` | [plasma](plasma.md) | `0x013a03e0` | interpolated + mnemonics 1.00 (next 0.40) | 1.00 | 70 |
| `0x01656130` | `WeaponStats_ParseXml` | [plasma](plasma.md) | `0x0169c610` | string xrefs 56/56 | 1.00 | 80 |
| `0x013a26b0` | `PsarcArchive_Mount` | [psarc-mount](psarc-mount.md) | `0x013d3600` | interpolated + mnemonics 0.93 (next 0.77) | 0.93 | 50 |
| `0x013a27d0` | `PsarcArchive_WaitAndMountAll` | [psarc-mount](psarc-mount.md) | `0x013d3730` | string xrefs 1/1 | 0.68 | 70 |
| `0x015c1f20` | `EnvSettings_RegisterKeys` | [lightmap-prelit](lightmap-prelit.md) | `0x015eda20` | string xrefs 79/79 | 0.70 | 70 |
| `0x015c2dc0` | `ShaderConstants_InitNameTable` | [lightmap-prelit](lightmap-prelit.md) | `0x015ef4e0` | string xrefs 103/103 | 0.90 | 80 |
| `0x015c6120` | `Scene_BindFrameConstants` | [lightmap-prelit](lightmap-prelit.md) | - | unresolved | - | - |
| `0x015c63a0` | `Scene_PrepareFrame` | [lightmap-prelit](lightmap-prelit.md) | `0x015f2d30` | string xrefs 3/3 | 0.90 | 80 |
| `0x01620980` | `ToneMap_ApplyEnvSettings` | [tonemap](tonemap.md) | `0x0164bc80` | by hand: same two `ToneMap_Instance` loads, same `+0x80..+0xbc` write sequence | - | 80 |
| `0x0178a160` | `PostTarget_Create` | [tonemap](tonemap.md) | `0x017da2d0` | string xrefs 6/6 | 0.83 | 70 |
| `0x0178bfb0` | `ToneMap_SetHistoryWeights` | [tonemap](tonemap.md) | `0x017dba10` | interpolated + mnemonics 0.95 (next 0.48) | 0.95 | 70 |
| `0x0178c490` | `ToneMap_UpdateCoefficients` | [tonemap](tonemap.md) | - | unresolved | - | - |
| `0x017864a0` | `PostTarget_Resolve` | [tonemap](tonemap.md) | `0x017d4560` | interpolated + mnemonics 0.95 (next 0.00) | 0.95 | 70 |
| `0x0178b970` | `ToneMap_RegisterPrograms` | [tonemap](tonemap.md) | `0x017db620` | string xrefs 4/4 | 1.00 | 80 |
| `0x01610df0` | `Composite_RegisterPrograms` | [tonemap](tonemap.md) | `0x0163de60` | string xrefs 6/6 | 1.00 | 80 |
| `0x017a4eb0` | `Display_SetCheckerboard` | [tonemap](tonemap.md) | `0x017fc030` | interpolated + mnemonics 0.95 (next 0.50) | 0.95 | 70 |
| `0x012e1260` | `MagstripWake_Update` | [ships-effects](ships-effects.md) | `0x012fa300` | interpolated + mnemonics 0.88 (next 0.35) | 0.88 | 60 |
| `0x012e2720` | `MagstripWake_EnqueueRender` | [ships-effects](ships-effects.md) | - | unresolved | - | - |
| `0x012e2770` | `MagstripWake_Draw` | [ships-effects](ships-effects.md) | `0x012fb790` | interpolated + mnemonics 0.88 (next 0.35) | 0.88 | 60 |
| `0x012e3350` | `MagstripWake_Destruct` | [ships-effects](ships-effects.md) | `0x012fc4c0` | interpolated + mnemonics 0.98 (next 0.47) | 0.98 | 70 |
| `0x012e36d0` | `MagstripWake_DeleteSelf` | [ships-effects](ships-effects.md) | `0x012fc710` | interpolated + mnemonics 1.00 (next 0.85) | 1.00 | 50 |
| `0x012f8c70` | `Ship_StartMagstripSound` | [ships-effects](ships-effects.md) | `0x0132b1e0` | string xrefs 3/3 | 0.77 | 70 |
| `0x01312770` | `Ship_StopMagstripSound` | [ships-effects](ships-effects.md) | - | unresolved | - | - |
| `0x01768a40` | `Ship_IsOverMagStrip` | [ships-effects](ships-effects.md) | - | unresolved | - | - |
| `0x012ff590` | `Ship_IsOverMagStripBase` | [ships-effects](ships-effects.md) | - | unresolved | - | - |
| `0x0131b510` | `Craft_UpdateSurfaceProbes` | [ships-effects](ships-effects.md) | - | unresolved | - | - |
| `0x01315b30` | `Ship_MagStripEnterRumble` | [ships-effects](ships-effects.md) | `0x0133a530` | interpolated + mnemonics 1.00 (next 0.48) | 1.00 | 70 |
| `0x01312b80` | `Ship_MagStripExitRumble` | [ships-effects](ships-effects.md) | - | unresolved | - | - |
| `0x012091a0` | `BlendState_SetColor` | [ships-effects](ships-effects.md) | - | unresolved | - | - |
| `0x012091c0` | `BlendState_SetAlpha` | [ships-effects](ships-effects.md) | - | unresolved | - | - |
| `0x012091f0` | `BlendState_SetSeparateAlpha` | [ships-effects](ships-effects.md) | - | unresolved | - | - |
| `0x01209180` | `BlendState_SetEnable` | [ships-effects](ships-effects.md) | - | unresolved | - | - |
| `0x01209170` | `BlendState_Clear` | [ships-effects](ships-effects.md) | - | unresolved | - | - |
| `0x015fa610` | `Scene_BuildMaterialSortKeys` | [material-state](material-state.md) | - | unresolved | - | - |
| `0x01301ba0` | `Ship_LoadModelSet` | [zone-craft](zone-craft.md) | `0x0131b1f0` | string xrefs 12/12 | 0.90 | 80 |
| `0x0174ca60` | `Language_ChoosePluginName` | [language](language.md) | - | unresolved | - | - |
| `0x01710730` | `Particle_RegisterShaders` | [heat-haze](heat-haze.md) | `0x0175b6a0` | string xrefs 12/12 | 1.00 | 80 |
| `0x01711310` | `Particle_DrawBatchesForPass` | [heat-haze](heat-haze.md) | - | unresolved | - | - |
| `0x016d3bf0` | `ParticleRenderer_Construct` | [heat-haze](heat-haze.md) | `0x0171ba00` | string xrefs 22/22 | 1.00 | 80 |
| `0x016d7ec0` | `Particle_RegisterEmitterStates` | [heat-haze](heat-haze.md) | - | unresolved | - | - |
| `0x01711af0` | `Particle_AcquireRenderState` | [heat-haze](heat-haze.md) | `0x0175ce60` | string xrefs 1/1 | 0.80 | 70 |
| `0x01710ec0` | `Particle_BuildBlendVariants` | [heat-haze](heat-haze.md) | `0x0175bf20` | interpolated + mnemonics 1.00 (next 0.55) | 1.00 | 70 |
| `0x01714000` | `Particle_PushQuadBatch` | [heat-haze](heat-haze.md) | `0x0175f540` | interpolated + mnemonics 0.93 (next 0.47) | 0.93 | 60 |
| `0x016304d0` | `Renderer_OpenDrawSegment` | [heat-haze](heat-haze.md) | `0x01669e20` | interpolated + mnemonics 0.85 (next 0.40) | 0.85 | 60 |
| `0x01629ce0` | `Renderer_RunPassSegments` | [heat-haze](heat-haze.md) | - | unresolved | - | - |
| `0x0161c960` | `Renderer_BeginScenePass` | [heat-haze](heat-haze.md) | - | unresolved | - | - |
| `0x0161d040` | `Renderer_BeginDistortionPass` | [heat-haze](heat-haze.md) | - | unresolved | - | - |
| `0x0161cf50` | `Renderer_BeginPass10` | [heat-haze](heat-haze.md) | - | unresolved | - | - |
| `0x01793990` | `Gnm_ClearSurface` | [heat-haze](heat-haze.md) | `0x017e2490` | interpolated + mnemonics 0.98 (next 0.62) | 0.98 | 70 |
| `0x01621650` | `Renderer_ResizeTargets` | [heat-haze](heat-haze.md) | - | unresolved | - | - |
| `0x012e8fb0` | `PlayerInput_Update` | [player-input](player-input.md) | `0x01302b50` | interpolated + mnemonics 0.85 (next 0.45) | 0.85 | 60 |
| `0x01747c90` | `Pad_UpdateState` | [player-input](player-input.md) | `0x01797b80` | interpolated + mnemonics 0.98 (next 0.50) | 0.98 | 70 |
| `0x017499b0` | `Pad_Open` | [player-input](player-input.md) | `0x0179ac40` | interpolated + mnemonics 1.00 (next 0.47) | 1.00 | 70 |
| `0x012c80c0` | `AiManager_AddDriver` | [ai-controls](ai-controls.md) | `0x012e0e70` | string xrefs 1/1 | 0.93 | 75 |
| `0x012c6570` | `AIManager_Update` | [ai-controls](ai-controls.md) | `0x012df1f0` | interpolated + mnemonics 0.85 (next 0.45) | 0.85 | 60 |
| `0x012c0480` | `AI_UpdateCraft` | [ai-controls](ai-controls.md) | `0x012d9190` | interpolated + mnemonics 0.95 (next 0.00) | 0.95 | 70 |
| `0x012c5650` | `AI_ComputeOpponentThrust` | [ai-controls](ai-controls.md) | `0x012de2d0` | interpolated + mnemonics 0.97 (next 0.38) | 0.97 | 70 |
| `0x012c9650` | `AI_ResolveSkillScale` | [ai-controls](ai-controls.md) | - | unresolved | - | - |
| `0x012c99b0` | `AiStats_Load` | [ai-controls](ai-controls.md) | `0x012e2930` | string xrefs 12/12 | 1.00 | 80 |
| `0x0147e8e0` | `CampaignCell_ParseValues` | [ai-controls](ai-controls.md) | `0x014a4590` | string xrefs 21/21 | 0.98 | 80 |
| `0x015a0f90` | `GameMode_SetCurrent` | [ai-controls](ai-controls.md) | `0x015cb2e0` | interpolated + mnemonics 0.95 (next 0.78) | 0.95 | 50 |
| `0x01393610` | `Collision_Construct` | [mp-time-trial-collision](mp-time-trial-collision.md) | `0x013c47f0` | string xrefs 6/6 | 0.78 | 70 |
| `0x0131ca50` | `Collision_ProcessPairs` | [mp-time-trial-collision](mp-time-trial-collision.md) | - | unresolved | - | - |
| `0x012539b0` | `Collision_TestObbSphere` | [mp-time-trial-collision](mp-time-trial-collision.md) | `0x01260640` | interpolated + mnemonics 1.00 (next 0.43) | 1.00 | 70 |
| `0x016544a0` | `Race_ComputeLoyaltyAward` | [endrace-loyalty](endrace-loyalty.md) | - | unresolved | - | - |
| `0x012f7200` | `HullShake_Update` | [hull-shake](hull-shake.md) | - | unresolved | - | - |
| `0x013674e0` | `QuakeManager_Fire` | [weapon-instance-limits](weapon-instance-limits.md) | - | unresolved | - | - |
| `0x01366150` | `Quake_Init` | [weapon-instance-limits](weapon-instance-limits.md) | - | unresolved | - | - |
| `0x01366760` | `QuakeManager_Update` | [weapon-instance-limits](weapon-instance-limits.md) | `0x01393eb0` | interpolated + mnemonics 0.93 (next 0.45) | 0.93 | 60 |
| `0x01372ee0` | `WeaponManager_DispatchFire` | [weapon-instance-limits](weapon-instance-limits.md) | `0x013a2cd0` | string xrefs 12/12 | 0.95 | 80 |

## 2048 cross-check

**Not checkable as a translation**: 2048's executables are separate programs with their own base
and patch (`vita-2048-eu-v104` is already the patch build, which is what a device runs). The lesson
does apply: on 2048 the docs' target of record is the patch (`v104`); on Omega it was the base.

## Open

- The ToneMap object's fields at the grid (`+0x80..+0xc4` through the pointer at `0x021dbdb8`)
  need another live read; the page's law has not been compared with a running value yet.
- 34 unresolved rows; the low-similarity string rows (55) want a reading by hand before they are
  used for a live read.
- Import into the main Ghidra project (command above) and re-point the base pages' readings that a
  live read depends on.
