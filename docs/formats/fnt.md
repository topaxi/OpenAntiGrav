# Bitmap fonts (`.fnt`)

**Status: understood.** Metrics and the glyph atlas are both decoded,
implemented in [`oag-formats::fnt`](../../crates/formats/src/fnt.rs) and
validated across all five fonts: 863 glyphs, and every atlas renders as a
recognisable character set - digits, upper and lower case, the accented Latin-1
capitals the five shipped languages need, and the PSP button glyphs.

The atlas resisted several passes of blind structural search. It was not a
transform that was missing; it was **where the pixels start**.

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

- **Atlas**, below.

## The atlas is a `.vex` Texture node, and its header is 64 bytes

This is the correction that unlocked everything. The atlas is **not** shaped
like a [standalone `.mip`](psp-texture.md). It is the same `Texture` node
payload a [`.vex`](vex.md#embedded-textures) model embeds, and the pixel data
does not begin until `+0x80`:

```text
+0x00  u16    width
+0x02  u16    height
+0x04  u8     bits_per_pixel, 4
+0x05  u8     mip_count, 1
+0x06  u8     flags; bit 0 means the texels are already swizzled
+0x07  u8     texture index
+0x08  u32    clut_size, 64
+0x0c  u32    texel_size, width * height / 2
+0x10  ptr    texels, zero at rest, patched at load
+0x14  ptr    clut, zero at rest, patched at load
+0x18  u8[40] zero
+0x40  clut, 16 RGBA8888 entries
+0x80  texels, width * height / 2 bytes
```

Every previous pass read the palette at `+0x10` and the pixels at `+0x50`, which
is what a `.mip` header would imply. Both are **48 bytes early**. The palette
comes out with 12 of its 16 entries fully transparent, and the pixels come out
shifted by 96, which is a displacement no block-geometry sweep can undo - which
is why sweeping every geometry from 4 to 128 bytes wide found "glyph-*like*
structure but not glyphs" and stalled there for several passes.

## The texels are stored already swizzled

Most textures on the disc are stored **linearly** and swizzled by the game at
load time. The `.fnt` atlas is not, and the file itself says so - as do 6 of the
13 standalone [`.mip`](psp-texture.md) textures in the same archive, which is a
bug this work turned up rather than a font-specific quirk.

`Texture_SwizzleForGe` at `0x08926da8` converts a linear image into the GE's
layout in place: 16-byte by 8-row blocks, emitted block-row major, then block
column, then the 8 rows inside a block. `Texture_BindEmbeddedData`
(`0x08927f28`) calls it **only when bit 0 of the node's `flags` byte at `+0x06`
is clear**, and sets that bit afterwards so it never runs twice. Every font
atlas ships with the bit already set, so the game skips the conversion and the
data on disc is swizzled. A reader has to undo it.

Two nibbles per byte, **low nibble first**, the same order as `.mip`.

## Evidence

### The block closes exactly, which is what pins the header at 64

```text
0x40 + clut_size + texel_size == atlas block length
```

with `clut_size == 4 << bits_per_pixel` and `texel_size == width * height / 2`,
on all five fonts, **with no padding at all**. The older reading needed 48
trailing bytes of unexplained padding to balance; there are none. The
`4 << bpp` sizing is independently what `FUN_08928550` in the executable
computes when it places the texel pointer after the CLUT.

### The palette becomes a palette

Read at `+0x40`, every one of the five fonts has **16 distinct alpha levels**
spanning 0 to 254 - exactly the quantised antialiasing ramp a 4-bit font atlas
needs. `pulse_text`, `Pulse_14` and `Pulse_20` are pure white with 16 alphas;
`PulseHud` and `small` are black with 16 alphas, which is what an outlined HUD
font looks like.

Read 48 bytes early the same fonts have four or five distinct alphas and a dozen
dead entries, and `small.fnt`'s brightest visible entry is alpha 26 of 255. No
font can be drawn with that, and it is the tell that should have been followed
sooner: the failure was never only in the pixel layout.

### The glyph boxes bound real ink

`u1 - u0` is the glyph's declared width, so the box is **tight horizontally** and
both edge columns have to be inked. Across 823 glyphs at least 3x3:

| Font | Glyphs checked | Edge columns inked |
| --- | ---: | ---: |
| `pulse_text` | 184 | 0.973 |
| `Pulse_14` | 199 | 0.967 |
| `Pulse_20` | 199 | 0.967 |
| `PulseHud` | 123 | **1.000** |
| `small` | 118 | **1.000** |

The same measurement on the old reading gives 0.42 overall - 0.399 on left
edges and 0.441 on right. The handful of misses here are glyphs whose outermost
antialiasing step quantises to the fully transparent palette entry.

`v0`/`v1` are deliberately **not** tested the same way: they are shared by every
glyph on an atlas row rather than tight to the ink, so a lowercase letter
legitimately leaves the top rows of its box empty. That asymmetry is itself a
finding - an earlier "ink inside the glyph box" metric that treated the vertical
box as tight was capped well below 1 even for a correct decode.

### It renders

`PulseHud.fnt` decodes to legible `0123456789`, `abcdefghijklmnopqrst` and two
rows of accented capitals; `Pulse_20.fnt` to the full ASCII range plus the PSP's
L, R, START, SELECT, HOME and face-button glyphs. That is the same standard
[`.mip`](psp-texture.md) was held to: previously-unseen artwork, not noise.

Confidence: **94**. The size identity is exact and holds on all five fonts, the
palette and edge-ink measurements are corpus-wide, and the swizzle direction and
the `flags` bit are read directly out of the executable rather than inferred.
Per the [rubric](../reverse-engineering/confidence-rubric.md) that is data
agreement plus static reading rather than a runtime trace, which caps at 94.

## Functions

| Address | Name | Conf |
| --- | --- | ---: |
| `0x08926da8` | `Texture_SwizzleForGe` | 90 |

`Texture_BindEmbeddedData` (`0x08927f28`) is documented in [`.vex`](vex.md).

## Reproducing

```sh
just test-data      # runs crates/formats/tests/fnt_ground_truth.rs
oag-wad extract 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad' -o /tmp/fe
```

The five `.fnt` entries are `00000`-`00004`. The test reports:

```text
  entry  0 256x128 lh=13 glyphs=197 checked=184 edge-ink=0.973
  entry  1 256x256 lh=17 glyphs=205 checked=199 edge-ink=0.967
  entry  2 256x256 lh=22 glyphs=205 checked=199 edge-ink=0.967
  entry  3 512x256 lh=25 glyphs=128 checked=123 edge-ink=1.000
  entry  4 256x128 lh=10 glyphs=128 checked=118 edge-ink=1.000
alpha levels {16: 5}
```

## Not determined

- **`+0x14`**, 0 in three fonts and 4 in two. The two with 4 are also the two
  whose codepoint table has no terminator and the two whose palette is black
  rather than white, so a version marker remains the best guess. It does not
  affect decoding.
- The five bytes at `+0x0d` of a glyph record. Mostly `0xff`; the space glyph has
  one `0x00`. Kerning is the guess.
- **Why some assets ship swizzled and others do not.** The `flags` bit is a
  general mechanism, and chasing it turned up a decoder bug: 6 of `FE.wad`'s 13
  standalone [`.mip`](psp-texture.md) textures set the same flag in their own
  header at `+0x07`, and were being decoded as noise. So the atlas is not the
  exception it first looked like. Which assets the pipeline swizzles ahead of
  time, and why, is still an open question.
- The `+0x18` field of the atlas header and the 40 zero bytes after the two
  pointers. Reserved, on the evidence of being zero in all five.

## Rendering it

`oag-game` still draws with [its own 5x7 glyphs](../../crates/game/src/font.rs);
wiring the real atlas into the front end is a separate change from decoding it.
That fallback matters less now, but how it *fails* is still worth keeping: an
accented letter with no glyph folds to its base letter rather than being
skipped, because losing a letter changes a word where losing an accent only
misspells it. The disc's `Français` used to come out as `FRANAIS`.
