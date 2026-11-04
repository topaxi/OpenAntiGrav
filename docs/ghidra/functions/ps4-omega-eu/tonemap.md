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

## The output encode (2026-10-27, `omega-display-encode`)

**The curve's `[0, 1]` reaches the display as linear light, and the video out
encodes it.** On the copy branch below (composite and scanout formats equal),
nothing between the composite and the screen applies a curve on the GPU.
Confidence **72** that this is what a base PS4 does, and **80** that it is what
the reference frames show, because the emulator that captured them was read too
(still a static read, so under this page's cap of 84). No PS4 watched.

| Step | Address | What |
| --- | --- | --- |
| Scanout buffers | `FUN_016134d0`, call to `sceVideoOutSetBufferAttribute` | Pixel format `0x88060000` (`A2R10G10B10`, **not** the `_SRGB` variant `0x88000000`), or `0x88740000` (`A2R10G10B10_BT2020_PQ`) when `sceVideoOutGetDeviceCapabilityInfo_` reports HDR (that bit also sets `DAT_02133c48`, the HDR flag `ToneMap_ApplyEnvSettings` reads). Then `sceVideoOutColorSettingsSetGamma_(1.0)` and `sceVideoOutAdjustColor_`, the only call sites of either. |
| Scanout render targets | same function, two `FUN_01217e20(..., 0xf2e009, ...)` builds | Data format `0xf2e009`: surface format `0x09` (`2_10_10_10`), channel type `0x0` (**UNORM**; sRGB would be `0x9`), swizzle `0xf2e` (B, G, R, A). The same field split reads this page's other two formats correctly: `0x204702` is `R16F` and `0xfac70e` is `RGBA32F`. So the colour unit stores the values the shader exports, with no sRGB encode on write. |
| SDR copy to scanout | `FUN_01629ce0` to `FUN_0122d760` to `FUN_0122d670` | When the composite target is not the scanout buffer and the two formats match, `FUN_0122d670` draws one full-screen pass with the program at `0x0193f060` (empty on disc, filled at load). The blob pointer beside it, `0x0193f050`, is `0x017e39e0`. Its code is `image_load_mip` of `texture` at `position`, two `v_cvt_pkrtz_f16_f32`, one `exp mrt0` and `s_endpgm`, with **no arithmetic**: a copy. The pairing of `0x0193f050` with the handle is read from the layout (blob pointer, then handle 16 bytes on), confidence 65. When the formats differ, `FUN_01794290` picks a conversion program by the destination's format byte (not read). That the compared word (`+0x1c` of the second 32-byte block) is the format is inferred from that branch, and the composite target's own format is not read: `0xf2e009` appears as an immediate only in the scanout builds (four sites, all in `FUN_016134d0`). |
| HDR | same branch, `DAT_02133c48` set | A `pal` program with the single constant `250.0` (`0x437a0000`) writes the PQ buffer. With the curve's HDR end at `40.0` (`ToneMap_ApplyEnvSettings`), `40 * 250 = 10000`, PQ's ceiling in nits. This reads as "curve output 1.0 is 250 nits of linear light", which corroborates the linear reading (the HDR blob itself was not disassembled). |

**What the reference frames went through.** Every `omega/*` original was
captured on shadPS4 0.20. Its presenter (`vk_presenter.cpp`, `GetFrameViewFormat`)
views every `A2R10G10B10*` buffer through a UNORM image view, onto a UNORM
swapchain, and its post-process pass (`post_process.frag`) applies, for a
non-HDR frame, `a * pow(rgb, 1 / (2.4 + 1 - gamma)) - b` with `a = 1.055`,
`b = 0.055`, below `0.0031308` the line `12.92 * rgb / gamma`. `gamma` is what
the game passed to `sceVideoOutAdjustColor` (`video_out.cpp`), 1.0 here, so the
encode is exactly the piecewise sRGB curve. Read on the shadPS4 snapshot `a5d307ee7`
(2026-09-14). In it, that path is unchanged since December 2024 (`8a409d86d`).
The one later change, `473633c31` (2026-09-02), adds a decode for the `_SRGB`
10-bit format only, which this game does not use. The swapchain code's last
change is April 2026. The installed 0.20 binary was built on 2026-10-16, so it
is later than the snapshot read; the commits in between were not read. So the captured picture is `sRGB_OETF(composite)`. What a real
PS4's video out does with `0x88060000` is not watched. The SDK pairs this format
with an `_SRGB` twin, and the game sets a gamma for it, so a display-side
encode is the reading. That a real PS4 matches shadPS4 here is the 72 above.

**Wired.** `oag_post::omega_tonemap` now ends on this curve
(`law::scanout_encode`), in place of the `pow(1/2.2)` chosen there on
2026-10-05. The two differ only below about 20% grey. The test
`the_chain_draws_the_law_on_a_real_device` holds a dark channel to it, where
the two encodes are 8 levels apart.

**It is not what separates the two race pairs.** Measured with
`compare-rerender-ours` (whole-frame mean grey difference; the box figure is
the difference of the box means of grey):

| Pair | `pow(1/2.2)` | sRGB encode |
| --- | --- | --- |
| `vineta-k`, whole frame | 46.69 | 48.19 |
| `vineta-k`, road box (x 700-1300, y 420-560) | 64.2 (ours 95.4, original 31.2) | 62.5 (ours 93.6) |
| `vineta-k`, wall box (x 1100-1900, y 340-520) | 51.6 (ours 89.1, original 37.6) | 49.9 (ours 87.4) |
| `anulpha-pass`, whole frame | 38.92 | 41.04 |

The encode darkens what is already dark, and both pairs lose a little because
ours is darker than the original over most of each frame. A per-pixel transfer
curve (median original grey for each 16-level band of ours) shows why no display
curve can fix both pairs. Where ours reads 96-111, Vineta's original median is
65.5 and Anulpha's is 120.3, so a curve would have to pull the same value down on
one pair and up on the other. (The frames are not pixel-aligned, Anulpha's camera
pitch differs, so the medians are rough.) The 2026-10-27 identity probe fixed Vineta's boxes and made both
whole frames worse; that is the signature of a spatial term, not a curve. What
the frames show instead, at the same ship and grid slot. These are **leads, not
measured causes**:

- **Vineta K's sky** is blown out to white in the original, and a mid-grey
  cloud picture in ours. The original adds its bloom after the curve, and
  this renderer does not draw Omega's bloom (module docs). A brighter sky would
  also raise `LAvg` and lower the exposure for the whole frame (not measured). Ours, read off a
  probe pixel: `LAvg` 0.196, exposure 1.25, `t1` 1.506 on Vineta K; 0.149, 1.32
  and 1.0 on Anulpha Pass.
- **Vineta K's shadow coverage.** The canopy shadows most of the road in the
  original. Ours draws sun streaks across it.
- **Anulpha Pass's road box** is dominated by the chevrons. Ours draws them
  cyan; the chevron program is unwired (`omega-status.md`).

**2048:** checked, differs. Its race composite is `wo_composite_nocc_fp`, which
ends on a square root
([vita-2048-eu-v104/exposure.md](../vita-2048-eu-v104/exposure.md)), drawn by
`oag_post::wo_composite`, which this change does not touch. Whether the Vita's
display adds an encode of its own after that is not read here.

## Read with the decoder: the patch keeps the curve, and changes the composite (2026-10-27, `gcn-operand-decoder`)

The programs above were first read with a scratch disassembler on the **base** eboot. `oag_rcs::gcn`
([gcn.md](../../../formats/gcn.md)) now decodes every program in both `uroot/eboot.bin`s (base 203, patch 1.07 213,
0 refused), and finds them by their reflection's own constant names rather than by address:

| Program | Base `Shdr` | Patch `Shdr` | Base against patch |
| --- | --- | --- | --- |
| `pal_ToneMapCoefficientsFilter_fp` (constants `a0L_b0L_dLAvgMax_cL` ...) | `0x9657d4`, 125 instructions | `0x9dbdb4`, 125 | **identical**, every line |
| curve resolve (`m_abcd`, `m_LAvgt0t1Exposure`) | `0x968b84`, 105 | `0x9df104`, 105 | **identical**, every line |
| race composite (`BloomScale_abVignette`, `ScreenTint_Aspect`, motion blur, `wh_whInv`, `uvOrigo`) | `0x954e64`, 133 | `0x9c9f94`, 135 | **differs** |

(file offsets of the `Shdr` tag in each `eboot.bin`.) The resolve reads as quoted above: `s_buffer_load_dwordx8 s[8:15]`
(`m_abcd` then `m_LAvgt0t1Exposure`), `v_mul_f32 v0, s15, v0` (`E * c`), `v_min_f32 v0, s14, v0` (`t1`),
`v_max_f32 v0, s13, v0` (`t0`), then `v_mad_f32` / `v_mad_f32` / `v_mad_f32` with `s11`, `s9`, `s8` (`a + x (b + x (c + d x))`,
`c` in a register), three channels each, per fragment. **So the end curve the running game applies is this cubic Hermite,
confidence 82** (read on the patch's own program, which is what runs; the curve's derivation stays at the 80 above).

**The patch's composite differs in two ways** (both motion-blur composites, `0x9c94b4` and `0x9c9f94`; the two
non-motion-blur ones already had a `cAccum_ColorScale`):

