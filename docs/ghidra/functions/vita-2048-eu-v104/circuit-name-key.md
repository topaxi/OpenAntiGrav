# Which string-table key names a circuit (`PI_Track`), and what `%02d_Track` is for

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`, read statically 2026-11-12 (`vita-2048-track-key`). Every
claim here was re-read on the v1.00 base executable
(`/2048/eboot-vita-2048-eu-base.elf`) at its own addresses; the two builds
agree on every point below (table at the end).

The question this page answers ([`omega-status.md`](../../../formats/omega-status.md),
"2048 Vita's circuit names"): the sixteen HD-lineage circuits (`Vineta_K` ...
`Sol_2`, and their `_R`) have their names in `entries.xml` only under
`01_TRACK` ... `08_TRACK`, and both their `soundregister` and the number in
their `FEModelLocation` (`...\fe\track01` ... `track08`) agree with that number
8 of 8. The executable holds the format `%02d_Track`. Which value feeds it?

**Answer: none of them, because `%02d_Track` is not a display key.** Its two
uses build track *ids* (an HD-era hash table, a debug command-line option).
Every site that turns a `PI_Track` into text does it with one rule:

> **name = string table [ case-fold ( `PI_Track` `name` attribute ) ]**, and on
> a miss, **the `name` attribute itself, as authored.**

No authored `soundregister`, `FEModelLocation` number or list index takes part.
For the sixteen the key is `VINETA_K`, `ANULPHA_PASS`, and so on. No string
table that ships holds those keys, and nothing inserts them at run time
(census below). So every such site prints the bare id (`Vineta_K`).

The same holds for 2048's own ten (`SQUARE` misses too). Their headline names
come from `SP.xml`'s `TrackDefinition` (`m_trackName` at `+0x0`, `m_displayName`
at `+0x40`, both `char[0x40]`, registered in `FUN_812b4d6c`). That route's
consumer is **not read here**. Whether the original ever *shows* a
`PI_Track`-name site's output on a screen a player sees is open (below). The
names on the DLC grid may be baked into its track photos instead.

## `%02d_Track`: two uses, neither on a display path

The string sits twice in the binary, at `0x814194d4` and `0x8141a6a0`, with
one reference each.

1. **`FUN_8100077c`** (`0x810007aa` references `0x814194d4`). This is a
   constructor that loops 24 times:
   `sprintf(buf, "%02d_Track", table[i])`, then
   `this[0x204 + 4 i] = Hash_ComputeCrc32(buf)`. The `int[24]` at
   `0x815190f4` holds `16 32 3 19 18 2 10 26 21 5 4 20 25 9 30 14 17 1 13 29 6 22 7 23`.
   The result is 24 CRC32s of HD-numbered *track ids* (`16_Track`, `32_Track`,
   ...). No 2048 `PI_Track` carries any of those names (the sixteen are
   `Vineta_K` ...). The table is an HD/Fury order of some kind; its consumer of
   `+0x204` is not read. **Not renamed** (hypothesis only: an HD-era per-track
   hash list, about 40).
2. **`Game_Main`'s command-line parser** (`0x81002eda`, which `Game_Main`
   reaches at `0x81003e40`; reference at `0x81003500`). For `-track <n>` with a
   numeric argument in 1..50, it does `sprintf(buf, "%02d_Track", n)` and
   passes the result as the race's *track name*. Out of range it uses
   `"01_Track"`. A non-numeric argument is used as the name directly. This is
   an HD debug option. On 2048 it names no shipped circuit.

The `NN_Track` literals elsewhere tell the same story. `FUN_8111dbd4` (a
trophy check) strcmps the current track's name against `"01_Track"` ...
`"17_Track"`. `FUN_810a1e50` selects `"08_Track"`. `0x8151e92c` is a pointer table of
more HD ids (`16_Track` first). These are HD/Fury leftovers keyed by HD
*id*, which on 2048 match no `PI_Track`.

## The naming rule, function by function

### `Hash_ComputeCrc32` - `0x81222a72`

**Confidence: 95.** `u32 Hash_ComputeCrc32(const char *s)`. It lazily builds
the reflected table for polynomial `0xedb88320`, runs over `strlen(s)` bytes,
and returns `~crc`. That is the standard CRC-32, byte for byte. Same shape at
`0x8121674c` on the base build.

### `String_FoldCase` - `0x81068674`

**Confidence: 85.** `char *String_FoldCase(char *s)`, in place. For each byte
`c`, if the libc ctype table (`SceLibc_3CE6109D`) has bit `2` for `c`, it
writes the libc case table (`SceLibc_D662E07C`) entry for `c`. The direction
(upper or lower) is not pinned on Vita, because the NIDs are unresolved. It
does not matter here: the loader folds the keys the same way, so the match is
**case-insensitive**. Omega's twin is explicit about it (`_Getptolower()` in
`FUN_014043c0`, below). Base build: `0x810678b2`.

### `StringTable_LookupString` - `0x8106b86e`

**Confidence: 85.** `const char *StringTable_LookupString(StringTable *table,
const char *key, bool null_on_miss)`. A null key returns `"NULL_STRING_ID"`.
Otherwise it copies the key into 256 bytes, calls `String_FoldCase`, takes
`Hash_ComputeCrc32`, and looks the hash up in the table's map (`table+0x80`,
then `+0x48`). On a miss it walks the chain of further tables headed at
`DAT_8151d20c` (appended by `FUN_8106b618`) with the same folded hash. If
nothing matches, it returns **the key as passed** when `null_on_miss` is 0,
or null when it is 1. Every circuit-name call site below passes 0.
Corroboration:

- 199 call sites, nearly all with `DAT_8151cdb0` (the language table) and a
  literal `ALL_CAPS` key.
- The miss marker strings `"%X_(MISSING)"` and `"%d_(MISSING)"` in its sibling
  `FUN_8106b90a`.
- Base build: `0x8106aa6a`, same shape.

### `PITrack_ParseValues` - `0x810c1f94`

**Confidence: 85.** This is the `PI_Track` item's `<Values>` parser:
vtable `0x8150d8cc` slot 15, in the `Frontend/Items/PI_Track_Item.cpp` class
whose constructor is `0x810c16d8`. Attributes, by literal:

| Attribute | Stored at |
| --- | --- |
| `Location` | `+0xa0` (string) |
| the literal at `0x81448b84` (likely `type`) | 32-byte buffer, mapped through a 6-entry table to `+0x1d0` |
| `Reversed`, `hdFury`, `availableInZone`, `collisionCageEnabled` | bytes `+0x1d9` ... `+0x1dd` |
| the literals at `0x81448bc8`, `0x81448bd0`, `0x81448bd8` (likely `RotX`, `PosZ`, `PosY`; not read) | floats `+0x1c4`, `+0x1c8`, `+0x1cc` |
| `SoundRegister` | int `+0x1c0` |
| `FEModelLocation` | string `+0x1e0` |

There is **no `StringTableId`** and no other key-shaped attribute. Then it
hashes the item's name (`+0x74`) for its save record, and also `"%s_fury"` of
it. Base build: `0x810bfb76`, same attribute set.

`SoundRegister` at `+0x1c0` and `FEModelLocation` at `+0x1e0` are stored, but
no name lookup reads either of them. The fifteen `PI_Track`-name call sites
listed below all pass `+0x74`.

### `+0x74` is the `name` attribute: `RaceSetup_SelectTrackByName` - `0x81001ef4`

**Confidence: 75.** `void RaceSetup_SelectTrackByName(RaceSetup *setup, const
char *name)`. It enumerates the `PI_Track` items and `strcmp`s each one's
`+0x74` against `name`. On a match it stores the item at `setup+0x1c0` and
copies `+0x74` to `setup+0x1c4`. Without a match it falls back to a lookup by
`Hash_ComputeCrc32(name)` against `+0x54`. Its callers pass `"square"` (in
`FUN_8123eae0`) and `"08_Track"` (in `FUN_810a1e50`). Those are a 2048
`PI_Track name=` value and an HD one, so `+0x74` is the element's `name`
attribute. `DAT_8153fe00`, the "current track" pointer every display site
below reads, is `DAT_8153fc40 + 0x1c0`, the slot this function fills.

### The display sites: all pass `+0x74`

Each call is `StringTable_LookupString(DAT_8151cdb0, item+0x74, 0)`.

**Shown to hold a track.** These read `DAT_8153fe00`, the current-track slot
`RaceSetup_SelectTrackByName` fills, or a tournament's track list:

| Function | What it does with the name |
| --- | --- |
| `FUN_81093baa` (line `393`) | `sprintf("%s %d/%d", name, i+1, n)`, then the label setter `FUN_810b16c4` (tournament header) |
| `FUN_810aa49c` (line `585`) | label setter `FUN_810b16c4` |
| `FUN_810c1180` | returns the name of a tournament's `n`th track |
| `FUN_81109ca0`, `FUN_8123eae0`, `FUN_810a1e50` | hand the name to `FUN_812275d6`, beside `Hash_ComputeCrc32` of the same `+0x74` for `FUN_81226994` |
| `FUN_8111dbd4` | trophy check; the result is discarded |
| `FUN_813ca190`, `FUN_813d3790`, `FUN_8123efc6`, `FUN_8123f250` | further uses of the current track's name (not read further) |

That makes eleven functions, and none of them passes `SoundRegister`, the
`FEModelLocation` or an index. **"The key a `PI_Track` is named by is
case-fold(`name`)": confidence 80.**

**Not shown to hold a track** (`+0x74` is every PI item's name, so these
support the same rule for items in general): `FUN_81136d36`, `FUN_811409e2`
and `FUN_81142f74` (beside `MSC_UNL_*` strings, so music-unlock screens),
`FUN_8100b930`, `FUN_8100ba12`, `FUN_81184da4`, and `FUN_810aa49c` line `611`.

### `PIItem_LoadLocationStringTable` - `0x810663ae`

**Confidence: 75.** This is vtable slot 17 of `PI_Track` (`0x8150d8cc`), and
the same pointer sits in 13 other item vtables (`0x8150a964`, `0x8150d430` ...
`0x8150d910`). If the item has a `Location` (`+0xa0`), it formats
`"%s\stringtable.xml"` from it. If that file exists, it loads it as a link
object (`FUN_8106a1c4`). The plugin constructor `FUN_810669f4` does the same
for `<plugin folder>\stringtable.xml` beside each `Definition.xml`. A
`StringTable` element joins the fallback chain at `DAT_8151d20c`
(`FUN_8106b618` appends; `StringTable.cpp`'s constructor is `0x8106b690`). So
a per-circuit `Data\art\published\DLC1\environments\Vineta_K\stringtable.xml`
holding `VINETA_K` would name that circuit, and so would one per plugin folder
(`...\DLC1\plugins\tracks\stringtable.xml`). That is where an authored name
for the sixteen *would* live. Base build: `0x81065666` and `0x81065c5c`.

**Census, 2026-11-12: no `stringtable.xml` ships anywhere.** Checked with
`scripts/psarc.py list` over every archive of both regions: `PCSF00007` base,
patch v1.04 `data1`/`data2`, `dlc1`, `dlc2` (24,434 entries), and `PCSA00015`
base, patch v1.04 `data1`/`data2`, `dlc1` (22,724 entries). No entry's path
contains `stringtable`. The language files are
`data/plugins/languages/<lang>/entries.xml` alone, and no copy has an `id`
equal to any of the 26 ids under case folding. The nearest are
`MP_SQUARE_RACE_FLASH`-style sentences. So all 26 `PI_Track` names miss.

## What the original shows for the sixteen: authored names, as text (measured)

Vita3K stills, 2026-11-12 (lane `vita-2048-trackname-still`; the full record is
[`omega-status.md`](../../../formats/omega-status.md), "2048 Vita's circuit names"):
the HD Campaign's event card, the mode card over the load and the pause card read
`VINETA K`; the Anulpha Pass card reads `ANULPHA PASS`; Fury's Amphiseum cards
read `THE AMPHISEUM`, which no re-spelling of the id `Amphiseum` produces, so it
is the authored `19_TRACK` string. Nothing showed `Vineta_K`.

So the rule above (confidence 80) describes the eleven `PI_Track`-name sites it
lists and is **not** what the event card, the pause card or the AdHoc lobby
draw: those reach an authored `NN_TRACK`-family name by a key this page did not
read. Which key (circuit number, direction, a list) is open; a `_R` cell's card
decides it. Ours still falls back to the id and does not match.

The paths this page checked and ruled out as a source stand: no run-time insert
into the string table, no `<StringTable>` outside the language plugins, no
`stringtable.xml` on either region's packages. The literal `TouchButton String=`
names in the v1.00 `NEWGUI/Definition.xml` (lines `526`-`537`) are not what
patch 1.04 draws either.

`01_TRACK` ... `24_TRACK` in `entries.xml` are HD's keys (HD's ids are
`01_Track` ...). No 2048 `PI_Track` names them and no executable site was found
that does, yet the original draws their strings on the HD and Fury cards.
Wiring them in ours through `soundregister` or `trackNN` is still **chosen, not
measured** until the card's key is read.

## Omega: checked, differs

`/omega/eboot-ps4-omega-eu-v107.bin`, same reads:

- **`PI_Track` parser `FUN_014b1330`** reads `SoundRegister` into a local
  (`_Stoul`), `FEModelLocation` into `+0x270`, and **`StringTableId` into
  `+0x278`**. Vita's parser has no `StringTableId` branch at all.
- **The lookup `FUN_014043c0`** has the same shape as
  `StringTable_LookupString`: `"NULL_STRING_ID"` for a null key, then
  `strncpy` 256, case fold through `_Getptolower()`, hash, chain at
  `DAT_01af58e0`.
- **Display sites** (`FUN_015c6e60`, `FUN_015c9860` at four places) find the
  item whose name (`+0xb8`) matches. Each one passes
  `item+0x278 != NULL ? item+0x278 : item+0xb8`, so the key is
  **`StringTableId` if authored, else the name**.
- **`%02d_Track`** at `0x01891e56` keeps the same two roles: the `-track`
  option in the boot function `FUN_0167a020`, and a table loop in
  `FUN_01638440` over `DAT_018a3ce0`.

So Omega's rule is Vita's rule with one authored override. Omega's 28 numbered
circuits are named by their `NN_Track` ids. Its ten 2048 circuits are named
by `StringTableId="Track_Altima_0"` .... This is what
`oag_raceplay::catalogue::Track::name_key` already implements. Vita has no
such override, and its sixteen have non-numbered ids. **Checked, differs**.

## Base build (v1.00) corroboration

| Role | v1.04 | v1.00 base |
| --- | --- | --- |
| `%02d_Track` table loop | `0x8100077c` | `0x81000738` |
| `%02d_Track` `-track` option | `0x81002eda` | `0x8100376c` |
| `Hash_ComputeCrc32` | `0x81222a72` | `0x8121674c` |
| `String_FoldCase` | `0x81068674` | `0x810678b2` |
| `StringTable_LookupString` | `0x8106b86e` | `0x8106aa6a` |
| `PITrack_ParseValues` | `0x810c1f94` | `0x810bfb76` |
| `PIItem_LoadLocationStringTable` | `0x810663ae` | `0x81065666` |

The base-build names are not applied: `names.tsv` covers v1.04 only.

## The card's key: the track alias table (2026-11-12, `vita-2048-card-title-key`)

Read statically on `/2048/eboot-vita-2048-eu-v104.elf`. This section answers
what the section above left open ("What the original shows for the sixteen"):
which key the event card, the mode card over the load and the pause-style
callout use. **The key is the HD track id `NN_Track` that a static alias table
pairs with the `PI_Track` name, one row per direction.** It predicts all three
photographed strings (`VINETA K`, `ANULPHA PASS`, `THE AMPHISEUM`).

> **title = string table [ case-fold ( alias ( `track` ) ) ]**, where
> `alias(n)` is the `key` of the table row whose `name` *or* `key` has the same
> CRC-32 as `n` (raw bytes, so case-sensitive), and `"99_Track"` when no row
> matches. On a string-table miss the key itself is printed.

Confidence **85**: three independent draw sites use it (below), the table is
literal data, and it reproduces the three stills. Not higher because no live
run watched the lookup.

### `g_track_alias_table` - `0x814b1578` (data)

**Confidence: 90.** 102 rows of `{const char *name; u32 reversed; const char
*key}` (stride 12) in the read-only segment. The rows for every circuit the
DLC `tracks/Definition.xml` files author (DLC1's sixteen, DLC2's eight and
`Zone_1`..`Zone_4`), with the English `entries.xml` string each key resolves
to (patch v1.04 `data2`):

| Row | `name` | `reversed` | `key` | English string |
| --- | --- | --- | --- | --- |
| 0 | `Vineta_K` | 0 | `01_Track` | `VINETA K` |
| 1 | `Anulpha_Pass` | 0 | `08_Track` | `ANULPHA PASS` |
| 2 | `Moa_Therma` | 0 | `03_Track` | `MOA THERMA` |
| 3 | `Chenghou_Project` | 0 | `04_Track` | `CHENGHOU PROJECT` |
| 4 | `Metropia` | 0 | `02_Track` | `METROPIA` |
| 5 | `Sebenco_Climb` | 0 | `06_Track` | `SEBENCO CLIMB` |
| 6 | `Ubermall` | 0 | `05_Track` | `UBERMALL` |
| 7 | `Sol_2` | 0 | `07_Track` | `SOL 2` |
| 8 | `Vineta_K_R` | 1 | `09_Track` | `VINETA K` |
| 9 | `Anulpha_Pass_R` | 1 | `16_Track` | `ANULPHA PASS` |
| 10 | `Moa_Therma_R` | 1 | `11_Track` | `MOA THERMA` |
| 11 | `Chenghou_Project_R` | 1 | `12_Track` | `CHENGHOU PROJECT` |
| 12 | `Metropia_R` | 1 | `10_Track` | `METROPIA` |
| 13 | `Sebenco_Climb_R` | 1 | `14_Track` | `SEBENCO CLIMB` |
| 14 | `Ubermall_R` | 1 | `13_Track` | `UBERMALL` |
| 15 | `Sol_2_R` | 1 | `15_Track` | `SOL 2` |
| 16 | `Talons_Junction` | 0 | `17_Track` | `TALON'S JUNCTION` |
| 17 | `Talons_Junction_R` | 1 | `18_Track` | `TALON'S JUNCTION` |
| 18 | `Amphiseum` | 0 | `19_Track` | `THE AMPHISEUM` |
| 19 | `Amphiseum_R` | 1 | `20_Track` | `THE AMPHISEUM` |
| 20 | `Modesto_Heights` | 0 | `21_Track` | `MODESTO HEIGHTS` |
| 21 | `Modesto_Heights_R` | 1 | `22_Track` | `MODESTO HEIGHTS` |
| 22 | `Tech_De_Ra` | 0 | `23_Track` | `TECH DE RA` |
| 23 | `Tech_De_Ra_R` | 1 | `24_Track` | `TECH DE RA` |
| 24 | `Zone_1` | 0 | `25_Track` | `PRO TOZO` |
| 25 | `Zone_2` | 0 | `26_Track` | `MALLAVOL` |
| 26 | `Zone_3` | 0 | `27_Track` | `CORRIDON 12` |
| 27 | `Zone_4` | 0 | `28_Track` | `SYNCOPIA` |

