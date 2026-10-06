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
    // TileMode 0x0d (`Thin_1DThin`): micro-tiled, not linear. See the module's
    // "What this does not do".
    assert_eq!(texture.tile_mode.0, 0x0d);
    assert!(!texture.is_linear());
}

#[test]
fn the_channel_order_is_standard_rgba() {
    // word3's channel-order fields decode to Red/Green/Blue/Alpha (4, 5, 6, 7,
    // `AmdGpu::CompSwizzle`'s values); not kept in a field, nothing reorders channels.
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
    assert!(!TileMode(0x0d).is_linear(), "Thin_1DThin");
}

#[test]
fn decode_reports_out_of_bounds_on_the_truncated_real_sample() {
    // `holographic_glow_head()` is the descriptor only, so decoding its
    // `Thin_1DThin` base level (32 BC7 blocks, 512 bytes past `data_offset`) runs
    // off the 52-byte fixture. See
    // `decode_refuses_a_tile_mode_with_no_address_formula_by_name` for the
    // refused-by-name case.
    let texture = Texture::parse(&holographic_glow_head()).expect("parses");
    assert!(matches!(
        texture.decode(&holographic_glow_head()),
        Err(Error::DataOutOfBounds { .. })
    ));
}

#[test]
fn decode_refuses_a_tile_mode_with_no_address_formula_by_name() {
    // `Thin_2DThin` (14), macro-tiled, has no address formula here and no sampled
    // `.gnf` declares it. Built from the linear fixture with only `word3`'s
    // `tile_mode` rewritten, so the refusal is about the tile mode alone.
    let mut bytes = linear_bc7_4x4([0u8; 16]);
    let at = 16;
    let word3 = 0x0eu32 << 20; // TileMode 14, Thin_2DThin
    bytes[at + 12..at + 16].copy_from_slice(&word3.to_le_bytes());
    let texture = Texture::parse(&bytes).expect("parses");
    assert_eq!(texture.decode(&bytes), Err(Error::Tiled { tile_mode: 14 }));
}

/// A minimal synthetic single-block `.gnf`: linear, BC7, 4x4, hand-buildable
/// unlike the real (tiled) samples. Exercises the linear path of `Texture::decode`.
fn linear_bc7_4x4(block: [u8; 16]) -> Vec<u8> {
    let descriptor_len = 36;
    let contents_len = 8 + descriptor_len;
    let mut bytes = vec![0u8; HEADER_LEN + contents_len + 16];
    bytes[0..4].copy_from_slice(b"GNF ");
    bytes[4..8].copy_from_slice(&(contents_len as u32).to_le_bytes());
    bytes[8] = 0x02; // version
    bytes[9] = 0x01; // texture count
    bytes[10] = 0x08; // alignment
    let stream_size = (HEADER_LEN + contents_len + 16) as u32;
    bytes[12..16].copy_from_slice(&stream_size.to_le_bytes());
    let at = 16;
    let word1 = 0x29u32 << 20; // SurfaceFormat::Bc7
    let word2 = 3u32 | (3u32 << 14); // (width-1) | (height-1)<<14 -> 4x4
    let word3 = 0x08u32 << 20; // TileMode::LINEAR_ALIGNED
    bytes[at + 4..at + 8].copy_from_slice(&word1.to_le_bytes());
    bytes[at + 8..at + 12].copy_from_slice(&word2.to_le_bytes());
    bytes[at + 12..at + 16].copy_from_slice(&word3.to_le_bytes());
    let data_offset = HEADER_LEN + contents_len;
    bytes[data_offset..data_offset + 16].copy_from_slice(&block);
    bytes
}

#[test]
fn decode_untiles_a_linear_bc7_surface() {
    // An all-zero BC7 block is the spec's reserved encoding, decoded as opaque
    // black by `bcn::bc7`; its expected output needs no BC7 knowledge to state.
    let bytes = linear_bc7_4x4([0u8; 16]);
    let texture = Texture::parse(&bytes).expect("parses");
    assert!(texture.is_linear());
    assert_eq!(texture.surface_format, SurfaceFormat::Bc7);
    let rgba = texture.decode(&bytes).expect("decodes");
    assert_eq!(rgba, vec![[0, 0, 0, 255]; 16]);
}

#[test]
fn decode_reports_an_unsupported_format_by_name() {
    let mut bytes = linear_bc7_4x4([0u8; 16]);
    // Rewrite word1 to a format this module does not name: `SurfaceFormat::Other`.
    let at = 16;
    let word1 = 0x3fu32 << 20;
    bytes[at + 4..at + 8].copy_from_slice(&word1.to_le_bytes());
    let texture = Texture::parse(&bytes).expect("parses");
    assert_eq!(
        texture.decode(&bytes),
        Err(Error::UnsupportedFormat {
            format: SurfaceFormat::Other(0x3f)
        })
    );
}
