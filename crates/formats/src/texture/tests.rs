//! What the `.mip` texture reader in [`super`] is asserted to do: palette and
//! pixel decoding at each bit depth, the mip chain, and the headers it
//! refuses.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `texture.rs`: the tests are 269 lines, well past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

/// Builds a texture blob by hand. No game data in any test.
fn build(width: u16, height: u16, bpp: u8, fill: u8) -> Vec<u8> {
    let colours = 1usize << bpp;
    let mut out = Vec::new();
    out.extend(width.to_le_bytes());
    out.extend(height.to_le_bytes());
    out.push(bpp);
    out.extend([0u8, 1, 2, 0]);
    out.extend([0u8; 7]);
    assert_eq!(out.len(), HEADER_LEN);

    for i in 0..colours {
        out.extend([i as u8, 0, 0, 255]);
    }
    out.extend(std::iter::repeat_n(
        fill,
        width as usize * height as usize * bpp as usize / 8,
    ));
    out
}

/// `+0x06` is a mip count, and a chain of them still decodes to level 0.
///
/// This exists because the parser assumed one level for a long time and
/// `psp-texture.md` flagged the assumption as untested against `Data.wad`.
/// The engine-flare sprite is the counter-example: 128x64 at 8bpp with
/// `+0x06 == 4` and 11,920 bytes, where one level implies 9,232. The shape is
/// reproduced here synthetically - no game data in any test.
#[test]
fn a_mipmapped_blob_declares_its_chain_and_decodes_level_zero() {
    let (w, h) = (128u16, 64u16);
    let mut blob = build(w, h, 8, 0);
    blob[6] = 4;
    // Levels 1..3, appended after the level-0 pixels `build` already wrote.
    // Every row here is already at least 16 bytes, so no padding applies -
    // which is exactly why the flare sprite's length is unaffected by it.
    for (lw, lh) in [(64usize, 32usize), (32, 16), (16, 8)] {
        blob.extend(std::iter::repeat_n(0u8, lw * lh));
    }

    // The exact length the disc's own sprite has, arrived at independently.
    assert_eq!(blob.len(), 11_920);

    let parsed = Texture::parse(&blob).expect("a mipmapped texture must parse");
    assert_eq!(parsed.mip_levels, 4);
    assert_eq!(parsed.width, w);
    assert_eq!(parsed.height, h);
    // Level 0 only: the extra levels must not lengthen `indices`, or
    // `png::encode_rgba` would reject the buffer for its dimensions.
    assert_eq!(parsed.indices.len(), usize::from(w) * usize::from(h));

    // Truncating the chain must be refused rather than read as a shorter
    // texture, which is the property that keeps `looks_like_texture` strong.
    blob.truncate(blob.len() - 1);
    assert!(matches!(
        Texture::parse(&blob),
        Err(Error::SizeMismatch { .. })
    ));
}

/// A tail level narrower than 16 bytes pads its rows to 16.
///
/// This is the rule the noise texture forced:
/// `Data\Tex\engineFlare\Engine_noise.mip` is 64x64 at 8bpp with 4 levels
/// and **6,544** bytes, where an unpadded chain implies 6,480. The whole
/// 64-byte difference is level 3, an 8x8 whose 8-byte rows pad to 16.
#[test]
fn a_narrow_tail_level_pads_its_rows_to_sixteen_bytes() {
    let mut blob = build(64, 64, 8, 0);
    blob[6] = 4;
    for (lw, lh) in [(32usize, 32usize), (16, 16)] {
        blob.extend(std::iter::repeat_n(0u8, lw * lh));
    }
    // Level 3 is 8x8: 8 rows of a 16-byte stride, not of an 8-byte one.
    blob.extend(std::iter::repeat_n(0u8, 16 * 8));

    assert_eq!(blob.len(), 6_544, "the length the disc's own texture has");
    let parsed = Texture::parse(&blob).expect("a padded chain must parse");
    assert_eq!(parsed.mip_levels, 4);
    assert_eq!(parsed.indices.len(), 64 * 64);

    // Without the padding the blob is 64 bytes shorter, and must be refused -
    // that is what makes the rule load-bearing rather than cosmetic.
    let mut unpadded = build(64, 64, 8, 0);
    unpadded[6] = 4;
    for n in [32 * 32usize, 16 * 16, 8 * 8] {
        unpadded.extend(std::iter::repeat_n(0u8, n));
    }
    assert_eq!(unpadded.len(), 6_480);
    assert!(matches!(
        Texture::parse(&unpadded),
        Err(Error::SizeMismatch { .. })
    ));
}

/// A single-level blob is unaffected by the mip arithmetic.
///
/// The 346 standalone `.mip` entries that already decoded must keep decoding;
/// this pins that the change only widened what parses.
#[test]
fn one_level_is_still_the_plain_case() {
    for (w, h, bpp) in [(64u16, 16u16, 8u8), (32, 32, 4), (8, 8, 8)] {
        let blob = build(w, h, bpp, 0);
        let parsed = Texture::parse(&blob).expect("parse");
        assert_eq!(parsed.mip_levels, 1);
        assert_eq!(parsed.indices.len(), usize::from(w) * usize::from(h));
    }
}

#[test]
fn unswizzle_is_a_permutation_and_its_own_documented_inverse() {
    for (row_bytes, height) in [
        (16usize, 8usize),
        (128, 128),
        (256, 256),
        (32, 64),
        (512, 128),
    ] {
        let src: Vec<u8> = (0..row_bytes * height).map(|i| (i % 251) as u8).collect();
        let out = unswizzle(&src, row_bytes, height);
        let mut a = src.clone();
        let mut b = out.clone();
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b, "{row_bytes}x{height} is not a permutation");
    }
    // One block column makes the swizzle the identity, which is exactly why
    // the disc's 32x32 4bpp texture cannot distinguish the two readings.
    let src: Vec<u8> = (0..16 * 32).map(|i| (i % 251) as u8).collect();
    assert_eq!(unswizzle(&src, 16, 32), src);
}

