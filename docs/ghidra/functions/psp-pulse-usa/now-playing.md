# The in-race "now playing" display (Pulse, PSP)

Addresses are `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`. Read 2026-10-26
from the decompiler; **not yet compared against a running original** (a capture is
the arbiter for the 60 Hz assumption in the last section).

| Address | Name | Conf | What it is |
| --- | --- | ---: | --- |
| `0x08826ff4` | `RaceManager_ShowNowPlaying` | 85 | Fills the music text and restarts the display |
| `0x0882be1c` | `RaceManager_UpdateNowPlaying` | 85 | Per-update state machine: fade in and scroll, scroll, fade out, clear |
| `0x088290bc` | `RaceManager_ApplyNowPlayingAlpha` | 80 | Writes the text alpha and the background's four vertex alphas |
| `0x08938e70` | `MusicPlayer_StartNextSong` | 65 | Picks the next song, loads it, copies its strings, calls `ShowNowPlaying` |
| `0x089385b4` | `MusicPlayer_Stop` | 75 | Stops the stream the music player holds and sets its state to 0 |
| `0x089387e8` | `MusicPlayer_StartPreRaceDialogue` | 65 | Plays the circuit's pre-race voice-over through the music player |

## Layout (authored)

`Data\Plugins\PI001\GUI\InGame_Definition.xml`, inside the `InGame` state, a
`Viewport x=130 height=272 width=150` holding `item InGameMusicDisplay
OffsetX=130 OffsetY=247`:

- `Image InGameMusicBG`: `width=150 height=20 color=0x50000000 blurY=1`, no texture (a
  translucent black bar).
- `Text InGameMusicText`: `idstring=IG_HUD_ARTIST font=HUDSmall scale=1.0 align=left
  blurY=1 x=150 y=5 color=0xFFFFFFFF`. `x=150` is the viewport's own width, so the text
  starts just off its right edge: a marquee.

`RaceManager_Construct` (`0x08829124`) looks both widgets up in the `InGame` state, stores
the text widget at `manager+0x19a8` and the background at `+0x19a4`, saves the
background's authored alpha (`+0xac >> 24`) at `+0x19b4`, zeroes its four vertex colours,
and stores the text's authored x (`widget+0x9c`) at `+0x19a0` as the scroll origin.

## Trigger and string

`MusicPlayer_StartNextSong` runs on every song start. When the race manager exists and
`g_game_mode != 2` (or the debug override is set) it calls `RaceManager_ShowNowPlaying`.
That sets the text to `sprintf("%s - %s", player+0xbc, player+0x1bc)` (format at
`0x08a7a564`), where `+0xbc` is the song entry's `+0xa4` and `+0x1bc` its `+0x74`. The
`PI_Music` node's `Artist` entry and its `name` attribute are the two: **"Artist - song
name"**, for example `Aphex Twin - Fenix Funk 5 (Wipeout Edit)`.

It is shown only while the music player state (`+0x3c`) is 1 or 2 **and**
`g_hud+0x121` is set. `Hud_Update` (`0x0881bf50`) sets that byte from the
`Dynamic HUD` setting: `FE_OFF` clears it (and blanks the text), `FE_DEFAULT` and
`FE_FLAT` set it. So the display follows the Dynamic HUD option.

## Timing (per update of the race manager, once per frame)

`RaceManager_UpdateNowPlaying` is called from `0x0882798c` on every race-manager update,
with no delta time, so every step is per update.

| State (`+0x1994`) | Each update | Ends when |
| --- | --- | --- |
| 0, entering | text alpha `+= 5` (of 255); background alpha `+= saved_alpha/256 * 5`, clamped to the saved alpha; text x `-= 1.0` | text alpha > 255 (clamped to 255), then state 1 |
| 1, scrolling | text x `-= 1.0` | `x <= -text width` (the widget's own width virtual), then state 2 |
| 2, leaving | text alpha `-= 5`, background alpha falls by the same step | text alpha < 0, then state 3 |
| 3 | text cleared, display off | |

`ShowNowPlaying` starts it with alpha 0, step 5, text x = the authored origin.

## When the song starts (read 2026-10-26, lane `race-music-at-green`)

Read from the decompiler, **not watched on a running original**. The race song, and with it the
display, starts on the step that voices `go`; nothing sounds on the music player between the
fly-over and then.

- `MusicPlayer_StartNextSong(player, pick)` opens with `if (g_race_manager == 0 ||
  race_manager+0x7c8 <= 1) { player+0x3c0 = pick; player+0x48 = pick; return 0; }`. The mode
  state `+0x7c8` is 0 in the fly-over and 1 in the countdown: there it **only stores the pick**
  and starts nothing.
- `RaceMode_SetState(mode, 1)` (`0x08827350`, the countdown's start) calls
  `MusicPlayer_Stop(player, 0, 1)` before the `ready` cue. `MusicPlayer_Stop` switches on the
  player's source kind (`+0x3bc`: 0 is the ATRAC stream, whose stop flag it sets and whose
  thread and Atrac id `0x0893be54` releases; 1 is a second player type, stopped by
  `0x0893c7a4`), then writes `+0x3bc = 0` and the state `+0x3c = 0`. Silence from `ready` to `go`, no fade.
- `RaceMode_UpdateIntro_q` (`0x08829e6c`), substate 1: at the fly-over counter's 0 it calls
  `MusicPlayer_StartPreRaceDialogue` (`Data\Sound\PreRaceDialogue\%d\Tr...`, the circuit's
  voice-over, through the same player); when the fly-over ends it calls `MusicPlayer_Stop`
  again. This build plays no pre-race dialogue, so the fly-over is silent here.
- `RaceMode_UpdateCountdown` (`0x088274b4`), when the whole seconds left reach zero:
  `Race_StartRacing`, `RaceMode_SetState(mode, 2)`, the `go` cue, `player+0x9c |= 2`, then
  `MusicPlayer_StartNextSong(player, -1)`. The other start site, `0x08825bb0` (a countdown
  that waits on a master-clock list, `MasterClList`, rather than a local timer), makes the same
  calls in the same order.
- The pick at that moment: the stored `+0x3c0` when it is in range, else the song after
  `player+0x48` (random when `+0x9c & 1`, the previous one when `+0x9c & 4`). `+0x3c0` is the
  stored pick above; who writes a pick other than -1 was not chased.

Confidence 80 for the timing (two call sites agree and `SetState(1)` is the stop); the player's
`+0x3bc` source-kind reading is 65.

## Other titles

Checked in the same change, see [the format page](../../../formats/now-playing.md).
