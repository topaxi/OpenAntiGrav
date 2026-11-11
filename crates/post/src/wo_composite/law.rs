//! The chain's arithmetic on the CPU: the same law `wo_composite.wesl` runs,
//! kept here so tests can hold the shader to it and so the law reads once in
//! Rust. Every formula is a `wo_*` program's or `Exposure_AdaptFromReadback`'s
//! (`0x8103d148`) - see `docs/ghidra/functions/vita-2048-eu-v104/exposure.md`.

/// A circuit's exposure and bloom keys, read from its `track.EnvSettings`, and
/// how this run sets the exposure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// `Lighting.ExposureScale`: the mean luminance the exposure steers the
    /// frame to, `E -> ExposureScale / a`.
    pub exposure_scale: f32,
    /// `Lighting.ExposureMax`: the ceiling on `E`.
    pub exposure_max: f32,
    /// `Lighting.ExposureSpeed`: the fraction of the way to the target `E`
    /// moves in one frame.
    pub exposure_speed: f32,
    /// `Lighting.BloomFactor`: the bloom's weight inside the composite's root.
    pub bloom_factor: f32,
    /// `Lighting.BloomGate`: the luminance the gate passes nothing below.
    pub bloom_gate: f32,
    /// Whether `E` adapts or is held.
    pub exposure: Exposure,
}

/// How a run sets the composite's exposure `E`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Exposure {
    /// Adapting once a frame to the gate target's mean luminance, as the
    /// original does on a Vita.
    Adapting,
    /// Held at one value and never stepped. **A comparison switch, never the
    /// play default**: Vita3K never writes the gate target back to the guest,
    /// so its readback is zero, the step never runs and every 2048 frame it
    /// draws is at [`RESET_EXPOSURE`]. Holding ours there compares like with
    /// like.
    Held(f32),
}

/// The exposure `Exposure_Set` (`0x8103d29c`) writes, the constant
/// `0x3f933333`, and where an adapting run starts before its first frame.
pub const RESET_EXPOSURE: f32 = 1.15;

/// `luminanceFactor`, seeded at `0x816af030`/`0x816af040`; the gate and the
/// composite both dot with it.
pub const LUMINANCE: [f32; 3] = [0.3, 0.59, 0.11];

/// The gate's alpha is its luminance times this, the program's half literal
/// `0x3400`.
pub const GATE_ALPHA_SCALE: f32 = 0.25;

/// The readback's scale on the mean alpha byte, `0x3c800000`. With the gate's
/// [`GATE_ALPHA_SCALE`] and an 8-bit target the two cancel to within `255/256`:
/// `a` is the mean luminance the gate saw.
pub const READBACK_SCALE: f32 = 1.0 / 64.0;

/// No step is taken unless the readback is above this, `0x3c23d70a`.
pub const READBACK_FLOOR: f32 = 0.01;

/// `wo_blur_fp`'s weights for the tap pairs at 1, 3 and 5 offsets out, the
/// program's halves `0x3c00`, `0x3a66` and `0x3800`.
pub const BLUR_WEIGHTS: [f32; 3] = [1.0, 0.799_804_7, 0.5];

/// The blur's sum is multiplied by this, the half `0x32f2` (1 / 4.6).
pub const BLUR_NORMALISE: f32 = 0.217_041_02;

/// `vignetteTint`, the half4 `PostFx_RenderChain` packs from `0x816aef08..0x816aef14`, which
/// `PostFx_RegisterPrograms` (`0x8103d2ae`, stores at `0x8103d762..0x8103d770`) seeds to
/// `(1.0, 1.0, 0.97, 1.0)` and nothing else writes.
pub const VIGNETTE_TINT: [f32; 3] = [1.0, 1.0, 0.97];

/// The vignette mesh's centre height in clip space, the float at `0x8151c6b4`
/// (`0xbf628f5c`): the mesh is drawn round `(0, -0.885)`, low in the frame, so the
/// picture's top corners are the far edge.
pub const VIGNETTE_CENTRE_Y: f32 = -0.885;

/// The radius, in clip units, inside which the vignette is 0, `0x8151c6b8` (1.5).
pub const VIGNETTE_INNER: f32 = 1.5;

/// The radius at which `w` reaches [`VIGNETTE_STRENGTH`], `0x8151c6bc` (`0x40099a99`, 2.15).
pub const VIGNETTE_OUTER: f32 = f32::from_bits(0x4009_9a99);

/// `w` on the outer ring, `0x8151c6b0` (1.0); `PostFx_RenderChain` writes 0 there instead
/// while `FUN_81021ef6` reports a photo capture in progress, which a race here has none of.
pub const VIGNETTE_STRENGTH: f32 = 1.0;

