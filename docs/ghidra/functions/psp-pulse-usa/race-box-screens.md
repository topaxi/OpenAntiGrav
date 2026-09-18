# `TrackSelection` and `TeamSelection`, decompiled

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`.

This is the follow-up to the string archaeology on
[`docs/formats/race-setup.md`](../../../formats/race-setup.md): both screen
classes were unreachable by cross-reference until the PSP relocation patch
landed (2026-09-07, see `HANDOVER.md`'s "Traps that are live"). With it
applied, `get_xrefs_to` on both class-name strings resolves cleanly -
`s_TrackSelection_08a84a00` to `0x088ee9f8`, `s_TeamSelection_08a84688` to
`0x088eb8e8` - and both classes decompile in full. **Nothing here is
runtime-verified**; every score below is capped by the confidence rubric's
70-84 "Probable" band unless stated otherwise.

## The registration pattern

Both classes register the same way the `Movie` widget does
([frontend-video.md](frontend-video.md)): a global-constructor-style function
with no callers found by `get_xrefs_to` (consistent with a `.ctors`/init-array
table Ghidra does not track as a call), that

1. inserts the class into a name-sorted registry (`FUN_0888fc68`, confidence
   73 - full body read, but the registry's own consumer, presumably the
   screen factory that looks a class up by name when the front-end XML says
   `type="TrackSelection"`, was not traced),
2. sets the object's vtable pointer directly (`DAT_08ad0ce4` for
   `TrackSelection`, `DAT_08ad0b04` for `TeamSelection`), and
3. prepends the object to a second, generic linked list (`FUN_08971afc`,
   not renamed - it is a plain list-prepend used by every registered
   widget and screen alike, so it carries no class-specific evidence).

| Address | Name | Conf |
| --- | --- | ---: |
| `0x088ee9f8` | `TrackSelection_Construct` | 75 |
| `0x088eb8e8` | `TeamSelection_Construct` | 75 |
| `0x0888fc68` | `ScreenClass_Register` | 73 |

## The vtable: eight slots shared with `Movie`, more shared between the two screens

Both classes' vtables were read directly (`read_memory`, 128 bytes each) and
compared word-for-word against `Movie`'s (`0x08acdacc`). Every vtable in this
binary follows the same odd layout: **each slot is 8 bytes, a zero word
followed by the function pointer**, not 4 bytes as a plain MIPS vtable would
be. Confirmed identically on all three vtables inspected; the reason for the
zero word (RTTI companion word, a this-adjustor always zero for
single-inheritance calls, or a `struct { int; void*; }` pair) was not
determined.

Eight slots (word indices 3, 5, 7, 11, 15, 19, 21, 23) hold the **identical**
function pointer across `Movie`, `TrackSelection` and `TeamSelection` -
`0x089444d0`, `0x0894479c`, `0x089447a4`, `0x089447f4`, `0x0894484c`,
`0x08944864`, `0x089449a4`, `0x08944a30` - a common root class's virtuals
(destructor variants, `IsA`, and similar - none traced). Two more slots
(word indices 13 and 25: `0x08944804`, `0x08944a94`) are shared between
`TrackSelection` and `TeamSelection` but **not** `Movie`, consistent with an
intermediate `Screen` base class that `Movie` (a widget) does not inherit
from. Neither was traced either.

The remaining four slots diverge per screen, and that divergence is where
the actual screen behaviour lives:

| Slot (word idx) | `TrackSelection` | `TeamSelection` | Role |
| --- | --- | --- | --- |
| 9 | `0x088ed7dc` | `0x088eab14` | `Update`, called once a frame |
| 27 | `0x088ed4f4` | `0x088e9ab8` | scalar deleting destructor |
| 29 | `0x088ed63c` | `0x088e9c00` | `OnEnter` |
| 31 | `0x088ed6cc` | `0x088e9e5c` | `OnExit` |

## `TrackSelection`

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x088ed63c` | `TrackSelection_OnEnter` | 74 | resets the cached selection index to `-1`, seeds `+0xd8` from the current `Mode` global |
| `0x088ed7dc` | `TrackSelection_Update` | 80 | up/down cycling with a 0.25 s repeat, list (re)build on a state-change flag |
| `0x088edf3c` | `TrackSelection_PopulateList` | 80 | builds the reachable-circuit list; see below |
| `0x088ee2f8` | `TrackSelection_ApplySelection` | 80 | loads the preview mesh and the per-mode stat panel for the newly selected circuit |
| `0x088ed6cc` | `TrackSelection_OnExit` | 80 | writes the chosen circuit's name into the `Track` front-end global |
| `0x088ed4f4` | `TrackSelection_Destruct` | 72 | restores the base vtable, frees |

