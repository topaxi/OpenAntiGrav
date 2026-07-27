# PSP indexed texture

**Status: understood.** Header, palette and pixel layout are decoded,
implemented in [`oag-formats::texture`](../../crates/formats/src/texture.rs),
and confirmed visually: decoded textures render as the Wipeout Pulse logo and
front-end icon sheets. Three header bytes remain unidentified but do not affect
decoding.

Found inside [WAD](wad.md) archives. Confirmed on Wipeout Pulse PSP; not yet
checked against PS2, whose blobs are compressed.

## Layout

```
+0x00  u16    width
+0x02  u16    height
+0x04  u16    bits_per_pixel      4 or 8
+0x06  u8     unk_0x06            always 1 in the sample
+0x07  u8     unk_0x07            2 or 3
+0x08  u8     unk_0x08            0 or 1
+0x09  u8[7]  zero
+0x10  RGBA8888 palette, (1 << bits_per_pixel) entries
       pixel indices, width * height * bits_per_pixel / 8 bytes
```

Total size is therefore:

```
16 + (1 << bpp) * 4 + width * height * bpp / 8
```

## Evidence

### Size arithmetic

Taking the first two `u16` values as dimensions, the file size minus
`width * height` is **exactly 1040** for twelve of the thirteen candidates in
`FE.wad`, and 1040 is `16 + 256 * 4`: a header plus a 256-entry RGBA palette.

| Blob | Size | w | h | `size - w*h` |
| --- | ---: | ---: | ---: | ---: |
| `00005_f7109b8e` | 66,576 | 512 | 128 | 1040 |
| `00006_cee1c5a4` | 33,808 | 256 | 128 | 1040 |
| `00007_4b3adf53` | 9,232 | 128 | 64 | 1040 |
| `00015_be8d8369` | 5,136 | 64 | 64 | 1040 |
| `00017_57d37d8c` | 66,576 | 256 | 256 | 1040 |
| `00010_28885c41` | 592 | 32 | 32 | **-432** |

The outlier is the giveaway. At 4 bits per pixel with a 16-entry palette:
`16 + 16*4 + 32*32/2 = 592`, exactly.

### The depth field

Comparing headers of the outlier against an 8-bit texture of identical
dimensions isolates `+0x04`:

```
32x32 4bpp:  20 00 20 00 04 00 01 03 00 00 00 00 00 00 00 00
32x32 8bpp:  20 00 20 00 08 00 01 03 00 00 00 00 00 00 00 00
                         ^^
```

### Palette and pixels

Reading `(1 << bpp)` RGBA quads from `+0x10` gives coherent palettes, and the
remaining bytes match `w * h * bpp / 8` exactly.

An 8-bit UI element, white throughout with a stepped alpha ramp:

```
(255,255,255,0) (255,255,255,128) (255,255,255,255) (255,255,255,16) ...
distinct alpha: 0, 16, 32, 48, 64, 80, 96, 128, ...
```

A 4-bit greyscale ramp with matching alpha:

```
(0,0,0,0) (32,32,32,38) (70,70,70,80) (104,104,104,128) (130,130,130,159) ...
```

Both are exactly what alpha-masked UI artwork looks like, and rendering confirms
it: the 256x256 icon sheet is white artwork whose shape lives entirely in the
alpha channel.

Channel order is RGBA, confirmed by the logo rendering in the correct cyan
rather than a channel-swapped orange.

Confidence: **94** for header, palette, pixel layout and channel order. The
stored byte count matches `w * h * bpp / 8` exactly, and decoding renders
recognisable, previously-unseen artwork (the Pulse logo, an icon sheet) rather
than noise. Per the [rubric](../reverse-engineering/confidence-rubric.md), that
is data agreement rather than a runtime trace, so it caps at 94; the earlier 95
predated the rubric saying so. The 4-bit nibble order (low nibble first) is
**92**: the only 4bpp `.mip` is a symmetric hexagon, so a mirrored decode does
not move the silhouette, but the comparison is decisive on inspection anyway -
low nibble first gives smooth gradient bands where high nibble first combs every
edge into one-pixel teeth. The [`.fnt`](fnt.md) atlas is a second, much larger
4bpp corpus and uses the same order.

## Open questions

### ~~Is the pixel data swizzled?~~ No

Answered by looking. Decoding linearly and writing PNG produces the **Wipeout
Pulse logo** at 512x128 8bpp, and a coherent icon sheet at 256x256. Swizzled
data decoded linearly would be scrambled into 16-byte-wide blocks; these are
pixel-perfect.

So the textures are stored linearly and the PSP swizzles at upload time, if at
all. `unk_0x07` and `unk_0x08` are therefore *not* swizzle flags.

```sh
oag-wad extract data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad -o /tmp/fe --png
```

`Texture::looks_swizzled` remains as a triage heuristic for other archives, but
it reports nothing in `FE.wad`.

The **why** is now read out of the executable rather than assumed.
`Texture_SwizzleForGe` (`0x08926da8`, documented on [`.fnt`](fnt.md)) converts a
linear image into the GE's 16-byte by 8-row block layout in place, and
`Texture_BindEmbeddedData` runs it on any texture whose `flags` bit 0 is clear,
setting the bit afterwards. Linear on disc is therefore the rule, and the only
exception found so far is the `.fnt` glyph atlas, which ships with that bit
already set and so is stored swizzled.

### `unk_0x05` to `unk_0x08`

`+0x05` is 0 and `+0x06` is 1 throughout; `+0x07` is 2 or 3 and `+0x08` is 0 or
1, inversely correlated and split by texture size.

The [embedded texture node in `.vex`](vex.md#embedded-textures) has a
**`mip_count`** byte at the corresponding offset, which suggests `+0x05` or
`+0x06` is a mip count here. Every standalone `.mip` has exactly one level, so
the size arithmetic above holds for this corpus but **would not** for a
mipmapped texture. Worth re-checking against `Data.wad`.

### Non-indexed formats

Only 4- and 8-bit indexed textures appear in `FE.wad`. The PSP also supports
direct-colour 5650, 5551, 4444 and 8888, and DXT. Whether Pulse uses them, and
how `bits_per_pixel` would encode them, is unknown; `Data.wad` is the place to
look.

## Reproducing

```sh
oag-wad extract data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad -o /tmp/fe
```

Then read `u16 width, u16 height, u16 bpp` from the start of each blob and
check the total against the formula above.
