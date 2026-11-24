# The pause screen's music list (Pulse, PSP)

Addresses are `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`. Read 2026-11-24 (lane
`pulse-pause-music-re`) with the Ghidra decompiler, then watched on PPSSPP 1.20.4 (software renderer, muted,
private profile, USA disc) from `data/saves/pulse-psp/time-trial-moa-therma-white-venom-grid.ppst`: START, then
pad presses with a breakpoint on `Sound_PlayNamedInSlot` (`0x0893a768`, the cue and its `ra`), a breakpoint on
`MusicPlayer_StartNextSong`, the music player's fields read between presses, and a 480x272 grab after each press.

This page **corrects** the first reading (2026-11-23, `pulse-pause-music`), which had LEFT and RIGHT going
through the focus step `0x08890874` with the `LEFTRIGHT` cue. Those two calls are UP and DOWN (button indices 0
and 1, `input.md`), and the music list never runs them; LEFT and RIGHT are the value step below, with `UPDOWN`.

| Address | Name | Conf | What it is |
| --- | --- | ---: | --- |
| `0x088bb4a0` | `MusicList_RegisterClass` | 85 | Registers the `MusicList` widget class (`0x08a80670`) with the vtable `0x08acdc6c` |
| `0x088bb0d4` | `MusicList_Construct` | 80 | `ListWidget_Construct`, then the `MusicList` vtable |
| `0x088bb264` | `MusicList_Update` | 80 | Rebuilds the entries when the music set reports a change, then `ListWidget_Update` |
| `0x088bb358` | `MusicList_RebuildEntries` | 85 | One entry per music node (the song's name), the selection set to the player's song index |
| `0x088bb2c0` | `MusicList_Enter` | 80 | The list's enter: `ListWidget_Enter`, `+0xd0 = 1` (works without focus), `MusicList_RebuildEntries` |
| `0x088bb2f4` | `MusicList_Leave` | 90 | The list's leave: starts the selected song when it is not the playing one and the game is not paused |
| `0x088a9520` | `ListWidget_Construct` | 80 | The `List` defaults: the four step buttons, the arrow offsets and sizes, no wrap, no auto-repeat |
| `0x088aa580` | `ListWidget_Enter` | 75 | Makes the two arrow images off `Data\FE\Images\pulse_assets.mip` and places them |
| `0x088aaae4` | `ListWidget_Update` | 80 | Per update: the selected entry's flags, the focus step (UP/DOWN), the value step (LEFT/RIGHT), the arrows |
| `0x088a9db4` | `ListWidget_StepBack` | 90 | Selection - 1 with `UPDOWN`; at 0, `DECLINE` and no change, or the last entry when `Wrap` |
| `0x088a9e7c` | `ListWidget_StepForward` | 90 | Selection + 1 with `UPDOWN`; at the last, `DECLINE` and no change, or 0 when `Wrap` |
| `0x088b528c` | `MenuWidget_StepCursor` | 85 | The `<Menu>` widget's cursor step: wraps both ways, skips a disabled entry, plays `UPDOWN` |
| `0x08ab0628` | `g_game_paused` | 85 | 1 while a single-player race is paused (data) |

## The class

`MusicList_RegisterClass` registers `MusicList` with `0x08acdc6c`, whose slots a `List` (`0x08acd194`) has, with
these overridden: the update (`+0x24`, `MusicList_Update`), the destructor (`0x088bb11c`), the enter (`+0x74`,
`MusicList_Enter`), the leave (`+0x7c`, `MusicList_Leave`) and the factory (`0x088bb178`). The authoring is the
four single-player pause screens' (`InGame_Definition.xml`, both PSP pressings):

```xml
<MusicList name="Music List" focus="true" transition="0">
<Values x="FEGlobals->MenuXOffset" y="220" font="small" color="FEGlobals->TextColor" arrowcolor="FEGlobals->TextColor" gap="0"></Values>
</MusicList>
```

`ListWidget_ReadElement` (`0x088a9f3c`, `race-box-screens.md`) reads the `Values` of any list: `X`, `Y`,
`Color` (`+0xa4`), `ArrowColor` (`+0xa8`), `Scale` (`+0xb4`), `Wrap` (`0x08a7f3c8`, `+0x102`), `Font` (`+0xb8`),
`DisableEntriesBitField`, `GSDisableEntriesBitField`, `Align`, `VertAlign`, `Allocate`, `workWhenNotFocus` (`+0xd0`),
`swapbuttons`, `PulseMultiplier` (`+0x104`) and `AutoRepeat` (`+0x108`). The music list authors no `Wrap`, no
`AutoRepeat` and no `swapbuttons`, so the constructor's defaults stand.

## The entries (`MusicList_RebuildEntries`)

The same node list `MusicPlayer_StartNextSong` indexes (`FUN_08889ee8`), one entry per node whose `+0xd4` virtual
answers non-zero, worded with the node's `+0x74` (the song's `name`; through the string table when `+0x2a4` is
set), **no artist**. The selection (`+0x98`) is the music player's song index (`player+0x48`), clamped to the
entries. Live: 16 entries, from `Fenix Funk 5 (Wipeout Edit)` at 0 (the player's `+0x48 = 0`, the playing song)
through `Steady Rush` at 1 to `Suspicious Thoughts` at 15, drawn in capitals by the `small` face.

## The step (`ListWidget_Update`)

The constructor sets the four step buttons to the input layer's indices: `+0xbc = 0` (UP), `+0xc0 = 1` (DOWN),
`+0xc4 = 2` (LEFT), `+0xc8 = 3` (RIGHT) (`input.md`'s table); `swapbuttons` would exchange the two pairs. The
update runs the step only while the list holds the focus flag (`+0x2c & 0x200`) **or** `+0xd0` is set, and
`MusicList_Enter` sets `+0xd0`: so LEFT and RIGHT step the music list **whichever `Option` row the cursor is on**.
The UP/DOWN focus step (`0x08890874`, `LEFTRIGHT`) runs only when `+0xd0` is clear, never here.

| Press | Code | Cue (`ra`, live) | Selection |
| --- | --- | --- | --- |
| RIGHT, not at the last | `ListWidget_StepForward` | `UPDOWN` (`0x088a9ec8`) | + 1 |
| RIGHT at the last (15) | `ListWidget_StepForward` | `DECLINE` (`0x088a9f30`) | unchanged: no wrap |
| LEFT, not at 0 | `ListWidget_StepBack` | `UPDOWN` (`0x088a9e04`) | - 1 |
| LEFT at 0 | `ListWidget_StepBack` | `DECLINE` (`0x088a9e70`) | unchanged: no wrap |
| UP/DOWN | `MenuWidget_StepCursor` | `UPDOWN` (`0x088b5340`) | unchanged; the `Option` cursor moves (and wraps) |

Live, from the state: RIGHT read `UPDOWN` and the row `STEADY RUSH`; LEFT `UPDOWN` and `FENIX FUNK 5 (WIPEOUT
EDIT)`; LEFT again `DECLINE` and the same row; 24 RIGHTs read 15 `UPDOWN` then 9 `DECLINE`, the row at
`SUSPICIOUS THOUGHTS`. A press is one step: `AutoRepeat` is unset (`+0x108 = 0`), so the held branch never runs.
The cursor never lands on the list: DOWN from QUIT RACE plays the `<Menu>`'s own `UPDOWN` and wraps over its entries.
Confidence 90 for the step, the ends and the cues (decompile plus the live `ra`s).

## While paused nothing plays

Every step above left the music player as it was: `player+0x48 = 0`, `+0x3c0 = -1`, the state `+0x3c = 1`
and the strings `Aphex Twin` / `Fenix Funk 5 (Wipeout Edit)` read the same after each press. **No audition**
(90).

## Resume starts the pick (`MusicList_Leave`)

```
if (list+0x98 != player+0x48 && g_game_paused == 0)
    MusicPlayer_StartNextSong(player, list+0x98);
ListWidget_Leave(list);
```

`g_game_paused` (`0x08ab0628`) is written by the in-game redirect handler `0x0889683c`: 1 when it hides the race
for the pause (single-player modes), 0 when it shows it again and when it quits; its other readers are the
per-frame effects that freeze while paused (camera, trails, particles, screen flash). Live it read 1 on the pause
screen and **0 at the leave's breakpoint after START**, so the redirect back to `InGame` clears it before the screen's
widgets leave.

Live, RIGHT then START: one hit of `MusicPlayer_StartNextSong` with `a0 = player`, `a1 = 1`, `ra = 0x088bb338`
(inside `MusicList_Leave`), then the player read `+0x48 = 1`, `Booka Shade` / `Steady Rush`, and the next frames
drew the now-playing marquee `BOOKA SHADE - STEADY RUSH` entering (`now-playing.md`). In a race
(`race_manager+0x7c8 > 1`) `MusicPlayer_StartNextSong` with a pick in range loads that song from its start and
sets `+0x48` to it, so **the pick plays from the beginning**, and the next race's `StartNextSong(-1)` takes the
song after it. RIGHT then LEFT (back to the playing song) does not call it: the playing song resumes where it was
(static, the `!=`; 85).

**GAME OPTIONS and back drops a pick** (live): RIGHT to `STEADY RUSH`, CROSS on GAME OPTIONS (`InGame Settings TT`),
CIRCLE back: the row reads `FENIX FUNK 5 (WIPEOUT EDIT)` again and the player is untouched (`+0x48 = 0`). The leave
ran with `g_game_paused` still 1, and `MusicList_Enter` rebuilt the selection from `+0x48`. RESTART RACE and QUIT
RACE leave the screen paused too (QUIT's redirect clears the flag after its state change, `0x0889683c`), so a pick
there is dropped: static reading, 70.

## The draw

- **Text.** The selected entry alone, `font="small"`, `color="FEGlobals->TextColor"`, at `x = FEGlobals->MenuXOffset`,
  `y = 220`, left aligned (`Align` unset, `+0xac = 0`).
- **Arrows.** `ListWidget_Enter` makes two images off `Data\FE\Images\pulse_assets.mip`: the left arrow at UV
  `(282, 68)` (`+0xf0`) and the right at `(306, 68)` (`+0xf4`), each 25 x 28 (`+0xd4`..`+0xe0`), coloured
  `ArrowColor`, placed at the text's x `- 32` and `- 20` (`+0xe4`, `+0xe8`), so the two halves of a pill sit left of
  the text; `ListWidget_Update` moves the UV's `v` to 96 when no focus-step press was seen this update (always, for
  this list). Live, the left half is dark at entry 0 and the right half dark at entry 15, both lit between;
  `0x088a9ba4` (the end-dim rule, from the frames) is not read. Confidence 80 for the geometry, 85 for the dimming
  from the frames.

## PS2 and Pure

See `docs/ui/pause-menu.md`, "Pulse".
