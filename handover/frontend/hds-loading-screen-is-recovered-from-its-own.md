---
status: re-first
titles: [hd]
updated: 2026-11-01
summary: "HD loading screen is a measured screen-layout file; mode ids unnamed, flag writer unread, 90 percent hold unknown"
---

# HD's loading screen is recovered; the seven unnamed mode ids and one flag writer are left

2026-10-05. Pages: [hd-loading.md](../../docs/formats/hd-loading.md), [loading-screen.md](../../docs/ghidra/functions/ps3-hdfury-eu/loading-screen.md) (its 2026-10-04 and 2026-10-05 sections carry the evidence), mode enum on [mode-manager.md](../../docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md). Caption + circuit name, a feature **drawn per screen from the mode's own deck** (`oag_hd::loading::DECK`), tinted by four `FEGlobals`. **Closed 2026-11-01**: HD's whole layout is a file; the bar's 30 rows and the mode icon clip, with an RPCS3 pair (docs "The pair"). **Closed 2026-10-06**: the bar fills from `race::load`'s stages (`oag_hd::loading::PROGRESSION`, the original's 0.2/0.4/0.5/0.75/0.95 and its easing, see loading-screen.md), proven with `--loading-live`. **Closed 2026-10-05**: the bar is a 166-column grid of `dot.gtf` (`Draw::TiledSprite`), lit in `HD_Blue` (the live dots peak at exactly `0xffac0717`, so the red is the palette, not a team colour) over `HD_LightGrey`, in whole columns; the five labels (`WIPEOUT® HD`, `FEATURE IMAGE`, `FEATURE DESCRIPTION`, `PROGRESSION BAR`, `MODE ICON`) are the game's own, not a debug overlay, and are drawn behind `square.gtf` bullets; the feature image's red is the `_fury` art, not a tint pass; `--loading-screen --track` names the circuit instead of printing the path. **Closed 2026-10-04**: the Fury-content flag, the five title ids, `HD_LightGrey`, `FUN_006762f8` is `rand()`. **The trap**: the `LSAD_*` ad frames sit in the same executable string-run and were taken for the whole screen until RPCS3 disproved it.

## Open

- **Whether the original holds the screen until the bar crosses 90 %** (`149.4` columns). Our load ends in about 3 s, with the bar near 33 of 166 columns at the original's ease, so the screen snaps to full and fades (chosen, not measured). The threshold is read at `0x002b7a44`, what waits on it is not. `0x000b43b0` and `0x000b9380` (the second kinds of the track and world constructors) are unread.
- **The inner two stage placements are by position**: `WorldBuilt` and `CraftsBuilt` in `race::load`, chosen, not measured, since our load reads the crafts before the track.
- **The mode icon** plays `single_race_icon_fury.bik` for mode 3 only (RPCS3 frame match); the other mode ids' clips are unread (the executable's pick from the mode). Base styling's name is chosen.
- **Layout**: closed 2026-11-01 (`hd-loading-layout`): the screen is `LoadingScreen_HD.toml` (docs `formats/screen-layout.md`, measured off `LoadingScreen_Draw`, drawn in its own 1920x1080 grid under the Safe Area), the promoted pair `hd-loading` went 10.52 % to 0.76 %. Left: the second `Title` pass at the caption, the tournament screen (type 1) and kind 8, and moving Pulse's and Pure's code layouts into files.
- The seven unnamed mode ids on `g_GameState` (`0`, `6`, `7`, `11`, `13`, `14`, `15`). Speed Lap and Zone draw from the default deck in this build, chosen, not measured.
- The writer of the Fury-content flag at `0x00b979fd`; this build assumes `true` for every HD source.
- Why one boot constructs two or three loading screens, only the last matching the frame.
- Pulse's and Pure's code-laid loading screens are not yet files: their layouts still sit in code (`LoadingScreen_HD.toml` is the only loading layout); `Title::loading.layout` is `None` for them.
- The loading layout is picked at a 16:9 default aspect; no per-aspect `[[layout]]` is authored (HD draws one 1920x1080 grid).
- A track's `[loading]` caption/feature `string_id`s resolve only against the title layout's ids.
- The paragraph wrap rule (a line plus its trailing space inside the width) is inferred from the pair, not read from the executable.

## Next Steps

- Read the second `Title` draw at `0x002b6e18`/`0x002b6e48` and the tournament block `0x002b778c`.

- Name the seven ids by the other places `g_GameState`'s mode picks a branch.
- Follow the `[PARAM]` sites `0x0021ba9c`, `0x0005b3c8`, `0x00229a58` to the flag's writer.
