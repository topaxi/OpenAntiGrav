# The pause redirect: which pause screen a game mode opens (Omega, PS4)

Read 2026-11-07, lane `pause-menu-2`, `/omega/eboot-ps4-omega-eu.bin` (image base `0x01000000`). HD's
counterpart: [`ps3-hdfury-eu/pause-menu.md`](../ps3-hdfury-eu/pause-menu.md); what the screens show and
what this build does with them: [the pause menu](../../../ui/pause-menu.md).

| Address | Name | Conf. | What it is |
| --- | --- | ---: | --- |
| `0x0155dc00` | `PauseMenu_BindRedirectForMode` | 80 | Finds `InGame Pause RedirectController` (`FUN_013cbc60`) and writes the pause screen's name into its `+0x364` (`+0x3a4 = 1`), or clears it |

The one string xref of `InGame Pause SP Time Trial` (`0x01825a72`) is this function; the three
`SP` names, the `Zone` trio, `Split Player 1/2` and the `MP` names are all here.

## The switch

The mode is `DAT_01f999e4`, **unless a campaign event object is held** (`DAT_01e31910`): its kind
(`+0x21c`, 1 to 3) first rewrites the mode (kind 1 to 8, or `0x14` with flag `+0x22c & 4`; kind 2 to 4
or 3, or `0x11` and `0x10` with the flag; kind 3 to 5, or 10 when its count `+0x214` is above 98 (`0x62`),
or 10 when the count is 0). Then, as HD's binder does and with the same ids:

| Mode | One view | Two or more views |
| ---: | --- | --- |
| 4 | `InGame Pause SP Tournament` | `Tournament Split Player 1` / `2` |
| 5 | `InGame Pause SP Time Trial` | the same: no view test |
| 10 | `InGame Pause SP Speed Lap` | the same |
| 6, 14 | `InGame Pause Zone` | the same |
| 13 | `InGame Pause Zone P1` / `P2` | the same pair by pad |
| 15, 16, 18 to 21 | `InGame Pause MP` | `MP` / `MP Player 2` |
| 17 | `InGame Pause MP Tournament` | `MP Tournament` or `MP Player 2` |
| anything else | the buffer is cleared: the redirect's `Default goto="InGame Pause SP"` | `InGame Pause Split Player 1` / `2` |

**Same mode ids as HD, same screens: ported** (static, 80; not walked live: **not checkable here**, no
shadPS4 or `sway` is installed on this machine; recipe: `scripts/shadps4-omega-pair.py`'s rig with a START
press after the grid, then `Race Box` set to Time Trial).

## The disc's file

`ingame_definition.xml` is in `data00`, `data08` and `data09` and the three agree on the five
single-player screens. They are HD's Fury file **without `ONL_VIEW_INV`** (PSN's invites) and
otherwise row for row: **checked, differs** in that one row (`omegas_pause_screens_are_hds_...`
asserts the three copies and the HD table less the row).

## History

- 2026-11-07: written.
