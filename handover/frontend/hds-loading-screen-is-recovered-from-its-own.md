# HD's loading screen is recovered; the seven unnamed mode ids and one flag writer are left

2026-10-05. Pages: [hd-loading.md](../../docs/formats/hd-loading.md), [loading-screen.md](../../docs/ghidra/functions/ps3-hdfury-eu/loading-screen.md) (its 2026-10-04 and 2026-10-05 sections carry the evidence), mode enum on [mode-manager.md](../../docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md). Caption + circuit name, a feature **drawn per screen from the mode's own deck** (`oag_hd::loading::DECK`), tinted by four `FEGlobals`. **Closed 2026-10-05**: the bar is a 166-column grid of `dot.gtf` (`Draw::TiledSprite`), lit in `HD_Blue` (the live dots peak at exactly `0xffac0717`, so the red is the palette, not a team colour) over `HD_LightGrey`, in whole columns; the five labels (`WIPEOUT® HD`, `FEATURE IMAGE`, `FEATURE DESCRIPTION`, `PROGRESSION BAR`, `MODE ICON`) are the game's own, not a debug overlay, and are drawn behind `square.gtf` bullets; the feature image's red is the `_fury` art, not a tint pass; `--loading-screen --track` names the circuit instead of printing the path. **Closed 2026-10-04**: the Fury-content flag, the five title ids, `HD_LightGrey`, `FUN_006762f8` is `rand()`. **The trap**: the `LSAD_*` ad frames sit in the same executable string-run and were taken for the whole screen until RPCS3 disproved it.

## Open

- **The bar's fill is still a time estimate.** The original's is load-driven: target 0.2 at construction, then milestones 0.4, 0.5, 0.75, 0.95 through `LoadingScreen_SetProgressTarget` (`0x002b2c60`), eased 0.1 column per frame. `race::load` (986 lines, at the size ratchet) reports no stage, so wiring it means stage reporting out of `race::load` first. Six callers are listed in loading-screen.md; `0x0067a858` also calls it, argument unread.
- **The mode icon** is a Bink clip bound from a video widget; the panel is drawn with its label and brackets and no icon.
- The bar has 21 dot rows against the original's 30 (chosen to fit this layout).
- The seven unnamed mode ids on `g_GameState` (`0`, `6`, `7`, `11`, `13`, `14`, `15`). Speed Lap and Zone draw from the default deck in this build, chosen, not measured.
- The writer of the Fury-content flag at `0x00b979fd`; this build assumes `true` for every HD source.
- Why one boot constructs two or three loading screens, only the last matching the frame.

## Next Steps

- Add stage milestones to `race::load` and feed them to the bar through `LoadingWorker::progress`.
- Name the seven ids by the other places `g_GameState`'s mode picks a branch.
- Follow the `[PARAM]` sites `0x0021ba9c`, `0x0005b3c8`, `0x00229a58` to the flag's writer.
