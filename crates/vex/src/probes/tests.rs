//! The `.probes` reader on a synthetic two-section file: the layout closes, a
//! block decodes the way the module doc says, and a position blends the probes
//! its section pair names.

use super::*;

fn file(positions: &[[f32; 3]], shade: &dyn Fn(usize) -> [u8; 3]) -> Vec<u8> {
    let n = positions.len();
    let mut out = Vec::new();
    out.extend_from_slice(&(n as u32).to_be_bytes());
    out.extend_from_slice(&16u32.to_be_bytes());
    for p in positions {
        for c in [p[0], p[1], p[2], p[2]] {
            out.extend_from_slice(&c.to_be_bytes());
        }
    }
    for bytes in [IRRADIANCE_BYTES, REFLECTION_BYTES] {
        for probe in 0..n {
            let rgb = shade(probe);
            for _ in 0..bytes / 12 {
                out.extend_from_slice(&[rgb[2], rgb[1], rgb[0], rgb[2], rgb[1], rgb[0]]);
                out.extend_from_slice(&[0; 6]);
            }
        }
    }
    out.extend(std::iter::repeat_n(0u8, n * TAIL_PER_PROBE));
    out
}

/// Two sections 12 apart along x, left and right probes 20 apart along z.
fn quad() -> [[f32; 3]; 4] {
    [[0.0, 0.0, -10.0], [0.0, 0.0, 10.0], [12.0, 0.0, -10.0], [12.0, 0.0, 10.0]]
}

fn shade(i: usize) -> [u8; 3] {
    [[200, 0, 0], [0, 200, 0], [0, 0, 200], [100, 100, 100]][i]
}

#[test]
fn a_block_is_two_endpoints_and_three_bit_indices_lsb_first() {
    // c0 = BGR (0, 0, 70) -> red 70; c1 = BGR (0, 0, 140) -> red 140; texel 0
    // index 0, texel 1 index 7, texel 2 index 1 (bits 0-8 = 0b001_111_000).
    let mut block = [0u8; 12];
    block[..6].copy_from_slice(&[0, 0, 70, 0, 0, 140]);
    let bits: u64 = 0b001_111_000;
    block[6..].copy_from_slice(&bits.to_le_bytes()[..6]);
    let mut bytes = Vec::new();
    for _ in 0..16 {
        bytes.extend_from_slice(&block);
    }
    let image = decode(&bytes, 16);
    assert_eq!(image.len(), 16 * 16 * 4);
    assert_eq!(image[0], 70, "texel 0 is c0");
    assert_eq!(image[4], 140, "texel 1 is c1");
    assert_eq!(image[8], 80, "texel 2 is c0 + (c1 - c0) / 7");
    assert_eq!(image[3], 255);
}

#[test]
fn the_layout_must_close_exactly() {
    let mut bytes = file(&quad(), &shade);
    assert_eq!(Probes::parse(&bytes).expect("closes").len(), 4);
    bytes.push(0);
    assert!(matches!(Probes::parse(&bytes), Err(Error::Length { .. })));
    assert_eq!(Probes::parse(&[0; 4]).unwrap_err(), Error::Truncated);
    let mut bad = file(&quad(), &shade);
    bad[7] = 15;
    assert_eq!(Probes::parse(&bad).unwrap_err(), Error::Stride(15));
}

#[test]
fn a_position_blends_the_four_probes_of_its_sections_bilinearly() {
    let probes = Probes::parse(&file(&quad(), &shade)).expect("parses");
    let centre = probes.locate([6.0, 0.0, 0.0]);
    assert_eq!(centre.probes, [0, 2, 1, 3]);
    for w in centre.weights {
        assert!((w - 0.25).abs() < 1e-6, "{:?}", centre.weights);
    }
    let corner = probes.locate([0.0, 0.0, -10.0]);
    assert!((corner.weights[0] - 1.0).abs() < 1e-6, "{:?}", corner.weights);
    let outside = probes.locate([60.0, 5.0, -90.0]);
    assert!((outside.weights[1] - 1.0).abs() < 1e-6, "clamped to the far corner: {:?}", outside.weights);
}

#[test]
fn the_blend_is_the_weighted_sum_of_the_decoded_probes() {
    let probes = Probes::parse(&file(&quad(), &shade)).expect("parses");
    let taps = Taps { probes: [0, 1, 0, 0], weights: [0.75, 0.25, 0.0, 0.0] };
    let irr = probes.blend_irradiance(&taps);
    assert_eq!(irr.len(), 64 * 32 * 4);
    assert_eq!(&irr[..4], &[150, 50, 0, 255]);
    let refl = probes.blend_reflection(&taps);
    assert_eq!(refl.len(), 128 * 64 * 4);
    assert_eq!(&refl[..4], &[150, 50, 0, 255]);
}
