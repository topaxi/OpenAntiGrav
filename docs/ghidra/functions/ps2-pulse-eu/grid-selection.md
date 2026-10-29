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

## `GridController_UpdateSelectorPulse` (`0x001905d8`, 2026-10-29, `pulse-ps2-hex-align`)

**Binary:** `SCES_547.48`. The PSP body is `GridController_UpdateSelectorPulse` (`0x088a5700`),
[`../psp-pulse-usa/race-campaign.md`](../psp-pulse-usa/race-campaign.md). Found by position (the PSP
function sits 0x1eb0 after the `SetTileColor` helper listed above) and confirmed by its literals.
Headless decompile, in the PSP's shape:

- looks up two named children of the grid controller by string, `Selector` (`0x002ada98`) and
  `SelectorGlow` (`0x002adaa8`);
- on `Selector`, **eases its position toward `(hex.x + 0.5 - 5.0, hex.y + 0.5 - 7.0)`**: each call sets
  `pos = pos + (target - pos) * 0.5`, with `hex` the two floats at `+0x98`/`+0x9c` of the controller. The
  literals `5.0` and `7.0` are the PSP's;
- advances a phase at `+0xa0` by `2 * dt`, wrapping at `2.0`, and tints `Selector` with
  `lerp(phase, 0xff33a6b9, 0xffffffff)` for the first half and the reverse for the second
  (`FUN_001a8cb0`, all four vertex colours), the PSP's law and its colour literal;
- `SelectorGlow` is moved to the same place and tinted `0 -> white`; no authored `SelectorGlow` widget
  exists in the PS2 XML either (the lookup is dead, as on the PSP);
- polls the four d-pad masks and calls `FUN_00190e90` (move cursor) per direction.

**Confidence 85** for the identification (same literals, same two string lookups, same pulse law and
colour; `5.0`, `7.0`, `2.0` and `0xff33a6b9` all present). **Law is shared with the PSP; the drawn
position is not**, see "Where the PS2 draws it" below.

### Where the PS2 draws it (measured, GS dump)

A PCSX2 GS dump (`scripts/pcsx2-gsdump.py`, frame of a settled `Grid Selection` and of `Cell
Selection`; coordinates are the frame's `x - 2048 + 320`, `y - 2048 + 224`, i.e. the 640x448 grid) lists
the quads:

| Screen | Hex outline (32x32, texture `hex_outline`) | `Selector` quad (56x56, `pulse_assets` uv `(114,75)` 43x43) | Offset |
| --- | --- | --- | --- |
| Grid Selection, tile 0 | `(87, 247)` | `(76, 233)` | `(-11, -14)` |
| Cell Selection, selected tile | `(155, 143)` | `(144, 129)` | `(-11, -14)` |

The outlines match the XML exactly (`Outline_0_0` at `(47+40, 82+31+134) = (87, 247)`; the four tier
outlines step `(40, -31)`; Cell Selection's five at `(-245,-19)`, `(-205,-50)`, `(-165,-81)`,
`(-125,-50)`, `(-85,-19)` plus the centre `(-165,-19)`, all 32x32, 27x36 locks). The selector is the
XML's `width="56" height="56"` stretch of a 43x43 sprite, at the outline's top-left **minus (11, 14)**,
not the PSP's `(5, 7)`. The code's `5.0`/`7.0` are the same literals, so the extra `(-6, -7)` is the
PS2 art's own size difference (a 56 px quad over a 43 px canvas puts the quad `(56-43)/2 = 6.5` left
of where a 43 px quad would sit: the x half is explained; the y half, `-7` where `-6.5` is expected, is
not). **The value `(-11, -14)` is measured, two screens, quad edges read straight off the GS packets
(confidence 90); the split into `5/7` plus art stretch is a reading, confidence 45.**

Not seen on the PS2: a second quad for `SelectorGlow` (one `Selector` quad per frame, as on the PSP).
Pulse colour and period: **same law as the PSP** by the decompile (`2 * dt` phase, wrap `2.0`, cyan
`0xff33a6b9` to white); **not measured on PCSX2 pixels** (the loaded states keep their own phase). The
GS dump shows the quad's vertex colour at `(47,92,100)`/`(121,125,125)` on the two frames, between the two
ends as the law says.
