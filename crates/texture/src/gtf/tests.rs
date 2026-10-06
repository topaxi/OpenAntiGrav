use super::*;

/// A one-texture file whose descriptor the caller fills in.
fn blob(descriptor: &[u8; DESCRIPTOR_LEN], texels: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0x0105_0000u32.to_be_bytes());
    out.extend_from_slice(&((DESCRIPTOR_LEN + texels.len()) as u32).to_be_bytes());
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(descriptor);
    out.extend_from_slice(texels);
    out
}

/// `format` at `width` x `height`, one level, tightly packed, texels at 0x30.
fn descriptor(format: u8, width: u16, height: u16, len: u32) -> [u8; DESCRIPTOR_LEN] {
    let mut out = [0u8; DESCRIPTOR_LEN];
    out[4..8].copy_from_slice(&((HEADER_LEN + DESCRIPTOR_LEN) as u32).to_be_bytes());
    out[8..12].copy_from_slice(&len.to_be_bytes());
    out[12] = format;
    out[13] = 1;
    out[14] = 2;
    out[16..20].copy_from_slice(&0x0000_aae4u32.to_be_bytes());
    out[20..22].copy_from_slice(&width.to_be_bytes());
    out[22..24].copy_from_slice(&height.to_be_bytes());
    out[24..26].copy_from_slice(&1u16.to_be_bytes());
    out
}

/// A BC1 block: two `R5G6B5` endpoints little-endian, then a byte of selectors
/// per row. Red then blue, every texel taking endpoint 0 or 1 alternately.
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
fn a_one_block_dxt1_texture_decodes_to_its_two_endpoints() {
    let data = blob(&descriptor(0x86, 4, 4, 8), &RED_BLUE_DXT1);
    let gtf = Gtf::parse(&data).expect("parses");
    let texture = gtf.only().expect("one texture");
    assert_eq!(texture.format, Format::Dxt1);
    assert_eq!((texture.width, texture.height), (4, 4));

    let rgba = texture.to_rgba(&data).expect("decodes");
    assert_eq!(rgba.len(), 16);
    assert_eq!(rgba[0], [255, 0, 0, 255]);
    assert_eq!(rgba[1], [0, 0, 255, 255]);
    assert_eq!(rgba[2], [255, 0, 0, 255]);
    assert_eq!(rgba[3], [0, 0, 255, 255]);
}

#[test]
fn dxt1_with_the_endpoints_in_the_other_order_gets_a_transparent_fourth_entry() {
    // `c0 < c1` is the three-colour mode, and selector 3 is transparent black.
    let mut block = RED_BLUE_DXT1;
    block[0..4].rotate_left(2);
    block[4] = 0b1110_0100; // texels 0..3 take entries 0, 1, 2, 3
    let data = blob(&descriptor(0x86, 4, 4, 8), &block);
    let texture = Gtf::parse(&data).expect("parses");
    let rgba = texture
        .only()
        .expect("one")
        .to_rgba(&data)
        .expect("decodes");

    assert_eq!(rgba[0], [0, 0, 255, 255]);
    assert_eq!(rgba[1], [255, 0, 0, 255]);
    assert_eq!(rgba[2], [127, 0, 127, 255], "the midpoint, not a third");
    assert_eq!(rgba[3], [0, 0, 0, 0], "transparent black");
}

#[test]
fn a_dxt45_block_interpolates_its_alpha_ramp() {
    let mut block = [0u8; 16];
    block[0] = 255;
    block[1] = 0;
    // Selector 1 is `a1`, which is 0; the default 0 selects `a0`, which is 255.
    block[2] = 0b0000_1000; // texel 1 takes selector 1
    block[8..].copy_from_slice(&RED_BLUE_DXT1);
    let data = blob(&descriptor(0x88, 4, 4, 16), &block);
    let texture = Gtf::parse(&data).expect("parses");
    let rgba = texture
        .only()
        .expect("one")
        .to_rgba(&data)
        .expect("decodes");

    assert_eq!(rgba[0][3], 255);
    assert_eq!(rgba[1][3], 0);
    // BC3 always takes the four-colour arithmetic, whatever the endpoint order.
    assert_eq!(&rgba[0][..3], &[255, 0, 0]);
}

