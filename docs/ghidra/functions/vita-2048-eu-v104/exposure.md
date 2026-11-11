# Exposure and the race composite: `Lighting.Exposure*` drives `wo_composite_nocc_fp`

2026-10-26, `2048-exposure`. `eboot.elf` (WipEout 2048, Vita, `PCSF00007` EU,
patch v1.04), Ghidra program `/2048/eboot-vita-2048-eu-v104.elf`. Static reading
plus live reads and writes of the guest's memory in Vita3K through
`scripts/vita-2048-pair.py --ram-read/--ram-poke/--ram-fill` (see
[compare-page.md](../../../tools/compare-page.md)).

This closes what [envsettings.md](../../../formats/envsettings.md) listed as
"authored and left unread" for 2048: `Lighting.ExposureScale`,
`Lighting.ExposureMax`, `Lighting.ExposureSpeed`, `Lighting.BloomFactor` and
`Lighting.BloomGate`. Since 2026-10-26 (`2048-exposure-wire`) ours reads all
five and runs this chain, `oag_post::wo_composite` (see "Ours" below).

## The law

```text
once per rendered frame, when the adapt flag 0x8151c5bc is set:
  a      = mean over every 4th pixel of every 4th row of the quarter-res gate
           target's alpha byte, times 1/64            (0x3c800000)
  if a > 0.01:
    E   += (Scale / a - E) * Speed                     no dt, no lower clamp
    E    = min(E, Max)
gate pass (wo_bloom_gate_fp, into the quarter-res target 0x816aef1c):
  s      = sum of 4 taps of the scene                  (offset0..3, one scene
                                                        texel out on each diagonal)
  Lg     = dot(s, lum * 0.25)                          mean luminance of the taps
  rgb    = (E * clamp(Lg - Gate, 0, 1))^2 * s          alpha = Lg * 0.25
two blur passes (wo_blur_fp), quarter target to quarter target:
  o      = (0.5 / w, 0.5 / h), then (-0.5 / w, 0.5 / h)  w, h the quarter size
  out    = 0.217041 * sum over k in (1, 3, 5) of
           weight_k * (tex(uv - k o) + tex(uv + k o))    weights 1.0, 0.7998, 0.5
composite (wo_composite_nocc_fp), per pixel:
  c      = alpha.rgb + (1 - alpha.a) * main.rgb
  e      = E * c
  L      = dot(e, lum)                                 lum = (0.3, 0.59, 0.11)
  out    = sqrt(e * L / (1 + L) + BloomFactor * bloom) * v + screenTint
```

`Scale`, `Max`, `Speed`, `BloomFactor` and `Gate` are the circuit's
`Lighting.ExposureScale`, `ExposureMax`, `ExposureSpeed`, `BloomFactor` and
`BloomGate`. `v` is a per-channel vignette factor `1 - t^2`, `t = vignetteTint.rgb * w` the vertex
program's third output (`wo_composite_vp`) and `w` the composite mesh's `pos.w`
("The vignette" below).
`main` is the scene, which 2048's lit programs write in a gamma-2 linear space:
a lightmapped track program decodes its diffuse texel by squaring it and
multiplies the raw lightmap scale-bias in (Vita3K's translation of the program,
read the same way as the composite below). The final `sqrt` is the matching
encode. Confidence for the composite as a whole: **85**; for the adaptation
step: **85**; for "hardware adapts where Vita3K does not": **75**; for the
gate's `alpha = Lg * 0.25` and so `a` = the mean gate luminance: **80**; for
the blur kernel: **75**. Parts below.

## Addresses

