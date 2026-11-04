//! The tone map's arithmetic on the CPU: the same law the chain's WGSL runs,
//! kept here so tests can hold the shader to it and so the law reads once in
//! Rust. Every formula is `pal_ToneMapCoefficientsFilter_fp`'s (blob
//! `0x019676c0`) and the curve resolve's (blob `0x0196aa70`), with the
//! constants `ToneMap_ApplyEnvSettings` (`0x01620980`) writes - see
//! `docs/ghidra/functions/ps4-omega-eu/tonemap.md`.

/// The `.EnvSettings` `Tonemap.*` block, in the units the file authors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// `Luminance a-coefficient`: the exposure target's constant term.
    pub luminance_a: f32,
    /// `Luminance b-coefficient`: the exposure target's slope over `LAvg`.
    pub luminance_b: f32,
    /// `Exposure minimum`.
    pub exposure_minimum: f32,
    /// `Exposure maximum`.
    pub exposure_maximum: f32,
    /// `Exposure response`: the most `LAvg` may move in one second; the
    /// executable divides it by 60 and applies it once a frame.
    pub exposure_response: f32,
    /// `Exposure time`: seconds of luminance history the target averages.
    pub exposure_time: f32,
    /// `Source color end a-coefficient`: the curve's input end `t1`, constant term.
    pub source_end_a: f32,
    /// `Source color end b-coefficient`: `t1`'s slope over `LAvg`.
    pub source_end_b: f32,
    /// `Start angle`, degrees: the curve's slope at its foot, as an angle off
    /// the straight line from `(0, 0)` to `(t1, 1)`.
    pub start_angle: f32,
    /// `End angle`, degrees: the same at its shoulder.
    pub end_angle: f32,
    /// The race composite's terms that follow the curve.
    pub composite: Composite,
}

/// The output end of the curve under SDR video out. `ToneMap_ApplyEnvSettings`
/// writes 1.0 there, and 40.0 when the console is in HDR mode.
pub const SDR_OUTPUT_END: f32 = 1.0;

/// The player's brightness factor `cL` at its reset value. The executable maps
/// its menu setting onto `0.25..1.75`; this port has a brightness grade of its
/// own after the frame, so the curve keeps the middle (**chosen, not measured**).
pub const BRIGHTNESS: f32 = 1.0;

/// The ceiling the executable clamps every luminance it reads back to.
pub const LUMINANCE_CEILING: f32 = 1000.0;

/// The ring the executable keeps, indexed by a byte.
pub const HISTORY_CAPACITY: usize = 256;

impl Params {
    /// Frames of history the target averages: `round(time * 60)` truncated to
    /// a byte, and 1 when that byte is zero - `ToneMap_ApplyEnvSettings`'s
    /// `uVar6 & 0xff` with its `(char)uVar6 != 0` test.
    #[must_use]
    pub fn history_frames(&self) -> u32 {
        let rounded = (self.exposure_time * 60.0 + 0.5).max(0.0) as u32 & 0xff;
        rounded.max(1)
    }

    /// The most `LAvg` moves in one frame: `response * 0.016666668`.
    #[must_use]
    pub fn step(&self) -> f32 {
        self.exposure_response * 0.016_666_668
    }
}

/// One frame's coefficients: what texels 0 and 1 of the original's 3x1
/// coefficient texture hold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve {
    /// `y = a + b x + c x^2 + d x^3`.
    pub abcd: [f32; 4],
    /// The input window the resolve clamps `exposure * colour` to.
    pub t0: f32,
    /// See [`Curve::t0`].
    pub t1: f32,
    /// The exposure the resolve multiplies colour by.
    pub exposure: f32,
}

/// One end's slope multiplier, `(cos + sin) / (cos - sin)` = `tan(phi + pi/4)`,
/// with the shader's own guard: a denominator within `1e-6` of zero becomes
/// `+1e-6` when it was positive and `-1e-6` otherwise.
fn slope_factor(phi: f32) -> f32 {
    let (sin, cos) = phi.sin_cos();
    let den = cos - sin;
    let den = if den.abs() > 1e-6 {
        den
    } else if den > 0.0 {
        1e-6
    } else {
        -1e-6
    };
    (cos + sin) / den
}

