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