The remaining rows: `Zone_Pad` (`29_Track`), `WV_` twins of rows 0-28 with the
same keys, 2048's ten (`square` `30_Track` ... `altima` `39_Track`), their
`WV_` scenes (`50_Track` ... `62_Track`), `WV_track1..10` and `track1..10`
(`63_track` ... `82_track`), and `Default` (`99_Track`) last. No `entries.xml`
has an id from `29` up, so those keys print as themselves; 2048's own events
are titled from `m_displayName` instead (below).

**The key is per direction, not per circuit.** A reverse has its own row and
its own id (`Vineta_K_R` is `09_Track`, not `01_Track`). English and most
languages give both ids the same string, but **Russian does not**: patch
v1.04 `data1` and `data2` Russian `entries.xml` have `03_TRACK` = `МОА THERMA`
and `11_TRACK` = `MOA THERMA`, and the base `data.psarc` Russian and Japanese
files differ on all eight forward/reverse pairs (`01_TRACK` `ВИНЕТА К` against
`09_TRACK` `VINETA K`). Every other language of every package gives the pairs
`01/09` ... `08/16`, `17/18`, `19/20`, `21/22`, `23/24` the same string
(census over the 49 `entries.xml` copies of the EU base, patch v1.04 `data1` and `data2`).

**`soundregister` is not the key.** `soundregister + 8 * reversed` happens to
equal the key's number for DLC1's sixteen, but DLC2 breaks it:
`Talons_Junction` has `soundregister="12"` and key `17_Track`, `Amphiseum`
has `10` and `19_Track`, `Modesto_Heights` `9` and `21_Track`.