| Address | Name | Conf. | What |
| --- | --- | --- | --- |
| `0x8103d2ae` | `PostFx_RegisterPrograms` | 85 | Registers the post programs by literal name: `wo_bloom_gate_vp/fp` (`0x816af080`/`0x816af098`), `wo_blur_ds_fp`, `wo_blur_vp/fp`, `wo_composite_vp`, the two Zone composites, `wo_composite_notonemap_fp` (`0x816af170`) and `wo_composite_nocc_fp` (`0x816af188`). Seeds the luminance factor `(0.3, 0.59, 0.11)` at `0x816af030`/`0x816af040`. |
| `0x81037670` | `PostFx_CreateTargets` | 75 | Creates three quarter-resolution colour surfaces (`0x816aef1c`, `0x816aef60`, `0x816aefa4`, width and height `>> 2`) and looks up every uniform the programs name (`exposure`, `luminanceFactor`, `bloomFactor`, `screenTintColour`, `accumFactor`, `mainScaleBias`, `bloomScaleBias`, `vignetteTint`, ...). |
| `0x810387b4` | `PostFx_RenderChain` | 72 | Begins a scene on `0x816aef1c` and draws the gate into it with `exposure = half2(E, Gate)`; blurs into the other two; binds the race composite `wo_composite_nocc_fp` with `bloomFactor = half4(BloomFactor, E, 1, 0)`, `luminanceFactor` from `0x816af040` and the screen tint. The Zone and front-end branches pick the other composites (`0x816a4d80`, `0x816a4d84`; both read 0 on every race below). |
| `0x81036800` | `PostFx_RunFrame` | 65 | Calls `PostFx_RenderChain`, then `Exposure_AdaptFromReadback` when `0x8151c5bc` is set. |
| `0x8103d148` | `Exposure_AdaptFromReadback` | 85 | The adaptation above, NEON: `vld1.8` of byte `+3` every 16 bytes, rows stepped by 4, `vrecpe` of the count, `vmul` by `0x3c800000`, `vcgt` against `0x3c23d70a` (0.01), then `E + (Scale / a - E) * Speed` (`vsub`, `vmla`) and `vbsl` against Max. |
| `0x81038724` | `Exposure_SetParams` | 85 | `(s0, s1, s2)` into Scale `0x8151c5dc`, Max `0x8151c5e0`, Speed `0x8151c5e4`. Called every frame by the per-frame scene setup `FUN_8101bc7a` at `0x8101cece` with the schema's `+0x41c`, `+0x420`, `+0x4b8` (Zone: `0x8101c0c6`). |
| `0x8103870a` | `Bloom_SetFactorAndGate` | 85 | `(s0, s1)` into `0x8151c5d4`/`0x8151c5d8`; called at `0x8101ce98` with the schema's `+0x414` (`Lighting.BloomFactor`, string `0x815087ec`) and `+0x418` (`Lighting.BloomGate`, `0x81508804`). |
| `0x8103d29c` | `Exposure_Set` | 80 | Writes E. Its one caller, `FUN_81071300` at `0x81071470`, passes the constant `0x3f933333` (1.15). When that caller runs is not read. |
| `0x8151c5e8` (data) | `g_Exposure` | 90 | E. |
| `0x8151c5dc` (data) | `g_ExposureScale` | 90 | |
| `0x8151c5e0` (data) | `g_ExposureMax` | 85 | |
| `0x8151c5e4` (data) | `g_ExposureSpeed` | 90 | |
| `0x8151c5d4` (data) | `g_BloomFactor` | 85 | |
| `0x8151c5d8` (data) | `g_BloomGate` | 85 | |
| `0x8151c5bc` (data) | `g_ExposureAdaptEnabled` | 60 | Read 1 on every race below; its four writers (`0x81022988`, `0x81035c08`, `0x81021c5e`, `0x81021cb8`) were not read past their stores. |
| `0x816aef08` (data) | `g_VignetteTint` | 80 | `vignetteTint`, four floats `1.0, 1.0, 0.97, 1.0` seeded by `PostFx_RegisterPrograms`. |
| `0x8151c6b0` (data) | `g_VignetteStrength` | 75 | The mesh's `w` on the outer ring, 1.0 (0 while a photo capture runs). |
| `0x8151c6b4` (data) | `g_VignetteCentreY` | 80 | The mesh's centre height in clip space, -0.885. |
| `0x8151c6b8` (data) | `g_VignetteInnerRadius` | 80 | Inside it `w` is 0, 1.5. |
| `0x8151c6bc` (data) | `g_VignetteOuterRadius` | 80 | At it `w` is the strength, 2.15. |

The schema offsets are `Environment_RegisterLightingSchema`'s
([lighting-schema.md](lighting-schema.md)): `+0x41c` is registered against
`"Lighting.ExposureScale"` (`0x81508818`, default 0.5), `+0x420` against
`"Lighting.ExposureMax"` (default 2.0), `+0x4b8` against
`"Lighting.ExposureSpeed"` (default 0.05).

## The composite, read twice

`wo_composite_nocc_fp`'s program is the GXP container at `0x81529598` (611
bytes). Its parameters (`scripts/vita-gxp.py dump`): `bloomFactor`,
`luminanceFactor`, `screenTintColour`, `accumFactor`, all half-precision in
container 14, and samplers `mainTex`, `alphaTex`, `bloomTex`. Vita3K caches the
SPIR-V it translates each program to under the SHA-256 of the container, so
`sha256(container)` names the translation of exactly this program
(`924e5382...`); `spirv-dis` of it, walked symbolically over its register
files, gives the formula above. Vita3K's output is used as a behaviour oracle
only, and the formula here is written out by hand. The secondary program copies
`accumFactor` over `bloomFactor.zw`; neither reaches the colour output.

The same formula, from an independent decoder, is in Omega's executable:
[`ps4-omega-eu/tonemap.md`](../ps4-omega-eu/tonemap.md) reads
`wo_composite_nocc_fp` off GCN microcode as
`c = alpha.rgb + (1 - alpha.a) * main; c *= exposure; L = dot(c, luminanceFactor); out = c * L / (1 + L) + ...`,
registered there and referenced by no code. That page calls the `wo_*` set
"Wipeout HD's composites carried forward"; this is the title that runs them.

## The vignette: `v = 1 - (vignetteTint * w)^2`, a mesh dark at the top of the frame (2026-11-11, `w2048-vignette`)

Static reading of `PostFx_RenderChain` and of the two programs' USSE listings
(`scripts/vita-usse.py disasm`), checked against the three Vita3K pairs. Confidence
**85** for the law and the mesh, **80** for the constants (a static seed with one writer,
not watched at run time: Vita3K was not run, see "Not measured"), **70** that the mesh's
`+y` is the picture's top (the corner measurement below, not the Vita's NDC convention).