/// The adaptation step: `LAvg = P + clamp(target - P, -step, step)`, both
/// sides first clamped to [`LUMINANCE_CEILING`].
#[must_use]
pub fn adapt(previous: f32, target: f32, step: f32) -> f32 {
    let previous = previous.min(LUMINANCE_CEILING);
    let target = target.min(LUMINANCE_CEILING);
    previous + (target - previous).clamp(-step, step)
}

impl Curve {
    /// The coefficient pass for one adapted luminance.
    ///
    /// `t0 = 0` and `s0 = 0` because `ToneMap_ApplyEnvSettings` zeroes
    /// `a1t0`, `b1t0`, `a3s0` and `b3s0`; `s1` is [`SDR_OUTPUT_END`].
    #[must_use]
    pub fn fit(p: &Params, lavg: f32) -> Self {
        let t0 = 0.0_f32;
        let t1 = (p.source_end_a + p.source_end_b * lavg).max(t0 + 0.01);
        let s0 = 0.0_f32;
        let s1 = SDR_OUTPUT_END.max(s0 + 0.01);
        let h = t1 - t0;
        let k = (s1 - s0) / h;
        let m0 = k * slope_factor(p.start_angle.to_radians());
        let m1 = k * slope_factor(p.end_angle.to_radians());
        // The Hermite cubic through (0, s0) and (h, s1) with those end
        // slopes, as a polynomial in x - t0 (= x, t0 being zero).
        let c = (3.0 * (s1 - s0) - h * (2.0 * m0 + m1)) / (h * h);
        let d = (2.0 * (s0 - s1) + h * (m0 + m1)) / (h * h * h);
        let exposure = ((p.luminance_a + p.luminance_b * lavg) / lavg.max(1e-4))
            .clamp(p.exposure_minimum, p.exposure_maximum)
            * BRIGHTNESS;
        Self {
            abcd: [s0, m0, c, d],
            t0,
            t1,
            exposure,
        }
    }

    /// The resolve's per-channel map of one linear colour value.
    #[must_use]
    pub fn map(&self, colour: f32) -> f32 {
        let [a, b, c, d] = self.abcd;
        let x = (self.exposure * colour).clamp(self.t0, self.t1);
        a + x * (b + x * (c + d * x))
    }
}

/// Rec.601 luma, `pal_LuminanceFilter_fp`'s own three constants.
pub const LUMA: [f32; 3] = [0.299, 0.587, 0.114];

/// The display's encode of the curve's linear `[0, 1]`: the piecewise sRGB curve.
///
/// The executable registers its scanout buffers as `A2R10G10B10`
/// (`0x88060000`, not the `_SRGB` variant) with gamma 1.0, renders them as a
/// UNORM `2_10_10_10` target (`0xf2e009`) and fills them with a plain copy, so
/// no encode happens on the GPU: the video out applies it. shadPS4 0.20, which
/// captured every reference frame, applies exactly this curve to that format.
/// See `docs/ghidra/functions/ps4-omega-eu/tonemap.md`, "The output encode".
#[must_use]
pub fn scanout_encode(linear: f32) -> f32 {
    let c = linear.clamp(0.0, 1.0);
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// The race composite that follows the curve (patch 1.07's program, `Shdr` at
/// `0x9c9f94`):
/// `out.rgb = ColorScale.rgb * vignette * (curved + LowResAdditive + BloomScale * Bloom) + ScreenTint.rgb`
/// (`docs/ghidra/functions/ps4-omega-eu/tonemap.md`, "Read with the decoder").
///
/// The curve's output is linear light in `[0, 1]`, so bloom, vignette and tint
/// all act on it before [`scanout_encode`]. **`LowResAdditive` is not drawn**
/// (the particles draw into the scene); this holds the terms the disc and the
/// executable author for the rest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Composite {
    /// `Vignette.Inner radius`: the distance from the centre, in uv, inside
    /// which the picture is untouched.
    pub vignette_inner_radius: f32,
    /// `Vignette.Outer radius intensity`: the factor the picture reaches at
    /// `|uv - 0.5| = 0.5`, the middle of an edge.
    pub vignette_outer_intensity: f32,
    /// `ColorScale`, which rests at `(1, 1, 1)`; a fade drives it lower and its
    /// trigger is unread (`colorscale.md`).
    pub color_scale: [f32; 3],
    /// `ScreenTint.rgb`, which rests at zero (`InGame` entry resets it from the
    /// zero at `0x01fc80a0`); flash effects raise it and their triggers are unread.
    pub screen_tint: [f32; 3],
    /// The circuit's `Bloom.*` block; `None` draws no bloom.
    pub bloom: Option<Bloom>,
}