### `g_track_alias_map` - `0x818bc250` (data)

**Confidence: 85.** BSS, 102 rows of `{name, crc32(name), key, crc32(key)}`,
filled once from the table by `FUN_81231df6` (`0x81231eb0` writes the base).
`FUN_81231df6` is the constructor of a larger object at `DAT_818bbf88` (it also
sets `"VITA_Node"` and `"MPStressVITA"`), so it is **not renamed**. The three
lookups below take that object as their first argument and read the map.

### `TrackAlias_KeyForHash` - `0x81232400`

**Confidence: 85.** `const char *(void *self, u32 hash)`. Returns row `key`
for the first row whose `crc32(name)` or `crc32(key)` equals `hash`, else
`"99_Track"` (the literal is in the function). Matching either hash makes the
map idempotent on HD ids: `alias("01_Track")` is `01_Track`.

### `TrackAlias_NameForHash` - `0x812323b6`

**Confidence: 85.** Same scan, returns row `name`, else `"Default"`. The card
uses it to pick the track photo of a tournament's `n`th track.

### `TrackAlias_KeyForName` - `0x81232436`

**Confidence: 80.** `const char *(void *self, const char *name)`. CRC-32 of
`name`, then the hash scan; on a miss, a second `strcmp` scan over `name` and
`key`, else `"01_Track"`. Its callers resolve a track item by either spelling
(`FUN_8105a5d2`).

