# Language plugin choice: `Language_ChoosePluginName`

2026-10-06. Wipeout HD EU `EBOOT.elf` (`ps3-hdfury-eu`, `BCES-00664`), PowerPC
64, TOC `r2 = 0x8bd3c4` (read from the `.opd` descriptor at `0x886ba0`).
Question: does the executable walk a manifest? **No: one plugin is chosen from
the PS3 system language and a region variable.**

## `Language_ChoosePluginName` - `0x003436b8`

**Confidence: 85** on the mapping, **70** on which region value is Europe.
Calls `cellSysutilGetSystemParamInt(0x111)` (the system language) and returns a
name string, which the boot formats into `Data\Plugins\languages\%s` (string
`0x007869c0`).

Jump table (`0x343734`, index = console language id, valid for ids 0 to 15):
2 French, 3 Spanish, 4 German, 5 Italian, 6 Dutch, 7 Portuguese, 8 Russian,
12 Finnish, 13 Swedish, 14 Danish, 15 Norwegian. Ids 0, 1, 9, 10, 11 and
anything above 15 take the default: `American` when the region variable is 1,
`English` otherwise. Region 2 returns `English` for id 1 and `Japanese`
otherwise. Region 3 returns `English` for id 1, `TraditionalChinese` for 10
and 11, `Korean` otherwise. The name strings are the TOC slots at
`0x8b5a84` to `0x8b5ac4` (`0x7aa6f0` onward).

## `Language_Region` - `0x00938564` (data, bss)

**Confidence: 70.** The region variable. The boot logs `setting region to
europe / usa / asia/korea / japan` (strings `0x779508` to `0x779560`). Region 1
selects `American` (USA), 2 selects Japanese (Japan) and 3 selects Korean and
Chinese (Asia/Korea); Europe is the value left over, so **a European release
reaches `English` and never `American`**. The writer of the variable was not
found (four in-range TOC slots for it were checked; none is a store), so the EU
value is by elimination. Open: `FUN_00235190` (`0x235190`) shows
`AmericanLegalLine` when the variable is 2.

## What it means for the project

EU reaches twelve of the sixteen plugins: English, French, Spanish, German,
Italian, Dutch, Portuguese, Russian, Finnish, Swedish, Danish, Norwegian.
`american`, `japanese`, `korean` and `traditionalchinese` ship on the disc and
are not reachable. No Polish plugin exists on the disc. Listed by
`oag_hd::frontend::LANGUAGE_MANIFESTS` under `BCES-00664`; order chosen, not
measured. Pinned by `language_offered_ground_truth`.

The HD retail EU listing is unknown, so there is no corroboration.
