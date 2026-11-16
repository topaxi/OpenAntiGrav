# Textures (PSP and PS2), and the GE swizzle

Indexed (palette) textures as shipped by Wipeout Pure and Pulse. Three related
forms:

| Form | Where | Platform |
| --- | --- | --- |
| Standalone `.mip` | Its own [WAD](wad.md) entry | PSP |
| Embedded `Texture` node | Inside a [`.vex`](vex.md) file | PSP |
| GS upload packet (`.pct`) | Its own WAD entry, one per model texture | PS2 |

Every form decodes to a palette plus one index per pixel. Palettes are
RGBA8888 (byte order R, G, B, A). All integers are little-endian. Confidence
scores follow the [rubric](../reverse-engineering/confidence-rubric.md).

## PSP standalone `.mip`

```text
+0x00  u16   width
+0x02  u16   height
+0x04  u8    bits_per_pixel   4 or 8
+0x05  u8    unknown          0 in every file seen
+0x06  u8    mip_levels       1 in most files; 4 on two engine textures
+0x07  u8    flags            bit 0: pixels are stored GE-swizzled
+0x08  u8    unknown          the complement of bit 0 of +0x07 (one flag stated twice)
+0x09  u8[7] zero
+0x10        palette: (1 << bpp) entries x 4 bytes (R, G, B, A)
             level 0 pixels
             further levels, each with both dimensions halved (floor at 1)
```

Level 0 holds `width * height * bpp / 8` bytes. 4-bit pixels pack two to a byte,
**low nibble first** (pixel 0 is the low half of byte 0), with no row padding at
level 0. Levels after the first pad each row up to a multiple of 16 bytes
(`row_bytes = ceil(w * bpp / 8)` rounded up to 16). The whole blob length is
fixed by the header, so a reader can reject a non-texture by size alone:

```text
16 + (1 << bpp) * 4 + sum over levels of (stride(level) * height(level))
```

Level 0 uses the unpadded row, the rest the 16-byte padded row. Only level 0 is
needed to draw. Whether level 0 would also pad a row narrower than 16 bytes is
untested, because no shipped texture has one; the reader leaves level 0
unpadded.

Confidence:

| Claim | Score | Basis |
| --- | ---: | --- |
| Header, palette, pixel layout, RGBA channel order | 94 | Size closes to the byte on all 346 standalone Pure `.mip` entries and the Pulse set; decodes to recognisable art |
| 4-bit low nibble first | 92 | The other order combs edges into one-pixel teeth; the 4-bit font atlases agree |
| `+0x06` is the mip count | 92 | The 128x64 engine flare is 11,920 bytes: `16 + 1024 + 8192 + 2048 + 512 + 128`, exactly |
| Levels 1+ pad rows to 16 bytes | 88 | A 64x64 noise texture is 6,544 bytes, 64 more than packed, all in an 8x8 level 3 |
| Bit 0 of `+0x07` is the swizzle flag | not scored | Copied into the texture node's flags by the loader; the unswizzled result is seam-free on 11 of the 12 textures wide enough to judge |

## PSP embedded texture (inside `.vex`)

A `Texture` node (class `0x3c1`, see [vex](vex.md)) carries:

```text
+0x00  u16   width
+0x02  u16   height
+0x04  u8    bits_per_pixel   4 or 8
+0x05  u8    mip_count
+0x06  u8    flags            bit 0: swizzled
+0x07  u8    texture index
+0x08  u32   clut_size        bytes
+0x0c  u32   texel_size       bytes, all mip levels
+0x10  u32   0 in the file, patched at load
+0x14  u32   0 in the file, patched at load
+0x38  char[] runtime asset path, e.g. Data\Ships\Feisar\Textures\texture1.tga
```

There is no pointer to pixel data. After the node tree, each texture's data is
packed in node order: `clut_size` bytes of palette, then `texel_size` bytes of
texels (base level, then mips). The sizes sum exactly to the model's declared
texture-block length.

Unlike a `.mip`, **every level here, including level 0, pads its rows** to
`align_up(width * bpp / 8, 16)`. This is a no-op except for narrow textures
(under 32 pixels at 4 bpp, under 16 at 8 bpp); 509 of 5,017 PSP textures have a
padded base level. If the flag byte has bit 0 set, each level is swizzled at its
own padded stride.

Confidence 90. Summing the padded strides over the declared levels reproduces
the node's `texel_size` for 5,017 of 5,017 textures; the unpadded sum
reproduces 775.

