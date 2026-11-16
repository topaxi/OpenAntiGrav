# The panel-map road: a ray march into a distance field, and a baked sky cube

2026-11-16, `panelmap-re`. Omega (PS4, EU). The road family
`road_alphaspec_normal_emissive_panelmap1` and `..._panelmap1_mask`
(`Data/environments2048/shared/objectlibrary/materials/`), the track surface of Queens Mall,
Sol, Altima and Unity Square in the WipEout 2048 mode, with smaller patches on Arena, Empire
Climb, Subway and Metro Park.

**Evidence kinds.** The shader law is read from the GCN microcode with `oag_rcs::gcn`
(`cargo run -p oag-rcs --example gcn_dump -- <data0N.psarc> <material> --at 0xOFFSET`,
[gcn.md](../../../formats/gcn.md)); offsets below are instruction offsets inside the named
program of the 1.07 patch's file (`data08.psarc`) unless said otherwise. The CPU side is read
in the 1.07 patch executable (Ghidra program `/omega/eboot-ps4-omega-eu-v107.bin`), with the
base address found by byte search of the base `eboot.bin`
([patch-v107-addresses.md](patch-v107-addresses.md)); the base program was not open. **No live
run**: nothing here was watched on the native port or an emulator, so no score passes 94 and
decompile-only claims stop at 84 unless shipped data closes an invariant.

**In one paragraph.** There is no vertex displacement. The pixel program marches the view ray
into a signed distance field (`Cg_PanelDisplacement_1In1panel_map`, an `R16F` texture) to find
where it meets a recessed channel's sloped wall or floor, and samples `DiffuseMap`, `NormalMap`
and `EmissiveMap` at the shifted coordinate. Lighting is the lightmapped "nova" law every Omega
circuit surface has ([lightmap-prelit.md](lightmap-prelit.md)), plus a sun specular lobe and a
fresnel-weighted reflection of a **baked, pre-blurred sky cube** (`skyCube.gnf` beside each
circuit), both gated by the diffuse map's alpha. The sky sheen the pairs show is that last
term: on Sol it is worth up to about 2.3 times the cube's colour at grazing angles.

## The programs

