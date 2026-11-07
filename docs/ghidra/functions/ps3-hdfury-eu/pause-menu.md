# The pause redirect: which pause screen a game mode opens (HD/Fury, PS3)

Read 2026-11-07, lane `pause-menu-2`, `/hdfury/EBOOT-ps3-hdfury-eu.elf`, TOC `r2 = 0x008ad4d8`
(the slot of `InGame Pause RedirectController` is `r2+0x3d1c = 0x008b11f4`). The Pulse counterpart is
[`psp-pulse-usa/pause-menu.md`](../psp-pulse-usa/pause-menu.md); what the screens show and what this
build does with them: [the pause menu](../../../ui/pause-menu.md).

| Address | Name | Conf. | What it is |
| --- | --- | ---: | --- |
| `0x0023d660` | `PauseMenu_BindRedirectForMode` | 85 | Finds the `InGame Pause RedirectController` redirect (`0x0016a920`) and writes the pause screen's name into its `+0x1f0` (a `0x40` byte buffer), with `+0x230 = 1`; clears the buffer when the mode has no override |

## The switch

`g_GameState` is `*0x008b11fc`, the mode `+0xe0` and the view count `+0xe4`. While the debug byte
`*0x008b11f8` is clear, the mode minus 4 indexes an 18-entry relative jump table at `0x0023d808`
(modes 4 to 21); a mode outside it, or the debug byte set, goes to the view-count branch only. Modes
4, 13, 15 to 21 read the view count: **one view takes the single-view screen, two or more take a
split screen** (`Split Player 1` for the first pad, `Player 2` for the second, or the mode's own
`... P1` / `... P2` pair). **Modes 5, 10, 6 and 14 write their screen for any view count**
(`0x0023dcb8`, `0x0023dbd4`, `0x0023da58` load the slot with no test), and every other mode
with two or more views takes the `Split Player` pair. The TOC slots, read from `0x008b1204` on:

| Slot (`r2+`) | String |
| --- | --- |
| `0x3d2c` / `0x3d30` | `InGame Pause Tournament Split Player 2` / `Player 1` |
| `0x3d34`, `0x3d38`, `0x3d3c` | `InGame Pause SP Tournament`, `SP Speed Lap`, `SP Time Trial` |
| `0x3d40`, `0x3d44`, `0x3d48` | `InGame Pause Zone`, `Zone P2`, `Zone P1` |
| `0x3d4c`, `0x3d50`, `0x3d54` | `InGame Pause MP Player 2`, `MP`, `MP Tournament` |
| `0x3d58` / `0x3d5c` | `InGame Pause Split Player 2` / `Player 1` |

The jump table's targets and what they load for one view:

| `g_GameState+0xe0` | Class (`mode-manager.md`) | Single view |
| ---: | --- | --- |
| 4 | `SPTournament` | `InGame Pause SP Tournament` |
| 5 | `SPTimeTrial` | `InGame Pause SP Time Trial` |
| 10 | `SPTimeTrial`'s class (Speed Lap) | `InGame Pause SP Speed Lap` |
| 6, 14 | no `ModeManager` (Zone and its kin) | `InGame Pause Zone` |
| 13 | no `ModeManager` | `InGame Pause Zone P1` for one view as well (`0x0023ddac`); unread which mode |
| 15, 16, 18 to 21 | `MPArcade`, `MPTimeTrial`, `MPElimination` | `InGame Pause MP` |
| 17 | `MPTournament` | reads the view count, then `InGame Pause MP Tournament` (`0x0023de58`) |
| 3, 7, 8, 9, 11, 12 | `SPArcade`, `SPElimination`, the rest | nothing written: the buffer is cleared, `+0x230 = 0`, and the redirect's authored `Default goto="InGame Pause SP"` stands |

So **Single Race and Eliminator (8) take `InGame Pause SP`**, Time Trial, Speed Lap, Tournament and
Zone their own. Pulse's binder has the same shape (Tournament 4, Time Trial 5, Speed Lap 10) with no
Zone screen and no split-screen branch.

## Live (RPCS3, Fury EU, Talon's Junction, 2026-11-07, 90)

START in a race read the state name off `TTY.log` (`Switching Screen`): `InGame Pause SP Time Trial`
(Racebox Time Trial) and `InGame Pause SP Speed Lap` (Racebox Speed Lap); the Single Race name,
`InGame Pause SP`, is `photo-mode.md`'s run. **Zone and Tournament were not walked live** (the
static reading, 85).

What the screens drew on RPCS3 (1280x720 inside the 2000x1200 window, frames kept in the lane's
scratch): a vertical list at `x = 480` of 1920, 50 apart from `y = 310`, the title `GAME PAUSED`
with its underline, the focused row `HD_Blue`, `FE_PILOT_ASSIST` and `FE_GHOST` drawn as on/off
lists with their value beside them (`OFF`, `ON`), **`ONL_VIEW_INV` drawn greyed (offline) and
skipped by the cursor**, a footer of `RESUME GAME` (START) and `CONFIRM`, the whole frame behind it
darkened to **0.25 of its brightness** (the same 270x270 patches before and after: 0.282 to
0.070, 0.285 to 0.070, 0.311 to 0.077, so a black overlay of alpha 0.75; **the race HUD stays
drawn, dimmed, under it**). The Speed Lap screen on the disc has no RESTART and no QUIT:
`IG_PAUSE_ENDSES` is its last row, and **END SESSION went to `Race End Photo`** (CONTINUE and PHOTO
MODE), then `EndRace Results` and `EndRace Menu`: it finishes the race, not abandons it.

## Which archive's copy

`ingame_definition.xml` is in five archives and the pause screens differ: `DATA00` and `DATA06` carry
the Fury file (`FE_GHOST` on the Time Trial and Speed Lap screens, `ONL_VIEW_INV` on all), `DATA05`
the same without `FE_GHOST`, and the base game's `DATA02` and `DATA03` neither, with a Speed Lap
screen that also has RESTART and QUIT and `IG_PAUSE_ENDSES` second. The game reads one
(`Archives::read_name`: the Fury file); the table follows it and the test
(`hds_pause_screens_are_the_five_single_player_widget_lists`) checks the other four copies
against it minus the two rows.

## Widgets, not entries

HD authors a screen's rows as sibling `Block` and `List` widgets in document order, each with its
own `y`, plus a `Redirect` of `Entry item="<Block name>" goto=...`. Pulse's `Option` menu of
`Entry` rows does not exist here, and the `aVertMenu name="Option"` and `aBlock name="Skip"` on
the Time Trial, Tournament and MP screens are commented out by the `a` prefix: they are not rows
(the Tournament one lists `IG_PAUSE_SKIP`, which the live widget list does not have).

## History

- 2026-11-07: written, the switch and three live names.