### `TrackSelection_PopulateList` closes "what populates the track list is unread"

`docs/formats/race-setup.md`'s open item is answered: `FUN_088edf3c`

1. seeds `+0xd8` (the current `Mode`, read once via the `Mode` front-end
   global on first entry) if not already set,
2. frees the previous list and collects **every** `PI_Track` definition
   through a generic allocate-and-grow helper (`FUN_088895b8`, not renamed -
   its allocation tag reads `"file"`, a source-location debug tag rather than
   a class name, so it carries no evidence of being track-specific beyond
   this call site),
3. keeps an entry only if `Definition_IsUnlocked` (`FUN_0888e29c`, 72 -
   see below) passes **and**, when `Mode == 6`, a second per-track byte flag
   at `+0x16e` is also set - this is the mechanism behind
   `race-setup.md`'s "why Custom Race offers three circuits": it is not
   only the absence of `<Unlock>`, it is a *second*, mode-gated flag that
   happens to be set on exactly those same three tracks. `Mode == 6` was not
   independently confirmed to mean "Custom Race" (no enum table was located),
   so read that identification at 55, below the naming floor - the
   PopulateList mechanics that use it are read directly from code and score
   higher on their own,
4. restores the previously-chosen circuit's index by name from the `Track`
   global if it is still present in the filtered list, and
5. runs a bubble-style adjacency sort that groups a track's `Black`/`White`
   pair together and always orders the entry whose name matches `Black`
   before the one matching `White` - the first **direct evidence** for how
   the two runs end up adjacent in the wrapping list `race-setup.md`'s
   PPSSPP capture already walked.

### `Definition_IsUnlocked` and the two-axis unlock read

`FUN_0888e29c` (confidence 72, `Definition_IsUnlocked`) reads:

- `+0x99`: a hard-hidden byte: nonzero always fails.
- a "cheat/dev unlock" global (`sceKernelGetGPI() & 1`, or a flag byte at
  `DAT_08b31774+0x45f`): if set, everything is unlocked.
- otherwise it walks a linked list at `+0x9c` - the `<Unlock>` rows
  `race-setup.md` already reads from `Definition.xml` - and calls one of nine
  unnamed predicate functions per entry (`FUN_0888e6e8`,
  `FUN_0888e86c`/`FUN_0888e9e0`/`FUN_0888ea30`/`FUN_0888ebd8`, and four
  companion "has this unlock type" gates). **Named-unlock entries OR
  together** (any one passing unlocks the item) while **empty-name entries
  AND together** - matching an `Exclusive="true"` row needing every
  condition against a plain named unlock needing only one. None of the nine
  predicates were individually traced; that is real remaining work, not
  guessed at here.

### `Top->Ship` has a confirmed referent after all

`race-setup.md` recorded `Top->Ship` as "no confirmed referent" from a
bounded live-capture probe. `TrackSelection_ApplySelection` resolves it:

```c
iVar3 = FUN_08890f30(*(int *)(param_1 + 0xf0) + 0x168, "Top->Ship", 0);
```

`param_1 + 0xf0` is **not** a static screen widget - it is the *currently
selected track definition object*, replaced every time the selection
changes (the old one is torn down via `FUN_088c4410` first). So `Top->Ship`
is a node inside **each track's own dynamically created preview scene**, not
a persistent widget on the `Track Creation` screen itself - which is exactly
why no button probe on the static widget tree ever found it. Once resolved,
the node is fed the mesh path `race-setup.md` already confirmed
(`%s\FE\%s.vex`, forward/reverse) through `FUN_089724b4` (sprintf) and
`FUN_088b9200` (a model-source setter, not independently named here).
Confidence 78 for the mechanism - unambiguous decompilation, matches the
already-confirmed path template exactly, not runtime-verified. The name
`Top->Ship` most likely survives from a shared preview-scene template also
used for an actual ship (compare `TeamSelection`'s equivalent node, named
`Info->Ship`, which *does* hold a craft) rather than describing what this
screen puts in it.

