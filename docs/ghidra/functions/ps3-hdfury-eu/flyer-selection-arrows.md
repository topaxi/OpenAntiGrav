# `FlyerSelection`'s paging arrows: the colour its update gives them

2026-11-06. `Grid Selection` and `Grid Selection Fury` are `type="FlyerSelection"` screens
(`FlyerSelection_Screen.cpp`, string `0x00794fd8`). Their two `Flyer Left/Right Arrow` images author
`Color="0xFF8AC0CA"` and draw `100,100,100` in a frame. This page is why: the screen's per-frame update
overwrites the colour. Nothing here is renamed (no Ghidra write); the two functions stay `FUN_`.

TOC for both functions: `0x008ad4d8`, `scripts/ps3-toc.py toc` reads `exact` for each, so the decompiler's
`DAT_`/`PTR_` constants are the ones the code loads.

| Address | What | Confidence |
| --- | --- | --- |
| `0x00230180` | The screen's init: resolves the widgets by name and caches them in the object | 85 |
| `0x002308a0` | The per-frame update (vtable slot at `0x0087f460`) | 90 |

## The init caches the widgets

`0x00230180` calls the name lookup (`FUN_0016a920(screen, name, 1)`) once per widget and stores the result:

| Object offset | TOC slot | String | Widget |
| --- | --- | --- | --- |
| `+0x108` | `0x3518` (`0x008b09f0`) | `0x00795040` | `Flyer Left Arrow` |
| `+0x10c` | `0x351c` | `0x00795058` | `Flyer Right Arrow` |
| `+0x110` | `0x3520` | `0x00795070` | `Flyer Pad Lock` |

The slot-to-string step is a read of the TOC (`0x008b09f0` holds `0x00795040`); the right and lock strings
sit at the next two slots and follow the same order as the three calls. The `flyerlist` widget is TOC slot
`0x356c` (`0x008b0a44` holds `0x007950e8`, `flyerlist`).

## The update sets the arrows' colour every frame

`0x002308a0` ends with this branch, on the index `i` of the card the `flyerlist` selects among all the
flyers (`param_2 + 0xf0`), and `n` the list's own entry count (`FUN_0015a238`):

```
if i == 0 or i == 8:                       left  = 0x7F646464    right = 0xFF646464
else:                                      left  = 0xFF646464
     if n - 1 == i or n + 7 == i:                right = 0x7F646464
     else:                                       right = 0xFF646464
```

(`Image_SetVertexColours(widget, 0xAARRGGBB, 0)`.) So:

- The RGB is the literal `0x646464`, never the authored `0x8AC0CA`: the authored colour is dead data.
- Alpha is `0x7F` on an arrow with nowhere to page to and `0xFF` otherwise. `i == 0 || i == 8` is the first card
  of the base campaign (`grid0`) or of Fury (`grid8`); `n - 1` and `n + 7` are their last cards (`n` is 8 on both
  screens). The `0 || 8` test comes first, so a page whose only card is its first leaves the right arrow opaque.
- The text colour `0xFF646464` is `FEGlobals->HD_Grey` as the same screen's `EventNum` authors it.
- The unlock panel (`unlockbox`, `unlockboxbrackets`, TOC slots `0x3594`/`0x3598`) is what the tail of the update
  hides; the arrows are never hidden, only dimmed.

**Evidence from the frame**, `data/reference/compare-page/hd/menu-grid-select/theirs.png` (grid 1, `Event 01/08`):
the right arrow's pixels read `100,100,100`; the left arrow, on the first card, adds `50` to each channel over
the dark backdrop (`0.5 * 100`), a faint grey shape at the card's left edge. Ours at the same frame: the right
arrow's box differs from the original's by 0.01 grey, and the left adds `50` as well. Confidence 90.

**What is not read.** The first branch of the update (a page flip in progress, `param_2 + 0x118 != 0`) also writes
the arrows' colour with the alpha cleared, and the same call overwrites it later in the frame; no frame caught a
flip, so a page flip draws the settled colour. The original dims at the ends of a page range and the engine's
`GridSelection::step` wraps; whether the original's pad wraps is not read.

## Omega: ported

Omega's `FlyerSelection_Screen.cpp` is `FUN_01546a50` (vtable `0x01922fe8` slot 3), its init `FUN_015480a0`
(arrows cached at `+0x1a0`, `+0x1a8`). The same branch, writing the four vertex colours of each arrow
(`+0x118..+0x127`): first card `i == 0x10 || (i | 8) == 8` (0, 8, 16) gives left `0x7F646464`, right `0xFF646464`;
otherwise left `0xFF646464` and right `0x7F646464` at `n - 1`, `n + 7` or `n + 15` (the third range is the 2048
grids, `Cup_Name_2048..2050`). Same literals, same rule, one more range. Confidence 85 (no PS4 emulator memory
readout; the decompile only).

On Omega's frame (`omega/menu-grid-select`) the left arrow over the white backdrop reads `190` where `0x7F`
alpha blended in sRGB reads `178` (ours) and blended in linear light reads `189`: **open**, one frame, a
different renderer's blend; the HD frame (dark backdrop) cannot tell the two apart.
