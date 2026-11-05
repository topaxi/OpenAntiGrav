# HD Fury's update packages

2026-11-05. Sony's four update packages for `BCES00664`, read directly. The project reads the
disc's 02.00; nothing below is loaded by the engine. Addresses and executables:
[update-0251.md](../ghidra/functions/ps3-hdfury-eu/update-0251.md).

A package is a PS3 retail `.pkg`; `rpcs3 --installpkg` installs one, `rpcs3 --decrypt` opens its
`EBOOT.BIN`. **The chain is not cumulative:**

| Package | `EBOOT` | Archive | Other |
| --- | --- | --- | --- |
| 02.10 | yes | `data07.psarc` | |
| 02.30 | yes | `data08.psarc` | `DFEngine.sprx` |
| 02.50 | yes | `data08.psarc` | |
| 02.51 | yes | **none** | |

02.51 alone has neither archive, and its executable names both (`data07.psarc`,
`data08.psarc`), so a complete install is all four in order. A third party's note that 02.51
carries `data08` is the installed state, not the package. It also counts 17 `entries.xml`;
there are **16** languages in `data08`, as in `data07`. Confidence 95: read from the packages.

## What changes against the disc

Compared by entry content, never by file bytes: 02.30's and 02.50's `data08` differ in the header
and in `korean/entries.xml` alone. Entry sizes in both match.

- **`data07`:** eight `environments/*/audioconfig.xml`, the Chinese fonts, `additional_definition.xml`,
  16 languages, `globalaudioconfig.xml`.
- **`data08`:** two `manual_jap_notify` images, `additional_definition.xml`,
  `ingame_definition.xml`, 16 languages.
- **Strings (English):** 2,038 on `DATA06`, 2,039 on `data07`, 2,041 on `data08`. New IDs:
  `ONL_AGE_RESTRICTION`, `3D`, `3D_STRENGTH`. One changed value, `BOOT_HEALTH_WARN_2`. No ID is
  removed, none of the new ones is a race HUD string.
- **`ingame_definition.xml`:** against `DATA00`, the only changes are the removed `InGame 3D
  Settings` screen and a repaired first line (the disc's copy lost its `<`). Race HUD layout is
  unchanged. Confidence 90.
- **`additional_definition.xml`:** 3D options rewritten (`3D Mode`, `3D Power`), one speaker-test
  row moved onto `FEGlobals` colours.
- **`globalaudioconfig.xml`:** `user1` group volume `1.1` to `1.2` in two mixes (frontend speech,
  by the file's own comment), and more lines this page does not list.

**So the update changes no racing HUD and no menu this project draws today**; the audio change
is the one a player could hear. Comparing the file against what the mix reads is left open.
**Omega and 2048:** not checkable as updates of this title; both have their own patches
([patches.md](patches.md)).
