# Omega's per-craft rocket and mine counts: the reader, the clamps, the default

2026-11-03, lane `rocket-count-data`. Binary `eboot.bin` (Omega EU base package, image base `0x01000000`, Ghidra program
`/omega/eboot-ps4-omega-eu.bin`). Static read only, no live capture. The question: 2048 and Omega author
`number_of_rockets` and `number_of_mines` in a craft's own `handlingstats.xml`
(`<Stats><BespokeStats><Weapons number_of_rockets="2" number_of_mines="4" .../>`); what does the executable do with them, and
what does a craft that leaves them out get?

## The reader

`FUN_012d8e30` (`0x012d8e30`) parses a `handlingstats.xml` into a record, `param_1`, by a flat `strcasecmp` chain over the
attribute names. Called as `FUN_012d8e30(&DAT_020fdca8, "Data\\XML\\HandlingStats.xml")` for the engine-wide file and as
`FUN_012d8e30(craft_record + 0x26c, <team>\\handlingstats.xml)` for a craft (`FUN_01309e90`, `0x01309e90`; also
`FUN_01488670`). The strings are at `0x01814a37` (`number_of_rockets`) and `0x01814a49` (`number_of_mines`), each with
one code xref, `0x012dcb2a` and `0x012dcb41`.

| Attribute | Law (`FUN_012d8e30`) | Slot |
| --- | --- | --- |
| `number_of_rockets` | decimal text to float, `(int)` truncation, `< 1` becomes 1, `> 3` becomes 3 | `param_1[0x128]`, record offset `0x70c` |
| `number_of_mines` | the same, `< 1` becomes 1, `> 5` becomes 5 | `param_1[0x129]`, record offset `0x710` |

Confidence 90 for the two clamps (read off the decompilation, both arms).

## The default

A craft that authors neither attribute (2048 and Omega's Fighter, and the Prototype of Auricom and Qirex, which author only
`gift_cannon_time`) keeps what its record was constructed with. `FUN_01485b60` (`0x01485b60`, reached through a vtable data
xref, not a call) writes, in one run of stores,
`[R15+0x704]=1.0f`, `[R15+0x708]=1.0f`, **`[R15+0x70c]=3`**, **`[R15+0x710]=5`**, `[R15+0x714]=0x1fff` (`0x01485e14`-`0x01485e40`):
**three rockets and five mines**. The engine-wide `HandlingStats.xml` authors no `<Weapons>` line, so nothing overlays the
default (checked on Omega's copy: no `number_of_` attribute in it).

Confidence 80: the offsets agree with the reader (`0x26c + 0x4a0 = 0x70c`, `0x26c + 0x4a4 = 0x710`), and the record is
constructed before the parse; what was not shown is the consumer of `+0x70c` / `+0x710` (the fire and drop code), so the
loop the original runs for a count below three is **unread**. Mines at five equals Pulse's literal `CLUSTER`
([`mine.md`](../psp-pulse-usa/mine.md)); rockets at three equals Pulse's three spawn calls
([`weapon-fire.md`](../psp-pulse-usa/weapon-fire.md)).

Named `HandlingStats_ParseXml` at `0x012d8e30`, confidence 75: it is the one reader called on `Data\\XML\\HandlingStats.xml`
and on every craft's `handlingstats.xml`, and the two attributes above are two of its arms; the rest of its chain (cameras,
engine, tilt) was not walked.

## Other titles

- **2048 (Vita, v1.04 `eboot.elf`)**: ported for the reader. `number_of_rockets` is at `0x8148fd98`, its one reader is
  `FUN_811abe80`, and the same clamps are there (`< 1`, `3 <` for rockets; `< 1`, `5 <` for mines). The default constructor
  was not located: the Omega one stands for it (Omega's twenty folders equal 2048's byte for byte,
  [`2048-status.md`](../../../formats/2048-status.md)).
- **Pulse, Pure, PS2 Pulse, HD**: **checked, differs**: neither string is in the executable (`BOOT.BIN`, `SCES_547.48`,
  `EBOOT.elf`), and no team file of Pure, HD or Pulse's default team authors the element
  (`remix_2048_sources_ground_truth::the_other_titles_author_no_weapon_counts`). Their fixed three rockets and five mines
  are the same numbers as this default.
- **Omega patch 1.07**: the string moved to `0x01876b62` / `0x01876b74` with the rest of the patch; not re-read.

## What the data says

Read by `oag_weapons::counts::WeaponCounts::from_stats` over all twenty native craft on both sources
(`remix_2048_sources_ground_truth::rocket_and_mine_counts_per_craft`): Fighter 3 / 5 (default), Agility 2 / 4, Speed 1 / 3 on all
five teams; Prototype AG Systems 2 / 4, Feisar and Piranha 1 / 3, Auricom and Qirex the default 3 / 5. Omega's equal 2048's.
