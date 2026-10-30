# 2048 has no per-weapon pickup colour in the race HUD

Static read of `eboot.elf` (2048 EU, patch v1.04, base `0x81000000`) on 2026-10-30, lane
`pickup-fade-titles`. Not run live (no Vita3K capture; the brief allowed static).

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x811505a8` | `Hud_SetPickupWidgetsVisible` | 75 |

## Findings

1. **`Hud_UpdatePickupIcon` (`0x81194f9c`, [pickup-icon-uv-table.md](pickup-icon-uv-table.md)) writes
   fixed colour words only** through `FUN_8109cfae(widget, word, flag)` (stores the word at `+0xb8`,
   `+0xbc`, `+0xc0`, `+0xc4`, the four corners): `PickupBgFrame` (`hud+0xa8`) gets `0xFFA7A5A7` (a grey)
   while the icon flies to its slot and then blinks to `0xFFFFFFFF` at 8 Hz (`(timer * 8) & 1`) for a
   second; the slot widget `hud+0xf0` keeps its colour and takes alpha `0xFF` while a weapon is held
   (blinking `0x64` at 8 Hz right after the grant) and `0x64` with nothing held; `DenyPickup`
   (`hud+0xb0`) blinks `0xFFFFFFFF`/`0x64FFFFFF`. No table indexed by the weapon, no `0x80BE1E69`.
   Confidence **65** (decompile; the Vita FPU/NEON helpers render as `VectorBitwiseSelect` noise, so a
   write hidden in them is not ruled out).
2. **The HD-lineage `PickupBackground` widget exists but is only shown and hidden.** The binder
   `FUN_81158d88` binds `PickupBackground` to `hud+0x5b8` (string at `0x8147ac44`, xref `0x81159048`);
   the only reader of `+0x5b8` is `Hud_SetPickupWidgetsVisible`, which sets or clears bit `0x4` of the
   six widgets' flags at `hud+0x5a4..0x5b8`. Confidence 75 for the reader being unique (an `ldr` search
   over the program for offset `0x5b8` found seven hits, six of them `sp`-relative locals and one this
   function).
3. **The three colours are in 2048, in the front end.** `FUN_811456f8` (`0x811456f8`, the 13-row
   weapon-toggle strip: each row's `FE_ON` state, a UV table at `0x8141921c`, grey `0x80808080` when
   off) colours an enabled row `0x801CBC1C` (rows 3, 4, 6), `0x802B1CBC` (rows 8, 9) and `0x80BE1E69`
   otherwise, Pulse's three words in that menu's own row order. Hypothesis (60), not renamed: the Race
   Box / Weapons screen's weapon toggles.

## Cross-check

- **Omega**: **checked, differs**: Omega's in-race HUD carries Pulse's per-weapon table
  ([omega-pickup-table.md](../ps4-omega-eu/omega-pickup-table.md)); 2048's does not (items 1 and 2).
  Omega is HD's HUD, 2048 is its own.
- **Pulse** ([pickup-colour.md](../psp-pulse-usa/pickup-colour.md)): the swatch words are the same
  authored palette; the in-race law is not shared.

Consequence for wiring: 2048 should **not** inherit Pulse's backdrop table for `PickupBgFrame`. The
words it does author are `0xFFA7A5A7` and the 8 Hz white blink; the layout's own colour applies
otherwise.