```text
base:   out.rgb = vignette * (Frame + LowResAdditive + BloomScale * Bloom + ScreenTint.rgb)
patch:  out.rgb = ColorScale.rgb * vignette * (Frame + LowResAdditive + BloomScale * Bloom) + ScreenTint.rgb
```

read off the tails: base `v_add_f32 v0, s16, v14` (tint added) then `v_mul_f32 v0, v2, v0` (vignette); patch
`v_mul_f32 v3, s4, v4` (`ColorScale.r * vignette`, `ColorScale` the new member at dword 24, three floats) then
`v_mad_f32 v3, v3, v14, s16` (`* sum + tint`). Confidence 80 for the shader law. **The value of `ColorScale` is not
read**: its CPU writer is in the patch's eboot, whose functions this page's addresses do not cover (they are the base
build's). **Wired in part (`omega-composite`, 2026-11-04)**: see the next section. Bloom and the low-resolution additive layer are not.

## The composite's constants: where each comes from (2026-11-04, `omega-composite`)

Static, base `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`), all read in Ghidra; no live readout. The composite's constant
writer is `FUN_01625750` (the per-frame pass setup): it writes `BloomScale_abVignette` and `ScreenTint_Aspect`
(handles `DAT_01940588` / `DAT_01940586`) from three globals, so each term has a source:

