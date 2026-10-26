# The now-playing display on the race screen

All five titles show the song on the race HUD; two author it and read it off their own
executable, one authors it and borrows the law, two author nothing and borrow a layout. Pinned
by `crates/game/tests/now_playing_ground_truth.rs` (`just test-data`).

| Title | Layout | Strings | Behaviour | State |
| --- | --- | --- | --- | --- |
| Pulse (PSP) | `InGame_Definition.xml`, `item InGameMusicDisplay` | `PI_Music` `Artist` and `name` | **read**, [Pulse page](../ghidra/functions/psp-pulse-usa/now-playing.md) | wired, measured |
| HD / Fury | `ingame_definition.xml`, the same item with a textured bar | `PI_Music` `Artist` and `name` | **read**, [HD page](../ghidra/functions/ps3-hdfury-eu/now-playing.md): Pulse's law | wired, measured |
| Omega | `data00.psarc`'s `ingame_definition.xml`: HD's item | `PI_Music` `Artist` and `name` | inherited from HD | wired, inherited |
| Pure | none authored | its own `PI_Music` | inherited from Pulse | wired, **borrowed**, chosen not measured |
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
   is untouched), by `location` for Omega.

## What differs from the originals

- The playlist resumes across races here (`docs/architecture/race-music-resume.md`): a race after
  the menus continues the same track, mid-way, from the green light, where the original starts
  the next song. A resume is not a new song, so its first show of the name depends on whether
  the race before it showed one.
- Pulse and HD gate the display on the HUD setting (Pulse's `Dynamic HUD` set to `FE_OFF`
  hides it). This build has no such setting: always on.
- Single view only. A split-screen race draws no marquee.
- The executable's per-update step is assumed to be 60 Hz; no capture of the original confirms
  it.
- A PS2 boot of Pulse shows nothing: its soundtrack archive carries no names.
- HD's bar vertex colour is `alpha << 24` (black); read from the code, not seen on a frame.

## When the song starts

The originals keep the race silent through the fly-over and the countdown, and start the song,
and the display with it, when the race leaves its countdown. `oag_title::Music::race_song_start`
says so per title, with where each rule came from:

| Title | Rule | Origin | Evidence |
| --- | --- | --- | --- |
| Pulse (PSP) | at the release | **measured** (read off the executable, not watched) | [Pulse page](../ghidra/functions/psp-pulse-usa/now-playing.md#when-the-song-starts-read-2026-10-26-lane-race-music-at-green): `RaceMode_SetState(1)` stops the player, `RaceMode_UpdateCountdown` starts the next song after `SetState(2)` |
| Pulse (PS2) | at the release | inherited from Pulse (PSP): one definition for both pressings, the PS2 executable's player unread | - |
| HD / Fury | at the release | **measured** (read off the executable, not watched) | [HD page](../ghidra/functions/ps3-hdfury-eu/now-playing.md#when-the-song-starts-read-2026-10-26-lane-race-music-at-green): `RaceManager_UpdateCountdown` ends in `MusicManager_ClearHold`; the authored mix has music at 0.0 until `RaceNormal` |
| Omega | at the release | inherited from HD | - |
| 2048 | at the release | inherited from HD | - |
| Pure | at the release | inherited from Pulse | - |

What plays meanwhile is **nothing**: Pulse's fly-over carries a pre-race voice-over of its own
(`MusicPlayer_StartPreRaceDialogue`), stopped when the fly-over ends, and its countdown has no
music and no fade; HD's fly-over and countdown sit on mix rows with music at 0. This build plays
neither the voice-over nor any menu loop under the race: the front end's voice stops at the
hand-off as it always did and the race song is parked until the release.

How it works here: the race hand-off arms `oag_sound::Audio::hold_race_song` from the title's
rule before it starts the music (`--race` and the `--screenshot` captures do the same through
`oag_game::sound::hold_race_song`). A track that lands while armed is parked, decoded; the first
`Audio::race_tick` whose frame is no longer gated starts it, on the step that voices `go`. The
fly-over runs no race tick, so it never releases. `Audio::now_playing` reports nothing while armed,
so a race never shows the previous race's song. A resume from the menus does not re-arm: past
the release it plays at once, before it the same hold applies.

**Open:** a `MUSIC SOURCE` switch made while the song is parked finds no race voice to move and
is ignored until the next race; the parked track plays from the source it was fetched on.

## The borrowed titles

**Pure** (checked 2026-10-26): its `BOOT.BIN` has no `InGameMusic` string (Pulse's has) and its
`InGame_Definition.xml` holds no such item. `assets/fallback/Data/XML/NowPlaying_Pure.xml` is
this project's own file carrying Pulse's numbers, in the fallback layer (it answers a name no
disc holds). The bar sits over the craft's exhaust on Pure's chase view: placement is Pulse's.

**2048** (checked 2026-10-26, base package and the 1.04 patch): the executable looks the three
widgets up (`InGameMusicDisplay`, `InGameMusicText`, `InGameMusicBG`) but no layout file holds
the item, so the original shows no song. `assets/fallback/Data/XML/NowPlaying_2048.xml` is HD's
item halved to the Vita's 960x544 grid with a plain translucent bar and a text size picked to sit
in it. 2048's `PI_Music` nodes are named `01`..`11`; the title is the second line of the language
table's `MT_01` (`title_key`), the same two-line text HD's `title_is_id` branch looks up.

## Lineage check, recorded

2048 against Omega (and the reverse): Omega ships HD's item, 2048 ships none - **checked,
differs**; 2048 borrows HD's through the fallback layer, **ported** as chosen, not measured.
Omega's executable was not read for the display (no capture path): **not checkable** beyond
the shared layout, so Omega inherits HD's measured law.
