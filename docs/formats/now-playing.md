# The now-playing display on the race screen

All five titles show the song on the race HUD; two author it and read it off their own
executable, one authors it and borrows the law, one (Pure) authors it as plain text and ours draws a borrowed marquee instead, one authors nothing and borrows a layout. Pinned
by `crates/game/tests/now_playing_ground_truth.rs` (`just test-data`).

| Title | Layout | Strings | Behaviour | State |
| --- | --- | --- | --- | --- |
| Pulse (PSP) | `InGame_Definition.xml`, `item InGameMusicDisplay` | `PI_Music` `Artist` and `name` | **read**, [Pulse page](../ghidra/functions/psp-pulse-usa/now-playing.md) | wired, measured |
| HD / Fury | `ingame_definition.xml`, the same item with a textured bar | `PI_Music` `Artist` and `name` | **read**, [HD page](../ghidra/functions/ps3-hdfury-eu/now-playing.md): Pulse's law | wired, measured |
| Omega | `data00.psarc`'s `ingame_definition.xml`: HD's item | `PI_Music` `Artist` and `name` | inherited from HD | wired, inherited |
| Pure | `InGame_Definition.xml`'s `InGame` screen: three `Text` widgets, no bar, no scroll ([Pure's own music text](#pures-own-music-text)); ours still draws Pulse's marquee | its own `PI_Music` `Artist`, `name` and `Label` | fade step Pulse's, hold measured once | **not wired**: ours draws the borrowed marquee, which the original does not |
| 2048 | none authored | its own `PI_Music` `Artist` and the language table's `MT_nn` title | inherited from HD | wired, **borrowed**, chosen not measured |

The title's row is `oag_title::Music::now_playing` (`oag_title::now_playing::NowPlaying`):
where the layout is, which widgets, the separator, the step sizes, and an `Origin`
(measured, inherited from a named title, or chosen). The loader report line says which.

## How it works here

A translucent bar with `Artist - song` in the `HUDSmall` face scrolling left through it.

1. **Geometry** is read off the layout at race load (`oag_hud::now_playing::geometry`): the
   item's offset, the bar (a fill, or a texture such as HD's `HUD_Components.gtf`), the text's
   origin just past the bar's right edge, and the viewport the text is clipped to. Pulse's
   viewport holds the item, HD's is inside it.
2. **The law** is the executable's, once per tick (`Display::step`): the text's alpha climbs 5
   of 255 a step and the bar's by `int(authored alpha / 256 * 5)`, the text moves 1 unit left a
   step, and once it is its own width past the bar's left edge both fade out by the same steps.
