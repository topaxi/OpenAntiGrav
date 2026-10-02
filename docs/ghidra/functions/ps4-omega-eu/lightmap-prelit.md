# The lightmap's combination: `Lighting.Nova prelit scale bias power` is `pow(lightmap, power) * scale + bias`

2026-10-02, `lane/omega-lightmap`. `eboot.bin` (WipEout: Omega Collection, PS4,
`CUSA05670`, EU), `x86:LE:64:default`, image base `0x01000000`, Ghidra program
`/omega/eboot-ps4-omega-eu.bin`. **Static evidence only**: no PS4 emulator
exists in this project's toolchain, so nothing here was watched running, and
every score below is capped at 84 by the
[rubric](../../../reverse-engineering/confidence-rubric.md) ("decompilation only,
consistent call sites").

The question this answers was left open by
[`omega-status.md`](../../../formats/omega-status.md#racing-a-race-starts-on-this-titles-own-data):
the patch's `.EnvSettings` carries `"Lighting.Nova prelit scale bias power"=2.5
0.2 2`, the renderer borrowed Wipeout HD's `scale * lightmap^power` curve
unread, and nothing said how three numbers become a light. **Answer, from the
GCN pixel shaders inside the circuit `.rcsmaterial` files:**

```text
prelit = scale * pow(lightmap.rgb, power) + bias        (per channel, no clamp)
         scale = .x, bias = .y, power = .z of the authored triple
lit    = prelit * shadowLightFactor + sun * N.L * lightmap.a   (+ specular)
colour = albedo * lit + specular
```

**No constant ambient joins that sum** on a lightmapped surface (below), the
lightmap's alpha is the sun mask exactly as on HD, and the shader exports
**fp16 linear** into a target this renderer does not have (the tonemap section).

## The CPU side: where the triple goes

All addresses are this binary's. Names are in [`names.tsv`](names.tsv).

| Step | Address | What |
| --- | --- | --- |
| Registration | `FUN_015c1f20`, call at `015c2001`, string `0x018299f8` | `FUN_01791af0(env, &DAT_01e3de80, "Lighting.Nova prelit scale bias power")`: one row of a ~110-row table of `(destination, key string)` registrations, called under a static guard (the call at `015cb539` inside `FUN_015c63a0` is one of the guarded accessors of the object at `0x01e3c0d0`). `FUN_01791af0` is the **vector** registrar (the same one registers `Lighting.Sun direction` and `Lighting.ShadowLight direction`; the scalar one is `FUN_01791540`, the bool one `FUN_01791100`). |
| Static default | `FUN_015c2dc0`, write at `015c609b` | `_DAT_01e3de80 = 0x3e4ccccd3fb33333`, `_DAT_01e3de88 = 0x3fc00000`, i.e. **`(1.4, 0.2, 1.5)`**. A file that omits the key keeps this, not the identity. |
| Read, once per frame | `FUN_015c63a0`, `015cb5a1` | `VMOVUPS XMM13, [0x01e3de80]`. The only instructions that touch `XMM13` before the store are two lane shuffles (`015cb7e1`, `015cb7e6`) and the three stores at `015cb88f`..`015cb898`: **the triple is uploaded verbatim as three floats, no arithmetic**. |
| Stored | `015cb89d` | `MOV [0x01e3e900], RCX`: the pointer to the three floats goes into a table of per-frame constant pointers. |
| Bound | `FUN_015c6120` | `*(DAT_01fc88f8 + 0x510) = DAT_01e3e900` (and a version counter at `+0x506` incremented). |
| Name | `FUN_015c2dc0`, entry at `0x01e3cf00` | The shader constant name table (hash, flags, name, count, size): `novaScaleBiasPower`, hash `0xa12d08aa`, count 1, size 3. |

**The binding is a second site, and the arithmetic closes.** Name-table entries
are `0x28` bytes from `0x01e3ca00`: `(0x01e3cf00 - 0x01e3ca00) / 0x28 = 32`. The
binding table is `0x10 + slot * 0x28`: `(0x510 - 0x10) / 0x28 = 32`. The CPU
global `0x01e3de80` and the shader constant `novaScaleBiasPower` are the same
slot, and the three floats stay in the order they were authored.

The shader side is the part that says what the three numbers mean.

## The shader law, read from the GCN microcode

**Tooling, and a negative worth recording.** LLVM 21 and 23 have no Sea Islands
disassembler (`llvm-mc -mcpu=bonaire --disassemble` says "disassembly not yet
supported for subtarget"). The PS4's GPU is GCN 1.1. A disassembler written for
this lane from the public ISA documents (scratch, not shipped: SOP1/2/C/K/P,
SMRD, VOP1/2/C/3, VINTRP, MIMG, EXP) decodes the circuit materials, and it is
**checked on the vertex shader first**: the head of `diffuse_normal_specular`'s
vertex program decodes to a clean `viewProj` matrix multiply (`v_mac_f32` chains
over `s[8:23]` loaded by `s_buffer_load_dwordx16 s[8:23], s[4:7], 0x0`), an
`exp tgt=12` position export, the Orbis fetch-shader call
(`s_swappc_b64 s[0:1], s[0:1]`) and a fog tail
`v_mul_f32 v4, 0x3fb8aa3b, v4` / `v_exp_f32` (`1.4427 = log2(e)`, so
`exp(-k d)`), ending in `s_endpgm`. A decoder that gets all of that right is not
guessing at the multiply-add shapes it is about to be asked about.

**The container.** A PS4 `.rcsmaterial` opens `0xCA5CADE5` and carries many
`Shdr` blobs. Each shader is: an `Shdr` header, a `s_mov_b32 vcc_hi, <literal>`
marker (`ff 03 eb be`) that starts the code, the code to `s_endpgm`
(`00 00 81 bf`), then the `OrbShdr` footer, and after it the reflection tables
and the name strings (`__GLOBAL_CB__`, the constant-buffer member names, the
texture and sampler names). Shader type is `(footer[8] >> 2) & 0xf`: `1` is a
vertex shader, `0` a pixel shader. The constant-buffer members are `(offset,
size)` records in the same order as their names, and their sizes agree with the
executable's own name table (`novaScaleBiasPower` 3 floats, `fogColour` 4,
`liveLighting0diffuse` 3, ...), which is what makes the member order and the
offsets a reading rather than a guess. `s_buffer_load` immediate offsets count
**dwords**.

**One pixel shader, read by hand** (`Data/environments/05_ubermall/Materials/Diffuse_Normal_Specular.rcsmaterial`
in `data02.psarc`, the pixel shader at byte `0xad04` of that file: textures `Texture1`, `Texture2`, `lightmap`;
constant buffer `novaScaleBiasPower`@dw0, `liveLighting0diffuse`@dw4,
`liveLighting0specular`@dw8, `shadowLightSpecularDiffuse`@dw12,
`liveLighting0direction`@dw16, `shadowLightDirection`@dw20, ..., `fogColour`@dw28):

```text
s_buffer_load_dwordx16 s[16:31], s[36:39], 0x0     ; s16,s17,s18 = nova.x,.y,.z
image_sample v[9..], dmask=0xf, v[5..], s[24:31], s[32:35]   ; coords attr2.xy = lightmapUV
v_log_f32 v9,  |v9|           ; log2(lightmap.r)
v_mul_f32 v9,  s18, v9        ; * nova.z       (the exponent)
v_exp_f32 v19, v9             ; = lightmap.r ^ nova.z
v_mad_f32 v10, v19, s16, v20  ; * nova.x + v20, and v20 = s17 = nova.y  (scale, bias)
v_mul_f32 v15, s20, v12       ; liveLighting0diffuse.r * lightmap.a   (v12 = sample .w)
...                           ; * clamp(N . liveLighting0direction)
v_mac_f32 v15, v10, v9        ; + prelit.r * shadowLightFactor
v_mac_f32 v18, v15, v5        ; + (that) * albedo.r        (v5 = Texture1 sample)
```

The lightmap sample is the one addressed by the vertex shader's second
interpolant (`attr2`), which is the vertex program's `lightmapUV` pass-through;
the albedo and the normal map share `attr1` (`Uv1`). The three channels go
through the same `log`/`mul`/`exp`/`mad`. There is **no clamp** on the
prelit sum (a `clamp` modifier appears only on the dot products). The result is
mixed with `fogColour` by `attr0.w` and exported through
`v_cvt_pkrtz_f16_f32`: **fp16 pairs, `exp ... compr`**.

**Consistency.** The same `log2 -> mul s(nova.z) -> exp2 -> mad s(nova.x), s(nova.y)`
shape reads off the three other nova variants read by hand
(`Shdr` blobs at `0x9a64`, `0xd044`, `0xe444` of the same file), and off a census
of all 7,911 tag-valid PS4 `.rcsmaterial` files in the five base archives
(`data00`..`data04`; the patch's were not read; this is more than
[`rcsmaterial.md`](../../../formats/rcsmaterial.md)'s "439 of 1,270" tag-valid
files of 2026-09-16, which predates the corrected extraction of 2026-09-29 and
is not reconciled here):

| Count | |
| --- | --- |
| unique pixel shaders that declare `novaScaleBiasPower` | **4,664** |
| of those, also declaring `lightmapTexture` | **4,664** (all) |
| of those, declaring `constantAmbientColour` | **0** |
| reflection parsed (member records == member names) | 4,664 |
| `log -> mul by the register holding constant-buffer dword nova+2 -> exp` found | **4,640** |
| not matched by that pattern | 24 (not read; see Open) |
| unique pixel shaders with `lightmapTexture` and no nova | 22 |
| unique pixel shaders with `constantAmbientColour` | 4,337, none with a lightmap or nova |

So on this title **a surface is either lightmapped and nova-curved with no
constant ambient, or it takes the constant ambient with no lightmap**, with no
shader that does both: the same split the HD microcode has
([`ps3-hdfury-eu/renderer.md`](../ps3-hdfury-eu/renderer.md), blocks #8/#9). The
census's SGPR bookkeeping is time-aware (`s_buffer_load` destinations are
overwritten as the program runs) and the member offsets come from the reflection
records, not from an assumed layout.

**The vertex-colour variants use the same curve.** A vertex shader that takes
`NOVAColor` (`Shdr` `0xb6c4`) runs `v_log_f32 |colour|`, multiplies by the
power, `v_exp_f32`, and `v_mad_f32 s48, v, bias`: the identical combination on
the baked per-vertex colour instead of the atlas. Not wired here (below).

**What the three numbers are.** `scale` is `.x`, `bias` is `.y`, `power` is `.z`
of the authored triple, in the file's order. Across the 97 `.EnvSettings` in the
five base archives and the patch's, **88 author the key** (the other nine are
front-end files) with **20 distinct triples**, from `1 0 1` to `16 0.96 12`, and
one with a **negative bias** (`2 -0.7 1`): the bias is added unclamped.

## The atlas: UNORM, and its alpha is the sun mask

Word 1 of the 8-dword GNF descriptor splits as data format bits 25:20, number
format bits 29:26. **All 1,028 `*-lmap.gnf` atlases on the base archives are BC7
(`0x29`) with number format 0 (UNORM).** The control: the same parse on
`mar_metalstrips.gnf` and `dc_concrete.gnf` (albedo) reads number format `9`
(sRGB), and on `mar_metalstrips_N.gnf` (a normal map) reads `0`. So the shader
samples the lightmap **raw** and raises it to `power`; there is no sRGB
decode in front of the curve. (The renderer's `pow(lightmap, 2.2)` pre-decode,
which is HD's, would be a second decode on this title.)

The atlas **alpha is the direct sun's mask**: `liveLighting0diffuse * lightmap.a *
clamp(N . L)` (`v_mul_f32 v15, s20, v12`), the same role it has on HD. 39 to 55 %
zero on the atlases sampled is therefore a baked sun shadow, not an unused
channel.

## What is recovered and not wired

- **The shadow-light factor.** In the hand-read shader the prelit term is
  multiplied by `0.75 + 0.25 * sat(N . shadowLightDirection) + shadowLightSpecularDiffuse.y *
  sat(R . shadowLightDirection)^shadowLightSpecularDiffuse.x`. All 4,664 nova
  shaders declare `shadowLightDirection`, but the `0.75` and `0.25` literals
  appear in only 1,780 of them, so the factor is **general in presence and not in
  one fixed form**. The patch's `.EnvSettings` authors `Lighting.ShadowLight
  direction` and `Lighting.ShadowLight specular power scale`. This renderer
  carries no shadow-light rig; the factor is left at 1 (**chosen, not
  measured**).
- **Vertex-colour variant** (above): the same curve, on a path whose role bits
  the PS4 material container does not yet provide.

## The `Tonemap.*` block: parsed, with no consumer found

`FUN_015c1f20` registers `Tonemap.Exposure minimum/maximum/response/time`,
`Tonemap.Luminance a/b-coefficient`, `Tonemap.Source color end a/b-coefficient`,
`Tonemap.Start angle`, `Tonemap.End angle` at object offsets
`+0x1e8,+0x1ec,+0x1f0,+0x1f4,+0x1e0,+0x1e4,+0x1f8,+0x1fc,+0x200,+0x204`, and the
`TonemapHDR.*` twins at `+0x208..+0x22c`, then `Bloom.*` at `+0x230`, `BloomHDR.*`
at `+0x254`, `Vignette`, `MotionBlur`, `DepthOfField` and `Water`.

**Clean negative.** With the object at `0x01e3c0d0` (the address the
registrar is called with at `015cb52f`), no instruction in the program reads
`0x01e3c2b0..0x01e3c2ff` by absolute address (operand searches for `0x01e3c2b`,
`0x01e3c2c`, `0x01e3c2d`, `0x01e3c2e`, `0x01e3c2f` and the zero-less `0x1e3c2`
all return zero matches; the same search style finds the `+0x160..+0x1a9`
fields read by `FUN_015c63a0`, so the method does work). A read through the
object pointer (`[reg + 0x1e8]`) cannot be told from the 265 unrelated
`+0x1e8` accesses in the program by an operand search, and was not chased.
**The consumer is not located.** The next addresses to try: the `wo_composite_*`
shader-program registry built at `FUN_01623500` (strings `wo_composite_fp`,
`wo_composite_notonemap_fp`, `wo_composite_nocc_fp` at `0x0182cb3b..0x0182cbc7`,
`pal_ToneMapCoefficientsFilter_fp` at `0x0183c926`, referenced from
`FUN_0178b970`), whose per-frame constants would come through a path other than
the scene-constant table.

**What it means for the picture.** The pixel shaders export **fp16 linear**
(`v_cvt_pkrtz_f16_f32`, `exp compr`) and the title carries `Tonemap` and `Bloom`
blocks plus composite shaders named `notonemap` and `nocc`, so the circuit is
rendered to an HDR target and mapped afterwards. A 2.5x prelit scale is
authored for that. **This project's target saturates, so Omega frames clip
more once the combination is right; that is the missing stage showing, not a
defect of the combination.**

## Names

[`names.tsv`](names.tsv) rows added by this page, scored on the static evidence
above (all below the 85 ceiling this evidence allows):

- `0x015c1f20` `EnvSettings_RegisterKeys` - 82. About 110 `(destination, key
  string)` rows through typed registrars, all keys the HD `.EnvSettings` reader
  also names, called once under a guard against the object at `0x01e3c0d0`.
- `0x015c2dc0` `ShaderConstants_InitNameTable` - 78. A static initialiser that
  writes the `(crc32, flags, name, count, size)` table at `0x01e3ca00` (111
  entries, `0x28` apart) and the defaults of `0x01e3de20..0x01e3de98`, including
  the nova default.
- `0x015c6120` `Scene_BindFrameConstants` - 72. Copies each per-frame constant
  pointer (`0x01e3e8a0..0x01e3e970`) into the constant bank at `DAT_01fc88f8` at
  `0x10 + slot * 0x28` and bumps its version byte.
- `0x015c63a0` `Scene_PrepareFrame_q` - 66. The per-frame function that reads the
  environment globals (`0x01e3de20..0x01e3de98`, `0x01e3c224..0x01e3c2a9`) and
  builds the pointer table; same role as `ps3-hdfury-eu`'s `Scene_PrepareFrame`
  (`0x003aa888`), not matched instruction by instruction, hence the `_q`.
- `0x01e3de80` (data) `EnvSettings_NovaPrelitScaleBiasPower` - 88. The registered
  destination of the triple, and the global `Scene_PrepareFrame_q` reads.

## Open

- **24 of 4,664** nova pixel shaders do not match the `log -> mul -> exp` pattern
  on nova's third dword; three of them have more than one constant buffer view
  and a `shadowMap` and were not read by hand.
- Whether the registered vec3 is read from the `.EnvSettings` as three numbers
  in file order is not independently watched: the registrar is a typed one
  (`FUN_01791af0`, the same one that registers the sun direction and the
  Physical Sun direction), and the shader's use of `.x/.y/.z` as scale/bias/power
  matches the key's own name.
- The `Tonemap.*` consumer, the shadow-light factor's generality, the vertex-
  colour variant's role bits, and every number above on a running PS4.
