# Bitmap fonts (`.fnt`)

**Status: partial.** The metrics are resolved and validated across all five
fonts. The glyph atlas's **pixel layout is not**: read as documented it is
noise, and no transform tried so far recovers glyphs from it. [PSP texture
swizzling](psp-texture.md) answers only whether `.mip` pixel data is swizzled
*in the file*: it is not. That page leaves open whether the PSP swizzles at
upload time, which this project's renderer never needed to answer since it
never uploads to real VRAM. So `.mip`'s resolved question is not a lead for
`.fnt` here, but it does not rule out the two being the same underlying
upload-time transform either — that is still open, on both sides.

Not implemented. Documented first so that whoever resolves the swizzle can
implement both at once. Until then the front end draws with
[its own 5x7 glyphs](../../crates/game/src/font.rs).

## Where they are

Five in `FE.wad` on the PSP Pulse disc, all named by hashing candidates from the
language plugins' `<Font Src="...">` attributes:

| Entry | Line height | Atlas | Glyphs | Used as |
| --- | ---: | --- | ---: | --- |
| `Data\FE\Fonts\pulse_text.fnt` | 13 | 256x128 | 197 | `Default` |
| `Data\FE\Fonts\Pulse_14.fnt` | 17 | 256x256 | 205 | `Small`, `Title`, `InGame`, `Stats` |
| `Data\FE\Fonts\Pulse_20.fnt` | 22 | 256x256 | 205 | `Menu` |
| `Data\FE\Fonts\PulseHud.fnt` | 25 | 512x256 | 128 | `HUD` |
| `Data\FE\Fonts\small.fnt` | 10 | 256x128 | 128 | `HUDSmall` |

## Layout

```text
+0x00  u8       version, 1
+0x01  u8[3]    "FNT"
+0x04  u32      codepoint count
+0x08  u32      offset of the codepoint table
+0x0c  u32      offset of the glyph-record offset table
+0x10  u32      line height in pixels
+0x14  u32      unknown; 0 in three fonts, 4 in the other two
+0x18  u32      offset of the atlas
+0x1c  u8[20]   zero
```

Then, in order:

- **Codepoint table**, `count` little-endian `u16` values, ascending, starting at
  `0x20`. Three of the five end with a `0x0000` terminator.
- **Offset table**, `count` little-endian `u32` byte offsets, one per glyph
  record. Every observed stride is 18.
- **Glyph records**, 18 bytes each:

```text
+0x00  u16   codepoint
+0x02  u8    width in pixels
+0x03  u8    height in pixels
+0x04  u16   u0        left edge in the atlas
+0x06  u16   u1        right edge
+0x08  u16   v0        top edge
+0x0a  u16   v1        bottom edge
+0x0c  u8    advance
+0x0d  u8[5] unknown; 0xff except one byte on some glyphs
```

- **Atlas**, a 4bpp palette-indexed image in the [`.mip`](psp-texture.md) shape
  but with its own header:

```text
+0x00  u16   width
+0x02  u16   height
+0x04  u8    bits per pixel, 4
+0x05  u8[3] flags, `01 01 00` in all five
+0x08  u32   palette bytes, 64
+0x0c  u32   pixel bytes, width * height / 2
+0x10        palette, 16 RGBA8888 entries
             indices, two per byte
```

## What pins the metrics down

Four checks, all of which hold for all five fonts:

1. **Every record's codepoint equals its codepoint-table entry.** 197, 205, 205,
   128 and 128 records, all matching.
2. **`u1 - u0 == width` and `v1 - v0 == height`** for every glyph. The box and
   the size are stored separately and they agree.
3. **The atlas header's own arithmetic**: `16 + palette_bytes + pixel_bytes`
   accounts for the whole tail, with exactly 48 zero bytes of padding after it in
   every one of the five.
4. **Every glyph box lies inside the atlas**, tightly: maximum `u1` is 255 of 256
   for the 256-wide fonts and 508 of 512 for `PulseHud`. A wrong width would put
   glyphs outside.

The glyph count is `count` minus one when the codepoint table ends in `0x0000`,
which is also `(atlas_offset - offset_table_end) / 18` in all five files. Two
derivations, same answer.

Confidence **92** for the metrics.

## What is not resolved

**The atlas pixel layout.** Read literally, as 4bpp indices row-major, the atlas
is noise in horizontal bands. Tried and rejected:

- PSP swizzle with 16-byte by 8-row blocks, both traversal orders and both
  nibble orders. This produces glyph-*like* structure but not glyphs.
- Every block geometry from 4 to 128 bytes wide and 1 to 32 rows tall, scored
  two ways: what fraction of ink falls inside a declared glyph box, and vertical
  coherence of ink. The best score was 0.80 and 0.68 respectively, where a
  correct decode should be near 1.
- Row de-interleaving at every stride from 2 to 32.
- Column-major (transposed) reading, and a four-bitplane decomposition (each of
  the 4 index bits as its own `w*h/8`-byte plane). Both score below the plain
  linear read.
- A Morton (Z-order) curve inside each block, block sizes 4 to 32 in both
  dimensions. No better than the block-swizzle sweep above.
- Re-scoring the whole block-swizzle sweep with ink weighted by the palette's
  alpha channel instead of "index nonzero", on the theory that the boolean
  oracle was drowning in low-alpha antialiasing dust. The ranking barely moves
  (best 0.41 against a 0.39 linear baseline) and the best candidate still
  renders as noise, so the boolean oracle was not the problem.

None of this narrows the search; it rules out two more transform families
(bitplanes, Morton order) without finding the right one.

Two things make this worth a second look rather than a rewrite. The `<Font>`
elements carry `borderExtendPixels="3"` and text widgets carry `RealGlow`, so ink
outside the declared glyph box is expected and the first scoring metric is
therefore capped below 1 even for a correct decode. And `.mip` textures have the
**same** unresolved question, recorded in [psp-texture.md](psp-texture.md), so
this is one problem rather than two.

The `+0x14` field, 0 in three fonts and 4 in two, is the obvious candidate for a
layout flag. The two fonts with 4 are also the two whose codepoint table has no
terminator, so it may be a version marker instead.

## Not determined

- The atlas pixel layout, above.
- The five bytes at `+0x0d` of a glyph record. Mostly `0xff`; the space glyph has
  one `0x00`. Kerning is the guess.
- Whether the palette is meaningful. Index 0 is transparent and the top indices
  are white with varying alpha, which reads like an antialiasing ramp, but three
  mid entries are opaque colours that a font should not need.
- `+0x14`.

## Until the atlas is readable

`oag-game` draws with a 5x7 font of its own, and how it *fails* matters as much
as how it draws. An accented letter with no glyph used to be skipped, which cost
the letter and not just the accent: the disc's `Français` came out as `FRANAIS`.

The fold is now two steps, in order. Case first, over the whole of Latin-1 rather
than ASCII, because `to_ascii_uppercase` leaves `ç` untouched and the lookup then
misses. Then, for an accented letter with no glyph, the **base letter** stands in.
Losing a letter changes a word; losing an accent only misspells it.

The set carries the accented capitals these five languages need, with the base
letter compressed into six of the seven rows so the diacritic has one. They read
slightly squat, which is what a 5x7 cell costs, and it is one more reason to
finish decoding the real atlas.
