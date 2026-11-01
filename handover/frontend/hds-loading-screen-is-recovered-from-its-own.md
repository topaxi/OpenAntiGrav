---
status: re-first
titles: [hd]
updated: 2026-11-01
summary: "HD loading screen: layout read, awaiting a screen-layout TOML; mode ids unnamed, flag writer unread, 90 percent hold unknown"
---

# HD's loading screen is recovered; the seven unnamed mode ids and one flag writer are left

2026-10-05. Pages: [hd-loading.md](../../docs/formats/hd-loading.md), [loading-screen.md](../../docs/ghidra/functions/ps3-hdfury-eu/loading-screen.md) (its 2026-10-04 and 2026-10-05 sections carry the evidence), mode enum on [mode-manager.md](../../docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md). Caption + circuit name, a feature **drawn per screen from the mode's own deck** (`oag_hd::loading::DECK`), tinted by four `FEGlobals`. **Closed 2026-11-01**: the bar's 30 rows and the mode icon clip, with an RPCS3 pair (docs "The pair"). **Closed 2026-10-06**: the bar fills from `race::load`'s stages (`oag_hd::loading::PROGRESSION`, the original's 0.2/0.4/0.5/0.75/0.95 and its easing, see loading-screen.md), proven with `--loading-live`. **Closed 2026-10-05**: the bar is a 166-column grid of `dot.gtf` (`Draw::TiledSprite`), lit in `HD_Blue` (the live dots peak at exactly `0xffac0717`, so the red is the palette, not a team colour) over `HD_LightGrey`, in whole columns; the five labels (`WIPEOUT® HD`, `FEATURE IMAGE`, `FEATURE DESCRIPTION`, `PROGRESSION BAR`, `MODE ICON`) are the game's own, not a debug overlay, and are drawn behind `square.gtf` bullets; the feature image's red is the `_fury` art, not a tint pass; `--loading-screen --track` names the circuit instead of printing the path. **Closed 2026-10-04**: the Fury-content flag, the five title ids, `HD_LightGrey`, `FUN_006762f8` is `rand()`. **The trap**: the `LSAD_*` ad frames sit in the same executable string-run and were taken for the whole screen until RPCS3 disproved it.

## Open

- **Whether the original holds the screen until the bar crosses 90 %** (`149.4` columns). Our load ends in about 3 s, with the bar near 33 of 166 columns at the original's ease, so the screen snaps to full and fades (chosen, not measured). The threshold is read at `0x002b7a44`, what waits on it is not. `0x000b43b0` and `0x000b9380` (the second kinds of the track and world constructors) are unread.
- **The inner two stage placements are by position**: `WorldBuilt` and `CraftsBuilt` in `race::load`, chosen, not measured, since our load reads the crafts before the track.
- **The mode icon** plays `single_race_icon_fury.bik` for mode 3 only (RPCS3 frame match); the other mode ids' clips are unread (the executable's pick from the mode). Base styling's name is chosen.
- **Layout**: ours draws the screen in a 480x272 PSP grid with a line-height normalisation; the original is native 1920x1080 with fonts `Title`/`default` at scales 1.0/0.8/0.45. The numbers are now read from `LoadingScreen_Draw` (loading-screen.md, "2026-11-01: the race load's layout"). The pair is promoted (`hd-loading`, baseline share 10.52 %). Next: a screen-layout TOML in the fallback layer (proposal awaiting the maintainer, lane `hd-loading-layout` phase 2), a mod track's `[loading]` manifest table.
- The seven unnamed mode ids on `g_GameState` (`0`, `6`, `7`, `11`, `13`, `14`, `15`). Speed Lap and Zone draw from the default deck in this build, chosen, not measured.
- The writer of the Fury-content flag at `0x00b979fd`; this build assumes `true` for every HD source.
- Why one boot constructs two or three loading screens, only the last matching the frame.

## Next Steps

- Phase 2 of `hd-loading-layout`: implement the screen-layout format, draw HD's loading screen from `LoadingScreen_HD.toml`, drop the share below 10.52 %.

- Name the seven ids by the other places `g_GameState`'s mode picks a branch.
- Follow the `[PARAM]` sites `0x0021ba9c`, `0x0005b3c8`, `0x00229a58` to the flag's writer.