### The hexagonal window is a per-circuit `screen.xml`, and it shows stills

Read 2026-09-10, off the same function. Every branch of
`TrackSelection_ApplySelection` that changes the selection calls
`FUN_088c4410` on the definition object it just installed at `+0xf0`:

```c
FUN_088c4410(old_definition, "", 0);          // teardown of the previous one
...
if (definition->availableInZone /*+0x16e*/ == 0 || mode != 6)
    FUN_088c4410(definition, "Info", 0);
else
    FUN_088c4410(definition, "Zone", 0);
```

The strings at `0x08a84948`/`0x08a84950` are `Zone` and `Info`. The callee -
now **`TrackDefinition_EnterScreenState`** (`0x088c4410`, confidence 78) -
reads:

1. if the definition has no state machine at `+0x168`, return;
2. `StateMachine_FindState(+0x168, name)`; when the state is **not** there
   and `name` is non-empty, load a file into the machine first:
   `"%s\screen_zone.xml"` of the definition's `location` (`+0x94`) when
   `availableInZone` (`+0x16e`) is set and the `Mode` global reads `6`,
   `"%s\MP_Screen.xml"` when the third argument is set (split screen), and
   `"%s\screen.xml"` otherwise - through `FUN_08891448`, now
   **`StateMachine_LoadXml`** (`0x08891448`, 72: opens the file with
   `Xml_OpenFile`, allocates a state under the named parent and feeds every
   root element to `FUN_0889165c`);
3. `StateMachine_TransitionTo(+0x168, name, ...)`.

So the `Top->Ship` node of the previous section is a node of **that file**,
and the file is on the disc: `Data\Environments\16_Track\screen.xml`
(2,204 bytes on `pulse-psp-usa.chd`, dictionary-compressed like every other
front-end XML), plus `screen_zone.xml` and `MP_Screen.xml` beside it. Its
`<Screen name="Top">` holds the `<Mode3D><Model name="Ship">` the outline
ribbon is loaded into (`Src="%s\FE\forward.vex"`, `x=0 y=-300 z=-3700
RotX=1.5 RotY=0.1`, `OriginX=-145 OriginY=13 nearZ=1000 farZ=5000`) and,
under it, **a chain of `Info` states two seconds apart, each one nested
inside an anonymous `Screen` carrying one more `Image`**:

```text
Info -> Info2 -> Info3 -> Info4 -> Info3 Close -> Info2 Close -> Info
  1        2        3        4          3             2         cards
```

The cards are `%s\FE\image_01.mip` .. `image_04.mip` at `(50,70)`,
`(43,65)`, `(36,60)`, `(29,55)`, each card after the first over a
`Data\FE\Images\track_sel_shadow.mip` four units down and right of it,
`0xCF000000`. **The window shows stills, not a flythrough** - the reading
`race-setup.md` recorded off two captures a second apart was two different
stills, and the "stack of frames" visible in one of them is the chain three
deep. The hexagon is the still's own shape: `image_01.mip` is 256x128, 8bpp,
hex-cropped with its border baked in. Confidence 92 for the mechanism
(decompiled, and the file it names is on the disc and reads as described)
and 95 for what the window shows (the file's own content, drawn and
compared against the capture). Implemented in
`oag_ui::picker::slideshow`; see `docs/ui/selection-screens.md`.

`FUN_088c4410` has four more callers (`FUN_088b09b4`, `FUN_088b12c8`,
`FUN_088ecb54`, `FUN_088ed05c`) besides `TrackSelection_OnExit`'s teardown -
untraced, presumably the tournament and split-screen track pickers, which
`MP_Screen.xml` and the PS2's `Track CreationSplit` screen both suggest.

