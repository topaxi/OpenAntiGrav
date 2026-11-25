# The pause redirect: which pause screen a game mode opens (2048, Vita)

Read 2026-11-25, lane `pause-menu-per-mode`, `/2048/eboot-vita-2048-eu-v104.elf` (image base
`0x81000000`). The counterparts: [HD](../ps3-hdfury-eu/pause-menu.md), [Omega](../ps4-omega-eu/pause-menu.md);
what the screens show and what this build does with them: [the pause menu](../../../ui/pause-menu.md), "2048".

| Address | Name | Conf. | What it is |
| --- | --- | ---: | --- |
| `0x8112e050` | `PauseMenu_BindRedirectForMode` | 80 | Finds `InGame Pause RedirectController` (`FUN_81067faa`) and writes the pause screen's name into its `+0x1f4` (`+0x234 = 1`), or clears the 64-byte buffer (`+0x234 = 0`) |

Its two string xrefs are the redirect's name (`0x8146ee10`, `0x8112e05e`) and `PauseMenuTournament`
(`0x8146ee30`, `0x8112e0ae`); `PauseMenuSpeed` (`0x8146ee44`) and `PauseMenuOnline` (`0x8146ee54`) are named in
the same function. `PauseMenuRace` is in no string of the executable: it is the redirect's authored default
(`PauseMenuRedirect`'s `<Default goto="PauseMenuRace">`), which is what a cleared buffer leaves.

## The switch

On the game mode `DAT_8153fd24` (the word `Ship_LoadModelSet` and the zone loaders read,
[`zone-craft.md`](zone-craft.md)), with no view test (2048's pause is single view):

| Mode | Screen | Note |
| ---: | --- | --- |
| 4 | `PauseMenuTournament` | |
| 5 | cleared: the default `PauseMenuRace` | Time Trial |
| 6, `0xd`, `0xe` | cleared: the default | Zone, Zone Battle, Detonator ([`zone-craft.md`](zone-craft.md), [`zone-environment-fallback.md`](zone-environment-fallback.md)) |
| 10 | `PauseMenuSpeed` | Speed Lap |
| `0xf`, `0x10`, `0x11`, `0x12` to `0x15` | `PauseMenuOnline` | network races, never opened by this build |
| anything else | by the held event object `DAT_81965b58` (`FUN_812b5890`): none held, cleared; held, with its `+0x190 == 3` (`FUN_812b0880(obj, 3)`, the mode ordinal [`campaign-event-card.md`](campaign-event-card.md) names) and `+0x18c == 0` (`m_numOfLaps`), `PauseMenuSpeed`; held, `+0x190 != 3` and `+0x1a0 & 4`, `PauseMenuOnline`; otherwise cleared | Single Race and Head to Head fall here and open the default |

The names of modes 4, 10 and the ids of the two `PauseMenuOnline` arms are the executable's own and match HD's
and Omega's binders' ids (**ported**, 80); that mode 4 is Tournament and 10 Speed Lap is the lineage's reading,
the screen names agreeing (the Speed Lap page has the ghost tile and END SESSION, the Tournament page no RESTART).
The default arm's event test is what a campaign Speed Lap event reaches when its mode word is not 10: **read, not
walked** (70). What the executable does with the buffer is HD's: the redirect resolves the name at the START press
(not walked on Vita3K: START on the race HUD opens the page, which the frame in
`docs/ui/pause-menu.md` shows for the default screen only).

**Differs from Omega and HD**: neither switches the same ids to the same *kind* of screens. HD's and Omega's are
`Block` and `List` widgets; 2048's are `<TouchButton>` icon pages with a nested confirm screen per destructive tile
(`InGame_Definition.xml`, `NEWGUI/`, v1.04 patch copy; the base package's copy has no `PauseMenuTournament`).

## History

- 2026-11-25: written.
