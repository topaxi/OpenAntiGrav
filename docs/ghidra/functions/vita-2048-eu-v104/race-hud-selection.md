# Which HUD skin a real race constructs

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. **The names here are applied**, from [names.tsv](names.tsv).
Found while composing `oag_2048::hud`'s 26 shipped HUD layouts and asking which
one a race actually reads - `crates/2048/src/hud.rs` carried a bare guess
("the root set is the default ... for the same reason it is [HD]") that this
settles with code evidence instead.

## `SpArcadeRaceManager_Construct` - `0x8117af40`

**Confidence: 90**

A race-manager constructor: tags its allocations
`"Backend/General/SPArcade_RaceManager.cpp"` (`param_1[0xb]`, the same
tagged-allocation idiom `RcsModel_Load` and `GameRoot_Construct` both rest
their own confidence on) and, on the single-screen path
(`DAT_8153fd28 < 2`), constructs its HUD from the literal
`Data\XML\2048_hud\Arcade_HUD.xml` - not the bare `Data\XML\Arcade_HUD.xml`
[`oag_hd::hud`]-style root. The two split-screen branches load
`Data\XML\SplitScreen_hud\SplitScreen_HUD_{0,1}.xml` and the vertical variant,
neither of which [`oag_2048::hud`] models - out of scope, [`oag_hd::hud`]
excludes its own split-screen family on the same terms.

"SP" is Single Player: the constructed name distinguishes this from whatever
multiplayer arcade race construction is (unread).

## `DemoRaceManager_Construct` - `0x8114a090`

**Confidence: 85**

Tags its allocations `"Backend/General/Demo_RaceManager.cpp"` and constructs
its HUD from the **bare** `Data\XML\Arcade_HUD.xml` - the root, HD-shaped set.
This is the attract-mode/demo race, not a mode a player reaches by playing the
game normally.

## `FUN_812bf3f0` (Elimination) - `0x812bf3f0`, `FUN_812c1722`
(SpeedLap/TimeTrial) - `0x812c1722`, `FUN_812c4e2e` (Zone) - `0x812c4e2e`

**Not renamed - confidence 70, below the renaming floor for a class name this
project has not confirmed independently for each.** Same shape as
`SpArcadeRaceManager_Construct` without an equally direct `__FILE__`-style
string in the snippet decompiled, but each references only `2048_hud\`
paths, never a bare or `wo3_hud`/`2097_hud` one:

- `FUN_812bf3f0` builds `Data\XML\2048_hud\Elimination_HUD.xml`.
- `FUN_812c1722` branches on a param (`*(int*)(param_1+0x18c) > 0`) between
  `Data\XML\2048_hud\SpeedLap_HUD.xml` and
  `Data\XML\2048_hud\SpeedLap_TimeTrial_HUD.xml` - so **speed lap and time
  trial are two distinct files on this title**, not one file shared the way
  both PSP discs share time trial's for speed lap. `oag_title::HudLayouts`
  models the two as separate fields already; this is the evidence that they
  should carry different values here, where HD's own table gives them the
  same file only because that happens to be what its manifest ships.
- `FUN_812c4e2e` builds `Data\XML\2048_hud\Zone_HUD.xml`.

## What this settles: `2048_hud` is the played skin, not `wo3_hud`/`2097_hud`

Every real race-construction path found reaches into `2048_hud\`. Searching
`eboot.elf`'s defined-string table for `wo3_hud` and `2097_hud` (both
substrings) returns **zero** matches - those two skins, which the archive
ships in full (`Data\XML\wo3_hud\*`, `Data\XML\2097_hud\*`, five files each,
composing cleanly with the same widget dialect), are never referenced by any
string this build's analysis has found. They are present on disc and
unreachable from any code path found so far - not proven dead code, since
`search_strings` only walks Ghidra's *auto-detected* string table and the
`SpeedLap_HUD.xml` / `MPTag_HUD.xml` misses below show that table is
incomplete, but no positive evidence for either skin exists either.

**Caveat found while writing this up**: `FUN_812c1722`'s own
`Data\XML\2048_hud\SpeedLap_HUD.xml` literal, visible directly in its
decompilation, does not appear in `search_strings("2048_hud.SpeedLap_HUD")`
- so a `search_strings` miss is not evidence a path is unreferenced, only that
it was not independently corroborated. The two constructors renamed above
each combine a `search_strings` hit *and* a decompiled read, which is why they
cleared the renaming floor and the three `FUN_*` ones did not.

**Consequence for `oag_2048::hud::LAYOUTS`**: it should read the `2048_hud\`
set (`Arcade_HUD.xml`, `Elimination_HUD.xml`, `SpeedLap_HUD.xml`,
`SpeedLap_TimeTrial_HUD.xml`, `Zone_HUD.xml`), not the bare root set the
previous reading guessed at by analogy with HD's own skin shape. See
`docs/formats/2048-hud.md`.

## Next step: `FUN_81158d88` - `0x81158d88`

**Not renamed - unread, not just unnamed.** The single xref to every one of
the six `MissileSightBG`/`Outer`/`Inner`/`Middle`/`LockedOnLines`/
`LockedOnMiddle` strings (`search_strings("MissileSight")`, all six, one
address apiece) all land in this one function. It decompiles to 66 KB - by
far the largest function this thread has opened - and a keyword sweep of the
full decompile for `Always`/`always`/`Visible`/`visible`/`Show`/`Hide` finds
**none**, which reads as a widget name-to-slot registration table rather than
a per-frame visibility state machine.

This is the strongest lead anyone has found for `oag_2048::hud::ALWAYS_ON`,
which is otherwise empty and unread - see `docs/formats/2048-hud.md#what-is-not-done`.
It is not itself the answer: a registration table says a widget *exists*, not
when it draws. What it is worth next: finding what calls into or reads
whatever structure this function builds, on the theory that the actual
per-frame show/hide logic is a caller or a sibling this function's own
callers point at - `get_function_callers`/`get_function_callees` on this
address, not another read of this function itself.

[`oag_hd::hud`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/hud.rs
[`oag_2048::hud`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/2048/src/hud.rs
