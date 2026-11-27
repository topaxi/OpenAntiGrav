# Texture levels: how the PS2 build decides `MXL`

`SCES_547.48` (Pulse PS2 EU), image base `0x00100000`. Read 2026-11-27. The runtime side (what a race
actually sends the GS) and the disc census are in
[texture-level-per-title.md](../../../rendering/texture-level-per-title.md#pulse-ps2-in-a-race).

## The law

The GS samples `flags & 0x1f` levels beyond the base, where `flags` is the byte at `+2` of the texture
file's header. The engine can upload and sample a chain; **no texture on this disc authors one** (14,005
of 14,005 texture objects hold 0). Nothing builds a level at load or upload time.

| Address | Name | Conf | What it does |
| --- | --- | ---: | --- |
| `0x00222798` | `Texture_ReadFromStream` | 90 | Reads a serialized texture object and builds its upload DMA chain |
| `0x00220e30` | `Texture_BuildGsState` | 92 | Builds the object's `TEX0`, `TEX1`, `MIPTBP1`, `MIPTBP2`, `CLAMP` packet |
| `0x00221138` | `Texture_RebuildSamplerRegisters` | 85 | Rewrites `TEX0`'s `TCC`/`TFX`, `TEX1` and `CLAMP` after a sampler change |
| `0x00221348` | `Texture_ComputeLevelBlocks` | 90 | VRAM block offset of each level and of the CLUT |
| `0x0026f6a0` | `Texture_SetSamplerState` | 80 | Virtual setter (vtable `0x0029cc00 + 0x34`) for the sampler fields, then `Texture_RebuildSamplerRegisters` |

## `Texture_ReadFromStream` `0x00222798`

Reached by name through `Texture_FindOrLoad` -> `FUN_0021a9c0`, and from `FUN_0021a7d0`. It reads, in
order, into the object:

| File | Bytes | Object | Meaning |
| --- | ---: | --- | --- |
| `+0x00` | 8 | `+0x90` | packed log2 size, bits per pixel, `flags`, CLUT bits per entry, height, width |
| `+0x08` | 18 | `+0xb0` | nine `u16` VRAM block offsets: level 0, 1, ..., then the CLUT |
| `+0x1a` | 7 | `+0x16c` | pixel format per level |
| `+0x21` | 72 | (local) | nine `u64`, copied into the upper half of the nine DMA tags |
| `+0x69` | 36 | `+0x194` | nine `u32` packet sizes in quadwords: one per level, then the CLUT |

It then clears bit 5 of `flags`, reads `(offset[MXL + 2] * 16 + (MXL + 1) * 4 + 4) * 16` bytes of upload
packets (with a CLUT), points one DMA `ref` tag at each of the `MXL + 1` level packets and the CLUT packet,
and calls `Texture_BuildGsState`. The packets go to the GS as they sit in the file: no pixel is touched
and nothing is resampled. `MXL` here is `flags & 0x1f`.

## `Texture_BuildGsState` `0x00220e30`

Five `A+D` pairs. `TEX1` (register `0x14`) is
`LCM(+0xa4) | (flags & 0x1f) << 2 | MMAG(+0x98) << 5 | MMIN(+0x9c) << 6 | K(+0xa8) << 32`, with
`MTBA` (bit 9) set only when `flags & 0x1f` is 0. `MIPTBP1`/`MIPTBP2` are packed from the six block
offsets at `+0xb2..+0xbc` and `width >> level`.

On a single-level texture those six slots are not levels: `offset[1]` is where the CLUT sits, `offset[2]`
is the block after it, and the rest are 0. So the register's level 1 points at the CLUT and level 3 at the
base. A GS dump shows exactly that (`MIPTBP` writes on every texture bind, none of them naming a level
image), and with `MXL` 0 the GS never reads them.

The loader sets the sampler fields first (through `Texture_SetSamplerState`): `MMAG` 1, `MMIN` 1
(linear, **no mipmapping**), `LCM` 0, `K` 0. With `MTBA` that is `TEX1 = 0x260`, the value a race writes.

## `Texture_RebuildSamplerRegisters` `0x00221138`

The same `flags & 0x1f` goes into `MXL`. It forces `LCM` 1 and takes `K` from `+0xa8` (scaled differently
by `+0xa4`). No other source of a level count.

## The two constructors that do not read a file

Both go through `FUN_00221458`, and both clear the level bits first:

- `FUN_0021a058` -> `FUN_002223b8`: header masked with `0xffffffffffc0ffff` (bits 16-21 cleared);
- `FUN_0021a960` -> `FUN_00222510`: header masked with `0xffffffffffa0ffff` (bits 16-20 cleared).

`FUN_00133548` and `FUN_001334c8` also write a halfword at `+0x92`, but of a bounding-box record, not a
texture.

## The other `TEX1` constant

`FUN_001d6bf8` and `FUN_0025d290` emit `TEX1 = 0x61` (`LCM` 1, `MXL` 0, linear) with a `TEX0` of their own.

## Not read

- Every GIF emitter in the binary that writes register `0x14`. The functions above are the texture class
  and the two constant sites found by their value; the race dumps are what covers the rest.
- `FUN_0021a7d0`'s caller.
