# The in-race "now playing" display (HD / Fury, PS3)

Addresses are `hdfury` `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`). Read 2026-10-26 from
the decompiler; the same display as Pulse's
([`psp-pulse-usa/now-playing.md`](../psp-pulse-usa/now-playing.md)), found by following
`FE_UNKNOWN_ARTIST` (the name strings of the display have no code references of their own:
Ghidra's xrefs to them land in shared string-pool tails).

| Address | Name | Conf | What it is |
| --- | --- | ---: | --- |
| `0x0005c8f8` | `RaceManager_ShowNowPlaying` | 80 | Fills the music text and restarts the display |
| `0x0005c638` | `RaceManager_UpdateNowPlaying` | 80 | Per-update state machine |

## Show

`RaceManager_ShowNowPlaying(manager, artist, title, title_is_id)`: runs when the HUD flag
(`*(g_hud) + 0x115`) is set and `g_GameState[0xe4] == 1`. An empty artist becomes the string
`FE_UNKNOWN_ARTIST`; an empty title `FE_UNKNOWN_TRACK` (both for custom music); when
`title_is_id` the title is looked up as a string id. The text is `"%.256s - %.256s"`:
**artist - title**. Then alpha 0, state 0 (`+0x2d44`), step 5 (`+0x2d6c`), the text's x back to its
authored value (`widget+0xb0` from `+0x2d50`), and both alphas applied.

## Update

Per update of the race manager, with no delta time:

| State | Each update | Ends when |
| --- | --- | --- |
| 0 | text alpha `+= 5`; bar alpha `+= int(saved * (5 * 0.00390625))`; text x `-= 1.0` (`DAT_008a6970` = `0x3f800000`) | text alpha > 255, then 1 |
| 1 | text x `-= 1.0` | `x <= -width` (the widget's virtual width call), then 2 |
| 2 | text alpha `-= 5`; bar alpha falls by the same integer step | text alpha < 0, then 3 |
| 3 | text cleared, off | |

`saved` (`+0x2d70`) is the bar image's own alpha byte (`widget+0xbc`), read in the constructor
(`0x0005d598`) before the vertex colours are zeroed: HD's bar is a textured image with no
authored colour, so 255, and the bar climbs `int(255 / 256 * 5)` = 4 a step. The vertex colour
written is `alpha << 24` (black at that alpha). Identical to Pulse's law with a textured bar.

## Layout

`/data/plugins/frontend/gui/ingame_definition.xml`, `item InGameMusicDisplay OffsetX=310
OffsetY=52`: `InGameMusicBG` (`HUD_Components.gtf` 145,653 230x48) and, in a viewport inside the
item (`x=19 width=192`), `InGameMusicText` (`IG_HUD_ARTIST`, `HUDSmall`, scale 0.8, x=230,
y=10), 1920x1080 grid.

Omega ships the same item in `data00.psarc`; 2048 ships none.
