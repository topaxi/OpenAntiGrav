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
([patch-v107-addresses.md](patch-v107-addresses.md)); the base program was not open. The page
was written with **no live run**, so its scores stopped at 88 unless shipped data closed an
invariant; since 2026-11-16 the reflection's terms are **knocked out live on the native port**
("Live", below), and the scores that run carries say so.

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
| Fresnel `Min + Scale * (1 - N2.V)^Power` | `@0xb18-0xb58`, `@0xc5c`, `@0xe20-0xe24` | **90** | Same shape as `FrontEndConstantFranelBlend` ([omega-status.md](../../../formats/omega-status.md)). Live: with `Scale = 0, Min = 1` the sheen falls to the flat `Amount * Dm.a * cube`, and ours' full sheen over the mapped normal lands within x0.8 to x1.2 of the original's in seven boxes ("Live"). `Scale = 0` alone gives 0.41 to 0.77 of the flat state where `Min = 0.6` predicts 0.6 (one frame each, moving crowd and hull), so the two constants' roles are confirmed and their exact values are not. |
| `N2` adds `cross(N, T)` to the bitangent | `@0x9c0`, `@0xa7c`, `@0xab0-0xad0` | **80** | Read literally; it looks like the source built the basis twice. With `B = cross(N, T) * w / scale` it doubles (or cancels) the `n.y` lane, so the fresnel's normal is not `Nw`. |
| Cube lookup: `-R.z`, two array layers mixed by `fract(lod)` | `v_cubeid/sc/tc/ma` `@0xc40-0xd3c`, layers `v33 * 8 + face` `@0xdf4`, `@0xe8c` | **92** | The `* 8` is the hardware's cube-array face encoding. Live: the road as a mirror (`Amount = 2, Min = 1, Scale = 0, Blurrines = 0`) on the original against ours' map built with this lookup agrees within x0.8 to x1.13 per channel in seven boxes of Sol's grid ("Live"), which a wrong flip, face order or level would not. One circuit. |
| Diffuse alpha gates specular and reflection | `@0xde0-0xde4` (`v20`, the fourth lane of the `DiffuseMap` sample at `@0xcbc`/`@0xcd4`, times `ReflectivityAmount`), `@0xe40` | **94** | The "alphaspec" of the name. Live on two circuits: the mirror state shows the map's alpha as a mottle on the road, and only this lane (mean 0.195 over `track_a_floor_ds_disp`) reproduces the original's sheen ("Live"). Runtime trace on one binary. |
| The sheen has no other weight: `refl = Amount * Dm.a * fres * cube`, added after the lighting | `@0xe34` (`v11 = v5 * v2`), `@0xeb0`, `@0xec4`, `@0xec8` (`+= v11 * cube`) | **92** | Re-read instruction by instruction for a lightmap, shadow or `Fsl` factor: none. Live: `Amount = 0` leaves the lit base, and the sheen scales x4.4 to x4.6 for `Amount` x4 with the blur also changed ("Live"). |
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