impl Composite {
    /// No vignette, no fade, no tint: the composite changes nothing.
    pub const NONE: Self = Self {
        vignette_inner_radius: 0.0,
        vignette_outer_intensity: 1.0,
        color_scale: [1.0; 3],
        screen_tint: [0.0; 3],
        bloom: None,
    };

    /// The circuit's vignette with the rest values for the other two terms.
    #[must_use]
    pub fn at_rest(inner_radius: f32, outer_intensity: f32) -> Self {
        Self {
            vignette_inner_radius: inner_radius,
            vignette_outer_intensity: outer_intensity,
            ..Self::NONE
        }
    }

    /// The shader's `(a, b)` in `sat(a + b * |uv - 0.5|^2)`, as
    /// `FUN_01625490` writes them to
    /// `DAT_01fc7c30`: `b = (intensity - 1) / (0.25 - r^2)`, `a = 1 - b r^2`.
    /// A radius of 0.5 or more would divide by zero or flip the sign; the
    /// executable has no guard and no circuit authors one, so it draws no
    /// vignette there.
    #[must_use]
    pub fn vignette_ab(&self) -> [f32; 2] {
        let r2 = self.vignette_inner_radius * self.vignette_inner_radius;
        let den = 0.25 - r2;
        if den <= 1e-6 {
            return [1.0, 0.0];
        }
        let b = (self.vignette_outer_intensity - 1.0) / den;
        [1.0 - b * r2, b]
    }

    /// The vignette factor at a uv: `sat(a + b * |uv - 0.5|^2)`.
    #[must_use]
    pub fn vignette(&self, uv: [f32; 2]) -> f32 {
        let [a, b] = self.vignette_ab();
        let (dx, dy) = (uv[0] - 0.5, uv[1] - 0.5);
        (a + b * (dx * dx + dy * dy)).clamp(0.0, 1.0)
    }

    /// One colour channel of the composite on the curve's output:
    /// `ColorScale * vignette * curved + ScreenTint`. The tint sits **outside**
    /// the vignette (the base program had it inside).
    #[must_use]
    pub fn apply(&self, channel: usize, curved: f32, uv: [f32; 2]) -> f32 {
        self.apply_with_bloom(channel, curved, 0.0, uv)
    }

    /// [`Composite::apply`] with the chain's output `bloom` added inside the vignette:
    /// `ColorScale * vignette * (curved + BloomScale * bloom) + ScreenTint`.
    #[must_use]
    pub fn apply_with_bloom(&self, channel: usize, curved: f32, bloom: f32, uv: [f32; 2]) -> f32 {
        let scale = self.bloom.map_or(0.0, |b| b.blend_factors[0]);
        self.color_scale[channel] * self.vignette(uv) * (curved + scale * bloom)
            + self.screen_tint[channel]
    }
}

/// Level 0 of the bloom chain, the executable's constant `0x3c0` by `0x21c` whatever the
/// frame's size (`FUN_016247c0`; patch `FUN_0165d1b0`).
pub const BLOOM_BASE: (u32, u32) = (960, 540);

/// Levels on a base PS4: `(checkerboard == 0) + 3`, each half the one before.
pub const BLOOM_LEVELS: usize = 4;

/// The same-size blur's tap distances in source texels, `FUN_01784570`'s immediates: the
/// twelfth row of Pascal's triangle folded into bilinear taps.
pub const BLUR_OFFSETS: [f32; 4] = [0.0, 1.384_615_4, 3.230_769_2, 5.076_923];

