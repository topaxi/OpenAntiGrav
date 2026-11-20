---
status: re-first
titles: [pulse]
updated: 2026-11-20
summary: "Track Help and Team Help are drawn; Pre Race Music Select (a MusicList screen) and Tournament C with its Help are new screens, not built"
difficulty: hard (40)
---

# What is left of the pickers' Help screens

`Track Help` and `Team Help` are drawn on both pressings, opened on `Roles::help` and closed on
cross, triangle, circle or a click, with the selected team's own paragraph and the craft dimmed under
the overlay (`docs/ui/back-and-help.md`, "The pickers' Help overlays"). Photo mode, the replay viewer,
the disc chooser and the key hints follow the resolved Back. What is not built are two screens.

## Open

- **`Pre Race Music Select`** (`Team Selection`'s `Music Redirect`, `forward="square"`; the footer draws
  `Music playlist` and the button does nothing). The XML authors a dim panel with hairlines, a `MusicList`
  (x 80, y 130, small face, `arrowcolor`) and three Back redirects, and nothing says what a `MusicList` draws
  or what its selection writes. Our `oag-sound` has the playlist, previous/next race song and now-playing
  (`previous_race_song`, `Shuffle`). The pause menu's `MusicList` (`docs/ui/pause-menu.md`, "Not built") is
  the same widget, so one build finishes both. **RE first**: the `MusicList` widget class on the PSP (draw,
  Left/Right, the pick it stores at `+0x3c0`, `docs/ghidra/functions/psp-pulse-usa/now-playing.md`) - about
  half a day (confidence 50 on the estimate); then drawing it and feeding the race music pick, 1-2 days.
- **`Tournament C` and `Tournament Help`.** `Tournament C` (`type="TournamentSelection"`, title `RB_TOURN_SET`) is
  a twelve-slot leg picker (`List` widgets `Track1`..`Track12`, an add and remove action, `MSC_REMOVE_TRACK`)
  that this build has no model of: `Mode::Tournament` is absent from the RACE page's mode list and a
  tournament launches only from a campaign cell's `tournament_tracks` (`docs/gameplay/race-modes.md`). Its Help
  (`MSC_HELP_RB_CRTN`, `forward="triangle"` on the PSP, `circle` on the PS2) has nothing to open from until the
  screen exists. 2-3 days with some RE (`docs/ghidra/functions/psp-pulse-usa/tournament.md` covers the
  leg state, not the screen's input), confidence 40.
- **The PS2's split Ship Select has no Help** to build: `Team SelectionSplit` authors no face-button
  `Redirect` and the executable never opens `Team HelpSplit` (`race-box-screens.md`). Not open; recorded so
  nobody searches again.

## Next Steps

1. Trace the `MusicList` widget (PSP `BOOT.BIN`), then build Pre Race Music Select and the pause menu's row on it.
2. Model `Tournament C` as its own screen, then its Help on `Roles::help`.
