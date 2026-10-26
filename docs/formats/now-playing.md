# The now-playing display on the race screen

Status: Pulse (PSP) wired and read off the executable; the others recorded below.

| Title | Authored layout | Strings | Timing | State |
| --- | --- | --- | --- | --- |
| Pulse (PSP) | `InGame_Definition.xml`, `item InGameMusicDisplay` | `PI_Music` `Artist` and `name`, `"%s - %s"` | read, [now-playing.md](../ghidra/functions/psp-pulse-usa/now-playing.md) | **wired** |
| HD / Fury | `/data/plugins/frontend/gui/ingame_definition.xml`, `item InGameMusicDisplay` | `PI_Music` in `/data/plugins/frontend/definition.xml` | not read | found, not wired |
| 2048, Omega, Pure | see below | | | |

## Pulse

The display is a translucent bar (150x20 at 130,247 of the 480x272 grid) with the text
`Artist - song name` in the `HUDSmall` face scrolling through it from the right. It is not in
the mode's `*_HUD.xml`; the front end's `InGame` state authors it and `RaceManager_UpdateNowPlaying`
animates it. Ours reads the item off the disc (`oag_hud::now_playing::geometry`), steps the
executable's own law once per tick (`Display::step`: fade 5 of 255 per update, scroll 1 unit
per update, out when the text is its own width past the bar's left edge), and the host appends
the bar and the clipped text to the HUD frame (`oag_hud::now_playing::append`).

What differs from the original, deliberately:

- The original starts the display with the song, and its song starts when the race leaves its
  countdown state. This build's race music starts at load, so the display waits for the green
  light and the music does not. Moving the music to the light is a separate change.
- The original gates the display on the `Dynamic HUD` setting (`FE_OFF` hides it). This build
  has no such setting, so it is always on.
- Single view only: a split screen draws no marquee (the original's split screen is unread).
- The executable's per-update step is assumed to run at 60 Hz; not yet confirmed by a capture.
- The PS2 pressing: its soundtrack archive carries no names, so a PS2 boot shows nothing.

Pulse's playlist order is untouched: its tracks are still found by what the entries are, and
the display matches each entry to its `PI_Music` node by the name hash of the node's
`music.at3` (`oag_music::song_names`).

Ground truth: `crates/game/tests/now_playing_ground_truth.rs`.

## HD / Fury

`/data/plugins/frontend/gui/ingame_definition.xml` has `item InGameMusicDisplay OffsetX=310
OffsetY=52 SDOffsetY=80`: an `InGameMusicBG` image (`HUD_Components.gtf`, 230x48 from 145,653)
and an `InGameMusicText` (`IG_HUD_ARTIST`, `HUDSmall`, scale 0.8, x=230, y=10) in a viewport of
x=19 width=192, in the 1920x1080 grid. The executable carries `InGameMusicDisplay`,
`InGameMusicText`, `InGameMusicBG` and `FE_UNKNOWN_ARTIST`. The soundtrack's `PI_Music` nodes
(`/data/plugins/frontend/definition.xml`) carry `Artist` and `Label`; HD already declares its
tracks through them (`oag_title::Music::tracks`), so `oag_music::song_names` would name them
with only a `NowPlaying` row on the title. **Timing not read**: the `.opd` call sites that
Ghidra lists for the three name strings are string-pool tails shared with an unrelated
`RankingsConfigTagModule`, not the display's code; the real consumer reaches the strings through
the data table at `0x008a7974` (display) and `0x008a6920` (text). Unwired.

## Omega

`data09.psarc` carries the same `item InGameMusicDisplay` as HD, byte for byte but for a
`reset="true"` on its viewport (**checked, applies, not wired**). Omega's songs are Wwise states
named by `PI_Music` in `data/plugins/music/Definition.xml`; `oag_music::omega::Song` has the
title. The Omega executable's display code is not read.

## 2048

**Checked, differs**: `data/plugins/frontend/NEWGUI/InGame_Definition.xml` and
`LegacyInGame_Definition.xml` author no music item; the executable has `InGameMusic` strings
(three), so the layout lives in a file this pass did not find or is built in code. Not wired.

## Pure

**Checked, differs**: Pure's `BOOT.BIN` has no `InGameMusic` string (Pulse's has), and its
`InGame_Definition.xml` holds no music item. Pure authors no display. By the lineage rule Pure
inherits Pulse's, which would be **chosen, not measured**, labelled borrowed; the brief says to
ask before designing one, so it is left to the lead.
