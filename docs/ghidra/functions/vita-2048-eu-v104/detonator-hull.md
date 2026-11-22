# 2048: Detonator's hull, handling file and object models

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image base `0x81000000`.
This is the 2048 check of [`ps3-hdfury-eu/detonator-hull.md`](../ps3-hdfury-eu/detonator-hull.md).
Static reading only, 2026-11-22: `llvm-objdump -d --triple=thumbv7-none-eabi` on
`data/extracted/vita/PCSF00007/patch-v104/eboot.elf`, with string references found by a
`movw`/`movt` pair scan. There is no Vita emulator in this toolchain, so nothing ran.

## Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x810bfd54` | `Team_LoadHandlingStats` | 75 |

## `Team_LoadHandlingStats` - `0x810bfd54`

HD's `0x001e6ff8` compiled for ARM, with the same shape:

1. The function reads the team's `+0xb8` byte. While it is clear, it registers
   `"%s HANDLING STATS"` (`0x810bfd64`-`0x810bfd9c`).
2. It reads the game mode `[0x8153fc40 + 0xe4]` = `DAT_8153fd24` (`0x810bfda0`-`0x810bfda8`), the
   word [zone-craft.md](zone-craft.md) reads.
3. It picks the file:
   - **`0xe`** -> `data\art\published\hdships\Detonator\handlingstats.xml` (`0x81447900`;
     `0x810bfdac`-`0x810bfdba`);
   - `0xd` -> `...\hdships\Zone Battle\handlingstats.xml` (`0x81447938`; `0x810bfdc6`);
   - otherwise, if `GameMode_IsZoneFamily` (`0x810018d4`, 65) holds and there is no variant
     directory -> `...\hdships\Zone\handlingstats.xml` (`0x81447974`; `0x810bfdf0`);
   - else `%s\handlingstats.xml`.
4. It sets `+0xb8` (`0x810bfdbe`).

**75**: one static read, structurally identical to HD's (82 there) and Omega's. So 2048's Detonator
craft flies Detonator's file, not Zone's.

## Census (`data.psarc`, patch `data2.psarc`)

- **Hull folder:** `data/art/published/hdships/Detonator/` ships the same set as HD's
  `/data/ships/detonator/`: `Ship`, `Ship_LOD`, `Ship_LOD1`, `ship_ghost`, `shipwreck`,
  `Ship_DeathShell`, `shipshield`, `AbsorbEffect`, `LeachEffect`, `Engineflare` and `Locators`
  (`.vex` + `.rcsmodel`), plus `handlingstats.xml`. The patch replaces `Ship.vex`, `ship.rcsmodel`
  and the death shell.
- **Handling:** `hdships/Detonator/handlingstats.xml` is byte-identical to HD's
  (MD5 `140f425a...`), and `hdships/Zone/handlingstats.xml` to HD's Zone file (`7341a427...`).
- **Liveries:** `hdships/Detonator/Zoneship_<Team>/`, for the twelve HD teams plus `Zoneship_Zone`,
  eleven `.gxt` each with HD's names (`Detonator_AO`, `Detonator_N`, ...). The base hull's
  `ship.rcsmodel` requests `.../detonator/zoneship_zone/*.gxt`, and so does the patch's.
- **Definition:** the patch's `Data\Plugins\teams\Definition.xml` authors twelve
  `PI_TeamModel name="detonator"`, `texturelocation="zoneship_<team>"`,
  `modellocation="Data\art\published\hdships\Detonator"`.
- **Objects:** `Weapons2048/` and `Weapons/` ship `mine_rita_football`, `mine_rita_spikes`,
  `electric_visualiser`, `detonator_bomb_final`, `_final_equalisers`, `_glowedges`, the four blast
  models, `detonator_cannonbolt`, `hd_muzzleflash_detonator`, `detonator_pickup` and
  `detonator_mine_explosion_kaleidoscopic`. `New_Mode_Pads/Detonator_Pad/Detonator_Pad` ships too.
  The executable carries the strings `mine_rita_football.vex`, `detonator_bomb_final.vex`,
  `detonator_bomb_glowedges.vex`, `detonator_cannonbolt.vex`, `hd_muzzleflash_detonator.vex`
  and `Data\New_Mode_Pads\Detonator_Pad\Detonator_Pad.vex`, and no `detonator_pickup` or
  `kaleido` string. Which directory prefix the object loaders join was not read.

## Verdicts (CLAUDE.md lineage rule)

- **Hull and livery: ported** (`oag_2048::race::DETONATOR_CRAFT`,
  `the_detonator_hull_handling_and_livery_entries_resolve`). `Ship_LoadModelSet` (`0x811b6dae`,
  [zone-craft.md](zone-craft.md), 75) loads the literal `hdships\Detonator\Ship.vex` in mode `0xe`
  and swaps `zoneship_zone` -> the entry's livery name, the same as HD.
- **Handling: ported** (`DETONATOR_HANDLING`, the same test; unauthored values inherited from HD). The file and the mode test are the same as HD's.
- **Mine, bomb, bolt, flash and pad models: checked, applies, not wired** (2048 draws no weapon
  bodies and the directory prefix is unread: `oag_2048::race::DETONATOR.objects` is `None`) (same entry names in the
  executable and on the disc). The loaders' bodies were not read here.
