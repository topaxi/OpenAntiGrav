# Language plugin choice: `Language_ChoosePluginName`

2026-10-06. Wipeout 2048 EU v1.04 `eboot.elf` (`vita-2048-eu-v104`), image base
`0x81000000`. Question: does the executable walk a manifest of language
plugins, as Pulse's and Pure's do? **No: it picks one plugin from the Vita
system language, and the list a release offers is what that choice can reach.**

## `Language_ChoosePluginName` - `0x8122eac0`

**Confidence: 90.** Returns one language name from a mode word and the Vita
system language (`SceAppUtil_5DFB9CA0(1, &id)`, system parameter 1).

- Mode 0 (the EU executable): id 2 French, 3 Spanish, 4 German, 5 Italian,
  6 Dutch, 7 Portuguese, 8 Russian, 12 Finnish, 13 Swedish, 14 Danish,
  15 Norwegian, 16 Polish, and **every other id returns `"English"`** (0
  Japanese, 1 American English, 9 Korean, 10/11 Chinese, 17 and up). Thirteen
  names. `american`, `japanese`, `korean` and `traditionalchinese` ship in
  `data.psarc` and cannot be returned.
- Mode 1 (the US executable): id 2 `"French"`, id 3 `"Spanish"`, otherwise
  `"American"`.
- Modes 2 (`"Japanese"`, or `"English"` for the English ids) and 3
  (`"English"`) are in the code and no executable held here stores them.

Caller: `FrontendRoot_Construct` (`0x8106322a`, reference at `0x81063b48`)
formats the name into `Data/Plugins/languages\%s` (string `0x8142afa8`), logs
`"Setting languages : %s"`, and loads that single plugin. There is no picker
screen on this path.

## `Language_RegionMode` - `0x81541318` (data)

**Confidence: 85.** The mode word. `Game_Main` (`0x81003dd2`) stores it once, at
`0x81003e00` (`movs r7,#N ; str r7,[r0]`), and nothing else writes it.

Measured on two executables: the EU v1.04 elf has `00 27` at file offset
`0x4dfe`, the US v1.04 elf (`PCSA00015`) has `01 27`. Diffing the two files
shows the routine otherwise identical, so **EU is mode 0 and US is mode 1**.

## What it means for the project

`oag_2048::frontend::LANGUAGE_MANIFESTS` lists, per `TITLE_ID` read from the
extracted package's `sce_sys/param.sfo` (`PCSF-00007`, `PCSA-00015`), the
plugins that choice can reach. EU: English, French, Spanish, German, Italian,
Dutch, Portuguese, Russian, Finnish, Swedish, Danish, Norwegian, Polish. US:
American, French, Spanish. The picker order is **chosen, not measured** (the
original has no picker): the default language first, then the console's id
order. Retail's EU listing of thirteen languages (no American, Japanese,
Korean, Traditional Chinese) matches the executable; corroboration only.

Pinned by `language_offered_ground_truth`.

## Omega / HD check

**Checked, differs in the key, same in shape.** Omega and HD follow the system
language the same way; Omega's region comes from `param.sfo`
(`oag-omega`'s page), HD's from a variable (`ps3-hdfury-eu/language.md`).
