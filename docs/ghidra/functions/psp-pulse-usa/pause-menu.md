# The pause redirect: which pause screen a game mode opens (Pulse, PSP)

Addresses are `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`. Read 2026-11-07 (lane
`pause-menu`) with the Ghidra decompiler, string xrefs on the screen names, and a live PPSSPP 1.20.4
run (software renderer, muted, private profile) that read the front-end state name after START.

| Address | Name | Conf | What it is |
| --- | --- | ---: | --- |
| `0x088e35d4` | `PauseMenu_BindRedirectForMode` | 85 | Finds the `InGame Pause RedirectController` redirect and writes into its `+0x1e0` the name of the pause screen the game mode opens, with `+0x220 = 1` |

## `PauseMenu_BindRedirectForMode`

The three screen-name strings each have this one reference (`0x08a83bf8` `InGame Pause SP Tournament` at
`0x088e3664`, `0x08a83c14` `... SP Speed Lap`, `0x08a83c30` `... SP Time Trial`, and `0x08a83c4c`
`InGame Pause MP` at `0x088e36a4` and `0x088e36b4`). The switch is on `g_game_mode` (`0x08b31048`),
read as `0` while `g_debug_mode_override` is set (it is never written):

| `g_game_mode` | Mode name (`g_game_mode_names`) | Screen written |
| ---: | --- | --- |
| 4 | `Tournament` | `InGame Pause SP Tournament` |
| 5 | `Time Trial` | `InGame Pause SP Time Trial` |
| 10 | `Speed Lap` | `InGame Pause SP Speed Lap` |
| 13 to 18 | the multiplayer modes | `InGame Pause MP`; also sets the `Option` menu's `+0xf8` to 8, or 0 when `FUN_088df720` answers non-zero |
| anything else | `Arcade`, `Head2Head`, `Zone`, `Elimination`, ... | nothing written (the buffer is cleared, `+0x220 = 0`): the redirect's authored `Default goto="InGame Pause SP"` stands |

`InGame Pause MP Tournament Host` is not named here; what picks it is unread.

**Live (PPSSPP, Pulse USA, 90).** START in a race read the state name `InGame Pause SP Time Trial`
(Time Trial, `menu --race-type 2`), `InGame Pause SP Speed Lap` (Speed Lap, type 3) and
`InGame Pause SP` (Single Race, type 0). Tournament was not walked live: the Racebox flow lands on
`Tournament C`, the tournament setup, and building a tournament was not done; its screen is the
static reading (conf 85).

What each screen shows and what this build does with it:
[the pause menu](../../../ui/pause-menu.md).
