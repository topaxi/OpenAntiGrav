//! What the PS2 texture-upload reader in [`super`] is asserted to do: the GIF
//! packets it walks, the GS layout it unswizzles, and the blobs it refuses.
//!
//! Its own file: the tests pass the 200-line inline limit
//! (`scripts/check-file-size.py`).

use super::*;

/// Builds a blob in the shape the disc uses, so the tests need no game data.
fn blob(width: u16, height: u16, bits_per_pixel: u8, rrw: u32, rrh: u32) -> Vec<u8> {
    let pixels = usize::from(width) * usize::from(height);
    let texel_bytes = pixels * usize::from(bits_per_pixel) / 8;
    let palette_bytes = (1usize << bits_per_pixel) * 4;

    let packed = ((height.trailing_zeros() as u8) << 4) | width.trailing_zeros() as u8;
    let mut out = vec![packed, bits_per_pixel, 0x00, 0x20];
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&[0u8; 5]);
    assert_eq!(out.len(), HEADER_LEN);

    out.extend(std::iter::repeat_n(0u8, 8 * 16));
    let reg = |data: u64, addr: u64, out: &mut Vec<u8>| {
        out.extend_from_slice(&data.to_le_bytes());
        out.extend_from_slice(&addr.to_le_bytes());
    };
    reg(0, 0x51, &mut out);
    reg(u64::from(rrw) | (u64::from(rrh) << 32), 0x52, &mut out);
    reg(0, 0x53, &mut out);
    let tag = (texel_bytes as u64 / 16) | (GIF_FLG_IMAGE << 58);
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(&[0u8; 8]);
    assert_eq!(out.len(), TEXEL_OFFSET);

    out.extend((0..texel_bytes).map(|i| (i % 251) as u8));

    reg(0, 0x51, &mut out);
    reg(16 | (16 << 32), 0x52, &mut out);
    reg(0, 0x53, &mut out);
    let tag = (palette_bytes as u64 / 16) | (1 << 15) | (GIF_FLG_IMAGE << 58);
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(&[0u8; 8]);

    out.extend((0..palette_bytes).map(|i| (i % 253) as u8));
    // Both transfers' padding lands at the end of the file (see `super::header`).
    // Every disc shape has a texel block at or above the 256-byte minimum, so this
    // term is zero for them; it lets a *small* blob be built, as the 8x8 repro needs.
    out.extend(std::iter::repeat_n(
        0u8,
        (padded(palette_bytes) - palette_bytes) + (padded(texel_bytes) - texel_bytes),
    ));
    out
}

#[test]
fn a_swizzled_blob_decodes_to_every_texel_exactly_once() {
    let data = blob(64, 64, 8, 32, 32);
    let texture = parse(&data).expect("parse");
    assert_eq!(texture.layout, Layout::Psmt8);
    assert_eq!(texture.indices.len(), 64 * 64);

    // The permutation must be a permutation: every source byte lands once.
    let mut seen = vec![false; 64 * 64];
    for y in 0..64 {
        for x in 0..64 {
            let at = psmt8_offset(x, y, 64);
            assert!(!std::mem::replace(&mut seen[at], true), "{at} twice");
        }
    }
}

#[test]
fn psmt8_offsets_are_a_permutation_for_every_shipped_shape() {
    // Every (width, height) the PS2 disc stores swizzled, widths 16 and up.
    for width in [16usize, 32, 64, 128, 256, 512] {
        for height in [4usize, 8, 16, 32, 64, 128, 256, 512] {
            let mut seen = vec![false; width * height];
            for y in 0..height {
                for x in 0..width {
                    let at = psmt8_offset(x, y, width);
                    assert!(at < width * height, "{width}x{height}: {at} out of range");
                    assert!(
                        !std::mem::replace(&mut seen[at], true),
                        "{width}x{height}: {at} twice"
                    );
                }
            }
        }
    }
}

#[test]
fn a_narrow_blob_is_linear_and_decodes_in_raster_order() {
    let data = blob(8, 32, 8, 8, 32);
    let texture = parse(&data).expect("parse");
    assert_eq!(texture.layout, Layout::Linear);
    assert_eq!(
        texture.indices,
        (0..8 * 32).map(|i| (i % 251) as u8).collect::<Vec<_>>()
    );
}

#[test]
fn the_palette_reordering_is_its_own_inverse() {
    let stored: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
    let once = unswizzle_clut(&stored, 8);
    let flat: Vec<u8> = once.iter().flatten().copied().collect();
    let twice = unswizzle_clut(&flat, 8);
    assert_eq!(twice, stored.as_chunks::<4>().0.to_vec());
}