| Term | Written at | Source | Value in a race |
| --- | --- | --- | --- |
| vignette `(a, b)` | `FUN_01625490`, `DAT_01fc7c30` | the circuit's `Vignette.Inner radius` (object `+0x278`) and `Vignette.Outer radius intensity` (`+0x27c`): `b = (I - 1) / (0.25 - r^2)`, `a = 1 - b r^2` | disc-authored. At `\|uv-0.5\|^2 = r^2` the factor is 1, at `0.25` (an edge's middle) it is `I`. Tech De Ra authors `r = 0.375`; two files read raw carry `I` = 0.716 and 0.8 |
| `ScreenTint.rgb` | `DAT_02134630` | reset from the zero at `0x01fc80a0` on `InGame` entry (`FUN_0129bd10`'s sibling at `0x0129bda1`); other writers (`FUN_013895d0`, `FUN_013db460`, `FUN_01299d30`, `FUN_015a3810`, `FUN_01280ca0`) store a scalar broadcast, one of them `s^5` | **0 at rest**; the flashes' triggers are unread |
| `ColorScale.rgb` | patch `0x104a520` | `(1, 1, 1)` at rest ([colorscale.md](colorscale.md)) | 1 |
| `BloomScale` | `FUN_01625490`, level array `DAT_01fc7848` (stride `0x108`, `DAT_01fc7820` levels) | `Bloom.Blend factors[level]` (object `+0x244`, four numbers; Tech De Ra-like files carry `0.15 1.004 0.906 0.85` and `0.384 1.404 0.904 1.268`), x `1.075` on the PS4 Pro's checkerboard mode | level 0's factor is the composite's `BloomScale` |

Confidence: **80** for the vignette law and its keys (the registrar `FUN_015c1f20` names `Vignette.Inner radius` at `+0x278`
and `Vignette.Outer radius intensity` at `+0x27c`, and `FUN_01625490` reads exactly those two), **70** that `DAT_01fc7c30`
is what `FUN_01625750` writes as the vignette dwords (one store of its eight bytes at `0x01628f23`), **65** for tint at rest.

**Drawn** (`oag_post::omega_tonemap::law::Composite`, `omega_tonemap.wesl` `fs_apply`): between the curve and the scanout
encode, `ColorScale * vignette * curved + ScreenTint`, tint outside the vignette (the patch's order; with the rest values the
base's order differs only in where a nonzero tint would sit, and no code path draws the base's). Pinned by
`the_composite_is_colour_scale_times_vignette_plus_tint` (frame 0.4, vignette 0.5, tint 0.1: patch 0.30, base 0.25),
`the_chain_draws_the_composite_after_the_curve` (a real device; red when the shader drops the term) and
`omega_lightmap_ground_truth` (Tech De Ra's keys reach the composite).

**`Bloom` is drawn since 2026-11-04 (next section); `LowResAdditive` is not.**
**2048:** checked, differs: its composite is `wo_composite` with its own vignette output and `vignetteTint`
(`oag_post::wo_composite`), no `Vignette.*` keys were found in the extracted Vita files, and the
Omega composite is gated to the PS4 geometry. **HD:** checked, differs: `hd_bloom` is its composite; a Fury frame is
byte-identical before and after this change.

## The bloom chain (2026-11-04, `omega-bloom-read`)

Static, read twice: base `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`) and patch 1.07 (`/omega/eboot-ps4-omega-eu-v107.bin`, which is
what runs). No live readout, so every score is capped at 84. The law, in the order the frame runs it:

```text
source = the frame the composite reads, past the curve            (puVar18 in FUN_01625750)
level 0..3: 960x540, 480x270, 240x135, 120x67                     (4 on a base PS4, 3 on checkerboard)
A[0]   = bright(source)
each level i:
  B[i] = horizontal blur of (i == 0 ? A[0] : A[i-1])               (halving for i > 0)
  A[i] = vertical blur of B[i]
for i = 3, 2, 1:  A[i-1] += f[i] * A[i]                            (blend one + one, coarse to fine)
BloomTexture = A[0], BloomScale = f[0]                             (x 1.075 on checkerboard)
out = ColorScale * vignette * (Frame + LowResAdditive + BloomScale * Bloom) + ScreenTint

bright(c): L = (c.r + c.g + c.b) / 3
           p = max(L - Threshold, 0) ^ Power
           w = Scale * p / (Offset + p) + AlphaScale * mask.a
           out.rgb = c * w
```

`Threshold`, `Offset`, `Scale`, `Power`, `AlphaScale` and `f[0..3]` are `Bloom.Bright-pass threshold/offset/scale/power/alpha scale`
and `Bloom.Blend factors`, per circuit.

| Claim | Evidence | Confidence |
| --- | --- | --- |
| The order above | `Bloom_RunChain` (`FUN_0177fbc0`): the bright pass into `level[0]+8`; a loop over `DAT_01fc7820` levels calling `Bloom_BlurHorizontal` (`FUN_01785290`) with target `level[i]+0x50`, source `level[i-1]+0x98` (level 0 its own), and `Bloom_BlurVertical` (`FUN_017856f0`) with target `level[i]+8`, source `level[i]+0xd0`; a second loop from the last level to 1 calling `Bloom_AddLevel` (`FUN_0177e1e0`) | 80 |
| Level array layout | Stride `0x108`: `+0` factor, `+8` target A, `+0x50` target B, `+0x98` view of A, `+0xd0` view of B (`Bloom_CreateLevels`, `FUN_0177e870`) | 80 |
| Four levels from 960x540 | `Bloom_CreateTargets` (`FUN_016247c0`) gets `0x3c0`, `0x21c` (`FUN_016134d0` at boot, `FUN_01620d80` on a mode change); the level count is `3` under `DAT_02133c49` (checkerboard) else `4`, and the size halves per level. Base counts the positive entries of `0x0183e240` (`0.25 0.95 0.9 0.85`); the patch's `FUN_0165d1b0` writes `(DAT_02212c39 == 0) + 3` directly and its table (`0x0185ef10`) starts `0.001`. The first entry is a placeholder: `Environment_ApplyBloomVignette` overwrites every level's factor. Both binaries agree on size and count | 82 |
| Split screen halves one axis | `FUN_01620d80` under its two-player mode: `0x1e0` by `0x21c`, or `0x3c0` by `0x10e`. Disassembly at `0x01620f6e` loads all six arguments the decompiler dropped; the patch's `FUN_0164b470` reads the same | 75 |
| Level format | Base: the scanout's format misses every case of `FUN_016247c0`'s switch, so `FUN_0177e870` takes its default `0x3ac706`, `R11G11B10F` by this page's field split. The patch's level loop creates `0x3ac706` first | 65 |
| The bright pass | `Shdr` `0x95fe54` (blob `0x1961d40`, 38 instructions): `image_sample` of slot 0, `image_sample` of slot 1 with `dmask:0x8` (alpha alone), `v_mul_f32 s4` / `v_mac_f32 s5` / `v_mac_f32 s6` (`cBrightness`), `v_subrev_f32 s0` (threshold), `v_max_f32 0`, `v_log_f32`, `v_mul_f32 s3` (power), `v_exp_f32`, `v_add_f32 s1` (offset), `v_rcp_f32`, `v_mul_f32 s2` (scale), `v_mac_f32 s7, mask.a` (boost), then `rgb * w`. The constants' order is the reflection name `Threshold_Offset_Scale_Power`, `cBrightness_BoostScale`. This reflection's header word refuses `oag_rcs::gcn`'s parser, so the names were read as strings at file `0x9600ea`, inside the blob | 84 |
| Which bright program | `Bloom_RunChain`: `level0.width * 2 < source.width` picks the four-tap twin `Shdr` `0x95f444` (blob `0x1961330`): the same arithmetic on the mean of `image_sample_o` at `(+-1, +-1)`. So 1920 wide takes the one tap, 3840 the four | 82 |
| `cBrightness` is a third each | `Bloom_CreatePrograms` (`FUN_0177f510`) writes `0x3eaaaaab` three times and the default boost `0.25`; `Environment_ApplyBloomVignette` (`FUN_01625490`) overwrites dwords 0-3 and 7 only. The patch's `FUN_017cfc40` and `FUN_01660030` read the same (`+0x27c`, with `BloomHDR` `+0x2a0` and the new `BloomVR` `+0x2c4` the other two picks) | 82 |
| Same-size blur | `Shdr` `0x961894` (`Offsets`, `Weights`, horizontal; `0x961ea4` its vertical twin): seven bilinear taps. `Bloom_CreateBlurPrograms` (`FUN_01784570`): offsets `0, 1.3846154, 3.2307692, 5.0769229` texels, weights `0.2255859, 0.3142090, 0.0698242, 0.0031738`, which are row 12 of Pascal's triangle over 4096 folded into pairs: an exact invariant, asserted | 84 |
| Halving blur | Source width differing from the target's selects `Shdr` `0x962534` (`aWeights`): eleven `image_sample_lz_o` taps at whole offsets `0, +-1..+-5` along x, weights `0.2256961, 0.1934538, 0.1209087, 0.0537372, 0.0161212, 0.0029311` (sum 1) | 82 |
| The add | `Bloom_AddLevel` binds the `Scale` program (`Shdr` `0x95fa14`, `rgba * Scale`) with the blend state `DAT_0212a154`, set by `FUN_0177def0` through `FUN_012091a0(.., 1, 0, 1)`: one + one. `FUN_0177e670` sets its sibling `(4, 0, 5)`, source alpha and its inverse, through the same call, and Gnm's `BlendMultiplier` numbers those so | 80 |
| Composite reads level 0's A | `FUN_01625750` binds `DAT_01fc7848 + 0x98` to the composite's slot 3, `BloomTexture` by reflection. All three of frame, additive layer and bloom are read at the distorted coordinate (`image_sample_lz` at `v0`, patch `Shdr` `0x9c9f94`) | 80 |
| The source is the composite's own `FrameTexture` | `FUN_01625750` passes `puVar18` to the chain (`0x01628bc5`) and then binds the same variable as the composite's slot 0. It is the curve resolve's output view `DAT_01fc63e0` or, when the accumulated motion blur ran, `DAT_01fc7920`. So the thresholds act on the curve's `[0, 1]`, and Sol 2's `1.252` never fires on the frame term | 76 |

**Every bloom program is identical in the patch**, compared line by line through `gcn_dump` (the disassembly text, not bytes: headers and
reflection addresses differ). Base `Shdr` to patch: `0x95fe54` to `0x9d6444`, `0x95f444` to `0x9d5a34`, `0x95fa14` to `0x9d6004`, `0x961894` to
`0x9d7e84`, `0x961ea4` to `0x9d8494`, `0x962534` to `0x9d8b24`, `0x969574` to `0x9dfb04`, `0x969764` to `0x9dfcf4`, `0x9699a4` to `0x9dff34`. The
same-size blur's immediates in the patch's `FUN_0141efe0` are the same numbers; the halving blur's `aWeights` were read in the base only.

**The census.** 38 circuit files (`track` and `track_reversed` of twelve HD-lineage circuits, four Zones, ten of 2048's) through the race's
reader, all whole: `omega_lightmap_ground_truth`. All 38 live in the patch's `data08`. The numbers differ by circuit and are not a family: `Threshold`
`0.35` to `1.252`, `Scale` `0.2` to `10`, `Power` `0.1` to `3.619`, `AlphaScale` `0.237` to `6`, `f[0]` `0.05` to `1.596`. `data/visual_effects/detonator.envsettings`,
`zone.envsettings` and `zone2048.envsettings` author blocks of their own; what selects them is unread.

**The mask, not drawn: one term unread, here is its recipe.** The second texture of the bright pass is `DAT_01fc6568`, a view of `DAT_01fc61e8`, a 960x540
8-bit target. The scene's `PostTarget_Create` call (`FUN_0178a160`, from `FUN_01621650`) passes `DAT_01fc61a0` as a second destination, and the programs
`0x969574`, `0x969764` and `0x9699a4` write it, one tap, two taps or four by the size ratio. Their outputs are named `main.m_Distance` (mrt0, `R16F`,
`DAT_01fc61a0`) and `main.m_Alpha` (mrt1, alpha of `DAT_01fc61e8`), out of channels `.x` and `.y` of a full-size `0x22c705` target the scene draws
as its second colour output. So the mask is half-size **per-material output alpha**, confidence 72. On the two tails read, an additive `cf_startbeam_glow`
writes the diffuse texture's alpha there (`exp mrt1 v0, v2 compr`, `v0 = pack(v4, v8)`, `v8` the sample's fourth channel), and an opaque
`diffuse_specular_V01_2` writes zero (`v_mov_b32_sdwa v1, 0 dst_sel:WORD_1`). Two programs out of 1.78 million: 60 that this is the rule.

`AlphaScale` is not small: `0.237` to `6`, against a `Scale` of the same order. So **a material the mask would light blooms too little here**, most on
Amphiseum (6), Chenghou Project (1.4) and Anulpha Pass (1.314); this is not compensated. To draw it: read `exp mrt1`'s second half off each material's
programs (a `Program` method in `oag_rcs::gcn`, the census over `gcn_ground_truth`'s pass), carry it to `GpuVertex::glow` on the `psp2` path (which hard-codes
0), write it to a scene channel, and add `AlphaScale * mask` in `fs_bright`. `Bloom::bright_weight` already takes the mask.

**Also not the original's.** The accumulated motion blur's target is not the source here, because this renderer has none. `LowResAdditive`
(slot 1, view `DAT_01fc6a50` of `DAT_01fc6a08`) stays undrawn: the particles draw into the scene and so pass through the curve. Drawing it needs
those draws moved to a half-size target past the curve, depth-tested against the `m_Distance` target, and which draws go there is unread.

**Drawn** (`oag_post::omega_tonemap::bloom`, `omega_tonemap.wesl` `fs_bright`, `omega_bloom.wesl`). Chosen, not measured: level 0 keeps height 540 and
takes its width from the frame's shape, because the executable only draws 16:9; the levels are `Rgba16Float`, since `R11G11B10F` is not renderable on every
device. The bright pass reads the four pixels a level 0 texel covers and curves each, which is the one-tap program's bilinear read at 1920 wide and the
four-tap's at 3840, with no extra full-size target. A split view runs the chain alone, in the corner of each level; the original's two-player
sizes come out of that. No setting: the 2026-10-02 ruling (bloom has none, only the original's). Pinned by `a_flat_frame_blooms_by_the_nested_sum`,
`bloom_bleeds_past_an_edge_and_only_with_a_block`, `a_split_view_blooms_alone` (red with the term dropped from `fs_apply`, or the add), and the law's tests.

**What it does to a frame**, 1920x1080, same craft and grid, ticks 100 and 900, ours with the chain against ours without (whole-frame mean grey; bloom only adds, no pixel fell):

| Circuit | Mean grey, before to after | Pixels up by more than 8 | Up by more than 32 |
| --- | --- | --- | --- |
| Vineta K | 70.2 to 71.3, 72.2 to 73.3 | 3.7 %, 3.9 % | 0.73 %, 0.69 % |
| Anulpha Pass | 73.0 to 73.4, 72.5 to 72.9 | 0.3 %, 0.4 % | 0.01 %, 0.02 % |
| Tech De Ra | 97.7 to 101.3, 99.0 to 103.3 | 11.4 %, 14.1 % | 2.96 %, 3.46 % |

Tech De Ra's light strips and start board glow, Vineta K's white crowd and hull highlights. **It does not blow out Vineta K's sky** (a lead on the
output-encode section above): ours draws that sky mid-grey, under `Threshold` 0.784, so the bright pass passes nothing there. The gap is the sky's
level before the chain, not the chain. An HD and a Pulse frame are byte-identical to a pre-change build (`sha256` equal at tick 100).

**Cost**: 0.55 ms a frame more at 1920x1080 on an RX 7800 XT (1.32 against 0.75 ms for the whole chain, 400 frames twice each, load 16, `quick` profile), 0.31 ms
of it recording twelve passes on the CPU in that unoptimised profile. Not measured on the Deck.

**2048:** checked, differs. No `Threshold_Offset_Scale_Power` in its `eboot.elf`; 50 `.envsettings` carry no `Bright-pass` key (base, patch, both DLC). It has
`Lighting.BloomFactor` and `BloomGate`, and `oag_post::wo_composite` draws that chain ([vita-2048-eu-v104/exposure.md](../vita-2048-eu-v104/exposure.md)).
**HD:** checked, differs. No such program in `EBOOT.elf`, none of 33 files carries the key; `hd_bloom`'s `FunkLayerBloom` is another chain: a gate on luminance and
alpha, nine-tap blurs and an adapted composite under the curve. Omega's own 2048-lineage circuits author `Bloom.*` and run this chain, which is why the census has them.

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

`oag_post::omega_tonemap` (2026-10-05) runs this law on an Omega race, with
the choices listed in its module docs and in
[`omega-status.md`](../../../formats/omega-status.md). Its test
`the_law_reproduces_the_original_coefficient_shader` holds the Rust law to the
texels the emulated coefficient shader produced.

## Names

[`names.tsv`](names.tsv) rows added by this page:

- `0x0177fbc0` `Bloom_RunChain` - 80.
- `0x016247c0` `Bloom_CreateTargets` - 76. Builds the levels, the motion-blur and related targets beside them.
- `0x0177e870` `Bloom_CreateLevels` - 80.
- `0x0177f510` `Bloom_CreatePrograms` - 80.
- `0x01784570` `Bloom_CreateBlurPrograms` - 82.
- `0x01785290` `Bloom_BlurHorizontal` - 80.
- `0x017856f0` `Bloom_BlurVertical` - 80.
- `0x0177e1e0` `Bloom_AddLevel` - 76. Also the one-step composite of `FUN_01625750`'s first branch.
- `0x01625490` `Environment_ApplyBloomVignette` - 82. Two independent sites: the registrar's offsets and the level array's consumers.

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

- Which level of the luminance chain the coefficient pass samples, and what
  image the chain starts from.
- The checkerboard compute resolve `0x19685a0`, beyond its constant names.
- Every number above on a running PS4.

## 2026-10-26, `2048-exposure`: `wo_composite_nocc_fp` is 2048's live race composite

The `wo_*` set this page records as unused here is the post chain Wipeout 2048 runs on every race, with the
exposure in `bloomFactor.y`; read there off Vita3K's translation of 2048's own program and checked live, see
[vita-2048-eu-v104/exposure.md](../vita-2048-eu-v104/exposure.md).