Confidence: `g_reflective_blur_coeffs` and its value **94** (the slot is identified by hash
on both ends, the bytes are in the file, and the value was read live on the 1.07 build, both at
the address and through the name table's pointer: "Live"; 88 before that read). `g_cube_reflection_tex` and the file it holds
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

## Live: the reflection's terms knocked out on the native port

2026-11-16, `sheen-re`. `scripts/wipeout-pc-road-sheen-live.py` on the `wipeout_pc` native port
(1.07 eboot, commit `84690ed896ea`, its own headless sway, 1920x1080), Sol in the 2048 mode at
A class (the `omega/sol-2048` pair's walk); Queens Mall, one boot. The script finds
the road material's values in the guest through the model's own parameter records (the name hash
`!crc32(name)`, a count of 1, and at `+0x10` a pointer to the float; Sol has eight such
materials, their ten values neighbours in the order `Blurrines, Aspect, Depth, SlopeWidth,
ReflectivityAmount, ReflectivityMin, ReflectivityPower, ReflectivityScale, SpecPower,
SpecStrength`), writes them **while the race intro loops**, and starts the race, so the grid frame
is the pair's own pose with the change in. Later states are written at the settled camera, about
14 s after the grid.

**What does not work** (three boots lost to it): the copies of the program's constant block
(`__GLOBAL_CB__`, found by its first eight floats) are per-frame staging copies, in the guest and
in the port's own caches. Writing them changes no frame, and writing an old value back into one
that has been reused corrupts the heap (`double free or corruption`). The frame constants
(`novaScaleBiasPower`, the two lights) are reached through the constant name table (entry: hash,
then the value pointer at `+0x10`) and are refilled every frame, so a write there does not hold.

**Frame constants read live** (Sol; from a staged block and through the name table, the two
agreeing). Confidence **94** each as a value of this circuit (runtime read, one binary):

| Constant | Sol | Queens Mall |
| --- | --- | --- |
| `reflectiveBlurCoeffs` | `(1.72285, 2, 0, 1)` at runtime `0x8010e3420` (patch `0x020e3420`) | same |
| `novaScaleBiasPower` | `(1, 0, 1)` | `(2, 0, 2.5)` |
| `liveLighting0diffuse` | `(7.608, 5.352, 3.72)` | `(9.625, 4.46, 2.38)` |
| `liveLighting0specular` | `(6.096, 3.2627, 1.6)` | `(9, 6.444, 4.3765)` |
| `liveLighting0direction` | `(-0.1053, 0.5740, -0.8120)` | `(0.2984, 0.4506, 0.8414)` |
| `shadowLightSpecularDiffuse` | `(10, 1, 0, 0)` | `(10, 1, 0, 0)` |
| `shadowLightDirection` | `(0.7908, -0.3750, -0.4838)` | `(-0.3068, 0.4453, -0.8412)` |
| `fogColour` | `(0.364, 0.5, 0.604, 0.000109)` at the grid | not read (no staged block found in the intro) |
| `fadeResCoeffs` | `(40000, 0, 1/1920, 1/1080)` | not read |

So on Sol the shadow light points below the horizon (`y = -0.375`): `sat(dot(Nw, shadowLightDirection))`
is 0 on a level road and `Fsl = 0.75 + sat(dot(R, shadowLightDirection))^10`, a prelit-coloured
lobe toward `(0.79, -0.38, -0.48)`, weight 1.

**Which program draws.** Every staged block of the road on Sol's grid (8 of 8, counted by where
`Blurrines, SpecPower` sit) has the shadow-map layout: `shadowCoeffs` in dwords 12 to 27 (filled:
`0, 1, 0, 1, 0, 1, 0, 1, 37.7, 0.4167, 108.5, 0.2083, -40.8, -799.2, -80, 0` after the first
four) and `Blurrines` at dword 52. The variants without `shadowMap` (`0x10244`, `0x164e4`) put it
at dword 36 and none is staged. So the race binds **`0xe144` or `0x14284`**, confidence **90**;
the two share the block's layout and every literal and are not told apart here (they differ in
three interpolant lanes).

**The knockouts**, Sol, grid frame at the pair's pose, mean of a box in linear light (sRGB
decoded, x100) less the same box with `ReflectivityAmount = 0`. `ours` is main's shader;
`ours, alpha` is a scratch build whose gate is the diffuse sample's alpha (below). Boxes at
1920x1080 as `(y0, y1, x0, x1)`: near left `(780, 920, 250, 500)`, near right
`(780, 920, 1400, 1700)`, mid left `(600, 680, 200, 500)`, mid right `(620, 680, 1150, 1350)`,
mid centre `(520, 560, 600, 900)`, far centre `(445, 470, 700, 1000)`, far left
`(470, 500, 300, 600)`.

