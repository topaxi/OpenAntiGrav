# PSP indexed texture

**Status: understood.** Header, palette and pixel layout are decoded,
implemented in [`oag-texture::texture`](../../crates/texture/src/texture.rs),
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

**Validated against two discs**: `pulse-psp-usa` and `pure-psp-usa`. Pure's
three archives hold 346 standalone `.mip` entries, and all 346 decode and write
as PNG under the same reader, with 5 of them carrying the `+0x07` swizzle flag -
the same small minority as Pulse. Because identification is by the header's own
size arithmetic against the stored length, a wrong reading would fail to
identify rather than mis-decode, which is what makes the count evidence. See the
[Pure probe](pure-status.md). One caveat that does not apply to standalone
`.mip` files but does to the same texel format elsewhere: Pure's *embedded*
model textures ship [pre-swizzled](pure-status.md#pures-model-textures-ship-pre-swizzled)
where Pulse's do not.

## Coverage

`oag_texture::texture::coverage` claims the header fields this page decodes,
the palette and level 0's pixels, and leaves two gaps this page already
named in prose rather than in numbers: the seven bytes at `+0x09..+0x10`
(never read at all, not even into [`Texture::unknown`](../../crates/texture/src/texture.rs),
which stops at `+0x08`) and every mip level past 0 on a mipmapped blob
("only level 0 is decoded" above). Swept across 435 standalone `.mip` blobs
found in `FE.wad`, `FEData.wad`, `Data.wad` and `BEData.wad` on
`pulse-psp-usa`
(`crates/texture/tests/mip_fnt_coverage_ground_truth.rs`): **99.26%** of
14,075,440 bytes, the worst single file (44,688 bytes, mipmapped) at 75.64%
- 10,880 of its bytes are mip levels 1+, which this parser validates the
length of but never reads a pixel from.

## Open questions

### ~~Is the pixel data swizzled?~~ Sometimes, and `+0x07` says which

The earlier answer here - "no, and `unk_0x07` is therefore *not* a swizzle flag"
- was **wrong**, and it was wrong in the way a negative result reached by
sampling usually is: the textures that were looked at were the ones that decode
linearly.

`FUN_08928980` in the PSP executable builds a texture node from this header and
copies **bit 0 of `+0x07`** into the node's own flags byte. That is exactly the
bit `Texture_BindEmbeddedData` tests to decide whether to run
`Texture_SwizzleForGe` (`0x08926da8`, documented on [`.fnt`](fnt.md)), which
converts a linear image into the GE's 16-byte by 8-row block layout. A texture
that already has the bit set is **stored swizzled** and the game leaves it alone.

`+0x07` is 2 or 3 across `FE.wad`, so bit 0 is set on **6 of the 13** textures,
and five of those are wide enough for it to matter. They were decoding as noise.
`00006_cee1c5a4` at 256x128 is the clearest: horizontal streaks read linearly,
and the Japanese Wipeout logotype - katakana, `V5.0 //2197` and the mark - once
unswizzled.

The rest, including the 512x128 Pulse logo and the 256x256 icon sheet, have bit
0 clear and are genuinely linear, which is why decoding them literally worked.

`+0x08` is the complement of bit 0 of `+0x07` on every texture, so it is one
flag stated twice rather than two.

```sh
oag-wad extract data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad -o /tmp/fe --png
```

`Texture::looks_swizzled` remains as a triage heuristic for other archives, but
it reports nothing in `FE.wad`.

It is a heuristic, and a weak one on this corpus: it is tuned for the PSP's
16-byte-wide banding and does not fire on several textures that really are
swizzled. The **flag** is the reliable answer, not the statistic.

A better statistic, and the one the ground-truth test uses, is the **seam
ratio**: the mean pixel step across a 16-byte block boundary over the mean step
elsewhere. A correct decode has nothing special happening at those boundaries;
a wrong one splices unrelated pixels together there. Across the 12 textures wide
enough to discriminate, the reading `+0x07` selects is the seam-free one on 11,
and the exception is a 32x16 blob that is four blocks in total.

### ~~`unk_0x05` to `unk_0x08`~~ mostly answered

`+0x05` is 0 throughout. `+0x06` is 1 throughout and is the **mip count**:
`FUN_08928980` reads it into the texture node's mip-count field. `+0x07` bit 0
is the **swizzle flag**, above, and `+0x08` is its complement. What the other
bits of `+0x07` mean, and what `+0x05` is, is still open.

The [embedded texture node in `.vex`](vex.md#embedded-textures) has a
**`mip_count`** byte at the corresponding offset, which suggests `+0x05` or
`+0x06` is a mip count here.

### `+0x06` is the mip count - confirmed, and it was `Data.wad` that showed it

**Confidence 92.** This section previously read "every standalone `.mip` has
exactly one level, so the size arithmetic above holds for this corpus but would
not for a mipmapped texture - worth re-checking against `Data.wad`". That
re-check has happened and the prediction was right.

`Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip` - the ship exhaust sprite,
named as a literal in the executable at `0x08a84c80` - is **128x64 at 8bpp with
`+0x06 == 4`**, and it is **11,920 bytes** where a single level implies 9,232. The
chain accounts for the difference exactly, with each level halving both
dimensions:

| Part | Bytes | Running |
| --- | ---: | ---: |
| header | 16 | 16 |
| palette, 256 x RGBA8888 | 1,024 | 1,040 |
| level 0, 128x64 | 8,192 | 9,232 |
| level 1, 64x32 | 2,048 | 11,280 |
| level 2, 32x16 | 512 | 11,792 |
| level 3, 16x8 | 128 | **11,920** |

An exact hit on a four-term sum is not a coincidence, which is what puts this at
92 rather than in the 70s.

### Levels after the first pad their rows to 16 bytes

**Confidence 88**, from a second texture that the mip count alone did not explain.
`Data\Tex\engineFlare\Engine_noise.mip` is **64x64 at 8bpp with `+0x06 == 4`**
and **6,544** bytes, where a tightly packed chain implies 6,480. The 64-byte
difference is entirely level 3: an 8x8 whose 8-byte rows are padded to a 16-byte
stride, `8 * 16 = 128` instead of `8 * 8 = 64`.

| Texture | Levels | Packed | Padded tail | Blob |
| --- | ---: | ---: | ---: | ---: |
| `grabbedEngineFlare128x64x8` 128x64 | 4 | 11,920 | 11,920 | **11,920** |
| `Engine_noise` 64x64 | 4 | 6,480 | **6,544** | **6,544** |

**One rule, two exact hits.** The flare is unaffected because every one of its
levels is already at least 16 bytes wide, which is also why the mip fix landed
before this one was noticed. 16 is [`SWIZZLE_BLOCK_BYTES`] - the GE's texture row
alignment - so the two are the same constant rather than a coincidence.

**Level 0 is deliberately left unpadded**, and that asymmetry is a statement about
the evidence rather than the hardware: no observed texture has a level-0 row under
16 bytes, so the question is untested there, and leaving it alone keeps the
single-level arithmetic byte-identical to what the 346-entry corpus was verified
against. If a narrow single-level texture ever fails to identify, try this first. `oag_texture::texture` now reads the count, validates
the whole chain and decodes **level 0 only**, so `indices` stays exactly
`width * height` long - `png::encode_rgba` asserts that. The change only widens
what parses: at one level the arithmetic is unchanged, and all 346 previously
decoding entries still decode.

Two smaller notes from the same blob, both mild contradictions of the FE-only
corpus this page was written against:

- `+0x06` is `4` here against "1 in every standalone `.mip` seen". That is the
  finding above.
- `+0x07` is `1`, i.e. **bit 0 set, so this texture is swizzled**, and `+0x08` is
  `0` - its complement, as documented. The existing unswizzle path handles it with
  no change; worth recording only because it means `Data.wad` swizzles textures
  that `FE.wad`'s sample would have suggested were rare.

The corpus statement above ("all 346 decode") was measured before this fix and
covers `FE.wad`, `FEData.wad` and `Data`'s *standalone* set as then identified.
**A mipmapped entry would previously have failed identification rather than
mis-decoded**, so the count is a floor: re-running the sweep may now find more.

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
