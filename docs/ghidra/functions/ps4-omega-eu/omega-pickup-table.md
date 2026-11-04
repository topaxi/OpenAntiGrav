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
`0x012647e1`, `0x01264cf1`). I did not rename either function; the rate is read below.

## The fade (read 2026-11-04, `pickup-backdrop-closeout`)

Static, from the decompile of `FUN_0125e9b0` in the base `eboot.bin` program
(`/omega/eboot-ps4-omega-eu.bin`); no live value, because shadPS4 has no memory readout. The table's
row reader sits in the pickup tail (`param_2 + 0x18c` selects the HUD's table copy):

- `fVar82 = param_1 * 1.5`, the step the timers gain per update (`param_1` is the update's `dt`; it is
  the only float argument), **rate `1.5`**, the same as HD's `Hud_UpdatePickupBar` and 2048's
  `FUN_81154172`, where Pulse has `1.2`. Confidence **80** (static; `param_1` being `dt` is read off its
  other uses, not traced to the caller).
- The held timer is `vminss(T + step, 0.5)` (`0x3f000000`), the cap HD and Pulse share.
- The backdrop word is read at `row * 0x20 + hud * 0x1c0 + 0x193f764`, the table's backdrop word
  (`0x193f750 + 0x14`), and scaled through a sine step (`_FSin(x * 9.424778)`, `3 pi x`) plus an
  `expf((x * -150 + 37.5) * (x * 6 - 1.5))` bump, the shape HD builds from `FUN_00676fe8` and
  `FUN_00677528`. The nothing-held branch writes `0x5affffff5affffff` over the backdrop's four corners,
  HD's idle word.
- The per-row timers and the hold are unread here; the patch 1.07 program's copy of the function
  (addresses move, [patch-v107-addresses.md](patch-v107-addresses.md)) was not read.

## Verdict

Omega: **ported, wired 2026-10-30** (static, confidence 90 for the table, unread for the fade). It
inherits HD's behaviour, which carries Pulse's table: the held backdrop takes the table word through
the fade at rate `1.5`, **read in `FUN_0125e9b0`** (static, confidence 80, above; no Omega frame was
taken, shadPS4 race shots being untrusted, and its hold and fade-out stay inherited from HD). The 2048
half of the cross-check: [pickup-backdrop.md](../vita-2048-eu-v104/pickup-backdrop.md) (**the same
table and update, a `1.5` rate; ported**, and 2048 itself draws no coloured backdrop, which Omega's
layout has not been checked for).
