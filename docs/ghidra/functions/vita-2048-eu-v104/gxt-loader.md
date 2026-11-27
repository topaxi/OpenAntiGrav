# 2048's GXT loader: every loaded texture is trilinear with a -1.5 LOD bias

**Binary:** `vita-2048-eu-v104` `eboot.elf`, image base `0x81000000`, Ghidra
`/2048/eboot-vita-2048-eu-v104.elf`. Read 2026-11-27 (lane `farfield-lod-titles`). The GXM imports
are named by NID from vitasdk's `vita-headers` database (`db/360/SceGxm.yml`). Per-title summary:
[texture-level-per-title.md](../../../rendering/texture-level-per-title.md).

## The function

`GxtLoader_LoadTexture` (`0x812f0ec4`, confidence **85**) loads one `.gxt` into a
`Pal::Render::Texture`. Its own messages name it:

- the allocation tag `"PSP2/Psp2.GxtLoader.cpp"`;
- `"The GXT file is abnormally to small. 64 bytes are required at least for the header section."`;
- `"Not enough Vram and MainUncache memory available to allocate GXT texture %s"`.

It reads the 64-byte header and allocates the texture object (`0x14` bytes) and the texel memory. It
then initialises the `SceGxmTexture` by the header's type word:

| Type word | Init call |
| --- | --- |
| `0x00000000` | `sceGxmTextureInitSwizzled` |
| `0x60000000` | `sceGxmTextureInitLinear` |
| `0x40000000` | `sceGxmTextureInitCube` |
| `0xa0000000` | `sceGxmTextureInitSwizzledArbitrary` |
| `0xe0000000` | `sceGxmTextureInitCubeArbitrary` |

Then, for **every** texture it loads, unconditionally (`0x812f1172..0x812f11a8`):

| NID | Call | Argument | Meaning |
| --- | --- | --- | --- |
| `0xFA695FD7` | `sceGxmTextureSetMagFilter` | 1 | linear |
| `0x416764E3` | `sceGxmTextureSetMinFilter` | 1 | linear |
| `0x8699ECF4` | `sceGxmTextureSetUAddrModeSafe` | 0 | repeat |
| `0xFA22F6CC` | `sceGxmTextureSetVAddrModeSafe` | 0 | repeat |
| `0x1CA9FE0B` | `sceGxmTextureSetMipFilter` | `0x200` | mipmaps enabled (linear between levels) |
| `0xB65EE6F7` | `sceGxmTextureSetLodBias` | `0x13` | raw 19 |

`sceGxmTextureSetLodBias` has **one call site in the binary**: this one. Raw 19 reads as
`(19 - 31) / 8` = **-1.5 levels**, the decode Vita3K applies (read for behaviour). No call sets an
anisotropic filter.

Two independent sources agree on the value:

1. the static call above;
2. the live `SceGxmTexture` of the front-end canvas atlas, which carries `lod_bias` raw 19 in all 606
   copies read from Vita3K's RAM ([canvas3d-lattice.md](canvas3d-lattice.md), "The atlas
   descriptor decoded").

The other `sceGxmTextureSetMinFilter`/`SetMipFilter` callers checked (`FUN_810242e8`) are render
targets with the mip filter off. They never call `SetLodBias`, so they sample at bias 0.

## What this says, and what it does not

- **Read.** Every `.gxt` the game loads through this loader, race materials included, samples
  trilinear over the file's own mip chain with a -1.5 bias. Confidence **80**: one call site,
  confirmed live on one texture, not on a race material's descriptor.
- **Not read.** Whether a draw rebinds a material texture through a different `SceGxmTexture` copy
  with other sampler bits. The bind path from a material to its descriptor was not followed.

## Ours

2048's `Looks::material_lod_bias` is `Rule::UNREAD`, so ours samples 2048's track materials at bias 0
with the player's anisotropy (16x by default). **Checked, differs.** The same `material_lod_bias`
mechanism HD uses (`oag_title::MATERIAL_LOD_BIAS = -1.5`, `docs/rendering/hd-track-tone.md`) would
carry 2048's value unchanged.

## Omega (the reverse check)

**Checked, differs.** Omega's native port logs the game's own S# per sampler
([omega-status.md](../../../formats/omega-status.md), "The port's anisotropy"):

- `AnisoLinear` 16x with mip `Linear` on 5 to 8 of each boot's 16 to 19 samplers;
- LOD bias 0.0, and +1.0 on 1 or 2 of them;
- no negative bias anywhere.

So Omega's PS4 build does not carry 2048's -1.5. Its sharpness comes from 16x anisotropy instead.

## Reproducing

```sh
ghidra: decompile 0x812f0ec4 in /2048/eboot-vita-2048-eu-v104.elf
ghidra: xrefs to 0x813f2630 (the SetLodBias import stub)   # one call, 0x812f11a4
```