### `CampaignEventCard_DrawCellPanel` - `0x8105021c`

**Confidence: 75.** The HD/Fury grid's version of
`CampaignEventCard_DrawPanel` (same 404-wide photo, header panel, title at
`x=682` `y=100` in `default` at scale `0x3f51eb85`). Its integer argument
(`r0`) is a `PI_Cell` item. The title:

```
if cell+0x128 == 0:                              ; not a tournament cell
    key  = TrackAlias_KeyForHash(&obj, Hash_ComputeCrc32(cell + 0xea))
    text = StringTable_LookupString(DAT_8151cdb0, key, 0)
else:                                            ; tournament cell
    "Tournament", then "%s (%d/%d)" of the same lookup on cell+0x12c[i]
```

`cell+0xea` is the cell's `track` attribute (below). The photo is
`TrackAlias_NameForHash` of the same hash. Callers, each passing the current
cell `DAT_8153fcd4` (`DAT_8153fc40 + 0x94`) or a cell item in `r0`:

| Caller | Screen | Evidence |
| --- | --- | --- |
| `0x810f8796` | HD grid cell screen | vtable `0x8150eab4` slot at `0x8150ead0`; the vtable is stored by `TouchCampaignHD_Item_Construct` |
| `0x810f5620` | Fury grid cell screen | vtable `0x8150ea48` slot at `0x8150ea64`; stored by `TouchCampaignFury_Item_Construct` |
| `LoadingScreen_Draw` state 4 | mode card over the load | `0x8128435c cmp r0,#0` ... `0x81284370 bl 0x8105021c`, `r0` = `DAT_8153fcd4` (the decompile shows `0` because the prototype is wrong) |
| `0x81082af4` (no function defined) | `Callout_Item` | vtable `0x8150b370` slot at `0x8150b38c`; stored by `FUN_813c6b58`, which names `Frontend/Items/Callout_Item.cpp`. `0x81082b1e` loads `DAT_8153fc40+0x94` into `r0` |

