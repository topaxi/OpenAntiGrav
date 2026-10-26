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
3. **The trigger**: `Race::note_song` when the music starts a song; the display starts at the
   green light. `oag_sound::Audio::now_playing` reports the song (a serial and the name) from
   the same state `--no-audio` reaches, and `oag_game::sound::race_tick` hands it to the race.
4. **Drawing**: `Readout::now_playing` carries the marquee; the host appends the bar and the
   clipped text after any placement (`oag_hud::now_playing::append`, a `Span` scissors the
   text to the viewport).
5. **Names**: `oag_music::song_names` matches each soundtrack entry to its `PI_Music` node:
   by position for a title that declares its tracks, by the name hash of the node's
   `music.at3` for Pulse (whose playlist is still found by what the entries are, so its order
   is untouched), by `location` for Omega.

## What differs from the originals

- The originals start a song, and the display, when the race leaves its countdown state. This
  build's race music starts at load; the display waits for the green light and the sound does
  not. Moving the music to the light is a separate change.
- Pulse and HD gate the display on the HUD setting (Pulse's `Dynamic HUD` set to `FE_OFF`
  hides it). This build has no such setting: always on.
- Single view only. A split-screen race draws no marquee.
- The executable's per-update step is assumed to be 60 Hz; no capture of the original confirms
  it.
- A PS2 boot of Pulse shows nothing: its soundtrack archive carries no names.
- HD's bar vertex colour is `alpha << 24` (black); read from the code, not seen on a frame.

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
