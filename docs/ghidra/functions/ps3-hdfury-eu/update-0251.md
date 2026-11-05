# The 02.51 executable against 02.00: what carries over

2026-11-05, lane `fury-0251-verify`. A third party sent notes from a static recompilation of
HD Fury 02.51, with a map of 5,097 function pairs from our 02.00 addresses to theirs. This page
is that map checked against both ELFs; every other project page stays on the disc's 02.00.
Packages and archives: [hd-updates.md](../../../formats/hd-updates.md).

**02.51 is in the Ghidra project** as `/hdfury/EBOOT-ps3-hdfury-eu-0251` (name `EBOOT.elf`, the
same as 02.00's, so address it **by path**), decrypted with `rpcs3 --decrypt`, imported through
the steps of [toolchain.md](../../../reverse-engineering/toolchain.md#ps3): 16,133 functions.

## The executable was rebuilt, not patched

| | 02.00 / 02.10 | 02.30 / 02.50 / 02.51 |
| --- | --- | --- |
| `EBOOT.elf` | 9.63 MB | 7.55 to 7.69 MB |
| Text segment | `0x848e48` | `0x6ea3e8` (02.51) |
| Function descriptors found | 24,892 | 7,972 |
| Ghidra functions | 26,112 | 16,133 |
| Entry descriptor | `0x00870530` | `0x0070f278` (02.51) |

Many virtual slots collapse onto one body (`0x000109d0` serves eight of the widget's first 26),
so the linker folded identical code; our reading, confidence 70. Few functions keep their bytes.
`DFEngine` is in 02.00, 02.10 and 02.30 and gone from 02.50 and 02.51, confirmed by string.

## The map

Direct byte comparison says little after a rebuild, so the test is the call graph: for a pair,
do the old function's calls, sent through the map, land on the new function's calls?
`pairs` have at least one mapped call; the rest have none to test.

| Class | Pairs tested | Calls hit | Pairs at 80 percent or more |
| --- | --- | --- | --- |
| `masked` | 364 | 99.8 % | 99.5 % |
| `exact` | 50 | 99.2 % | 98.0 % |
| `fuzzy` | 71 | 95.6 % | 95.8 % |
| `callgraph` | 1,035 | 93.0 % | 90.8 % |
| `caller` | 317 | 89.7 % | 90.2 % |
| `string` | 296 | 88.5 % | 79.1 % |
| `order` | 185 | 86.3 % | 84.9 % |
| `strings` | 598 | 83.3 % | 83.8 % |
| All | 2,916 of 5,097 | **89.2 %** (25,010 of 28,044) | 89.1 % |

The same test on a shuffled map hits **0.9 %**. The map is injective. **So the map is good and
not perfect.** Confidence 85 for `masked`, `exact` and `fuzzy`; treat the rest as a lead.
Their 388 carried names all match `names.tsv` at the old address, 21 of them below 70.

**A checked error in `order`:** the widget's vtable is `0x008652b8` on 02.00 and `0x00702e78` on
02.51, and slot by slot it maps `0x00327040`, `0x00327228`, `0x00183b38`, `0x00186b90` and
`0x0017e810` as the map says. Slot 12 does not: `0x00180ff8` on one side and `0x000a4e1c` on the
other, which the map gives to `0x00182358`. Slot 7 is `BackgroundAnimFury_Render`, `0x00183c88`
to `0x000a64bc`, a pair the map lacks.

**Names are not imported.** [names.tsv](names.tsv) is the 02.00 file. Nothing from this map went in,
below or above 70.

## The offsets

| Claim | Evidence | Verdict |
| --- | --- | --- |
| Widget fields `+0x20` | `stw r0, 0x248(r26)` in `PickPath` at `0x000a3004` against `+0x228`; `addi r29, r29, 5732` in `OnEnable` against 5700 | Confirmed, 90 |
| `g_FurySettings` fields `+0x30` | `lwz r24, -0x51dc(r2)`, then `0x444` at `0x000a3010` and `0x000a314c`, `0x448` at `0x000a301c` and `0x000a30b0` | Confirmed, 90 |
| Vtable `0x00702e78` | Slot 21 `OnEnable`, slot 18 `ParseScreenSetting` | Confirmed, 90 |
| Boot needs firmware | `PS3_SYSTEM_VER` 04.00; RPCS3 refuses on 2.76 | Our own finding |

**Running it:** 02.51 boots on RPCS3 0.0.42-19980 with firmware 2.76 only after `PS3_SYSTEM_VER`
is edited in a private `PARAM.SFO`, and then sits on the `Sony Computer Entertainment presents`
logo for over 9 minutes with a live RSX (`APP_VER=02.51` in the log, screen `Top`). So nothing
here is a live 02.51 reading. Next step: firmware 4.x in a private tree.
