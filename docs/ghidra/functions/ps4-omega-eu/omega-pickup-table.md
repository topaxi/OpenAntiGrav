# Omega's pickup widget table is HD's, and Pulse's

Static read of `eboot.bin` (Omega EU) on 2026-10-30, lane `pickup-fade-titles`. Not run live
(shadPS4 has no memory readout).

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0193f750` | `g_hud_pickup_widget_table` (data, 14 rows of `0x20` bytes) | 90 |

## The table

14 rows, each: an 8-byte name pointer (row 0 `0x01811056` = the string `PickupBackground`, rows 1-13 the
weapon icon names), an 8-byte widget slot (zero until bound), then four 32-bit words: the icon word,
the backdrop word, `1.0f`, `1.0f`. Read at `0x0193f750` for `0x1c0` bytes: row 0 both words
`0xFFFFFFFF`; the backdrop words of rows 1-13 are `0x80BE1E69` (rows 1, 2, 3, 7, 10, 11, 12, 13),
`0x801CBC1C` (4, 5, 6) and `0x802B1CBC` (8, 9), the same thirteen words as Pulse's table
([pickup-colour.md](../psp-pulse-usa/pickup-colour.md)) and HD's
([hd-pickup-table.md](../ps3-hdfury-eu/hd-pickup-table.md)).

The binder `FUN_0126c020` stores the table's widget slots (xrefs to `0x0193f750` and `0x0193f758` at
`0x0126e892`, `0x0126e8ac`, `0x0126e8ea`, `0x0126e948`), and the HUD update `FUN_0125e9b0` (a 7,249-line
decompile covering the whole in-race HUD) reads the table's rows (xrefs at `0x01264113`..`0x012650bc`,
`0x012647e1`, `0x01264cf1`). The fade law in it (easing, rate, hold) was **not** re-derived from that
decompile; I did not rename either function.

## Verdict

Omega: **ported, wired 2026-10-30** (static, confidence 90 for the table, unread for the fade). It
inherits HD's behaviour, which carries Pulse's table: the held backdrop takes the table word through
the fade at HD's rate `1.5`, **inherited from HD** (`FUN_0125e9b0` was not read for it; no Omega frame
was taken, shadPS4 race shots being untrusted). The 2048 half of the cross-check:
[pickup-backdrop.md](../vita-2048-eu-v104/pickup-backdrop.md) (**the same table and update, a `1.5`
rate; ported by data**).