| Box | Original's sheen | Ours | Ours / original | Ours, alpha | Ours, alpha / original |
| --- | --- | --- | --- | --- | --- |
| near left | (4.6, 5.5, 6.6) | (7.7, 9.8, 12.3) | 1.69, 1.80, 1.85 | (4.6, 5.6, 7.0) | 1.00, 1.03, 1.05 |
| near right | (1.9, 3.3, 4.9) | (2.7, 4.9, 7.5) | 1.42, 1.47, 1.54 | (1.5, 2.8, 4.3) | 0.78, 0.83, 0.87 |
| mid left | (4.0, 5.8, 7.5) | (8.2, 13.3, 17.5) | 2.06, 2.31, 2.34 | (4.4, 6.9, 9.0) | 1.10, 1.20, 1.21 |
| mid right | (4.2, 6.2, 7.5) | (7.3, 11.2, 13.9) | 1.74, 1.81, 1.85 | (4.2, 6.3, 7.7) | 0.99, 1.01, 1.02 |
| mid centre | (4.8, 6.6, 8.4) | (7.3, 12.7, 17.0) | 1.52, 1.92, 2.03 | (4.1, 6.9, 9.3) | 0.85, 1.05, 1.11 |
| far centre | (10.3, 14.4, 17.6) | (19.3, 31.6, 38.5) | 1.87, 2.19, 2.19 | (10.9, 17.5, 21.4) | 1.06, 1.21, 1.22 |
| far left | (11.3, 15.3, 18.6) | (16.1, 25.1, 30.7) | 1.43, 1.64, 1.65 | (9.2, 14.0, 17.2) | 0.81, 0.91, 0.92 |

The mirror state (`Amount = 2, Min = 1, Scale = 0, Blurrines = 0`, so `2 * Dm.a * cube[0]`), same
boxes, original then ours with the alpha gate, then the ratio: near left (13.2, 16.2, 20.2),
(12.3, 15.3, 19.3), 0.93 to 0.96; near right (4.5, 7.9, 11.8), (3.5, 7.3, 12.2), 0.79 to 1.04;
mid left (3.7, 6.7, 9.0), (4.2, 7.0, 9.3), 1.03 to 1.12; mid right (5.0, 7.9, 9.7),
(5.4, 8.3, 10.2), 1.04 to 1.08; mid centre (4.8, 7.2, 9.4), (4.0, 6.9, 9.5), 0.84 to 1.00; far
centre (5.2, 7.6, 9.5), (5.2, 8.6, 10.7), 1.00 to 1.13; far left (7.3, 11.6, 14.7),
(5.9, 9.5, 12.1), 0.81 to 0.83. Main's shader reads 1.5 to 2.0 times the original there. By eye
the original's mirror road is mottled (the alpha map) and ours' on main is an even pale blue.

