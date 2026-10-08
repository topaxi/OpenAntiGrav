# Quitting a campaign race: where the pause menu's QUIT RACE lands (2026-10-08, pulse-h2h)

**Binary:** `pulse-psp` `BOOT.BIN` (USA), image base `0x08804000`. PPSSPP v1.20.4
(SDL, OpenGL software renderer), debugger on `:45497`, fresh profile, Xvfb.
Screenshots under `data/scratch/pulse-h2h/` (gitignored).

## The question

Our Escape in a campaign race opened the main menu. Where does the original put a
player who pauses a campaign race and chooses QUIT RACE?

## Authored road (decompiled XML, not a guess)

Every pause menu (`InGame Pause SP`, `... SP Time Trial`, `... SP Speed Lap`,
`... SP Tournament`) sends `IG_PAUSE_QUIT` to `Kill Game Transition`
(`InGame_Definition.xml`, `Data.wad` entry `Data\Plugins\PI001\GUI\InGame_Definition.xml`).
`Kill Game Transition` redirects to `Kill Game`, whose redirect goes to `Show Unlocks`
(entry 1091 and `InGame_Definition.xml`). `Show Unlocks` (entry 1104) carries two
redirects, both `StartEnabled="false"`:

| Redirect | Entries | Default |
| --- | --- | --- |
| `Main Menu Redirect` | `Main Menu->Mode == FE_MP` -> `GameLobby`; `== FE_RACE_CAM` -> `Cell Selection` (`previous="false"`); `Racebox->RBMode == RB_LOAD_GRID` -> `Cell Selection` | `Main Menu` |
| `Tournament Hack Redirect` | `FE_MP` -> `GameLobby`; `RB_LOAD_GRID` -> `Cell Selection` | `Main Menu` |

## `ShowUnlocks_BindRedirect` (`0x088e92f8`), confidence 75

The screen's enter handler picks which redirect is live. The two strings
`"Main Menu Redirect"` (`0x08a844b0`, referenced at `0x088e9364`) and
`"Tournament Hack Redirect"` (`0x08a84494`, referenced at `0x088e9344`) each have this
one reference. `FUN_088099b4(DAT_08b31774, 0, ..) == 0` selects `Main Menu Redirect`,
nonzero selects `Tournament Hack Redirect`. What `FUN_088099b4` tests is unread, so
which redirect a given quit takes is not decided statically; the live walk below is
what settles the destination. Name confidence 75: the two strings, the call and the
store into `+0xdc` are read directly, the predicate is not.

## Live walk (PPSSPP, 2026-10-08), confidence 85

| Cell (mode) | Launched from | After pause -> QUIT RACE |
| --- | --- | --- |
| `grid0` default cell, Single Race | `Cell Selection`, cursor on it | `Cell Selection`, cursor on the same default cell (`q2.png`) |
| `grid4_5_2`, Head to Head, Flash, Weapons Off, 4 laps (dev-unlock byte at `*(0x08b31774)+0x45f` set so grid 5 opens) | `Cell Selection` | `Cell Selection`, cursor on the grid's **default** cell (`grid4`'s Single Race, "The Amphiseum White"), **not** on the Head2Head cell (`hq.png`) |

So QUIT RACE lands on `Cell Selection` for a campaign launch, and the cursor is reset to
the screen's default cell. The earlier post-race capture
(`docs/ui/campaign-screens.md`, `back-to-cellselect-postrace.png`) shows the cursor on
the raced cell after `EndRace` -> `RETURN TO GRID`: the two paths differ in the
cursor, as measured. Time Trial, Speed Lap, Zone, Elimination and Tournament were not
walked; their pause menus carry the same `IG_PAUSE_QUIT` -> `Kill Game Transition`
entry, so the destination follows by the same data (confidence 75 for those).

## What this build does

`Session::escape` in a campaign race (any mode, Pulse and HD/Fury through the shared
code) records the result, discards the race and opens `Cell Selection` on the cell's
grid. On Pulse the cursor is the default cell (measured); HD and Fury keep the raced
cell under the cursor (**chosen, not measured**: nothing on HD was walked). There is no
pause menu in this build, so escape stands for QUIT RACE and a campaign race cannot be
resumed from the menus (**chosen, not measured**). Our own walk reaches the same screen
(`o11.png`).

**2048 / Omega:** not checkable here. 2048's pause is a different screen set and Omega
launches through HD's front end; neither executable was read for a quit chain.

## Names landed

| Address | Name | Confidence |
| --- | --- | --- |
| `0x088e92f8` | `ShowUnlocks_BindRedirect` | 75 |
