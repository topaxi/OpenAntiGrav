# 2048's crossfade system and the SCREAM name hash

2026-10-05, lane `v2048-engine`. Program `/2048/eboot-vita-2048-eu-v104.elf`
(v1.04). The format side is [`2048-xfx.md`](../../../formats/2048-xfx.md); HD's
twin of this code is [`xfade.md`](../ps3-hdfury-eu/xfade.md). Nothing here was
run: it is static reading, corroborated by the data (the hash against 32 names,
the bank choice against every layer of the 23 tables).

## Names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x8125e5d0` | `XFadeSystem_AddCrossFader` | 90 |
| `0x8125ed5a` | `XFadeSystem_SetInput` | 82 |
| `0x8125e03c` | `XFadeSystem_UpdateChannels` | 72 |
| `0x8125d61e` | `XFadeSystem_UpdateLayers` | 72 |
| `0x8125d4b6` | `XFadeSystem_StartLayer` | 68 |
| `0x81263f6c` | `XFadeShip_GetTeamTable` | 80 |
| `0x81264056` | `XFadeShip_InitSystem` | 72 |
| `0x811cb9ac` | `Ship_UpdateEngineCrossfade` | 72 |
| `0x812cd154` | `ShipTeam_UpdateEngineCrossfade` | 68 |
| `0x81352b9c` | `Scream_FindCueByName` | 85 |
| `0x8134bb9e` | `Scream_StartCueByIndex` | 72 |
| `0x8134bca8` | `Scream_StartCueByName` | 72 |

Below 70 the name carries `_q` in Ghidra; the rows in `names.tsv` are the bare
names.

## `XFadeSystem_AddCrossFader` (90)

`XFDX`, then `*(u32 *)(file + 4) & 0xffffff == 0x60000` (the platform byte is not
tested here, where HD's tests it), then the element and controller counts
against `DAT_818c3d04` / `DAT_818c3d08`, the system's maximum elements and
controllers per handle. The system is created by `FUN_8125eea4(sys, 8, 0x20, 5,
0x30)` inside `XFadeShip_InitSystem`: eight handles, `0x20` elements, **five
controllers**, `0x30` triggers. HD's call passes 4.

## `XFadeSystem_SetInput` (82)

`inputs[channel] = value * channel[+0x50] + (short)channel[+0x54] * 0x10000`,
HD's law word for word.

## `XFadeSystem_StartLayer` (68) and the kinds

`*(i8 *)layer` selects: `<= 0` (kind 0) plays a cue: `layer[1] == 0` (an empty
name) calls `Scream_StartCueByIndex(bank, (short)layer[+0x18], ...)`, otherwise
`Scream_StartCueByName(bank, layer + 1, ...)`. `1` calls a registered stream
callback or logs "Attempt to start a stream, but not callback is registered".
`XFadeSystem_UpdateLayers` handles `2` (`< 3`): `element[layer[+0x12]] + 8 =
gain`, `+ 0x34 = pitch offset`, no voice. That is the modulator (72).

## The name hash (85)

`Scream_FindCueByName`: `h = 0; for each byte: h = h * 0x1000193 ^ (signed char)b`,
then the table search `FUN_813529f2`. Confidence 85 for the function, 95 for the
hash itself: 32 names (the HD-era layer names and the weapons bank's `ABSORB`,
`~SHIELD`) land on records in `shipHD.bnk`, `Ship_NGP_Zone.bnk` and
`Weapons_NGP.bnk` whose cue is the one the tables expect. `Scream_StartCueByIndex`
refuses a cue index at or past the bank's `cue_count` (`+0x16`).

## The table names (80)

`XFadeShip_GetTeamTable(sound, team_name)`: the key is the CRC-32 (table at
`0x81521238`, seed `0xffffffff`, no final xor) of the team record's name string,
over a 24-entry cache at `0x818c4920`. A miss formats
`data/audio/sound/xfship_%s.xfx` (`xfship_det.xfx` when the mode id
`DAT_8153fd24` is `0xe`; `xfship_ZONE_%s.xfx` in Zone), loads it, and **falls back
to `xfship_feisar.xfx`** when the file is absent. The team name is the record's
own, so a livery directory never reaches it.

## The ship classes (72, 68)

`FUN_811b8806` (`"%s HANDLING STATS"`, `Backend/Ships/Ship.cpp`) constructs the
base class, vtable `0x81511d00`, whose update slot is `Ship_UpdateEngineCrossfade`
(four `SetInput` calls, channels 0 to 3). Five vtables (`0x815166e4`,
`0x815167f8`, `0x815168d8`, `0x815169f0`, `0x81516ad0`) carry
`ShipTeam_UpdateEngineCrossfade` instead (five `SetInput` calls, channels 0 to
4); it compares the team name against `Feisar2048`, `Qirex2048`,
`AG_Systems2048`, `Auricom2048`, `Piranha2048`. The laws are in
[2048-xfx.md](../../../formats/2048-xfx.md#the-per-tick-law-82-for-the-read-terms).

## The bank loader (72)

`FUN_8121bce2` (a race's audio setup) stores, in `DAT_818c4de0` (the ship bank
every `Scream_Start*` in the ship code reads), `Ship_NGP.bnk` (`0x814a92a4`),
`Ship_NGP_Zone.bnk` (`0x814a9390`) or `ShipHD.bnk` (`0x814a93dc`), by a selection
object's field `+400` (0 to 4) and, with no selection, by the mode id
(8/0x14, 0xd/0x15, 0xe). The branch a base circuit takes is unread.

## What is not recovered

`ship[+0x6098]` (channel 4's source); the gear state machine in
`ShipTeam_UpdateEngineCrossfade` (`ship[+0x76b0..]`, `+0x1d92`, `+0x1dac`), which
plays the `~Ship<N>_*` cues; the class word `DAT_8153fd18`'s writer.