/// The mesh is a fan and a ring of this many sectors, `2 pi / 0.19634952`.
pub const VIGNETTE_SECTORS: u32 = 32;

/// The vignette factor `v` of the composite for a mesh `w`: `wo_composite_vp` forms
/// `t = vignetteTint * w` and `wo_composite_nocc_fp` multiplies by `1 - t * t`.
#[must_use]
pub fn vignette(w: f32) -> [f32; 3] {
    VIGNETTE_TINT.map(|tint| 1.0 - (tint * w) * (tint * w))
}

/// The `w` the mesh interpolates to at a point of clip space, over the same triangles
/// `PostFx_RenderChain` builds: per sector a fan from the centre to the inner circle
/// (`w` 0) and a ring quad split along the diagonal from the inner circle's far corner
/// to the outer circle's near one (`w` 0 inside, [`VIGNETTE_STRENGTH`] outside). Beyond
/// the outer polygon the mesh draws nothing and the frame keeps what it held, which is
/// no vignette; the frame's own corners are inside it.
#[must_use]
pub fn vignette_w(clip: [f32; 2]) -> f32 {
    let p = [clip[0], clip[1] - VIGNETTE_CENTRE_Y];
    let step = std::f32::consts::TAU / VIGNETTE_SECTORS as f32;
    let angle = p[0].atan2(p[1]).rem_euclid(std::f32::consts::TAU);
    let sector = ((angle / step) as u32).min(VIGNETTE_SECTORS - 1);
    let point = |k: u32, radius: f32| {
        let a = k as f32 * step;
        [radius * a.sin(), radius * a.cos()]
    };
    let (b, c) = (
        point(sector, VIGNETTE_INNER),
        point(sector + 1, VIGNETTE_INNER),
    );
    let (d, e) = (
        point(sector, VIGNETTE_OUTER),
        point(sector + 1, VIGNETTE_OUTER),
    );
    let inside = |w: [f32; 3]| w.iter().all(|&weight| weight >= -1e-5);
    let bcd = barycentric(p, b, c, d);
    if inside(bcd) {
        return bcd[2] * VIGNETTE_STRENGTH;
    }
    let cde = barycentric(p, c, d, e);
    if inside(cde) {
        return (cde[1] + cde[2]) * VIGNETTE_STRENGTH;
    }
    0.0
}

fn barycentric(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> [f32; 3] {
    let det = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    let wa = ((b[1] - c[1]) * (p[0] - c[0]) + (c[0] - b[0]) * (p[1] - c[1])) / det;
    let wb = ((c[1] - a[1]) * (p[0] - c[0]) + (a[0] - c[0]) * (p[1] - c[1])) / det;
    [wa, wb, 1.0 - wa - wb]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// `wo_bloom_gate_fp` over its four taps: the colour it writes and its alpha.
#[must_use]
pub fn gate(taps: [[f32; 3]; 4], exposure: f32, bloom_gate: f32) -> ([f32; 3], f32) {
    let mut sum = [0.0_f32; 3];
    for tap in taps {
        for (s, t) in sum.iter_mut().zip(tap) {
            *s += t;
        }
    }
    let quarter = LUMINANCE.map(|w| w * 0.25);
    let luminance = dot(sum, quarter);
    let k = exposure * (luminance - bloom_gate).clamp(0.0, 1.0);
    (sum.map(|s| k * k * s), luminance * GATE_ALPHA_SCALE)
}

/// The readback's `a` for the mean alpha byte of the sampled pixels.
#[must_use]
pub fn readback(mean_alpha_byte: f32) -> f32 {
    mean_alpha_byte * READBACK_SCALE
}

/// One step of `Exposure_AdaptFromReadback` from `exposure` for a readback `a`.
#[must_use]
pub fn adapt(exposure: f32, a: f32, p: &Params) -> f32 {
    if a > READBACK_FLOOR {
        (exposure + (p.exposure_scale / a - exposure) * p.exposure_speed).min(p.exposure_max)
    } else {
        exposure
    }
}

/// `wo_composite_nocc_fp` for one pixel of the scene and the bloom under it,
/// before the vignette [`vignette`] (multiply the result by it) and with the screen
/// tint at 0 (unread).
#[must_use]
pub fn composite(main: [f32; 3], bloom: [f32; 3], exposure: f32, bloom_factor: f32) -> [f32; 3] {
    let e = main.map(|c| exposure * c.max(0.0));
    let l = dot(e, LUMINANCE);
    std::array::from_fn(|i| (e[i] * l / (1.0 + l) + bloom_factor * bloom[i]).sqrt())
}
