# PS2 texture

**Status: understood.** Located, decoded, implemented in
[`oag-formats::ps2_texture`](../../crates/formats/src/ps2_texture.rs) and
rendering: `oag-view --mesh ... --textures ...` draws the PS2 Feisar ship in
its own livery instead of the white silhouette it was.

Found as standalone entries in the PS2 [WAD](wad.md) archives. Not the same
format as the [PSP `.mip`](psp-texture.md) at all, and not embedded in the model
the way the PSP's are: see [`.vex`](vex.md#embedded-textures).

## Where they are

The PS2 `.vex` files declare a texture block of length **zero**. Their textures
are **separate WAD entries**, and there are 5,348 of them across `WADS2.WAD` and
`WADSP.WAD`.

A model's set is also gathered into a **nested WAD** — the same 8-byte header
and 16-byte entries as the outer archive, [`oag-formats::wad`](wad.md) parses it
unchanged — with one entry per `Texture` node of the model, **in node order**.
For `Data\Ships\Feisar\Ship.vex` that set is entry `0xfc3f75cf`: 5 entries
against the model's 5 `Texture` nodes, and decoding them gives the Feisar hull
atlas, its lights sheet, an environment map and a glass gradient, matching the
node names `Textures_All`, `Lights_GLOW`, `envtest4bit`, `Glass2_ADD`,
`blink_GLOW`.

Every one of the 2,252 distinct hashes inside the 443 texture sets in
`WADS2.WAD` is *also* a top-level entry of the same archive, so the sets are a
locality optimisation rather than the only copy.

## Layout

```text
+0x00  u8    packed log2 dimensions, high nibble log2(height), low nibble log2(width)
+0x01  u8    bits_per_pixel      4 or 8
+0x02  u16   flags               0x2000 or 0x2040
+0x04  u16   height
+0x06  u16   width
+0x08  u32   unknown, correlates with +0x0c
+0x0c  u8    unknown
+0x0d  qword[8]  GS state, not decoded
+0x8d  qword     A+D register write, TRXPOS (0x51)
+0x9d  qword     A+D register write, TRXREG (0x52)
+0xad  qword     A+D register write, TRXDIR (0x53)
+0xbd  qword     GIFtag, FLG=IMAGE, NLOOP*16 == texel bytes
+0xcd  texels, width * height * bpp / 8 bytes
       qword     A+D register write, TRXPOS
       qword     A+D register write, TRXREG
       qword     A+D register write, TRXDIR
       qword     GIFtag, FLG=IMAGE, EOP, NLOOP*16 == palette bytes
       palette, (1 << bpp) * 4 bytes, RGBA8888
       padding
```

The blob is not a texture *file*. It is the **DMA packet that uploads one**: a
13-byte header the game reads, then GIF packets that transfer the texels and
then the palette into GS local memory. `BITBLTBUF` is absent, so the destination
buffer is chosen by code, not by the file.

Each transfer block is budgeted 256 bytes even when it needs fewer, so the total
size is

```text
205 + max(width * height * bpp / 8, 256) + 64 + max((1 << bpp) * 4, 256)
```

but the blocks themselves sit back to back and the slack lands at the **end** of
the file. A 4x4 texture is 1,549 bytes, 240 of them trailing padding. Padding
the palette *offset* instead of the total reads correctly on all but 70 of the
disc's textures and then falls apart on the small ones.

## The dimensions are stored height first

`+0x04` is the **height** and `+0x06` the width. Reading them the other way
round decodes every square texture perfectly — which is most of them — and
scrambles every other one.

`TRXREG` is what settles it, because it is a fixed function of the dimensions
and it is asymmetric:

| Stored `+0x04`, `+0x06` | `TRXREG` | Consistent with |
| --- | --- | --- |
| 256, 128 | 64 x 128 | width 128, height 256 |
| 128, 256 | 128 x 64 | width 256, height 128 |
| 64, 128 | 64 x 32 | width 128, height 64 |

