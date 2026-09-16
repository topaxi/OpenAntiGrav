use super::*;

/// The first 32 bytes of `omega-ps4-eu`'s `data03.psarc` entry
/// `Data/art/published/hdships/harimau/Livery2/Holographic_02_GLOW.gnf`,
/// read through `psarc_cat` - `docs/formats/psarc.md`'s own "valid" example.
/// Padded to `DESCRIPTOR_LEN` + contents so [`Texture::parse`] has enough to
/// read a full descriptor; the real file continues with pixel data this
/// fixture does not need.
fn holographic_glow_head() -> Vec<u8> {
    let mut bytes = vec![0u8; HEADER_LEN + 8 + DESCRIPTOR_LEN];
    bytes[0..4].copy_from_slice(b"GNF ");
    bytes[4..8].copy_from_slice(&0x0000_00f8u32.to_le_bytes());
    bytes[8] = 0x02; // version
    bytes[9] = 0x01; // texture count
    bytes[10] = 0x08; // alignment
    bytes[12..16].copy_from_slice(&0x0000_4100u32.to_le_bytes()); // stream_size = 16640
    let at = 16;
    bytes[at..at + 4].copy_from_slice(&0u32.to_le_bytes()); // word0
    bytes[at + 4..at + 8].copy_from_slice(&0x2690_0008u32.to_le_bytes()); // word1
    bytes[at + 8..at + 12].copy_from_slice(&0x700f_c07fu32.to_le_bytes()); // word2
    bytes[at + 12..at + 16].copy_from_slice(&0x96d7_0facu32.to_le_bytes()); // word3
    bytes
}

#[test]
fn the_magic_and_contents_fields_read_off_a_real_entry() {
    let texture = Texture::parse(&holographic_glow_head()).expect("parses");
    assert_eq!(texture.version, 2);
    assert_eq!(texture.alignment, 8);
    assert_eq!(texture.stream_size, 16_640, "the entry's own declared size");
    assert_eq!(
        texture.data_offset,
        HEADER_LEN + 0xf8,
        "pixel data at +0x100"
    );
}

#[test]
fn the_descriptor_decodes_to_a_plausible_bc7_srgb_livery_decal() {
    let texture = Texture::parse(&holographic_glow_head()).expect("parses");
    assert_eq!(texture.surface_format, SurfaceFormat::Bc7);
    assert_eq!(texture.channel_type, ChannelType::Srgb);
    assert_eq!(texture.width, 128);
    assert_eq!(texture.height, 64);
    assert!(texture.surface_format.is_bcn());
}

#[test]
fn the_real_sample_is_tiled_not_linear() {
    let texture = Texture::parse(&holographic_glow_head()).expect("parses");
    // TileMode 0x0d ("Thin_2DThin") - genuinely tiled, the reason this
    // module stops at identification. See the module's own "What this does
    // not do".
    assert_eq!(texture.tile_mode.0, 0x0d);
    assert!(!texture.is_linear());
}

#[test]
fn the_channel_order_is_standard_rgba() {
    // word3's four 3-bit channel-order fields decode to Red/Green/Blue/Alpha
    // (4, 5, 6, 7 - AmdGpu::CompSwizzle's own values), the ordinary case, not
    // decoded into its own field since nothing here reorders channels yet.
    let word3 = 0x96d7_0facu32;
    let cx = word3 & 0x7;
    let cy = (word3 >> 3) & 0x7;
    let cz = (word3 >> 6) & 0x7;
    let cw = (word3 >> 9) & 0x7;
    assert_eq!((cx, cy, cz, cw), (4, 5, 6, 7));
}

#[test]
fn a_bad_magic_is_rejected() {
    let mut bytes = holographic_glow_head();
    bytes[0] = b'X';
    assert_eq!(
        Texture::parse(&bytes),
        Err(Error::BadMagic { found: *b"XNF " })
    );
}

#[test]
fn a_short_buffer_is_rejected_rather_than_panicking() {
    assert_eq!(
        Texture::parse(&[]),
        Err(Error::TooShort {
            need: HEADER_LEN,
            got: 0
        })
    );
    assert_eq!(
        Texture::parse(b"GNF "),
        Err(Error::TooShort {
            need: HEADER_LEN,
            got: 4
        })
    );
}

#[test]
fn linear_tile_modes_are_recognised() {
    assert!(TileMode(0x08).is_linear(), "Display_LinearAligned");
    assert!(TileMode(0x1f).is_linear(), "Display_LinearGeneral");
    assert!(!TileMode(0x0d).is_linear(), "Thin_2DThin");
}