Each material file carries 86 programs (`--summary`); the base (`data00`), 1.05 (`data05`) and
1.07 (`data08`) compiles differ in instruction counts and not in constants or structure
(opcode histograms and literal lists compared for the lit program: the 1.07 one adds only the
velocity export's `0.0625` clamp). Pixel programs that carry the panel march, 1.07 offsets:

| Program | Instr. | Lighting | Read here |
| --- | --- | --- | --- |
| `0x2fe4` | 441 (428 in base) | `constantAmbientColour` only, no normal map | by hand, whole |
| `0x4a04`, `0x6764` | 613, 620 | constant ambient + sun, cube reflection | summary only |
| `0xa324` | 675 | SH probe (`shCoeffs`), `cubeDistanceTex` | summary only |
| `0xc264`, `0x12244` | 642, 651 | sun + shadow light, no lightmap | summary only |
| **`0xe144`** | 765 | nova lightmap + `shadowMap` + sun + shadow light + cube | **by hand, whole** |
| `0x10244` | 650 | as `0xe144` without `shadowMap` | constants compared |
| `0x14284`, `0x164e4` | 770, 658 | as the two above, three more interpolant lanes | constants compared |

Every panel submesh on the eight circuits but one (128 triangles on Unity Square) binds a
lightmap (`psp2_material_dump`), so the nova programs are the ones that draw. Which of
`0xe144`/`0x10244`/`0x14284`/`0x164e4` a draw selects (shadow map on or off, the three extra
lanes) is **unread**; they share every literal.

The vertex program of `0xe144` (`0xd8e4`, 119 instructions) exports, in order: the previous
frame's clip position with the fog factor `min(1, exp(-(fogColour.w * w_clip)^2))` in `.w`;
`uv1`; `lightmapUV`; the bitangent `cross(N, T) * tangent.w * worldScaleInv`; the eye position;
`eye - P`; the world normal; the world position; the world tangent; and a shadow-space
position. It moves no vertex. **Confidence 88** (decode unambiguous, one program).

## The law

Names: `uv` the interpolated `uv1`; `T`, `B`, `N` the interpolated world tangent, bitangent
and normal; `V = normalize(eye - P)`; `Depth`, `SlopeWidth`, `Aspect` the three
`Cg_PanelDisplacement_1In1*` constants; `df(x)` the panel map's one channel.

```text
; --- ray setup (0xe144 @0x24-0x1a4; 0x2fe4 @0x1c-0x1bc)
a = -dot(V, normalize(T))            ; view ray along u, pointing away from the eye
b = -dot(V, normalize(B))            ; along v
c = sqrt(1 - a*a - b*b)              ; into the surface; rsq with no clamp
D = 0.01 * Depth                     ; channel depth, in v units (texture heights)
w = 0.01 * SlopeWidth                ; horizontal run of the sloped wall, same units
step = (a * D / c * min(1, 1/Aspect),  b * D / c * min(1, Aspect))   ; uv travelled at the floor
k = 1 / sqrt(1 + (SlopeWidth/Depth)^2)      ; horizontal distance -> distance to the slanted wall

; --- sphere trace, t in [0, 1] is the fraction of D descended
d(t, x) = df(x) + w * (1 - t)        ; solid where d <= 0: df <= -w at the top, df <= 0 at the floor
t = 0
repeat 10 times:                     ; 10 panel-map samples, the first at uv itself
    dist = k * d(t, uv + step * t)
    if dist <= 0.0001: hit, stop     ; t stays where the sample was taken (+ (c/D) * sat(dist))
    t = t + (c / D) * (dist + 0.0001)
    if t >= 1: stop                  ; reached the floor
t = sat(t)
uv' = uv + step * t

; --- lip blend (0xe144 @0x6f0-0x7b0); see "The lip blend is inert" below
e = 0;  uv_e = uv'
if t > 0:
    px = min(|ddx(uv)|, |ddy(uv)|)                    ; lengths of the two screen derivatives
    d0 = df(uv) + w
    if 0 <= d0 < 3.3333 * px and df(uv + 0.01 * step) >= d0:
        uv_e = uv - 2 * normalize(step) * d0
        e    = sat(1 - 0.3 * d0 / px)
tex(X) = mix(X(uv'), X(uv_e), e)     ; each sampled with ddx(uv), ddy(uv) as explicit gradients

; --- surface terms
Dm = tex(DiffuseMap)   (rgba)        E = tex(EmissiveMap).rgb        n = 2 * tex(NormalMap).rgb - 1
Nw = normalize(T * n.x + B * n.y + N * n.z)                  ; len^2 floored at 1e-4
N2 = T * n.x + (B + cross(N, T)) * n.y + N * n.z             ; the fresnel's normal, see below
R  = 2 * dot(Nw, V) * Nw - V
LM = lightmap(lightmapUV)  (rgba, raw UNORM)
S  = LM.a * shadow                                           ; shadow = the shadowMap term, 1 without it
prelit = nova.x * LM.rgb ^ nova.z + nova.y
Fsl    = 0.75 + 0.25 * sat(dot(Nw, shadowLightDirection))
              + shadowLightSpecularDiffuse.y * sat(dot(R, shadowLightDirection)) ^ shadowLightSpecularDiffuse.x
light  = liveLighting0diffuse * S * sat(dot(Nw, liveLighting0direction)) + prelit * Fsl

; --- specular and reflection, both scaled by the diffuse map's alpha
beta = reflectiveBlurCoeffs.z + Blurrines * reflectiveBlurCoeffs.w          ; = Blurrines
p    = SpecPower * (1 - 0.75 * min(1, beta))
spec = liveLighting0specular * SpecStrength * Dm.a * S
       * sat(dot(R, liveLighting0direction)) ^ p
       * 0.06981317 / max(0.06981317, 0.17453294 * min(1, beta))
fres = ReflectivityMin + ReflectivityScale * sat(1 - dot(N2, V) / |N2|) ^ ReflectivityPower
lod  = reflectiveBlurCoeffs.y * beta                                        ; = 2 * Blurrines
cube = mix(skyCube[floor(lod)], skyCube[floor(lod) + 1], fract(lod))  at direction (R.x, R.y, -R.z)
refl = ReflectivityAmount * Dm.a * fres * cube

rgb  = Dm.rgb * light + E + spec + refl          ; `_mask`: + (spec + refl) * ReflectionMask(uv).rgb
out  = mix(fogColour.rgb, rgb, fog)              ; fp16, alpha 0; mrt1/mrt2 are depth and velocity
```

The unlit program `0x2fe4` runs the same march and lip blend and ends
`rgb = Dm.rgb * constantAmbientColour + E` (`@0x870-0x878`).

### Term by term

| Term | Where (`0xe144`) | Confidence | Why not higher |
| --- | --- | --- | --- |
| Ray setup, `0.01` scales, `Aspect` on `u` | `@0x58-0x198`; `0.01` at `@0x130` | **88** | Two hand-read programs agree; **the data closes it twice**: `Aspect` is authored `4.0` on the 1024x256 map and `8.0` on the 1024x128 one, and the map's gradient is exactly one unit per texture height (below), the units the `Aspect` scaling assumes. No live run. |
| Sphere trace: 10 samples, `0.0001` epsilon, `t >= 1` exit, hit keeps the sample's `t` | `@0x1a8-0x674` | **88** | Same in `0x2fe4` (`@0x19c-0x604`) and the `_mask` file. |
| What is recessed: the `df > 0` strips, walls `w` wide | from `d(t)` | **85** | Reading of the formula; agrees with the original's Sol frame (the diagonal strips sit below the plates with a lit edge). |
| Lip blend formula | `@0x6f0-0x7b0` | **85** | |
| Lip blend never fires in practice | arithmetic below | **75** | Not measured on a frame. |
| Texture samples take the *unshifted* `uv` gradients | `image_sample_cd` `@0x988`, `@0xcbc-0xcd4`, operands `v5..v8`/`v11..v14` | **88** | |
| Normal map `2x - 1`, `T`/`B`/`N` basis, no `w` use in the pixel program | `@0xa34-0xaac` | **88** | Matches [gcn.md](../../../formats/gcn.md)'s tangent reading. |
| Nova prelit, `LM.a` sun mask, shadow-light factor `0.75 + 0.25 ...` | `@0xbd0-0xc20`, `@0xc90-0xe3c` | **88** | The law [lightmap-prelit.md](lightmap-prelit.md) read on another family; here the `0.75`/`0.25` form is present. |
| Specular lobe: `R.L`, exponent `SpecPower * (1 - 0.75 * min(1, beta))`, the 4 degree over 10 degree normalisation | `@0xbec-0xc08`, `@0xcfc-0xd30`, `@0xe04` | **85** | `0.25` comes from `v_cvt_off_f32_i4 v35, 4` (`@0xb54`), read as `4/16`. |
| Fresnel `Min + Scale * (1 - N2.V)^Power` | `@0xb18-0xb58`, `@0xc5c`, `@0xe20-0xe24` | **88** | Same shape as `FrontEndConstantFranelBlend` ([omega-status.md](../../../formats/omega-status.md)). |
| `N2` adds `cross(N, T)` to the bitangent | `@0x9c0`, `@0xa7c`, `@0xab0-0xad0` | **80** | Read literally; it looks like the source built the basis twice. With `B = cross(N, T) * w / scale` it doubles (or cancels) the `n.y` lane, so the fresnel's normal is not `Nw`. |
| Cube lookup: `-R.z`, two array layers mixed by `fract(lod)` | `v_cubeid/sc/tc/ma` `@0xc40-0xd3c`, layers `v33 * 8 + face` `@0xdf4`, `@0xe8c` | **88** | The `* 8` is the hardware's cube-array face encoding. |
| Diffuse alpha gates specular and reflection | `@0xde4`, `@0xe40` | **88** | The "alphaspec" of the name. |
| `_mask`: `(spec + refl) * ReflectionMask(uv).rgb`, mask at the unshifted `uv` | `_mask` program `0xe2c4` `@0xe8c-0xefc` | **85** | One program read. |
| Fog and fp16 export | `@0xeb4-0xf18` | **88** | |
| `shadow`: a cascaded exponential shadow map, `min(1, sample * exp(clamp(c12 + c13 * z, c14, c15)))` with `shadowCoeffs` | `@0x7b8-0x964`, `@0xb84-0xbac` | **60** | Structure only; cascade choice not taken apart. Not this family's own. |

### The lip blend is inert

The inner test compares the raw sample `df(uv + 0.01 * step)` with `d0 = df(uv) + w`
(`v_cmpx_ge_f32 vcc, v19, v22` `@0x764`, `v22` having had `w` added at `@0x1a4`). The field's
gradient is 1, so the sample can exceed `df(uv)` by at most `0.01 * |step|`; passing needs
`0.01 * D * tan(theta) >= w`, that is `tan(theta) >= SlopeWidth / (0.01 * Depth)`, about 51
at the authored `1.8 / 3.5` (a view within 1.1 degrees of the surface). So `e = 0` and
`tex(X) = X(uv')` everywhere a player looks. An implementation may leave the blend out and say
so; the second sample of each texture is then never visible.

## The panel map

`Track_A_Floor_panel_df.gnf` (1024x256) and `Track_A_Floor_RunOff_panel_df.gnf` (1024x128),
`Data/environments2048/shared/objectlibrary/Textures/`. GNF surface format `0x02` (one 16-bit
channel), channel type `7` (float), one level, `TileMode(13)`.

- **Storage: confidence 92.** Half floats, little-endian, in 8x8-texel micro tiles laid
  row-major, the texel index inside a tile being the Morton order `x0 y0 x1 y1 x2 y2` (the
  order the BC7 path already uses for blocks, [gnf.md](../../../formats/gnf.md)). Chosen among
  all 720 bit orders by total variation: this one gives a field whose largest step between
  neighbours is `0.0056`, and the next best is 11 to 13 % rougher; row-major reads as pairs of
  repeated values.
- **Content: confidence 92.** A signed distance field in **texture-height units**: the
  central-difference gradient magnitude is `1/256` per texel on the first (median exactly
  `0.00390625`, 5th to 95th percentile `0.00381` to `0.00394`) and `1/128` on the second. The
  field wraps in `u` exactly and in `v` to one texel's step. Range `-0.345` to `+0.078`
  (road) and `-0.367` to `+0.202` (run-off); positive over 14.0 % and 17.5 % of the area, in
  diagonal chevron strips. With the law above those strips are the recessed channels.
- **What decoding takes.** `ModelTexture::from_gnf_form` refuses it because
  `oag_texture::gnf` names no 16-bit format. It needs: surface format `0x02` with channel type
  `7` recognised, the same micro-tile walk at 2 bytes a texel, and an `R16Float` upload with
  linear filtering and repeat addressing (the field must be sampled as floats; an 8-bit copy
  would quantise a range of 0.42 to steps as large as the wall width's tenth). The sampler
  state the game binds was **not read**.

## Authored values

Material parameter hashes are `!crc32(name)` (`oag_rcs::rcsmaterial::name_hash`):
`Cg_PanelDisplacement_1In1Depth` `0x95de1208`, `...SlopeWidth` `0x4e231706`, `...Aspect`
`0x185b0018`, `ReflectivityScale` `0x91643095`, `ReflectivityPower` `0xd6a814b1`,
`ReflectivityMin` `0x20ea500d`, `ReflectivityAmount` `0x366e9d28`, `SpecStrength`
`0x2920cd2d`, `SpecPower` `0x05fec07d`, `Blurrines` `0xc09f740a`; the panel map's sampler is
`0x4736b3bf`. Read from each circuit's `track.final.rcsmodel` with `psp2_material_dump`
(**confidence 90**, the values are the files'):

| Circuit (`environments2048/`) | Variant, textures | Triangles | Depth, SlopeWidth, Aspect | Refl. Amount, Min, Scale, Power | SpecStrength, SpecPower | Blurrines |
| --- | --- | --- | --- | --- | --- | --- |
| `mall` (Queens Mall) | road | 1,686 | 3.5, 1.8, 4 | 2.0, 0.6, 0.7, 5 | 1, 31.56 | 0.2 |
| `sol` | road | 6,749 | 3.5, 1.8, 4 | 0.5, 0.6, 10, 4 | 5, 100 | 0.3 |
| `altima` | road | 5,743 | 3.5, 1.8, 4 | 1, 0.4, 0.7, 5 | 2, 31.56 | 0.5 |
| `altima` | run-off | 4,437 | 3.5, 1.8, 8 | 0, 0.4, 0.7, 5 | 0, 31.56 | 0.5 |
| `altima` | run-off `_mask` | 3,866 | 3.5, 1.8, 8 | 1, 0.4, 0.7, 5 | 2, 31.56 | 0.5 |
| `square` (Unity Square) | road | 1,948 | 3.5, 1.8, 4 | 1, 0.2, 0.7, 5 | 1, 31.56 | 0.5 |
| `square` | run-off (one unlightmapped, 128 tris) | 994 | 3.5, 1.8, 8 | 0, 0.4, 0.7, 5 | 1, 31.56 | 0.5 |
| `square` | run-off `_mask` | 648 | 5.0, 1.8, 8 | 1, 0.2, 0.7, 5 | 1, 31.56 | 0.5 |
| `arena` | road | 1,140 | 3.5, 1.8, 4 | 1, 0.2, 0.7, 5 | 1, 31.56 | 0.5 |
| `park` (Metro Park) | road | 2,060 | 3.5, 1.8, 4 | 1, 0.4, 0.7, 5 | 1, 31.56 | 0.5 |
| `subway` | road | 1,422 | 3.0, 1.8, 4 | 0.3, 0.2025, 0.7362, 4.601 | 1, 50 | 0.5 |
| `tower` (Empire Climb) | road | 4,213 | 3.0, 1.9, 4 | 1, 1, 1, 1 | 1, 35 | 1.0 |
| `tower` | run-off `_mask` (two emissive sets) | 1,244 | 6.0, 1.8, 8 | 1, 1, 1, 1 | 1, 31.56 | 1.0 |

Road set: `track_a_floor_panel_df`, `track_a_floor_ds_disp` (BC7 sRGB 8192x2048, alpha is the
specular mask), `track_a_floor_e` (512x128), `track_a_floor_disp_n` (BC7 UNORM 8192x2048).
Run-off set: `track_a_floor_runoff_panel_df`, `..._runoff_ds` (8192x1024), `..._runoff_emis`
(64x64; Empire Climb also `..._runoff_mag_f`), `..._runoff_n`, and for `_mask`
`track_a_floor_runoff_mask` (32x1024). So at the authored values a channel is `0.035` texture
heights deep with walls `0.018` wide, and Sol's reflection weight is
`0.5 * Dm.a * (0.6 + 10 * (1 - N.V)^4)`: `0.3` face-on, `2.35` at `N.V = 0.2`.

## The CPU side: the blur constants and the cube

Addresses are the **1.07 patch's**, with the base address beside each where it was found.

| What | Patch (1.07) | Base | Evidence |
| --- | --- | --- | --- |
| Constant name table entry `reflectiveBlurCoeffs` | registrar `FUN_015ef4e0`, entry `+0x1108` (slot 109), hash `0xb898a7c6` | registrar `FUN_015c2dc0` | `!crc32("reflectiveBlurCoeffs") = 0xb898a7c6`; the name pointer stored at `+0x1110` is the string `0x0188dd1a` (`015f2648`, `015f2686`). |
| Its value pointer | `FUN_015f2ad0`: `*(table + 0x1118) = &DAT_020e3420` | `Scene_BindFrameConstants` (`0x015c6120`) references `0x0202f080` at `015c6138` | Entry `+0x10` is the value pointer (the layout [lightmap-prelit.md](lightmap-prelit.md) closed for `novaScaleBiasPower`). |
| `g_reflective_blur_coeffs` | `0x020e3420` | `0x0202f080` | Written once, by the static initialiser `FUN_0166be90` (`0166c09d`; base `016310ae`), from the 16 bytes at `0x01857d40` (base `0x017f97d0`): `78 86 dc 3f, 00 00 00 40, 00 00 00 00, 00 00 80 3f` = **`(1.72285, 2.0, 0.0, 1.0)`**. No other instruction names the address in either build (RIP-relative scan of the whole text segment: two references each, the initialiser and the binder). |
| Constant name table entry `cubeReflectionTex` | entry `+0x5f0` (slot 38), hash `0x693d1163` | | `!crc32("cubeReflectionTex") = 0x693d1163`. |
| Its binding in the main scene pass | `Scene_PrepareFrame` (patch `0x015f2d30`), `015fd9a1`-`015fd9be`: `*(table + 0x600) = **0x020084b8`, version byte `+0x5f6` bumped | `0x01f8f688`, referenced at `015d06a0` and 16 more sites up to `015fafc7` (not disassembled) | A second pass (`FUN_01618bb0`, `0161998a`) binds `*0x020084c0` instead, and `cubeDistanceTex` from `*0x020084c8`. |
| `g_cube_reflection_tex` | `0x020084b8` | `0x01f8f688` | Stored at `0162d399` (base `01601ead`) in the circuit loader `FUN_0162a390`, right after `FUN_013d0cc0(path, "skycube.rcsmodel", ..., "skyCube.gnf")` (strings `0x0188fe4a`, `0x0188fe70`; base `0x0182b987`, `0x0182b9ad`) and the texture load `FUN_017cc0d0`: **the circuit's `skycube.rcsmodel` path with the file name replaced by `skyCube.gnf`**. Released in `FUN_01636440` (`01636b06`). |

Confidence: `g_reflective_blur_coeffs` and its value **88** (the slot is identified by hash
on both ends and the bytes are in the file; a write through the pointer would not show in a
reference scan, and no live read was made). `g_cube_reflection_tex` and the file it holds
**85** (one load site read; the same instruction pattern at the base address).

With `(1.72285, 2, 0, 1)`: `beta = Blurrines`, `lod = 2 * Blurrines`, and lane `.x` is not
read by these programs.

## `skyCube.gnf`

Every circuit directory ships one (`Data/environments2048/<circuit>/skyCube.gnf`,
`Data/environments/<circuit>/skyCube.gnf`; 26 files on the base archives `data00` to
`data04`: 22 of 44,892,416 bytes and the four Zone circuits' of 2,949,376, not read). The four
read (Sol, Queens Mall, Altima, Unity Square) share one header and differ in content: BC7 sRGB, 1024x1024, 11
levels, GNF texture type `0xB` (cube) in word 3 bits 31:28, array slices 0 to 23,
`TileMode(13)`.

- **Layout: confidence 92.** Level-major. Each level stores **32** slices (24 rounded up to
  a power of two), each slice `max(8, w/4)^2` blocks of 16 bytes (levels under 32 texels
  are padded to one micro tile), micro-tiled like any BC7 level:
  `offset(level, slice) = sum over k < level of 32 * size(k) + slice * size(level)`. The sum
  over the 11 levels is 44,892,160, the file's data size to the byte. Slice `6 * c + f` is
  face `f` of cube `c`: decoded at level 2, the 24 slices are four rows of the same six
  faces, each row blurrier than the last.
- **Faces** in slice order read `+X, -X, +Y` (sky), `-Y` (ground), `+Z, -Z`, the usual cube
  order (**80**: by picture).
- **Orientation: confidence 75.** The program looks up `(R.x, R.y, -R.z)` with no sky
  rotation. On Sol the sun's glare is on the `+Z` face and `Lighting.Sun direction` is
  `(-0.105, 0.574, -0.812)`: world `-Z`, which is what the flip predicts. One circuit;
  Queens Mall's cube is an overcast city capture with no sun to check. `Lighting.Sky rotation`
  (`-75` on Sol) is not applied to it.
- **Blur levels.** Cube 0 is sharp; `lod = 2 * Blurrines` picks `0.4` (Queens Mall), `0.6`
  (Sol), `1.0` (most), `2.0` (Empire Climb). Cube 3 is reachable only by the `+ 1` layer at
  `lod = 2`, where its weight is 0.
- Sol's and Altima's cubes are sky and a painted ground; Queens Mall's holds the city's
  buildings, so it is a capture from inside the circuit, not the sky dome alone.

## 2048 against Omega

**Checked, differs.** 2048 (Vita, `PCSF00007` base, 1.04 patch, both DLC packs: 24,434
entries) ships no file whose name carries `panelmap`, `panel_df` or `PanelDisplacement`. Its
road is `fc16_phong_AlphaSpec_Normal_Emissive` over `Track_A_Floor_DS`/`_N`/`_E`, with no
`_disp` textures: the displaced road is a PS4 addition. The Vita program's reflection and
specular terms were not read (GXP operands, [gxp.md](../../../formats/gxp.md)), so whether
2048's own road has an alpha-gated sheen is open and not answered by this page.

Omega's HD-mode circuits carry sibling families this page did not read:
`Diffuse_Normal_Specular_PanelMap`, `..._PanelMap2In`, `D_S_N_CustomR_PanelMap(2In)` (Vineta
K) and `Diffuse_Normal_Specular_panelmap2` (Sol 2).

## Open

- Which of the four nova programs a draw selects, and the three extra interpolant lanes of
  `0x14284`/`0x164e4`.
- The panel map's and the cube's sampler states (filter, addressing, anisotropy).
- A live read of `g_reflective_blur_coeffs` on the native port (with the image mapped at
  `0x800000000` the runtime address is the patch address plus `0x7ff000000`; expect `(1.72285, 2, 0, 1)`), which would lift it to 94.
- The cube's orientation on a second circuit with a visible sun.
- `N2`'s doubled bitangent, checked against a frame: it changes the fresnel only where the
  normal map's green lane is far from 0.5.

## Names

- `0x0202f080` `g_reflective_blur_coeffs` - 88. Data; the four floats bound as the shader
  constant `reflectiveBlurCoeffs` (patch `0x020e3420`).
- `0x01f8f688` `g_cube_reflection_tex` - 85. Data; the handle of the circuit's `skyCube.gnf`,
  bound as `cubeReflectionTex` (patch `0x020084b8`).

`FUN_0162a390` (patch; the circuit loader that opens `skyCube.gnf`) and `FUN_015f2ad0`
(patch; the binder, base `Scene_BindFrameConstants`) are not renamed here: the first was read
at one site only (hypothesis: the circuit load, confidence 45), the second already has its
base name.
