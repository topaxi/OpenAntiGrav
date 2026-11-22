# Omega: Detonator's hull, handling file and object models

This is the Omega check of
[`ps3-hdfury-eu/detonator-hull.md`](../ps3-hdfury-eu/detonator-hull.md). **Every address on this
page is in the 1.07 patch `eboot.bin`** (Ghidra program `/omega/eboot-ps4-omega-eu-v107.bin`,
image base `0x01000000`), not the base build this directory's `names.tsv` uses
([patch-v107-addresses.md](patch-v107-addresses.md)). That is why nothing here is renamed. Static
decompile and disassembly only, 2026-11-22. There is no memory readout of a live Omega race, so
nothing here was observed running.

## The hull: v1.07 `0x0131b1f0` (base `Ship_LoadModelSet`, `0x01301ba0`)

The base function is named on [zone-craft.md](zone-craft.md) (75). Its game mode switch is on
`DAT_0201280c` (v1.07). For `case 0xe`, with the craft entry's VR byte `+0x74d` clear, the base
directory is `sprintf("%s\Detonator", "Data\Art\Published\HDShips")` and the hull is
`"%s\Detonator\Ship.vex"`. For modes 6 and `0xe` (`(mode | 8) == 0xe`), the code pushes the livery
pair: key `zoneship_team` (VR: `detonator_vr/zoneship_vr_hd`), value the entry's name at `+0x228`.
It then loads `%s\shipwreck.vex` and, for `0xe`, `%s\ship_deathshell.vex` (Zone gets
`%s\Zoneship_dead\zoneship_dead_whiteshell.vex`), followed by `locators.vex` and `engineflare.vex`.
The mode mask is `0x8181 >> (mode - 6)`, modes {6, 0xd, 0xe, 0x15}. **75.**

**Checked, applies, not wired; the key differs** (`zoneship_team`, as Omega's Zone). The census
agrees with that key. `data03.psarc` `hdships/detonator/ship.rcsmodel` requests
`.../detonator/zoneship_team/team.gnf` plus eight shared `.../detonator/zoneship_zone/detonator_*.gnf`.
The twelve `Zoneship_<Team>/` folders carry only `Team.gnf`. `data09.psarc`'s
`Data\Plugins\teams\Definition.xml` authors fourteen `PI_TeamModel name="detonator"`
(`texturelocation="zoneship_<team>"`, `modellocation="Data\art\published\hdships\Detonator"`).
`data08.psarc` adds a `Detonator_VR/` hull for the VR mode.

## The handling file: v1.07 `0x014af2a0`

At `0x014af60f`-`0x014af627` the function tests `DAT_0201280c == 0xd` -> `...\Zone Battle\...`, then
`== 0xe` -> `data\art\published\hdships\Detonator\handlingstats.xml` (`0x01880fea`). Otherwise, with
the `0x8181` mask -> `...\hdships\Zone\handlingstats.xml` (`0x014af64f`-`0x014af697`). **75.**
`hdships/detonator/handlingstats.xml` (`data03`) and `Detonator_VR/handlingstats.xml` (`data08`)
are byte-identical to HD's Detonator file, and `hdships/zone/handlingstats.xml` (`data01`) to HD's
Zone file. **Ported** (`oag_omega::race::DETONATOR_HANDLING`,
`the_detonator_hull_handling_and_object_entries_resolve`; unauthored values inherited from HD).

Engine lights: v1.07 `0x01335030` reads `Data\art\published\hdships\Zone\EngineLightData.xml` under
the same `0x8181` mask, as HD does.

## Object models

The v1.07 strings `mine_rita_football.vex`/`mine_rita_spikes.vex`/`Electric_Visualiser.vex`
(referenced from `0x01364870`), `detonator_bomb_final.vex` and `_final_equalisers`/`_glowedges`
(from `0x0135f380`), and `detonator_cannonbolt.vex`/`hd_muzzleflash_detonator.vex` (from
`0x0135a740`) are HD's names. The models ship in `data00.psarc` under `weapons/` and `weapons2048/`.
**Ported**: `oag_omega::race::DETONATOR.objects` names HD's entries and the test resolves each.

**Pad: checked, differs.** v1.07 has no `Detonator_Pad` string. In mode `0xe`, the world loader
(`0x013c47f0`) formats `"%s.vex"`/`"%s.rcsmodel"` over a path held at `DAT_020128f8 + 0x288`, a data
field whose writer was not read. `Data/new_mode_pads/Detonator_Pad/Detonator_Pad.vex` ships in
`data00.psarc`. **60.**

There is no string naming `detonator_pickup` or `kaleidoscopic`, as on HD.
