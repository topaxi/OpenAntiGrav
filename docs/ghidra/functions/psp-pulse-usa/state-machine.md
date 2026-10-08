# The game state machine and the game-mode enum

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`, language `Allegrex:LE:32:default`.

**The names below are applied**, from [names.tsv](names.tsv) via
`just apply-names`. Per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md), applying a
rename needs a page carrying its evidence; this page is that evidence.
Nothing here is under 70, so nothing carries a `_q`.

[main-loop.md](main-loop.md) established the mechanism - a string-keyed,
hierarchical machine whose current state name sits in an inline buffer at
`machine+0x18c` - and listed the states it could find by string search, at
confidence 70 for the list. This page closes the M2 item by answering the
two questions that list left open: **where the states come from** (the
front end's are authored in XML and loaded on demand; only a dozen are
literals in code) and **what the separate integer "mode" at `0x08b31048`
is** (a 19-entry enum with its name table in `.data`, which several pages
had been decoding one value at a time).

## Summary

| Address | Name | Kind | Conf |
| --- | --- | --- | ---: |
| `0x08805628` | `GameMode_FromName` | function | 92 |
| `0x088e0784` | `MainMenu_Update` | function | 80 |
| `0x088e24fc` | `FrontEnd_TransitionTo` | function | 80 |
| `0x088df820` | `GriefReport_UpdateCancel` | function | 78 |
| `0x088df8b0` | `GriefReport_UpdateSend` | function | 78 |
| `0x08830990` | `AiBatchTest_Construct` | function | 80 |
| `0x088331ac` | `Demo_Construct` | function | 80 |
| `0x0882c108` | `ArcadeRace_Construct` | function | 82 |
| `0x0882e57c` | `Tournament_Construct` | function | 82 |
| `0x0882d9c0` | `TimeTrial_Construct` | function | 82 |
| `0x0882d1a0` | `FreePlay_Construct` | function | 82 |
| `0x0882c9b4` | `Elimination_Construct` | function | 82 |
| `0x08821f98` | `MPRace_Construct` | function | 80 |
| `0x08823614` | `MPTournament_Construct` | function | 80 |
| `0x08822dd8` | `MPTimeTrial_Construct` | function | 80 |
| `0x0882267c` | `MPElimination_Construct` | function | 80 |
| `0x08ab0788` | `g_game_mode_names` | data | 92 |
| `0x08b31048` | `g_game_mode` | data | 90 |
| `0x08ab07e3` | `g_debug_mode_override` (never written) | data | 85 |
| `0x08ab1f88` | `g_unlock_all_code` | data | 88 |
| `0x08ab1fbc` | `g_unlock_all_code_index` | data | 88 |

Already named and used here without a new row: `StateMachine_TransitionTo`
`0x0889123c`, `StateMachine_FindState` `0x08890ce4`, `g_state_machine`
`0x08b31784` ([main-loop.md](main-loop.md)); `StateMachine_LoadXml`
`0x08891448`, `TrackDefinition_EnterScreenState` `0x088c4410`
([race-box-screens.md](race-box-screens.md)); `StateMachine_EvaluateRedirect`
`0x088c8798` ([endrace-screens.md](endrace-screens.md));
`Race_CreateModeObject` `0x0882112c`, `Zone_Create`
([zone-mode.md](zone-mode.md)); `Race_ReadSetupOptions` `0x08896b84`
([shield.md](shield.md)).

## The machine and a transition

A machine is a node ([resource-loading.md](resource-loading.md)) whose
states are its children of the "state" class (tag `FUN_08a6bbc0`), chained
through each state's `+0xa4`. The fields a transition touches:

| Offset | Field |
| --- | --- |
| `machine+0x184` | root; a machine with none is inert |
| `machine+0x18c` | current state **name**, inline `char[]` - the buffer [main-loop.md](main-loop.md) read live |
| `machine+0x1d0` | current state object |
| `machine+0x1e8` | the name most recently *requested* |
| `state+0x94` | redirect target name |
| `state+0xd0` | flags: bit 0 = this state is a redirect, follow `+0x94`; bit 3 = hidden from the first lookup pass |

`StateMachine_TransitionTo(machine, name, context)` (`0x0889123c`) returns
the current state object, and does nothing when `name` already equals the
current name. Otherwise: find the target (`StateMachine_FindState`, which
walks the chain twice - first skipping hidden states, then including them);
give up with 0 if it is not there; call the old state's **exit** virtual
(vtable `+0x9c`, with `context`); follow redirects (while the target's bit 0
is set, look up its `+0x94` instead - the `Redirect`/`goto` elements
[fexml.md](../../../formats/fexml.md) lists); `FUN_08890248(new, context)`;
the new state's **enter** virtual (`+0x74`); the old state's **post-exit**
virtual (`+0x7c`); then copy the resolved name into `+0x18c` and cache the
object at `+0x1d0`. `context` is almost always `machine + 0x18c` itself -
the previous name, so a state can read where it came from.

**There is more than one machine.** `g_state_machine` is the front-end
root, but every screen definition owns one (`definition+0x168` in
`TrackDefinition_EnterScreenState`), and nodes reached through `+0xa0` /
`+0xa4` in the lobby and options code are machines too. The `+0x18c` buffer
convention is the same for all of them.

## Where the states come from

**The front end's states are data.** `StateMachine_LoadXml(machine, path,
parent_state, flag)` (`0x08891448`) opens an XML document, allocates a
0xd8-byte container node under `parent_state` (or the machine), and for
every root element calls `FUN_0889165c(machine, element, container)` to
build states from it. `TrackDefinition_EnterScreenState` shows the shape
every screen uses: on the first request for a state the definition does
not yet have, it loads `<location>\screen.xml` - or `screen_zone.xml` when
the definition is Zone-capable and the mode is Zone, or `MP_Screen.xml`
when asked for the multiplayer variant - and *then* transitions. So the
state graph of the menus is [the disc's own XML](../../../ui/menus-original.md),
loaded lazily per screen, and a full enumeration of it is an asset read,
not a binary one. Of the 31 callers of `StateMachine_TransitionTo`, 16
pass a name read from a widget or definition field (`+0x94`, `+0xa4`,
`+0x223`, `+0x5c`, `+0x40`, `+0xdc`, `+0x1a0`, `+0x1e0`) rather than a
literal.

**The literal states**, with what triggers each - this is the complete set
of string constants passed to the transition anywhere in the binary:

| State name | Caller | Trigger |
| --- | --- | --- |
| `Language Selection` / `Launch Game` | `Game_MainLoop` | boot: the first when `g_debug_mode_override` is 0, the second otherwise |
| `Launch Game` | `FUN_088abf80` | the network/sync path re-entering the launch |
| `InGame Pause` | `InGame_UpdatePauseInput` | a press on index `0x10` (the pause button [main-loop.md](main-loop.md) reads as START) while in `InGame`; local modes only (`g_game_mode < 0xe`) |
| `InGame` | `InGame_UpdatePauseInput` | a second edge test (`FUN_0894f258`) on the same index while in `InGame Pause` |
| `InGame` | `FUN_08829e6c` (three sites) | the race-start countdown's own nested machine handing the race over |
| `Reset Demo` | `Demo_UpdateAttractMode` | the 120-second attract timeout |
| `Demo Launch` | `MainMenu_Update` | 60 s without a press while on `Main Menu` |
| `Show Unlock All` | `MainMenu_Update` | the unlock code below |
| `SyncPopUp` | seven functions `0x088ad998`-`0x088aff40` | ad hoc session sync dialogs |
| `Sync Data` | `FUN_088aec5c` | ad hoc data sync |
| `GriefReport Cancel` / `Success` / `Failure` | `GriefReport_UpdateCancel` / `GriefReport_UpdateSend` | the online grief-report dialog, 10 s after submit |
| `EnterGamePassword` (context `Game List`) | `FUN_088a0ad4` | joining a password-protected online game |
| `FilterList` | `FUN_088a0ad4` via `FrontEnd_TransitionTo` | button 7 (square) on the game list |
| `""` (the empty name, at `0x08a83878`, `0x08a7ff10`, `0x08a795e0`) | `FUN_088e0f90`, `FUN_088b3428`, `InGame_Construct` | teardown of a sub-machine: `FindState("")` fails, so the call returns 0 without running any enter, and only the request field `+0x1e8` changes - Ghidra left these untyped because a one-byte string has no body |

`FrontEnd_TransitionTo(name, context)` (`0x088e24fc`) is the wrapper the
online screens use: a transition on `g_state_machine` followed by a call to
`FUN_08965630`, which zeroes two fields of a singleton (`*p = 0`, `*(p + 4) = 0`)
and reaches no `Sound_*` or `Scream_*` function
([menu-sounds.md](menu-sounds.md)). An earlier reading of this paragraph called it a
front-end sound; it is not.

### The unlock-everything code

`MainMenu_Update` (`0x088e0784`) carries a cheat. While the current state of
the screen's machine is `Main Menu`, unlocks are not already granted
(`DAT_08b31774+0x45f` is 0 - or `sceKernelGetGPI() & 1`, the devkit's GPI
switch, which forces it), and any button (index `0x14`) is pressed this
frame: if the pressed button matches `g_unlock_all_code[g_unlock_all_code_index]`
the index advances, otherwise it resets to 0; when the table reaches its
`-1` terminator the flag byte is set, a `Show Unlocks Root->Dialog` with
`MSC_UNL_ALL` and `FE_CONTINUE` is built, and the machine transitions to
`Show Unlock All`. The table at `0x08ab1f88`, in the
[abstract button indices](input.md):

```
2, 3, 2, 8, 9, 8, 3, 2, 3, 9, 8, 9, -1
Left, Right, Left, L, R, L, Right, Left, Right, R, L, R
```

Confidence 88 for the table and the gate; the code was read, not entered.
`MainMenu_Update` is otherwise the idle handler: it also fades the seven
`helptext%d` widgets toward the focused entry (`+0x18` per frame, clamped
`0`..`0xff`) and, after 60 s with no press on this screen, sends
`Demo Launch` to the machine and clears the press.

## The game-mode enum: `g_game_mode` (`0x08b31048`)

`DAT_08b31048` is `*(0x08b30f90 + 0xb8)` in the global game-state block
([pads.md](pads.md)), compared against literals on a dozen pages. It is an
index into **`g_game_mode_names`** (`0x08ab0788`), 19 string pointers that
`GameMode_FromName(game, name)` (`0x08805628`) searches with `strcasecmp`,
returning the index or -1. `Race_ReadSetupOptions` reads the front end's
`Mode` global, resolves it through this function and stores it, which is
the only write path - the value is what the menu XML named.

| Index | Name | `Race_CreateModeObject` constructor | Object size |
| ---: | --- | --- | ---: |
| 0 | `Debug` | (no case) | |
| 1 | `MODE_AI_BATCH_TEST` | `AiBatchTest_Construct` `0x08830990` (`TimeTrial_HUD.xml`) | `0x1a10` |
| 2 | `Demo` | `Demo_Construct` `0x088331ac` (`Arcade_HUD.xml`) | `0x1a10` |
| 3 | `Arcade` | `ArcadeRace_Construct` `0x0882c108` (`Arcade_HUD.xml`) | `0x1a10` |
| 4 | `Tournament` | `Tournament_Construct` `0x0882e57c` (`Arcade_HUD.xml`) | `0x1a2c` |
| 5 | `Time Trial` | `TimeTrial_Construct` `0x0882d9c0` (`TimeTrial_HUD.xml`) | `0x1a10` |
| 6 | `Zone` | `Zone_Create` `0x0882eee4` (`Zone_HUD.xml`) | `0x1a30` |
| 7 | `Free Play` | `FreePlay_Construct` `0x0882d1a0` (`TimeTrial_HUD.xml`) | `0x1a10` |
| 8 | `Elimination` | `Elimination_Construct` `0x0882c9b4` (`Elimination_HUD.xml`) | `0x1a14` |
| 9 | `Head2Head` | `ArcadeRace_Construct` (shared with 3) | `0x1a10` |
| 10 | `Speed Lap` | `TimeTrial_Construct` (shared with 5) | `0x1a10` |
| 11 | `Custom Grid` | (no case) | |
| 12 | `AI Race` | `ArcadeRace_Construct` (shared with 3) | `0x1a10` |
| 13 | `Multiplayer` | (no case - the lobby, not a race) | |
| 14 | `Multiplayer Single Race` | `MPRace_Construct` `0x08821f98` (`Arcade_HUD.xml` / `MPTag_HUD.xml`) | `0x1aa4` |
| 15 | `Multiplayer Head2Head` | `MPRace_Construct` (shared with 14) | `0x1aa4` |
| 16 | `Multiplayer Tournament` | `MPTournament_Construct` `0x08823614` (`Arcade_HUD.xml` / `MPTag_HUD.xml`) | `0x1afc` |
| 17 | `Multiplayer Time Trial` | `MPTimeTrial_Construct` `0x08822dd8` (`Arcade_HUD.xml` / `MPTag_HUD.xml`) | `0x1aa4` |
| 18 | `Multiplayer Elimination` | `MPElimination_Construct` `0x0882267c` (`Elimination_HUD.xml`) | `0x1aac` |

The constructors are named from the case that reaches them and the HUD
file each loads (`search_strings "HUD.xml"` gives fifteen copies, one xref
each, all inside these twelve functions), which is why they sit at 80-82
rather than higher: the case label is unambiguous, the internals are not
read. `Race_CreateModeObject` builds two objects before the switch - a
`0x6d0`-byte one (`FUN_0887a2a0`) and a `0xc80`-byte one (`FUN_08886af4`)
- as the mode object's parents, then `FUN_08887470` on the result.

`g_debug_mode_override` (`0x08ab07e3`) is a byte with 100+ readers and **no
writer**; every reader evaluates `override ? 0 : g_game_mode`, so setting it
would make the whole game behave as mode 0, `Debug`. It is the same shape
as the three never-written `g_force_fixed_*` bytes on
[main-loop.md](main-loop.md). What `Debug` mode would do is unknown; no
`case 0` exists in the factory.

This resolves several per-page notes: [pads.md](pads.md)'s "mode `2` is a
second, unidentified mode" is **Demo** (the attract-mode race, which is why
pads there behave differently); [missile.md](missile.md)'s `3` is `Arcade`
(a single race); [tournament.md](tournament.md)'s `4` is `Tournament`, and
its "single-player family, modes 3..12" / "split-screen family, modes >=
14" split is exactly the `Multiplayer *` block; the `0xe` threshold in
`InGame_UpdatePauseInput`, `FUN_08821bd4` ([prng.md](prng.md)) and
`Heap_DumpBlocks` ([memory.md](memory.md)) is "is this a network game".

### What `Race_ReadSetupOptions` derives from the mode

Read straight off the switches in `0x08896b84`, for a reimplementation to
match:

| Setting | Modes | Value |
| --- | --- | --- |
| Grid size (`FUN_0880502c`) | 3, 4, 8, 12, 14, 16, 18 | 8 crafts (then `Opponents + 1` if the XML sets `Opponents` and the grid read back as 8) |
| | 9 (`Head2Head`) | 2 |
| | everything else | 1 |
| Default laps (`FUN_08805058`, when the XML `Laps` is empty) | 6, 7, 8, 18 | 0 (untimed / mode-driven) |
| | 10 (`Speed Lap`) | 7 |
| | everything else | 3 / 4 / 5 by speed class `g_class` 0 / 1-2 / 3 |
| `g_weapons_enabled` default | 5, 9, 10, 15, 17 | off (and the XML `Weapons` toggle is ignored for 5, 8, 9, 10, 15, 17) |
| | others | on, XML `Weapons` may turn it off |
| `g_damage_enabled` default | 5, 7, 10 | off (XML `Damage` ignored for 6, 8, 10) |
| | others | on |

`g_class` is `DAT_08b31040` (`*(0x08b30f90 + 0xb0)`), the speed-class index
[pads.md](pads.md) identified.

## Cross-platform

Not carried to PS2 or Pure on this pass. The 19-name table is the search
key: `"MODE_AI_BATCH_TEST"` is distinctive enough for `search_strings`.

## Open questions

- `FUN_0889165c`, the XML-element-to-state builder: how a `Screen` element
  becomes a state object, what sets `+0xd0` bit 0 (`Redirect`) and bit 3.
  Reading it would make the menu graph derivable from the XML by a script
  without opening Ghidra.
- Modes 0, 11 and 13 have no factory case: `Custom Grid` presumably reuses
  another mode's object through a path not on this page; `Debug` may have
  none.

## History

- 2026-09-16: first reading, static. The mode table is read from `.data`
  at the address `GameMode_FromName` iterates; the literal-state table is
  from a regex over the decompilation of all 31 transition callers.
