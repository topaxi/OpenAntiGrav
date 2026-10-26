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
| `0x002fb8a0` | `MusicManager_PlaySongFromPlaylist` | 80 | Picks and starts a song, calls `ShowNowPlaying` (its own `PlaySongFromPlaylist: No music...` log string) |
| `0x002fae68` | `MusicManager_Hold` | 75 | Sets the manager's hold byte `+0x54` and the global hold byte `+0x6d` |
| `0x002fae30` | `MusicManager_ClearHold` | 75 | Clears both |

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

## When the song starts (read 2026-10-26, lane `race-music-at-green`)

Read from the decompiler, **not watched on a running original**. The song sounds from the
release, and the display with it.

- `RaceManager_UpdateCountdown` (`0x00057690`) ends, after `RaceManager_SetPhase(this, 2)` and the
  queued `go`, with `MusicManager_ClearHold(g_music_manager)` ([countdown-voice.md](countdown-voice.md)).
- The manager's per-update (`0x002f7608`, called from `0x002fc070`) pauses the stream while the
  game is paused, the manager's hold byte (`+0x54`) is set or the global one (`+0x6d`) is; the
  display's text sits pending (`+0x31a`) until `+0x6d` reads 0, and only then calls
  `RaceManager_ShowNowPlaying`. So the display follows the hold, not the song's selection.
- `MusicManager_Hold` has one caller, the table-driven state action `0x001793d0` (case 3, with
  `ClearHold` as case 4 and stop and resume as 1 and 2); that function has no direct caller and
  which states carry the hold was not read. `InGame_Restart`'s handler (`0x0003d0f8`) stops the
  music (`0x002fbea8`) and clears the hold.
- The authored mix agrees from the other side: `GlobalAudioConfig.xml`'s `PreRace` and
  `Countdown` rows have music at 0.0 and `RaceNormal` at 0.65 ([audio-mix.md](audio-mix.md)), so a
  song started early would not be heard either.
- **Not read:** whether HD starts the song fresh at the release or resumes a stream held from
  the load. The first audible sample is the release either way, which is what this port does.

Confidence 75 for the timing: the release call and the pending-display logic are read, the state
action that arms the hold is not.