Which of these the 2026-11-12 pause still shows is not read; the callout
item is the in-race candidate.

### Two more sites with the same rule

- **`CampaignEventCard_DrawPanel`, no-event branch** (`param_2 == 0`, the
  pre-race callout and Racebox path): title is
  `StringTable_LookupString(TrackAlias_KeyForHash(Hash_ComputeCrc32(current
  track +0x74)))`, tournaments `"%s (%d/%d)"` of the same.
- **`FUN_810a27f8`**, a row of the online game list: the label is
  `StringTable_LookupString(TrackAlias_KeyForHash(row+0x1474))`. Its only
  caller `FUN_810a2a92` is called by `FUN_810a2cd8`, slot `0x8150c1a0` of the
  vtable `0x8150c184` that `FUN_810a19e0` stores; that constructor labels the
  list with `ONL_CRJ_GLIST`.

**Prediction, not measured:** on these two sites a 2048 circuit (`square`)
resolves to `30_Track`, which no English `entries.xml` holds, so they would
print `30_Track`. No still shows either site with a 2048 circuit. The AdHoc
lobby still (`UNITY SQUARE` over `COMBAT`, with a NAME / XP / WINS player list)
is **not** the no-event branch: the player list is page kind `7`, which
`DrawPanel` draws only when it is passed an event, so that title is the
event's `m_displayName` and agrees with the rule below.

