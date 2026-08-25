# HD's loading screen is recovered from its own constructor; three threads left

2026-08-24. Pages: [hd-loading.md](../docs/formats/hd-loading.md), [loading-screen.md](../docs/ghidra/functions/ps3-hdfury-eu/loading-screen.md), mode enum on [mode-manager.md](../docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md). Caption + circuit name, a feature **drawn** per screen out of five in two stylings, a bar, tinted by four `FEGlobals` the constructor at `0x002b3bb0` names - roles from the draw function's usage counts (`HD_Grey` ~10 reads, the rest once each), not from the names, which mislead. Reading it named `g_GameState` and 11 of its 22 mode ids. **Open**: what `HD_LightGrey` colours (one element, translucent, drawn as the bar trough at confidence 55); the byte at `*0x00b979fc + 1` picking a deck of 3 or 4 (arithmetic recovered, object read from ~20 places); the seven unnamed mode ids; three feature title ids left unguessed. **Corroboration worth knowing**: two of the five feature descriptions (`FE_ABSORB_INST`, `FE_FLIP_INST`) are in `DATA06`'s string table only - the same copy that carries all 28 circuit names, arrived at from a second direction. **The trap**: the `LSAD_*` ad frames sit in the same executable string-run and were taken for the whole screen until RPCS3 disproved it.

## Open

- What `HD_LightGrey` colours (confidence 55, one element, translucent)
- What the byte at `*0x00b979fc + 1` picks between a deck of 3 or 4 (arithmetic recovered, object read from ~20 places)
- The seven unnamed mode ids on `g_GameState`
- Three feature title ids left unguessed

## Next Steps

- No next step named in the original record - read the prose above and decide one.
