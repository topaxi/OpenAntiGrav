# `Flyer_Item`: the card's clock

Binary `EBOOT.BIN` of `hdfury-ps3-eu-dec.iso` (Ghidra `/hdfury/EBOOT-ps3-hdfury-eu.elf`, TOC `r2 = 0x008ad4d8`). Read on the GUI bridge
(decompile of both functions), then live on RPCS3 (Grid Selection Fury, `09_blitzed`). The pair that uses it:
`hd/menu-grid-select`, [`docs/ui/campaign-screens.md`](../../../ui/campaign-screens.md), "The card's clock".

## The names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x001a2060` | `FlyerItem_Construct` | 80 |
| `0x001a2510` | `FlyerItem_Render` | 80 |

## `0x001a2060`, `FlyerItem_Construct(Flyer_Item *self)`

Calls the `Model_Item` constructor (`0x001d3150`), stores the class's three vtable words (`TOC[-0x172]`, `TOC[-0x16f]`, `TOC[-0x16e]`;
the one at `TOC - 0x5bc` is the vtable the live read scans for, `0x00865cb8`), zeroes the front and back model slots (`+0x224`,
`+0x228`), and sets the loop: `+0x234 = 6.0` (`0x40c00000`), `+0x238 = 3.0` (`0x40400000`), `+0x23c = 0` (which side is drawn).

## `0x001a2510`, `FlyerItem_Render(Flyer_Item *self, ...)`

With both models present: pushes the model's own camera (`+0x22c`, or `+0x230` for the back when `+0x23c` is set), reads the model's time with
`AnimNode_GetTime` (`0x002c0d78`, `+0xc0` of the first `MeshImporter` in the tree), and when that is **at least `+0x234`** calls
`AnimNode_UpdateTransformTree(time - 6.0 + 3.0)`, so the card's clock lives in `[3, 6)` and goes round every 3 s. Then it draws.

## Live (RPCS3, 2026-11-19)

40 reads 0.15 s apart of the first `MeshImporter` under the instance's front model: 4.34, 4.51, ... 5.86, then 3.03, 3.18 ... 5.89, then 3.04: it
rises 1.0 a second of wall time and goes from 6.01 to 3.03 at the wrap. All 50 `MeshImporter` nodes of the tree read one value. A node samples its
keys at the clock modulo its own `LoopEnd` (4.0 on Fury); the stored value is not wrapped by it. The first `MeshImporter` is the node `AnimNode_GetTime`
returns; `scripts/hd_flyer_clock.py` finds the instance by the vtable word and a non-null `+0x224`.

**Omega: checked, applies, not wired, live clock not checkable.** Omega's cards are the same flyers through the PS4 reader (`campaign-screens.md`,
"Omega: Fury's cards draw"); `--menu-model-seconds` reaches its `grid-select` the same way, the executable's render was not decompiled for the
`[3, 6)` wrap, and there is no memory readout of a live clock. Omega's `menu-grid-select` pair is not re-shot with it.