#[test]
fn a_dxt23_block_takes_four_bits_of_alpha_per_texel() {
    let mut block = [0u8; 16];
    block[0] = 0x0f; // texel 0 opaque, texel 1 clear
    block[8..].copy_from_slice(&RED_BLUE_DXT1);
    let data = blob(&descriptor(0x87, 4, 4, 16), &block);
    let texture = Gtf::parse(&data).expect("parses");
    let rgba = texture
        .only()
        .expect("one")
        .to_rgba(&data)
        .expect("decodes");

    assert_eq!(rgba[0][3], 255);
    assert_eq!(rgba[1][3], 0);
}

#[test]
fn a_linear_a8r8g8b8_texture_reads_alpha_first() {
    // One texel, bytes A R G B.
    let data = blob(&descriptor(0xa5, 1, 1, 4), &[0x40, 0x10, 0x20, 0x30]);
    let texture = Gtf::parse(&data).expect("parses");
    let rgba = texture
        .only()
        .expect("one")
        .to_rgba(&data)
        .expect("decodes");
    assert_eq!(rgba, [[0x10, 0x20, 0x30, 0x40]]);
}

#[test]
fn a_swizzled_texture_reads_the_rsx_z_order_not_raster_order() {
    // 4x4, because a 2x2 Z-order coincides with raster order and would pass
    // with no addressing at all. Sixteen texels `[index*0x10, 0, 0, 0xff]` in
    // raster position, permuted into Z-order; decoding must recover
    // `index*0x10` in R. The permutation is the textbook 4x4 sequence
    // 0,1,4,5,2,3,6,7,8,9,12,13,10,11,14,15.
    const Z_ORDER: [usize; 16] = [0, 1, 4, 5, 2, 3, 6, 7, 8, 9, 12, 13, 10, 11, 14, 15];
    let mut texels = [[0u8; 4]; 16];
    for (raster, &z) in Z_ORDER.iter().enumerate() {
        texels[z] = [0xff, (raster * 0x10) as u8, 0x00, 0x00];
    }
    let bytes: Vec<u8> = texels.iter().flatten().copied().collect();
    let data = blob(&descriptor(0x85, 4, 4, 64), &bytes);
    let texture = Gtf::parse(&data).expect("parses");
    assert!(!texture.only().expect("one").is_linear());

    let rgba = texture
        .only()
        .expect("one")
        .to_rgba(&data)
        .expect("decodes");
    for (raster, texel) in rgba.iter().enumerate() {
        assert_eq!(
            *texel,
            [(raster * 0x10) as u8, 0x00, 0x00, 0xff],
            "raster position {raster}"
        );
    }
}

#[test]
fn the_pitch_does_not_halve_down_the_mip_chain() {
    // `zone_2/gradienttex_tr01_set01.gtf`: 3x1 A8R8G8B8, pitch 12, two levels,
    // 24 bytes - which is 12 + 12 rather than 12 + 6.
    let mut d = descriptor(0xa5, 3, 1, 24);
    d[13] = 2;
    d[28..32].copy_from_slice(&12u32.to_be_bytes());
    let data = blob(&d, &[0u8; 24]);
    let gtf = Gtf::parse(&data).expect("parses");
    let texture = gtf.only().expect("one");
    assert_eq!(texture.level_len(0), 12);
    assert_eq!(texture.level_len(1), 12);
    assert_eq!(texture.chain_len(), 24);
}

#[test]
fn a_pitch_on_a_compressed_texture_is_a_row_of_blocks() {
    // `hexmedal_hd.gtf`: 1024x768 DXT45, pitch 4096, one level, 786,432 bytes -
    // which is 4,096 x 192 block rows, not 4,096 x 768 pixel rows.
    let mut d = descriptor(0x88, 1024, 768, 786_432);
    d[28..32].copy_from_slice(&4096u32.to_be_bytes());
    let mut data = blob(&d, &[]);
    data.resize(HEADER_LEN + DESCRIPTOR_LEN + 786_432, 0);
    let gtf = Gtf::parse(&data).expect("parses");
    assert_eq!(gtf.only().expect("one").chain_len(), 786_432);
}

#[test]
fn a_declared_length_that_disagrees_with_the_descriptor_is_refused() {
    let data = blob(&descriptor(0x86, 8, 8, 8), &[0u8; 8]);
    // 8x8 DXT1 is four blocks, so eight bytes is one quarter of the truth.
    assert_eq!(
        Gtf::parse(&data),
        Err(Error::ChainLengthMismatch {
            expected: 32,
            declared: 8,
        })
    );
}

#[test]
fn a_blob_that_is_not_a_gtf_fails_a_check_rather_than_decoding() {
    assert_eq!(Gtf::parse(b"VEXX").unwrap_err(), Error::TooShort { got: 4 });
    assert!(matches!(
        Gtf::parse(&[0xff; 256]).unwrap_err(),
        Error::BadTextureCount { .. }
    ));
}