With an event (`param_2 != 0`), `DrawPanel` draws `event + 0x144` unchanged:
`GameModeBase.m_trackDef` is at `+0x104` and `TrackDefinition.m_displayName`
at `+0x40` of it. `SP.xml` authors that as text (`METRO PARK`, ten values), so
2048's own events never touch the alias table.

### `PICell_ParseValues` - `0x810bbfce`

**Confidence: 70.** Parses a grid cell's children: `Values` (attributes
`Weapons`, `Damage`, `AICount`, `Status`, `Locked`, `ShipChoice`, `Track`,
`skillEasy`/`Medium`/`Hard`, `Class`, and three literals not resolved), `Gold`/`Silver`/`Bronze`, the
nine `Easy`/`Medium`/`HardGold`... targets, `NitroElim*` and
`TournamentTrack` (`track`, hashed into `+0x12c`, count `+0x128`). `Track`
is copied to `+0xea` (`0x18` bytes). The element set is exactly the
`<PI_Cell>` grammar of `data/plugins/grids/grid_NN.xml`. Not higher: the
`PI_Cell` class's vtable was not followed to this function.

### `TrackAlias_OtherHash` - `0x812324c6`

**Confidence: 80.** `u32 (void *self, u32 hash)`. Returns the *other*
spelling's hash of the matching row: `crc32(key)` when `hash` is
`crc32(name)`, `crc32(name)` when it is `crc32(key)`; no row returns row 0's
`crc32(key)` (`DAT_818bc25c`). Callers use it to find a track item by either
spelling after a direct miss: `RaceSetup_SelectTrackByName`, `FUN_810c1180`,
`FUN_8123eae0`, `FUN_8105a4f6` and ten more.