- **`wo_composite_vp`** (`eboot.elf` program #9, container `0x81526230`, 477 bytes): attributes
  `pos` (4 floats) and `uv`, uniforms `mainScaleBias`, `bloomScaleBias`, `vignetteTint`. The
  position output is `pos.xyz, 1` scaled by the constant ROM's `c1.y`; the vignette output is
  `vignetteTint.xyz * pos.w` in half precision times `c15.y` (1.0), written as `o8`/`o10`
  with `w = 1`. The mesh is drawn in clip space with no transform, so the composite is **the
  mesh**, not a full-screen triangle; the program has no other input of `w`.
- **`wo_composite_nocc_fp`** (program #37, `0x81529598`): its last three instructions are
  `pa0.xyz = -t * t`, `pa0 = pa0 * S + screenTint`, `pa0.xyz += S`: `out = S * (1 - t^2) +
  screenTint`, `S` the `sqrt` term above. So `v = 1 - t^2` per channel, `t = tint * w`.
- **`vignetteTint`** is the half4 `PostFx_RenderChain` packs (`local_348`/`local_344`) from four
  floats at `0x816aef08..0x816aef14`. `PostFx_RegisterPrograms` stores `1.0`, `1.0`, `0.97`
  (`0x3f7851ec`), `1.0` there (`0x8103d762..0x8103d770`); the address has no other writer
  (`get_xrefs_to`), so the tint is `(1, 1, 0.97)`: blue is darkened a little less.
- **The mesh** (the race branch, `0x8103ae80` on): 32 sectors (angle step `0x3e490fd9`,
  `2 pi / 32`) of nine vertices, `A B C | B C D | C D E`, each `(uv, pos)` with
  `uv = (x / 2 + 0.5, 1 - (y / 2 + 0.5))`. With `a`, `b` the sector's two angles: `A` the centre
  `(0, cy)`, `B`/`C` the inner circle `(R1 sin, cy + R1 cos)` at `a`/`b`, `D`/`E` the outer circle
  at `a`/`b`; `pos.w` is 0 on `A`, `B`, `C` and `s` on `D`, `E`. The values are the floats
  `g_VignetteStrength` `0x8151c6b0` = 1.0 (`s`, read at `0x8103aea6`), `g_VignetteCentreY`
  `0x8151c6b4` = `0xbf628f5c` = -0.885, `g_VignetteInnerRadius` `0x8151c6b8` = 1.5,
  `g_VignetteOuterRadius` `0x8151c6bc` = `0x4009999a` = 2.15. Each has one reader and no writer.
  So `w` is 0 inside radius 1.5 of `(0, -0.885)` in clip space and ramps linearly over the
  ring's triangles to 1.0 at 2.15; the frame's top corners are at 2.134, its bottom corners at
  1.007 (no vignette), its top edge's middle at 1.885 (`w` 0.59, `v` 0.65). The outer polygon's
  chord minimum is 2.1396, so the mesh just covers the frame.
- **The switch**: `s` is 0 when `FUN_81021ef6` (`0x81021ef6`) returns nonzero (`0x8103ae80`).
  It reads two 64-bit words at `0x81519638`/`0x81519640` and the byte `0x815d29e0` that
  `FUN_81021992` (the photo capture's job start, `photo-mode.md`: `FUN_81021992(job, 0)`)
  sets to 1 (four more functions write it, unread): it is true only while that
  flag is set and the 64-bit word at `0x81519638` is past `0x81519640`, which
  `FUN_81021992` sets to its start plus the job's length, so it reads as "a photo
  capture job is running past its window" and a race has the vignette on throughout
  (confidence 60; no name given). **Departure: ours' photo mode does not read it.** Its
  accumulated shot (`oag_raceplay`'s `photo::shot`, frames of a motion blur or a depth of field)
  draws each frame through this chain with the vignette on, where the original drops it for the
  capture; wiring that on a score-60 reading is left open.
- **Precision**: the vertex program's product and the fragment's square are half
  precision (about 3 digits); ours runs `f32`.

### The original shows it

The original's top corners are black on all three pairs and its bottom corners are lit.
The outermost 8x8 pixels, luma, original / ours before / ours after:

| Pair | TL | TR | BL | BR |
| --- | --- | --- | --- | --- |
| Queens Mall | 1.8 / 8.6 / 0.6 | 1.8 / 64.6 / 4.5 | 90.4 / 116.0 / 116.0 | 47.7 / 47.0 / 47.0 |
| Empire Climb | 1.6 / 7.1 / 0.4 | 1.2 / 8.6 / 0.6 | 80.7 / 149.5 / 149.5 | 140.0 / 216.1 / 216.1 |
| Vineta K (DLC) | 0.9 / 11.3 / 0.7 | 1.3 / 66.6 / 4.8 | 23.2 / 19.0 / 19.0 | 17.6 / 8.1 / 8.1 |

The 120x120 corner boxes (HUD pixels masked: near-white in any of the three frames, grown 3 px;
the unmasked means follow), luma, original / before / after:

| Pair | TL | TR | BL | BR |
| --- | --- | --- | --- | --- |
| Queens Mall | 25.4 / 45.6 / 26.4 | 21.5 / 19.6 / 11.1 | 79.8 / 84.6 / 84.6 | 58.9 / 60.8 / 60.8 |
| Empire Climb | 12.2 / 16.9 / 9.4 | 19.4 / 8.6 / 4.2 | 80.0 / 97.1 / 97.1 | 152.4 / 155.5 / 155.5 |
| Vineta K (DLC) | 14.0 / 25.5 / 15.3 | 29.1 / 54.6 / 32.8 | 26.6 / 25.4 / 25.4 | 60.0 / 52.2 / 52.2 |

Unmasked: Queens Mall TL 61.3 / 74.4 / 60.0, TR 48.3 / 42.0 / 33.9; Empire Climb TL 51.1 / 51.3 /
45.6, TR 42.6 / 31.5 / 27.6; Vineta K TL 53.7 / 61.1 / 52.2, TR 47.5 / 70.8 / 50.6. The bottom
boxes do not move (the mesh's `w` is 0 there), the top-left boxes of Queens Mall and Vineta K and
the top-right of Vineta K land on the original's, and the top rows of Vineta K fit
(rows 0-7 masked 11.8 / 33.5 / 13.5, top edge middle 100.4 / 161.3 / 107.6). **Where it does not
close**: the top edge's middle on Queens Mall and Empire Climb goes under the original's (59.1 /
62.1 / 41.3 and 108.8 / 108.0 / 72.0), and Empire Climb's top-right box under it. Those are
the regions the original lights with the sun's lens-flare streak and haze that ours does not draw
([lens-flare.md](../../../rendering/lens-flare.md)), and ours' sky and walls are darker there
before the vignette (Empire Climb rows 8-39: 42.5 / 41.6 / 29.5), so the box mean is not a
measure of `v` alone; the 8x8 corners, where the original is black on all three, are. Frame
mean luma, original / before / after (before is this tree with the vignette off, which carries
the lighting work landed since the table under "Ours" was taken): Queens Mall 60.2 / 53.8 /
51.3, Empire Climb 83.1 / 69.3 / 68.0, Vineta K 48.1 / 41.8 / 39.4. The whole frame goes a little
darker, further from the original, because the vignette removes light from a frame that is
already dark elsewhere (`tone-2048-grid`, below); that is the cost of a term the original has,
not a reason to leave it out.

### Not measured

- Vita3K was **not run** for a poke probe (`--ram-poke 0x8151c6b0=0:f32` would render the
  original with and without the vignette on one scene): this machine has no Vita3K profile
  (no `ux0`, no `apps.dat`). The tint and strength are the static seeds; a `--ram-read` of
  `0x816aef08..0x816aef14` and `0x8151c6b0` is the check that would watch them.
- Which way the Vita's clip-space `+y` points is not read; the mesh's orientation is taken from the
  pairs (the dark side is the top) and from `uv`'s `1 - (y / 2 + 0.5)`.
- The screen tint (`screenTintColour`, `FUN_813b1d28`) and `alphaTex` stay open.

### Omega: checked, differs

Omega registers `wo_composite_nocc_fp` and runs its own tone map, whose vignette is two disc keys
(`Vignette.Inner radius`, `Vignette.Outer radius intensity`, `ps4-omega-eu/tonemap.md`) over a
radial law of its own; 2048's constants are code, not disc keys, and Omega's file has no mesh. Ours
runs 2048's chain only on 2048 (`Looks::wo_composite`), so no Omega frame moves.

## Live evidence (Vita3K v0.2.1 4095, 960x544)

**The setters carry the circuit's own file.** `--ram-read` on the countdown of
three races, every grab of the burst:

| Pair | Scale | Max | Speed | BloomFactor | Gate | E | Branch flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Queens Mall (`mall`) | 0.4 | 12 | 0.1 | 0.2 | 0.2 | 1.15 | 0, 0 |
| Empire Climb (`tower`) | 0.5 | 12 | 0.1 | 0.3 | 0.1 | 1.15 | 0, 0 |
| Vineta K (HD Campaign) | 0.5 | 8 | 0.05 | 0.2 | 0.3 | 1.15 | 0, 0 |

Each equals the circuit's `track.EnvSettings`. Vineta K reads the **DLC1
pack's** copy (`dlc1.psarc`: 0.5 / 8 / 0.05, gate 0.3), not the copy inside
`data.psarc` (0.6 / 12 / 0.05, gate 0.2), so a loader must take the pack's.

**Landed 2026-10-26: the light and fog readers take the pack's copy too**
(`read_circuit_envsettings`, one lookup shared with the exposure keys). The pack
and base copies differ, on four circuits (both directions each): Vineta K sun
diffuse (1.0, 0.831, 0.486) base vs (2.0, 1.663, 0.965) pack; Anulpha Pass sun
(0.151, 1.035, 1.1) vs (0.173, 1.218, 1.3); Moa Therma sun (1.5, 1.406, 0.982)
vs (3.0, 2.776, 1.953); Chenghou Project ambient (0.706, 0.231, 0) vs (0.176,
0.220, 0.412), sun equal. Fog colour is equal in both copies everywhere.
`vita_2048_lighting_ground_truth::a_dlc_circuit_lights_and_fogs_with_the_packs_copy`
asserts the loaded sun and ambient equal the pack's. Vineta K, 1280x720, tick
240: frame mean grey 82.5 to 84.3, road box (440,480)-(840,640) 88.2 to 94.0.
Wipeout HD's own Vineta K authors sun (4.0, 2.26, 1.08), ambient 0.4 (a different
key spelling and scale); the pack's copy is the nearer of the two in
every channel but equals neither. **Omega: checked, not applicable.** Omega's
title record lists no add-on pack (`add_ons: &[]`), so the preference is inert
there; its patch archives already win a name through `Archives::read_name`'s own
search order (patch first), which `read_every_name` lists in the same order. No Vita3K frame was taken.

**E is the composite's exposure, and the law fits.** With
`--ram-poke 0x815cb93c=0:f32` (the schema's BloomFactor, so the bloom term is
0) and E poked from 1.15 to 2.3 halfway through one burst on Queens Mall, the
picture brightens at the switch (centre crop mean grey 55.2 to 94.3). The grey
quantiles of four frames either side, against the law's prediction for a grey
pixel (`y = e / sqrt(1 + e)`):

| Quantile | E = 1.15 | E = 2.3 drawn | law | `x2` linear |
| --- | --- | --- | --- | --- |
| 0.10 | 0.075 | 0.148 | 0.144 | 0.149 |
| 0.25 | 0.124 | 0.238 | 0.235 | 0.248 |
| 0.50 | 0.166 | 0.314 | 0.309 | 0.332 |
| 0.75 | 0.218 | 0.403 | 0.399 | 0.437 |
| 0.90 | 0.367 | 0.630 | 0.643 | 0.735 |
| 0.97 | 0.580 | 0.818 | 0.969 | 1.000 |

Within 0.013 to the 90th percentile. Above it the brightest pixels are the
coloured hull, where the grey form does not hold (`L` couples the channels) and
the output clamps, so the top two rows test nothing.

**Vita3K never adapts E; the readback does drive it.** E reads 1.15 on every
grab of every race above, the adapt flag reads 1, and the gate target's memory
(`0x816aef1c`: 240x136, stride 256, data pointer at `+0x10`) reads all zero:
Vita3K does not write the GPU's render into guest memory, with
`disable-surface-sync` either way. `--ram-fill '[0x816aef2c]+3=0x40:4:34816@20'`
writes alpha 64 into every pixel from the 20th grab on, and E then falls from
1.15 to 0.843, 0.662, 0.539, 0.474, ... and settles at 0.4000 = `Scale / (64 /
64)`: the 1/64 scale, the `Scale / a` target and `Speed` per update are
measured. The update count between grabs (about 5 per 0.17 s) puts the update
at once per rendered frame at about 30 per second on Vita3K. The gate pass
renders into the same surface the CPU reads (`FUN_812f6052(..., 0x816aef1c, 0)`
right before the gate's programs are bound in `PostFx_RenderChain`), which is
the static half of "on hardware the readback holds the gate's alpha".

So **every 2048 original-side frame captured on Vita3K is drawn at E = 1.15**,
the constant `Exposure_Set` resets to, not at the adapted value a Vita would
reach.

## Where a Vita settles: `E -> ExposureScale / mean luminance` (2026-10-26, `2048-exposure-wire`)

The link the first reading left open (which channel byte 3 of the gate surface
is, and the range the 1/64 assumes) is closed, and the earlier estimate of a
frame mean of 23 to 29 on every circuit was a misread alpha:

- **The gate writes `alpha = Lg * 0.25`, not `Lg`.** The alpha lane of
  `wo_bloom_gate_fp` (Vita3K's SPIR-V of the container, walked by hand as the
  composite was) multiplies the taps' mean luminance by the half literal
  `0x3400`, 0.25.
- **Byte 3 is that alpha.** `PostFx_CreateTargets` creates the three quarter
  targets with the format `sceGxmColorSurfaceGetFormat` (NID `0xF3C1C6C6`)
  returns for the current display buffer (`FUN_812ede9c`, a copy of one of the
  three buffers `FUN_812f5b98` returns at `0x81978fb8`). `FUN_812f51fa` creates
  those three with format `0` (`U8U8U8U8_ABGR`, A8B8G8R8) at 960x544 and hands
  them to `sceDisplaySetFrameBuf` (`SceDisplayUser_7A410B64`) with pixel format
  `0`, A8B8G8R8. So the gate target is 8 bits a channel, 4 bytes a pixel, red
  in byte 0 and alpha in byte 3, and the readback's `vld1.8` of byte `+3` every
  16 bytes reads the alpha of every fourth pixel. Only the scene surface
  (`0x816ae340`, created in `FUN_81035b10`) has the float format `0x41000000`,
  `U2F10F10F10`.
- **So `a = round(255 * Lg / 4) / 64`, `255/256` of the mean gate luminance**,
  and `E` settles at `ExposureScale / mean Lg`: a Reinhard key value, 0.4 to 0.6
  on every raced circuit. On the three reference frames' inverted luminance
  (0.24, 0.39, 0.16) that is about 1.7, 1.3 and 3.1: a Vita draws these frames
  brighter than Vita3K does, Vineta K most of all.

The `--ram-fill` measurement above is consistent with it: alpha byte 64 is
`a = 1`, and `E` settled at `Scale`.

**Confidence for the closed link as a whole: 75.** Every step is static. The
gate's 0.25 is read off the program; the A8B8G8R8 step is read off the format
call and the display buffers' creation (`FUN_812f51fa`'s format argument and
the `sceDisplaySetFrameBuf` pixel format), **not watched**: Vita3K never writes
the gate target back, so no live byte of it was ever seen, and a real Vita
running the readback is the only capture that would.

The alternative considered before the link closed (the lead's option 2 of
2026-10-26): take `a` as the frame's plain mean luma, which estimated frame
means of 86 to 104 on the three pairs. Ours adapting, as wired, draws 85.7,
111.4 and 80.3 ("Ours" below).

## Ours (2026-10-26, `2048-exposure-wire`)

`oag_post::wo_composite` runs the gate, the two blurs, the composite and the
step on the GPU, over a scene target whose pipelines square a texel instead of
raising it to 2.2 (`oag_gpu::formats::SceneStore::Squared`). A title takes it
through `oag_title::Looks::wo_composite`, measured on 2048 and unread
everywhere else; the five keys are read by
`oag_raceplay`'s environment staging, from an add-on pack's copy of the file
before the base archive's (the setters read `dlc1.psarc`'s on Vineta K, the
table above). The race report says which copy and which values.

- **The play default adapts**, as a Vita does: the link above is closed, so
  2048's own law is used and not Omega's `Tonemap.*` value. Origin: measured.
- **`oag-game --hold-exposure E` holds it** (`Options::exposure_hold`), a
  comparison switch and never the play default. The compare tooling passes
  `--hold-exposure 1.15` for every 2048 pair, because Vita3K draws at 1.15, and
  the page labels each such pair "exposure held at 1.15"
  ([compare-page.md](../../../tools/compare-page.md)).
- **Chosen, not the disc's**: the first frame starts settled on its own
  readback instead of walking from 1.15; the step runs once per rendered frame
  with no readback latency (60 a second here, about 30 on Vita3K); the scene
  target is `Rgba16Float` rather than `U2F10F10F10`; the screen tint is left out
  (tint 0) and `alphaTex` reads as empty, both unread; a split screen runs one
  chain per view with its own exposure and its own vignette over its rectangle.
  **The vignette is the disc's** since 2026-11-11 ("The vignette" above): the
  composite is drawn as the original's mesh.
- **Which of 2048's other programs square their texel is unread.** The
  lightmapped track program does (above); ours squares every texel a pipeline
  built for the scene's store decodes (meshes, clouds, exhaust, beams, ghosts),
  and the particle, mist, flash and weapon-quad pipelines keep their 2.2.

Frame means on the three pairs, original against ours (`-alpha off`):

| Pair | Original (Vita3K, E = 1.15) | Ours before | Ours, E held at 1.15 | Ours adapting |
| --- | --- | --- | --- | --- |
| Queens Mall | 60.2 | 106.2 | 54.9 | 85.7 |
| Empire Climb | 83.1 | 107.7 | 61.4 | 111.4 |
| Vineta K | 48.1 | 80.1 | 36.7 | 80.3 |

The mean grey difference falls from 55.4 to 26.3 (Queens Mall), 45.7 to 39.0
(Empire Climb) and 41.4 to 23.4 (Vineta K). Held at 1.15 ours is now darker
than the original, by 5 to 22 grey levels: Empire Climb's track and walls still
draw in flat colours where the original's are lit and textured, and Vineta K's
sky draws darker than the original's.

## Omega: checked, differs

Omega's `.EnvSettings` carry no `Lighting.ExposureScale/Max/Speed`,
`BloomFactor` or `BloomGate` key, and its executable has none of those strings
(`search_strings` on `/omega/eboot-ps4-omega-eu.bin` finds only the `pal_`
tone map's `v0Phi_v1Phi_ExposureMin_ExposureMax`). Omega replaced this chain
with the adaptive cubic tone map of `Tonemap.*`
([tonemap.md](../ps4-omega-eu/tonemap.md)), which ours already runs, and
keeps `wo_composite_nocc_fp` registered and unused. Its frames already match
the original's brightness: frame means original against ours (alpha off)
96.3 / 90.9 on Anulpha Pass and 79.3 / 78.1 on Vineta K.

Re-checked when the chain was wired (2026-10-26, `2048-exposure-wire`): still
**checked, differs**, nothing forward-ported. `Looks::wo_composite` is unread on
Omega, so an Omega race builds only its `Tonemap` chain, and the Omega pairs on
Vineta K and Anulpha Pass draw 0 differing pixels against the binary before the
change (as do HD's two, Pulse PSP and PS2 and Pure).

## Where ours is darker than the original: not a frame term (2026-11-09, `tone-2048-grid`)

Claim tested: one frame-wide term of ours (fog, bloom, exposure, sky or lighting scale) accounts for most of the dark bias
on the three 2048 grid pairs, and correcting it closes half of Empire Climb's bottom-third bias. **Result: false.**
Confidence 75 that no single frame-wide term does it (ours is deterministic and was repeated; each original is one capture,
`twin: no second capture of this pair`); 40 for what the rest is (below). Read-only measure, ours rendered headless on SwiftShader (the only Vulkan driver on that machine; the stored RADV
render and the SwiftShader one of the same binary differ by 0.44 / 255 mean on Empire Climb, and the three baselines
agree to 0.3 luma and 0.1 M impact). Every toggle is a scratch environment multiplier on the loaded values (`Lighting.*`
read in `envsettings_exposure`, `envsettings_light`, `envsettings_fog`), built off the lane's tree and not landed.
A repeat render of one toggle (ambient x3, Empire Climb) is byte-identical.

**Falsifier written before the renders.** A frame-wide term darkens hull and road together, so a toggle that closes the
bottom third only by pushing the road past the original, or that leaves the pair's impact flat, is not the term.

**Method.** Luma bias is ours minus original (`0.2126 R + 0.7152 G + 0.0722 B`, unblurred, 8-bit), per third of the
960x544 frame. The bottom third splits into the hull box `(290,240)-(680,544)` (the craft and the road under it, minus the
HUD), the HUD boxes, and the scenery that is left (37 %, 28 % and 35 % of the band's pixels). Impact is the scout's
measure (Gaussian sigma 7, 32 blocks across, hot at mean error 6, 8-connected, px x err), re-implemented, so the
baselines differ a little from the scout's 9.67 / 7.33 / 5.22 M: 9.78 / 7.38 / 6.33 M on the same pair files (Vineta K
is 0.9 M off the scout's and was not reconciled; every change below is against this method's own baseline).

| Toggle (Empire Climb) | top | mid | bottom | bottom hull box | bottom scenery | impact (change) |
| --- | --- | --- | --- | --- | --- | --- |
| baseline | -11.2 | -12.2 | -16.5 | -43.1 | -2.0 | 9.78 M |
| E 1.4 (`--hold-exposure`) | -5.0 | -0.4 | -0.9 | -28.1 | +16.7 | 9.28 M (-5 %) |
| BloomFactor x0 / x3 | -15.7 / -4.7 | -23.2 / +4.2 | -35.0 / +9.4 | -62.5 / -17.5 | -21.8 / +27.8 | 13.81 M (+41 %) / 10.77 M (+10 %) |
| BloomGate 0 | -8.2 | -6.3 | -8.5 | -34.9 | +7.4 | 8.85 M (-10 %) |
| fog off / density x2 | -11.3 / -10.7 | -12.2 / -12.3 | -16.5 / -16.5 | -43.1 | -2.0 | 9.84 M / 9.73 M (+-1 %) |
| sun diffuse x2 | -8.8 | +26.8 | +36.7 | +8.9 | +64.4 | 17.08 M (+75 %) |
| lightmap scale x1.5 | -5.3 | -0.4 | -4.4 | -36.3 | +14.9 | 9.88 M (+1 %) |
| ambient x2 / x3 / x4 / x6 | -10.8 / -10.3 / -9.8 / -8.8 | -11.0 / -10.0 / -9.2 / -8.0 | -10.6 / -6.3 / -3.3 / +0.8 | -27.3 / -15.7 / -7.5 / +3.4 | -2.0 | 8.94 / 8.81 / 8.94 / 9.37 M (-9 / -10 / -9 / -4 %) |

Queens Mall (baseline -4.5 / -5.7 / -8.3, hull box -24.8, impact 7.38 M): E 1.4 +23 % impact; bloom x3 0 %, gate 0 +9 %,
lightmap x1.5 +27 %, fog off 0 %; **ambient x2 / x3 / x4 / x6: bottom -5.3 / -2.6 / -0.2 / +3.7, hull box -16.9 / -9.5 /
-3.0 / +7.3, impact 6.77 / 6.32 / 6.07 / 6.16 M (-8 / -14 / -18 / -17 %)**, scenery unchanged.
Vineta K (baseline -1.3 / -10.1 / -6.9, hull box -13.4, impact 6.33 M): E 1.4 +11 %, bloom x3 +14 %, gate 0 +3 %,
lightmap x1.5 +6 %, fog off 0 %; ambient x2 / x3 / x4 / x6: bottom -5.3 / -3.7 / -1.9 / +1.5, hull box -9.8 / -6.3 / -2.5 /
+5.0, impact 5.89 / 5.73 / 6.15 / 7.74 M (-7 / -9 / -3 / +22 %), but the top third goes -1.3 to +3.6 / +8.4 / +12.2 / +19.1.

What it says:

- **The bottom third's bias is the craft's box, not the frame.** On Empire Climb the hull box is 37 % of the band at -43.1,
  which is -15.9 of the band's -16.5; Queens Mall's is -9.2 of -8.3; Vineta K's -5.0 of -6.9. The scenery of the band is
  -2.0 / +2.4 / -2.0. The box holds the craft and the road round it, and the original's frame has more of both: its hull
  is 1.31 x wider and 1.35 x taller than ours (the pair's recorded pose difference), a white ring under it (not the HUD's
  Pilot Assist bracket, which both sides draw bottom right; the ring is unidentified and ours draws none) and a glow on the
  road. Yellow paint inside `(290,240)-(680,470)` (R > G > B, R - B > 60, R > 90), baseline ours against the original:
  Queens Mall 8,320 px at median luma 130.1 against 8,803 px at 156.1 (**ours' paint 17 % darker, same area**: a light
  term); Empire Climb 10,514 px at 152.6 against 30,354 px at 143.1 (the original's mask also catches its peach road, so
  the paint is **not** darker there; the box gap is the road's glow, the ring and the larger pose); Vineta K's craft is not
  yellow and the mask finds none. Ambient x4 lifts Queens Mall's paint to 158.7 (matches), and lifts Empire Climb's to 192.9
  and its near-white pixels from 246 to 5,807 against the original's 1,088 (overshoots). So a craft light term is real on
  Queens Mall and not the story on Empire Climb.
- **The top and middle thirds' scenery is 10 to 15 luma dark on Empire Climb and Vineta K, and no one term owns it.** The
  original's top third carries the sun's lens-flare streak and a bright haze round it that ours has neither of
  ([lens-flare.md](../../../rendering/lens-flare.md): 2048 "checked, differs, not wired"). Lightmap scale x1.5 closes the middle
  scenery (-11.7 to -0.5) but pushes the bottom scenery to +14.9; exposure x1.2 does the same sort of thing.
- **Fog is nothing here.** Empire Climb's density is 0.001, so `exp(-(0.001 d)^2)` is above 0.9 to 300 units: off or x2
  moves no band by more than 0.6 luma and impact by 1 %. The inherited curve is not the cause of any dark bias.
- **Bloom is already large in ours and is not what is missing.** Zeroing it darkens by 15 to 35 luma on Empire Climb; x3
  overshoots the road by 28. Its gate at 0 helps Empire Climb (-10 %) and hurts Queens Mall (+9 %).
- **Exposure is not a correction.** E = 1.15 is what the original draws at (read live, above). The flag sweep is a proxy for an
  upstream light gain, and it closes the band means by overshooting the road (+16.7 at E 1.4) with impact -5 %, +23 %, +11 %.
- **A display gain recovers nothing worth having.** Scaling ours' output by a constant g (1.0 to 1.4): Empire Climb's best is
  g 1.1 at 9.03 M (-8 %); Queens Mall and Vineta K rise at every g. A per-band per-channel affine fit leaves 9.06 / 8.53 /
  6.59 M. So the pairs' impact is spatial (texture, light pattern, the craft, the flare), and the tone rows of the ranking
  cannot be closed by a tone term.
- **The one toggle that lowers impact on all three is the ambient light, as a crude brightener of the craft.** Ambient x3
  closes 62 / 69 / 46 % of the bottom third on Empire Climb / Queens Mall / Vineta K and lowers impact 10 / 14 / 9 %, with
  the bottom scenery unchanged (Empire Climb, Queens Mall); the hull box reaches zero bias at about x4.5 to x5.5. That is a
  fit of the box's mean, not a law: it overshoots Empire Climb's paint and white panels (above), and on Vineta K the same
  multiplier also lifts the top third (+9.7), so ambient is read by more than the craft. A factor of 3 to 5 on three
  circuits with different authored ambients (Empire Climb `0.31 0.47 0.70`) does not read as "ambient authored too low".
  Ours lights a no-lightmap material (the craft's included, its program unread) as the `constantAmbientColour` family,
  `(ambient + sunDiffuse * max(0, N.L)) * D^2` ([2048-material-params.md](../../../formats/2048-material-params.md), "The
  ambient family"). **Chosen probe, not a law; no score.** Texel decode of x instead of x^2 on every draw overshoots
  everything (+31.6 hull box, +87.8 scenery) and does not isolate the craft.

**Lineage (Omega).** The fog, bloom, exposure and vignette terms are **checked, differs** (Omega runs `Tonemap.*` and its own
`Bloom.*` chain, see above). The craft's light is **not checkable here**: neither title's craft pixel program was read in this
lane (2048's is unread; Omega's `constantAmbientColour` appears on 4,337 pixel shaders and in the shield program
([lightmap-prelit.md](../ps4-omega-eu/lightmap-prelit.md)), which is not the craft's), and shadPS4 race frames are untrusted, so
there is no Omega pair to fit a factor against. Recipe: `cargo run -p oag-rcs --example gcn_dump` on an Omega craft material.

## Open

- ~~The vignette~~: closed 2026-11-11, "The vignette" above. Left over: a Vita3K
  `--ram-read` of its constants, and the clip-space `+y` direction.
- The screen tint's source (`FUN_813b1d28` of the vector at the composite's
  bind) and `accumFactor`'s; what `alphaTex` holds in a race.
- When `FUN_81071300` resets E to 1.15 (race load, Retry, or both).
- `bloomScaleBias` and `mainScaleBias` (ours maps the bloom over the scene's
  own rectangle); `wo_blur_ds_fp`, registered and not in the race chain.
- The Zone race's composites (`0x816a4d80`, `0x816a4d84`).
- **The light and fog keys of a DLC circuit still come from the base copy.**
  Only these five keys prefer the pack's copy (`envsettings_exposure`); on
  Vineta K the pack's copy authors a sun diffuse twice the base copy's, and
  ours lights it with the base's.
- Every number above on a real Vita: the readback is the one step Vita3K does
  not perform.
- **The original's craft box is brighter than ours by more than the authored ambient explains** (2026-11-09, section above):
  identify the white ring under the original's craft and match the pose first; on Queens Mall the craft's paint is also 17 % dark,
  so read the 2048 craft's pixel program (`constantAmbientColour`, texel decode, any reflection or rim term) before any lighting
  multiplier is wired.
