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
`Ease_CosineWithBump` is built from.

## The fade constants (read 2026-10-30, `pickup-fade-wire`)

Static, from the decompile of `0x0008df30` and the floats it names (bridge `read_memory`):

| Float | Value | Use | Pulse |
| --- | --- | --- | --- |
| `0x008a776c` | `0x3fc00000` = **1.5** | `step = dt * 1.5`, the first line of the update | **1.2** |
| `0x008a764c` | `0.5` | the cap `T` stops at while held | same |
| `0x008a7768` | `3.0` | `T * 3` into the cosine step | same |
| `0x008a76bc` | `pi` | the cosine step's argument scale | same |
| `0x008a780c` | `-25.0` | the bump's `-25 (u - 0.5)^2`, next to the table's TOC slot | same |

So **HD's rate is 1.5, not Pulse's 1.2**, and the shape is the same (cap `0.5`, `x3`, cosine step,
`exp(-25 ...)` bump). Confidence **80** (static; not run live, RPCS3 has no memory trace of it here).
**Not read, inherited from Pulse and labelled so in code**: the hold threshold (`0.1` sits at
`0x008a756c`/`0x008a76c8`, plausibly it), the icon's `T >= 0.4` gate, the fade-out branch (the
decompile's second half, `iVar17 + 0xc` reloaded through the same easing, reads like it), and whether HD
folds the backdrop's size like Pulse's `+0x9c`/`+0xa0` (HD's art draws no size pulse). The 2048 update
uses the same `1.5`.

## Consequence

`docs/formats/hd-hud.md`'s line that HD fixes `0x5AFFFFFF` over `PickupBackground` is the idle path.
With a weapon held HD's hexagon takes the table word, scaled by the eased timer: **ported, wired
2026-10-30** (`oag_hud::runtime::colour` for the always-on `PickupBackground`, the icon through
`pickup_sprites`; the rate is `oag_hd::hud::PICKUP_FADE_RATE_PERCENT`). Headless HD frames of Missile
(pink), Turbo (green) and Mine (blue) at `--give`, ticks 8 and 10 (the overshoot) and 60 (settled), read
2026-10-30. Omega carries the same table
([omega-pickup-table.md](../ps4-omega-eu/omega-pickup-table.md)).
