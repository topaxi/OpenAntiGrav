# HD's pickup widget table is Pulse's

Static read of `EBOOT.elf` (HD/Fury EU, `BCES-00664`) on 2026-10-30, lane `pickup-fade-titles`. The
caller's brief said HD "fixes `0x5AFFFFFF`"; that is `Hud_UpdatePickupBar`'s nothing-held value only
([hud-readouts.md](hud-readouts.md), "The pickup bar's idle dim"), and the held path takes a per-weapon
table. This page reads the table.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x00920598` | `g_hud_pickup_widget_table` (data, what the TOC slot `0x008a7800` points at) | 90 |

## The table

`PTR_008a7800 + hud * 0x150` (`hud` 0 and 1 read; both identical) is 14 rows of 6 big-endian words,
the row shape of Pulse's `g_hud_pickup_widget_table`
([pickup-colour.md](../psp-pulse-usa/pickup-colour.md)): name pointer, widget slot (zero until bound),
the icon word, the backdrop word, two `1.0f`. Row 0 (name at `0x0077f598`) has both words
`0xFFFFFFFF`; rows 1-13 have the icon word `0xFFFFFFFF` and the backdrop word:

| Rows | Word |
| --- | --- |
| 1, 2, 3, 7, 10, 11, 12, 13 | `0x80BE1E69` |
| 4, 5, 6 | `0x801CBC1C` |
| 8, 9 | `0x802B1CBC` |

Byte for byte the thirteen words of Pulse's table, alpha `0x80` included. Read with the bridge's
`read_memory` at `0x00920598` for `0x150 * 2` bytes (16 bytes per call).

`Hud_UpdatePickupBar`'s tail already walks `PTR_008a7800 + hud*0x150 + row*0x18` ("an ARGB pair at
`+8`/`+0xc`") and scales the row colour by an eased per-row timer; the two easing helpers are
`0x00676fe8` (a wrapper over `_FSin`) and `0x00677528` (a wrapper over `expf`), the same pair Pulse's
`Ease_CosineWithBump` is built from (a cosine step plus an `exp(-25 (u - 0.5)^2)` bump). The
constants and the `1.2` rate were not read here: **static, 70** that HD's fade is Pulse's curve.

## Consequence

`docs/formats/hd-hud.md`'s line that HD fixes `0x5AFFFFFF` over `PickupBackground` is the idle path.
With a weapon held HD's hexagon takes the table word, scaled by the eased timer: **checked, applies,
not wired** (HD draws the layout colour). Omega carries the same table
([omega-pickup-table.md](../ps4-omega-eu/omega-pickup-table.md)).