On Pure, embedded model textures ship pre-swizzled where Pulse's do not. Read
the flag, do not assume.

## The GE swizzle

The PSP's graphics engine (GE) reads texture memory in blocks of **16 bytes by 8
rows**. A swizzled image stores those blocks one after another in raster order
of the blocks; inside a block, its 8 rows of 16 bytes are stored one after the
other. To unswizzle, with `row_bytes = width * bpp / 8`:

```rust
let mut read = 0;
for block_row in 0..height / 8 {
    for block_col in 0..row_bytes / 16 {
        for row in 0..8 {
            let dst = (block_row * 8 + row) * row_bytes + block_col * 16;
            out[dst..dst + 16].copy_from_slice(&src[read..read + 16]);
            read += 16;
        }
    }
}
```

Rules that matter:

- It works on bytes, so it is the same for 4-bit and 8-bit textures. Unpack
  nibbles after unswizzling.
- If `row_bytes` is 16 or `height` is under 8, the swizzle is the identity.
- A height that is not a multiple of 8 leaves a tail that is copied through.
- The flag, not a statistic, says whether a blob is swizzled. A heuristic that
  compares neighbouring rows is weak on this corpus.

The same swizzle applies to `.fnt` glyph atlases, `.mip` and `.vex` embedded
textures.

This is the inverse of the game's own swizzle routine (`Texture_SwizzleForGe`),
and the flag is read from the game's loader. Not scored on the evidence page.

## PS2 texture (`.pct`)

The PS2 does not store a texture file. It stores the **DMA packet that uploads
one** into the Graphics Synthesizer (GS): a 13-byte header the game reads,
then GIF packets for the texels and the palette. Pixels are in GS layout, not
raster order.

```text
+0x00  u8    log2 dims: high nibble log2(height), low nibble log2(width)
+0x01  u8    bits_per_pixel   4 or 8
+0x02  u16   flags            0x2000 or 0x2040 (meaning unknown)
+0x04  u16   height           (height first, then width)
+0x06  u16   width
+0x08  u32   unknown
+0x0c  u8    unknown
+0x0d  qword[8]   GS state, not decoded
+0x8d  qword      A+D write TRXPOS (register 0x51)
+0x9d  qword      A+D write TRXREG (0x52): the transfer rectangle
+0xad  qword      A+D write TRXDIR (0x53)
+0xbd  qword      GIFtag, FLG=IMAGE; NLOOP*16 == texel bytes
+0xcd  texels, width * height * bpp / 8 bytes
       3 x A+D qwords (TRXPOS, TRXREG, TRXDIR)
       qword      GIFtag, FLG=IMAGE, EOP; NLOOP*16 == palette bytes
       palette, (1 << bpp) * 4 bytes
       padding
```

Total length is `205 + max(texel_bytes, 256) + 64 + max(palette_bytes, 256)`,
but the blocks sit back to back and the slack falls at the end (a 4x4 texture is
1,549 bytes, 240 of them trailing padding). So the palette's offset is
`0xcd + texel_bytes + 64`, not padded.

Width and height are stored **height first**. Square textures decode either way;
the rest only decode correctly height first. `TRXREG` settles it.

### Three transfer shapes

The `TRXREG` rectangle selects the pixel layout:

| `TRXREG` (w x h) | Layout | Count on the PS2 disc |
| --- | --- | ---: |
| `(width/2, height/2)`, 8 bpp | 8-bit texels sent as 32-bit pixels, pre-swizzled to GS `PSMT8` order | 5,077 |
| `(width, height)`, 8 bpp | Raster order, linear | 266 |
| `(width/2, height/4)`, 4 bpp | Same trick for 4-bit (`PSMT4`); the five font atlases | 5 |

The linear case is used when the width is 4 or 8, where the halved rectangle
cannot be formed. To recover raster order, for each destination `(x, y)` read
the stored byte at the offset below.

PSMT8 (`width >= 16`, `height >= 4`, both powers of two):

```rust
fn psmt8_offset(x: usize, y: usize, width: usize) -> usize {
    let block  = (y & !0xf) * width + (x & !0xf) * 2;
    let swap   = (((y + 2) >> 2) & 1) * 4;
    let row    = ((((y & !3) >> 1) + (y & 1)) & 7) * width * 2;
    let column = ((x + swap) & 7) * 4;
    let byte   = ((y >> 1) & 1) + ((x >> 2) & 2);
    block + row + column + byte
}
```

