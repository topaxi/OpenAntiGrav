# HD's loading screen is recovered; the seven unnamed mode ids and one flag writer are left

2026-10-04. Pages: [hd-loading.md](../../docs/formats/hd-loading.md), [loading-screen.md](../../docs/ghidra/functions/ps3-hdfury-eu/loading-screen.md) (its 2026-10-04 section carries the evidence), mode enum on [mode-manager.md](../../docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md). Caption + circuit name, a feature **drawn per screen from the mode's own deck** (`oag_hd::loading::DECK`), a bar, tinted by four `FEGlobals`. **Closed 2026-10-04**: the byte at `0x00b979fd` is the Fury-content flag (read `1` live; picks a deck of 4 or 3); the three title ids are `MAN_2_BR`, `MAN_2_SS`, `IG_HUD_ABSORB` and the heading over the prose is `FE_INSTRUCTIONS` / `ONL_CON_DESC`, all drawn now; `HD_LightGrey` is the unfilled dots of the bar (88, from a live fade-in frame); `FUN_006762f8` is `rand()`. **Corroboration worth knowing**: two of the five feature descriptions (`FE_ABSORB_INST`, `FE_FLIP_INST`) are in `DATA06`'s string table only. **The trap**: the `LSAD_*` ad frames sit in the same executable string-run and were taken for the whole screen until RPCS3 disproved it.

## Open

- The seven unnamed mode ids on `g_GameState` (`0`, `6`, `7`, `11`, `13`, `14`, `15`). The loading screen shows `6`, `0xd` and `0xe` carry decks of their own, so they are real modes; which is Zone, Zone Battle or Detonator is unread. Speed Lap and Zone draw from the default deck in this build, chosen, not measured.
- The writer of the Fury-content flag at `0x00b979fd`: no store through its own TOC slot, and it is passed by address to about twenty functions. This build assumes `true` for every HD source (the one disc in hand reads `1`); a base-game-only source would read `0`.
- Why one boot constructs two or three loading screens, only the last matching the frame.
- The dotted bar texture and the real fill (`race::load` reports no progress).

## Next Steps

- Name the seven ids by the other places `g_GameState`'s mode picks a branch (`Environment_LoadStageTextures` already separates `0xe`).
- Follow the `[PARAM]` sites `0x0021ba9c`, `0x0005b3c8`, `0x00229a58` to the flag's writer.
