# Omega: Zone flies one shared hull, `hdships\Zone`, for every craft

Function in `eboot.bin` (WipEout: Omega Collection, PS4, EU), Ghidra program
`/omega/eboot-ps4-omega-eu.bin`, image base `0x01000000`. The same finding as
[`vita-2048-eu-v104/zone-craft.md`](../vita-2048-eu-v104/zone-craft.md), read in
the third recompilation of the `Backend/` tree.

## `Ship_LoadModelSet` - `0x01301ba0`

**Confidence: 75** (static; one source tree, so the Vita and PS4 reads corroborate
the structure, not the behaviour; no PS4 emulator exists here).

`FUN_01301ba0(ship, team_name)` has the same switch as Vita's `0x811b6dae` on
`DAT_01f999e4` (the game mode): mode 6 formats `%s\Zone` and `%s\Zone\Ship.vex`
over `s_Data_Art_Published_HDShips_0193fd50` (`Data\Art\Published\HDShips`);
`0xd`/`0x15` the `_n1` pair; `0xe` Detonator; otherwise the craft entry's own
directory (`+0x220`, picked as `speed`/`normal` by `DAT_01f99bc0`) `%s\ship.vex`.
Mode 6 never reads `param_2` or the entry. Mode 6 alone loads
`%s\Zoneship_dead\zoneship_dead_whiteshell.vex`, and the `0x8181 >> (mode - 6)`
mask selects exactly modes {6, 0xd, 0xe, 0x15}, the same four special cases as
Vita's switch.

Livery: the entry's name at `+0x218`, or the literal `Zoneship_Faisar` when no
entry is set and the mode is in that mask. The material key differs from Vita's:
Omega assembles it from stack bytes `0x6873656e6f7a 0x7069 0x6d6165745f` =
`zoneship_team` (`zoneship_zone` on Vita). **Checked, differs** (key only; the
mechanism is the same). Omega's per-team Zone liveries are
`hdships/zone/Zoneship_<Team>/Team.gnf`.

## Census

`hdships/zone/` carries in `data01.psarc`: `Ship.vex`, `ship.rcsmodel`,
`Ship_LOD.vex`, `Locators.vex`, `Engineflare.vex`, `shipwreck`, `ship_ghost`,
`Zoneship_dead/*`, `Zoneship_<Team>/Team.gnf` and `handlingstats.xml`. The
18 per-craft 2048 `ship_zone.vex` files are in the archives too; this binary has
no `ship_zone` string (63 `_zone` matches checked), so as on 2048 they are
unreferenced as far as found.

## Open

An Omega **2048-era** craft cannot start a race at all: `feisar2048\3` resolves its
`handlingstats.xml` under `hdships\`, which does not hold it
(`data09.psarc has no entry at Data\art\published\hdships\feisar2048\3\handlingstats.xml`).
That is a roster gap independent of Zone mode, so Omega's Zone hull is shown on
HD-era craft only.
