# Exposure and the race composite: `Lighting.Exposure*` drives `wo_composite_nocc_fp`

2026-10-26, `2048-exposure`. `eboot.elf` (WipEout 2048, Vita, `PCSF00007` EU,
patch v1.04), Ghidra program `/2048/eboot-vita-2048-eu-v104.elf`. Static reading
plus live reads and writes of the guest's memory in Vita3K through
`scripts/vita-2048-pair.py --ram-read/--ram-poke/--ram-fill` (see
[compare-page.md](../../../tools/compare-page.md)).

This closes what [envsettings.md](../../../formats/envsettings.md) listed as
"authored and left unread" for 2048: `Lighting.ExposureScale`,
`Lighting.ExposureMax`, `Lighting.ExposureSpeed`, `Lighting.BloomFactor` and
`Lighting.BloomGate`. Ours reads none of them and draws 2048 with no post chain
at all, which is the subsystem the 2048 pairs' brightness gap sits in.

## The law

```text
once per rendered frame, when the adapt flag 0x8151c5bc is set:
  a      = mean over every 4th pixel of every 4th row of the quarter-res gate
           target's alpha byte, times 1/64            (0x3c800000)
  if a > 0.01:
    E   += (Scale / a - E) * Speed                     no dt, no lower clamp
    E    = min(E, Max)
gate pass (wo_bloom_gate_fp, into the quarter-res target 0x816aef1c):
  s      = sum of 4 taps of the scene                  (offset0..3)
  Lg     = dot(s, lum * 0.25)                          mean luminance of the taps
  rgb    = (E * clamp(Lg - Gate, 0, 1))^2 * s          alpha = Lg
composite (wo_composite_nocc_fp), per pixel:
  c      = alpha.rgb + (1 - alpha.a) * main.rgb
  e      = E * c
  L      = dot(e, lum)                                 lum = (0.3, 0.59, 0.11)
  out    = sqrt(e * L / (1 + L) + BloomFactor * bloom) * v + screenTint
```

`Scale`, `Max`, `Speed`, `BloomFactor` and `Gate` are the circuit's
`Lighting.ExposureScale`, `ExposureMax`, `ExposureSpeed`, `BloomFactor` and
`BloomGate`. `v` is a per-channel vignette factor `1 - t^2`, `t` the vertex
program's third output (`wo_composite_vp`, `vignetteTint`), not resolved here.
`main` is the scene, which 2048's lit programs write in a gamma-2 linear space:
a lightmapped track program decodes its diffuse texel by squaring it and
multiplies the raw lightmap scale-bias in (Vita3K's translation of the program,
read the same way as the composite below). The final `sqrt` is the matching
encode. Confidence for the composite as a whole: **85**; for the adaptation
step: **85**; for "hardware adapts where Vita3K does not": **75**. Parts below.

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
reach. Inferred from the law and the frames (not measured, no score): inverting
the composite on the three reference frames gives a mean scene luminance of
0.24, 0.39 and 0.16, so a Vita would settle at E = `Scale / (4 * 0.24)` and so
on, about 0.42, 0.32 and 0.78, darker than Vita3K draws. The estimate ignores
bloom, the vignette and the HUD-free full frame the gate really averages.

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

## Open

- The vignette: `wo_composite_vp`'s third output and the `vignetteTint`
  values `PostFx_RenderChain` packs (`local_348`/`local_344`).
- The screen tint's source (`FUN_813b1d28` of the vector at the composite's
  bind) and `accumFactor`'s.
- When `FUN_81071300` resets E to 1.15 (race load, Retry, or both).
- The blur passes' weights and `bloomScaleBias`.
- Every number above on a real Vita: the readback is the one step Vita3K does
  not perform.