### Where the earlier rule still holds

`TrackAlias_KeyForHash`'s only callers are the two card painters and
`FUN_810a27f8`; `TrackAlias_NameForHash`'s are the two card painters. Three of
the earlier section's eleven sites (`RaceSetup_SelectTrackByName`,
`FUN_810c1180`, `FUN_8123eae0`) do touch the map, but only through
`TrackAlias_OtherHash`, to *find* the `PI_Track` item; the text they produce is
still `StringTable_LookupString(item + 0x74)` (read in `FUN_810c1180`: it
returns `FUN_8106b86e(DAT_8151cdb0, item+0x74, 0)`). So the case-fold(`name`)
rule stands for those sites, and the card titles are a separate path. `FUN_8100077c`'s `%02d_Track` CRC list and `FUN_81059ebc` (which drops
the sixteen HD ids from a track list) stay unrelated.

### Omega: the alias table is present (card site not read)

`/omega/eboot-ps4-omega-eu-v107.bin` carries the same table at `0x01990120`,
stride 24 (8-byte pointers): row 0 `Vineta_K` / `0` / `01_Track`, row 1
`Anulpha_Pass` / `08_Track`, row 2 `Moa_Therma` / `03_Track`, and the strings
`WV_Vineta_K_R`, `Zone_Pad`, `99_Track` (`0x0189386c`, 25 references, inlined
in about twelve functions). Which of those is Omega's event card is not read.
For Omega's numbered circuits the map is idempotent (the key's own hash
matches), so `oag_raceplay::catalogue::Track::name_key` (`StringTableId`, else
the `NN_Track` id) gives the same strings it would for those 28. This covers
the alias table only: Omega's ten 2048 circuits would miss on it (`30_Track`
and up), and Omega names them by `StringTableId` through the `PI_Track` path
the section above read, so that section's **checked, differs** still holds.
Which path Omega's event card takes is not read.

