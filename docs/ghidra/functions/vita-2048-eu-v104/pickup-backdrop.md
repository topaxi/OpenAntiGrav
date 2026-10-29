# 2048 carries Pulse's pickup table and fade; what draws it is open

Static read of `eboot.elf` (2048 EU, patch v1.04, base `0x81000000`) on 2026-10-30, lane
`pickup-fade-titles`; byte search of the base and patch ELFs for the three words. Not run live (no
Vita3K capture; the brief allowed static). **A first draft of this page said 2048 has no per-weapon
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

## What draws it is open

2048's layout `HUD_pickups.xml` authors `PickupBackground` with no `<Values>`
(`crates/2048/src/hud.rs`), the held pickup is one `PickupIcon` image with a rewritten UV rect, and the
visible arc is `PickupBgFrame`. `Hud_UpdatePickupIcon` ([pickup-icon-uv-table.md](pickup-icon-uv-table.md))
writes only fixed colour words through `FUN_8109cfae(widget, word, flag)` (the four corners at `+0xb8`,
`+0xbc`, `+0xc0`, `+0xc4`): `PickupBgFrame` `hud+0xa8` takes `0xFFA7A5A7` while the icon flies and an
8 Hz blink to `0xFFFFFFFF` for a second afterwards; the slot widget `hud+0xf0` alpha-blinks
`0x64`/`0xFF`; `DenyPickup` `hud+0xb0` blinks white. `Hud_SetPickupWidgetsVisible` is the show/hide of the
six HD-style widgets at `hud+0x5a4..0x5b8` (`PickupBackground` is `+0x5b8`, the only `ldr` reader of
`+0x5b8` besides stack locals, both `ldr` and `ldr.w` searched). So 2048 has the table and the update;
whether `PickupBackground` is visibly drawn in the 2048 race HUD, as opposed to the layout's own
`PickupBgFrame` arc, is **unverified** (a Vita3K frame with a weapon held would settle it).
Front end: `FUN_811456f8` (`0x811456f8`, the 13-row weapon-toggle strip) uses the same three words in its
own row order (3, 4, 6 green; 8, 9 blue; pink otherwise; grey `0x80808080` when off), hypothesis 60, not
renamed.

## Cross-check

- **Omega**: **ported by data**: the same table and an HD-lineage update on both; Omega's HUD update
  `FUN_0125e9b0` was not read for its rate. 2048 and Omega agree on the table, 2048's rate differs from
  Pulse's.
- **Pulse**: **checked, applies, not wired**; rate `1.5` to confirm against Pulse's `1.2`.