The lit base under the sheen, the original's `Amount = 0` frame against ours' (display values,
% of full scale): near left (44.3, 25.4, 23.4) and, a second boot, (41.0, 22.7, 21.1), ours
(41.1, 23.5, 23.6); mid right (30.3, 20.5, 21.4), ours within 1; the left and far boxes are
1.2 to 2.3 times ours in linear red (far left (43.6, 29.0, 28.0) against ours' (29.7, 22.9, 24.5)),
which is the size and place of the `Fsl` lobe ours does not draw. Boot to boot the original's
boxes move by up to 3.

At the settled camera (one boot, near box `(780, 920, 250, 500)`, linear x100 less the
`Amount = 0` state): authored (4.8, 5.7, 6.8); `Scale = 0, Min = 1` (2.6, 3.4, 4.1); the mirror
(11.8, 15.2, 18.2), x4.4 to x4.6 of the flat state for `Amount` x4 with the blur taken off;
`Scale = 0` (1.1, 1.8, 2.3); `Blurrines = 0` (4.0, 5.7, 7.0); `SpecStrength = 0`
(3.7, 5.5, 6.9), the sun lobe adding nothing visible; restored (4.5, 5.9, 7.2). The output is
linear in the sheen's weight, so the frame is tone mapped by nothing stronger than the sRGB
curve over this range.

Queens Mall, one boot with `Amount = 0` against the pair's own frame (another boot), boxes on the
panel road: near right `(820, 940, 1300, 1650)` original (8.0, 8.3, 5.3), ours (14.5, 14.7, 9.9),
ours with the alpha gate (9.1, 8.8, 5.8), x1.06 to x1.14; mid right `(640, 720, 1200, 1500)`
(2.8, 3.7, 2.4), (7.5, 7.2, 4.5), (4.7, 4.4, 2.7); mid centre `(560, 600, 700, 1000)`
(5.9, 5.9, 4.0), (12.5, 11.4, 5.9), (7.3, 6.4, 3.2); far centre `(480, 510, 800, 1050)`
(4.0, 5.2, 3.9), (9.9, 9.7, 7.3), (5.7, 5.5, 4.0). The original's base there is 10 to 35 %
brighter than ours (near right (37.3, 15.2, 7.4) against (31.5, 12.6, 6.0) linear x100).

**What it settles.** The sheen ours drew at 1.4 to 2.3 times the original's, bluer, was **the
gate**: ours multiplied by the diffuse map's **red** lane where the program multiplies by its
**alpha**. `shade.wesl`'s `texel.a` is `coverage[(slots >> 3) & 3]`, and a PS4 lightmapped
material's slot word is `SECOND_IS_LIGHTMAP` alone (`mesh::rcs::psp2`), so the lane index is 0;
a build whose gate is `first.r` is pixel-identical to main on Sol (0 pixels differ) and one whose
gate is `first.a` gives the table above. `track_a_floor_ds_disp`'s alpha has mean 0.195 (10th to
90th percentile 0.125 to 0.275); its red is the road's own red-brown, about 0.34 linear, which is
the 1.75 and the missing mottle. The cube, its orientation, its level law and the fresnel were
right. Confidence **92** that this is the whole of the overshoot (two circuits, seven boxes
within x0.8 to x1.2, the residue the size of boot-to-boot noise and of the `Fsl` lobe).

Scored with `compare-decompose.py --road road_alphaspec` (each build on its own masks, 16x):

| Pair | Road `LF` | Road `bias` | Road `G` |
| --- | --- | --- | --- |
| `sol-2048` | 17.0 to 8.8 | +13.8 to -3.0 | 0.74 to 0.79 |
| `queens-mall-2048` | 14.7 to 15.5 | +8.6 to -0.6 | 0.85 to 0.83 |

Queens Mall's `LF` does not fall: with the sheen right, what is left there is the base, darker
than the original's (above).

## 2048 against Omega

