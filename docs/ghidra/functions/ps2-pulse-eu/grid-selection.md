# Grid Selection's tier tiles on PS2 Pulse (2026-10-17, `medal-tier-colour`)

**Binary:** `SCES_547.48` (`/pulse/SCES_547.48-ps2-pulse-eu`), image base `0x00100000`.
The reference reading is the PSP's,
[`../psp-pulse-usa/race-campaign.md`](../psp-pulse-usa/race-campaign.md), "The tier
unlock rule" and "The tier's medal colour"; this page is the PS2 corroboration.

## `GridSelection_PopulateTiles` (`0x001b6490`)

Found from the data side: the medal-colour table `0xfffaeb38`/`0xffdae3e4`/`0xffdf942f`
is stored twice in `.data` on this build (file offsets `0x1b44c8` and `0x1b58f8`, so
virtual `0x2b34c8` and `0x2b48f8`), where the PSP builds it from immediates. The only
references to `0x2b48f8` are `lui`/`addiu` at `0x001b66b8`/`0x001b66c0`, inside
`FUN_001b6490`. (`0x2b34c8` is referenced from `FUN_001afea0`, the expected
`CellSelection_PopulateGrid` counterpart; not read on this pass.)

Its headless decompile is the PSP `GridSelection_PopulateTiles` (`0x088de630`)
line for line:

- four tiles per page, each looked up by `FUN_001b6440(screen, column, page)`;
- base layer 1 visible and tinted `(fade * 0.5 * 255) << 24 | 0x34acc2`;
- a grid whose `Locked` word is set (`+0xb0` here, `+0xa0` on PSP) shows the lock
  layer 4 tinted white at `fade * 255`, cleared again when the previous tile's
  earned points reach its required points (wrapping to slot 3 of the previous page);
- then, with `earned`, `possible`, `required` read from the screen's `u16` arrays at
  `+0xf8`, `+0xfc`, `+0x100` (PSP: `+0xf4`, `+0xf8`, `+0xfc`):

```
if earned != 0:
    hide lock layer
    tier = 0 if earned == possible
         else 1 if earned >= (int)(possible - required) / 2 + required
         else 2 if earned >= required
         else none
    if tier: layer 2 colour = table[tier] & 0xffffff | (fade * 255) << 24, made visible
```

The same comparisons in the same order, the same three colours, the same layer
numbers and visibility bit (`4`). The only differences are the screen-struct offsets
(all shifted by four) and the colour table's storage.

**Confidence 88.** The decompile is unambiguous and matches a named PSP function body
for body, with the identifying literal (`0x34acc2`) and table in both. Not
runtime-verified on PCSX2.

## Callees, not named

Each matches a named PSP callee by argument shape and position, but none was read on
this pass, so they keep their `FUN_` names:

| PS2 | Likely PSP counterpart |
| --- | --- |
| `FUN_001b6440` | `GridSelection_FindTileAt` (`0x088de320`) |
| `FUN_0018fbc0` | `GridController_HideAllTiles` (`0x088a34c8`) |
| `FUN_0018fd20` | `GridController_SetTileFlags` (`0x088a3638`) |
| `FUN_0018fee0` | `GridController_ClearTileFlags` (`0x088a374c`) |
| `FUN_0018ff78` | `GridController_SetTileColor` (`0x088a3850`) |
| `FUN_0011f9d0` | the float-to-int conversion the PSP inlines |
