# Environment probes: how Wipeout HD loads `track.probes` and lights a hull from it

Binary: `PS3_GAME/USRDIR/EBOOT.BIN` from `hdfury-ps3-eu-dec.iso`, as
`/hdfury/EBOOT-ps3-hdfury-eu.elf` in Ghidra. The file's layout, the block format and the
measurements that decoded it are on [hd-probes.md](../../../formats/hd-probes.md); this page
is the executable's side, and what the live fragment programs do with the maps.

**Runtime-verified** on RPCS3 (Talon's Junction, Harimau and Assegai; Metropia, Feisar): the
maps a hull binds equal a blend of this file's probes to within 8-bit rounding
([hd-probes.md](../../../formats/hd-probes.md)). The function names below are static readings
(listing and decompile) and carry the scores the confidence rubric gives that.

## The names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x003c4d78` | function | `EnvProbes_LoadForTrack` | 82 |
| `0x003c1120` | function | `EnvProbes_ReadTables` | 85 |
| `0x003c3b30` | function | `EnvProbes_SubmitDecodeJob` | 65 |
| `0x003c4a70` | function | `EnvProbes_SubmitBlendJob` | 60 |

## `EnvProbes_LoadForTrack` (`0x003c4d78`)

Called by `Pvs_LoadForTrack` (`0x003f5550`) once the `.pvs` has loaded
([visibility.md](visibility.md)). It copies the path built from `%s\%strack%s.probes`
(`0x007b3fa0`), opens it by a copy of that path first and by the original name after a
message if that fails, calls `EnvProbes_ReadTables`, and then:

- builds **eight pairs of textures** through `0x005aa3a0`, each pair a `128x64` and a
  `64x32` with the format word `0x500aae4` (the word the live units 5 and 4 carry), the pool
  of per-craft maps. Eight is `MAX_SHIPS`;
- builds the cell tree over the position table the way `Pvs_LoadForTrack` does over the
  `.pvs` cells: for every pair of entries (`count >> 1`) it forms a box (a min/max over the
  two positions) and calls `Pvs_BuildCellTree` (`0x006d1548`), twice (the body is the
  loop duplicated).

An earlier reading of this function ("`.probes` is a visibility structure that happens to
share a function with a probe pool", 75) was wrong: the structure it builds is a lookup over
the probe positions, and the file's bulk is the two image sets.

## `EnvProbes_ReadTables` (`0x003c1120`)

```text
read u32 count             -> *obj
read u32 stride            -> must be 0x10, else print an error and return 0
alloc  count * 16          -> obj+0x04    read: the position table   (f32 x y z z)
alloc  count * (128*64*3/4) = count * 6144 -> obj+0x08   (read second)
alloc  count * (64*32*3/4)  = count * 1536 -> obj+0x0c   (read FIRST)
alloc  count * 24          -> obj+0x10    read: 24 bytes a probe
alloc  count * 4           -> obj+0x14    read
alloc  count * 3           -> obj+0x18    read
```

`0x005fd5f8(w, h)` is `(w * h >> 4) * 12`, the byte size of a `w x h` image at 12 bytes per
4x4 block: `6144` for `128x64` and `1536` for `64x32`. The file order is therefore the
position table, the **64x32 irradiance images**, the **128x64 reflection images**, then the
three small arrays; the sum of the sizes closes the Talon's Junction file exactly (6,660,682
bytes), which is what pinned the layout. Confidence 85.

## The SPU wrappers (`0x003c3b30`, `0x003c4a70`)

Both build an SPU job (`0x00465138`/`0x004651c0`/`0x00465588` and submit through
`0x004688a0` on the job manager at `0x008b7780`), neither does arithmetic on the PPU:

- `0x003c3b30(w, h, src)` takes one input of `w * h * 3 / 4` bytes (`0x005fd5f8`) and one
  output of `w * h * 4`: a probe image **decoded to `RGBA8`**.
- `0x003c4a70(w, h, a, b, c, d)` takes four inputs of `w * h * 4` bytes and one output: the
  four decoded images **blended**, which is the four probes of a bilinear cell.

Their SPU programs were not read, so the block format and the weights come from the data,
not from them (confidence 60-65). Both sit in the probe class's vtable (`0x0088bec0`),
beside the table reader.

## What the hull's `IBL` programs do with the maps

Live microcode, `ps3-fp-live.py` on the draws of a Harimau rest frame
(`diffuse_with_specular_from_alpha_n`, program `0x747101`, `IBL` block 17 of the
`.rcsmaterial`; confidence 85 for the arithmetic, 90 that it is the variant the hull draws):

```text
n    = normalise(interpolated normal)
uv(d) = ( 0.25 + (d.x > 0 ? 0.5 + d.z / 4 : -d.z / 4),  0.5 - 0.5 d.y )       dual paraboloid
ibl   = tex(unit 4, uv(n))                                    64x32  paraboloidIblTex   0x872ed3bd
r     = reflect(-V, n)                                        V = normalised f[TC3], to the eye
refl  = tex(unit 5, uv(r))                                    128x64 paraboloidReflectionTex 0x9edd3243
light = 4 * ibl                         (MUL by the probe tint {4, 4, 4, 0}; .w a desaturation lerp, 0)
        + sunColour * N.L * shadow(units 2, 3) + f[TC1]      f[TC1] the SPU vertex light
out   = albedo.rgb * light
      + albedo.a * sunColour * (N.H)^300                      specular from alpha (existing)
      + 4 * refl * (albedo.a * 0.5)                           the reflection term, 0.5 = 0xab31c2b1
out   = fog(out) / 2
```

- **The probe tint `0x7480de6d` is `(4, 4, 4, 0)`** on Talon's Junction (Harimau) and on
  Metropia (Feisar, a 1,377-draw grid frame with eight craft: the two programs read,
  `0x748081` and `0x749041`, both `MUL H0.xyz, H2, {4, 4, 4, 0}` after the unit-4 fetch; 66
  programs there sample both units, two were read). It has no authored preimage; two circuits
  and two teams agree. **Not a constant**: in a Zone race the live program reads `(0.69804, 0.69804, 0.69804, 1.0)`
  (two frames of Zone mode, `hd-ship-materials.md`, "The Zone-mode hull"), and `.w` lerps the tinted probe towards its
  luma. `Scene_PrepareFrame` publishes a vec4 copied from `object+0x510` (`lvx` at `0x003ab100`, `stvx` to
  `block+0x7c40`, the table slot at `0x003ab5b0`); `(4, 4, 4, 0)` is stored at `object+0x510` at
  `0x003abb10`-`0x003abb28` when the byte at `r17` is 0, and the other path (`0x003ad748`) is unread.
- **The reflectivity `0.5` is material parameter `0xab31c2b1`** of the `.rcsmodel`'s material
  record (Harimau: `[0.5, 0, 0, 0]`), and the exponent `300` is `0x4232e459`; both are read
  from the material, not transcribed (`mesh::rcs::probe_lit`).
- The reflection is added **after** the albedo multiply and **before** the fog, gated by the
  livery's alpha: where the alpha says metal the hull shows the sky, which is the silver
  of the original's yellow panels.
- The unit-4 sampler's address word is `0x60730303` (gamma nibble `7`), but this project
  settled that the RSX multiplies raw bytes ([renderer.md](renderer.md), "Per-texture sRGB/`GAMMA`
  decode: settled negative") and the pixels agree: the probe enters raw, see
  [hd-ship-materials.md](../../../rendering/hd-ship-materials.md), "Finding 5".

## What the producer is not

**Corrected 2026-10-21 (`hd-one-pair-moa`): the same two names are the producer of the other map.** The engine's
`paraboloidReflectionTex` that scenery glass, water and floors read is one 512x256 target at `0xc4065380`, and it is
the circuit's `sky.gtf` resampled to a dual hemisphere map (four cold-boot reads, 0.008-0.014 rms against the cube's
raw bytes); it is written once at load, not in any frame caught, which is why the frame captures below found no draw
into it. The hull's 128x64 maps stay the `.probes` blend described here. See
[hd-track-tone.md](../../../rendering/hd-track-tone.md), "The reflection map is the sky". Confidence for the
producer's *program* stays unread (the law is fitted to the target's bytes, not read off its microcode).

`liveRsxPanXForm_CubemapToDualParaboloide_fp` and `liveRsxPanXForm_CubemapConvolve_fp` (names
in the executable) draw nothing in any frame caught: two Talon's Junction draw captures (601 and 471 draws) name
no probe address as a render target and issue no blit or transfer into one. They
read as the offline bake of the file (confidence 55). The maps change **every frame** under
thrust (distinct hashes at 17-40 ms) because the SPU blend moves with the craft, which is what
the earlier lane read as a runtime render.