**Checked, differs.** 2048 (Vita, `PCSF00007` base, 1.04 patch, both DLC packs: 24,434
entries) ships no file whose name carries `panelmap`, `panel_df` or `PanelDisplacement`. Its
road is `fc16_phong_AlphaSpec_Normal_Emissive` over `Track_A_Floor_DS`/`_N`/`_E`, with no
`_disp` textures: the displaced road is a PS4 addition. The Vita program's reflection and
specular terms were not read (GXP operands, [gxp.md](../../../formats/gxp.md)), so whether
2048's own road has an alpha-gated sheen is open and not answered by this page. **Checked, applies, not
wired** for the lane: 2048 reads through the same `mesh::rcs::psp2` slot word (no alpha lane), so any
2048 term that `shade.wesl` multiplies by `texel.a` (the generic specular's `specular_mask`) reads the
diffuse's red there; which Vita programs gate by the diffuse alpha is unread (GXP operands).

Omega's HD-mode circuits carry sibling families this page did not read:
`Diffuse_Normal_Specular_PanelMap`, `..._PanelMap2In`, `D_S_N_CustomR_PanelMap(2In)` (Vineta
K) and `Diffuse_Normal_Specular_panelmap2` (Sol 2).

## Open

- ~~Which of the four nova programs a draw selects~~: a shadow-map one, `0xe144` or `0x14284`
  ("Live"). Which of the two, and the three extra interpolant lanes of `0x14284`/`0x164e4`, are open.
- **The `Fsl` lobe** `sat(dot(R, shadowLightDirection))^10` on the prelit term (weight 1, live
  constants above) is not drawn by ours; the original's base is 1.2 to 2.3 times ours in linear red
  in Sol's left and far boxes and 10 to 35 % brighter on Queens Mall's road. The frame constants refill every frame, so a
  knockout needs their source (the lighting object) or a patched program.
- **Queens Mall's base** under the sheen is darker in ours than in the original; `novaScaleBiasPower`
  is `(2, 0, 2.5)` there live. Compare ours' values before reading anything else.
- The panel map's and the cube's sampler states (filter, addressing, anisotropy).
- ~~A live read of `g_reflective_blur_coeffs`~~: read on the native port at runtime `0x8010e3420`
  (the patch address plus `0x7ff000000`) as `(1.72285, 2, 0, 1)`, six Sol boots and one Queens Mall boot, and through the name
  table's value pointer; 94 ("Live").
- The cube's orientation on a second circuit with a visible sun. **Sol checked a second way**
  (`panelmap-sun`): in the map as built (the `(R.x, R.y, -R.z)` composition), the brightest texel lies at
  `(-0.18, 0.43, -0.89)`, 0.15 rad from Sol's authored sun `(-0.105, 0.574, -0.812)`; the map reads 233
  at the sun's direction against a mean of 117 and 118 at its z mirror, so the flip puts the glare where
  the sun is. Altima, Queens Mall, Unity Square, Metro Park and Empire Climb's brightest texels do not
  sit at their suns (no usable glare in those cubes), so they decide nothing.
- `N2`'s doubled bitangent, checked against a frame: it changes the fresnel only where the
  normal map's green lane is far from 0.5. With the mapped normal ours lands within x0.8 to x1.2
  of the original's sheen, so it is worth at most that.
- The cube's orientation on a second circuit: Queens Mall's sheen agrees in size with the alpha
  gate (x0.8 to x1.7 across four boxes, one boot against another), which does not test the flip.

## Names

- `0x0202f080` `g_reflective_blur_coeffs` - 94. Data; the four floats bound as the shader
  constant `reflectiveBlurCoeffs` (patch `0x020e3420`). Read live ("Live").
- `0x01f8f688` `g_cube_reflection_tex` - 85. Data; the handle of the circuit's `skyCube.gnf`,
  bound as `cubeReflectionTex` (patch `0x020084b8`).

`FUN_0162a390` (patch; the circuit loader that opens `skyCube.gnf`) and `FUN_015f2ad0`
(patch; the binder, base `Scene_BindFrameConstants`) are not renamed here: the first was read
at one site only (hypothesis: the circuit load, confidence 45), the second already has its
base name.

## Implemented where

2026-11-16. `oag_mesh::mesh::rcs::psp2::panel_map` admits both families by name and the four
`Reflectivity*` parameters (PS4, with the material's normal map bound, since the march needs the
tangent frame) and carries the law's numbers in three glow-table entries of mode 15
(`PROGRAM_PANEL_MAP`); `crates/mesh/shaders/panel.wesl` is the march, `shade.wesl` the rest.
Bindings: the normal map third, `EmissiveMap` fourth, the panel map fifth as one half-float
channel (`BlockFormat::R16Float`, decoded by `oag_texture::gnf::Texture::half_floats`),
`ReflectionMask` sixth on `_mask`. `psp2::sky_reflection` reads `skyCube.gnf` beside a PS4
`skycube.rcsmodel` (the loader's own pairing, "The CPU side") with
`oag_texture::gnf::Texture::cube_faces` into a four-level `mesh::sky_paraboloid` map, level `k`
being cube `k`; it rides `Model::sky_reflection` to the scene's sky binding, and the shader mixes
levels `floor(2B)` and `floor(2B) + 1` by `fract(2B)`. The paraboloid's address composes to the
program's `(R.x, R.y, -R.z)` lookup (`sky_paraboloid`'s measured law).

What is chosen, not measured, or left out:

- **The cube is read at mip `2 + k`** (256 texels a face for the sharp cube) into the 512x256 map:
  the fragment stage has no free sampled-texture slot for a cube array, so the sky binding HD's
  glass already reads carries it.
- **The fresnel takes the mapped normal**, not the doubled-bitangent `N2` (open above).
- **The gate is the diffuse alpha, wired** (`sheen-gloss-wire`, 2026-11-16): the `if panel` block reads
  `first.a`, the diffuse sample at the marched coordinate, for `specular` and `sky_reflection`. It had read
  `texel.a`, which is `coverage[(slots >> 3) & 3]`, and `mesh::rcs::psp2` builds the slot word as 0 or
  `SECOND_IS_LIGHTMAP` with no alpha lane (`slots::DEFAULT` is only OR-ed in on the HD path, `skin::roles`), so
  lane 0 was the diffuse's red. The slot word is not changed: that would move every Omega and 2048 material
  that reads `texel.a`. Road at 16x, `compare-decompose.py --road road_alphaspec`, before to after: Sol `LF`
  17.0 to 8.8, `bias` +13.8 to -3.0, `G` 0.74 to 0.79; Queens Mall `LF` 14.7 to 15.5, `bias` +8.6 to -0.6, `G`
  0.85 to 0.83; Altima `LF` 24.7 to 24.9, `bias` +6.4 to +6.7, `G` 1.01 to 1.02; Unity Square `LF` 16.1 to
  16.2, `bias` -14.3 to -14.4, `G` 0.81 to 0.82 (their sheen is nearly all masked by a low alpha). Test:
  `mesh/tests/omega_panel_map_lit_path.rs`, `the_sheen_is_gated_by_the_diffuse_alpha_not_its_red` (a bright red
  with no alpha reflects nothing; fails on `texel.a`).
- **The lip blend is left out** (inert, above), and the lighting is the generic Omega nova path:
  the `0.75 + 0.25 ...` shadow-light factor is not drawn.
- **The sun lobe's colour is wired** (2026-11-16, `panelmap-sun`): `envsettings_light` fills
  `Light::sun_specular` with `Lighting.Sun specular colour * Lighting.Sun specular intensity` on a PS4
  source (Sol `(6.096, 3.263, 1.6) * 1`, Queens Mall `(3, 2.148, 1.459) * 3`), carried through the
  zone grade. At the pairs' poses it draws almost nothing (Sol 339 pixels, largest step 7 grey;
  Queens Mall 527, 1): the lobe is `sat(R.L)^p` with `p = 100 (1 - 0.75 * 0.3)` on Sol, narrow, and
  neither original frame shows a road highlight. A x100 probe moves 85,305 pixels on Sol (largest
  step 168), so the colour does reach the shader; the direction convention is the diffuse term's
  (`dot(n, direction)`), so `R.L` aims at the sun.