3. **The trigger**: `Race::note_song` when the music starts a song; the display starts with it,
   and the song starts at the green light ([below](#when-the-song-starts)).
   `oag_sound::Audio::now_playing` reports the song (a serial and the name) from the same state
   `--no-audio` reaches, and `oag_game::sound::race_tick` hands it to the race.
4. **Drawing**: `Readout::now_playing` carries the marquee; the host appends the bar and the
   clipped text after any placement (`oag_hud::now_playing::append`, a `Span` scissors the
   text to the viewport).
5. **Names**: `oag_music::song_names` matches each soundtrack entry to its `PI_Music` node:
   by position for a title that declares its tracks, by the name hash of the node's
   `music.at3` for Pulse (whose playlist is still found by what the entries are, so its order
   is untouched), by `location` for Omega. The name of the song that started travels with the
   decoded sound (`oag_sound::Loaded`, a `Song` of name and author), decided on the fetch worker by
   `oag_sound::song::name_song`, stored when the voice starts or a `MUSIC SOURCE` switch swaps it,
   and cached beside the sound in `race_cache`. `Audio::now_playing` only reads it: no listing, no
   disc, no lock, so it is cheap every tick, and the name is the one of the song that sounds, at
   the position actually fetched (never the booted listing at the pool-absolute index).
   - **The ladder**, first that applies: (1) the played source's own listing at the matched
     position (`author` = that title); (2) the booted title's other pressing, at the track of the
     same length (PS2 names nothing; `Soundtrack::pairs_with`, within 11.4 ms on all sixteen tracks,
     the closest wrong match 130 ms away, `docs/formats/ps2-audio.md`), `author` = the title; (3) a
     folder file's tags (`oag_music::file::decode_tagged`, ID3v2 `TIT2`/`TPE1`, Vorbis comments of
     FLAC and Ogg, OpusTags), `author` none; (4) the file's stem as the title, with the tags' artist
     when there is one; (5) the title's name as the artist and `Track N` as the title, N 1-based in
     that source's own listing order, through the string row `OAG_HUD_TRACK_N`, `author` none.
     Nothing is invented: each level is data the source carries or a plain locator for it.
     **Chosen, not measured.**
   - **The record rule**: the marquee shows any accurate name, and `TKR_SONGS`/`TKR_FAVMUS` count a
     song only when `NowPlaying::author` is the title being played
     (`oag_game::app::session::music`). A source switch lands another name on the marquee (the
     serial moves, so it restarts) and does not count again: the count follows `NowPlaying::started`,
     which a rename leaves alone.
   - **2048's `title_key`** applies only to a name its own title authored, the one name the table
     can hold a key for; a key the table lacks shows the raw title, and any other name is shown as
     it is (`oag_raceplay::now_playing::song_text`).
   - **Tags, spiked on symphonia 0.6**: the probe reads ID3v2 before the container and queues it
     on the format reader's metadata log, but only with symphonia's `id3v2` feature, which the
     workspace now enables (without it an MP3's tag block is skipped and nothing is reported);
     FLAC's and Ogg Vorbis's Vorbis comments and Opus's `OpusTags` arrive as standard
     `TrackTitle`/`Artist` tags from the container reader (Opus decodes through `opus_rs`, only
     its demux is symphonia's). Tests generate an ID3v2.4 MP3 and a FLAC `VORBIS_COMMENT` block in
     the test; Ogg and Opus were read off `ffmpeg`-tagged copies by hand, with no fixture committed.

## What differs from the originals

- The playlist resumes across races here (`docs/architecture/race-music-resume.md`): a race after
  the menus continues the same track, mid-way, from the green light, where the original starts
  the next song at every race start (`MusicPlayer_StartNextSong`, watched on PPSSPP after a
  restart). **Deliberate: the maintainer's resume decision.** A resume is not a new song, so its
  first show of the name depends on whether the race before it showed one.
- Pulse's `Dynamic HUD` option (default, concave, flat, off) is wired as a DYNAMIC HUD row on
  Pulse's Display page, `default` or `off` (`[display] dynamic_hud`, applied live): `off` draws
  no HUD and no song marquee, as the original's `FE_OFF` does, and a song that starts while it is
  off is not shown late. `FE_CONCAVE` and `FE_FLAT` set a flag on the HUD's two panel groups whose
  effect is undecoded, so they are not offered. **Checked, differs:** HD has no such option (its
  `hud+0x115` is a runtime "HUD shown" byte Photo Mode clears), Pure's options list has none,
  Omega and 2048 carry no string for it; those titles ignore the setting and the row is not
  offered ([Pulse page](../ghidra/functions/psp-pulse-usa/now-playing.md#dynamic-hud-read-and-poked-2026-10-26-lane-now-playing-leftovers),
  [HD page](../ghidra/functions/ps3-hdfury-eu/now-playing.md)).
- Single view only, and that is the original's rule where it can be read: HD's
  `RaceManager_ShowNowPlaying` shows the display only while `g_GameState+0xe4` is 1, the count of
  human entrants (1 in single player, 2 in split screen,
  [opponent-handling](../ghidra/functions/ps3-hdfury-eu/opponent-handling.md)), and its
  `splitscreen_hud` layouts author no such item, so a split race shows none; Omega and 2048
  inherit it (Omega's executable is unread). Pulse's and Pure's PSP originals have no split
  screen, so theirs is the same choice, **chosen, not measured**; the PS2 Pulse's split is unread.
  `oag_raceplay::Race::now_playing` returns nothing while the race has more than one view.
- The per-update step is 60 Hz: **watched** on Pulse (PPSSPP, 150 updates one frame apart,
  [Pulse page](../ghidra/functions/psp-pulse-usa/now-playing.md#watched-on-ppsspp-2026-10-26-lane-now-playing-leftovers));
  HD's, Omega's and 2048's are the same law read from code, not watched.
- A PS2 boot of Pulse names its songs from the PSP pressing's listing when that disc is found (the
  same recordings, paired by length); with only the PS2 disc it shows `Wipeout Pulse - Track N`.
  The original shows nothing there.
- HD's bar vertex colour is `alpha << 24` (black); read from the code, not seen on a frame.

## When the song starts

The originals keep the race silent through the fly-over and the countdown, and start the song,
and the display with it, when the race leaves its countdown. `oag_title::Music::race_song_start`
says so per title, with where each rule came from:

| Title | Rule | Origin | Evidence |
| --- | --- | --- | --- |
| Pulse (PSP) | at the release | **measured** (read off the executable; the display's start at the release watched on PPSSPP, the silence before it not) | [Pulse page](../ghidra/functions/psp-pulse-usa/now-playing.md#when-the-song-starts-read-2026-10-26-lane-race-music-at-green): `RaceMode_SetState(1)` stops the player, `RaceMode_UpdateCountdown` starts the next song after `SetState(2)` |
| Pulse (PS2) | at the release | inherited from Pulse (PSP): one definition for both pressings, the PS2 executable's player unread | - |
| HD / Fury | at the release | **measured** (read off the executable, not watched) | [HD page](../ghidra/functions/ps3-hdfury-eu/now-playing.md#when-the-song-starts-read-2026-10-26-lane-race-music-at-green): `RaceManager_UpdateCountdown` ends in `MusicManager_ClearHold`; the authored mix has music at 0.0 until `RaceNormal` |
| Omega | at the release | inherited from HD | - |
| 2048 | at the release | inherited from HD | - |
| Pure | at the release | inherited from Pulse | - |

What plays meanwhile is **nothing**: Pulse's fly-over carries a pre-race voice-over of its own
(`MusicPlayer_StartPreRaceDialogue`), stopped when the fly-over ends, and its countdown has no
music and no fade; HD's fly-over and countdown sit on mix rows with music at 0. This build plays
Pulse's voice-over on its own voice (`oag_sound::Audio::start_pre_race_voice`, Pulse PSP only)
and no menu loop under the race: the front end's voice stops at the hand-off as it always did and
the race song stays parked until the release, whatever the voice-over's stop does.

How it works here: the race hand-off arms `oag_sound::Audio::hold_race_song` from the title's
rule before it starts the music (`--race` and the `--screenshot` captures do the same through
`oag_game::sound::hold_race_song`). A track that lands while armed is parked, decoded; the first
`Audio::race_tick` whose frame is no longer gated starts it, on the step that voices `go`. The
fly-over runs no race tick, so it never releases. `Audio::now_playing` reports nothing while armed,
so a race never shows the previous race's song. A resume from the menus does not re-arm: past
the release it plays at once, before it the same hold applies.

A `MUSIC SOURCE` switch made while the song is parked swaps the parked track for the same track on
the new release (`Audio::set_music_source` treats a parked song as the race voice), so the release
starts the recording the player chose; a switch that has to be fetched is queued like any other and
a release that will not read leaves the parked track as it was. One that lands after the release
moves the sounding song from where it has got to, as before. **Chosen, not measured**: the original
has no such row.

## The borrowed titles

**Pure** (checked 2026-10-26, **corrected 2026-11-25**): its `BOOT.BIN` has no `InGameMusic` string (Pulse's has) and its
`InGame_Definition.xml` holds no `InGameMusicDisplay` item; that check looked for Pulse's item and missed what the screen does carry, three
plain `Text` widgets ([below](#pures-own-music-text)). `assets/fallback/Data/XML/NowPlaying_Pure.xml` is
this project's own file carrying Pulse's numbers, in the fallback layer (it answers a name no
disc holds), and what it draws is **not** what the original draws: a translucent bar with the song scrolling through it at the foot of
the frame, over the craft's exhaust and ground shadow on Pure's chase view, where the original draws two plain lines of text at the left.

**2048** (checked 2026-10-26, base package and the 1.04 patch): the executable looks the three
widgets up (`InGameMusicDisplay`, `InGameMusicText`, `InGameMusicBG`) but no layout file holds
the item, so the original shows no song. `assets/fallback/Data/XML/NowPlaying_2048.xml` is HD's
item halved to the Vita's 960x544 grid with a plain translucent bar and a text size picked to sit
in it. 2048's `PI_Music` nodes are named `01`..`11`; the title is the second line of the language
table's `MT_01` (`title_key`), the same two-line text HD's `title_is_id` branch looks up.

## Pure's own music text

Read 2026-11-25 (`pure-classic-vr-set`): the USA disc's `Data\Plugins\PI001\GUI\InGame_Definition.xml` (`FE.wad` and `Data.wad` hold the same
bytes, `oag-wad cat --expand`), screen `InGame`, under the comment `Music info text`:

| Widget | `idstring` | Font, scale, align | `x`, `y` (480x272 grid) |
| --- | --- | --- | --- |
| `InGameArtistText` | `Artist` | `HUDSmall`, 1.0, left | 80, 240 |
| `InGameTrackText` | `Track` | `HUDSmall`, 1.0, left | 80, 250 |
| `InGameLabelText` | `Label` | `HUDSmall`, 1.0, left | 80, 260 |

The strings are `Definition.xml`'s `PI_Music` nodes: `name` (`Kinection`, `Onyx`, `Room 2`...) is the track, `Artist` the artist (`Cosmos`,
`CoLD SToRAGE`, `Jay TripWire`...), `Label` a record label (`Frost Byte Records`, empty for `Kinection`). What the original draws, in 12
race frames, 10 of them showing the text with 6 different songs (`data/reference/compare-page/pure-psp/*/theirs.png`, USA disc): **two lines**, the artist over the track, upper case in
`HUDSmall`, left edge at `x 80`, no bar behind them; the label line never shows (`Onyx`'s `Frost Byte Records` is absent in the frame that
shows `COLD STORAGE / ONYX`). Ours draws the one-line `ARTIST - TRACK` marquee (`COLD STORAGE - ONYX`) in a dark bar at `x 130..280`.

**Its life, measured** on Karbonis (the song `Cosmos / Kinection`) and Sebenco Climb in a private PPSSPP 1.20.4 (software renderer), stepping a loaded
state frame by frame and grabbing the 480x272 window at chosen race clocks (`launch-grade a0 + 0x130`, GO at 0). The text's peak grey,
against the 100 of the road behind it on Karbonis:

| Race clock (s) | 1.6, 3.0, 5.0 | 7.00 | 7.25 | 7.50 | 7.75 | 8.00 | 8.25 | 8.50 and later |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Peak grey | 230 to 243 | 220 | 195 | 175 | 153 | 134 | 114 | 100 (gone) |

A straight line of about 80 grey a second from full at about 6.7 s to gone at about 8.5 s: about 1.8 s at 0.25 s sampling, consistent with 51 frames
at 30 a second times Pulse's 5 of 255 per step (1.70 s; `fade_step`, above), so **the fade step is probably Pulse's**. The fade-in on Sebenco Climb reads peak 163, 174, 189, 205, 227, 242
at 0.83, 1.00, 1.20, 1.40, 1.60, 1.80 s (the road behind it is brighter and moving there): the same slope, from zero at about GO, full at about
1.8 s. So the display is: on at GO, fade in 1.8 s, hold about 4.9 s, fade out 1.8 s, nothing scrolls; in frames at race clock 16.7 and 17.8 s (Vineta,
Modesto) it is gone (what happens at a song change is not measured). Confidence **80** for the layout and the strings (the disc authors them and 10 frames show the result), **60** for the
timing (seen once: one song per end of the life, 0.25 s resolution, the hold's start and end each read once; whether the display times from GO or from the song's
start is not separated, because every state's song started at the grid). The song a state plays differs per state; the original's choice of
song is not read here.

**Effect on the comparison.** Ours' bar is the largest piece of the Classic pairs' hull box's "original brighter" (`docs/tools/compare-page.md`, "The
Classic hull"): pasting the original's pixels over ours in the bar's left part and the original's text box lowers the whole-frame mean absolute RGB-channel difference by 0.4 to
0.7 on every Pure race pair whose original shows the text (the page's own Gray score reads about the same; not recomputed). **Wiring** is the display, a title-data row (a static three-widget text display: where, fonts, the two
strings, the fade step, the hold) instead of Pulse's bar, and `NowPlaying_Pure.xml` retired for the disc's own widgets: ready to wire, not wired by the
lane that read it. Whether Pulse's, HD's, 2048's or Omega's `InGame` screens carry the same three widgets beside their `InGameMusicDisplay` was not checked
(the Pulse and HD pages above read the item, not the screen's other `Text` widgets).

## Lineage check, recorded

2048 against Omega (and the reverse): Omega ships HD's item, 2048 ships none - **checked,
differs**; 2048 borrows HD's through the fallback layer, **ported** as chosen, not measured.
Omega's executable was not read for the display (no capture path): **not checkable** beyond
the shared layout, so Omega inherits HD's measured law.
