# Where an Omega race reads its particle effects from

Functions in `eboot.bin` (WipEout: Omega Collection, PS4, `CUSA05670`, EU),
image base `0x01000000`. Read 2026-10-05 for the `omega-particles` lane.

## The path builder - `0x01372ee0` and 13 siblings

**Confidence: 85** for the choice, **40** for what the flag means. The string
`Data/Particles2048/%s` (`0x018328f0`) has 29 references in seven functions
(`FUN_017231d0`, `FUN_017691d0`, `FUN_01372ee0`, `FUN_01713130`, `FUN_01711af0`,
`FUN_01710100` and others), each the same shape: `snprintf` into a 128-byte
buffer (`DAT_0204ff10`) with `Data/Particles/%s` when the u32 at `0x01f99bc0`
is 1 and `Data/Particles2048/%s` otherwise, then a load call per name
(`FUN_016d7af0`). The effect-name list itself is picked by `DAT_01f999e4 < 0x17`
(23), the same game-mode threshold 2048's own builder tests
(`../vita-2048-eu-v104/particle-paths.md`).

**This is the opposite polarity to 2048's**: 2048 asks "mode below 23" for the
2048 set; Omega asks "flag is 1" for the plain `Data/Particles` set.

## The flag - `0x01f99bc0`

Written in four places: `Game_Main` (`0x0163cd35` writes 1 and `0x01f99bc4 = 3`,
`0x0163d0f9` writes 0 and `0x01f99bc4 = 1` - two launch-argument handlers),
and `FUN_0124f4a0` (`0x0124f73c` / `0x0124f751`), which picks an entry from a
list by name and sets the flag to 1 when that entry's byte at `+0x24d` is
non-zero, 0 otherwise, calling `0x0160e430(flag)` after. `+0x24d` is set to 1
by `FUN_01510900` and to 0 by `FUN_01510b70`.

**Not read: which circuit or mode leaves the flag at 1.** The port therefore
chooses by the circuit's own directory (`oag_omega::race::DEFAULTS`,
`effect_dir_by_circuit`): `Data\environments2048\*` plays `Data\particles2048`,
everything else (the HD-heritage `environments\*` circuits, Zone) plays
`Data\particles`. **Chosen, not measured.** No names are applied: none of these
functions is recovered well enough to name.

## What ships

See `docs/formats/pob.md`, "Wipeout: Omega Collection", for the census.
