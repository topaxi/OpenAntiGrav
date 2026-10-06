# 2048's end-race pages draw; Results' table, XP, parade laps and the way back are open

2026-10-06, lane `2048-endrace`. After a 2048 race the player now gets
`RaceSummary` then `ObjectiveSummary` (the disc's own widgets, the verdict's own
colours and words, the medal, a tick or cross), with pad, pointer, restart and
exit - **[docs/ui/endrace-2048.md](../../docs/ui/endrace-2048.md)** is the
account and
[the decompiled chain](../../docs/ghidra/functions/vita-2048-eu-v104/endrace-summary.md)
is the evidence. `oag_title::FrontEnd::endrace_style` is the new per-title axis
(`Pulse`, `Field`, `Touch`); `oag_game::endrace` dispatches on it.

**Not compared against an original frame**: no reference capture of 2048's post-race
exists and no Vita emulator was run; layout, colours and chain are the XML and the
decompile. The live frames were taken in a window larger than the Xvfb screen, so
the tile strip is clipped there; the `--menu-page` stills show it whole.

## Open

- **`Results`' table.** `<RaceResults>` is filled by native code
  (`FUN_810cd0b8` walks to it after `ObjectiveSummary`) that this lane did not
  decompile, so the third page is not drawn and the flow stops at the last page
  it has. Find who builds the rows, then draw them.
- **The result line's wording.** `RaceResultText`/`SpeedResultText` take the
  string at `event+0x250`, written by game-mode code not traced. This build
  words a place, a time (`M:SS.CC`), a zone count or a kill count: **chosen**.
- **XP and rank.** `RaceXP`, `PassBonusXP`, `TotalXP`, `RankText` and the rank bar
  are counted up from a per-event award and a profile total this build does not
  keep. Not drawn. If a progression system lands, the widgets are authored.
- **The way back to the campaign map.** Exit goes to this build's own menus (the
  built-in results table's destination); the original returns to the map. A
  finished race cannot reopen `Stage::Frontend` today.
- **`ParadeLaps`** (a free camera over the finished race) is not built, so its tile
  is not drawn.
- **The idle timer** `FUN_810cd0b8` runs when a flow has two or more pages (`10.0` s
  then `KillGameVita`) reads like an auto-leave; not watched, not implemented.
- **`EndRaceTipsScreen`/`SkipRaceConfirm`** are authored and unreached from this
  chain.
- **Omega** ships the same `EndRace_Definition.xml` byte-for-byte as
  `vita/vita_EndRace_Definition.xml` and nothing reaches it. If Omega races its
  2048 events, give it `Touch` style and a `.gnf` texture spelling in
  `oag_game::endrace::touch::load`.

## Next Steps

1. Decompile the `Results` populate (xref `EndRace Results`-shaped strings near
   `0x81465e40`, `EndRaceResults_Screen.cpp`'s neighbours) and draw its table
   under `ObjectiveSummary`. About an afternoon.
2. Reopen the campaign map from a finished race (`Session::leave_finished_race`
   builds menus today); needs a `Stage::Frontend` rebuild path.
3. Trace `event+0x250`'s writer and replace the chosen wording.
