//! A track file's colours are linear (glTF's `COLOR_0`, Blender's own output);
//! Pulse and Pure's renderer draws gamma bytes (`byte / 255`, the PSP's vertex
//! colour and GE light registers). The curve is sRGB's piecewise one, which is
//! what Blender's glTF exporter inverts on the way out.
//!
//! Render-side only, so `powf` is fine here: `oag-trackfile` is in
//! `check-determinism`'s scan and stores the linear numbers untouched.

/// sRGB's encoding: linear to gamma. Worked in `f64` and rounded once, so it
/// is monotone at `f32`'s resolution and every gamma float has a linear one
/// that maps onto it ([`exact_linear`]); the textbook `f32` form tops out at
/// `0.99999994` and skips values in between.
#[must_use]
pub fn to_gamma(linear: f32) -> f32 {
    let l = f64::from(linear);
    let g = if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    g as f32
}

/// sRGB's decoding: gamma to linear, in `f64` like [`to_gamma`].
#[must_use]
pub fn to_linear(gamma: f32) -> f32 {
    let g = f64::from(gamma);
    let l = if g <= 0.040_45 {
        g / 12.92
    } else {
        ((g + 0.055) / 1.055).powf(2.4)
    };
    l as f32
}

/// A linear channel as the gamma byte the renderer draws: the nearest one,
/// clipped to `0..=255`. **Rounded, not truncated** (chosen, not measured): the
/// curve's last-bit error would otherwise drop an exact byte a step on its way
/// back, and the round trip of a disc export is what proves the loader.
#[must_use]
pub fn gamma_byte(linear: f32) -> u8 {
    (to_gamma(linear.max(0.0)) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8
}

/// A coverage channel (alpha, on no curve) as the nearest byte.
#[must_use]
pub fn coverage_byte(alpha: f32) -> u8 {
    (alpha * 255.0).round().clamp(0.0, 255.0) as u8
}

/// A linear RGBA vertex colour as the `byte / 255` floats the `.vex` path
/// builds (`oag_mesh::mesh`'s vertex colour), alpha off the curve.
#[must_use]
pub fn vertex(linear: [f32; 4]) -> [f32; 4] {
    let [r, g, b, a] = linear;
    [
        gamma_byte(r),
        gamma_byte(g),
        gamma_byte(b),
        coverage_byte(a),
    ]
    .map(|byte| f32::from(byte) / 255.0)
}

/// A lit vertex's colour (the viewer's rig lights it; it carries no baked light
/// and the PSP gave it no vertex colour) as the gamma floats the renderer
/// takes, on no byte grid: the `.vex` path gives such a vertex an invented
/// grey, `[0.75, 0.78, 0.82]`, which no byte spells.
#[must_use]
pub fn lit_vertex(linear: [f32; 4]) -> [f32; 4] {
    let [r, g, b, a] = linear;
    [to_gamma(r), to_gamma(g), to_gamma(b), a]
}

/// The linear value whose [`to_gamma`] is exactly `gamma`, where one lies within a
/// few ulps of [`to_linear`]'s answer; otherwise the nearest. A colour the
/// renderer takes as a float (the lens flare's, which its ring law steps in
/// HSV before truncating) survives the disc export's round trip bit for bit.
#[must_use]
pub fn exact_linear(gamma: f32) -> f32 {
    let guess = to_linear(gamma);
    let mut best = guess;
    for step in 0..=16i32 {
        for sign in [1i32, -1] {
            let bits = guess.to_bits() as i32 + sign * step;
            let Ok(bits) = u32::try_from(bits) else {
                continue;
            };
            let candidate = f32::from_bits(bits);
            if to_gamma(candidate) == gamma {
                return candidate;
            }
            if (to_gamma(candidate) - gamma).abs() < (to_gamma(best) - gamma).abs() {
                best = candidate;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_byte_survives_the_round_trip() {
        for byte in 0..=255u8 {
            let linear = to_linear(f32::from(byte) / 255.0);
            assert_eq!(gamma_byte(linear), byte, "{byte}");
        }
    }

    #[test]
    fn the_curve_ends_where_srgb_does() {
        assert_eq!(gamma_byte(0.0), 0);
        assert_eq!(gamma_byte(1.0), 255);
        assert_eq!(gamma_byte(4.0), 255);
        assert_eq!(gamma_byte(-1.0), 0);
        // sRGB 128 is linear 0.2159.
        assert_eq!(gamma_byte(0.2159), 128);
    }

    #[test]
    fn a_float_colour_comes_back_exactly() {
        for g in [
            1.0f32, 0.6, 0.0, 0.9365, 0.619, 0.2, 0.7107, 0.75, 0.78, 0.82,
        ] {
            assert_eq!(to_gamma(exact_linear(g)), g, "{g}");
        }
    }
}
