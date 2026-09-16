use super::*;

/// A one-texture file whose descriptor the caller fills in.
fn blob(descriptor: &[u8; DESCRIPTOR_LEN], texels: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&0x1000_0003u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&((HEADER_LEN + DESCRIPTOR_LEN) as u32).to_le_bytes());
    out.extend_from_slice(&(texels.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(descriptor);
    out.extend_from_slice(texels);
    out
}

/// `format` at `width` x `height`, one level, texels at 0x40.
fn descriptor(format: u8, width: u16, height: u16, len: u32) -> [u8; DESCRIPTOR_LEN] {
    descriptor_mips(format, width, height, len as usize, 1)
}

/// The same, with a mip count - what a chain-length check needs.
fn descriptor_mips(
    format: u8,
    width: u16,
    height: u16,
    len: usize,
    mips: u8,
) -> [u8; DESCRIPTOR_LEN] {
    let len = len as u32;
    let mut out = [0u8; DESCRIPTOR_LEN];
    out[0..4].copy_from_slice(&((HEADER_LEN + DESCRIPTOR_LEN) as u32).to_le_bytes());
    out[4..8].copy_from_slice(&len.to_le_bytes());
    out[8..12].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    out[0x14..0x18].copy_from_slice(&(u32::from(format) << 24).to_le_bytes());
    out[0x18..0x1a].copy_from_slice(&width.to_le_bytes());
    out[0x1a..0x1c].copy_from_slice(&height.to_le_bytes());
    out[0x1c] = mips;
    out
}

/// A BC2 block: 4 bits alpha per texel, then a BC1 colour block. Texel 0
/// opaque, texel 1 clear, the rest at whatever the nibble byte leaves them -
/// same block `gtf`'s own `a_dxt23_block_takes_four_bits_of_alpha_per_texel`
/// test uses, since the block layout is the same hardware standard either
/// console reads.
const RED_BLUE_DXT1: [u8; 8] = [
    0x00,
    0xf8, // 0xf800, pure red
    0x1f,
    0x00, // 0x001f, pure blue
    0b0100_0100,
    0b0100_0100,
    0b0100_0100,
    0b0100_0100,
];

#[test]
fn a_one_block_ubc2_texture_decodes_to_its_two_endpoints_and_alpha_nibbles() {
    let mut block = [0u8; 16];
    block[0] = 0x0f; // texel 0 opaque, texel 1 clear
    block[8..].copy_from_slice(&RED_BLUE_DXT1);
    let data = blob(&descriptor(0x86, 4, 4, 16), &block);

    let gxt = Gxt::parse(&data).expect("parses");
    let texture = gxt.only().expect("one texture");
    assert_eq!(texture.format(), Some(Format::Ubc2));
    assert_eq!((texture.width, texture.height), (4, 4));

    let rgba = texture.to_rgba(&data).expect("decodes");
    assert_eq!(rgba.len(), 16);
    assert_eq!(rgba[0], [255, 0, 0, 255]);
    assert_eq!(rgba[0][3], 255, "texel 0's alpha nibble is 0xf");
    assert_eq!(rgba[1][3], 0, "texel 1's alpha nibble is 0x0");
    assert_eq!(
        rgba[1][..3],
        [0, 0, 255],
        "texel 1 still carries its colour"
    );
}

/// A 2x2 `Argb8888` texture, one distinct texel per storage slot, so the
/// twiddle mapping and the `A, R, G, B` channel order can both be checked at
/// once against `twiddle`'s own hand-verified 2x2 answer
/// (`twiddle_over_a_square_grid_is_a_plain_bit_interleave`): storage index 0
/// lands at `(0, 0)`, 1 at `(0, 1)`, 2 at `(1, 0)`, 3 at `(1, 1)`.
#[test]
fn a_two_by_two_argb8888_texture_twiddles_and_reorders_channels() {
    #[rustfmt::skip]
    let texels: [u8; 16] = [
        255, 255,   0,   0, // storage 0 -> (0, 0): opaque red
        255,   0, 255,   0, // storage 1 -> (0, 1): opaque green
        255,   0,   0, 255, // storage 2 -> (1, 0): opaque blue
          0,  10,  20,  30, // storage 3 -> (1, 1): transparent, distinct RGB
    ];
    let data = blob(&descriptor(0x0c, 2, 2, 16), &texels);

    let gxt = Gxt::parse(&data).expect("parses");
    let texture = gxt.only().expect("one texture");
    assert_eq!(texture.format(), Some(Format::Argb8888));

    let rgba = texture.to_rgba(&data).expect("decodes");
    assert_eq!(rgba.len(), 4);
    assert_eq!(rgba[0], [255, 0, 0, 255], "(0, 0): red");
    assert_eq!(rgba[1], [0, 0, 255, 255], "(1, 0): blue");
    assert_eq!(rgba[2], [0, 255, 0, 255], "(0, 1): green");
    assert_eq!(rgba[3], [10, 20, 30, 0], "(1, 1): transparent, RGB kept");
}

/// No `MIN_LEVEL_LEN` floor applies to `Argb8888` - unlike `PVRTII4BPP`'s
/// single-word minimum, there is no evidence one is needed (see
/// `Texture::level_len`), so a mip chain down to a single texel is expected
/// to close on the plain `width * height * 4` arithmetic with nothing added.
#[test]
fn an_argb8888_mip_chain_closes_on_the_plain_arithmetic_with_no_floor() {
    // 4x4 down to 1x1: three levels, 16 + 4 + 1 texels, 4 bytes each.
    let plain = (16 + 4 + 1) * 4;
    let data = blob(&descriptor_mips(0x0c, 4, 4, plain, 3), &vec![0u8; plain]);
    let gxt = Gxt::parse(&data).expect("parses");
    assert_eq!(gxt.only().expect("one").format(), Some(Format::Argb8888));
}

#[test]
fn a_magic_that_is_not_gxt_is_refused() {
    assert_eq!(
        Gxt::parse(b"VEXX0000000000000000000000000000"),
        Err(Error::BadMagic {
            tag: u32::from_le_bytes(*b"VEXX")
        })
    );
}

#[test]
fn a_blob_shorter_than_the_header_is_refused() {
    assert_eq!(
        Gxt::parse(b"GXT\0").unwrap_err(),
        Error::TooShort { got: 4 }
    );
}

#[test]
fn a_declared_length_that_disagrees_with_ubc2s_own_arithmetic_is_refused() {
    // 8x8 UBC2 is four blocks of 16 bytes, so 16 is one quarter of the truth.
    let data = blob(&descriptor(0x86, 8, 8, 16), &[0u8; 16]);
    assert_eq!(
        Gxt::parse(&data),
        Err(Error::ChainLengthMismatch {
            expected: 64,
            declared: 16,
        })
    );
}

#[test]
fn an_unsupported_format_parses_but_refuses_to_decode() {
    // 0x02 is U4U4U4U4 - a real SceGxm base format (see the module doc's
    // corroboration of 0x0c against the same public enum ordering), not
    // observed on this title's disc and not decoded here. UBC1 (0x85) used
    // to be this test's example until it was decoded on 2026-09-16 - see
    // `docs/formats/gxt.md`'s "UBC1/UBC3 decode too" section. The declared
    // length is trusted rather than checked, since the block size for a
    // format this module does not know is not knowable; the length here is
    // deliberately not any formula's answer, to show that it passes anyway.
    let data = blob(&descriptor(0x02, 256, 256, 1234), &[0u8; 1234]);
    let gxt = Gxt::parse(&data).expect("parses");
    let texture = gxt.only().expect("one");
    assert_eq!(texture.format(), None);
    assert_eq!(
        texture.to_rgba(&data),
        Err(Error::Unsupported {
            format: 0x0200_0000
        })
    );
}

/// A `PVRTII4BPP` texture, which this module decoded nothing of until
/// 2026-08-27, now goes through the same length check `UBC2` always did - see
/// [`super::MIN_LEVEL_LEN`], whose 16-byte floor is what makes a real mip
/// chain close.
#[test]
fn a_pvrtc_mip_chain_closes_only_with_the_sixteen_byte_level_floor() {
    // 32x32 with four levels: 512 + 128 + 32 + 8 on the plain arithmetic, and
    // the last level is one 4x4 word, which is stored in 16 bytes rather than
    // 8. Every shipped file agrees with the floored figure and none with the
    // plain one.
    let floored = 512 + 128 + 32 + 16;
    let data = blob(
        &descriptor_mips(0x83, 32, 32, floored, 4),
        &vec![0u8; floored],
    );
    let gxt = Gxt::parse(&data).expect("parses");
    assert_eq!(gxt.only().expect("one").format(), Some(Format::Pvrtii4bpp));

    let plain = 512 + 128 + 32 + 8;
    let data = blob(&descriptor_mips(0x83, 32, 32, plain, 4), &vec![0u8; plain]);
    assert_eq!(
        Gxt::parse(&data),
        Err(Error::ChainLengthMismatch {
            expected: floored,
            declared: plain as u32,
        })
    );
}

#[test]
fn an_odd_sized_texture_keeps_only_the_texels_it_has() {
    // 3x2 UBC2 is one block, twelve of whose sixteen texels are off the image.
    let mut block = [0u8; 16];
    block[0..4].fill(0xff); // every texel opaque - four alpha bytes, two texels each
    block[8..].copy_from_slice(&RED_BLUE_DXT1);
    let data = blob(&descriptor(0x86, 3, 2, 16), &block);
    let texture = Gxt::parse(&data).expect("parses");
    let rgba = texture
        .only()
        .expect("one")
        .to_rgba(&data)
        .expect("decodes");
    assert_eq!(rgba.len(), 6);
    assert_eq!(rgba[0], [255, 0, 0, 255]);
    assert_eq!(rgba[3], [255, 0, 0, 255], "row 1 starts the pattern again");
}

#[test]
fn a_header_that_disagrees_with_its_own_dataoffset_and_datasize_is_refused() {
    // `blob` writes a header whose dataOffset/dataSize agree with the
    // descriptor table and the file length; corrupt just the header's
    // dataSize field (byte 0x10) and leave the descriptor's own copy alone,
    // so the failure is unambiguously the header-extent check rather than
    // the per-descriptor `ChainLengthMismatch` this file already covers.
    let mut data = blob(&descriptor(0x86, 4, 4, 16), &[0u8; 16]);
    data[0x10..0x14].copy_from_slice(&999u32.to_le_bytes());
    assert_eq!(
        Gxt::parse(&data),
        Err(Error::BadHeaderExtent {
            offset: (HEADER_LEN + DESCRIPTOR_LEN) as u32,
            size: 999,
        })
    );
}

#[test]
fn twiddle_degenerates_to_raster_order_on_a_single_row() {
    // h == 1 from the start, so every step takes the "w > 1" branch and
    // never interleaves - the same answer raster order gives.
    assert_eq!(twiddle(0, 0, 2, 1), 0);
    assert_eq!(twiddle(1, 0, 2, 1), 1);
}

#[test]
fn twiddle_over_a_4x2_grid_groups_each_column_s_two_rows_together() {
    // Measured shape: `hud_2048.gxt` is 1024x512 (256x128 blocks), so the
    // taller dimension - here the columns, `across` - runs out of bits to
    // interleave against `down` long before `down` does, and the tail
    // becomes a linear run rather than more interleaving. This is the
    // smallest grid that exhibits it: `down` (2) contributes exactly one
    // interleaved bit before `across` (4) needs a second, linear one.
    let order: Vec<(u32, u32)> = (0..4)
        .flat_map(|bx| (0..2).map(move |by| (bx, by)))
        .collect();
    let mut by_index: Vec<((u32, u32), u32)> = order
        .iter()
        .map(|&(bx, by)| ((bx, by), twiddle(bx, by, 4, 2)))
        .collect();
    by_index.sort_by_key(|&(_, index)| index);
    let visiting_order: Vec<(u32, u32)> = by_index.into_iter().map(|(coord, _)| coord).collect();
    assert_eq!(
        visiting_order,
        vec![
            (0, 0),
            (0, 1),
            (1, 0),
            (1, 1),
            (2, 0),
            (2, 1),
            (3, 0),
            (3, 1),
        ],
        "both rows of one column before moving to the next column"
    );
}

#[test]
fn twiddle_over_a_square_grid_is_a_plain_bit_interleave() {
    // Cross-checked by hand against `missile_reticule.gxt`'s 64x64 block
    // grid, the case that first showed the block order was not raster - see
    // `blocks`'s own doc comment. `bx` odd bits, `by` even bits.
    assert_eq!(twiddle(0, 0, 2, 2), 0b00);
    assert_eq!(twiddle(1, 0, 2, 2), 0b10);
    assert_eq!(twiddle(0, 1, 2, 2), 0b01);
    assert_eq!(twiddle(1, 1, 2, 2), 0b11);
}

#[test]
fn coverage_claims_the_header_descriptor_and_texel_span_exactly() {
    let texels = vec![0u8; 32];
    let data = blob(&descriptor(0x00, 4, 4, 32), &texels);
    let seen = coverage(&data);
    assert_eq!(seen.claimed(), data.len());
    assert_eq!(seen.gaps(1), Vec::new());
}

#[test]
fn coverage_on_a_blob_that_does_not_parse_claims_nothing() {
    let seen = coverage(&[0u8; 8]);
    assert_eq!(seen.claimed(), 0);
}

#[test]
fn a_zero_sized_texture_is_refused() {
    let data = blob(&descriptor(0x86, 0, 4, 0), &[]);
    assert_eq!(
        Gxt::parse(&data),
        Err(Error::ZeroSized {
            width: 0,
            height: 4
        })
    );
}
