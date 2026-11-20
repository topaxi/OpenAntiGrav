# Pure's front-end button names: `backward` and `cancel` are the triangle

Reads which pad control Pure's PSP executable binds the front end's abstract `backward` and
`cancel` to. Read headless on 2026-11-20, `psp-pure-usa` `BOOT.BIN` (image base `0x08804000`):
the decompiler and the instruction stream of the name parser, and its reverse table. **Static
only** (nothing was read from a running Pure): the rendered footers of the Pure EU disc agree
(below). Names went into [`names.tsv`](names.tsv) in the same change.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08898af8` | `Input_ParseButtonName` | 90 |
| `0x08898eb4` | `Input_GetButtonName` | 85 |

## The answer

`FUN_08898af8` compares its string argument with `strcasecmp` against a chain of names and returns
the abstract button index of the first match: `-1` for the first (the none name), `0xe`/`0xf` for
`start`/`select`, `4` for `circle` (`0x08a4fcb0`), `5` for `cross` (`0x08a4fcb8`), `6` for
`triangle` (`0x08a4fcc0`), `7` for `square` (`0x08a4fccc`), `0`-`3` and `8`-`0xd` for the
directions and shoulders. It is Pulse's `Input_ParseButtonName` (`0x0894f2a0` PSP, `0x002025d8`
PS2) with the same index table, and called from the same kind of places (the animation values and
key parser, the redirect parser).

The two names this lane needed:

| String | Address | Returns | Which is |
| --- | --- | --- | --- |
| `forward` | `0x08a4fd0c` | `5` | cross |
| `backward` | `0x08a4fd14` | **`6`** | **triangle** |
| `activate` | `0x08a4fd20` | `5` | cross |
| `cancel` | `0x08a4fd2c` | **`6`** | **triangle** |

Pulse's PSP returns `4` (the circle) for `backward` and `cancel`; Pulse's PS2 returns `6`
(`../ps2-pulse-eu/input.md`, "The divergence"). **Pure's PSP is on the PS2 Pulse's side:** its Back
is the triangle. The index equals the index its own `triangle` string returns, so this does not
depend on which pad bit the index is built from. `FUN_08898eb4` is the same table the other way
round (an index to its name string), with the same two rows. Confidence 90: both returns read off
the instruction stream (`li v0,0x6` at `0x08898ce8` and `0x08898d10`).

## What the disc agrees on

- The footers on the Pure EU disc draw the triangle beside `BACK` and the circled cross beside
  `CONFIRM` (in the `Stats` face: `Ϊ` U+03AA and `έ` U+03AD, read off the strings the footer draws).
- The front-end XML names no face button for Back on either Pure disc: `scripts/fe-redirect-census.py
  --buttons` over `FE.wad`, `FEData.wad` and `Data.wad` (EU and USA, expanded) lists only
  `forward="cancel"` on `WOW Download` (with `Cancel` text `Press X to Cancel`), `forward="any"`
  on the demo's screen and `forward="backward"` on `Records`.
- The Pure EU executable is not in the Ghidra project; the EU disc's footers are the evidence for
  EU (the strings and the string table layout are the USA's shifted by a constant).

## The prompt table

Pure's `Stats` face holds four filled discs beside its outlined set: `ά` U+03AC, `έ` U+03AD, `Ϊ`
U+03AA and `Ϋ` U+03AB (the circled cross and triangle are what the footers draw; the circle is
confirmed by the footer drawn under MENU BACK `circle`; the square by the shape of the cell, 75),
and the outlined `ή ί ΰ` that are not read.
`oag_pure::prompts::PROMPTS` carries the four discs after the six of the menus' face, so the
footers get the device's glyph and the hints depict the control the glyph shows.

## Cross-title

**Pulse PSP** binds the same names to the circle (`../psp-pulse-usa/input.md`, 88); **Pulse PS2** to the
triangle. **HD, Omega, 2048**: not read here (`docs/ui/back-and-help.md`, "The resolver"). **Pure PS2**: no such pressing.
