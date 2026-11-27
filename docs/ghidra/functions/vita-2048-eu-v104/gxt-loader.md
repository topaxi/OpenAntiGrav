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

`GxtLoader_LoadTexture` has no direct caller: its address sits in a function-pointer table at
`0x81517624`, so the resource system reaches it. `SetMinFilter` and `SetMipFilter` have about twenty
other callers, which never call `SetLodBias`. Two were read:

- **`FUN_810242e8`**: render targets, mip filter off.
- **`Environment_LoadHDFuryContent`**: the same, plus the **Zone and Detonator track skins**. Fifteen
  descriptors (`DAT_815d2d0c`, allocated under `Game/FunkLayer.LIVE/FunkLayer.zone.cpp`; the
  `data/Tex/zoneModeTrackN.gxt` and `DetonatorModeTrackN.gxt` strings) are set to min filter linear
  and **mip filter 0 (off)** after loading. Those skins sample level 0 at every distance, whatever
  bias the loader left.

`FUN_81043b1c` (nine call pairs) and the rest were not read.

## In a race: the bound descriptors (2026-11-27, `v2048-texlevel-measure`)

The claim under test: race track, scenery and craft textures are sampled with the loader's state. It would
be false for a texture whose live `SceGxmTexture` had a bias other than raw 19, mip filter off, or a min or
mag filter other than linear. **It holds on the race read, confidence 90.**

**Method.** Vita3K 0.2.1 (`885f9168d`), the EU v1.04 game, Single Player Campaign, Queens Mall (offensive
weapons only). Guest memory was dumped under `SIGSTOP` and read twice over
([vita3k-capture.md](../../../reverse-engineering/vita3k-capture.md), "Which textures a frame binds"):

1. **Bound state**: Vita3K's queued `SetState`/`Texture` commands, which carry the 16-byte descriptor each
   bind used. Each descriptor's texel address was matched to a `.gxt` on the disc by the texels themselves.
2. **Resident state**: every descriptor in memory pointing at a matched `.gxt`'s first texel.

**Bound, one boot, two frames** (grid at 0:10, mid-lap at 0:25), 269 distinct disc textures over both:

| Class | Textures | Binds, 0:10 | Binds, 0:25 | Bias raw, min, mag, mip filter |
| --- | ---: | ---: | ---: | --- |
| track and scenery | 244 | 4,647 | 5,626 | 19, 1, 1, 1 on every bind |
| craft | 14 | 521 | 526 | 19, 1, 1, 1 on every bind |
| other disc textures | 11 | 176 | 109 | 19, 1, 1, 1 on every bind |

- **None differs.** Lightmaps (`lmaps/*-lmap.gxt`) and pad textures (`weaponpad_norm`) are in the set.
- Minimum level of detail 0 on all, and the mip count is the file's level count minus one. One race texture
  has a single level (`m_lightstripv01_e`), where the bias has nothing to select.
- Texture units 0 to 3. A descriptor holds one sampler state, so near and far draws of a texture read the same.
- No anisotropy: min and mag read plain linear on every bind, the filter values Vita3K declares (point,
  linear and two mipmap forms) include no anisotropic one, and the loader makes no anisotropy call.

**Bound textures that are not `.gxt` files** follow other states, which also shows the read can see one:

- the circuit's `track.probes` (64x32): bias raw 31 (0.0), linear/linear, **mip filter off**;
- 768x512 targets: raw 31, point, mip filter off;
- small run-time textures: the loader's four values.

**Second boot, resident state only.** The same event wedged mid-load, with the circuit's textures already in
memory. 226 of the 269 were resident, all with the loader's four values at their texel address; so were all
444 environment and ship textures that had a descriptor. A mid-load read cannot see a rewrite made after the
load, so the bound state itself is **seen on one boot**.

**Outside the race**, two front-end textures, `tex/photomode_dots` and `tex/sony_legal_text`, keep bias 19
with the mip filter off.

**Instrument traps.** A search of all memory for descriptors also finds stale renderer command slots (partly
overwritten, so the filters read 0) and stray words that decode as descriptors. Neither is bound state; only
the command read counts.

## What this says, and what it does not

- **Read.** A texture this loader builds samples trilinear over the file's own mip chain with a -1.5
  bias. Confidence **90**: the static call, and every disc texture bound in two race frames.
- **Not measured.** The Zone and Detonator track skins (the static read above says their mip filter is off);
  the HD and Fury campaign circuits; any second circuit. An HD Zone load wedged in the emulator.
- **Inferred.** Other circuits and modes keep the loader's bits, because the one loader builds their
  textures and the race rewrote none of them.

## Ours

2048's `Looks::material_lod_bias` is `Rule::UNREAD`, so ours samples 2048's track materials at bias 0 with
the player's anisotropy (16x by default). **Checked, differs**, ready to wire: the same
`material_lod_bias` mechanism HD uses (`oag_title::MATERIAL_LOD_BIAS = -1.5`,
`docs/rendering/hd-track-tone.md`) carries 2048's value unchanged.

- The shader bias reaches `material_sample` and `material_sample_grad`, which includes lightmaps. Ours reads
  environment probes at an explicit level, so they stay at bias 0, as the original's do.
- `oag_raceplay`'s `load/lod_bias.rs` leaves separate pad and weapon models at 0 on HD's census. 2048's
  pad textures read -1.5, so check which model draws 2048's pads when wiring.
- The original takes no anisotropic taps, so the 2048 compare profile should say what it runs.

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

The race read: [vita3k-capture.md](../../../reverse-engineering/vita3k-capture.md), "Which textures a frame
binds".
