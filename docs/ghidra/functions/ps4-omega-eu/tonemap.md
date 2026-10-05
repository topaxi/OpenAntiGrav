# The tone map: `Tonemap.*` drives an adaptive cubic curve applied in the MSAA resolve

2026-10-05, `omega-tonemap`. `eboot.bin` (WipEout: Omega Collection, PS4,
`CUSA05670`, EU), `x86:LE:64:default`, image base `0x01000000`, Ghidra program
`/omega/eboot-ps4-omega-eu.bin`. **Static evidence only**: no PS4 emulator
exists in this project's toolchain, so nothing here was watched running, and
every score is capped at 84 by the
[rubric](../../../reverse-engineering/confidence-rubric.md) ("decompilation only,
consistent call sites"). The shader laws were read off GCN microcode embedded in
the executable, with the scratch disassembler
[`lightmap-prelit.md`](lightmap-prelit.md) describes, and the longest one was
also run through a small one-lane emulator and checked against a closed form.

This closes the question [`lightmap-prelit.md`](lightmap-prelit.md) left open
("The `Tonemap.*` block: parsed, with no consumer found"). **The consumer is
`FUN_01620980`, and it reads the block through a pointer** (`param_1 + 0x1e0`),
which is why the earlier absolute-address search over `0x01e3c2b0..0x01e3c2ff`
found nothing.

## The law

```text
per frame:
  L      = mean Rec.601 luma of the scene               (0.299, 0.587, 0.114; no log)
  target = min(1000, (L + sum of the previous N-1 frames' L) / N)   N = round(time * 60)
  LAvg   = P + clamp(target - P, -response/60, +response/60)        P = last frame's LAvg
  E      = clamp((lumA + lumB * LAvg) / max(LAvg, 1e-4), expMin, expMax) * cL
  t1     = max(srcEndA + srcEndB * LAvg, 0.01)          t0 = 0
  s0 = 0, s1 = 1.0 (SDR output) or 40.0 (HDR output)
  y(x)   = cubic Hermite with y(t0)=s0, y(t1)=s1,
           y'(t0) = k * tan(startAngle + pi/4), y'(t1) = k * tan(endAngle + pi/4),
           k = (s1 - s0) / (t1 - t0), angles in degrees in the file
per pixel, per colour channel, per MSAA sample:
  out    = average over samples of y(clamp(E * c, t0, t1))
```

`lumA/lumB`, `expMin/expMax`, `response`, `time`, `srcEndA/srcEndB` and the two
angles are the `.EnvSettings` keys `Tonemap.Luminance a/b-coefficient`,
`Tonemap.Exposure minimum/maximum`, `Tonemap.Exposure response`,
`Tonemap.Exposure time`, `Tonemap.Source color end a/b-coefficient`,
`Tonemap.Start angle` and `Tonemap.End angle`. `cL` is the player's brightness
setting (1.0 at its middle). Confidence for the law as a whole: **76**. Its
parts are scored below.

## The CPU side

| Step | Address | What |
| --- | --- | --- |
| Registration | `EnvSettings_RegisterKeys` (`FUN_015c1f20`) | Registers the ten `Tonemap.*` keys at object offsets `+0x1e0..+0x204` and the `TonemapHDR.*` twins at `+0x208..+0x22c` ([`lightmap-prelit.md`](lightmap-prelit.md)). |
| Consumer | `FUN_01620980` | `puVar10 = param_1 + 0x1e0`, or `+0x208` when `DAT_02133c48` (HDR video out) is set. Writes the tone-map object `DAT_0212a7d8`: `+0x98/+0x9c` = exposure min/max; `+0x80` = lum a, `+0x84` = lum b, `+0x88` = response * `0.016666668`; `+0x90/+0x94` = start/end angle * `0.017453292`; `+0xa8/+0xac` = source colour end a/b; `+0xa0/+0xa4/+0xb0/+0xb4/+0xbc` = 0; `+0xb8` = 1.0, or 40.0 under HDR. Then builds a vector of `round(time * 60)` weights, all 1.0 (`uVar6 = time * 60.0 + 0.5`, minimum 1), and passes it to `FUN_0178bfb0`. |
| Brightness | `FUN_0155e450`, write at `0155f0a5`; reset in `FUN_01561f20` | `+0x8c` (`cL`) = `(1 - t) * 1.75 + t * 0.25`, `t = clamp(1 - v * 0.01, 0, 1)` for a menu value `v`. The reset path writes 1.0. |
| History weights | `FUN_0178bfb0` | Stores the weights at `+0x1f8..`, sizes a ring of `N - 1` history slots at `+0x1d8..`, and writes `1 / sum(weights)` to `+0xc4`. |
| Object | `FUN_0178a160` | Creates the tone-map object (`0x210` bytes) when the target's flag `0x10` is set: a 256x256 `R16F` luminance chain (format `0x204702`, 8 levels halving from 256), and two 3x1 `RGBA32F` coefficient textures (`0xfac70e`) used ping-pong. Looks up the five constant names `a0L_b0L_dLAvgMax_cL`, `v0Phi_v1Phi_ExposureMin_ExposureMax`, `a1t0_b1t0_a2t1_b2t1`, `a3s0_b3s0_a4s1_b4s1`, `c0LAvg1_iLAvg` in `pal_ToneMapCoefficientsFilter_fp`. The main scene target is created with flags `0x276` (`0x2f6`), `0x10` included (`FUN_01621650`, call at its end; `FUN_016134d0` line-equivalent call). |
| Per frame | `FUN_0178c490` | Reads back the previous coefficient texture's luminance, clamps it to `[0, 1000]` and stores it in the ring; computes `c0LAvg1 = w0 / sum(w)` and `iLAvg = sum(w_k * hist_k) / sum(w)` and writes both into the coefficient pass's constants; runs the luminance chain and the coefficient pass. |
| Resolve | `FUN_017864a0` | For a tone-mapped target that is not checkerboarded: a source with fmask compression (`CB_COLOR_INFO` bit 14, `(+0x11 & 0x40)`) binds the curve resolve `DAT_01947e08` (blob `0x196aa70`) and the coefficient texture as a `0x20`-byte constant buffer (texels 0 and 1); one without binds the plain box resolve `DAT_01947c28` (blob `0x196a6a0`). |

**Why the curved resolve is the live one on a base PS4.** `FUN_0178a160` pairs
each resolve destination with a surface created with flags `0x15` when the sizes
match, and in `FUN_017968a0` flag `0x10` means multisampled. Its sample-count
exponent is `((DAT_0212f90e & 1) == 0) + 1`, overridden for a surface with bit 0
set (as `0x15` has) by `((DAT_0212f90c & 1) == 0) * 2`: **4x on a base PS4**,
where `DAT_0212f90c` is 0. `FUN_017a4eb0` sets `DAT_0212f90c/d/e` when it switches
on checkerboard mode (and builds a 1920x2160 target, the PS4 Pro's half-width 4K),
which reads as a single sample there; that path's sample setup is not read
further. Checkerboard frames take
the compute resolve `0x19685a0` instead, whose constants are the same
`m_abcd`/`m_LAvgt0t1Exposure` pair (not read further). Confidence that a base-PS4
race applies the curve in the resolve: **72** (the fmask bit was not watched
being set).

## The shader side

All blobs are in the executable's data segment and are registered as
`(name, blob, size)` records on a list headed at `DAT_0212f900`. The `pal_` set
is registered by `ToneMap_RegisterPrograms` (`FUN_0178b970`); the composite's
by `FUN_01610df0`; the older `wo_*` set by `FUN_01623500`. Constant-buffer
member offsets come from each blob's reflection records
`(type, offset, size, 0, -1, 0, -1, name, typename)` with self-relative name
offsets.

**`pal_LuminanceFilter_fp`** (blob `0x01965120`): `image_sample_lz` of the
source, then `v_mul_f32 0.299`, `v_madmk_f32 0.587`, `v_madmk_f32 0.114`, exported
as fp16. No `v_log_f32`: the average is of linear luma, not a log average. A
second blob (`0x01962660`) is a plain bilinear copy, the shape of the 2x
downsample steps. Which level of the 256x256 chain the coefficient pass samples
is not read (it samples `(0.5, 0.5)` of a view `FUN_0178c490` binds); a mean of
the whole frame is the plain reading, confidence **60**.

**`pal_ToneMapCoefficientsFilter_fp`** (blob `0x019676c0`, constants as named
above, 18 dwords) writes a 3x1 target, one texel per `In.m_Position.x`:

```text
LAvg   = P + clamp(min(1000, iLAvg + c0LAvg1 * L) - min(1000, P), -dLAvgMax, dLAvgMax)
         (P = texel 1 .x of the previous coefficient texture, image_load_mip at (1, 0))
t0 = a1t0 + b1t0 * LAvg            t1 = max(a2t1 + b2t1 * LAvg, t0 + 0.01)
s0 = max(0, a3s0 + b3s0 * LAvg)    s1 = max(a4s1 + b4s1 * LAvg, s0 + 0.01)
slopes from v_sin_f32 / v_cos_f32 of v0Phi, v1Phi (input pre-scaled by 1/(2 pi))
E  = clamp((a0L + b0L * LAvg) / max(LAvg, 1e-4), ExposureMin, ExposureMax) * cL
texel 0 = (a, b, c, d)             the cubic a + b x + c x^2 + d x^3
texel 1 = (LAvg, t0, t1, E)
```

Checked by emulation, not only by reading: the 123-instruction body run on one
lane for six random parameter sets reproduces texel 0 as exactly the Hermite
cubic through `(t0, s0)` and `(t1, s1)` with end slopes
`k * tan(phi + pi/4)`, and texel 1 as `(LAvg, t0, t1, E)`, to five decimals.
The `tan(phi + pi/4)` form is the shader's `(cos + sin) / (cos - sin)`, with a
`+-1e-6` guard on the denominator. Confidence **80**.

**The curve resolve** (blob `0x0196aa70`, `m_abcd` at dw0, `m_LAvgt0t1Exposure`
at dw4): per sample `x = clamp(E * c, t0, t1)` (`v_mul_f32 s15`, `v_min_f32 s14`,
`v_max_f32 s13`), `y = a + x (b + x (c + d x))`, summed over the fragments the
fmask names and divided by their count, exported fp16. **Per colour channel**,
not on luminance. Confidence **80** for the shader, 72 for it being the one a
base-PS4 race runs (above).

**What runs after it.** The composite `FUN_016110f0` binds is the blob at
`0x01956d50` (constants `BloomScale_abVignette`, `ScreenTint_Aspect`, a motion
blur block, `wh_whInv`, `uvOrigo`; textures `FrameTexture`,
`LowResAdditiveTexture`, `DistortionTexture`, `BloomTexture`):

```text
out = (Frame + LowResAdditive + BloomScale * Bloom + ScreenTint.rgb)
      * sat(abVignette.a + abVignette.b * |uv - 0.5|^2)
```

with no exposure and no curve (`exp` of `v_cvt_pkrtz_f16_f32`, alpha 1.0). So
**bloom is added after the curve**, in the curve's output range. Confidence
**70** that this blob is the race's composite (it is the one bound with the
`ScreenTint_Aspect` handle `FUN_016110f0` looks up; the selection between it and
its four siblings was not traced).

## Not live, recorded so nobody chases them

- **`wo_composite_nocc_fp`** (`0x01959910`): `c = alpha.rgb + (1 - alpha.a) * main`,
  `c *= exposure` (one float from a buffer named `ExposureTexture`),
  `L = dot(c, luminanceFactor)`, `out = c * L / (1 + L) + screenTint + bloomFactor.x * bloom`.
  `wo_composite_notonemap_fp` (`0x01959d80`): `out = main * (bloomFactor.y * main
  + bloomFactor.z) + bloomFactor.x * bloom + screenTint`. Both are registered by
  `FUN_01623500` with the rest of the `wo_*` set, whose CPU-side default
  `luminanceFactor` is `(0.3, 0.59, 0.11)` (`0x01fc8090`); **no code reads either
  record** outside its registrar and a teardown, so they are taken as Wipeout HD's
  composites carried forward and unused. A third, with a `colourCubeTexture`
  lookup, sits unregistered at `0x019593a0`.
- **The `pal` exposure shader** at `0x01968180` (`m_cExposure / max(L, 0.001)`
  clamped to `[m_ExposureMin, m_ExposureMax]`) is listed in the `pal` program
  table at `0x01947750` and referenced by no code.

## Wired

`oag_render::post::omega_tonemap` (2026-10-05) runs this law on an Omega race, with
the choices listed in its module docs and in
[`omega-status.md`](../../../formats/omega-status.md). Its test
`the_law_reproduces_the_original_coefficient_shader` holds the Rust law to the
texels the emulated coefficient shader produced.

## Names

[`names.tsv`](names.tsv) rows added by this page:

- `0x01620980` `ToneMap_ApplyEnvSettings` - 80. Reads the ten `Tonemap.*`
  fields (or `TonemapHDR.*`) through the environment object and writes them,
  scaled, into the slots whose layout is the coefficient shader's own constant
  names: two independent sites agree on every field.
- `0x0178a160` `PostTarget_Create` - 66. Builds a post-processing target
  wrapper, its multisampled companions and, under flag `0x10`, the tone-map
  object. Role read from what it allocates; the flag meanings beyond `0x08`,
  `0x10` and `0x40` are not.
- `0x0178bfb0` `ToneMap_SetHistoryWeights` - 74.
- `0x0178c490` `ToneMap_UpdateCoefficients` - 72.
- `0x017864a0` `PostTarget_Resolve` - 70.
- `0x0178b970` `ToneMap_RegisterPrograms` - 82. Registers the four `pal_`
  programs by name.
- `0x01610df0` `Composite_RegisterPrograms` - 72. Registers the six programs
  `FUN_016110f0` binds; their names were not resolved to strings here.
- `0x017a4eb0` `Display_SetCheckerboard` - 74.
- `0x0212a7d8` (data) `ToneMap_Instance` - 80.

## Open

- **The output encoding.** Whether the display buffer is an sRGB format (the
  curve's [0, 1] then goes through a hardware sRGB encode) or UNORM was not read;
  `FUN_01629ce0` copies the composite to scanout (`FUN_0122d760` under SDR, a
  `pal` shader with a `250.0` constant under HDR).
- Which level of the luminance chain the coefficient pass samples, and what
  image the chain starts from.
- The checkerboard compute resolve `0x19685a0`, beyond its constant names.
- Every number above on a running PS4.
