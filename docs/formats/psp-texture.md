# PSP indexed texture

**Status: partial.** Header, palette and pixel layout are decoded and confirmed
by exact size arithmetic across 13 textures. Three header bytes remain unknown,
and whether the pixel data is swizzled is **not yet established** and matters
for display.

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

Both are exactly what alpha-masked UI artwork looks like. Channel order is taken
as RGBA on the strength of the alpha column ramping smoothly while the other
three stay constant; a different order would make the constant channel the
alpha, which these images' appearance contradicts.

Confidence: **90** for the header, palette and pixel layout. Not higher because
nothing has been rendered yet.

## Open questions

### Is the pixel data swizzled?

**This is the important one.** The PSP GPU reads textures in a swizzled layout,
and games commonly store them pre-swizzled to avoid converting at load time. If
these are swizzled, decoding them literally produces a recognisable but scrambled
image, in 16-byte-wide blocks.

`unk_0x07` (2 or 3) and `unk_0x08` (0 or 1) are the candidates for a swizzle
flag. They are inversely correlated in the sample, and split by size: the 32x32
and 32x16 textures carry `03`/`00` while the 512x128, 256x256 and 64x64 carry
`02`/`01`. A size-dependent split is consistent with swizzling, which is only
worth doing above some dimension.

Resolving this needs a renderer, so it is deferred until there is one. Until
then, treat decoded pixels as provisional.

### `unk_0x06`

Always 1 in this sample. Plausibly a mipmap count, in which case a texture with
mipmaps would carry a larger value and additional pixel data, and the size
arithmetic above would need revisiting.

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