The PS2 pressing's `screen.xml` (7,244 bytes, plain XML) is the same file
scaled onto its 640x448 grid - cards at `(67,115)`, `(57,107)` .. and the
shadow `301x211` - with **five** cards rather than four, and the `Zone`
chain (`zone_01.mip` .. `zone_05.mip`) authored in the same file rather than
in a `screen_zone.xml` of its own.

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x088c4410` | `TrackDefinition_EnterScreenState` | 78 | loads `<location>\screen.xml` (`screen_zone.xml`, `MP_Screen.xml`) into the definition's state machine on first use, then transitions to the named state |
| `0x08891448` | `StateMachine_LoadXml` | 72 | opens a front-end XML file and adds its root elements as states under the named parent |

### The `Info Track %d.%d` layout hypothesis, still not settled either way

`TrackSelection_ApplySelection` switches on the `Mode` value cached at
`OnEnter` and picks one of two widget-count layouts via `FUN_088edc08`
(not traced further): a 2-widget layout for the default case (only
`Info1`/`Info3`, both cleared to blank) and a 7-widget layout for five other
`Mode` values, which additionally computes summed lap/stage records from a
handling-stats-shaped table (`FUN_088085d0`, not traced - **and that reading is
now corrected: it is the profile's saved-record store**, keyed by a definition's
name hash, so the summed figures are saved bests rather than handling stats. See
[`race-campaign.md`](race-campaign.md)). This dispatch is by
**`Mode`**, not by the circuit's own row-count property, so it neither
confirms nor refutes `race-setup.md`'s `Info Track %d.%d` = (count, index)
reading - it shows a *different* axis (which stat rows are shown) is
mode-driven, orthogonal to the still-open question of which numbered layout
template renders them.

## `TeamSelection`

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x088e9c00` | `TeamSelection_OnEnter` | 72 | resets cached music-select flag, seeds `+0xd8` |
| `0x088eab14` | `TeamSelection_Update` | 78 | craft/livery cycling, stat-bar refresh, Eliminator/Normal variant pick |
| `0x088e9e5c` | `TeamSelection_OnExit` | 74 | writes the pre-race music selection string, frees the roster list |
| `0x088e9ab8` | `TeamSelection_Destruct` | 72 | restores the base vtable, frees |
| `0x088ea6b8` | `FEScreen_SetStatBar` | 82 | shared bar-widget setter (see below) |

`TeamSelection_Update` is the single largest function in either class. Once a
team and craft are picked it:

1. resolves the four rating bars (`Speed`, `Thrust`, `Handling`, `Shield`)
   and the `Loyalty` bar through `FEScreen_SetStatBar`, reading the ratings
   off a per-craft record via `FUN_08808664` (offsets `+0xb4`/`+0xb8`/`+0xbc`/
   `+0xc0`) - a **different** table from the `<Handling>` XML
   [handling-stats.md](../../../formats/handling-stats.md) documents; this
   is a display-rating record, not the physics tuning block, and its own
   source file/struct was not located,
2. distinguishes three variant classes by comparing the resolved skin id
   against `6`, `8`/`0x12`, or neither - the `6` branch wires the fixed
   skin flag against the literal string `"Eliminator"`, the `8`/`0x12`
   branch against `"Normal"` (Eliminator mode itself does not exist in this
   project yet - see `docs/gameplay/race-modes.md` - but the disc's own
   Eliminator-variant livery selection is wired here, in code, for whenever
   that mode is picked up), and
3. picks `"ER_SUG_SHIP"` vs `"ER_FOR_SHIP"` help text depending on whether
   the current craft appears on a per-team suggestion/forced-ship list
   (`FUN_08973424`), which is a live mechanism `race-setup.md` did not
   record at all.

`FEScreen_SetStatBar(screen, label, value, max, scale)` (confidence 82,
clean and unambiguous) sets a `<Text name="{label}">` to `%d` of `value` and
a sibling `<Image name="{label} Bar">`'s fill fraction to
`scale * min(value, max) / max` - the generic widget both `Speed`/`Thrust`/
`Handling`/`Shield`/`Loyalty` on `Team Selection` go through.

