# Which HUD layout Omega's race managers read

Functions in `eboot.bin` (WipEout: Omega Collection, PS4, `CUSA05670`, EU), image
base `0x01000000`, Ghidra program `/omega/eboot-ps4-omega-eu.bin`. Read
2026-10-06 for the `omega-hud` lane. No names are applied (`names.tsv` is the
Pulse database's): the race-manager names below are the ones the Ghidra
database already carries, and the others stay `FUN_`.

## The literals and who reads them

**Confidence: 80** that a manager reads the layout beside it (a direct data
xref from inside the constructor to the string); **40** on what the
"HUD Style" check's other values mean beyond the strings next to them.

| Literal | Data xrefs from |
| --- | --- |
| `Data\XML\Arcade_HUD.xml` (`0x018128f6`) | `RaceManager_ConstructArcadeHud` (`0x0129c58f`), `DemoRaceManager_Construct` (`0x0125baff`), `MPTimeTrialRaceManager_Construct` (`0x0127edff`), `FUN_01284af0` |
| `Data\XML\TimeTrial_HUD.xml` (`0x018133ac`) | `SPFreePlayRaceManager_Construct` (`0x012a99ef`), `AIBatchRaceManager_Construct` (`0x0124f13f`), `SPTimeTrialRaceManager_Construct` (`0x012afaef`) |
| `Data\XML\SpeedLap_HUD.xml` (`0x0181334b`) | `SPTimeTrialRaceManager_Construct` (`0x012afab2`) - both files in one constructor, chosen by a runtime flag, as on 2048 |
| `Data\XML\Zone_HUD.xml` (`0x01813574`) | `SPZoneRaceManager_Construct` (`0x012b4d3f`) |
| `Data\XML\Elimination_HUD.xml` (`0x01813081`) | `SPEliminationRaceManager_Construct` (`0x012a836f`), `MPEliminationRaceManager_Construct` (`0x0127b23f`) |
| `Data\XML\2048_hud\Arcade_HUD.xml` (`0x0182911b`), `...\Zone_HUD.xml` (`0x018295e4`) | `FUN_015ae4d0`, `FUN_015ac880`, `FUN_015bc700`: no direct caller; each is referenced from vtable data (`0x018c2380`, `0x018c216c`, `0x01925ca8`, ...) - a class of its own, the 2048-lineage managers |

## The skin check inside `RaceManager_ConstructArcadeHud`

The function compares a settings entry (the CRC of the string `HUD Style`)
against `WIP3OUT` and a second literal at `0x018244b6`, and picks the path:
`Data\XML\wo3_HUD\Arcade_HUD.xml` for the first,
`Data\XML\2097_HUD\Arcade_HUD.xml` for the second, **the bare
`Data\XML\Arcade_HUD.xml` for anything else**. A single-screen race takes the
bare name, a split-screen one `SplitScreen_hud\SplitScreen_HUD_0/1.xml` (or
`Vert_SplitScreen_HUD_0/1.xml`), in the same three skins. This is HD's
three-skin split, so HD's own `docs/ghidra/functions/ps3-hdfury-eu/race-hud.md`
reading carries: the bare root is the default skin.

## What it settles, and what it does not

- Settled: the five root layouts a single-player race reads are the bare
  `Data\XML\{Arcade,Elimination,TimeTrial,SpeedLap,Zone}_HUD.xml`, which is what
  `oag_omega::hud::LAYOUTS` names. The file spellings are the executable's own.
- Not settled: **which race selects the `2048_hud\` set.** Its constructors
  are reached through virtual dispatch only. `Title::hud` is one set per
  title, so a circuit-by-circuit choice (as `oag_omega::race::DEFAULTS` makes
  for the particle directory, `particle-paths.md`) is an open item, not a
  guess.
- Not read: the executable's HUD text and shield runtime (`RUNTIME`, the
  always-on widget list). `oag_omega::hud::ART` carries HD's readings for
  those, **chosen, not measured**.