### Ours (2026-11-12, `vita-2048-card-names-wire`)

**Wired on two of the sites, not the rest.** `oag_title::FrontEnd::card_track_aliases`
holds rows 0 to 27 of the table (`oag_2048::frontend::CARD_TRACK_ALIASES`; the
DLC circuits, not 2048's own ten and not the `WV_` rows). `oag_game::boot::tiers2048::card_title`
titles an HD or Fury event's card by `strings[alias(track)]` (exact key, else
case-folded, else the key), with no `FE_REVERSE` word. Drawn on the
`grid0_3_1` card: `VINETA K`; `grid0_5_1`: `ANULPHA PASS`; the reverse
`Talons_Junction_R` (`grid9_1_2`): `TALON'S JUNCTION`. The online game list asks
the same table by circuit id (`alias_title`); the room form and Track Select keep
the picker names. A circuit with no row keeps its ordinary name: ours does not
print `99_Track` or the unchecked `30_Track` prediction for 2048's own ten.
Not drawn by ours at all: the mode card over the load and `Callout_Item` (no
2048 screen of ours draws a circuit name there). Test:
`vita_2048_dlc_menus_ground_truth::an_hd_event_card_titles_its_circuit_by_the_alias_key_per_direction`.
Omega: **checked, applies, not wired**. Its card path is unread, its 28 ids
are the identity under the table, and its ten 2048 circuits are not rows of the
28 listed here, so `oag_omega`'s field stays empty and nothing there changes.

### Base build (v1.00)

The base `eboot.elf` has `99_Track` (file offset `0x4a4bdc`); the table was not
dumped there.