#[test]
fn a_four_bit_blob_decodes() {
    let data = blob(256, 512, 4, 128, 128);
    let head = header(&data).expect("header");
    assert_eq!(head.layout, Layout::Psmt4);
    let texture = parse(&data).expect("parse");
    assert_eq!(texture.indices.len(), 256 * 512);
    // 4bpp, so every index is a nibble.
    assert!(texture.indices.iter().all(|&i| i < 16));
}

/// An 8x8 `PSMT8` blob whose `GIFtag`s and `TRXREG` all agree, so nothing before
/// the unswizzle rejects it. `psmt8_offset(4, 6, 8)` is 77 against a 64-byte
/// texel buffer, which once panicked the parser (reachable from
/// `Archives::read_font` and archive browsing).
#[test]
fn an_eight_bit_blob_too_small_for_its_permutation_is_refused() {
    let data = blob(8, 8, 8, 4, 4);
    assert_eq!(
        header(&data).expect("header").layout,
        Layout::Psmt8,
        "the shape is recognised"
    );
    assert_eq!(parse(&data), Err(Error::UnsupportedLayout(Layout::Psmt8)));
    // 8 wide is the panicking half; 4 tall the other (a 16x2 blob passes a width-only guard).
    let short = blob(16, 2, 8, 8, 1);
    assert_eq!(
        parse(&short),
        Err(Error::UnsupportedLayout(Layout::Psmt8)),
        "a two-row blob is under the permutation's floor too"
    );
    // The first shapes either side of the bound still parse.
    assert!(parse(&blob(16, 4, 8, 8, 2)).is_ok(), "16x4 is valid PSMT8");
}

#[test]
fn a_four_bit_blob_smaller_than_a_page_is_refused() {
    let data = blob(64, 128, 4, 32, 32);
    assert_eq!(
        header(&data).expect("header").layout,
        Layout::Psmt4,
        "the shape is recognised"
    );
    assert_eq!(parse(&data), Err(Error::UnsupportedLayout(Layout::Psmt4)));
}

/// The test that kills a wrong `PSMT4` model before any pixel is looked at:
/// the reading without the block transposition is not even a bijection.
#[test]
fn psmt4_offsets_are_a_permutation_for_every_shipped_shape() {
    // The five font atlases are 256x128, 512x256 and 512x512; the rest is
    // headroom. `parse` refuses anything under a 128x128 PSMT4 page
    // (`a_four_bit_blob_smaller_than_a_page_is_refused`).
    for width in [128usize, 256, 512] {
        for height in [128usize, 256, 512] {
            let mut seen = vec![false; width * height];
            for y in 0..height {
                for x in 0..width {
                    let (at, nibble) = psmt4_offset(x, y, width);
                    assert!(nibble < 2, "{width}x{height}: nibble {nibble}");
                    let slot = at * 2 + nibble;
                    assert!(
                        slot < width * height,
                        "{width}x{height}: {slot} out of range"
                    );
                    assert!(
                        !std::mem::replace(&mut seen[slot], true),
                        "{width}x{height}: {slot} twice"
                    );
                }
            }
        }
    }
}

#[test]
fn a_truncated_blob_is_refused() {
    let data = blob(64, 64, 8, 32, 32);
    assert!(!looks_like_ps2_texture(&data[..data.len() - 1]));
    assert!(!looks_like_ps2_texture(&data[..10]));
}

#[test]
fn disagreeing_dimensions_are_refused() {
    let mut data = blob(64, 64, 8, 32, 32);
    data[0] = 0x77;
    assert!(matches!(
        header(&data),
        Err(Error::DimensionMismatch { .. })
    ));
}

#[test]
fn a_giftag_that_does_not_match_the_dimensions_is_refused() {
    let mut data = blob(64, 64, 8, 32, 32);
    data[TEXEL_GIFTAG_OFFSET] = 0x01;
    assert!(matches!(header(&data), Err(Error::BadGifTag { .. })));
}

#[test]
fn an_unrecognised_transfer_rectangle_is_refused() {
    let data = blob(64, 64, 8, 7, 9);
    assert!(matches!(header(&data), Err(Error::UnknownTransfer { .. })));
}

#[test]
fn alpha_is_doubled_out_of_the_gs_scale() {
    let mut data = blob(64, 64, 8, 32, 32);
    let clut = TEXEL_OFFSET + 64 * 64 + CLUT_SETUP_QWORDS * 16;
    data[clut..clut + 8].copy_from_slice(&[1, 2, 3, 128, 4, 5, 6, 64]);
    let texture = parse(&data).expect("parse");
    assert_eq!(texture.palette[0], [1, 2, 3, 128]);
    let rgba = texture.to_rgba();
    // Index 0 appears wherever the source byte was 0.
    let first = texture
        .indices
        .iter()
        .position(|&i| i == 0)
        .expect("index 0");
    assert_eq!(&rgba[first * 4..first * 4 + 4], &[1, 2, 3, 255]);
}