Measured on the compare pairs at 16x anisotropy, scored on the region masks of the before
render (`compare-decompose.py`'s law; the after build's own `sky` skip also removes the road's
reflection, so its own masks move):

| Pair | Road `LF` | Road `bias` | Road `G` |
| --- | --- | --- | --- |
| `sol-2048` | 32.7 to 18.0 | -32.4 to +16.1 | 0.35 to 0.75 |
| `queens-mall-2048` | 30.7 to 14.5 | -16.9 to +8.5 | 0.71 to 0.85 |
| `altima-2048` | 25.2 to 24.3 | +7.1 to +6.4 | 1.02 to 1.01 |
| `square-2048` | 17.1 to 16.0 | -14.9 to -14.0 | 0.83 to 0.80 |

By eye on Sol the sheen lands where the original's does (the far half, at grazing angles), and the
near road overshoots toward grey-blue: mean colour of a near patch (45, 32, 34) % in the original,
(48, 40, 43) % ours. Two probes on Sol: without the march the road scores `LF` 19.3, `bias` +18.0,
`G` 0.77: the march reshapes the recessed strips (3.3 % of the frame's pixels move by more than 8
levels, 1.3 % by more than 25; a debug render of the shift shows it across every strip), but the
road's gradient ratio comes from the sheen, not from the march; with the lookup's `z` unflipped the road
scores `LF` 16.0, `bias` +10.0, `G` 0.76, near better and right-hand mid worse, so the orientation
is not settled by this pair and the page's flip stands. Queens Mall's cyan lines in the pair sit on
the edge of a deep trench, not on the panel-map road: which material draws them is open.