Under the width-first reading none of those are `(w/2, h/2)`, `(w, h)` or
anything else with a rule; under the height-first reading every one of the 5,348
textures matches one of exactly three shapes.

## Three transfer shapes, and the file says which

Dividing the texel byte count by the `TRXREG` area gives the bytes per
destination pixel, which identifies the transfer format from the data rather
than by assumption:

| `TRXREG` | Bytes/pixel | Meaning | Count |
| --- | ---: | --- | ---: |
| `(width/2, height/2)` | 4 | 8-bit texels blitted as **PSMCT32** | 5,077 |
| `(width, height)` | 1 | direct **PSMT8** transfer | 266 |
| `(width/2, height/4)` | 4 | 4-bit texels blitted as PSMCT32 (**PSMT4**) | 5 |

The first is the standard PS2 trick for uploading an indexed texture: the GS has
no host-to-local path that is fast for 8-bit, so the data goes across as 32-bit
pixels and is **pre-swizzled into `PSMT8` order** on disc so that reading it back
as `PSMT8` comes out right. That swizzle is what has to be undone, and the
decoder does:

```rust
fn psmt8_offset(x, y, width) -> usize {
    let block = (y & !0xf) * width + (x & !0xf) * 2;
    let swap = (((y + 2) >> 2) & 1) * 4;
    let row = ((((y & !3) >> 1) + (y & 1)) & 0x7) * width * 2;
    let column = ((x + swap) & 0x7) * 4;
    let byte_select = ((y >> 1) & 1) + ((x >> 2) & 2);
    block + row + column + byte_select
}
```

This is a permutation of `0..width * height` only for widths of 16 and up, and
the widths where it is not are **exactly** the ones the game stores linearly
instead — 4 and 8. That is a satisfying corroboration rather than a coincidence:
the halved rectangle the 32-bit path needs cannot be formed below that width, so
the game falls back, and the fallback set the arithmetic predicts is the set the
data has. The three widths that use the direct path *and* could have used the
other one (32x128, 64x128, 512x512) are not explained.

## The palette is in `CSM1` order

A 256-entry CLUT is uploaded as a 16x16 `PSMCT32` rectangle, which is `CSM1`
layout: within each group of 32 entries, 8-15 and 16-23 are the other way round.
Undoing it is a bit swap on the index:

```rust
palette[i] = stored[(i & 0xe7) | ((i & 0x08) << 1) | ((i & 0x10) >> 1)]
```

The two failure modes are distinguishable on sight, which is worth knowing
before debugging one: a missed **texel** swizzle scrambles the image into blocks,
a missed **palette** swap leaves the shapes perfectly intact and bands the
colours every eight indices.

A 16-entry CLUT is a plain 8x2 rectangle and needs no reordering.

## Alpha is 0-128

