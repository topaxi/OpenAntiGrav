use super::*;

/// Builds a minimal blob with one valid header at `header_at`, its palette
/// and pixel data placed right after it - a hand-authored fixture, not
/// extracted game bytes, per ADR-0006.
fn fixture(header_at: usize, width: u16, height: u16, levels: u8) -> Vec<u8> {
    let palette_bytes = 1024u32;
    let pixel_bytes = u32::from(width) * u32::from(height);
    let palette_offset = header_at + TEXTURE_HEADER_LEN;
    let pixel_offset = palette_offset + palette_bytes as usize;
    let mut data = vec![0u8; pixel_offset + pixel_bytes as usize];

    data[header_at..header_at + 2].copy_from_slice(&width.to_le_bytes());
    data[header_at + 2..header_at + 4].copy_from_slice(&height.to_le_bytes());
    data[header_at + 4] = 8; // bits per pixel
    data[header_at + 5] = levels;
    data[header_at + 8..header_at + 12].copy_from_slice(&palette_bytes.to_le_bytes());
    data[header_at + 12..header_at + 16].copy_from_slice(&pixel_bytes.to_le_bytes());
    data[header_at + 16..header_at + 20].copy_from_slice(&(pixel_offset as u32).to_le_bytes());
    data[header_at + 20..header_at + 24].copy_from_slice(&(palette_offset as u32).to_le_bytes());

    // A recognisable palette and pixel pattern so a caller can tell the
    // slices landed in the right place, not just the right length.
    for (i, entry) in data[palette_offset..pixel_offset].chunks_mut(4).enumerate() {
        entry.copy_from_slice(&[i as u8, 0, 0, 255]);
    }
    for (i, pixel) in data[pixel_offset..].iter_mut().enumerate() {
        *pixel = (i % 256) as u8;
    }
    data
}

#[test]
fn a_valid_header_parses_at_its_own_offset() {
    let data = fixture(0x100, 64, 64, 4);
    let texture = parse_at(&data, ByteOrder::Little, 0, 0x100).expect("valid header");
    assert_eq!(texture.width, 64);
    assert_eq!(texture.height, 64);
    assert_eq!(texture.bits_per_pixel, 8);
    assert_eq!(texture.levels, 4);
    assert_eq!(texture.palette.len(), 1024);
    assert_eq!(texture.indices.len(), 64 * 64);
    assert_eq!(texture.palette[..4], [0, 0, 0, 255]);
    assert_eq!(texture.indices[0], 0);
    assert_eq!(texture.indices[1], 1);
}

#[test]
fn base_plus_offset_addresses_the_header_not_offset_alone() {
    let data = fixture(0x180, 32, 32, 3);
    // The header sits at base(0x80) + offset(0x100) = 0x180.
    let texture = parse_at(&data, ByteOrder::Little, 0x80, 0x100).expect("valid header");
    assert_eq!(texture.width, 32);
    assert_eq!(texture.height, 32);
}

#[test]
fn a_missing_texture_is_none_not_an_error() {
    // All zero bytes: width 0 fails the power-of-two-in-range check. This is
    // the documented common case - five PSP root emitters and all of PS2 -
    // not a malformed file.
    let data = vec![0u8; 0x100];
    assert_eq!(parse_at(&data, ByteOrder::Little, 0, 0), None);
}

#[test]
fn an_implausible_width_is_refused() {
    let mut data = fixture(0, 64, 64, 4);
    // 100 is not a power of two.
    data[0..2].copy_from_slice(&100u16.to_le_bytes());
    assert_eq!(parse_at(&data, ByteOrder::Little, 0, 0), None);
}

#[test]
fn an_out_of_range_pixel_offset_is_refused() {
    let mut data = fixture(0, 64, 64, 4);
    // Point pixel_offset past the end of the blob.
    let bogus = data.len() as u32 + 1;
    data[16..20].copy_from_slice(&bogus.to_le_bytes());
    assert_eq!(parse_at(&data, ByteOrder::Little, 0, 0), None);
}

#[test]
fn a_pixel_region_shorter_than_level_0_is_refused() {
    let mut data = fixture(0, 64, 64, 4);
    // pixel_bytes smaller than width * height (4095 < 4096) must be
    // refused rather than silently truncating level 0.
    data[12..16].copy_from_slice(&4095u32.to_le_bytes());
    assert_eq!(parse_at(&data, ByteOrder::Little, 0, 0), None);
}

#[test]
fn too_short_a_blob_is_refused() {
    let data = vec![0u8; TEXTURE_HEADER_LEN - 1];
    assert_eq!(parse_at(&data, ByteOrder::Little, 0, 0), None);
}
