# 2048 carries Pulse's pickup table and fade, and does not draw the coloured backdrop

Static read of `eboot.elf` (2048 EU, patch v1.04, base `0x81000000`) on 2026-10-30, lane
`pickup-fade-titles`; byte search of the base and patch ELFs for the three words. What draws it was
settled live on 2026-11-04 (below). **A first draft of this page said 2048 has no per-weapon
table. That was wrong**: it searched instruction immediates for the words, which cannot find a data
table, and read only `Hud_UpdatePickupIcon`.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x8151f104` | `g_hud_pickup_widget_table` (data, two copies, `hud` 0 at `0x8151f104`, `hud` 1 at `0x8151f254`) | 90 |
| `0x81154172` | `Hud_UpdatePickupBar` (the HD-lineage pickup update, HD's `Hud_UpdatePickupBar` twin) | 70 |
| `0x811505a8` | `Hud_SetPickupWidgetsVisible` | 75 |

## The table

A search of `eboot.elf` for the little-endian words `69 1E BE 80`, `1C BC 1C 80` and `BC 1C 2B 80`
hits 16, 6 and 4 times, in both the base and the v1.04 patch (so v1.00 already has it): two tables
of 14 rows of `0x18` bytes, file offset `0x520104` and `0x520254` (`0x8151f104`, `0x8151f254`; `LOAD`
segment `0x81519000` from file `0x51a000`). The rows are Pulse's: name pointer (row 0 `0x8147ac44` =
`PickupBackground`, then the weapon icon names), widget slot, the icon word `0xFFFFFFFF`, the backdrop
word, two `1.0f`. The backdrop words, rows 1-13: `0x80BE1E69` (1, 2, 3, 7, 10, 11, 12, 13),
`0x801CBC1C` (4, 5, 6), `0x802B1CBC` (8, 9), the same thirteen as Pulse
([pickup-colour.md](../psp-pulse-usa/pickup-colour.md)), HD
([hd-pickup-table.md](../ps3-hdfury-eu/hd-pickup-table.md)) and Omega
([omega-pickup-table.md](../ps4-omega-eu/omega-pickup-table.md)). `PickupBackground`'s string has only a
code reference (the binder `FUN_81158d88`, `0x81159048`), the table rows hold the pointer.

## The update

`FUN_81154172` (called from the dirty-flag dispatcher `FUN_8115c2d8`, the same one that calls
`Hud_UpdatePickupIcon`) reads the table (`DAT_8151f110 + hud * 0x150 + row * 0x18`, the backdrop word) and
has Pulse's fade: `T` capped at `0.5` and advanced by `dt-ish * 1.5` (`local_80 * 1.5`; Pulse's rate is
`dt * 1.2`; `local_80` is not confirmed to be `dt`), the scale `T * 2 * 1.5` through a cosine step
`(1 - cos x) * 0.5` plus `exp(-25 * ((x - 0.5) * 2 - 0.5)^2)` (Pulse's `Ease_CosineWithBump`), the idle
write `0x5AFFFFFF` over the backdrop (HD's idle word), `0xFFFFFFFF` while the held-pickup state at
`*(*(hud+0x3c)+0x58)` is non-zero and a flag is set, and the same `0.1` and `(2 - 2t) * 1.5` after
use. Static, confidence **70** that it is Pulse's law with a **different rate (1.5, not 1.2)**; the
bump's centre/width constants are as Pulse's in the decompile but were not fitted.

## What draws it: not the coloured backdrop (live, 2026-11-04, `pickup-backdrop-closeout`)

**Falsifier written before the capture**: if `PickupBackground` drew, the held weapon's table word
(alpha `0x80`; pink `0x80BE1E69`, green `0x801CBC1C` or blue `0x802B1CBC`) would tint the slot behind
the icon, and none of the three is the yellow of the icon disc. If it does not draw, the slot shows only
the icon over `PickupBgFrame`'s fixed `0xFFA7A5A7` arc.

- **Authored layout** (`data/xml/2048_hud/HUD_pickups.xml` in `data2.psarc`, 5,416 bytes):
  `<Image name="PickupBackground" transition="0"></Image>`, no `<Values>`: no texture, no size, no
  colour. The runtime write to its colour has no image to colour.
- **Frame, boot 1** (Vita3K 0.2.1, EU v1.04, Queens Mall, Feisar; the
  frames `36-w-5`, `36-w-15`, `35-w-2` of `vita3k-capture.md`'s earlier pass): a held Missile is the
  yellow disc of `hud_2048.gxt`'s icon row with the grey arc around it. Read at 1:1: disc pixels
  `(168,140,12)`, `(186,150,20)`, arc grey, nothing pink, green or blue.
- **Frame, boot 2** (a private profile and a headless weston and Xwayland on `:93`, the walk of
  `scripts/vita-2048-pair.py --walk` and R1 held through a lap): a Rocket is granted (icon at the top
  centre with `ROCKETS`, then in the slot) and stays held for the next 90 s; the slot shows the same
  disc `(176,144,7)`, `(164,133,8)` inside the same grey arc. No tint in 30 frames.
- **Ours**, `oag-game <PCSF00007> --race --event "2048 - Event 2" --give rocket` at the same
  settings (960x544, Feisar, Queens Mall): the same yellow disc with the rocket glyph inside the grey
  arc, no tint, side by side with the Vita3K frame at the same place on screen.
- **Verdict**: **`PickupBackground` is never drawn with a colour in 2048's race HUD; only
  `PickupBgFrame` and the icon are** (confidence 85: two boots, a Missile and a Rocket, one layout; a
  weapon of the green or blue rows was not held). `FUN_81154172` still steps the table's words (the code is
  there, HD's lineage), but the widget it writes has no image. `oag_2048::hud::ART` keeps
  `pickup_colours: None` and `pickup_fade: None`, now for this reason, and 2048 does not take
  `oag_hud::pickup_fade`.

`PickupBgFrame` (`hud+0xa8`) takes `0xFFA7A5A7` while the icon flies and an 8 Hz blink to
`0xFFFFFFFF` for a second afterwards, the slot widget `hud+0xf0` alpha-blinks `0x64`/`0xFF`, and
`DenyPickup` `hud+0xb0` blinks white (`Hud_UpdatePickupIcon`,
[pickup-icon-uv-table.md](pickup-icon-uv-table.md)). `Hud_SetPickupWidgetsVisible` shows and hides the
six HD-style widgets at `hud+0x5a4..0x5b8` (`PickupBackground` is `+0x5b8`).
Front end: `FUN_811456f8` (`0x811456f8`, the 13-row weapon-toggle strip) uses the same three words in its
own row order (3, 4, 6 green; 8, 9 blue; pink otherwise; grey `0x80808080` when off), hypothesis 60, not
renamed.

## Cross-check

- **Omega**: **ported by data, rate read**: the same table and an HD-lineage update on both; Omega's
  `FUN_0125e9b0` steps its timers by `param_1 * 1.5` too (static, confidence 80,
  [omega-pickup-table.md](../ps4-omega-eu/omega-pickup-table.md), "The fade"). The one thing that does
  not carry over is the drawing: 2048 draws no coloured backdrop, and Omega's own layout
  (`Zone_HUD.xml`, HD's dialect) was not checked for an authored `PickupBackground` image: unread.
- **Pulse**: **checked, differs** in the rate (`1.5` against Pulse's `1.2`; HD's is `1.5` as well,
  [hd-pickup-table.md](../ps3-hdfury-eu/hd-pickup-table.md)).
- **HD**: the same table; HD draws the backdrop (an authored image), 2048 does not.