Palette alpha runs 0 to 128, not 0 to 255: the GS treats 128 as full intensity
through the texture-modulate path, the same convention `.vex` documents for
[vertex colour](vex.md#ps2-the-vertex-type-still-names-the-attributes-but-the-data-is-a-vif-packet).
`Ps2Texture` keeps the palette exactly as stored and `to_rgba` doubles it, so a
caller that wants the raw bytes still has them. Not one of the 5,343 decoded
textures has a palette entry above 128, which the ground-truth test asserts.

## Evidence

### The arithmetic

Almost every field is declared twice, and the two declarations have to agree:

- The **packed log2 byte** at `+0x00` and the dimension words at `+0x04` say the
  same thing in two encodings. 5,348 of 5,348 agree; nothing else in either
  archive does.
- Both **`GIFtag`s** declare their payload in quadwords, and `NLOOP * 16` has to
  come out at exactly the texel and palette byte counts the dimensions imply.
  This is a field *in the file* stating the decoded size, which is a much
  stronger check than a size delta: 5,348 of 5,348, `FLG` = 2 (`IMAGE`) on both,
  `EOP` set on the second.
- The **register addresses** at `+0x95`, `+0xa5`, `+0xb5` are 0x51, 0x52, 0x53 —
  `TRXPOS`, `TRXREG`, `TRXDIR`, in that order, on all 5,348.
- The **total closes**, by the formula above, on 5,348 of 5,350 blobs that pass
  the log2 check.

### It renders

`oag-view --mesh "Data\Ships\Feisar\Ship.vex" --textures 0xfc3f75cf` against
`WADS2.WAD` draws the ship in Feisar's blue and yellow with the team name legible
along the hull and the UVs landing on the right panels. Decoded standalone, the
same textures come out as recognisable ship atlases, a logo sheet and panel
artwork rather than noise.

Confidence: **94** for the header, both packet shapes, the `PSMT8` swizzle, the
`CSM1` palette order and the alpha scale. Every declared size closes against
every other one across 5,348 real files, and decoding produces
previously-unseen, correctly-coloured artwork that lines up with a separately
decoded mesh's texture coordinates. Per the
[rubric](../reverse-engineering/confidence-rubric.md) that is data agreement
rather than a runtime trace, which caps at 94.

## Reproducing

```sh
just test-data                       # runs the ground-truth test below
just view 'data/images/pulse-ps2-eu.chd:54748/WADS2.WAD' \
  --mesh 'Data\Ships\Feisar\Ship.vex' --textures 0xfc3f75cf --screenshot /tmp/feisar.png
```

The ground-truth test is
[`crates/formats/tests/ps2_texture_ground_truth.rs`](../../crates/formats/tests/ps2_texture_ground_truth.rs).
It walks both PS2 archives and reports:

```text
blobs        7377
textures     5348
decoded      5343
refused      5
pixels       140638128
layouts      {"linear": 266, "psmt4": 5, "psmt8": 5077}
flags        {8192: 3925, 8256: 1423}
shapes       39
```

## Not determined

- **How the game finds a model's texture set.** The set is a WAD entry like any
  other, so it is looked up by name hash, and the name is not recovered. It is
  not `Ship.<anything>` under the ship's own directory: an exhaustive search over
  extensions up to four characters of `[a-z0-9_]` for eleven plausible stems
  found nothing, and the hash appears nowhere in the `.vex` or in any other blob
  on the disc. Until it is known, `oag-view` takes the entry explicitly. Pairing
  by directory adjacency is **not** good enough: the nearest preceding set has
  the right entry count for only 535 of the 975 models that have `Texture` nodes.
- **The `PSMT4` swizzle**, so five 4-bit textures do not decode. `parse` refuses
  them by name rather than guessing.
- **`flags` at `+0x02`.** 0x2000 on 3,925 textures and 0x2040 on 1,423. Nothing
  correlates it with dimensions or depth.
- **`+0x08` and `+0x0c`.** They move together — `0x00400000` with 68,
  `0x00100000` with 20, `0x01000000` with 4 — and look like a GS memory
  allocation, which the file would have no reason to fix. Not needed to decode.
- **The eight quadwords at `+0x0d`.** GS state the game replays; `TEX0`-shaped
  values are in there but nothing depends on reading them.
- **The `Texture` node payloads in a PS2 `.vex` are stale.** They still carry
  PSP-shaped `clut_size`/`texel_size` including mip chains, and disagree with the
  actual texture: the Feisar node 0 says 512x512 8bpp with 6 mips, the entry the
  game loads is 256x256 with one level. The PS2 blobs carry **no mips at all** —
  the size formula would not close if they did.
- **Two outliers.** Entries `0xb077d3bd` and `0x8d17fa0d` of `WADS2.WAD` start
  with a valid 256x256 texture packet and then continue with more GIF packets to
  561,667 and 1,123,333 bytes. Some multi-part resource with a texture on the
  front; refused rather than half-decoded.
