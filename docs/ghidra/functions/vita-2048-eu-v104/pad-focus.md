# Pad input and focus on the touch item classes

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04,
`/2048/eboot-vita-2048-eu-v104.elf`), image base `0x81000000`. Read to answer:
does any `Touch*` item the front-end XML builds its screens from take a pad, or
the per-item focus flag, so that the original has a focus state to play?
**No function is renamed** (nothing here is needed by the wiring), so nothing
is added to `names.tsv`; the addresses are Ghidra's own `FUN_` names.

## The generic focus machinery is real and not reached by the touch items

| Address | What | Confidence |
| --- | --- | --- |
| `0x8122d1e0` | button query `(pad, action)`; 66 callers (option pages, music list, lobbies, photo mode, end-of-race) | 60 |
| `0x81068e0e` | the base widget's XML reader: the attribute `Focus` set to `true` or `1` ORs `0x100` into `item+0x30`; `force` ORs `0x500`; `Save` ORs `0x800` | 80 |
| `0x81067b92` | `(screen, step)`: walks the screen's child list (`screen+0x98`, next at `item+0x5c`), keeps the focus index at `screen+0xbc`, clears and sets `0x200` on the next item that carries `0x100` and is visible | 75 |
| `0x81067c92` | the screen base's enter: calls the stepper with 0 (initial focus). about 55 callers, one per code screen class | 70 |

So a screen class that wants a pad-driven focus marks items `Focus="true"`
and enters through `0x81067c92`. In the shipped XML that attribute is on
`Animation` widgets of `InGame_Definition.xml` and `Demo_Definition.xml`, and
on `TouchList` `teamskin_touch` (v1.04 also `PlayerSkinHD`, `PlayerSkinFury`)
of `Team_Definition.xml`. **On no `TouchButton`, no `TouchCampaign`, no
`TouchHomeButton`**: 458 `<TouchButton>` and every other touch widget were
read across the base package and both patch archives (data, data1, data2).

## None of the six touch item classes reads a pad or the focus flag

Every distinct function in the first 13 to 18 vtable slots of each class was
decompiled and searched for a call to `0x8122d1e0`, a `& 0x200` test and the
strings `Focus` / `HasFocus`:

| Class | Vtable | Update slot | Result |
| --- | --- | --- | --- |
| `TouchButton` | `0x8150e8ec` | `0x810ee2f8` | none (confidence 80) |
| `TouchHomeButton` | `0x8150eb20` | `0x810fb68c` | none (80) |
| `TouchTeamGrid` | `0x8150edc8` | `0x8110772e` | none (80) |
| `TouchSlider` | `0x8150ed5c` | `0x81104f9a` | none (80) |
| `Touchlist` | `0x8150eb8c` | `0x810fce98` | only the XML reader `0x810fc48c` names `Focus` (parse, not a test) (75) |
| `TouchCampaign` | `0x8150e958` | `0x810f2f04` | none (80) |

`TouchButton`'s update (`0x810ee2f8`) is touch only: it hit-tests the finger
against the rect (`FUN_8105b25a`, press state at `+0xfb`), fires on a release
inside it (`FUN_8105b54c`), plays `newGUI_select` and follows `redirect`. So
the original's touch screens answer touch only and draw **no pad focus**
(confidence 75; static reading, no pad was pressed on a live item in a
debugger). Live check: Vita3K v0.2.1, EU v1.04, d-pad, cross, circle and
triangle on `GameModeChoice`, `Home`, `options` and `2048extras`: the frames are
bit-identical before and after every press (0 differing pixels); on `team` the
cursor row is bit-identical; the campaign map shows no marker that tracks a
key. See `docs/formats/2048-frontend.md`, "Controller focus".

## What the original does author: the pressed colour

`TouchButton`'s constructor (`0x810ed470`) loads `Blue2048` (twice), `Orange2048`,
`White2048` and `Grey2048` into `+0xdc`, `+0xe0`, `+0xe4`, `+0xec`, `+0xe8`.
Its draw (slot 7, `0x810edc10`) fills the tile with `Blue2048` and, while
`+0xfb` (finger pressed inside), `+0xfd` (`toggle` latched) or `+0xfc` is set,
with `Orange2048` instead (confidence 65: the three flags are read, only
`+0xfb` and `+0xfd` are understood). The live frames agree: `Options` shows its
selected button, `CAMERA`, in a solid `Orange2048` tile with the other three
`Blue2048`, and `team` fills its cursor cell `Orange2048`
(`team-grid.md`). That is the original's one "selected" look, and the pad focus
plays it (`docs/formats/2048-frontend.md`).

## Omega

**Checked, applies, not wired.** Omega's `eboot.bin`
(`/omega/eboot-ps4-omega-eu.bin`) carries `TouchButton_Item.cpp`
(`0x01820635`), `Orange2048` (`0x01822db4`), `Blue2048` and the same
`HasFocus` / `NotHasFocus` / `FocusMenu_Item.cpp` strings. Its front end is HD's
`<HorizMenu>` (`omega-status.md`), so it builds no 2048 touch grid and the
class's draw law was not re-read on PS4 code.
