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