Sol's near-road overshoot, probed (`panelmap-sun`, one shader change each, the near patch is a
250x140 px box at (250, 780) of the 1920x1080 frame, mean colour in % of full scale; original
(48.1, 34.5, 35.6), ours (50.5, 41.4, 44.7)). Re-rendered on main the road scores `LF` 16.9, `bias`
+13.7, `G` 0.74 (the lane above recorded 18.0, +16.1, 0.75):

| Probe | Near patch | Sol road `LF` / `bias` / `G` | Queens Mall road `LF` / `bias` / `G` |
| --- | --- | --- | --- |
| no sheen | (41.1, 23.5, 23.6) | 31.8 / -31.6 / 0.36 (own masks) | 29.1 / -15.7 / 0.68 |
| cube `z` unflipped | (46.1, 35.4, 37.2) | 14.7 / +8.2 / 0.71 | 19.7 / -3.4 / 0.73 |
| fresnel from the geometric normal | (50.6, 41.6, 45.0) | 17.4 / +14.8 / 0.63 | 14.4 / +9.3 / 0.87 |
| sheen read one blur level blurrier | (49.4, 40.2, 43.5) | 16.8 / +13.4 / 0.66 | 13.4 / +8.8 / 0.78 |

The fresnel's normal and the cube's blur level do not move the near patch, so neither `N2` nor the
mip choice is the overshoot. The unflipped cube lands the patch (the original's sheen, taken as the
patch less the no-sheen patch, is (7, 11, 12) against ours (9.4, 18, 21): ours about 1.7 times too
bright and bluer) but costs Queens Mall's road `LF` 14.6 to 19.7 and contradicts the sun check
under "Open"; the sun's glare sits where the sun is only with the flip. **The cause is not found**:
what is left is the sheen's magnitude and colour (a missing weight on the sheen term, or the base
the sheen is added to), not one of the three suspects.

Tests: `crates/texture/tests/omega_gnf_ground_truth.rs` (the two decoders against the disc),
`crates/mesh/tests/omega_panel_map_ground_truth.rs` (Sol's build: every panel vertex carries the
entries with Sol's authored values, the panel map binds as a half float, the skycube carries the
four-level map), `crates/mesh/tests/omega_panel_map_lit_path.rs` (a GPU pixel test that fails
with the march, the reflection weight or the emissive dropped). HD and 2048 frames are unchanged
to the pixel (`hd-race/talons-pack`, `2048-vita/anulpha-pass`).

## History

- 2026-11-16 (`panelmap-re`): the law read from the microcode, no live run; scores to 88.
- 2026-11-16 (`panelmap-sun`): the fresnel's normal, the blur level and the cube's flip probed in
  ours; cause of the overshoot not found.
- 2026-11-16 (`sheen-re`): knockouts on the native port. `g_reflective_blur_coeffs` 88 to 94 (live
  read); the diffuse-alpha gate 88 to 94, the cube lookup 88 to 92, the fresnel 88 to 90 (live
  states against ours); the drawing program narrowed to the shadow-map pair. The overshoot is
  ours' gate lane, not a term of the program.