/// The weight of each [`BLUR_OFFSETS`] tap, both sides of the centre carrying the same one.
pub const BLUR_WEIGHTS: [f32; 4] = [0.225_585_94, 0.314_208_98, 0.069_824_22, 0.003_173_828_1];

/// The halving blur's `aWeights`, centre first, for taps a whole source texel apart.
pub const DOWN_WEIGHTS: [f32; 6] = [
    0.225_696_15,
    0.193_453_83,
    0.120_908_65,
    0.053_737_17,
    0.016_121_15,
    0.002_931_12,
];

/// `cBrightness`, `FUN_0177f510`'s constant: the bright pass's luminance is the plain mean
/// of the channels, not Rec.601.
pub const BRIGHTNESS_WEIGHT: f32 = 1.0 / 3.0;

/// The bright pass and the per-level blend the disc authors (`Bloom.*`), read by the
/// programs `Shdr` `0x95fe54` and `0x95f444` and the chain `FUN_0177fbc0`
/// (`docs/ghidra/functions/ps4-omega-eu/tonemap.md`, "The bloom chain").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bloom {
    /// `Bright-pass threshold`, on the curved frame's `[0, 1]` luminance.
    pub threshold: f32,
    /// `Bright-pass offset`: the softening under the power term.
    pub offset: f32,
    /// `Bright-pass scale`.
    pub scale: f32,
    /// `Bright-pass power`.
    pub power: f32,
    /// `Bright-pass alpha scale`: the weight on the scene's `m_Alpha` mask. The disc's
    /// number, carried for the mask's lane; nothing reads the mask yet.
    pub alpha_scale: f32,
    /// `Blend factors`, finest level first; level 0's is the composite's `BloomScale`.
    pub blend_factors: [f32; 4],
}

impl Bloom {
    /// What a pixel of luminance `l` and scene mask `mask` is multiplied by:
    /// `Scale * p / (Offset + p) + AlphaScale * mask`, `p = max(l - Threshold, 0) ^ Power`.
    #[must_use]
    pub fn bright_weight(&self, l: f32, mask: f32) -> f32 {
        let t = (l - self.threshold).max(0.0);
        let p = if t > 0.0 { t.powf(self.power) } else { 0.0 };
        self.scale * p / (self.offset + p) + self.alpha_scale * mask
    }

    /// The bright pass on a curved colour with no mask drawn.
    #[must_use]
    pub fn bright_pass(&self, c: [f32; 3]) -> [f32; 3] {
        let l = (c[0] + c[1] + c[2]) * BRIGHTNESS_WEIGHT;
        c.map(|channel| channel * self.bright_weight(l, 0.0))
    }

    /// The chain's blend back up, coarse to fine: each level adds the one below it scaled
    /// by that level's factor, so `A0 + f1 (A1 + f2 (A2 + f3 A3))`. Level 0's factor is
    /// the composite's, not this sum's.
    #[must_use]
    pub fn blend_up(&self, levels: [f32; BLOOM_LEVELS]) -> f32 {
        let mut sum = levels[BLOOM_LEVELS - 1];
        for level in (0..BLOOM_LEVELS - 1).rev() {
            sum = levels[level] + self.blend_factors[level + 1] * sum;
        }
        sum
    }
}

/// A level's size: each is the last halved, rounding down, as `FUN_0177e870` does.
#[must_use]
pub fn bloom_level_size(base: (u32, u32), level: usize) -> (u32, u32) {
    ((base.0 >> level).max(1), (base.1 >> level).max(1))
}

/// Level 0 for a frame of `size`: the executable's height, and its width kept to the
/// frame's shape so the blur reaches as far across any window as it does across 16:9.
/// The executable only ever draws 16:9, where this is [`BLOOM_BASE`]; the width rule for
/// any other shape is **chosen, not measured**.
#[must_use]
pub fn bloom_base(size: (u32, u32)) -> (u32, u32) {
    let (width, height) = (size.0.max(1) as f32, size.1.max(1) as f32);
    let scaled = (BLOOM_BASE.1 as f32 * width / height).round() as u32;
    (scaled.max(2) & !1, BLOOM_BASE.1)
}