## What is still open after this pass

- The nine unlock-predicate functions inside `Definition_IsUnlocked` are
  unnamed and untraced - this is what "circuits gate on a named grid" and
  "craft variants gate on loyalty" would need to become a confirmed
  mechanism rather than an XML reading corroborated by one live capture.
- `Mode == 6` reading as "Custom Race" is a 55-confidence hypothesis, not a
  finding - no enum table was located. **An enum table has since been located**
  (`0x08ab062c`, see [`race-campaign.md`](race-campaign.md)) and in *that* table
  `6` is **`Zone`**, not a custom race - which would make
  `TrackSelection_PopulateList`'s extra `+0x16e` gate "the circuits Zone is
  playable on", a reading that fits three circuits at least as well. Whether the
  `Mode` global this screen caches is the same enumeration was **not** checked,
  so this stays a lead: 60, and the `Mode == 6` identification above is
  superseded but not yet replaced.
- The per-craft rating record `FUN_08808664` resolves (`+0xb4..+0xc0`) is a
  distinct table from `HandlingStats.xml`, unlocated.
- `FUN_088edc08`'s layout dispatch and `FUN_088085d0`'s record lookup are
  read only at the call site, not decompiled themselves.
- None of this is runtime-verified. A PPSSPP capture stepping through
  `TrackSelection_PopulateList` or `TeamSelection_Update` with a breakpoint
  would move several of the 70s here into the 90s.

## Cross-platform: one exact-hash match on `psp-pulse-eu`

Per this project's EU-preference (`psp-pulse-eu` is the Ghidra target of
record, [ADR-0048](../../../architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md)),
all fifteen functions named above were checked against `/pulse/BOOT-psp-pulse-eu.BIN`
using the exact-opcode-hash method
[`exact-hash-transfer.md`](../psp-pulse-eu/exact-hash-transfer.md) already
established: `get_function_hash` on each USA address, matched against a full
`get_bulk_function_hashes` pull of all 10,672 EU functions.

**Only `Definition_IsUnlocked` (`0x0888e29c` USA) has an exact hash match**,
at `0x0888e0c4` on `psp-pulse-eu`. `diff_functions` confirms it: 123/123
instructions equal, zero added, zero removed - the callee list differs only
in naming (`FUN_0888ea30` etc. on one side, differently-addressed but
logically identical callees on the other), the same asymmetry
`exact-hash-transfer.md` already documents for every one of its 115 rows.
Applied to `docs/ghidra/functions/psp-pulse-eu/names.tsv` at USA confidence
minus 5 (72 → 67, below the 70 floor so the applier suffixes it `_q`).

**The other fourteen have no exact-hash match on `psp-pulse-eu` at all**, and
a single-address fuzzy check (`find_similar_functions_fuzzy`) against the
same binary returned nothing above 0.9 and only a 0.55 candidate at 0.5 for
`FEScreen_SetStatBar` - too weak to act on under this project's rule against
renaming off a score alone. That is a real difference from
`exact-hash-transfer.md`'s collision/physics-code sweep, where 115 of 131
candidate functions matched exactly: **this front-end/menu code region
diverges more between the USA and EU pressings than the simulation code
does**, plausibly because the front end carries region-specific text and
menu wiring where the simulation does not. Confirming that with a proper
`bulk_fuzzy_match` sweep (descending thresholds, collision-filtered, each
candidate individually diffed) rather than one address at a time is left for
whoever picks EU coverage of this area up next.

## History

- 2026-09-10: `FUN_088c4410` and `FUN_08891448` named, off
  `TrackSelection_ApplySelection`'s own call; the per-circuit `screen.xml`
  found and read on both pressings. The "flythrough" reading is retired.
- 2026-09-08: first pass, immediately after the PSP relocation patch
  (`HANDOVER.md`, 2026-09-07) unblocked `get_xrefs_to` on both class-name
  strings. Confirmed live before decompiling anything:
  `get_xrefs_to("TrackSelection")` → `FUN_088ee9f8`,
  `get_xrefs_to("TeamSelection")` → `FUN_088eb8e8`, both "No references
  found" as recently as 2026-09-05.