#[test]
fn a_swizzled_blob_is_unswizzled_on_parse() {
    let mut blob = build(64, 16, 8, 0);
    blob[7] = 3; // bit 0 set: stored swizzled
    let pixels = 64 * 16;
    let start = HEADER_LEN + 256 * 4;
    for i in 0..pixels {
        blob[start + i] = (i % 251) as u8;
    }
    let parsed = Texture::parse(&blob).expect("parse");
    let expected = unswizzle(&blob[start..start + pixels], 64, 16);
    assert_eq!(parsed.indices, expected);
    assert_ne!(parsed.indices, blob[start..start + pixels].to_vec());

    blob[7] = 2; // bit 0 clear: stored linear
    let linear = Texture::parse(&blob).expect("parse");
    assert_eq!(linear.indices, blob[start..start + pixels].to_vec());
}

/// The case that reached [`crate::png::encode_rgba`]'s assertion.
///
/// A 3x1 4bpp blob truncates to one packed byte, which unpacks to two
/// indices for three pixels, and `to_rgba` then produces 8 bytes where the
/// dimensions call for 12. Refusing at parse keeps that gap from existing.
#[test]
fn rejects_4bpp_with_an_odd_pixel_count() {
    let mut blob = Vec::new();
    blob.extend(3u16.to_le_bytes());
    blob.extend(1u16.to_le_bytes());
    blob.push(4);
    blob.extend([0u8, 1, 2, 0]);
    blob.extend([0u8; 7]);
    blob.extend(std::iter::repeat_n(0u8, 16 * 4));
    blob.push(0x12);

    assert_eq!(
        Texture::parse(&blob),
        Err(Error::OddPixelCountAt4Bpp {
            width: 3,
            height: 1
        })
    );
}

/// The invariant `to_rgba` and the PNG encoder both depend on.
#[test]
fn indices_always_hold_one_entry_per_pixel() {
    for (w, h, bpp) in [(32u16, 16u16, 8u8), (32, 32, 4), (1, 1, 8), (2, 1, 4)] {
        let t = Texture::parse(&build(w, h, bpp, 0x11)).expect("parse");
        let pixels = usize::from(w) * usize::from(h);
        assert_eq!(t.indices.len(), pixels, "{w}x{h} at {bpp}bpp");
        assert_eq!(t.to_rgba().len(), pixels * 4);
    }
}

#[test]
fn parses_an_8bpp_texture() {
    let t = Texture::parse(&build(32, 16, 8, 3)).unwrap();
    assert_eq!((t.width, t.height, t.bits_per_pixel), (32, 16, 8));
    assert_eq!(t.palette.len(), 256);
    assert_eq!(t.indices.len(), 32 * 16);
    assert!(t.indices.iter().all(|&i| i == 3));
}

#[test]
fn parses_a_4bpp_texture_and_unpacks_nibbles() {
    // 0xAB unpacks to index 0xB then 0xA: low nibble is the first pixel.
    let t = Texture::parse(&build(32, 32, 4, 0xAB)).unwrap();
    assert_eq!(t.palette.len(), 16);
    assert_eq!(t.indices.len(), 32 * 32);
    assert_eq!(&t.indices[..4], &[0xB, 0xA, 0xB, 0xA]);
}

#[test]
fn the_4bpp_size_matches_the_real_outlier() {
    // 16 + 16*4 + 32*32/2 = 592, the size of the one 4bpp blob in FE.wad.
    assert_eq!(build(32, 32, 4, 0).len(), 592);
}

#[test]
fn the_8bpp_size_matches_a_real_texture() {
    // 16 + 256*4 + 512*128 = 66576.
    assert_eq!(build(512, 128, 8, 0).len(), 66_576);
}

#[test]
fn rejects_a_blob_of_the_wrong_size() {
    let mut data = build(32, 32, 8, 0);
    data.push(0);
    assert!(matches!(
        Texture::parse(&data),
        Err(Error::SizeMismatch { .. })
    ));
}

#[test]
fn rejects_an_unsupported_depth() {
    let mut data = build(32, 32, 8, 0);
    data[4] = 16;
    assert!(matches!(
        Texture::parse(&data),
        Err(Error::UnsupportedDepth { bits_per_pixel: 16 })
    ));
}

#[test]
fn rejects_a_zero_dimension() {
    let mut data = build(32, 32, 8, 0);
    data[0..2].copy_from_slice(&0u16.to_le_bytes());
    assert!(matches!(
        Texture::parse(&data),
        Err(Error::ZeroSized { .. })
    ));
}

#[test]
fn rejects_something_that_is_not_a_texture() {
    // A .vex model starts with its version word; it must not parse.
    let vex = [6u8, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
    assert!(!Texture::looks_like_texture(&vex));
}

#[test]
fn expands_to_rgba() {
    let t = Texture::parse(&build(32, 16, 8, 5)).unwrap();
    let rgba = t.to_rgba();
    assert_eq!(rgba.len(), 32 * 16 * 4);
    assert_eq!(&rgba[..4], &[5, 0, 0, 255]);
}

#[test]
fn a_flat_image_does_not_look_swizzled() {
    let t = Texture::parse(&build(64, 64, 8, 7)).unwrap();
    assert!(!t.looks_swizzled());
}