PSMT4 (`width >= 128`, `height >= 128`), returning the byte offset and the nibble
(0 low, 1 high):

```rust
fn psmt4_offset(x: usize, y: usize, width: usize) -> (usize, usize) {
    let page   = (x / 128) * 64 + (y / 128) * 32 * (width / 2);
    let block  = ((y % 128) / 16) * 8 + ((x % 128) / 32) * 8 * (width / 2);
    let swap   = (((y + 2) >> 2) & 1) * 4;
    let column = (x + swap) & 7;
    let row    = (((y & !3) >> 1) + (y & 1)) & 7;
    let word   = page + block + row * (width / 2) + column;
    (word * 4 + ((x >> 3) & 3), (y >> 1) & 1)
}
```

### Palette order and alpha

A 256-entry palette is stored in GS `CSM1` order, where within each group of 32
entries, entries 8 to 15 and 16 to 23 are swapped. Undo with:

```rust
palette[i] = stored[(i & 0xe7) | ((i & 0x08) << 1) | ((i & 0x10) >> 1)]
```

A 16-entry palette needs no reordering. A missed texel swizzle scrambles the
image into blocks; a missed palette swap keeps shapes intact and bands the
colours every eight indices.

Palette alpha runs 0 to 128, where 128 is full intensity (the GS convention).
Multiply by 2 to get 0 to 255; no shipped palette entry exceeds 128.

### Where textures live and which one a model uses

A PS2 `.vex` declares a texture block of length zero. Its textures are separate
WAD entries named by the same path with the extension replaced by `.pct`
(`.mip` and `.tga` both become `.pct`) after stripping
`WIPEOUT PSP\PS2\` wherever it occurs. A name that resolves to nothing falls back to
`Data/Tex/Missing.pct`. A model's textures are also gathered into one nested
WAD, one entry per `Texture` node in node order, for locality; each is also a
top-level entry.

### Confidence

| Claim | Score | Basis |
| --- | ---: | --- |
| Header, packet framing, height-first, alpha scale | 94 | 5,348 of 5,348 blobs: the log2 byte agrees with the dimensions, both `NLOOP` fields equal the implied byte counts, the register bytes are 0x51/0x52/0x53 |
| PSMT4 permutation | 93 | Derived from the PSMT8 formula; the non-transposed reading is not even a bijection; every lit texel of one font atlas falls inside a glyph box its metrics declare |
| PSMT8 permutation, linear/swizzled split, CSM1 palette | 92 | 98% of 4,956 textures are smoother under this reading than the alternative; 185 of 185 linear ones smoother linear; ship textures land on the right panels |

## Other platforms

The newer titles use platform containers, not these:

| Platform | Container | Page |
| --- | --- | --- |
| PS3 (HD) | `.gtf` | [`docs/formats/gtf.md`](../formats/gtf.md) |
| Vita (2048) | `.gxt` (PVRTC, BC1-3, raw) | [`docs/formats/gxt.md`](../formats/gxt.md) |
| PS4 (Omega) | `.gnf` (BC, tiled) | [`docs/formats/gnf.md`](../formats/gnf.md) |

## Known unknowns

- `.mip` byte `+0x05`, and what bits other than 0 of `+0x07` mean.
- The PS2 `flags` word (`0x2000` vs `0x2040`) and the unknown dword at `+0x08`.
- Direct-colour PSP formats (5650, 4444, 8888, DXT): only 4 and 8 bit indexed
  textures appear in the archives surveyed (`FE.wad` and the standalone `.mip`
  set); `Data.wad` has not been surveyed for others.
- Level-0 row padding for a `.mip` narrower than 16 bytes.
- `psmt8_offset` below width 16 or `psmt4_offset` below 128x128 is not defined
  for the PS2; the disc has no such swizzled texture.

## Evidence

- Evidence pages: [`docs/formats/psp-texture.md`](../formats/psp-texture.md),
  [`docs/formats/ps2-texture.md`](../formats/ps2-texture.md),
  [`docs/formats/vex.md`](../formats/vex.md#embedded-textures),
  [`docs/formats/fnt.md`](../formats/fnt.md).
- Readers: [`crates/texture/src/texture.rs`](../../crates/texture/src/texture.rs),
  [`crates/texture/src/ps2_texture.rs`](../../crates/texture/src/ps2_texture.rs),
  [`crates/formats/src/swizzle.rs`](../../crates/formats/src/swizzle.rs).