#[test]
fn an_odd_sized_texture_keeps_only_the_texels_it_has() {
    // 3x2 DXT1 is one block with twelve texels off the image.
    let data = blob(&descriptor(0x86, 3, 2, 8), &RED_BLUE_DXT1);
    let texture = Gtf::parse(&data).expect("parses");
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
fn a_remap_word_decomposes_into_the_three_the_disc_carries() {
    let identity = Remap::decode(0xaae4);
    assert!(identity.is_identity(), "{identity:?}");

    // The 7 `A8B8G8R8` files. Alpha is forced; nothing else moves.
    let forces_alpha = Remap::decode(0xa9e4);
    assert_eq!(forces_alpha.source, [0, 1, 2, 3]);
    assert_eq!(
        forces_alpha.control,
        [Control::Read, Control::Read, Control::Read, Control::One]
    );

    // The 9 `B8` files. Every output reads blue - slot 2 of an RGBA texel -
    // and alpha is forced on top of that.
    let broadcasts_blue = Remap::decode(0xa9ff);
    assert_eq!(broadcasts_blue.source, [2, 2, 2, 2]);
    assert_eq!(
        broadcasts_blue.control,
        [Control::Read, Control::Read, Control::Read, Control::One]
    );
}

/// The disc's own `B8` shape: one byte per texel, swizzled, 128x64 (the only
/// non-square swizzled files, so the only ones reaching
/// `decode::morton_index`'s second phase).
#[test]
fn a_single_channel_texture_is_broadcast_by_its_own_remap() {
    // 4x2. `morton_index` interleaves one bit each way, then lets x's
    // remaining high bit continue on its own: raster order maps to
    // 0,1,4,5,2,3,6,7 rather than 0..7.
    const Z_ORDER: [usize; 8] = [0, 1, 4, 5, 2, 3, 6, 7];
    let mut texels = [0u8; 8];
    for (raster, &z) in Z_ORDER.iter().enumerate() {
        texels[z] = (raster * 0x10) as u8;
    }

    // With the identity remap the byte stays in blue; that is all the decoder does.
    let plain = blob(&descriptor(0x01, 4, 2, 8), &texels);
    let gtf = Gtf::parse(&plain).expect("parses");
    let rgba = gtf.only().expect("one").to_rgba(&plain).expect("decodes");
    for (raster, texel) in rgba.iter().enumerate() {
        assert_eq!(*texel, [0, 0, (raster * 0x10) as u8, 0], "at {raster}");
    }

    // With the word the disc actually carries, the same decode broadcasts.
    let mut d = descriptor(0x01, 4, 2, 8);
    d[16..20].copy_from_slice(&0x0000_a9ffu32.to_be_bytes());
    let data = blob(&d, &texels);
    let gtf = Gtf::parse(&data).expect("parses");
    let rgba = gtf.only().expect("one").to_rgba(&data).expect("decodes");
    for (raster, texel) in rgba.iter().enumerate() {
        let v = (raster * 0x10) as u8;
        assert_eq!(*texel, [v, v, v, 0xff], "at {raster}");
    }
}

/// No `B8` file on the disc sets the `0x20` bit, so this is the only thing
/// that exercises the raster path for one.
#[test]
fn a_linear_single_channel_texture_reads_rows_in_order() {
    let texels: Vec<u8> = (0..8u8).map(|i| i * 0x10).collect();
    let data = blob(&descriptor(0x21, 4, 2, 8), &texels);
    let gtf = Gtf::parse(&data).expect("parses");
    let texture = gtf.only().expect("one");
    assert!(texture.is_linear());
    let rgba = texture.to_rgba(&data).expect("decodes");
    for (raster, texel) in rgba.iter().enumerate() {
        assert_eq!(*texel, [0, 0, (raster * 0x10) as u8, 0], "at {raster}");
    }
}

#[test]
fn a_cubemap_parses_and_is_not_decoded() {
    let mut d = descriptor(0x86, 4, 4, 48);
    d[15] = 1;
    let data = blob(&d, &[0u8; 48]);
    let gtf = Gtf::parse(&data).expect("parses");
    let texture = gtf.only().expect("one");
    assert!(texture.cubemap);
    assert_eq!(texture.faces(), 6);
    assert_eq!(texture.to_rgba(&data), Err(Error::Cubemap));
}
