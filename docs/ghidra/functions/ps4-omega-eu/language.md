# Language plugin choice: `Language_ChoosePluginName`

2026-10-06. Wipeout Omega Collection EU `eboot.bin` (`ps4-omega-eu`,
`CUSA05670`), image base `0x01000000`, 1.00 base (the program Ghidra holds);
the 1.07 patch's table was read from its file. Question: does the executable
walk a manifest? **No: one plugin is chosen from the PS4 system language and
a region mask.**

## `Language_ChoosePluginName` - `0x0174ca60`

**Confidence: 90.** `sceSystemServiceParamGetInt(1)` gives the console
language. It then walks the 23 rows of `Language_PluginTable` for the first
whose id equals it and whose mask ANDs non-zero with `Language_RegionMask`. No
match falls back per region: mask 1 to row 0 (`English`, console id 18), 2 to
id 1, 4 to id 0, 8 to id 18, 16 to id 11. Caller: `FrontendRoot_Construct`
(`0x013c28f0`, reference `0x013c3d53`) formats the result into
`Data/Plugins/languages\%s` (`0x01819271`).

## `Language_PluginTable` - `0x01947160` (data)

**Confidence: 90.** Rows of 24 bytes: console id (int), index (int), name
pointer, region mask (int), pad. In file order: English (18, mask 13), Japanese
(0, 4), American (1, 2), French (2, 1), Spanish (3, 1), German (4, 1), Italian
(5, 1), Dutch (6, 1), Portuguese (7, 1), Russian (8, 1), Korean (9, 8),
TraditionalChinese (10, 8), Chinese (11, 16), Finnish (12, 1), Swedish (13, 1),
Danish (14, 1), Norwegian (15, 1), Polish (16, 1), PortugueseBR (17, 2),
Turkish (19, 0), SpanishLA (20, 2), Arabic (21, 0), French again (22, 3).
Masks: 1 EU, 2 US, 4 Japan, 8 Asia, 16 China.

**The 1.07 patch's table** (file offset `0x9ba330`; base `0x945250`) is the
same except **Turkish carries mask 1**, so the patched EU release offers it.
The patch is mandatory and its `param.sfo` adds `TITLE_19`.

## `Language_RegionMask` - `0x02037c98` (data)

**Confidence: 85.** `Game_Main` (`0x0163b920`) stores
`sceAppContentAppParamGetInt(1)` there at `0x0163ba10`, which is `param.sfo`'s
`USER_DEFINED_PARAM_1`. Both EU packages (`data/images/omega-ps4-eu.pkg` and the
patch) carry **1**.

## What it means for the project

EU reaches fourteen: English, French, Spanish, German, Italian, Dutch,
Portuguese, Russian, Finnish, Swedish, Danish, Norwegian, Polish, Turkish.
`portuguesebr` (mask 2) is not reachable on an EU package; it is listed last
anyway (chosen, not measured) so the project's Brazilian Portuguese stands on
the disc's own text for it instead of a copy of English. Retail's EU store page lists twelve, which neither table gives:
corroboration only, not matched. Listed by
`oag_omega::frontend::LANGUAGE_MANIFESTS` under `CUSA-05670`, in the table's
own order.

This project's PS4 extract keeps no `param.sfo`, so the source reports no serial
and `FrontEnd::assumed_release` names the EU release for it (chosen, not
measured). An extract that kept it would key itself.

## 2048 check

**Checked, differs in the key, same in shape.** 2048 picks from a mode word
stored by the executable (EU 0, US 1) rather than a `param.sfo` value.
Omega's EU list matches 2048 EU's thirteen plus Turkish.
